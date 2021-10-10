use super::*;

// Import hacspec and all needed definitions.
use hacspec_lib::*;

/* TLS 1.3 Key Schedule: See RFC 8446 Section 7 */

pub fn hkdf_expand_label(
    ha: &HashAlgorithm,
    k: &Key,
    label: &Bytes,
    context: &Bytes,
    len: usize,
) -> Res<Key> {
    if len >= 65536 {Err(payload_too_long)}
    else {
        let lenb = bytes(&U16_to_be_bytes(U16(len as u16)));
        let tls13_label = label_tls13.concat(label);
        let info = lenb
            .concat(&lbytes1(&tls13_label)?)
            .concat(&lbytes1(context)?);
        hkdf_expand(ha, k, &info, len as usize)
    }
}

pub fn derive_secret(ha: &HashAlgorithm, k: &Key, label: &Bytes, tx: &Digest) -> Res<Key> {
    hkdf_expand_label(ha, k, label, &bytes(tx), hash_len(ha))
}

pub fn derive_binder_key(ha: &HashAlgorithm, k: &Key) -> Res<MacKey> {
    let early_secret = hkdf_extract(ha, k, &zero_key(ha))?;
    let mk = derive_secret(ha, &early_secret, &bytes(&label_res_binder), &hash_empty(ha)?)?;
    Ok(MacKey::from_seq(&mk))
}

pub fn derive_aead_key_iv(ha: &HashAlgorithm, ae: &AeadAlgorithm, k: &Key) -> Res<AeadKeyIV> {
    let sender_write_key = hkdf_expand_label(ha, k, &bytes(&label_key), &empty(), ae_key_len(ae))?;
    let sender_write_iv = hkdf_expand_label(ha, k, &bytes(&label_iv), &empty(), ae_iv_len(ae))?;
    Ok((
        AeadKey::from_seq(&sender_write_key),
        AeadIv::from_seq(&sender_write_iv),
    ))
}

pub fn derive_0rtt_keys(ha: &HashAlgorithm, ae: &AeadAlgorithm, k: &Key, tx: &Digest) -> Res<(AeadKeyIV, Key)> {
    let early_secret = hkdf_extract(ha, k, &zero_key(ha))?;
    let client_early_traffic_secret =
        derive_secret(ha, &early_secret, &bytes(&label_c_e_traffic), tx)?;
    let early_exporter_master_secret =
        derive_secret(ha, &early_secret, &bytes(&label_c_e_traffic), tx)?;
    let sender_write_key_iv = derive_aead_key_iv(ha, ae, &client_early_traffic_secret)?;
    Ok((sender_write_key_iv, early_exporter_master_secret))
}

pub fn derive_finished_key(ha: &HashAlgorithm, k: &Key) -> Res<MacKey> {
    Ok(hkdf_expand_label(ha,k,&bytes(&label_finished),&empty(),hmac_key_len(ha))?)
}

