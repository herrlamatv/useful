// Encrypting with a password or for a key pair, one file format for every cipher:
//
//   "ESHA" | version | lock | cipher | chunk size (log2) | lock data | nonce / IV | ciphertext | HMAC tag
//
//   lock 1 = password: Argon2id (m KiB, t, p as u32 LE) + 16 byte salt
//   lock 2 = X25519:   ephemeral public key, key = HKDF-SHA256(shared secret, eph pub | recipient pub)
//   lock 3 = RSA:      u16 LE length + RSA-OAEP(SHA-256) wrapped random key
//
// AEAD ciphers (GCM, GCM-SIV, ChaCha20-Poly1305) encrypt in chunks the STREAM way (Hoang et al. 2015):
// nonce = random prefix | u32 BE chunk counter | 1 on the last chunk, the header is associated data,
// so changed, swapped, cut or missing chunks are all detected. No tag at the end.
// Block ciphers (CBC / CTR) are encrypt-then-MAC: HMAC-SHA256 over header + ciphertext at the end,
// checked in a first pass before a single byte gets decrypted (no padding oracle).

use crate::keys::PrivKey;
use crate::util::{err, random, read_full, tick, Progress, Res};
use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead as _, KeyInit, Payload};
use argon2::{Algorithm, Argon2, Params, Version};
use cipher::{BlockDecrypt, BlockEncrypt, KeyIvInit, StreamCipher};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use rand::rngs::OsRng;
use rsa::{Oaep, RsaPrivateKey, RsaPublicKey};
use sha2::Sha256;
use std::io::{BufWriter, Cursor, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

pub const MAGIC: &[u8; 4] = b"ESHA";
const VERSION: u8 = 1;
/// 256 KiB plaintext per AEAD chunk
const CHUNK_LOG2: u8 = 18;
/// read / write block for CBC and CTR (a multiple of every block size)
const BUF: usize = 1 << 20;
const TAG: usize = 32;
const X25519_INFO: &[u8] = b"encsha v1 x25519";

pub const WRONG: &str = "wrong password / key, or the data was changed";
pub const NOT_OURS: &str = "not encrypted with encsha (no ESHA header)";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cipher {
    Aes256Gcm,
    Aes128Gcm,
    Aes256GcmSiv,
    ChaCha20Poly1305,
    XChaCha20Poly1305,
    Aes256Cbc,
    Aes128Cbc,
    Aes256Ctr,
    Camellia256Cbc,
    Twofish256Cbc,
    Serpent256Cbc,
    Sm4Cbc,
    BlowfishCbc,
    Cast5Cbc,
    TripleDesCbc,
    DesCbc,
}

impl Cipher {
    pub const ALL: [Cipher; 16] = [
        Cipher::Aes256Gcm,
        Cipher::Aes128Gcm,
        Cipher::Aes256GcmSiv,
        Cipher::ChaCha20Poly1305,
        Cipher::XChaCha20Poly1305,
        Cipher::Aes256Cbc,
        Cipher::Aes128Cbc,
        Cipher::Aes256Ctr,
        Cipher::Camellia256Cbc,
        Cipher::Twofish256Cbc,
        Cipher::Serpent256Cbc,
        Cipher::Sm4Cbc,
        Cipher::BlowfishCbc,
        Cipher::Cast5Cbc,
        Cipher::TripleDesCbc,
        Cipher::DesCbc,
    ];

    pub fn id(self) -> u8 {
        match self {
            Cipher::Aes256Gcm => 1,
            Cipher::Aes128Gcm => 2,
            Cipher::Aes256GcmSiv => 3,
            Cipher::ChaCha20Poly1305 => 4,
            Cipher::XChaCha20Poly1305 => 5,
            Cipher::Aes256Cbc => 10,
            Cipher::Aes128Cbc => 11,
            Cipher::Aes256Ctr => 12,
            Cipher::Camellia256Cbc => 13,
            Cipher::Twofish256Cbc => 14,
            Cipher::Serpent256Cbc => 15,
            Cipher::Sm4Cbc => 16,
            Cipher::BlowfishCbc => 17,
            Cipher::Cast5Cbc => 18,
            Cipher::TripleDesCbc => 19,
            Cipher::DesCbc => 20,
        }
    }

    pub fn from_id(id: u8) -> Option<Cipher> {
        Cipher::ALL.into_iter().find(|c| c.id() == id)
    }

    pub fn name(self) -> &'static str {
        match self {
            Cipher::Aes256Gcm => "AES-256-GCM",
            Cipher::Aes128Gcm => "AES-128-GCM",
            Cipher::Aes256GcmSiv => "AES-256-GCM-SIV",
            Cipher::ChaCha20Poly1305 => "ChaCha20-Poly1305",
            Cipher::XChaCha20Poly1305 => "XChaCha20-Poly1305",
            Cipher::Aes256Cbc => "AES-256-CBC",
            Cipher::Aes128Cbc => "AES-128-CBC",
            Cipher::Aes256Ctr => "AES-256-CTR",
            Cipher::Camellia256Cbc => "Camellia-256-CBC",
            Cipher::Twofish256Cbc => "Twofish-256-CBC",
            Cipher::Serpent256Cbc => "Serpent-256-CBC",
            Cipher::Sm4Cbc => "SM4-CBC",
            Cipher::BlowfishCbc => "Blowfish-CBC",
            Cipher::Cast5Cbc => "CAST5-CBC",
            Cipher::TripleDesCbc => "3DES-CBC",
            Cipher::DesCbc => "DES-CBC",
        }
    }

    pub fn parse(s: &str) -> Option<Cipher> {
        let norm = |x: &str| x.to_ascii_lowercase().replace(['-', '_', ' '], "");
        let want = match norm(s).as_str() {
            "aes" | "aes256" | "aesgcm" => "aes256gcm".to_string(),
            "aes128" => "aes128gcm".into(),
            "chacha" | "chacha20" => "chacha20poly1305".into(),
            "xchacha" | "xchacha20" => "xchacha20poly1305".into(),
            "3des" | "tripledes" | "tdes" | "des3" | "desede3" | "desede3cbc" => "3descbc".into(),
            "camellia" => "camellia256cbc".into(),
            "twofish" => "twofish256cbc".into(),
            "serpent" => "serpent256cbc".into(),
            "sm4" => "sm4cbc".into(),
            "blowfish" | "bf" => "blowfishcbc".into(),
            "cast5" | "cast" => "cast5cbc".into(),
            "des" => "descbc".into(),
            other => other.to_string(),
        };
        Cipher::ALL.into_iter().find(|c| norm(c.name()) == want)
    }

    pub fn aead(self) -> bool {
        self.id() < 10
    }

    pub fn key_len(self) -> usize {
        match self {
            Cipher::Aes128Gcm | Cipher::Aes128Cbc | Cipher::Sm4Cbc | Cipher::Cast5Cbc => 16,
            Cipher::TripleDesCbc => 24,
            Cipher::DesCbc => 8,
            _ => 32,
        }
    }

    /// block size of the block cipher, 0 for the AEAD ciphers
    pub fn block_len(self) -> usize {
        match self {
            c if c.aead() => 0,
            Cipher::BlowfishCbc | Cipher::Cast5Cbc | Cipher::TripleDesCbc | Cipher::DesCbc => 8,
            _ => 16,
        }
    }

    /// bytes of nonce / IV stored in the header (AEAD: nonce minus 4 byte counter and 1 byte flag)
    fn iv_len(self) -> usize {
        match self {
            Cipher::XChaCha20Poly1305 => 19,
            c if c.aead() => 7,
            c => c.block_len(),
        }
    }

    fn mac_len(self) -> usize {
        if self.aead() {
            0
        } else {
            TAG
        }
    }

    /// short note for the weak ones
    pub fn warning(self) -> Option<&'static str> {
        match self {
            Cipher::DesCbc => Some("broken: 56-bit key, brute-forced in hours"),
            Cipher::TripleDesCbc | Cipher::BlowfishCbc | Cipher::Cast5Cbc => Some("legacy: 64-bit blocks, only for old systems"),
            // measured: the RustCrypto twofish / serpent crates are unoptimized (AES-CBC: ~370 MB/s)
            Cipher::Twofish256Cbc | Cipher::Serpent256Cbc => Some("slow here (3-10 MB/s): fine for text and small files"),
            _ => None,
        }
    }
}

