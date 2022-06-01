#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_imports)]
#![allow(unused_parens)]

// A module that for the formatting code needed by TLS 1.3
use crate::tls13utils::*;
use hacspec_cryptolib::*;
//use super::*;

// Import hacspec and all needed definitions.
use hacspec_lib::*;

/// Well Known Constants

pub const label_iv: Bytes2 = Bytes2(secret_bytes!([105, 118]));
pub const label_key: Bytes3 = Bytes3(secret_bytes!([107, 101, 121]));
pub const label_tls13: Bytes6 = Bytes6(secret_bytes!([116, 108, 115, 049, 051, 032]));
pub const label_derived: Bytes7 = Bytes7(secret_bytes!([100, 101, 114, 105, 118, 101, 100]));
pub const label_finished: Bytes8 = Bytes8(secret_bytes!([102, 105, 110, 105, 115, 104, 101, 100]));
pub const label_res_binder: Bytes10 = Bytes10(secret_bytes!([
    114, 101, 115, 032, 098, 105, 110, 100, 101, 114
]));
pub const label_ext_binder: Bytes10 = Bytes10(secret_bytes!([
    101, 120, 116, 032, 098, 105, 110, 100, 101, 114
]));
pub const label_exp_master: Bytes10 = Bytes10(secret_bytes!([
    101, 120, 112, 032, 109, 097, 115, 116, 101, 114
]));
pub const label_res_master: Bytes10 = Bytes10(secret_bytes!([
    114, 101, 115, 032, 109, 097, 115, 116, 101, 114
]));
pub const label_c_e_traffic: Bytes11 = Bytes11(secret_bytes!([
    099, 032, 101, 032, 116, 114, 097, 102, 102, 105, 099
]));
pub const label_e_exp_master: Bytes12 = Bytes12(secret_bytes!([
    101, 032, 101, 120, 112, 032, 109, 097, 115, 116, 101, 114
]));
pub const label_c_hs_traffic: Bytes12 = Bytes12(secret_bytes!([
    099, 032, 104, 115, 032, 116, 114, 097, 102, 102, 105, 099
]));
pub const label_s_hs_traffic: Bytes12 = Bytes12(secret_bytes!([
    115, 032, 104, 115, 032, 116, 114, 097, 102, 102, 105, 099
]));
pub const label_c_ap_traffic: Bytes12 = Bytes12(secret_bytes!([
    099, 032, 097, 112, 032, 116, 114, 097, 102, 102, 105, 099
]));
pub const label_s_ap_traffic: Bytes12 = Bytes12(secret_bytes!([
    115, 032, 097, 112, 032, 116, 114, 097, 102, 102, 105, 099
]));

pub const prefix_server_certificate_verify: Bytes98 = Bytes98(secret_bytes!([
    0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
    0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
    0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
    0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
    0x54, 0x4c, 0x53, 0x20, 0x31, 0x2e, 0x33, 0x2c, 0x20, 0x73, 0x65, 0x72, 0x76, 0x65, 0x72, 0x20,
    0x43, 0x65, 0x72, 0x74, 0x69, 0x66, 0x69, 0x63, 0x61, 0x74, 0x65, 0x56, 0x65, 0x72, 0x69, 0x66,
    0x79, 0x00
]));

const sha256_empty: Bytes32 = Bytes32(secret_bytes!([
    0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9, 0x24,
    0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52, 0xb8, 0x55
]));



// Algorithmns(ha, ae, sa, gn, psk_mode, zero_rtt)
#[derive(Clone, Copy, PartialEq)]
pub struct Algorithms(
    pub HashAlgorithm,
    pub AeadAlgorithm,
    pub SignatureScheme,
    pub KemScheme,
    pub bool,
    pub bool,
);

pub fn hash_alg(algs:&Algorithms) -> HashAlgorithm {algs.0}
pub fn aead_alg(algs:&Algorithms) -> AeadAlgorithm {algs.1}
pub fn sig_alg(algs:&Algorithms) -> SignatureScheme {algs.2}
pub fn kem_alg(algs:&Algorithms) -> KemScheme {algs.3}
pub fn psk_mode(algs:&Algorithms) -> bool {algs.4}
pub fn zero_rtt(algs:&Algorithms) -> bool {algs.5}

fn ciphersuite(algs: &Algorithms) -> Res<Bytes> {
    let Algorithms(ha, ae, sa, gn, psk_mode, zero_rtt) = algs;
    match (ha, ae) {
        (HashAlgorithm::SHA256, AeadAlgorithm::Aes128Gcm) => Ok(bytes2(0x13, 0x01)),
        (HashAlgorithm::SHA256, AeadAlgorithm::Chacha20Poly1305) => Ok(bytes2(0x13, 0x03)),
        _ => Err(unsupported_algorithm),
    }
}