pub fn derive_hk_ms(
    ha: &HashAlgorithm,
    ae: &AeadAlgorithm,
    gxy: &Key,
    psko: &Option<PSK>,
    tx: &Digest,
) -> Res<(AeadKeyIV, AeadKeyIV, MacKey, MacKey, Key)> {
    let psk = if let Some(k) = psko {Key::from_seq(k)} else {zero_key(ha)};
    let early_secret = hkdf_extract(ha, &psk, &zero_key(ha))?;
    let Digest_emp = hash_empty(ha)?;
    let derived_secret =
        derive_secret(ha, &early_secret, &bytes(&label_derived), &Digest_emp)?;
//    println!("derived secret: {}", derived_secret.to_hex());
    let handshake_secret = hkdf_extract(ha, gxy, &derived_secret)?;
//    println!("handshake secret: {}", handshake_secret.to_hex());
    let client_handshake_traffic_secret =
        derive_secret(ha, &handshake_secret, &bytes(&label_c_hs_traffic), tx)?;
//    println!("c h ts: {}", client_handshake_traffic_secret.to_hex());
    let server_handshake_traffic_secret =
        derive_secret(ha, &handshake_secret, &bytes(&label_s_hs_traffic), tx)?;
 //   println!("s h ts: {}", server_handshake_traffic_secret.to_hex());
    let client_finished_key = derive_finished_key(ha, &client_handshake_traffic_secret)?;
 //   println!("cfk: {}", client_finished_key.to_hex());
    let server_finished_key = derive_finished_key(ha, &server_handshake_traffic_secret)?;
//    println!("sfk: {}", server_finished_key.to_hex());
    let client_write_key_iv = derive_aead_key_iv(ha, ae, &client_handshake_traffic_secret)?;
 //   let (k,iv) = &client_write_key_iv; println!("chk: {}\n     {}", k.to_hex(), iv.to_hex());
    let server_write_key_iv = derive_aead_key_iv(ha, ae, &server_handshake_traffic_secret)?;
 //   let (k,iv) = &server_write_key_iv; println!("shk: {}\n     {}", k.to_hex(), iv.to_hex());
    let master_secret_ =
        derive_secret(ha, &handshake_secret, &bytes(&label_derived), &Digest_emp)?;
    let master_secret = hkdf_extract(ha, &zero_key(ha), &master_secret_)?;
    Ok((
        client_write_key_iv,
        server_write_key_iv,
        client_finished_key,
        server_finished_key,
        master_secret,
    ))
}

pub fn derive_app_keys(
    ha: &HashAlgorithm,
    ae: &AeadAlgorithm,
    master_secret: &Key,
    tx: &Digest,
) -> Res<(AeadKeyIV, AeadKeyIV, Key)> {
    let client_application_traffic_secret_0 =
        derive_secret(ha, &master_secret, &bytes(&label_c_ap_traffic), tx)?;
    let server_application_traffic_secret_0 =
        derive_secret(ha, &master_secret, &bytes(&label_s_ap_traffic), tx)?;
    let client_write_key_iv = derive_aead_key_iv(ha, ae, &client_application_traffic_secret_0)?;
    let server_write_key_iv = derive_aead_key_iv(ha, ae, &server_application_traffic_secret_0)?;
    let exporter_master_secret = derive_secret(ha, master_secret, &bytes(&label_exp_master), tx)?;
    Ok((
        client_write_key_iv,
        server_write_key_iv,
        exporter_master_secret,
    ))
}

pub fn derive_rms(ha: &HashAlgorithm, master_secret: &Key, tx: &Digest) -> Res<Key> {
    let resumption_master_secret = derive_secret(ha, master_secret, &bytes(&label_res_master), tx)?;
    Ok(resumption_master_secret)
}

/* CipherStates Exported by the TLS 1.3 Handshake */
pub struct ClientCipherState0(pub AeadAlgorithm, pub AeadKeyIV, pub u64, pub Key);
pub struct ServerCipherState0(pub AeadAlgorithm, pub AeadKeyIV, pub u64, pub Key);
pub struct DuplexCipherStateH(pub AeadAlgorithm, pub AeadKeyIV, pub u64, pub AeadKeyIV, pub u64);
pub struct DuplexCipherState1(pub AeadAlgorithm, pub AeadKeyIV, pub u64, pub AeadKeyIV, pub u64, pub Key);

/* Incremental Transcript Construction 
   For simplicity, we store the full transcript, but an internal Digest state would suffice. */

pub struct TranscriptTruncatedClientHello(pub HashAlgorithm, pub Digest);
pub struct TranscriptClientHello(pub HashAlgorithm, pub bool, pub HandshakeData, pub Digest);
pub struct TranscriptServerHello(pub HashAlgorithm, pub bool, pub HandshakeData, pub Digest);
pub struct TranscriptServerCertificate(pub HashAlgorithm, pub bool, pub HandshakeData, pub Digest);
pub struct TranscriptServerCertificateVerify(pub HashAlgorithm, pub bool, pub HandshakeData, pub Digest);
pub struct TranscriptServerFinished(pub HashAlgorithm, pub bool, pub HandshakeData, pub Digest);
pub struct TranscriptClientFinished(pub HashAlgorithm, pub bool, pub HandshakeData, pub Digest);


