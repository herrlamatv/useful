// Key pairs: generate, save / load as PEM (PKCS#8 + SubjectPublicKeyInfo, the files OpenSSL writes),
// sign and verify. Ed25519 signs the SHA-512 of the input (Ed25519ph, RFC 8032), RSA uses PSS with
// SHA-256, P-256 ECDSA with SHA-256 - all three can sign files of any size without loading them.
// X25519 and RSA keys can also be used to encrypt (see crypt.rs).

use crate::crypt::{self, Cipher, Kdf, Lock, Unlock};
use crate::util::{b64, b64_decode, err, read_full, tick, Progress, Res};
use digest::Digest;
use rand::rngs::OsRng;
use rsa::pkcs1::{DecodeRsaPrivateKey, DecodeRsaPublicKey};
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey};
use rsa::{RsaPrivateKey, RsaPublicKey};
use sha2::{Sha256, Sha512};
use std::io::Read;
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

const PROTECTED: &str = "ENCSHA ENCRYPTED PRIVATE KEY";
const OID_ED25519: u8 = 0x70;
const OID_X25519: u8 = 0x6e;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyKind {
    Ed25519,
    X25519,
    Rsa(usize),
    P256,
}

impl KeyKind {
    pub fn name(self) -> String {
        match self {
            KeyKind::Ed25519 => "Ed25519".into(),
            KeyKind::X25519 => "X25519".into(),
            KeyKind::Rsa(bits) => format!("RSA-{bits}"),
            KeyKind::P256 => "ECDSA P-256".into(),
        }
    }

    pub fn parse(s: &str, bits: usize) -> Option<KeyKind> {
        Some(match s.to_ascii_lowercase().replace(['-', '_', ' '], "").as_str() {
            "ed25519" | "eddsa" => KeyKind::Ed25519,
            "x25519" | "curve25519" => KeyKind::X25519,
            "rsa" => KeyKind::Rsa(bits),
            "rsa2048" => KeyKind::Rsa(2048),
            "rsa3072" => KeyKind::Rsa(3072),
            "rsa4096" => KeyKind::Rsa(4096),
            "p256" | "ecdsa" | "ecdsap256" | "secp256r1" | "prime256v1" | "nistp256" => KeyKind::P256,
            _ => return None,
        })
    }

    /// what a key of this kind is for
    pub fn can_sign(self) -> bool {
        self != KeyKind::X25519
    }
    pub fn can_encrypt(self) -> bool {
        matches!(self, KeyKind::X25519 | KeyKind::Rsa(_))
    }

    pub fn sig_name(self) -> &'static str {
        match self {
            KeyKind::Ed25519 => "Ed25519ph (SHA-512)",
            KeyKind::Rsa(_) => "RSA-PSS SHA-256",
            KeyKind::P256 => "ECDSA P-256 SHA-256",
            KeyKind::X25519 => "-",
        }
    }
}

pub enum PrivKey {
    Ed25519(ed25519_dalek::SigningKey),
    X25519(x25519_dalek::StaticSecret),
    Rsa(RsaPrivateKey),
    P256(p256::SecretKey),
}

#[derive(Clone)]
pub enum PubKey {
    Ed25519(ed25519_dalek::VerifyingKey),
    X25519(x25519_dalek::PublicKey),
    Rsa(RsaPublicKey),
    P256(p256::PublicKey),
}

pub fn generate(kind: KeyKind) -> Res<PrivKey> {
    Ok(match kind {
        KeyKind::Ed25519 => PrivKey::Ed25519(ed25519_dalek::SigningKey::generate(&mut OsRng)),
        KeyKind::X25519 => PrivKey::X25519(x25519_dalek::StaticSecret::random_from_rng(OsRng)),
        KeyKind::Rsa(bits) => {
            if !(2048..=8192).contains(&bits) {
                return Err("RSA keys need 2048 to 8192 bits".into());
            }
            PrivKey::Rsa(RsaPrivateKey::new(&mut OsRng, bits).map_err(err)?)
        }
        KeyKind::P256 => PrivKey::P256(p256::SecretKey::random(&mut OsRng)),
    })
}

// fixed DER around the 32 key bytes (RFC 8410), what `openssl genpkey -algorithm ed25519` writes
fn der_25519(oid: u8, key: &[u8; 32], private: bool) -> Vec<u8> {
    let mut d = if private {
        vec![0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, oid, 0x04, 0x22, 0x04, 0x20]
    } else {
        vec![0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, oid, 0x03, 0x21, 0x00]
    };
    d.extend_from_slice(key);
    d
}