fn supported_group(algs: &Algorithms) -> Res<Bytes> {
    let Algorithms(ha, ae, sa, gn, psk_mode, zero_rtt) = algs;
    match gn {
        NamedGroup::X25519 => Ok(bytes2(0x00, 0x1D)),
        NamedGroup::Secp256r1 => Ok(bytes2(0x00, 0x17)),
        NamedGroup::X448 => Err(unsupported_algorithm),
        NamedGroup::Secp384r1 => Err(unsupported_algorithm),
        NamedGroup::Secp521r1 => Err(unsupported_algorithm),
    }
}

fn signature_algorithm(algs: &Algorithms) -> Res<Bytes> {
    let Algorithms(ha, ae, sa, gn, psk_mode, zero_rtt) = algs;
    match sa {
        SignatureScheme::RsaPssRsaSha256 => Ok(bytes2(0x08, 0x04)),
        SignatureScheme::EcdsaSecp256r1Sha256 => Ok(bytes2(0x04, 0x03)),
        SignatureScheme::ED25519 => Err(unsupported_algorithm),
    }
}

fn check_ciphersuites(algs: &Algorithms, b: &ByteSeq) -> Res<usize> {
    let len = check_lbytes2(b)?;
    let cs = ciphersuite(algs)?;
    let csl = b.slice_range(2..2 + len);
    check_mem(&cs, &csl)?;
    Ok(len + 2)
}

fn server_name(sn: &ByteSeq) -> Res<Bytes> {
    Ok(bytes2(0, 0).concat(&lbytes2(&lbytes2(&bytes1(0).concat(&lbytes2(sn)?))?)?))
}

fn check_server_name(ext: &ByteSeq) -> Res<Bytes> {
    check_lbytes2_full(ext)?;
    check_eq(&bytes1(0), &ext.slice_range(2..3))?;
    check_lbytes2_full(&ext.slice_range(3..ext.len()))?;
    Ok(ext.slice_range(5..ext.len()))
}

fn supported_versions(algs: &Algorithms) -> Res<Bytes> {
    Ok(bytes2(0, 0x2b).concat(&lbytes2(&lbytes1(&bytes2(3, 4))?)?))
}

fn check_supported_versions(algs: &Algorithms, ch: &ByteSeq) -> Res<()> {
    check_lbytes1_full(ch)?;
    check_mem(&bytes2(3, 4), &ch.slice_range(1..ch.len()))
}

fn server_supported_version(algs: &Algorithms) -> Res<Bytes> {
    Ok(bytes2(0, 0x2b).concat(&lbytes2(&bytes2(3, 4))?))
}

fn check_server_supported_version(algs: &Algorithms, b: &ByteSeq) -> Res<()> {
    check_eq(&bytes2(3, 4), b)
}

fn supported_groups(algs: &Algorithms) -> Res<Bytes> {
    Ok(bytes2(0, 0x0a).concat(&lbytes2(&lbytes2(&supported_group(algs)?)?)?))
}

fn check_supported_groups(algs: &Algorithms, ch: &ByteSeq) -> Res<()> {
    check_lbytes2_full(ch)?;
    check_mem(&supported_group(algs)?, &ch.slice_range(2..ch.len()))
}

fn signature_algorithms(algs: &Algorithms) -> Res<Bytes> {
    Ok(bytes2(0, 0x0d).concat(&lbytes2(&lbytes2(&signature_algorithm(algs)?)?)?))
}

fn check_signature_algorithms(algs: &Algorithms, ch: &ByteSeq) -> Res<()> {
    check_lbytes2_full(ch)?;
    check_mem(&signature_algorithm(algs)?, &ch.slice_range(2..ch.len()))
}

pub fn psk_key_exchange_modes(algs: &Algorithms) -> Res<Bytes> {
    Ok(bytes2(0, 0x2d).concat(&lbytes2(&lbytes1(&bytes1(1))?)?))
}

pub fn check_psk_key_exchange_modes(algs: &Algorithms, ch: &ByteSeq) -> Res<()> {
    check_lbytes1_full(ch)?;
    check_eq(&bytes1(1), &ch.slice_range(1..2))
}

pub fn key_shares(algs: &Algorithms, gx: &KemPk) -> Res<Bytes> {
    let ks = supported_group(algs)?.concat(&lbytes2(&bytes(gx))?);
    Ok(bytes2(0, 0x33).concat(&lbytes2(&lbytes2(&ks)?)?))
}

