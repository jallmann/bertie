//! Zero-copy ASN.1 parser to read public keys from X.509 certificates.
//!
//! This parser is very limited and only for this specific use case.
//! It may or may not be extended to support more of the ASN.1 syntax.
//!
//! In particular do we parse the following structure:
//! ```ignore
//! Sequence { // 0x30 - 0x10 for sequence + 0x20 for constructed (XXX: Is this a must?)
//!     Sequence {
//!     }
//!     Sequence {
//!     }
//!     BitString // 0x03
//! }
//! ```

use crate::*;

/// Certificate key start and length within the certificate DER.
#[derive(Clone, Copy)]
pub struct CertificateKey(usize, usize);

pub type Asn1Error = u8;
pub const ASN1_SEQUENCE_TOO_LONG: Asn1Error = 21u8;
pub const ASN1_INVALID_TAG: Asn1Error = 22u8;
pub const ASN1_INVALID_CERTIFICATE: Asn1Error = 23u8;
pub const ASN1_UNSUPPORTED_ALGORITHM: Asn1Error = 24u8;
pub const ASN1_ERROR: Asn1Error = 25u8;

type UsizeResult = Result<usize, Asn1Error>;
type DoubleUsizeResult = Result<(usize, usize), Asn1Error>;
type SpkiResult = Result<Spki, Asn1Error>;
type PkResult = Result<PublicVerificationKey, Asn1Error>;
pub type VerificationKeyResult = Result<VerificationKey, Asn1Error>;
pub type RsaVerificationKeyResult = Result<RsaVerificationKey, Asn1Error>;

pub fn asn1err<T>(err: Asn1Error) -> Result<T, Asn1Error> {
    let bt = backtrace::Backtrace::new();
    println!("{:?}", bt);
    Err(err)
}

// Long form length
// * Must be used when the length is 128 or greater
// * XXX: We do not accept lengths greater than 32-bit.
fn long_length(b: &Bytes, offset: usize, len: usize) -> UsizeResult {
    if len > 4 {
        asn1err(ASN1_SEQUENCE_TOO_LONG)
    } else {
        let mut u32word: Bytes = Bytes::zeroes(4);
        u32word[0..len].copy_from_slice(&b[offset..offset + len]);
        UsizeResult::Ok(U32::from_be_bytes(&u32word)?.declassify() as usize >> ((4 - len) * 8))
    }
}

// Read the length of a long form length
fn length_length(b: &Bytes, offset: usize) -> usize {
    if b[offset].declassify() >> 7 == 1u8 {
        // Only in this case we have a length length.
        (b[offset].declassify() & 0x7fu8) as usize
    } else {
        0
    }
}

// Short form length
// * Must be used when the length is between 0 and 127
// * The byte must start with a 0 bit, the following 7 bits are the length.
fn short_length(b: &Bytes, offset: usize) -> UsizeResult {
    if b[offset].declassify() & 0x80u8 == 0u8 {
        UsizeResult::Ok((b[offset].declassify() & 0x7fu8) as usize)
    } else {
        asn1err(ASN1_ERROR)
    }
}

/// Get the length of an ASN.1 type.
/// This assumes that the length starts at the beginning of the provided byte
/// sequence.
///
/// Returns: (offset, length)
fn length(b: &Bytes, mut offset: usize) -> DoubleUsizeResult {
    if b[offset].declassify() & 0x80 == 0u8 {
        let len = short_length(b, offset)?;
        DoubleUsizeResult::Ok((offset + 1, len))
    } else {
        let len = length_length(b, offset);
        offset = offset + 1;
        let end = long_length(b, offset, len)?;
        DoubleUsizeResult::Ok((offset + len, end))
    }
}

/// Read a byte sequence from the provided bytes.
///
/// Returns the new offset into the bytes.
fn read_sequence_header(b: &Bytes, mut offset: usize) -> UsizeResult {
    check_tag(b, offset, 0x30u8)?;
    offset = offset + 1;

    let length_length = length_length(b, offset);
    offset = offset + length_length + 1; // 1 byte is always used for length

    UsizeResult::Ok(offset)
}

fn check_tag(b: &Bytes, offset: usize, value: u8) -> Result<(), Asn1Error> {
    if b[offset].declassify() == value {
        Result::<(), Asn1Error>::Ok(())
    } else {
        // println!("Got tag {:x}, expected {:x}", b[offset], value);
        asn1err(ASN1_INVALID_TAG)
    }
}

/// Skip a sequence.
/// XXX: Share code with [read_sequence_header].
///
/// Returns the new offset into the bytes.
fn skip_sequence(b: &Bytes, mut offset: usize) -> UsizeResult {
    check_tag(b, offset, 0x30u8)?;
    offset = offset + 1;

    let (offset, length) = length(b, offset)?;

    UsizeResult::Ok(offset + length)
}

/// Read the version number.
/// We don't really care, just check that it's some valid structure and keep the
/// offset moving.
///
/// Note that this might be missing. So we don't fail in here.
fn read_version_number(b: &Bytes, mut offset: usize) -> UsizeResult {
    match check_tag(b, offset, 0xA0u8) {
        Ok(_) => {
            offset = offset + 1;

            let length = short_length(b, offset)?;
            UsizeResult::Ok(offset + 1 + length)
        }
        Err(_) => UsizeResult::Ok(offset),
    }
}

/// Read an integer.
/// We don't really care, just check that it's some valid structure and keep the
/// offset moving.
fn read_integer(b: &Bytes, mut offset: usize) -> UsizeResult {
    check_tag(b, offset, 0x02u8)?;
    offset = offset + 1;

    let (offset, length) = length(b, offset)?;
    UsizeResult::Ok(offset + length)
}

pub type Spki = (SignatureScheme, CertificateKey);

pub fn x962_ec_public_key_oid() -> Bytes {
    [0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x02, 0x01].into()
}
pub fn ecdsa_secp256r1_sha256_oid() -> Bytes {
    [0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x03, 0x01, 0x07].into()
}
pub fn rsa_pkcs1_encryption_oid() -> Bytes {
    [0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x01].into()
}

fn check_success(val: bool) -> Result<(), Asn1Error> {
    if val {
        Result::<(), Asn1Error>::Ok(())
    } else {
        asn1err(ASN1_ERROR)
    }
}