fn parse_25519(der: &[u8], oid: u8, private: bool) -> Option<[u8; 32]> {
    let algo = [0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, oid];
    let at = if private {
        // version 0 or 1 (v1 = public key attached at the end)
        (der.len() >= 48 && der[0] == 0x30 && der[2..4] == [0x02, 0x01] && der[4] <= 1 && der[5..12] == algo && der[12..16] == [0x04, 0x22, 0x04, 0x20]).then_some(16)?
    } else {
        (der.len() == 44 && der[0] == 0x30 && der[2..9] == algo && der[9..12] == [0x03, 0x21, 0x00]).then_some(12)?
    };
    der[at..at + 32].try_into().ok()
}

impl PrivKey {
    pub fn kind(&self) -> KeyKind {
        match self {
            PrivKey::Ed25519(_) => KeyKind::Ed25519,
            PrivKey::X25519(_) => KeyKind::X25519,
            PrivKey::Rsa(k) => KeyKind::Rsa(rsa::traits::PublicKeyParts::size(k) * 8),
            PrivKey::P256(_) => KeyKind::P256,
        }
    }

    pub fn kind_name(&self) -> String {
        self.kind().name()
    }

    pub fn public(&self) -> PubKey {
        match self {
            PrivKey::Ed25519(k) => PubKey::Ed25519(k.verifying_key()),
            PrivKey::X25519(k) => PubKey::X25519(x25519_dalek::PublicKey::from(k)),
            PrivKey::Rsa(k) => PubKey::Rsa(RsaPublicKey::from(k)),
            PrivKey::P256(k) => PubKey::P256(k.public_key()),
        }
    }

    fn der(&self) -> Res<Zeroizing<Vec<u8>>> {
        Ok(Zeroizing::new(match self {
            PrivKey::Ed25519(k) => der_25519(OID_ED25519, &k.to_bytes(), true),
            PrivKey::X25519(k) => der_25519(OID_X25519, &k.to_bytes(), true),
            PrivKey::Rsa(k) => k.to_pkcs8_der().map_err(err)?.as_bytes().to_vec(),
            PrivKey::P256(k) => k.to_pkcs8_der().map_err(err)?.as_bytes().to_vec(),
        }))
    }

    /// PKCS#8 "PRIVATE KEY", optionally locked with a password (then only encsha reads it)
    pub fn to_pem(&self, password: Option<&str>) -> Res<Zeroizing<String>> {
        let plain = Zeroizing::new(pem("PRIVATE KEY", &self.der()?));
        match password.filter(|p| !p.is_empty()) {
            None => Ok(plain),
            Some(pw) => {
                let enc = crypt::encrypt_bytes(Cipher::Aes256Gcm, &Lock::Password(pw.as_bytes(), Kdf::default()), plain.as_bytes())?;
                Ok(Zeroizing::new(pem(PROTECTED, &enc)))
            }
        }
    }
}

impl PubKey {
    pub fn kind(&self) -> KeyKind {
        match self {
            PubKey::Ed25519(_) => KeyKind::Ed25519,
            PubKey::X25519(_) => KeyKind::X25519,
            PubKey::Rsa(k) => KeyKind::Rsa(rsa::traits::PublicKeyParts::size(k) * 8),
            PubKey::P256(_) => KeyKind::P256,
        }
    }

    pub fn kind_name(&self) -> String {
        self.kind().name()
    }

    pub fn der(&self) -> Res<Vec<u8>> {
        Ok(match self {
            PubKey::Ed25519(k) => der_25519(OID_ED25519, k.as_bytes(), false),
            PubKey::X25519(k) => der_25519(OID_X25519, k.as_bytes(), false),
            PubKey::Rsa(k) => k.to_public_key_der().map_err(err)?.as_bytes().to_vec(),
            PubKey::P256(k) => k.to_public_key_der().map_err(err)?.as_bytes().to_vec(),
        })
    }

    pub fn to_pem(&self) -> Res<String> {
        Ok(pem("PUBLIC KEY", &self.der()?))
    }