pub fn check_key_share(algs: &Algorithms, ch: &ByteSeq) -> Res<Bytes> {
    check_lbytes2_full(ch)?;
    check_eq(&supported_group(algs)?, &ch.slice_range(2..4))?;
    check_lbytes2_full(&ch.slice_range(4..ch.len()))?;
    Ok(ch.slice_range(6..ch.len()))
}

pub fn server_key_shares(algs: &Algorithms, gx: &KemPk) -> Res<Bytes> {
    let ks = supported_group(algs)?.concat(&lbytes2(&bytes(gx))?);
    Ok(bytes2(0, 0x33).concat(&lbytes2(&ks)?))
}

pub fn check_server_key_share(algs: &Algorithms, b: &ByteSeq) -> Res<Bytes> {
    check_eq(&supported_group(algs)?, &b.slice_range(0..2))?;
    check_lbytes2_full(&b.slice_range(2..b.len()))?;
    Ok(b.slice_range(4..b.len()))
}

pub fn pre_shared_key(algs: &Algorithms, tkt: &ByteSeq) -> Res<(Bytes, usize)> {
    let Algorithms(ha, ae, sa, gn, psk_mode, zero_rtt) = algs;
    let identities = lbytes2(&lbytes2(tkt)?.concat(&U32_to_be_bytes(U32(0xffffffff))))?;
    let binders = lbytes2(&lbytes1(&zero_key(ha))?)?;
    let ext = bytes2(0, 41).concat(&lbytes2(&identities.concat(&binders))?);
    Ok((ext, binders.len()))
}

pub fn check_psk_shared_key(algs: &Algorithms, ch: &ByteSeq) -> Res<()> {
    let len_id = check_lbytes2(ch)?;
    let len_tkt = check_lbytes2(&ch.slice_range(2..2 + len_id))?;
    if len_id == len_tkt + 6 {
        check_lbytes2_full(&ch.slice_range(2 + len_id..ch.len()))?;
        check_lbytes1_full(&ch.slice_range(4 + len_id..ch.len()))?;
        if ch.len() - 6 - len_id != 32 {
            Err(parse_failed)
        } else {
            Ok(())
        }
    } else {
        Err(parse_failed)
    }
}

pub fn server_pre_shared_key(algs: &Algorithms) -> Res<Bytes> {
    Ok(bytes2(0, 41).concat(&lbytes2(&bytes2(0, 0))?))
}

pub fn check_server_psk_shared_key(algs: &Algorithms, b: &ByteSeq) -> Res<()> {
    check_eq(&bytes2(0, 0), &b)
}

pub struct EXTS(
    pub Option<Bytes>,
    pub Option<Bytes>,
    pub Option<Bytes>,
    pub Option<Bytes>,
);

pub fn merge_opts<T>(o1: Option<T>, o2: Option<T>) -> Res<Option<T>> {
    match (o1, o2) {
        (None, Some(o)) => Ok(Some(o)),
        (Some(o), None) => Ok(Some(o)),
        (None, None) => Ok(None),
        _ => Err(parse_failed),
    }
}
pub fn merge_exts(e1: EXTS, e2: EXTS) -> Res<EXTS> {
    let EXTS(sn1, ks1, tkt1, bd1) = e1;
    let EXTS(sn2, ks2, tkt2, bd2) = e2;
    Ok(EXTS(
        merge_opts(sn1, sn2)?,
        merge_opts(ks1, ks2)?,
        merge_opts(tkt1, tkt2)?,
        merge_opts(bd1, bd2)?,
    ))
}

fn check_extension(algs: &Algorithms, b: &ByteSeq) -> Res<(usize, EXTS)> {
    let l0 = (b[0] as U8).declassify() as usize;
    let l1 = (b[1] as U8).declassify() as usize;
    let len = check_lbytes2(&b.slice_range(2..b.len()))?;
    let mut out = EXTS(None, None, None, None);
    match (l0, l1) {
        (0, 0) => {
            let sn = check_server_name(&b.slice_range(4..4 + len))?;
            out = EXTS(Some(sn), None, None, None)
        }
        (0, 0x2d) => check_psk_key_exchange_modes(algs, &b.slice_range(4..4 + len))?,
        (0, 0x2b) => check_supported_versions(algs, &b.slice_range(4..4 + len))?,
        (0, 0x0a) => check_supported_groups(algs, &b.slice_range(4..4 + len))?,
        (0, 0x0d) => check_signature_algorithms(algs, &b.slice_range(4..4 + len))?,
        (0, 0x33) => {
            let gx = check_key_share(algs, &b.slice_range(4..4 + len))?;
            out = EXTS(None, Some(gx), None, None)
        }
        (0, 41) => check_psk_shared_key(algs, &b.slice_range(4..4 + len))?,
        _ => (),
    }
    Ok((4 + len, out))
}