pub fn transcript_add(ha:HashAlgorithm,tx:HandshakeData,msg:&HandshakeData) -> Res<(HandshakeData,Digest)> {
    let tx = handshake_concat(tx,msg);
    let HandshakeData(txby) = tx;
    let th = hash(&ha,&txby)?;
    Ok((HandshakeData(txby),th))
}

pub fn transcript_truncated_client_hello(algs:Algorithms,ch:&HandshakeData,trunc_len:usize) ->
    Res<TranscriptTruncatedClientHello> {
        let Algorithms(ha, ae, sa, gn, psk_mode, zero_rtt) = algs;
        let HandshakeData(ch) = ch;
        let th = hash(&ha,&ch.slice_range(0..trunc_len))?;
        Ok(TranscriptTruncatedClientHello(ha,th))
    }

pub fn transcript_client_hello(algs:Algorithms,ch:&HandshakeData) -> Res<TranscriptClientHello> {
        let Algorithms(ha, ae, sa, gn, psk_mode, zero_rtt) = algs;
        let transcript = HandshakeData(empty());
        let (transcript,th) = transcript_add(ha,transcript,ch)?;
        Ok(TranscriptClientHello(ha,psk_mode,transcript,th))
    }

pub fn transcript_server_hello(tx:TranscriptClientHello,sh:&HandshakeData) -> Res<TranscriptServerHello> {
        let TranscriptClientHello(ha,psk_mode,transcript,_) = tx;
        let (transcript,th) = transcript_add(ha,transcript,sh)?;
        Ok(TranscriptServerHello(ha,psk_mode,transcript,th))
    }

pub fn transcript_server_certificate(tx:TranscriptServerHello,ee:&HandshakeData,sc:&HandshakeData) -> Res<TranscriptServerCertificate> {
        let TranscriptServerHello(ha,psk_mode,transcript,_) = tx;
        if psk_mode {Err(psk_mode_mismatch)}
        else {
            let transcript = handshake_concat(transcript,ee);
            let (transcript,th) = transcript_add(ha,transcript,sc)?;           
            Ok(TranscriptServerCertificate(ha,psk_mode,transcript,th))
        }
    }

pub fn transcript_server_certificate_verify(tx:TranscriptServerCertificate,cv:&HandshakeData) -> Res<TranscriptServerCertificateVerify> {
        let TranscriptServerCertificate(ha,psk_mode,transcript,_) = tx;
        if psk_mode {Err(psk_mode_mismatch)}
        else {
            let (transcript,th) = transcript_add(ha,transcript,cv)?;
            Ok(TranscriptServerCertificateVerify(ha,psk_mode,transcript,th))
        }
    }

pub fn transcript_skip_server_certificate_verify(tx:TranscriptServerHello,ee:&HandshakeData) -> Res<TranscriptServerCertificateVerify> {
        let TranscriptServerHello(ha,psk_mode,transcript,_) = tx;
        if !psk_mode {Err(psk_mode_mismatch)}
        else {
            let (transcript,th) = transcript_add(ha,transcript,ee)?;
            Ok(TranscriptServerCertificateVerify(ha,psk_mode,transcript,th))
        }
    }

pub fn transcript_server_finished(tx:TranscriptServerCertificateVerify,sf:&HandshakeData) -> Res<TranscriptServerFinished> {
        let TranscriptServerCertificateVerify(ha,psk_mode,transcript,_) = tx;
        let (transcript,th) = transcript_add(ha,transcript,sf)?;
        Ok(TranscriptServerFinished(ha,psk_mode,transcript,th))
    }