    /// SHA-256 of the public key, first 16 bytes - compare it over the phone before trusting a key
    pub fn fingerprint(&self) -> String {
        let d = Sha256::digest(self.der().unwrap_or_default());
        hex::encode(&d[..16]).as_bytes().chunks(4).map(|c| std::str::from_utf8(c).unwrap()).collect::<Vec<_>>().join(" ")
    }

    /// what crypt::encrypt needs
    pub fn lock(&self) -> Res<Lock<'_>> {
        match self {
            PubKey::X25519(k) => Ok(Lock::X25519(k)),
            PubKey::Rsa(k) => Ok(Lock::Rsa(k)),
            other => Err(format!("{} keys only sign - use an X25519 or RSA key to encrypt", other.kind_name())),
        }
    }
}

pub fn pem(label: &str, der: &[u8]) -> String {
    let body = b64(der);
    let mut s = format!("-----BEGIN {label}-----\n");
    for line in body.as_bytes().chunks(64) {
        s.push_str(std::str::from_utf8(line).unwrap());
        s.push('\n');
    }
    s.push_str(&format!("-----END {label}-----\n"));
    s
}

/// first PEM block in the text: (label, DER)
pub fn unpem(text: &str) -> Res<(String, Zeroizing<Vec<u8>>)> {
    let start = text.find("-----BEGIN ").ok_or("no PEM block (-----BEGIN ...-----) found")?;
    let rest = &text[start + 11..];
    let label_end = rest.find("-----").ok_or("broken PEM header")?;
    let label = rest[..label_end].to_string();
    let body_start = &rest[label_end + 5..];
    let end = body_start.find(&format!("-----END {label}-----")).ok_or("PEM block has no END line")?;
    // skip header lines like "Proc-Type: 4,ENCRYPTED"
    let body: String = body_start[..end].lines().filter(|l| !l.contains(':')).collect();
    Ok((label, Zeroizing::new(b64_decode(&body)?)))
}

pub fn is_protected(text: &str) -> bool {
    text.contains(&format!("BEGIN {PROTECTED}"))
}

pub fn parse_private(text: &str, password: Option<&str>) -> Res<PrivKey> {
    let (label, der) = unpem(text)?;
    match label.as_str() {
        PROTECTED => {
            let pw = password.filter(|p| !p.is_empty()).ok_or("this private key is locked with a password")?;
            let un = Unlock { password: Some(pw.as_bytes()), key: None };
            let (plain, _) = crypt::decrypt_bytes(&un, &der).map_err(|e| if e == crypt::WRONG { "wrong key password".to_string() } else { e })?;
            parse_private(std::str::from_utf8(&plain).map_err(err)?, None)
        }
        "PRIVATE KEY" => {
            if let Some(k) = parse_25519(&der, OID_ED25519, true) {
                return Ok(PrivKey::Ed25519(ed25519_dalek::SigningKey::from_bytes(&k)));
            }
            if let Some(k) = parse_25519(&der, OID_X25519, true) {
                return Ok(PrivKey::X25519(x25519_dalek::StaticSecret::from(k)));
            }
            if let Ok(k) = RsaPrivateKey::from_pkcs8_der(&der) {
                return Ok(PrivKey::Rsa(k));
            }
            if let Ok(k) = p256::SecretKey::from_pkcs8_der(&der) {
                return Ok(PrivKey::P256(k));
            }
            Err("unsupported key type (RSA, Ed25519, X25519 and P-256 work)".into())
        }
        "RSA PRIVATE KEY" => RsaPrivateKey::from_pkcs1_der(&der).map(PrivKey::Rsa).map_err(err),
        "EC PRIVATE KEY" => p256::SecretKey::from_sec1_der(&der).map(PrivKey::P256).map_err(|_| "only P-256 EC keys are supported".to_string()),
        "ENCRYPTED PRIVATE KEY" => Err("OpenSSL-encrypted key: unlock it first with `openssl pkcs8 -in key.pem -out plain.pem`".into()),
        "OPENSSH PRIVATE KEY" => Err("OpenSSH key format: convert it with `ssh-keygen -p -m PKCS8 -f key`".into()),
        "PUBLIC KEY" | "RSA PUBLIC KEY" => Err("this is a public key, a private key is needed here".into()),
        other => Err(format!("unknown PEM type \"{other}\"")),
    }
}