pub fn check_server_extension(algs: &Algorithms, b: &ByteSeq) -> Res<(usize, Option<Bytes>)> {
    let l0 = (b[0] as U8).declassify() as usize;
    let l1 = (b[1] as U8).declassify() as usize;
    let len = check_lbytes2(&b.slice_range(2..b.len()))?;
    let mut out = None;
    match (l0, l1) {
        (0, 0x2b) => check_server_supported_version(algs, &b.slice_range(4..4 + len))?,
        (0, 0x33) => {
            let gx = check_server_key_share(algs, &b.slice_range(4..4 + len))?;
            out = Some(gx)
        }
        (0, 41) => check_server_psk_shared_key(algs, &b.slice_range(4..4 + len))?,
        _ => (),
    }
    Ok((4 + len, out))
}

fn check_extensions(algs: &Algorithms, b: &ByteSeq) -> Res<EXTS> {
    let (len, out) = check_extension(algs, b)?;
    if len == b.len() {
        Ok(out)
    } else {
        let out_rest = check_extensions(algs, &b.slice_range(len..b.len()))?;
        merge_exts(out, out_rest)
    }
}

pub fn check_server_extensions(algs: &Algorithms, b: &ByteSeq) -> Res<Option<Bytes>> {
    let (len, out) = check_server_extension(algs, b)?;
    if len == b.len() {
        Ok(out)
    } else {
        let out_rest = check_server_extensions(algs, &b.slice_range(len..b.len()))?;
        merge_opts(out, out_rest)
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum HandshakeType {
    ClientHello,
    ServerHello,
    NewSessionTicket,
    EndOfEarlyData,
    EncryptedExtensions,
    Certificate,
    CertificateRequest,
    CertificateVerify,
    Finished,
    KeyUpdate,
    MessageHash,
}

pub fn hs_type(t: HandshakeType) -> u8 {
    match t {
        HandshakeType::ClientHello => 1,
        HandshakeType::ServerHello => 2,
        HandshakeType::NewSessionTicket => 4,
        HandshakeType::EndOfEarlyData => 5,
        HandshakeType::EncryptedExtensions => 8,
        HandshakeType::Certificate => 11,
        HandshakeType::CertificateRequest => 13,
        HandshakeType::CertificateVerify => 15,
        HandshakeType::Finished => 20,
        HandshakeType::KeyUpdate => 24,
        HandshakeType::MessageHash => 254,
    }
}

pub fn get_hs_type(t: u8) -> Res<HandshakeType> {
    match t {
        1 => Ok(HandshakeType::ClientHello),
        2 => Ok(HandshakeType::ServerHello),
        4 => Ok(HandshakeType::NewSessionTicket),
        5 => Ok(HandshakeType::EndOfEarlyData),
        8 => Ok(HandshakeType::EncryptedExtensions),
        11 => Ok(HandshakeType::Certificate),
        13 => Ok(HandshakeType::CertificateRequest),
        15 => Ok(HandshakeType::CertificateVerify),
        20 => Ok(HandshakeType::Finished),
        24 => Ok(HandshakeType::KeyUpdate),
        254 => Ok(HandshakeType::MessageHash),
        _ => Err(parse_failed),
    }
}

// Tagged Handshake Data

pub struct HandshakeData(Bytes);

pub fn handshake_data(b:Bytes) -> HandshakeData{HandshakeData(b)}
pub fn handshake_data_bytes(hd:&HandshakeData) -> Bytes{hd.0.clone()}

pub fn handshake_data_len(p:&HandshakeData) -> usize {p.0.len()}

pub fn handshake_message(ty:HandshakeType,by:&ByteSeq) -> Res<HandshakeData> {
    Ok(HandshakeData(bytes1(hs_type(ty)).concat(&lbytes3(by)?)))
}

pub fn handshake_concat(msg1: HandshakeData, msg2: &HandshakeData) -> HandshakeData {
    let HandshakeData(m1) = msg1;
    let HandshakeData(m2) = msg2;
    HandshakeData(m1.concat(m2))
}

pub fn get_first_handshake_message(p: &HandshakeData) -> Res<(HandshakeData, HandshakeData)> {
    let HandshakeData(p) = p;
    if p.len() < 4 {
        Err(parse_failed)
    } else {
        let len = check_lbytes3(&p.slice_range(1..p.len()))?;
        let msg = p.slice_range(0..4+len);
        let rest = p.slice_range(4+len..p.len());
        Ok((HandshakeData(msg),HandshakeData(rest)))
    }
}

pub fn get_handshake_message(p:&HandshakeData) -> Res<(HandshakeData)> {
    let (m1,p) = get_first_handshake_message(p)?;
    if handshake_data_len(&p) != 0 {Err(parse_failed)}
    else {Ok((m1))}
}

pub fn get_handshake_message_ty(ty:HandshakeType, p:&HandshakeData) -> Res<(HandshakeData)> {
    let HandshakeData(m) = get_handshake_message(p)?;
    let tyb = bytes1(hs_type(ty));
    check_eq(&tyb, &m.slice_range(0..1))?;
    Ok(HandshakeData(m.slice_range(4..m.len())))
}

pub fn get_handshake_messages2(p:&HandshakeData) -> Res<(HandshakeData,HandshakeData)> {
    let (m1,p) = get_first_handshake_message(p)?;
    let (m2,p) = get_first_handshake_message(&p)?;
    if handshake_data_len(&p) != 0 {Err(parse_failed)}
    else {Ok((m1,m2))}
}
pub fn get_handshake_messages4(p:&HandshakeData) -> Res<(HandshakeData,HandshakeData,HandshakeData,HandshakeData)> {
    let (m1,p) = get_first_handshake_message(p)?;
    let (m2,p) = get_first_handshake_message(&p)?;
    let (m3,p) = get_first_handshake_message(&p)?;
    let (m4,p) = get_first_handshake_message(&p)?;
    if handshake_data_len(&p) != 0 {Err(parse_failed)}
    else {Ok((m1,m2,m3,m4))}
}


pub fn find_handshake_message(ty: HandshakeType, payload: &HandshakeData, start: usize) -> bool {
    let HandshakeData(p) = payload;
    if p.len() < start + 4 {
        false
    } else {
        match check_lbytes3(&p.slice_range(start+1..p.len())) {
            Err(_) => false,
            Ok(len) => {
                if eq1(p[start], U8(hs_type(ty))) {
                    true
                } else {
                    find_handshake_message(ty, payload, start + 4+ len)
                }
            }
        }
    }
}


pub fn client_hello(
    algs: &Algorithms,
    cr: &Random,
    gx: &KemPk,
    sn: &ByteSeq,
    tkt: &Option<Bytes>,
) -> Res<(HandshakeData, usize)> {
    let Algorithms(ha, ae, sa, gn, psk_mode, zero_rtt) = algs;
    let ver = bytes2(3, 3);
    let sid = lbytes1(&ByteSeq::new(32))?;
    let cip = lbytes2(&ciphersuite(algs)?)?;
    let comp = bytes2(1, 0);
    let sn = server_name(sn)?;
    let sv = supported_versions(algs)?;
    let sg = supported_groups(algs)?;
    let sa = signature_algorithms(algs)?;
    let ks = key_shares(algs, gx)?;
    let mut exts = sn.concat(&sv).concat(&sg).concat(&sa).concat(&ks);
    let mut trunc_len = 0;
    match (psk_mode, tkt) {
        (true, Some(tkt)) => {
            let pskm = psk_key_exchange_modes(algs)?;
            let (psk, len) = pre_shared_key(algs, tkt)?;
            exts = exts.concat(&pskm).concat(&psk);
            trunc_len = len;
        }
        (false, None) => {}
        _ => return Err(psk_mode_mismatch),
    }

    let ch = handshake_message(HandshakeType::ClientHello,
        &ver.concat(cr)
            .concat(&sid)
            .concat(&cip)
            .concat(&comp)
            .concat(&lbytes2(&exts)?),
    )?;
    Ok((ch, trunc_len))
}

pub fn set_client_hello_binder(
    algs: &Algorithms,
    binder: &Option<HMAC>,
    ch: HandshakeData,
    trunc_len: Option<usize>
) -> Res<HandshakeData> {
    let Algorithms(ha, ae, sa, gn, psk_mode, zero_rtt) = algs;
    let HandshakeData(ch) = ch;
    let chlen = &ch.len();
    match (binder,trunc_len) {
        (Some(m), Some(trunc_len)) => 
            if chlen - hash_len(ha) == trunc_len {
                Ok(HandshakeData(ch.update_slice(trunc_len,m,0,hash_len(ha))))
            } else {Err(parse_failed)}
        (None, None) => Ok(HandshakeData(ch)),
        (_,_) => Err(parse_failed)
    }
}

pub fn parse_client_hello(
    algs: &Algorithms,
    ch: &HandshakeData,
) -> Res<(
    Random,
    Bytes,
    Bytes,
    Bytes,
    Option<Bytes>,
    Option<Bytes>,
    usize,
)> {
    let HandshakeData(ch) = get_handshake_message_ty(HandshakeType::ClientHello,ch)?;
    let ver = bytes2(3, 3);
    let comp = bytes2(1, 0);
    let mut next = 0;
    check_eq(&ver, &ch.slice_range(next..next + 2))?;
    next = next + 2;
    let crand = ch.slice_range(next..next + 32);
    next = next + 32;
    let sidlen = check_lbytes1(&ch.slice_range(next..ch.len()))?;
    let sid = ch.slice_range(next + 1..next + 1 + sidlen);
    next = next + 1 + sidlen;
    let cslen = check_ciphersuites(algs, &ch.slice_range(next..ch.len()))?;
    next = next + cslen;
    check_eq(&comp, &ch.slice_range(next..next + 2))?;
    next = next + 2;
    check_lbytes2_full(&ch.slice_range(next..ch.len()))?;
    next = next + 2;
    let exts = check_extensions(algs, &ch.slice_range(next..ch.len()))?;
    let Algorithms(ha, ae, sa, gn, psk_mode, zero_rtt) = algs;
    let trunc_len = ch.len() - hash_len(ha) - 3;
    match (psk_mode, exts) {
        (true, EXTS(Some(sn), Some(gx), Some(tkt), Some(binder))) => Ok((
            Random::from_seq(&crand),
            sid,
            sn,
            gx,
            Some(tkt),
            Some(binder),
            trunc_len,
        )),
        (false, EXTS(Some(sn), Some(gx), None, None)) => {
            Ok((Random::from_seq(&crand), sid, sn, gx, None, None, 0))
        }
        _ => Err(parse_failed),
    }
}

pub fn server_hello(
    algs: &Algorithms,
    sr: &Random,
    sid: &ByteSeq,
    gy: &KemPk,
) -> Res<HandshakeData> {
    let Algorithms(ha, ae, sa, gn, psk_mode, zero_rtt) = algs;
    let ty = bytes1(hs_type(HandshakeType::ServerHello));
    let ver = bytes2(3, 3);
    let sid = lbytes1(sid)?;
    let cip = ciphersuite(algs)?;
    let comp = bytes1(0);
    let ks = server_key_shares(algs, gy)?;
    let sv = server_supported_version(algs)?;
    let mut exts = ks.concat(&sv);
    match psk_mode {
        true => exts = exts.concat(&server_pre_shared_key(algs)?),
        false => {}
    }
    let sh = handshake_message(HandshakeType::ServerHello,
        &ver.concat(sr)
            .concat(&sid)
            .concat(&cip)
            .concat(&comp)
            .concat(&lbytes2(&exts)?))?;
    Ok(sh)
}

pub fn parse_server_hello(algs: &Algorithms, sh: &HandshakeData) -> Res<(Random, KemPk)> {
    let HandshakeData(sh) = get_handshake_message_ty(HandshakeType::ServerHello,sh)?;
    let ver = bytes2(3, 3);
    let cip = ciphersuite(algs)?;
    let comp = bytes1(0);
    let mut next = 0;
    check_eq(&ver, &sh.slice_range(next..next + 2))?;
    next = next + 2;
    let srand = sh.slice_range(next..next + 32);
    next = next + 32;
    let sidlen = check_lbytes1(&sh.slice_range(next..sh.len()))?;
    next = next + 1 + sidlen;
    check_eq(&cip, &sh.slice_range(next..next + 2))?;
    next = next + 2;
    check_eq(&comp, &sh.slice_range(next..next + 1))?;
    next = next + 1;
    check_lbytes2_full(&sh.slice_range(next..sh.len()))?;
    next = next + 2;
    let gy = check_server_extensions(algs, &sh.slice_range(next..sh.len()))?;
    if let Some(gy) = gy {
        Ok((Random::from_seq(&srand), gy))
    } else {
        Err(unsupported_algorithm)
    }
}

pub fn encrypted_extensions(algs: &Algorithms) -> Res<HandshakeData> {
    let ty = bytes1(hs_type(HandshakeType::EncryptedExtensions));
    Ok(HandshakeData(ty.concat(&lbytes3(&empty())?)))
}

pub fn parse_encrypted_extensions(algs: &Algorithms, ee: &HandshakeData) -> Res<()> {
    let HandshakeData(ee) = ee;
    let ty = bytes1(hs_type(HandshakeType::EncryptedExtensions));
    check_eq(&ty, &ee.slice_range(0..1))?;
    check_lbytes3_full(&ee.slice_range(1..ee.len()))
}

pub fn server_certificate(algs: &Algorithms, cert: &ByteSeq) -> Res<HandshakeData> {
    let creq = lbytes1(&empty())?;
    let crt = lbytes3(cert)?;
    let ext = lbytes2(&empty())?;
    let crts = lbytes3(&crt.concat(&ext))?;
    Ok(handshake_message(HandshakeType::Certificate,&creq.concat(&crts))?)
}

pub fn parse_server_certificate(algs: &Algorithms, sc: &HandshakeData) -> Res<Bytes> {
    let HandshakeData(sc) = get_handshake_message_ty(HandshakeType::Certificate,sc)?;
    let mut next = 0;
    let creqlen = check_lbytes1(&sc.slice_range(4..sc.len()))?;
    next = next + 1 + creqlen;
    check_lbytes3_full(&sc.slice_range(next..sc.len()))?;
    next = next + 3;
    let crtlen = check_lbytes3(&sc.slice_range(next..sc.len()))?;
    next = next + 3;
    let crt = sc.slice_range(next..next + crtlen);
    next = next + crtlen;
    let extlen = check_lbytes2(&sc.slice_range(next..sc.len()))?;
    Ok(crt)
}

fn ecdsa_signature(sv: &ByteSeq) -> Res<Bytes> {
    if sv.len() != 64 {
        return Err(parse_failed);
    } else {
        let b0 = bytes1(0x0);
        let b1 = bytes1(0x30);
        let b2 = bytes1(0x02);
        let mut r: Seq<U8> = sv.slice(0, 32);
        let mut s: Seq<U8> = sv.slice(32, 32);
        if (r[0] as U8).declassify() >= 128 {
            r = b0.concat(&r);
        }
        if (s[0] as U8).declassify() >= 128 {
            s = b0.concat(&s);
        }
        Ok(b1.concat(&lbytes1(
            &b2.concat(&lbytes1(&r)?).concat(&b2).concat(&lbytes1(&s)?),
        )?))
    }
}

fn parse_ecdsa_signature(sig: Bytes) -> Res<Bytes> {
    if sig.len() < 4 {
        return Err(parse_failed);
    } else {
        check_eq(&bytes1(0x30), &sig.slice_range(0..1))?;
        check_lbytes1_full(&sig.slice_range(1..sig.len()))?;
        check_eq(&bytes1(0x02), &sig.slice_range(2..3))?;
        let rlen = check_lbytes1(&sig.slice_range(3..sig.len()))?;
        let r = sig.slice(4 + rlen - 32, 32);
        if sig.len() < 6 + rlen + 32 {
            return Err(parse_failed);
        } else {
            check_eq(&bytes1(0x02), &sig.slice_range(4 + rlen..5 + rlen))?;
            check_lbytes1_full(&sig.slice_range(5 + rlen..sig.len()))?;
            let s = sig.slice(sig.len() - 32, 32);
            return Ok(r.concat(&s));
        }
    }
}
pub fn certificate_verify(algs: &Algorithms, cv: &ByteSeq) -> Res<HandshakeData> {
    if cv.len() != 64 {
        Err(parse_failed)
    } else {
        let sv = ecdsa_signature(cv)?;
        let sig = signature_algorithm(algs)?.concat(&lbytes2(&sv)?);
        Ok(handshake_message(HandshakeType::CertificateVerify,&sig)?)
    }
}

pub fn parse_certificate_verify(algs: &Algorithms, cv: &HandshakeData) -> Res<Bytes> {
    let HandshakeData(cv) = get_handshake_message_ty(HandshakeType::CertificateVerify,cv)?;
    let sa = sig_alg(algs);
    check_eq(&signature_algorithm(algs)?, &cv.slice_range(0..2))?;
    check_lbytes2_full(&cv.slice_range(2..cv.len()))?;
    match sa {
        SignatureScheme::EcdsaSecp256r1Sha256 => parse_ecdsa_signature(cv.slice_range(4..cv.len())),
        SignatureScheme::RsaPssRsaSha256 => Ok(cv.slice_range(4..cv.len())),
        SignatureScheme::ED25519 => {
            if cv.len() - 4 == 64 {
                Ok(cv.slice_range(8..cv.len()))
            } else {
                Err(parse_failed)
            }
        }
    }
}

pub fn finished(algs: &Algorithms, vd: &ByteSeq) -> Res<HandshakeData> {
    Ok(handshake_message(HandshakeType::Finished,vd)?)
}

pub fn parse_finished(algs: &Algorithms, fin: &HandshakeData) -> Res<Bytes> {
    let HandshakeData(fin) = get_handshake_message_ty(HandshakeType::Finished,fin)?;
    Ok(fin)
}

pub fn session_ticket(algs: &Algorithms, tkt: &ByteSeq) -> Res<HandshakeData> {
    let lifetime = U32_to_be_bytes(U32(172800));
    let age = U32_to_be_bytes(U32(9999));
    let nonce = lbytes1(&bytes1(1))?;
    let stkt = lbytes2(&tkt)?;
    let grease_ext = bytes2(0x5a, 0x5a).concat(&lbytes2(&empty())?);
    let ext = lbytes2(&grease_ext)?;
    Ok(handshake_message(HandshakeType::NewSessionTicket,
            &lifetime
                .concat(&age)
                .concat(&nonce)
                .concat(&stkt)
                .concat(&grease_ext),
        )?)
}

pub fn parse_session_ticket(algs: &Algorithms, tkt: &HandshakeData) -> Res<(U32, Bytes)> {
    let HandshakeData(tkt) = get_handshake_message_ty(HandshakeType::NewSessionTicket,tkt)?;
    let lifetime = U32_from_be_bytes(U32Word::from_seq(&tkt.slice_range(0..4)));
    let age = U32_from_be_bytes(U32Word::from_seq(&tkt.slice_range(4..8)));
    let nonce_len = check_lbytes1(&tkt.slice_range(8..tkt.len()))?;
    let stkt_len = check_lbytes2(&tkt.slice_range(9 + nonce_len..tkt.len()))?;
    let stkt = tkt.slice_range(11 + nonce_len..11 + nonce_len + stkt_len);
    check_lbytes2_full(&tkt.slice_range(11 + nonce_len + stkt_len..tkt.len()))?;
    Ok((lifetime + age, stkt))
}

/* Record Layer Serialization and Parsing */
#[derive(Clone, Copy, PartialEq)]
pub enum ContentType {
    Invalid,
    ChangeCipherSpec,
    Alert,
    Handshake,
    ApplicationData,
}

pub fn content_type(t: ContentType) -> u8 {
    match t {
        ContentType::Invalid => 0,
        ContentType::ChangeCipherSpec => 20,
        ContentType::Alert => 21,
        ContentType::Handshake => 22,
        ContentType::ApplicationData => 23,
    }
}

pub fn get_content_type(t: u8) -> Res<ContentType> {
    match t {
        0 => Err(parse_failed),
        20 => Ok(ContentType::ChangeCipherSpec),
        21 => Ok(ContentType::Alert),
        22 => Ok(ContentType::Handshake),
        23 => Ok(ContentType::ApplicationData),
        _ => Err(parse_failed),
    }
}

pub fn handshake_record(p: &HandshakeData) -> Res<Bytes> {
    let HandshakeData(p) = p;
    let ty = bytes1(content_type(ContentType::Handshake));
    let ver = bytes2(3, 3);
    Ok(ty.concat(&ver).concat(&lbytes2(p)?))
}

pub fn check_handshake_record(p: &ByteSeq) -> Res<(HandshakeData, usize)> {
    if p.len() < 5 {
        Err(parse_failed)
    } else {
        let ty = bytes1(content_type(ContentType::Handshake));
        let ver = bytes2(3, 3);
        check_eq(&ty, &p.slice_range(0..1))?;
        check_eq(&ver, &p.slice_range(1..3))?;
        let len = check_lbytes2(&p.slice_range(3..p.len()))?;
        Ok((HandshakeData(p.slice_range(5..5 + len)), 5 + len))
    }
}

pub fn get_handshake_record(p:&ByteSeq) -> Res<HandshakeData> {
    let (hd,len) = check_handshake_record(p)?;
    if len == p.len() {
        Ok(hd)
    } else {Err(parse_failed)}
}

/* Incremental Transcript Construction
For simplicity, we store the full transcript, but an internal Digest state would suffice. */

pub struct Transcript(HashAlgorithm, HandshakeData);

pub fn transcript_empty(ha: HashAlgorithm) -> Transcript {
    Transcript(ha,HandshakeData(empty()))
}

pub fn transcript_add1(
    tx: Transcript,
    msg: &HandshakeData,
) -> Transcript {
    let Transcript(ha,tx) = tx;
    Transcript(ha,handshake_concat(tx,msg))
}

pub fn get_transcript_hash(
    tx: &Transcript
) -> Res<Digest> {
    let Transcript(ha,HandshakeData(txby)) = tx;
    let th = hash(ha,txby)?;
    Ok(th)
}

pub fn get_transcript_hash_truncated_client_hello(
    tx: &Transcript,
    ch: &HandshakeData,
    trunc_len: usize
) -> Res<Digest> {
    let Transcript(ha,HandshakeData(tx)) = tx;
    let HandshakeData(ch) = ch;
    Ok(hash(&ha, &tx.concat(&ch.slice_range(0..trunc_len)))?)
}