pub fn transcript_client_finished(tx:TranscriptServerFinished,cf:&HandshakeData) -> Res<TranscriptClientFinished> {
        let TranscriptServerFinished(ha,psk_mode,transcript,_) = tx;
        let (transcript,th) = transcript_add(ha,transcript,cf)?;
        Ok(TranscriptClientFinished(ha,psk_mode,transcript,th))
    }


/* Handshake State Machine */
/* We implement a simple linear state machine:
PostClientHello -> PostServerHello -> PostCertificateVerify ->
PostServerFinished -> PostClientFinished -> Complete
There are no optional steps, all states must be traversed, even if the traversals are NOOPS.
See "put_skip_server_signature" below */

pub struct ClientPostClientHello(Random, Algorithms, KemSk, Option<PSK>);
pub struct ClientPostServerHello(Random, Random, Algorithms, Key, MacKey, MacKey);
pub struct ClientPostCertificateVerify(Random, Random, Algorithms, Key, MacKey, MacKey);
pub struct ClientPostServerFinished(Random, Random, Algorithms, Key, MacKey);
pub struct ClientPostClientFinished(Random, Random, Algorithms, Key);
pub struct ClientComplete(Random, Random, Algorithms, Key);

pub struct ServerPostClientHello(Random, Random, Algorithms, Key, Option<PSK>);
pub struct ServerPostServerHello(Random, Random, Algorithms, Key, MacKey, MacKey);
pub struct ServerPostCertificateVerify(Random, Random, Algorithms, Key, MacKey, MacKey);
pub struct ServerPostServerFinished(Random, Random, Algorithms, Key, MacKey);
pub struct ServerPostClientFinished(Random, Random, Algorithms, Key);
pub struct ServerComplete(Random, Random, Algorithms, Key);

/* Handshake Core Functions: See RFC 8446 Section 4 */
/* We delegate all details of message formatting and transcript Digestes to the caller */

/* TLS 1.3 Client Side Handshake Functions */

pub fn get_client_hello(
    algs0:Algorithms,
    psk: Option<PSK>,
    ent: Entropy,
) -> Res<(Random, KemPk, ClientPostClientHello)> {
    let Algorithms(ha, ae, sa, ks, psk_mode, zero_rtt) = &algs0;
    if ent.len() < 32 + dh_priv_len(ks) {Err(insufficient_entropy)}
    else {
        let cr = Random::from_seq(&ent.slice_range(0..32));
        let (x,gx) = kem_keygen(ks,ent.slice_range(32..32+dh_priv_len(ks)))?;
        Ok((cr, gx, ClientPostClientHello(cr, algs0, x, psk)))
    }
}

pub fn get_client_hello_binder(
    tx: &TranscriptTruncatedClientHello,
    st: &ClientPostClientHello,
) -> Res<Option<HMAC>> {
    let ClientPostClientHello(cr, algs0, x, psk) = st;
    let Algorithms(ha, ae, sa, gn, psk_mode, zero_rtt) = algs0;
    let TranscriptTruncatedClientHello(_,tx_Digest) = tx;
    match (psk_mode, psk) {
        (true,Some(k)) => {
            let mk = derive_binder_key(ha, &k)?;
            let mac = hmac_tag(ha, &mk, &bytes(tx_Digest))?;
            Ok(Some(mac))},
        (false,None) => Ok(None),
        _ => Err(psk_mode_mismatch)
     }
}

pub fn client_get_0rtt_keys(
    tx: &TranscriptClientHello,
    st: &ClientPostClientHello,
) -> Res<Option<ClientCipherState0>> {
    let ClientPostClientHello(cr, algs0, x, psk) = st;
    let TranscriptClientHello(_,_,_,tx_Digest) = tx;
    let Algorithms(ha, ae, sa, gn, psk_mode, zero_rtt) = algs0;
    match (psk_mode, zero_rtt, psk) {
        (true,true,Some(k)) => {
            let (aek, Key) = derive_0rtt_keys(ha, ae, &k, tx_Digest)?;
            Ok(Some(ClientCipherState0(*ae, aek, 0, Key)))},
        (false,false,None) => Ok(None),
        (true,false,Some(k)) => Ok(None),
        _ => Err(psk_mode_mismatch)
    }
}