fn read_spki(cert: &Bytes, mut offset: usize) -> SpkiResult {
    check_tag(cert, offset, 0x30u8)?;
    offset = offset + 1;

    let (mut offset, _seq_len) = length(cert, offset)?;

    // The algorithm name is another sequence
    check_tag(cert, offset, 0x30u8)?;
    offset = offset + 1;
    let (mut offset, seq_len) = length(cert, offset)?;
    // OID Tag
    check_tag(cert, offset, 0x06u8)?;
    // OID length
    let (mut oid_offset, oid_len) = length(cert, offset + 1)?;
    let (mut ec_pk_oid, mut ecdsa_p256, mut rsa_pk_oid) = (false, false, false);
    let ec_oid = x962_ec_public_key_oid();
    let rsa_oid = rsa_pkcs1_encryption_oid();
    // Check OID
    if ec_oid.len() == oid_len {
        ec_pk_oid = true;
        for i in 0..ec_oid.len() {
            let oid_byte_equal = cert[oid_offset + i].declassify() == ec_oid[i].declassify();
            ec_pk_oid = ec_pk_oid && oid_byte_equal;
        }
        if ec_pk_oid {
            oid_offset = oid_offset + oid_len;
            check_tag(cert, oid_offset, 0x06u8)?;
            oid_offset = oid_offset + 1;
            let (oid_offset, _oid_len) = length(cert, oid_offset)?;
            ecdsa_p256 = true;
            // In this case we also need to read the curve OID.
            let ec_oid = ecdsa_secp256r1_sha256_oid();
            for i in 0..ec_oid.len() {
                let oid_byte_equal = cert[oid_offset + i].declassify() == ec_oid[i].declassify();
                ecdsa_p256 = ecdsa_p256 && oid_byte_equal;
            }
            check_success(ecdsa_p256)?;
        }
    }
    if rsa_oid.len() == oid_len {
        rsa_pk_oid = true;
        for i in 0..rsa_oid.len() {
            let oid_byte_equal = cert[oid_offset + i].declassify() == rsa_oid[i].declassify();
            rsa_pk_oid = rsa_pk_oid && oid_byte_equal;
        }
    }
    check_success((ec_pk_oid && ecdsa_p256) || rsa_pk_oid)?;

    // Skip all the way to the end of the sequence.
    // RSA has a NULL element in there as well. We don't care.
    offset = offset + seq_len;

    // The public key is now a bit string. Let's find the start of the actual
    // DER structure in there.
    check_tag(cert, offset, 0x03u8)?;
    offset = offset + 1;
    let (mut offset, bit_string_len) = length(cert, offset)?;
    if cert[offset].declassify() == 0x00 {
        offset = offset + 1; // There's a 0x00 at the end of the length
    }

    if ec_pk_oid && ecdsa_p256 {
        SpkiResult::Ok((
            SignatureScheme::EcdsaSecp256r1Sha256,
            CertificateKey(offset, bit_string_len - 1),
        ))
    } else {
        if rsa_pk_oid {
            SpkiResult::Ok((
                SignatureScheme::RsaPssRsaSha256,
                CertificateKey(offset, bit_string_len - 1),
            ))
        } else {
            asn1err(ASN1_INVALID_CERTIFICATE)
        }
    }
}

/// Basic, but complete ASN.1 parser to read the public key from an X.509
/// certificate.
///
/// Returns the start offset within the `cert` bytes and length of the key.
pub fn verification_key_from_cert(cert: &Bytes) -> SpkiResult {
    // An x509 cert is an ASN.1 sequence of [Certificate, SignatureAlgorithm, Signature].
    // Take the first sequence inside the outer because we're interested in the
    // certificate

    let mut offset = read_sequence_header(cert, 0)?;
    offset = read_sequence_header(cert, offset)?;

    // Now we're inside the first sequence, the certificate.
    offset = read_version_number(cert, offset)?; // x509 version number
    offset = read_integer(cert, offset)?; // serial number
    offset = skip_sequence(cert, offset)?; // signature algorithm
    offset = skip_sequence(cert, offset)?; // issuer
    offset = skip_sequence(cert, offset)?; // validity
    offset = skip_sequence(cert, offset)?; // subject

    // Now there's the SPKI that we're actually interested in.
    read_spki(cert, offset)
}

/// Read the EC PK from the cert as uncompressed point.
pub fn ecdsa_public_key(cert: &Bytes, indices: CertificateKey) -> VerificationKeyResult {
    let CertificateKey(offset, len) = indices;

    check_tag(cert, offset, 0x04u8)?; // We only support uncompressed

    // Return the uncompressed point
    VerificationKeyResult::Ok(cert.slice(offset + 1, len - 1)) // Drop the 0x04 here.
}

pub fn rsa_public_key(cert: &Bytes, indices: CertificateKey) -> RsaVerificationKeyResult {
    let CertificateKey(mut offset, _len) = indices;

    // An RSA PK is a sequence of modulus N and public exponent e,
    // each encoded as integer.
    check_tag(cert, offset, 0x30u8)?;
    offset = offset + 1;
    let (mut offset, _seq_len) = length(cert, offset)?;

    // Integer: N
    check_tag(cert, offset, 0x02u8)?;
    offset = offset + 1;
    let (mut offset, int_len) = length(cert, offset)?;
    let n = cert.slice(offset, int_len);
    offset = offset + int_len;

    // Integer: e
    check_tag(cert, offset, 0x02u8)?;
    offset = offset + 1;
    let (offset, int_len) = length(cert, offset)?;
    let e = cert.slice(offset, int_len);

    RsaVerificationKeyResult::Ok((n, e))
}

pub fn cert_public_key(cert: &Bytes, spki: &Spki) -> PkResult {
    match spki.0 {
        SignatureScheme::ED25519 => asn1err(ASN1_UNSUPPORTED_ALGORITHM),
        SignatureScheme::EcdsaSecp256r1Sha256 => {
            let pk = ecdsa_public_key(cert, spki.1)?;
            PkResult::Ok(PublicVerificationKey::EcDsa(pk))
        }
        SignatureScheme::RsaPssRsaSha256 => {
            let pk = rsa_public_key(cert, spki.1)?;
            PkResult::Ok(PublicVerificationKey::Rsa(pk))
        }
    }
}