/// public key PEM, or the public half of an (unlocked) private key
pub fn parse_public(text: &str) -> Res<PubKey> {
    let (label, der) = unpem(text)?;
    match label.as_str() {
        "PUBLIC KEY" => {
            if let Some(k) = parse_25519(&der, OID_ED25519, false) {
                return ed25519_dalek::VerifyingKey::from_bytes(&k).map(PubKey::Ed25519).map_err(err);
            }
            if let Some(k) = parse_25519(&der, OID_X25519, false) {
                return Ok(PubKey::X25519(x25519_dalek::PublicKey::from(k)));
            }
            if let Ok(k) = RsaPublicKey::from_public_key_der(&der) {
                return Ok(PubKey::Rsa(k));
            }
            if let Ok(k) = p256::PublicKey::from_public_key_der(&der) {
                return Ok(PubKey::P256(k));
            }
            Err("unsupported key type (RSA, Ed25519, X25519 and P-256 work)".into())
        }
        "RSA PUBLIC KEY" => RsaPublicKey::from_pkcs1_der(&der).map(PubKey::Rsa).map_err(err),
        PROTECTED => Err("this is a locked private key - load the public key (.pub.pem) instead".into()),
        _ => parse_private(text, None).map(|k| k.public()),
    }
}

pub fn read_key_file(path: &Path) -> Res<Zeroizing<String>> {
    std::fs::read_to_string(path).map(Zeroizing::new).map_err(|e| format!("{}: {e}", path.display()))
}

/// `<dir>/<name>.key.pem` + `<dir>/<name>.pub.pem`, never overwrites
pub fn write_pair(dir: &Path, name: &str, key: &PrivKey, password: Option<&str>) -> Res<(PathBuf, PathBuf)> {
    let private = dir.join(format!("{name}.key.pem"));
    let public = dir.join(format!("{name}.pub.pem"));
    for p in [&private, &public] {
        if p.exists() {
            return Err(format!("{} already exists", p.display()));
        }
    }
    std::fs::write(&private, key.to_pem(password)?.as_bytes()).map_err(err)?;
    std::fs::write(&public, key.public().to_pem()?).map_err(err)?;
    Ok((private, public))
}

fn digest_of<D: Digest>(mut r: impl Read, prog: Option<&Progress>) -> Res<D> {
    let mut d = D::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = read_full(&mut r, &mut buf).map_err(err)?;
        d.update(&buf[..n]);
        tick(prog, n)?;
        if n < buf.len() {
            return Ok(d);
        }
    }
}

pub fn sign(key: &PrivKey, r: impl Read, prog: Option<&Progress>) -> Res<Vec<u8>> {
    match key {
        PrivKey::Ed25519(k) => Ok(k.sign_prehashed(digest_of::<Sha512>(r, prog)?, None).map_err(err)?.to_bytes().to_vec()),
        PrivKey::Rsa(k) => {
            use rsa::signature::{RandomizedDigestSigner, SignatureEncoding};
            let sk = rsa::pss::BlindedSigningKey::<Sha256>::new(k.clone());
            Ok(sk.sign_digest_with_rng(&mut OsRng, digest_of::<Sha256>(r, prog)?).to_vec())
        }
        PrivKey::P256(k) => {
            use p256::ecdsa::signature::DigestSigner;
            let sig: p256::ecdsa::Signature = p256::ecdsa::SigningKey::from(k).sign_digest(digest_of::<Sha256>(r, prog)?);
            Ok(sig.to_der().as_bytes().to_vec())
        }
        PrivKey::X25519(_) => Err("X25519 keys only encrypt - use Ed25519, RSA or P-256 to sign".into()),
    }
}