pub fn put_server_hello(
    sr: Random,
    gy: KemPk,
    algs: Algorithms,
    tx: &TranscriptServerHello,
    st: ClientPostClientHello,
) -> Res<(DuplexCipherStateH, ClientPostServerHello)> {
    let ClientPostClientHello(cr, algs0, x, psk) = st;
    let TranscriptServerHello(_,_,_,tx_Digest) = tx;
    if algs == algs0 {
        let Algorithms(ha, ae, sa, ks, psk_mode, zero_rtt) = &algs;
        let gxy = kem_decap(ks, &gy, x)?;
        let (chk, shk, cfk, sfk, ms) = derive_hk_ms(ha, ae, &gxy, &psk, tx_Digest)?;
        Ok((
            DuplexCipherStateH(*ae, chk, 0, shk, 0),
            ClientPostServerHello(cr, sr, algs, ms, cfk, sfk),
        ))
    } else {
        Err(negotiation_mismatch)
    }
}

pub fn put_server_signature(
    pk: &VerificationKey,
    sig: &Bytes,
    tx: &TranscriptServerCertificate,
    st: ClientPostServerHello,
) -> Res<ClientPostCertificateVerify> {
    let ClientPostServerHello(cr, sr, algs, ms, cfk, sfk) = st;
    let TranscriptServerCertificate(_,_,_,tx_Digest) = tx;
    if let Algorithms(ha, ae, sa, gn, false, zero_rtt) = &algs {
        let sigval = prefix_server_certificate_verify.concat(tx_Digest);
        verify(sa, &pk, &bytes(&sigval), &sig)?;
        Ok(ClientPostCertificateVerify(cr, sr, algs, ms, cfk, sfk))
    } else {
        Err(psk_mode_mismatch)
    }
}

pub fn put_skip_server_signature(st: ClientPostServerHello) -> Res<ClientPostCertificateVerify> {
    let ClientPostServerHello(cr, sr, algs, ms, cfk, sfk) = st;
    if let Algorithms(ha, ae, sa, gn, true, zero_rtt) = &algs {
        Ok(ClientPostCertificateVerify(cr, sr, algs, ms, cfk, sfk))
    } else {
        Err(psk_mode_mismatch)
    }
}

pub fn put_server_finished(
    vd: &HMAC,
    tx: &TranscriptServerCertificateVerify,
    st: ClientPostCertificateVerify,
) -> Res<ClientPostServerFinished> {
    let ClientPostCertificateVerify(cr, sr, algs, ms, cfk, sfk) = st;
    let TranscriptServerCertificateVerify(_,_,_,tx_Digest) = tx;
    let Algorithms(ha, ae, sa, gn, psk_mode, zero_rtt) = &algs;
    hmac_verify(ha, &sfk, &bytes(tx_Digest), &vd)?;
    Ok(ClientPostServerFinished(cr, sr, algs, ms, cfk))
}
pub fn client_get_1rtt_keys(
    tx: &TranscriptServerFinished,
    st: &ClientPostServerFinished,
) -> Res<DuplexCipherState1> {
    let ClientPostServerFinished(_, _, algs, ms, cfk) = st;
    let TranscriptServerFinished(_,_,_,tx_Digest) = tx;
    let Algorithms(ha, ae, sa, gn, psk_mode, zero_rtt) = algs;
    let (cak, sak, exp) = derive_app_keys(ha, ae, &ms, tx_Digest)?;
    Ok(DuplexCipherState1(*ae, cak, 0, sak, 0, exp))
}