/// Argon2id cost stored in every password-locked file
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Kdf {
    pub m_kib: u32,
    pub t: u32,
    pub p: u32,
}

impl Default for Kdf {
    fn default() -> Kdf {
        // 64 MiB, 3 passes, 4 lanes: ~0.1-0.2 s on a desktop
        Kdf { m_kib: 64 * 1024, t: 3, p: 4 }
    }
}

/// what encrypts
pub enum Lock<'a> {
    Password(&'a [u8], Kdf),
    X25519(&'a x25519_dalek::PublicKey),
    Rsa(&'a RsaPublicKey),
}

/// what may decrypt; the header decides which one is needed
#[derive(Default, Clone, Copy)]
pub struct Unlock<'a> {
    pub password: Option<&'a [u8]>,
    pub key: Option<&'a PrivKey>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LockKind {
    Password,
    X25519,
    Rsa,
}

impl LockKind {
    fn id(self) -> u8 {
        match self {
            LockKind::Password => 1,
            LockKind::X25519 => 2,
            LockKind::Rsa => 3,
        }
    }
    fn from_id(id: u8) -> Option<LockKind> {
        [LockKind::Password, LockKind::X25519, LockKind::Rsa].into_iter().find(|l| l.id() == id)
    }
    pub fn name(self) -> &'static str {
        match self {
            LockKind::Password => "password",
            LockKind::X25519 => "X25519 key",
            LockKind::Rsa => "RSA key",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Info {
    pub cipher: Cipher,
    pub lock: LockKind,
}

/// what a file / text was encrypted with, from the first 8 bytes
pub fn peek(head: &[u8]) -> Res<Info> {
    if head.len() < 8 || &head[..4] != MAGIC {
        return Err(NOT_OURS.into());
    }
    if head[4] != VERSION {
        return Err(format!("format version {} is newer than this program", head[4]));
    }
    let lock = LockKind::from_id(head[5]).ok_or("unknown lock type")?;
    let cipher = Cipher::from_id(head[6]).ok_or("unknown cipher")?;
    Ok(Info { cipher, lock })
}

// ---------------- keys ----------------

fn argon(pw: &[u8], salt: &[u8], k: Kdf, len: usize) -> Res<Zeroizing<Vec<u8>>> {
    let params = Params::new(k.m_kib, k.t, k.p, Some(len)).map_err(err)?;
    let mut out = Zeroizing::new(vec![0u8; len]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params).hash_password_into(pw, salt, &mut out).map_err(err)?;
    Ok(out)
}

fn x25519_key(secret: &x25519_dalek::StaticSecret, other: &x25519_dalek::PublicKey, eph: &[u8; 32], recipient: &[u8; 32], len: usize) -> Res<Zeroizing<Vec<u8>>> {
    let shared = secret.diffie_hellman(other);
    // an all-zero / low-order point would give everyone the same key
    if !shared.was_contributory() {
        return Err("invalid X25519 public key".into());
    }
    let salt = [&eph[..], &recipient[..]].concat();
    let mut out = Zeroizing::new(vec![0u8; len]);
    Hkdf::<Sha256>::new(Some(&salt), shared.as_bytes()).expand(X25519_INFO, &mut out).map_err(err)?;
    Ok(out)
}

// ---------------- AEAD ----------------

enum Aead {
    Gcm256(aes_gcm::Aes256Gcm),
    Gcm128(aes_gcm::Aes128Gcm),
    Siv(aes_gcm_siv::Aes256GcmSiv),
    ChaCha(chacha20poly1305::ChaCha20Poly1305),
    XChaCha(chacha20poly1305::XChaCha20Poly1305),
}

macro_rules! each_aead {
    ($s:expr, $c:ident => $e:expr) => {
        match $s {
            Aead::Gcm256($c) => $e,
            Aead::Gcm128($c) => $e,
            Aead::Siv($c) => $e,
            Aead::ChaCha($c) => $e,
            Aead::XChaCha($c) => $e,
        }
    };
}

impl Aead {
    fn new(c: Cipher, key: &[u8]) -> Res<Aead> {
        Ok(match c {
            Cipher::Aes256Gcm => Aead::Gcm256(KeyInit::new_from_slice(key).map_err(err)?),
            Cipher::Aes128Gcm => Aead::Gcm128(KeyInit::new_from_slice(key).map_err(err)?),
            Cipher::Aes256GcmSiv => Aead::Siv(KeyInit::new_from_slice(key).map_err(err)?),
            Cipher::ChaCha20Poly1305 => Aead::ChaCha(KeyInit::new_from_slice(key).map_err(err)?),
            Cipher::XChaCha20Poly1305 => Aead::XChaCha(KeyInit::new_from_slice(key).map_err(err)?),
            _ => unreachable!("not an AEAD cipher"),
        })
    }

    fn seal(&self, nonce: &[u8], aad: &[u8], msg: &[u8]) -> Res<Vec<u8>> {
        each_aead!(self, c => c.encrypt(GenericArray::from_slice(nonce), Payload { msg, aad }).map_err(|_| "encryption failed".to_string()))
    }

    fn open(&self, nonce: &[u8], aad: &[u8], msg: &[u8]) -> Res<Vec<u8>> {
        each_aead!(self, c => c.decrypt(GenericArray::from_slice(nonce), Payload { msg, aad }).map_err(|_| WRONG.to_string()))
    }
}

fn stream_nonce(prefix: &[u8], counter: u32, last: bool) -> Vec<u8> {
    let mut n = prefix.to_vec();
    n.extend_from_slice(&counter.to_be_bytes());
    n.push(last as u8);
    n
}

fn aead_seal(c: Cipher, key: &[u8], prefix: &[u8], head: &[u8], mut r: impl Read, w: &mut impl Write, prog: Option<&Progress>) -> Res<()> {
    let a = Aead::new(c, key)?;
    let size = 1usize << CHUNK_LOG2;
    let (mut cur, mut next) = (vec![0u8; size], vec![0u8; size]);
    let mut n = read_full(&mut r, &mut cur).map_err(err)?;
    let mut counter = 0u32;
    loop {
        // a chunk is the last one when nothing follows it (also when it is exactly full)
        let m = if n == size { read_full(&mut r, &mut next).map_err(err)? } else { 0 };
        let last = m == 0;
        let ct = a.seal(&stream_nonce(prefix, counter, last), head, &cur[..n])?;
        w.write_all(&ct).map_err(err)?;
        tick(prog, n)?;
        if last {
            return Ok(());
        }
        std::mem::swap(&mut cur, &mut next);
        n = m;
        counter = counter.checked_add(1).ok_or("input too large")?;
    }
}

fn aead_open(h: &Header, key: &[u8], mut r: impl Read, w: &mut impl Write, prog: Option<&Progress>) -> Res<()> {
    let a = Aead::new(h.info.cipher, key)?;
    let size = (1usize << h.chunk_log2) + 16;
    let (mut cur, mut next) = (vec![0u8; size], vec![0u8; size]);
    let mut n = read_full(&mut r, &mut cur).map_err(err)?;
    let mut counter = 0u32;
    loop {
        if n < 16 {
            return Err("the data is cut off".into());
        }
        let m = if n == size { read_full(&mut r, &mut next).map_err(err)? } else { 0 };
        let last = m == 0;
        let pt = a.open(&stream_nonce(&h.iv, counter, last), &h.raw, &cur[..n])?;
        w.write_all(&pt).map_err(err)?;
        tick(prog, n)?;
        if last {
            return Ok(());
        }
        std::mem::swap(&mut cur, &mut next);
        n = m;
        counter = counter.checked_add(1).ok_or("input too large")?;
    }
}

// ---------------- block ciphers ----------------

trait Block: Send {
    fn size(&self) -> usize;
    fn enc(&self, b: &mut [u8]);
    fn dec(&self, b: &mut [u8]);
}

impl<C: BlockEncrypt + BlockDecrypt + Send> Block for C {
    fn size(&self) -> usize {
        C::block_size()
    }
    fn enc(&self, b: &mut [u8]) {
        self.encrypt_block(GenericArray::from_mut_slice(b));
    }
    fn dec(&self, b: &mut [u8]) {
        self.decrypt_block(GenericArray::from_mut_slice(b));
    }
}

fn bx<C: KeyInit + BlockEncrypt + BlockDecrypt + Send + 'static>(key: &[u8]) -> Res<Box<dyn Block>> {
    Ok(Box::new(<C as KeyInit>::new_from_slice(key).map_err(err)?))
}

fn block(c: Cipher, key: &[u8]) -> Res<Box<dyn Block>> {
    match c {
        Cipher::Aes256Cbc => bx::<aes::Aes256>(key),
        Cipher::Aes128Cbc => bx::<aes::Aes128>(key),
        Cipher::Camellia256Cbc => bx::<camellia::Camellia256>(key),
        Cipher::Twofish256Cbc => bx::<twofish::Twofish>(key),
        Cipher::Serpent256Cbc => bx::<serpent::Serpent>(key),
        Cipher::Sm4Cbc => bx::<sm4::Sm4>(key),
        Cipher::BlowfishCbc => bx::<blowfish::Blowfish>(key),
        Cipher::Cast5Cbc => bx::<cast5::Cast5>(key),
        Cipher::TripleDesCbc => bx::<des::TdesEde3>(key),
        Cipher::DesCbc => bx::<des::Des>(key),
        _ => unreachable!("not a CBC cipher"),
    }
}

fn xor_into(a: &mut [u8], b: &[u8]) {
    a.iter_mut().zip(b).for_each(|(x, y)| *x ^= y);
}

/// CBC with PKCS#7 padding: c_i = E(p_i ^ c_i-1), c_0 = IV
fn cbc_seal(b: &dyn Block, iv: &[u8], mac: &mut Hmac<Sha256>, mut r: impl Read, w: &mut impl Write, prog: Option<&Progress>) -> Res<()> {
    let bs = b.size();
    let mut prev = iv.to_vec();
    let (mut cur, mut next) = (vec![0u8; BUF + bs], vec![0u8; BUF + bs]);
    let mut n = read_full(&mut r, &mut cur[..BUF]).map_err(err)?;
    loop {
        let m = if n == BUF { read_full(&mut r, &mut next[..BUF]).map_err(err)? } else { 0 };
        let last = m == 0;
        let mut len = n;
        if last {
            // always pad, a full block of padding when the input ends on a block boundary
            let pad = bs - n % bs;
            cur[n..n + pad].fill(pad as u8);
            len = n + pad;
        }
        for blk in cur[..len].chunks_exact_mut(bs) {
            xor_into(blk, &prev);
            b.enc(blk);
            prev.copy_from_slice(blk);
        }
        mac.update(&cur[..len]);
        w.write_all(&cur[..len]).map_err(err)?;
        tick(prog, n)?;
        if last {
            return Ok(());
        }
        std::mem::swap(&mut cur, &mut next);
        n = m;
    }
}

fn cbc_open(b: &dyn Block, iv: &[u8], mut r: impl Read, len: u64, w: &mut impl Write, prog: Option<&Progress>) -> Res<()> {
    let bs = b.size();
    if len == 0 || !len.is_multiple_of(bs as u64) {
        return Err("the data is cut off".into());
    }
    let mut prev = iv.to_vec();
    let mut saved = vec![0u8; bs];
    let mut buf = vec![0u8; BUF];
    let mut left = len;
    while left > 0 {
        let n = left.min(BUF as u64) as usize;
        r.read_exact(&mut buf[..n]).map_err(err)?;
        left -= n as u64;
        for blk in buf[..n].chunks_exact_mut(bs) {
            saved.copy_from_slice(blk);
            b.dec(blk);
            xor_into(blk, &prev);
            prev.copy_from_slice(&saved);
        }
        let mut out = n;
        if left == 0 {
            // the HMAC already matched, so bad padding means a bug, not an attack
            let pad = buf[n - 1] as usize;
            if pad == 0 || pad > bs || buf[n - pad..n].iter().any(|&x| x as usize != pad) {
                return Err("bad padding".into());
            }
            out = n - pad;
        }
        w.write_all(&buf[..out]).map_err(err)?;
        tick(prog, n)?;
    }
    Ok(())
}

/// CTR is its own inverse
fn ctr_apply(key: &[u8], iv: &[u8], mac: Option<&mut Hmac<Sha256>>, mut r: impl Read, len: Option<u64>, w: &mut impl Write, prog: Option<&Progress>) -> Res<()> {
    let mut ctr = ctr::Ctr128BE::<aes::Aes256>::new_from_slices(key, iv).map_err(err)?;
    let mut buf = vec![0u8; BUF];
    let mut mac = mac;
    let mut left = len.unwrap_or(u64::MAX);
    loop {
        let want = left.min(BUF as u64) as usize;
        let n = read_full(&mut r, &mut buf[..want]).map_err(err)?;
        if n == 0 {
            return Ok(());
        }
        left -= n as u64;
        ctr.apply_keystream(&mut buf[..n]);
        // encrypting: MAC over the ciphertext we just made
        if let Some(m) = mac.as_deref_mut() {
            m.update(&buf[..n]);
        }
        w.write_all(&buf[..n]).map_err(err)?;
        tick(prog, n)?;
    }
}

fn new_mac(key: &[u8]) -> Hmac<Sha256> {
    <Hmac<Sha256> as Mac>::new_from_slice(key).expect("hmac key")
}

// ---------------- encrypt / decrypt ----------------

/// write header + ciphertext to `w`
pub fn encrypt(cipher: Cipher, lock: &Lock, r: impl Read, w: &mut impl Write, prog: Option<&Progress>) -> Res<()> {
    let need = cipher.key_len() + cipher.mac_len();
    let kind = match lock {
        Lock::Password(..) => LockKind::Password,
        Lock::X25519(_) => LockKind::X25519,
        Lock::Rsa(_) => LockKind::Rsa,
    };
    let mut head = MAGIC.to_vec();
    head.extend_from_slice(&[VERSION, kind.id(), cipher.id(), if cipher.aead() { CHUNK_LOG2 } else { 0 }]);
    let km = match lock {
        Lock::Password(pw, kdf) => {
            let salt = random(16);
            head.push(1); // Argon2id
            for v in [kdf.m_kib, kdf.t, kdf.p] {
                head.extend_from_slice(&v.to_le_bytes());
            }
            head.extend_from_slice(&salt);
            argon(pw, &salt, *kdf, need)?
        }
        Lock::X25519(to) => {
            let eph = x25519_dalek::StaticSecret::random_from_rng(OsRng);
            let eph_pub = x25519_dalek::PublicKey::from(&eph);
            head.extend_from_slice(eph_pub.as_bytes());
            x25519_key(&eph, to, eph_pub.as_bytes(), to.as_bytes(), need)?
        }
        Lock::Rsa(to) => {
            let km = Zeroizing::new(random(need));
            let wrapped = to.encrypt(&mut OsRng, Oaep::new::<Sha256>(), &km).map_err(err)?;
            head.extend_from_slice(&(wrapped.len() as u16).to_le_bytes());
            head.extend_from_slice(&wrapped);
            km
        }
    };
    let iv = random(cipher.iv_len());
    head.extend_from_slice(&iv);
    w.write_all(&head).map_err(err)?;
    let (key, mac_key) = km.split_at(cipher.key_len());
    if cipher.aead() {
        return aead_seal(cipher, key, &iv, &head, r, w, prog);
    }
    let mut mac = new_mac(mac_key);
    mac.update(&head);
    if cipher == Cipher::Aes256Ctr {
        ctr_apply(key, &iv, Some(&mut mac), r, None, w, prog)?;
    } else {
        cbc_seal(block(cipher, key)?.as_ref(), &iv, &mut mac, r, w, prog)?;
    }
    w.write_all(&mac.finalize().into_bytes()).map_err(err)
}

struct Header {
    raw: Vec<u8>,
    info: Info,
    chunk_log2: u8,
    kdf: Option<(Kdf, Vec<u8>)>,
    eph: Option<[u8; 32]>,
    wrapped: Option<Vec<u8>>,
    iv: Vec<u8>,
}

fn read_header(r: &mut impl Read) -> Res<Header> {
    let mut raw = vec![0u8; 8];
    let n = read_full(r, &mut raw).map_err(err)?;
    let info = peek(&raw[..n])?;
    let chunk_log2 = raw[7];
    let mut take = |raw: &mut Vec<u8>, n: usize| -> Res<Vec<u8>> {
        let mut b = vec![0u8; n];
        r.read_exact(&mut b).map_err(|_| "the data is cut off".to_string())?;
        raw.extend_from_slice(&b);
        Ok(b)
    };
    let (mut kdf, mut eph, mut wrapped) = (None, None, None);
    match info.lock {
        LockKind::Password => {
            let p = take(&mut raw, 13)?;
            if p[0] != 1 {
                return Err("unknown key derivation".into());
            }
            let u = |i: usize| u32::from_le_bytes(p[i..i + 4].try_into().unwrap());
            let k = Kdf { m_kib: u(1), t: u(5), p: u(9) };
            // a crafted file shouldn't make us allocate 100 GB or loop for hours
            if k.m_kib > 4 << 20 || k.t > 100 || k.p > 64 {
                return Err("unreasonable key derivation settings in the header".into());
            }
            kdf = Some((k, take(&mut raw, 16)?));
        }
        LockKind::X25519 => eph = Some(take(&mut raw, 32)?.try_into().unwrap()),
        LockKind::Rsa => {
            let l = take(&mut raw, 2)?;
            let len = u16::from_le_bytes([l[0], l[1]]) as usize;
            if len > 2048 {
                return Err("damaged header".into());
            }
            wrapped = Some(take(&mut raw, len)?);
        }
    }
    if info.cipher.aead() && !(10..=24).contains(&chunk_log2) {
        return Err("damaged header".into());
    }
    let iv = take(&mut raw, info.cipher.iv_len())?;
    Ok(Header { raw, info, chunk_log2, kdf, eph, wrapped, iv })
}

fn key_material(h: &Header, un: &Unlock) -> Res<Zeroizing<Vec<u8>>> {
    let need = h.info.cipher.key_len() + h.info.cipher.mac_len();
    match h.info.lock {
        LockKind::Password => {
            let pw = un.password.ok_or("this needs the password")?;
            let (k, salt) = h.kdf.as_ref().unwrap();
            argon(pw, salt, *k, need)
        }
        LockKind::X25519 => match un.key {
            Some(PrivKey::X25519(sk)) => {
                let me = x25519_dalek::PublicKey::from(sk);
                let eph = h.eph.unwrap();
                x25519_key(sk, &x25519_dalek::PublicKey::from(eph), &eph, me.as_bytes(), need)
            }
            Some(other) => Err(format!("this was encrypted for an X25519 key, not for a {} key", other.kind_name())),
            None => Err("this needs the X25519 private key".into()),
        },
        LockKind::Rsa => match un.key {
            Some(PrivKey::Rsa(sk)) => {
                let km = rsa_unwrap(sk, h.wrapped.as_ref().unwrap())?;
                if km.len() != need {
                    return Err(WRONG.into());
                }
                Ok(km)
            }
            Some(other) => Err(format!("this was encrypted for an RSA key, not for a {} key", other.kind_name())),
            None => Err("this needs the RSA private key".into()),
        },
    }
}

fn rsa_unwrap(sk: &RsaPrivateKey, wrapped: &[u8]) -> Res<Zeroizing<Vec<u8>>> {
    sk.decrypt(Oaep::new::<Sha256>(), wrapped).map(Zeroizing::new).map_err(|_| "this was encrypted for a different RSA key".to_string())
}

/// read header + ciphertext from `r` (from its current position to the end), write the plaintext
pub fn decrypt<R: Read + Seek>(un: &Unlock, mut r: R, w: &mut impl Write, prog: Option<&Progress>) -> Res<Info> {
    let start = r.stream_position().map_err(err)?;
    let end = r.seek(SeekFrom::End(0)).map_err(err)?;
    r.seek(SeekFrom::Start(start)).map_err(err)?;
    let h = read_header(&mut r)?;
    let km = key_material(&h, un)?;
    let c = h.info.cipher;
    let (key, mac_key) = km.split_at(c.key_len());
    let body = end - start - h.raw.len() as u64;
    if c.aead() {
        if let Some(p) = prog {
            p.grow(body);
        }
        aead_open(&h, key, r, w, prog)?;
        return Ok(h.info);
    }
    let ct_len = body.checked_sub(TAG as u64).ok_or("the data is cut off")?;
    if let Some(p) = prog {
        p.grow(ct_len * 2);
    }
    // pass 1: check the HMAC before decrypting anything
    let mut mac = new_mac(mac_key);
    mac.update(&h.raw);
    let ct_start = r.stream_position().map_err(err)?;
    let mut buf = vec![0u8; BUF];
    let mut left = ct_len;
    while left > 0 {
        let n = left.min(BUF as u64) as usize;
        r.read_exact(&mut buf[..n]).map_err(err)?;
        mac.update(&buf[..n]);
        left -= n as u64;
        tick(prog, n)?;
    }
    let mut tag = [0u8; TAG];
    r.read_exact(&mut tag).map_err(err)?;
    mac.verify_slice(&tag).map_err(|_| WRONG.to_string())?;
    // pass 2: decrypt
    r.seek(SeekFrom::Start(ct_start)).map_err(err)?;
    let body = (&mut r).take(ct_len);
    if c == Cipher::Aes256Ctr {
        ctr_apply(key, &h.iv, None, body, Some(ct_len), w, prog)?;
    } else {
        cbc_open(block(c, key)?.as_ref(), &h.iv, body, ct_len, w, prog)?;
    }
    Ok(h.info)
}

// ---------------- bytes, text, files ----------------

pub fn encrypt_bytes(cipher: Cipher, lock: &Lock, data: &[u8]) -> Res<Vec<u8>> {
    let mut out = Vec::with_capacity(data.len() + 128);
    encrypt(cipher, lock, data, &mut out, None)?;
    Ok(out)
}

pub fn decrypt_bytes(un: &Unlock, data: &[u8]) -> Res<(Zeroizing<Vec<u8>>, Info)> {
    let mut out = Zeroizing::new(Vec::with_capacity(data.len()));
    let info = decrypt(un, Cursor::new(data), &mut *out, None)?;
    Ok((out, info))
}

/// encrypted text as one Base64 line, easy to paste into a chat or mail
pub fn encrypt_text(cipher: Cipher, lock: &Lock, text: &str) -> Res<String> {
    Ok(crate::util::b64(&encrypt_bytes(cipher, lock, text.as_bytes())?))
}

pub fn decrypt_text(un: &Unlock, b64: &str) -> Res<(Zeroizing<Vec<u8>>, Info)> {
    decrypt_bytes(un, &crate::util::b64_decode(b64)?)
}

pub fn peek_text(b64: &str) -> Res<Info> {
    // 8 header bytes = the first 12 Base64 characters
    let head: String = b64.chars().filter(|c| !c.is_whitespace()).take(12).collect();
    peek(&crate::util::b64_decode(&head)?)
}

pub fn peek_file(path: &Path) -> Res<Info> {
    let mut f = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut head = [0u8; 8];
    let n = read_full(&mut f, &mut head).map_err(err)?;
    peek(&head[..n])
}

/// `x.pdf` -> `x.pdf.esh` and back (`x.pdf.dec` when it doesn't end in .esh)
pub fn default_out(src: &Path, encrypting: bool) -> PathBuf {
    let s = src.as_os_str().to_string_lossy();
    if encrypting {
        PathBuf::from(format!("{s}.esh"))
    } else if let Some(stripped) = s.strip_suffix(".esh") {
        PathBuf::from(stripped)
    } else {
        PathBuf::from(format!("{s}.dec"))
    }
}

/// `x.txt` exists -> `x (2).txt`
pub fn unused(p: &Path) -> PathBuf {
    if !p.exists() {
        return p.to_path_buf();
    }
    let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let ext = p.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    (2..)
        .map(|i| p.with_file_name(format!("{stem} ({i}){ext}")))
        .find(|c| !c.exists())
        .unwrap()
}

/// written to `dst.part` first and renamed at the end, so a failed / cancelled run leaves nothing half done
fn to_file(dst: &Path, overwrite: bool, work: impl FnOnce(&mut BufWriter<std::fs::File>) -> Res<()>) -> Res<()> {
    if dst.exists() && !overwrite {
        return Err(format!("{} already exists", dst.display()));
    }
    let part = PathBuf::from(format!("{}.part", dst.display()));
    let f = std::fs::File::create(&part).map_err(|e| format!("{}: {e}", part.display()))?;
    let mut w = BufWriter::with_capacity(BUF, f);
    let res = work(&mut w).and_then(|_| w.flush().map_err(err));
    drop(w);
    if let Err(e) = res {
        let _ = std::fs::remove_file(&part);
        return Err(e);
    }
    if overwrite && dst.exists() {
        std::fs::remove_file(dst).map_err(err)?;
    }
    std::fs::rename(&part, dst).map_err(err)
}

pub fn encrypt_file(cipher: Cipher, lock: &Lock, src: &Path, dst: &Path, overwrite: bool, prog: Option<&Progress>) -> Res<()> {
    let f = std::fs::File::open(src).map_err(|e| format!("{}: {e}", src.display()))?;
    if let Some(p) = prog {
        p.grow(f.metadata().map(|m| m.len()).unwrap_or(0));
    }
    to_file(dst, overwrite, |w| encrypt(cipher, lock, f, w, prog))
}

pub fn decrypt_file(un: &Unlock, src: &Path, dst: &Path, overwrite: bool, prog: Option<&Progress>) -> Res<Info> {
    let f = std::fs::File::open(src).map_err(|e| format!("{}: {e}", src.display()))?;
    let mut info = None;
    to_file(dst, overwrite, |w| {
        info = Some(decrypt(un, f, w, prog)?);
        Ok(())
    })?;
    Ok(info.unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    // fast KDF so the tests don't spend seconds in Argon2
    const FAST: Kdf = Kdf { m_kib: 64, t: 1, p: 1 };

    fn pw(p: &str) -> Unlock<'_> {
        Unlock { password: Some(p.as_bytes()), key: None }
    }

    #[test]
    fn every_cipher_round_trips_every_size() {
        let lock = Lock::Password(b"pw", FAST);
        // around block and chunk boundaries
        let sizes = [0usize, 1, 7, 8, 15, 16, 17, 1000, (1 << CHUNK_LOG2) - 1, 1 << CHUNK_LOG2, (1 << CHUNK_LOG2) + 1, BUF, BUF + 3];
        for c in Cipher::ALL {
            for &n in &sizes {
                let data: Vec<u8> = (0..n).map(|i| (i * 7 + 3) as u8).collect();
                let enc = encrypt_bytes(c, &lock, &data).unwrap();
                let (dec, info) = decrypt_bytes(&pw("pw"), &enc).unwrap();
                assert_eq!(&*dec, &data, "{} {n}", c.name());
                assert_eq!(info, Info { cipher: c, lock: LockKind::Password });
            }
        }
    }

    #[test]
    fn wrong_password_and_tampering_fail() {
        let lock = Lock::Password(b"right", FAST);
        let data = vec![42u8; (1 << CHUNK_LOG2) * 2 + 10];
        for c in Cipher::ALL {
            let enc = encrypt_bytes(c, &lock, &data).unwrap();
            assert_eq!(decrypt_bytes(&pw("wrong"), &enc).unwrap_err(), WRONG, "{}", c.name());
            // flip one bit in the header, in the middle and at the end
            for at in [10, enc.len() / 2, enc.len() - 1] {
                let mut bad = enc.clone();
                bad[at] ^= 1;
                assert!(decrypt_bytes(&pw("right"), &bad).is_err(), "{} flip at {at}", c.name());
            }
            // cut off the end
            assert!(decrypt_bytes(&pw("right"), &enc[..enc.len() - 1]).is_err(), "{} truncated", c.name());
            if c.aead() {
                // drop the last chunk completely: the one before isn't marked "last"
                let chunk = (1 << CHUNK_LOG2) + 16;
                let head = enc.len() - 2 * chunk - (10 + 16);
                assert!(decrypt_bytes(&pw("right"), &enc[..head + 2 * chunk]).is_err(), "{} chunk dropped", c.name());
            }
        }
    }

    #[test]
    fn same_input_encrypts_differently() {
        let lock = Lock::Password(b"pw", FAST);
        let a = encrypt_bytes(Cipher::Aes256Gcm, &lock, b"hello").unwrap();
        let b = encrypt_bytes(Cipher::Aes256Gcm, &lock, b"hello").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn cbc_matches_nist_sp800_38a() {
        // F.2.1 CBC-AES128.Encrypt, first two blocks
        let key = hex::decode("2b7e151628aed2a6abf7158809cf4f3c").unwrap();
        let iv = hex::decode("000102030405060708090a0b0c0d0e0f").unwrap();
        let pt = hex::decode("6bc1bee22e409f96e93d7e117393172aae2d8a571e03ac9c9eb76fac45af8e51").unwrap();
        let mut mac = new_mac(b"k");
        let mut out = vec![];
        cbc_seal(block(Cipher::Aes128Cbc, &key).unwrap().as_ref(), &iv, &mut mac, &pt[..], &mut out, None).unwrap();
        assert_eq!(hex::encode(&out[..32]), "7649abac8119b246cee98e9b12e9197d5086cb9b507219ee95db113a917678b2");
        // + one block of padding
        assert_eq!(out.len(), 48);
        let mut back = vec![];
        cbc_open(block(Cipher::Aes128Cbc, &key).unwrap().as_ref(), &iv, &out[..], 48, &mut back, None).unwrap();
        assert_eq!(back, pt);
    }

    #[test]
    fn ctr_matches_nist_sp800_38a() {
        // F.5.5 CTR-AES256.Encrypt, first block
        let key = hex::decode("603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4").unwrap();
        let iv = hex::decode("f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff").unwrap();
        let pt = hex::decode("6bc1bee22e409f96e93d7e117393172a").unwrap();
        let mut out = vec![];
        ctr_apply(&key, &iv, None, &pt[..], None, &mut out, None).unwrap();
        assert_eq!(hex::encode(out), "601ec313775789a5b7a7f504bbf3d228");
    }

    #[test]
    fn text_and_peek() {
        let lock = Lock::Password(b"pw", FAST);
        let t = encrypt_text(Cipher::ChaCha20Poly1305, &lock, "Grüße 🔐").unwrap();
        assert_eq!(peek_text(&t).unwrap().cipher, Cipher::ChaCha20Poly1305);
        // pasted with line breaks
        let wrapped: String = t.as_bytes().chunks(20).map(|c| format!("{}\n", std::str::from_utf8(c).unwrap())).collect();
        assert_eq!(&*decrypt_text(&pw("pw"), &wrapped).unwrap().0, "Grüße 🔐".as_bytes());
        assert_eq!(peek(b"hello world").unwrap_err(), NOT_OURS);
    }

    #[test]
    fn names_parse_back() {
        for c in Cipher::ALL {
            assert_eq!(Cipher::parse(c.name()), Some(c));
            assert_eq!(Cipher::from_id(c.id()), Some(c));
        }
        assert_eq!(Cipher::parse("aes"), Some(Cipher::Aes256Gcm));
        assert_eq!(Cipher::parse("xchacha"), Some(Cipher::XChaCha20Poly1305));
        assert_eq!(Cipher::parse("3des"), Some(Cipher::TripleDesCbc));
    }

    #[test]
    fn files_are_atomic() {
        let dir = std::env::temp_dir().join(format!("encsha-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("a.bin");
        std::fs::write(&src, vec![7u8; 3_000_000]).unwrap();
        let enc = default_out(&src, true);
        let lock = Lock::Password(b"pw", FAST);
        encrypt_file(Cipher::Aes256Gcm, &lock, &src, &enc, false, None).unwrap();
        assert!(encrypt_file(Cipher::Aes256Gcm, &lock, &src, &enc, false, None).is_err(), "must not overwrite");
        let dec = dir.join("b.bin");
        // wrong password: no output and no .part left behind
        assert!(decrypt_file(&pw("no"), &enc, &dec, false, None).is_err());
        assert!(!dec.exists() && !dir.join("b.bin.part").exists());
        let p = Progress::default();
        decrypt_file(&pw("pw"), &enc, &dec, false, Some(&p)).unwrap();
        assert_eq!(std::fs::read(&dec).unwrap(), std::fs::read(&src).unwrap());
        assert_eq!(p.fraction(), Some(1.0));
        assert_eq!(default_out(&enc, false), src);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