/// Ok(false) = the signature doesn't belong to this input + key
pub fn verify(key: &PubKey, r: impl Read, sig: &[u8], prog: Option<&Progress>) -> Res<bool> {
    Ok(match key {
        PubKey::Ed25519(k) => {
            let Ok(s) = ed25519_dalek::Signature::from_slice(sig) else { return Ok(false) };
            k.verify_prehashed(digest_of::<Sha512>(r, prog)?, None, &s).is_ok()
        }
        PubKey::Rsa(k) => {
            use rsa::signature::DigestVerifier;
            let Ok(s) = rsa::pss::Signature::try_from(sig) else { return Ok(false) };
            rsa::pss::VerifyingKey::<Sha256>::new(k.clone()).verify_digest(digest_of::<Sha256>(r, prog)?, &s).is_ok()
        }
        PubKey::P256(k) => {
            use p256::ecdsa::signature::DigestVerifier;
            // DER like OpenSSL, or the raw 64 bytes r | s (JWT / WebCrypto)
            let s = p256::ecdsa::Signature::from_der(sig).or_else(|_| p256::ecdsa::Signature::from_slice(sig));
            let Ok(s) = s else { return Ok(false) };
            p256::ecdsa::VerifyingKey::from(k).verify_digest(digest_of::<Sha256>(r, prog)?, &s).is_ok()
        }
        PubKey::X25519(_) => return Err("X25519 keys can't sign, so there is nothing to verify".into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all() -> Vec<PrivKey> {
        [KeyKind::Ed25519, KeyKind::X25519, KeyKind::Rsa(2048), KeyKind::P256].into_iter().map(|k| generate(k).unwrap()).collect()
    }

    #[test]
    fn pem_round_trip() {
        for k in all() {
            let pem = k.to_pem(None).unwrap();
            let back = parse_private(&pem, None).unwrap();
            assert_eq!(back.kind(), k.kind());
            assert_eq!(back.public().der().unwrap(), k.public().der().unwrap());
            let pub_pem = k.public().to_pem().unwrap();
            assert_eq!(parse_public(&pub_pem).unwrap().der().unwrap(), k.public().der().unwrap());
            // the public half of a private key file
            assert_eq!(parse_public(&pem).unwrap().fingerprint(), k.public().fingerprint());
        }
    }

    #[test]
    fn locked_private_key() {
        let k = generate(KeyKind::Ed25519).unwrap();
        let pem = k.to_pem(Some("secret")).unwrap();
        assert!(is_protected(&pem));
        assert!(parse_private(&pem, None).is_err());
        assert_eq!(parse_private(&pem, Some("nope")).err().unwrap(), "wrong key password");
        assert_eq!(parse_private(&pem, Some("secret")).unwrap().public().der().unwrap(), k.public().der().unwrap());
    }

    #[test]
    fn sign_and_verify() {
        let data = vec![1u8; 3_000_000];
        for k in all() {
            if !k.kind().can_sign() {
                assert!(sign(&k, &data[..], None).is_err());
                continue;
            }
            let sig = sign(&k, &data[..], None).unwrap();
            let p = k.public();
            assert!(verify(&p, &data[..], &sig, None).unwrap(), "{}", k.kind_name());
            let mut other = data.clone();
            other[1_234_567] ^= 1;
            assert!(!verify(&p, &other[..], &sig, None).unwrap(), "{} changed data", k.kind_name());
            assert!(!verify(&p, &data[..], &sig[1..], None).unwrap(), "{} broken signature", k.kind_name());
        }
    }

    #[test]
    fn ed25519ph_rfc8032_vector() {
        // RFC 8032 7.3, Ed25519ph, message "abc"
        let sk: [u8; 32] = hex::decode("833fe62409237b9d62ec77587520911e9a759cec1d19755b7da901b96dca3d42").unwrap().try_into().unwrap();
        let k = PrivKey::Ed25519(ed25519_dalek::SigningKey::from_bytes(&sk));
        assert_eq!(
            hex::encode(sign(&k, &b"abc"[..], None).unwrap()),
            "98a70222f0b8121aa9d30f813d683f809e462b469c7ff87639499bb94e6dae4131f85042463c2a355a2003d062adf5aaa10b8c61e636062aaad11c2a26083406"
        );
    }

    #[test]
    fn encrypt_for_a_key() {
        for k in all() {
            let p = k.public();
            let Ok(lock) = p.lock() else {
                assert!(!k.kind().can_encrypt());
                continue;
            };
            let enc = crypt::encrypt_bytes(Cipher::XChaCha20Poly1305, &lock, b"for your eyes only").unwrap();
            let (dec, info) = crypt::decrypt_bytes(&Unlock { password: None, key: Some(&k) }, &enc).unwrap();
            assert_eq!(&*dec, b"for your eyes only");
            assert_eq!(info.cipher, Cipher::XChaCha20Poly1305);
            // a different key of the same kind can't open it
            let other = generate(k.kind()).unwrap();
            assert!(crypt::decrypt_bytes(&Unlock { password: None, key: Some(&other) }, &enc).is_err());
            // and a password doesn't help
            assert!(crypt::decrypt_bytes(&Unlock { password: Some(b"x"), key: None }, &enc).is_err());
        }
    }
}