pub fn get_client_finished(
    tx: &TranscriptServerFinished,
    st: ClientPostServerFinished,
) -> Res<(HMAC, ClientPostClientFinished)> {
    let ClientPostServerFinished(cr, sr, algs, ms, cfk) = st;
    let TranscriptServerFinished(_,_,_,tx_Digest) = tx;
    let Algorithms(ha, ae, sa, gn, psk_mode, zero_rtt) = &algs;
    let m = hmac_tag(ha, &cfk, &bytes(tx_Digest))?;
    Ok((m, ClientPostClientFinished(cr, sr, algs, ms)))
}

pub fn client_complete(
    tx: &TranscriptClientFinished,
    st: ClientPostClientFinished,
) -> Res<ClientComplete> {
    let ClientPostClientFinished(cr, sr, algs, ms) = st;
    let TranscriptClientFinished(_,_,_,tx_Digest) = tx;
    let Algorithms(ha, ae, sa, gn, psk_mode, zero_rtt) = &algs;
    let rms = derive_rms(ha, &ms, tx_Digest)?;
    Ok(ClientComplete(cr,sr,algs,rms))
}

/* TLS 1.3 Server Side Handshake Functions */

pub fn put_client_hello(
    cr: Random,
    algs: Algorithms,
    gx: &KemPk,
    psk: Option<PSK>,
    tx: TranscriptTruncatedClientHello,
    binder: Option<HMAC>,
    ent: Entropy,
) -> Res<(Random, KemPk, ServerPostClientHello)> {
    let Algorithms(ha, ae, sa, ks, psk_mode, zero_rtt) = &algs;
    if ent.len() < 32 + dh_priv_len(ks) {Err(insufficient_entropy)}
    else {
        let sr = Random::from_seq(&ent.slice_range(0..32));
        let (gxy,gy) = kem_encap(ks,gx,ent.slice_range(32..32+dh_priv_len(ks)))?;
        match (psk_mode, psk, binder) {
            (true, Some(k), Some(binder)) => {
                let mk = derive_binder_key(ha, &k)?;
                let TranscriptTruncatedClientHello(_,tx_Digest) = tx;
                hmac_verify(ha, &mk, &bytes(&tx_Digest), &binder)?;
                Ok((sr, gy, ServerPostClientHello(cr, sr, algs, gxy, Some(k))))
            }
            (false, None, None) => Ok((sr, gy, ServerPostClientHello(cr, sr, algs, gxy, None))),
            _ => Err(psk_mode_mismatch),
        }
    }
}

pub fn server_get_0rtt_keys(
    tx: &TranscriptClientHello,
    st: &ServerPostClientHello,
) -> Res<Option<ServerCipherState0>> {
    let ServerPostClientHello(cr, sr, algs, gxy, psk) = st;
    let TranscriptClientHello(_,_,_,tx_Digest) = tx;
    let Algorithms(ha, ae, sa, gn, psk_mode, zero_rtt) = algs;
    match (psk_mode, zero_rtt, psk) {
        (true,true,Some(k)) => {
            let (aek, Key) = derive_0rtt_keys(ha, ae, &k, tx_Digest)?;
            Ok(Some(ServerCipherState0(*ae, aek, 0, Key)))},
        (false,false,None) => Ok(None),
        (true,false,Some(k)) => Ok(None),
        _ => Err(psk_mode_mismatch)    
        }
}

pub fn get_server_hello(
    tx: &TranscriptServerHello,
    st: ServerPostClientHello,
) -> Res<(DuplexCipherStateH, ServerPostServerHello)> {
    let ServerPostClientHello(cr, sr, algs, gxy, psk) = st;
    let TranscriptServerHello(_,_,_,tx_Digest) = tx;
    let Algorithms(ha, ae, sa, gn, psk_mode, zero_rtt) = &algs;
    let (chk, shk, cfk, sfk, ms) = derive_hk_ms(ha, ae, &gxy, &psk, tx_Digest)?;
    Ok((
        DuplexCipherStateH(*ae, shk, 0, chk, 0),
        ServerPostServerHello(cr, sr, algs, ms, cfk, sfk),
    ))
}