#[cfg(test)]
mod unit_test {
    use std::{fs, io::Read};

    use super::*;

    fn test(cert: &Bytes) {
        let spki = verification_key_from_cert(&cert);
        match spki {
            Ok(spki) => {
                let pk = cert_public_key(cert, &spki).expect("Error reading public key from cert");
                println!("Got pk {:?}", pk);
            }
            Err(e) => {
                println!("verif key extraction error {}", e);
                None.unwrap()
            }
        }
    }

    #[test]
    fn ecdsa_cert() {
        let cert = CLOUDFLARE_COM_DER.into();
        test(&cert);
        test(&OTHER_ECDSA_P256_SHA256_CERT.into());
    }

    #[test]
    fn rsa_cert() {
        let cert = GOO_GL_DER.into();
        test(&cert);
    }

    #[test]
    fn read_cert() {
        let files = fs::read_dir("test_certs").expect("Error listing files.");
        for file in files {
            let file = file.expect("Error reading file ...").path();
            let mut f = fs::File::open(file.clone())
                .expect(&format!("Didn't find the file {}.", file.display()));
            let mut bytes = Vec::new();
            f.read_to_end(&mut bytes)
                .expect(&format!("Error reading file {}", file.display()));
            test(&bytes.into());
        }
    }

    // RSA cert
    const GOO_GL_DER: [u8; 896] = [
        0x30u8, 0x82, 0x03, 0x7c, 0x30, 0x82, 0x02, 0x64, 0xa0, 0x03, 0x02, 0x01, 0x02, 0x02, 0x09,
        0x00, 0x90, 0x76, 0x89, 0x18, 0xe9, 0x33, 0x93, 0xa0, 0x30, 0x0d, 0x06, 0x09, 0x2a, 0x86,
        0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b, 0x05, 0x00, 0x30, 0x4e, 0x31, 0x31, 0x30, 0x2f,
        0x06, 0x03, 0x55, 0x04, 0x0b, 0x0c, 0x28, 0x4e, 0x6f, 0x20, 0x53, 0x4e, 0x49, 0x20, 0x70,
        0x72, 0x6f, 0x76, 0x69, 0x64, 0x65, 0x64, 0x3b, 0x20, 0x70, 0x6c, 0x65, 0x61, 0x73, 0x65,
        0x20, 0x66, 0x69, 0x78, 0x20, 0x79, 0x6f, 0x75, 0x72, 0x20, 0x63, 0x6c, 0x69, 0x65, 0x6e,
        0x74, 0x2e, 0x31, 0x19, 0x30, 0x17, 0x06, 0x03, 0x55, 0x04, 0x03, 0x13, 0x10, 0x69, 0x6e,
        0x76, 0x61, 0x6c, 0x69, 0x64, 0x32, 0x2e, 0x69, 0x6e, 0x76, 0x61, 0x6c, 0x69, 0x64, 0x30,
        0x1e, 0x17, 0x0d, 0x31, 0x35, 0x30, 0x31, 0x30, 0x31, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30,
        0x5a, 0x17, 0x0d, 0x33, 0x30, 0x30, 0x31, 0x30, 0x31, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30,
        0x5a, 0x30, 0x4e, 0x31, 0x31, 0x30, 0x2f, 0x06, 0x03, 0x55, 0x04, 0x0b, 0x0c, 0x28, 0x4e,
        0x6f, 0x20, 0x53, 0x4e, 0x49, 0x20, 0x70, 0x72, 0x6f, 0x76, 0x69, 0x64, 0x65, 0x64, 0x3b,
        0x20, 0x70, 0x6c, 0x65, 0x61, 0x73, 0x65, 0x20, 0x66, 0x69, 0x78, 0x20, 0x79, 0x6f, 0x75,
        0x72, 0x20, 0x63, 0x6c, 0x69, 0x65, 0x6e, 0x74, 0x2e, 0x31, 0x19, 0x30, 0x17, 0x06, 0x03,
        0x55, 0x04, 0x03, 0x13, 0x10, 0x69, 0x6e, 0x76, 0x61, 0x6c, 0x69, 0x64, 0x32, 0x2e, 0x69,
        0x6e, 0x76, 0x61, 0x6c, 0x69, 0x64, 0x30, 0x82, 0x01, 0x22, 0x30, 0x0d, 0x06, 0x09, 0x2a,
        0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01, 0x05, 0x00, 0x03, 0x82, 0x01, 0x0f, 0x00,
        0x30, 0x82, 0x01, 0x0a, 0x02, 0x82, 0x01, 0x01, 0x00, 0xcd, 0x62, 0x4f, 0xe5, 0xc3, 0x13,
        0x84, 0x98, 0x0c, 0x05, 0xe4, 0xef, 0x44, 0xa2, 0xa5, 0xec, 0xde, 0x99, 0x71, 0x90, 0x1b,
        0x28, 0x35, 0x40, 0xb4, 0xd0, 0x4d, 0x9d, 0x18, 0x48, 0x81, 0x28, 0xad, 0x5f, 0x10, 0xb3,
        0x2a, 0xdb, 0x7d, 0xae, 0x9d, 0x91, 0x1e, 0x42, 0xe7, 0xef, 0xaa, 0x19, 0x8d, 0xd3, 0x4e,
        0xdb, 0x91, 0x0f, 0xa7, 0xe4, 0x20, 0x32, 0x25, 0x94, 0xfe, 0xb9, 0x24, 0x07, 0x4d, 0x18,
        0xd7, 0xc3, 0x9a, 0x87, 0x0e, 0x5f, 0x8b, 0xcb, 0x3e, 0x2b, 0xd7, 0x51, 0xbf, 0xa8, 0xbe,
        0x81, 0x23, 0xa2, 0xbf, 0x68, 0xe5, 0x21, 0xe5, 0xbf, 0x4b, 0x48, 0x4e, 0xb3, 0x05, 0x14,
        0x0c, 0x7d, 0x09, 0x5c, 0x59, 0x04, 0x3c, 0xa2, 0x0b, 0xce, 0x99, 0x79, 0x30, 0xbe, 0xf0,
        0x76, 0x9e, 0x64, 0xb7, 0xdd, 0xef, 0x1f, 0x16, 0xbb, 0x1e, 0xcc, 0x0e, 0xb4, 0x0c, 0x44,
        0xcf, 0x65, 0xad, 0xc4, 0xc7, 0x5e, 0xce, 0x6f, 0xf7, 0x0a, 0x03, 0xb7, 0xb2, 0x5b, 0x36,
        0xd3, 0x09, 0x77, 0x5b, 0x4d, 0xe2, 0x23, 0xe9, 0x02, 0xb7, 0xb1, 0xf2, 0xbe, 0x11, 0xb2,
        0xd9, 0xa4, 0x4f, 0x2e, 0x12, 0x5f, 0x78, 0x00, 0x69, 0x42, 0xbd, 0x14, 0x92, 0xed, 0xea,
        0xea, 0x6b, 0x68, 0x9b, 0x2d, 0x9c, 0x80, 0x56, 0xb0, 0x7a, 0x43, 0x7f, 0x5f, 0xf6, 0x87,
        0xf0, 0xa9, 0x27, 0x5f, 0xbf, 0x7d, 0x30, 0xf7, 0x2e, 0x5a, 0xeb, 0x4c, 0xda, 0xaf, 0x3c,
        0x9a, 0xd5, 0x04, 0x06, 0xcb, 0x99, 0x9b, 0x2d, 0xa7, 0xb2, 0x32, 0xbd, 0x27, 0xbf, 0xf2,
        0x86, 0x10, 0x91, 0x0f, 0x33, 0x95, 0xff, 0x26, 0x3c, 0x73, 0x9f, 0xa5, 0xfe, 0xef, 0xeb,
        0x5a, 0xec, 0x30, 0x91, 0x9d, 0xa5, 0x83, 0x31, 0xa9, 0xe3, 0x10, 0x41, 0x7e, 0x15, 0xdd,
        0xaf, 0xaf, 0xa6, 0xf6, 0x49, 0xb0, 0x58, 0x25, 0x26, 0xf5, 0x02, 0x03, 0x01, 0x00, 0x01,
        0xa3, 0x5d, 0x30, 0x5b, 0x30, 0x0e, 0x06, 0x03, 0x55, 0x1d, 0x0f, 0x01, 0x01, 0xff, 0x04,
        0x04, 0x03, 0x02, 0x02, 0xa4, 0x30, 0x1d, 0x06, 0x03, 0x55, 0x1d, 0x25, 0x04, 0x16, 0x30,
        0x14, 0x06, 0x08, 0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x01, 0x06, 0x08, 0x2b, 0x06,
        0x01, 0x05, 0x05, 0x07, 0x03, 0x02, 0x30, 0x0f, 0x06, 0x03, 0x55, 0x1d, 0x13, 0x01, 0x01,
        0xff, 0x04, 0x05, 0x30, 0x03, 0x01, 0x01, 0xff, 0x30, 0x19, 0x06, 0x03, 0x55, 0x1d, 0x0e,
        0x04, 0x12, 0x04, 0x10, 0xbb, 0x0f, 0x38, 0x96, 0x6f, 0x3e, 0xbe, 0x4f, 0x2b, 0x46, 0xd0,
        0x41, 0x6a, 0xd4, 0xac, 0xb5, 0x30, 0x0d, 0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d,
        0x01, 0x01, 0x0b, 0x05, 0x00, 0x03, 0x82, 0x01, 0x01, 0x00, 0xb9, 0xd9, 0xe2, 0x54, 0x5c,
        0xf5, 0x61, 0xed, 0x69, 0xf3, 0xb8, 0x63, 0xed, 0x03, 0x5a, 0x9e, 0x2a, 0x81, 0x27, 0x5a,
        0x1b, 0x28, 0x33, 0x4b, 0xfc, 0x2d, 0x71, 0x13, 0xfe, 0x4b, 0x65, 0x7e, 0x1c, 0x53, 0x82,
        0x79, 0x80, 0xe6, 0x79, 0x9f, 0x6a, 0xb3, 0x45, 0xa9, 0x36, 0x5a, 0xed, 0xc9, 0xe0, 0x4a,
        0xcc, 0x11, 0xfc, 0x84, 0xeb, 0x7d, 0xcb, 0xc6, 0x94, 0x6d, 0x90, 0x70, 0xd8, 0xcd, 0x45,
        0xd8, 0xc8, 0xb6, 0xdd, 0x0f, 0x9d, 0x84, 0x01, 0x14, 0x7d, 0x00, 0x8e, 0x29, 0xb2, 0x13,
        0xb6, 0xe9, 0xc1, 0xb9, 0x57, 0xc3, 0x4d, 0x36, 0xc0, 0x1d, 0x4b, 0x8d, 0x97, 0xf7, 0xb2,
        0xaf, 0xbf, 0x2f, 0xf0, 0x48, 0x22, 0xd7, 0x7d, 0xf3, 0xef, 0x35, 0x60, 0xc9, 0xd5, 0x46,
        0xd4, 0xa0, 0x34, 0x00, 0xe4, 0x82, 0x07, 0xe0, 0x7a, 0xe6, 0x09, 0x5b, 0xa7, 0x1f, 0xb1,
        0x30, 0x2a, 0x60, 0x64, 0xbb, 0xb1, 0xf5, 0x31, 0xf2, 0x77, 0x08, 0x37, 0xb4, 0xfa, 0x3f,
        0x2d, 0xf6, 0x1b, 0x44, 0x2a, 0x1f, 0xf8, 0xc6, 0xfc, 0x23, 0x76, 0x42, 0x63, 0xd3, 0xba,
        0x15, 0xf6, 0x46, 0x8e, 0xec, 0x49, 0x9f, 0xed, 0x2e, 0xc7, 0x74, 0x83, 0xa2, 0xb6, 0xb7,
        0x35, 0x7f, 0xc5, 0x98, 0x9f, 0xa2, 0x91, 0x30, 0x93, 0xb0, 0xcb, 0x48, 0x15, 0x68, 0x47,
        0xde, 0x1a, 0x32, 0x60, 0x06, 0xa6, 0x38, 0xeb, 0x88, 0x4e, 0x93, 0xd9, 0x1c, 0x3e, 0xf2,
        0x3f, 0x49, 0x5f, 0x6e, 0xe9, 0xdc, 0x18, 0x31, 0x2a, 0x01, 0x0b, 0xb6, 0x61, 0x66, 0xd8,
        0xc5, 0x18, 0xb1, 0x7e, 0xad, 0x95, 0x4b, 0x18, 0x2f, 0x81, 0x66, 0xc5, 0x72, 0x69, 0x20,
        0x04, 0xb6, 0x29, 0x13, 0xc8, 0x83, 0x59, 0x3d, 0xca, 0x76, 0x5b, 0xa8, 0xd7, 0xee, 0x8f,
        0x1d, 0xa0, 0xda, 0x2e, 0x0d, 0x92, 0x69, 0xc3, 0x98, 0xe8, 0x6a,
    ];
    // ECDSA cert
    const CLOUDFLARE_COM_DER: [u8; 1385] = [
        0x30, 0x82, 0x05, 0x65, 0x30, 0x82, 0x05, 0x0a, 0xa0, 0x03, 0x02, 0x01, 0x02, 0x02, 0x10,
        0x06, 0x40, 0x7b, 0x70, 0xe1, 0x45, 0x6a, 0xb0, 0xe2, 0xa5, 0x89, 0x0e, 0xfd, 0x75, 0xd1,
        0xe5, 0x30, 0x0a, 0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02, 0x30, 0x4a,
        0x31, 0x0b, 0x30, 0x09, 0x06, 0x03, 0x55, 0x04, 0x06, 0x13, 0x02, 0x55, 0x53, 0x31, 0x19,
        0x30, 0x17, 0x06, 0x03, 0x55, 0x04, 0x0a, 0x13, 0x10, 0x43, 0x6c, 0x6f, 0x75, 0x64, 0x66,
        0x6c, 0x61, 0x72, 0x65, 0x2c, 0x20, 0x49, 0x6e, 0x63, 0x2e, 0x31, 0x20, 0x30, 0x1e, 0x06,
        0x03, 0x55, 0x04, 0x03, 0x13, 0x17, 0x43, 0x6c, 0x6f, 0x75, 0x64, 0x66, 0x6c, 0x61, 0x72,
        0x65, 0x20, 0x49, 0x6e, 0x63, 0x20, 0x45, 0x43, 0x43, 0x20, 0x43, 0x41, 0x2d, 0x33, 0x30,
        0x1e, 0x17, 0x0d, 0x32, 0x32, 0x30, 0x35, 0x30, 0x34, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30,
        0x5a, 0x17, 0x0d, 0x32, 0x33, 0x30, 0x35, 0x30, 0x34, 0x32, 0x33, 0x35, 0x39, 0x35, 0x39,
        0x5a, 0x30, 0x6e, 0x31, 0x0b, 0x30, 0x09, 0x06, 0x03, 0x55, 0x04, 0x06, 0x13, 0x02, 0x55,
        0x53, 0x31, 0x13, 0x30, 0x11, 0x06, 0x03, 0x55, 0x04, 0x08, 0x13, 0x0a, 0x43, 0x61, 0x6c,
        0x69, 0x66, 0x6f, 0x72, 0x6e, 0x69, 0x61, 0x31, 0x16, 0x30, 0x14, 0x06, 0x03, 0x55, 0x04,
        0x07, 0x13, 0x0d, 0x53, 0x61, 0x6e, 0x20, 0x46, 0x72, 0x61, 0x6e, 0x63, 0x69, 0x73, 0x63,
        0x6f, 0x31, 0x19, 0x30, 0x17, 0x06, 0x03, 0x55, 0x04, 0x0a, 0x13, 0x10, 0x43, 0x6c, 0x6f,
        0x75, 0x64, 0x66, 0x6c, 0x61, 0x72, 0x65, 0x2c, 0x20, 0x49, 0x6e, 0x63, 0x2e, 0x31, 0x17,
        0x30, 0x15, 0x06, 0x03, 0x55, 0x04, 0x03, 0x13, 0x0e, 0x63, 0x6c, 0x6f, 0x75, 0x64, 0x66,
        0x6c, 0x61, 0x72, 0x65, 0x2e, 0x63, 0x6f, 0x6d, 0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a,
        0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01,
        0x07, 0x03, 0x42, 0x00, 0x04, 0x99, 0xf3, 0x6d, 0xdd, 0x6b, 0xad, 0x71, 0xb7, 0x89, 0x96,
        0xdc, 0xed, 0xf6, 0x5e, 0x4f, 0x4d, 0x03, 0xd3, 0xe9, 0xc3, 0x18, 0xcf, 0x68, 0xe2, 0x6d,
        0x80, 0x1b, 0x1e, 0xaa, 0xdb, 0x91, 0x4a, 0xb6, 0xa8, 0xf2, 0xec, 0x9a, 0x8e, 0xf8, 0xa3,
        0x4a, 0x60, 0x9d, 0xb7, 0x47, 0xd7, 0x41, 0xac, 0xd9, 0x11, 0x1f, 0x8f, 0x58, 0xc2, 0x6a,
        0x80, 0x2e, 0x84, 0x8c, 0xf5, 0x0f, 0x3e, 0x2c, 0xfe, 0xa3, 0x82, 0x03, 0xac, 0x30, 0x82,
        0x03, 0xa8, 0x30, 0x1f, 0x06, 0x03, 0x55, 0x1d, 0x23, 0x04, 0x18, 0x30, 0x16, 0x80, 0x14,
        0xa5, 0xce, 0x37, 0xea, 0xeb, 0xb0, 0x75, 0x0e, 0x94, 0x67, 0x88, 0xb4, 0x45, 0xfa, 0xd9,
        0x24, 0x10, 0x87, 0x96, 0x1f, 0x30, 0x1d, 0x06, 0x03, 0x55, 0x1d, 0x0e, 0x04, 0x16, 0x04,
        0x14, 0xf2, 0x21, 0x1f, 0x0c, 0x78, 0xfa, 0xf3, 0x5a, 0x72, 0x30, 0x41, 0x0d, 0x26, 0x67,
        0xf3, 0xaa, 0x62, 0x72, 0xf7, 0x72, 0x30, 0x71, 0x06, 0x03, 0x55, 0x1d, 0x11, 0x04, 0x6a,
        0x30, 0x68, 0x82, 0x18, 0x2a, 0x2e, 0x73, 0x74, 0x61, 0x67, 0x69, 0x6e, 0x67, 0x2e, 0x63,
        0x6c, 0x6f, 0x75, 0x64, 0x66, 0x6c, 0x61, 0x72, 0x65, 0x2e, 0x63, 0x6f, 0x6d, 0x82, 0x10,
        0x2a, 0x2e, 0x63, 0x6c, 0x6f, 0x75, 0x64, 0x66, 0x6c, 0x61, 0x72, 0x65, 0x2e, 0x63, 0x6f,
        0x6d, 0x82, 0x14, 0x2a, 0x2e, 0x61, 0x6d, 0x70, 0x2e, 0x63, 0x6c, 0x6f, 0x75, 0x64, 0x66,
        0x6c, 0x61, 0x72, 0x65, 0x2e, 0x63, 0x6f, 0x6d, 0x82, 0x0e, 0x63, 0x6c, 0x6f, 0x75, 0x64,
        0x66, 0x6c, 0x61, 0x72, 0x65, 0x2e, 0x63, 0x6f, 0x6d, 0x82, 0x14, 0x2a, 0x2e, 0x64, 0x6e,
        0x73, 0x2e, 0x63, 0x6c, 0x6f, 0x75, 0x64, 0x66, 0x6c, 0x61, 0x72, 0x65, 0x2e, 0x63, 0x6f,
        0x6d, 0x30, 0x0e, 0x06, 0x03, 0x55, 0x1d, 0x0f, 0x01, 0x01, 0xff, 0x04, 0x04, 0x03, 0x02,
        0x07, 0x80, 0x30, 0x1d, 0x06, 0x03, 0x55, 0x1d, 0x25, 0x04, 0x16, 0x30, 0x14, 0x06, 0x08,
        0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x01, 0x06, 0x08, 0x2b, 0x06, 0x01, 0x05, 0x05,
        0x07, 0x03, 0x02, 0x30, 0x7b, 0x06, 0x03, 0x55, 0x1d, 0x1f, 0x04, 0x74, 0x30, 0x72, 0x30,
        0x37, 0xa0, 0x35, 0xa0, 0x33, 0x86, 0x31, 0x68, 0x74, 0x74, 0x70, 0x3a, 0x2f, 0x2f, 0x63,
        0x72, 0x6c, 0x33, 0x2e, 0x64, 0x69, 0x67, 0x69, 0x63, 0x65, 0x72, 0x74, 0x2e, 0x63, 0x6f,
        0x6d, 0x2f, 0x43, 0x6c, 0x6f, 0x75, 0x64, 0x66, 0x6c, 0x61, 0x72, 0x65, 0x49, 0x6e, 0x63,
        0x45, 0x43, 0x43, 0x43, 0x41, 0x2d, 0x33, 0x2e, 0x63, 0x72, 0x6c, 0x30, 0x37, 0xa0, 0x35,
        0xa0, 0x33, 0x86, 0x31, 0x68, 0x74, 0x74, 0x70, 0x3a, 0x2f, 0x2f, 0x63, 0x72, 0x6c, 0x34,
        0x2e, 0x64, 0x69, 0x67, 0x69, 0x63, 0x65, 0x72, 0x74, 0x2e, 0x63, 0x6f, 0x6d, 0x2f, 0x43,
        0x6c, 0x6f, 0x75, 0x64, 0x66, 0x6c, 0x61, 0x72, 0x65, 0x49, 0x6e, 0x63, 0x45, 0x43, 0x43,
        0x43, 0x41, 0x2d, 0x33, 0x2e, 0x63, 0x72, 0x6c, 0x30, 0x3e, 0x06, 0x03, 0x55, 0x1d, 0x20,
        0x04, 0x37, 0x30, 0x35, 0x30, 0x33, 0x06, 0x06, 0x67, 0x81, 0x0c, 0x01, 0x02, 0x02, 0x30,
        0x29, 0x30, 0x27, 0x06, 0x08, 0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x02, 0x01, 0x16, 0x1b,
        0x68, 0x74, 0x74, 0x70, 0x3a, 0x2f, 0x2f, 0x77, 0x77, 0x77, 0x2e, 0x64, 0x69, 0x67, 0x69,
        0x63, 0x65, 0x72, 0x74, 0x2e, 0x63, 0x6f, 0x6d, 0x2f, 0x43, 0x50, 0x53, 0x30, 0x76, 0x06,
        0x08, 0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x01, 0x01, 0x04, 0x6a, 0x30, 0x68, 0x30, 0x24,
        0x06, 0x08, 0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x30, 0x01, 0x86, 0x18, 0x68, 0x74, 0x74,
        0x70, 0x3a, 0x2f, 0x2f, 0x6f, 0x63, 0x73, 0x70, 0x2e, 0x64, 0x69, 0x67, 0x69, 0x63, 0x65,
        0x72, 0x74, 0x2e, 0x63, 0x6f, 0x6d, 0x30, 0x40, 0x06, 0x08, 0x2b, 0x06, 0x01, 0x05, 0x05,
        0x07, 0x30, 0x02, 0x86, 0x34, 0x68, 0x74, 0x74, 0x70, 0x3a, 0x2f, 0x2f, 0x63, 0x61, 0x63,
        0x65, 0x72, 0x74, 0x73, 0x2e, 0x64, 0x69, 0x67, 0x69, 0x63, 0x65, 0x72, 0x74, 0x2e, 0x63,
        0x6f, 0x6d, 0x2f, 0x43, 0x6c, 0x6f, 0x75, 0x64, 0x66, 0x6c, 0x61, 0x72, 0x65, 0x49, 0x6e,
        0x63, 0x45, 0x43, 0x43, 0x43, 0x41, 0x2d, 0x33, 0x2e, 0x63, 0x72, 0x74, 0x30, 0x0c, 0x06,
        0x03, 0x55, 0x1d, 0x13, 0x01, 0x01, 0xff, 0x04, 0x02, 0x30, 0x00, 0x30, 0x82, 0x01, 0x7f,
        0x06, 0x0a, 0x2b, 0x06, 0x01, 0x04, 0x01, 0xd6, 0x79, 0x02, 0x04, 0x02, 0x04, 0x82, 0x01,
        0x6f, 0x04, 0x82, 0x01, 0x6b, 0x01, 0x69, 0x00, 0x77, 0x00, 0xe8, 0x3e, 0xd0, 0xda, 0x3e,
        0xf5, 0x06, 0x35, 0x32, 0xe7, 0x57, 0x28, 0xbc, 0x89, 0x6b, 0xc9, 0x03, 0xd3, 0xcb, 0xd1,
        0x11, 0x6b, 0xec, 0xeb, 0x69, 0xe1, 0x77, 0x7d, 0x6d, 0x06, 0xbd, 0x6e, 0x00, 0x00, 0x01,
        0x80, 0x8c, 0xc8, 0xda, 0x53, 0x00, 0x00, 0x04, 0x03, 0x00, 0x48, 0x30, 0x46, 0x02, 0x21,
        0x00, 0x9b, 0x07, 0x41, 0xfa, 0x71, 0xb3, 0x56, 0x56, 0x5b, 0x7c, 0x09, 0xb0, 0x8a, 0xbe,
        0x41, 0x56, 0x4c, 0x6c, 0xa5, 0x73, 0xc6, 0x68, 0x71, 0x20, 0x55, 0xf2, 0x73, 0xef, 0xdc,
        0xaa, 0xc1, 0x29, 0x02, 0x21, 0x00, 0xb4, 0x19, 0x7c, 0x1b, 0x27, 0x84, 0xc9, 0xd8, 0x55,
        0xf0, 0x76, 0xac, 0x3e, 0xe3, 0x4b, 0xd6, 0x2a, 0x98, 0x7f, 0xdc, 0x70, 0x78, 0xad, 0x52,
        0x6a, 0x29, 0x84, 0xaf, 0x23, 0xcf, 0x01, 0x56, 0x00, 0x76, 0x00, 0x35, 0xcf, 0x19, 0x1b,
        0xbf, 0xb1, 0x6c, 0x57, 0xbf, 0x0f, 0xad, 0x4c, 0x6d, 0x42, 0xcb, 0xbb, 0xb6, 0x27, 0x20,
        0x26, 0x51, 0xea, 0x3f, 0xe1, 0x2a, 0xef, 0xa8, 0x03, 0xc3, 0x3b, 0xd6, 0x4c, 0x00, 0x00,
        0x01, 0x80, 0x8c, 0xc8, 0xda, 0x93, 0x00, 0x00, 0x04, 0x03, 0x00, 0x47, 0x30, 0x45, 0x02,
        0x21, 0x00, 0x90, 0xc8, 0x23, 0x7b, 0x2c, 0xa6, 0xe3, 0x27, 0xef, 0x8d, 0x58, 0x5a, 0x99,
        0x14, 0x76, 0x52, 0x4b, 0xef, 0x28, 0xe9, 0x94, 0x52, 0x05, 0x9d, 0x0e, 0x6e, 0x2b, 0x6a,
        0xf7, 0x1d, 0x85, 0x7d, 0x02, 0x20, 0x2b, 0xd5, 0x1f, 0xc0, 0x36, 0xb5, 0x40, 0xab, 0xd2,
        0x1f, 0xf2, 0x3a, 0x44, 0x28, 0x77, 0x47, 0x89, 0x99, 0xc6, 0x5c, 0x51, 0x16, 0x23, 0xc2,
        0x7d, 0xd0, 0x3d, 0xb4, 0x83, 0x7f, 0x1e, 0xae, 0x00, 0x76, 0x00, 0xb3, 0x73, 0x77, 0x07,
        0xe1, 0x84, 0x50, 0xf8, 0x63, 0x86, 0xd6, 0x05, 0xa9, 0xdc, 0x11, 0x09, 0x4a, 0x79, 0x2d,
        0xb1, 0x67, 0x0c, 0x0b, 0x87, 0xdc, 0xf0, 0x03, 0x0e, 0x79, 0x36, 0xa5, 0x9a, 0x00, 0x00,
        0x01, 0x80, 0x8c, 0xc8, 0xda, 0xc5, 0x00, 0x00, 0x04, 0x03, 0x00, 0x47, 0x30, 0x45, 0x02,
        0x20, 0x44, 0xf5, 0x19, 0xaf, 0x5b, 0xfb, 0x54, 0xec, 0xab, 0xf9, 0x7e, 0xdd, 0xd5, 0xd2,
        0x8a, 0x70, 0x34, 0xdd, 0x45, 0x11, 0x07, 0xf4, 0x7f, 0x4a, 0x2f, 0x63, 0x18, 0x26, 0x69,
        0xf1, 0xf3, 0x82, 0x02, 0x21, 0x00, 0xda, 0x82, 0xbc, 0x32, 0xbe, 0x8a, 0x71, 0x34, 0xd9,
        0x10, 0xe1, 0xdf, 0x1d, 0xaa, 0xb3, 0x6b, 0x40, 0xb3, 0x27, 0x7d, 0x58, 0xe9, 0xc2, 0x56,
        0x0a, 0xf1, 0x98, 0x7c, 0x6a, 0xae, 0xa5, 0xf3, 0x30, 0x0a, 0x06, 0x08, 0x2a, 0x86, 0x48,
        0xce, 0x3d, 0x04, 0x03, 0x02, 0x03, 0x49, 0x00, 0x30, 0x46, 0x02, 0x21, 0x00, 0xbf, 0x17,
        0xd1, 0xd2, 0xfa, 0x07, 0x05, 0x50, 0x38, 0x75, 0x66, 0x53, 0x0a, 0xa7, 0x2a, 0x29, 0x12,
        0x92, 0x07, 0xba, 0x70, 0xa1, 0xde, 0x8e, 0x90, 0x0f, 0xd6, 0x64, 0x36, 0x84, 0x5b, 0x69,
        0x02, 0x21, 0x00, 0xba, 0x66, 0x4b, 0xe1, 0x76, 0x98, 0x64, 0x46, 0x6d, 0x3d, 0xa2, 0x81,
        0x10, 0x1b, 0xc4, 0x0d, 0x3b, 0xb7, 0xed, 0x40, 0x5b, 0x2b, 0x37, 0xf0, 0xaa, 0x62, 0xda,
        0x84, 0x2a, 0xe4, 0xda, 0x0c,
    ];
    const OTHER_ECDSA_P256_SHA256_CERT: [u8; 522] = [
        0x30, 0x82, 0x02, 0x06, 0x30, 0x82, 0x01, 0xAC, 0x02, 0x09, 0x00, 0xD1, 0xA2, 0xE4, 0xD5,
        0x78, 0x05, 0x08, 0x61, 0x30, 0x0A, 0x06, 0x08, 0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x03,
        0x02, 0x30, 0x81, 0x8A, 0x31, 0x0B, 0x30, 0x09, 0x06, 0x03, 0x55, 0x04, 0x06, 0x13, 0x02,
        0x44, 0x45, 0x31, 0x0F, 0x30, 0x0D, 0x06, 0x03, 0x55, 0x04, 0x08, 0x0C, 0x06, 0x42, 0x65,
        0x72, 0x6C, 0x69, 0x6E, 0x31, 0x0F, 0x30, 0x0D, 0x06, 0x03, 0x55, 0x04, 0x07, 0x0C, 0x06,
        0x42, 0x65, 0x72, 0x6C, 0x69, 0x6E, 0x31, 0x10, 0x30, 0x0E, 0x06, 0x03, 0x55, 0x04, 0x0A,
        0x0C, 0x07, 0x68, 0x61, 0x63, 0x73, 0x70, 0x65, 0x63, 0x31, 0x0F, 0x30, 0x0D, 0x06, 0x03,
        0x55, 0x04, 0x0B, 0x0C, 0x06, 0x62, 0x65, 0x72, 0x74, 0x69, 0x65, 0x31, 0x17, 0x30, 0x15,
        0x06, 0x03, 0x55, 0x04, 0x03, 0x0C, 0x0E, 0x62, 0x65, 0x72, 0x74, 0x69, 0x65, 0x2E, 0x68,
        0x61, 0x63, 0x73, 0x70, 0x65, 0x63, 0x31, 0x1D, 0x30, 0x1B, 0x06, 0x09, 0x2A, 0x86, 0x48,
        0x86, 0xF7, 0x0D, 0x01, 0x09, 0x01, 0x16, 0x0E, 0x62, 0x65, 0x72, 0x74, 0x69, 0x65, 0x40,
        0x68, 0x61, 0x63, 0x73, 0x70, 0x65, 0x63, 0x30, 0x1E, 0x17, 0x0D, 0x32, 0x31, 0x30, 0x34,
        0x32, 0x39, 0x31, 0x31, 0x34, 0x37, 0x34, 0x35, 0x5A, 0x17, 0x0D, 0x33, 0x31, 0x30, 0x34,
        0x32, 0x37, 0x31, 0x31, 0x34, 0x37, 0x34, 0x35, 0x5A, 0x30, 0x81, 0x8A, 0x31, 0x0B, 0x30,
        0x09, 0x06, 0x03, 0x55, 0x04, 0x06, 0x13, 0x02, 0x44, 0x45, 0x31, 0x0F, 0x30, 0x0D, 0x06,
        0x03, 0x55, 0x04, 0x08, 0x0C, 0x06, 0x42, 0x65, 0x72, 0x6C, 0x69, 0x6E, 0x31, 0x0F, 0x30,
        0x0D, 0x06, 0x03, 0x55, 0x04, 0x07, 0x0C, 0x06, 0x42, 0x65, 0x72, 0x6C, 0x69, 0x6E, 0x31,
        0x10, 0x30, 0x0E, 0x06, 0x03, 0x55, 0x04, 0x0A, 0x0C, 0x07, 0x68, 0x61, 0x63, 0x73, 0x70,
        0x65, 0x63, 0x31, 0x0F, 0x30, 0x0D, 0x06, 0x03, 0x55, 0x04, 0x0B, 0x0C, 0x06, 0x62, 0x65,
        0x72, 0x74, 0x69, 0x65, 0x31, 0x17, 0x30, 0x15, 0x06, 0x03, 0x55, 0x04, 0x03, 0x0C, 0x0E,
        0x62, 0x65, 0x72, 0x74, 0x69, 0x65, 0x2E, 0x68, 0x61, 0x63, 0x73, 0x70, 0x65, 0x63, 0x31,
        0x1D, 0x30, 0x1B, 0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x01, 0x16,
        0x0E, 0x62, 0x65, 0x72, 0x74, 0x69, 0x65, 0x40, 0x68, 0x61, 0x63, 0x73, 0x70, 0x65, 0x63,
        0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x02, 0x01, 0x06, 0x08,
        0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00, 0x04, 0xD8, 0xE0, 0x74,
        0xF7, 0xCB, 0xEF, 0x19, 0xC7, 0x56, 0xA4, 0x52, 0x59, 0x0C, 0x02, 0x70, 0xCC, 0x9B, 0xFC,
        0x45, 0x8D, 0x73, 0x28, 0x39, 0x1D, 0x3B, 0xF5, 0x26, 0x17, 0x8B, 0x0D, 0x25, 0x04, 0x91,
        0xE8, 0xC8, 0x72, 0x22, 0x59, 0x9A, 0x2C, 0xBB, 0x26, 0x31, 0xB1, 0xCC, 0x6B, 0x6F, 0x5A,
        0x10, 0xD9, 0x7D, 0xD7, 0x86, 0x56, 0xFB, 0x89, 0x39, 0x9E, 0x0A, 0x91, 0x9F, 0x35, 0x81,
        0xE7, 0x30, 0x0A, 0x06, 0x08, 0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x03, 0x02, 0x03, 0x48,
        0x00, 0x30, 0x45, 0x02, 0x21, 0x00, 0xA1, 0x81, 0xB3, 0xD6, 0x8C, 0x9F, 0x62, 0x66, 0xC6,
        0xB7, 0x3F, 0x26, 0xE7, 0xFD, 0x88, 0xF9, 0x4B, 0xD8, 0x15, 0xD1, 0x45, 0xC7, 0x66, 0x69,
        0x40, 0xC2, 0x55, 0x21, 0x84, 0x9F, 0xE6, 0x8C, 0x02, 0x20, 0x10, 0x7E, 0xEF, 0xF3, 0x1D,
        0x58, 0x32, 0x6E, 0xF7, 0xCB, 0x0A, 0x47, 0xF2, 0xBA, 0xEB, 0xBC, 0xB7, 0x8F, 0x46, 0x56,
        0xF1, 0x5B, 0xCC, 0x2E, 0xD5, 0xB3, 0xC4, 0x0F, 0x5B, 0x22, 0xBD, 0x02,
    ];
}