pub fn get_server_signature(
    sk: &SignatureKey,
    tx: &TranscriptServerCertificate,
    st: ServerPostServerHello,
    ent: Entropy,
) -> Res<(Signature, ServerPostCertificateVerify)> {
    let ServerPostServerHello(cr, sr, algs, ms, cfk, sfk) = st;
    let TranscriptServerCertificate(_,_,_,tx_Digest) = tx;
    if let Algorithms(ha, ae, sa, gn, false, zero_rtt) = &algs {
        let sigval = prefix_server_certificate_verify.concat(tx_Digest);
        let sig = sign(sa, &sk, &sigval, ent)?;
        Ok((sig, ServerPostCertificateVerify(cr, sr, algs, ms, cfk, sfk)))
    } else {
        Err(psk_mode_mismatch)
    }
}

pub fn get_skip_server_signature(st: ServerPostServerHello) -> Res<ServerPostCertificateVerify> {
    let ServerPostServerHello(cr, sr, algs, ms, cfk, sfk) = st;
    if let Algorithms(ha, ae, sa, gn, true, zero_rtt) = algs {
        Ok(ServerPostCertificateVerify(cr, sr, algs, ms, cfk, sfk))
    } else {
        Err(psk_mode_mismatch)
    }
}

pub fn get_server_finished(
    tx: &TranscriptServerCertificateVerify,
    st: ServerPostCertificateVerify,
) -> Res<(HMAC, ServerPostServerFinished)> {
    let ServerPostCertificateVerify(cr, sr, algs, ms, cfk, sfk) = st;
    let TranscriptServerCertificateVerify(_,_,_,tx_Digest) = tx;
    let Algorithms(ha, ae, sa, gn, psk_mode, zero_rtt) = &algs;
    let m = hmac_tag(ha, &sfk, &bytes(tx_Digest))?;
    Ok((m, ServerPostServerFinished(cr, sr, algs, ms, cfk)))
}

pub fn server_get_1rtt_keys(
    tx: &TranscriptServerFinished,
    st: &ServerPostServerFinished,
) -> Res<DuplexCipherState1> {
    let ServerPostServerFinished(_, _, algs, ms, cfk) = st;
    let TranscriptServerFinished(_,_,_,tx_Digest) = tx;
    let Algorithms(ha, ae, sa, gn, psk_mode, zero_rtt) = algs;
    let (cak, sak, exp) = derive_app_keys(ha, ae, &ms, tx_Digest)?;
    Ok(DuplexCipherState1(*ae, sak, 0, cak, 0, exp))
}

pub fn put_client_finished(
    mac: HMAC,
    tx: &TranscriptServerFinished,
    st: ServerPostServerFinished,
) -> Res<ServerPostClientFinished> {
    let ServerPostServerFinished(cr, sr, algs, ms, cfk) = st;
    let TranscriptServerFinished(_,_,_,tx_Digest) = tx;
    let Algorithms(ha, ae, sa, gn, psk_mode, zero_rtt) = &algs;
    hmac_verify(ha, &cfk, &bytes(tx_Digest), &mac)?;
    Ok(ServerPostClientFinished(cr, sr, algs, ms))
}

pub fn server_complete(
    tx: &TranscriptClientFinished,
    st: ServerPostClientFinished,
) -> Res<ClientComplete> {
    let TranscriptClientFinished(_,_,_,tx_Digest) = tx;
    let ServerPostClientFinished(cr, sr, algs, ms) = st;
    let Algorithms(ha, ae, sa, gn, psk_mode, zero_rtt) = &algs;
    let rms = derive_rms(ha, &ms, tx_Digest)?;
    Ok(ClientComplete(cr,sr,algs,rms))
}
