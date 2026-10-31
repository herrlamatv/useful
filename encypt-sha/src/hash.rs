// Every hash / checksum by name. Any number of them run in one pass over the input:
// a reader thread fetches the next block while all hashers work on the current one in parallel.

use crate::util::{err, read_full, tick, Progress, Res};
use digest::core_api::BlockSizeUser;
use digest::{Digest, ExtendableOutput, Mac};
use hmac::SimpleHmac;
use rayon::prelude::*;
use std::io::Read;
use std::path::Path;
use std::sync::mpsc::sync_channel;

const BLOCK: usize = 4 << 20;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Algo {
    Crc32,
    Xxh64,
    Xxh3_64,
    Xxh3_128,
    Md4,
    Md5,
    Ntlm,
    Sha1,
    Ripemd160,
    Sha224,
    Sha256,
    Sha384,
    Sha512,
    Sha512_224,
    Sha512_256,
    Sha3_224,
    Sha3_256,
    Sha3_384,
    Sha3_512,
    Keccak256,
    Shake128,
    Shake256,
    Blake2s,
    Blake2b,
    Blake3,
    Sm3,
    Streebog256,
    Streebog512,
    Whirlpool,
    Tiger,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    /// finds accidental changes only (downloads, copies)
    Checksum,
    /// collisions are known, fine for checksums, not for security
    Broken,
    Secure,
}

impl Algo {
    pub const ALL: [Algo; 30] = [
        Algo::Crc32,
        Algo::Xxh64,
        Algo::Xxh3_64,
        Algo::Xxh3_128,
        Algo::Md4,
        Algo::Md5,
        Algo::Ntlm,
        Algo::Sha1,
        Algo::Ripemd160,
        Algo::Sha224,
        Algo::Sha256,
        Algo::Sha384,
        Algo::Sha512,
        Algo::Sha512_224,
        Algo::Sha512_256,
        Algo::Sha3_224,
        Algo::Sha3_256,
        Algo::Sha3_384,
        Algo::Sha3_512,
        Algo::Keccak256,
        Algo::Shake128,
        Algo::Shake256,
        Algo::Blake2s,
        Algo::Blake2b,
        Algo::Blake3,
        Algo::Sm3,
        Algo::Streebog256,
        Algo::Streebog512,
        Algo::Whirlpool,
        Algo::Tiger,
    ];

    /// what the window ticks by default
    pub const COMMON: [Algo; 7] = [Algo::Crc32, Algo::Md5, Algo::Sha1, Algo::Sha256, Algo::Sha512, Algo::Sha3_256, Algo::Blake3];

    pub fn name(self) -> &'static str {
        match self {
            Algo::Crc32 => "CRC32",
            Algo::Xxh64 => "xxHash64",
            Algo::Xxh3_64 => "XXH3-64",
            Algo::Xxh3_128 => "XXH3-128",
            Algo::Md4 => "MD4",
            Algo::Md5 => "MD5",
            Algo::Ntlm => "NTLM",
            Algo::Sha1 => "SHA-1",
            Algo::Ripemd160 => "RIPEMD-160",
            Algo::Sha224 => "SHA-224",
            Algo::Sha256 => "SHA-256",
            Algo::Sha384 => "SHA-384",
            Algo::Sha512 => "SHA-512",
            Algo::Sha512_224 => "SHA-512/224",
            Algo::Sha512_256 => "SHA-512/256",
            Algo::Sha3_224 => "SHA3-224",
            Algo::Sha3_256 => "SHA3-256",
            Algo::Sha3_384 => "SHA3-384",
            Algo::Sha3_512 => "SHA3-512",
            Algo::Keccak256 => "Keccak-256",
            Algo::Shake128 => "SHAKE128",
            Algo::Shake256 => "SHAKE256",
            Algo::Blake2s => "BLAKE2s-256",
            Algo::Blake2b => "BLAKE2b-512",
            Algo::Blake3 => "BLAKE3",
            Algo::Sm3 => "SM3",
            Algo::Streebog256 => "Streebog-256",
            Algo::Streebog512 => "Streebog-512",
            Algo::Whirlpool => "Whirlpool",
            Algo::Tiger => "Tiger",
        }
    }

    /// "sha256", "SHA-256", "sha_256", "sha2-256" ... all mean the same
    pub fn parse(s: &str) -> Option<Algo> {
        let norm = |x: &str| x.to_ascii_lowercase().replace(['-', '_', '/', ' '], "");
        let want = norm(s);
        let alias = match want.as_str() {
            "sha2224" => "sha224",
            "sha2256" => "sha256",
            "sha2384" => "sha384",
            "sha2512" => "sha512",
            "blake2s" => "blake2s256",
            "blake2b" => "blake2b512",
            "keccak" => "keccak256",
            "ripemd" => "ripemd160",
            "xxh64" => "xxhash64",
            "xxh3" => "xxh364",
            "crc" => "crc32",
            "gost" | "streebog" => "streebog256",
            "sha3" => "sha3256",
            "shake" => "shake256",
            "sha" => "sha1",
            other => other,
        }
        .to_string();
        Algo::ALL.into_iter().find(|a| norm(a.name()) == alias)
    }

    pub fn class(self) -> Class {
        match self {
            Algo::Crc32 | Algo::Xxh64 | Algo::Xxh3_64 | Algo::Xxh3_128 => Class::Checksum,
            Algo::Md4 | Algo::Md5 | Algo::Ntlm | Algo::Sha1 => Class::Broken,
            _ => Class::Secure,
        }
    }

    /// output length in bytes
    pub fn out_len(self) -> usize {
        match self {
            Algo::Crc32 => 4,
            Algo::Xxh64 | Algo::Xxh3_64 => 8,
            Algo::Xxh3_128 | Algo::Md4 | Algo::Md5 | Algo::Ntlm => 16,
            Algo::Sha1 | Algo::Ripemd160 => 20,
            Algo::Tiger => 24,
            Algo::Sha224 | Algo::Sha512_224 | Algo::Sha3_224 => 28,
            Algo::Sha384 | Algo::Sha3_384 => 48,
            Algo::Sha512 | Algo::Sha3_512 | Algo::Shake256 | Algo::Blake2b | Algo::Streebog512 | Algo::Whirlpool => 64,
            _ => 32,
        }
    }

    /// NTLM hashes a password (as UTF-16), not file contents
    pub fn text_only(self) -> bool {
        self == Algo::Ntlm
    }

    pub fn hmac_ok(self) -> bool {
        self.class() != Class::Checksum && !matches!(self, Algo::Ntlm | Algo::Shake128 | Algo::Shake256 | Algo::Blake3)
    }

    pub fn hasher(self) -> Box<dyn Hasher> {
        match self {
            Algo::Crc32 => Box::new(Crc(crc32fast::Hasher::new())),
            Algo::Xxh64 => Box::new(X64(xxhash_rust::xxh64::Xxh64::new(0))),
            Algo::Xxh3_64 => Box::new(X3(Box::new(xxhash_rust::xxh3::Xxh3::new()), false)),
            Algo::Xxh3_128 => Box::new(X3(Box::new(xxhash_rust::xxh3::Xxh3::new()), true)),
            Algo::Md4 => d::<md4::Md4>(),
            Algo::Md5 => d::<md5::Md5>(),
            Algo::Ntlm => Box::new(Ntlm(Vec::new())),
            Algo::Sha1 => d::<sha1::Sha1>(),
            Algo::Ripemd160 => d::<ripemd::Ripemd160>(),
            Algo::Sha224 => d::<sha2::Sha224>(),
            Algo::Sha256 => d::<sha2::Sha256>(),
            Algo::Sha384 => d::<sha2::Sha384>(),
            Algo::Sha512 => d::<sha2::Sha512>(),
            Algo::Sha512_224 => d::<sha2::Sha512_224>(),
            Algo::Sha512_256 => d::<sha2::Sha512_256>(),
            Algo::Sha3_224 => d::<sha3::Sha3_224>(),
            Algo::Sha3_256 => d::<sha3::Sha3_256>(),
            Algo::Sha3_384 => d::<sha3::Sha3_384>(),
            Algo::Sha3_512 => d::<sha3::Sha3_512>(),
            Algo::Keccak256 => d::<sha3::Keccak256>(),
            Algo::Shake128 => Box::new(Xof(sha3::Shake128::default(), 32)),
            Algo::Shake256 => Box::new(Xof(sha3::Shake256::default(), 64)),
            Algo::Blake2s => d::<blake2::Blake2s256>(),
            Algo::Blake2b => d::<blake2::Blake2b512>(),
            Algo::Blake3 => Box::new(B3(blake3::Hasher::new())),
            Algo::Sm3 => d::<sm3::Sm3>(),
            Algo::Streebog256 => d::<streebog::Streebog256>(),
            Algo::Streebog512 => d::<streebog::Streebog512>(),
            Algo::Whirlpool => d::<whirlpool::Whirlpool>(),
            Algo::Tiger => d::<tiger::Tiger>(),
        }
    }

    /// HMAC with this hash, None if that doesn't exist (checksums, XOFs, BLAKE3, NTLM)
    pub fn hmac(self, key: &[u8]) -> Option<Box<dyn Hasher>> {
        Some(match self {
            Algo::Md4 => m::<md4::Md4>(key),
            Algo::Md5 => m::<md5::Md5>(key),
            Algo::Sha1 => m::<sha1::Sha1>(key),
            Algo::Ripemd160 => m::<ripemd::Ripemd160>(key),
            Algo::Sha224 => m::<sha2::Sha224>(key),
            Algo::Sha256 => m::<sha2::Sha256>(key),
            Algo::Sha384 => m::<sha2::Sha384>(key),
            Algo::Sha512 => m::<sha2::Sha512>(key),
            Algo::Sha512_224 => m::<sha2::Sha512_224>(key),
            Algo::Sha512_256 => m::<sha2::Sha512_256>(key),
            Algo::Sha3_224 => m::<sha3::Sha3_224>(key),
            Algo::Sha3_256 => m::<sha3::Sha3_256>(key),
            Algo::Sha3_384 => m::<sha3::Sha3_384>(key),
            Algo::Sha3_512 => m::<sha3::Sha3_512>(key),
            Algo::Keccak256 => m::<sha3::Keccak256>(key),
            Algo::Blake2s => m::<blake2::Blake2s256>(key),
            Algo::Blake2b => m::<blake2::Blake2b512>(key),
            Algo::Sm3 => m::<sm3::Sm3>(key),
            Algo::Streebog256 => m::<streebog::Streebog256>(key),
            Algo::Streebog512 => m::<streebog::Streebog512>(key),
            Algo::Whirlpool => m::<whirlpool::Whirlpool>(key),
            Algo::Tiger => m::<tiger::Tiger>(key),
            _ => return None,
        })
    }
}

pub trait Hasher: Send {
    fn update(&mut self, data: &[u8]);
    fn finish(self: Box<Self>) -> Vec<u8>;
}

struct D<T>(T);
impl<T: Digest + Send> Hasher for D<T> {
    fn update(&mut self, data: &[u8]) {
        Digest::update(&mut self.0, data);
    }
    fn finish(self: Box<Self>) -> Vec<u8> {
        self.0.finalize().to_vec()
    }
}
fn d<T: Digest + Send + 'static>() -> Box<dyn Hasher> {
    Box::new(D(T::new()))
}

struct M<T>(T);
impl<T: Mac + Send> Hasher for M<T> {
    fn update(&mut self, data: &[u8]) {
        Mac::update(&mut self.0, data);
    }
    fn finish(self: Box<Self>) -> Vec<u8> {
        self.0.finalize().into_bytes().to_vec()
    }
}
fn m<T: Digest + BlockSizeUser + Send + 'static>(key: &[u8]) -> Box<dyn Hasher> {
    // HMAC takes keys of any length
    Box::new(M(<SimpleHmac<T> as Mac>::new_from_slice(key).expect("hmac key")))
}

struct Xof<T>(T, usize);
impl<T: digest::Update + ExtendableOutput + Send> Hasher for Xof<T> {
    fn update(&mut self, data: &[u8]) {
        digest::Update::update(&mut self.0, data);
    }
    fn finish(self: Box<Self>) -> Vec<u8> {
        let mut out = vec![0u8; self.1];
        self.0.finalize_xof_into(&mut out);
        out
    }
}

struct Crc(crc32fast::Hasher);
impl Hasher for Crc {
    fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }
    fn finish(self: Box<Self>) -> Vec<u8> {
        self.0.finalize().to_be_bytes().to_vec()
    }
}

struct X64(xxhash_rust::xxh64::Xxh64);
impl Hasher for X64 {
    fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }
    fn finish(self: Box<Self>) -> Vec<u8> {
        self.0.digest().to_be_bytes().to_vec()
    }
}

// Xxh3 keeps a 600 byte buffer inline, boxed so the Vec of hashers stays small
struct X3(Box<xxhash_rust::xxh3::Xxh3>, bool);
impl Hasher for X3 {
    fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }
    fn finish(self: Box<Self>) -> Vec<u8> {
        if self.1 {
            self.0.digest128().to_be_bytes().to_vec()
        } else {
            self.0.digest().to_be_bytes().to_vec()
        }
    }
}

struct B3(blake3::Hasher);
impl Hasher for B3 {
    fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }
    fn finish(self: Box<Self>) -> Vec<u8> {
        self.0.finalize().as_bytes().to_vec()
    }
}

/// Windows password hash: MD4 over the UTF-16LE password
struct Ntlm(Vec<u8>);
impl Hasher for Ntlm {
    fn update(&mut self, data: &[u8]) {
        self.0.extend_from_slice(data);
    }
    fn finish(self: Box<Self>) -> Vec<u8> {
        let utf16: Vec<u8> = String::from_utf8_lossy(&self.0).encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        md4::Md4::digest(utf16).to_vec()
    }
}

/// feed the whole input to every hasher; the next block is read while the current one is hashed
pub fn run(mut hashers: Vec<Box<dyn Hasher>>, mut r: impl Read + Send, prog: Option<&Progress>) -> Res<Vec<Vec<u8>>> {
    let (tx, rx) = sync_channel::<std::io::Result<Vec<u8>>>(2);
    std::thread::scope(|s| {
        s.spawn(move || loop {
            let mut buf = vec![0u8; BLOCK];
            let n = match read_full(&mut r, &mut buf) {
                Ok(n) => n,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    return;
                }
            };
            buf.truncate(n);
            // the receiver is gone after an error / cancel
            if tx.send(Ok(buf)).is_err() || n < BLOCK {
                return;
            }
        });
        for block in rx {
            let block = block.map_err(err)?;
            if hashers.len() > 1 {
                hashers.par_iter_mut().for_each(|h| h.update(&block));
            } else if let Some(h) = hashers.first_mut() {
                h.update(&block);
            }
            tick(prog, block.len())?;
        }
        Ok::<(), String>(())
    })?;
    Ok(hashers.into_iter().map(|h| h.finish()).collect())
}

pub fn hash_reader(algos: &[Algo], r: impl Read + Send, prog: Option<&Progress>) -> Res<Vec<Vec<u8>>> {
    run(algos.iter().map(|a| a.hasher()).collect(), r, prog)
}

pub fn hash_bytes(algos: &[Algo], data: &[u8]) -> Vec<Vec<u8>> {
    algos
        .iter()
        .map(|a| {
            let mut h = a.hasher();
            h.update(data);
            h.finish()
        })
        .collect()
}

pub fn hash_one(algo: Algo, data: &[u8]) -> Vec<u8> {
    hash_bytes(&[algo], data).remove(0)
}

pub fn hash_file(algos: &[Algo], path: &Path, prog: Option<&Progress>) -> Res<Vec<Vec<u8>>> {
    let f = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if let Some(p) = prog {
        p.grow(f.metadata().map(|m| m.len()).unwrap_or(0));
    }
    hash_reader(algos, f, prog)
}

/// HMAC of the input for every algorithm that has one (None for the others)
pub fn hmac_reader(algos: &[Algo], key: &[u8], r: impl Read + Send, prog: Option<&Progress>) -> Res<Vec<Option<Vec<u8>>>> {
    let ok: Vec<Algo> = algos.iter().copied().filter(|a| a.hmac_ok()).collect();
    let mut out = run(ok.iter().map(|a| a.hmac(key).expect("hmac_ok")).collect(), r, prog)?.into_iter();
    Ok(algos.iter().map(|a| if a.hmac_ok() { out.next() } else { None }).collect())
}

pub fn hmac_file(algos: &[Algo], key: &[u8], path: &Path, prog: Option<&Progress>) -> Res<Vec<Option<Vec<u8>>>> {
    let f = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if let Some(p) = prog {
        p.grow(f.metadata().map(|m| m.len()).unwrap_or(0));
    }
    hmac_reader(algos, key, f, prog)
}

/// how a digest is shown
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Fmt {
    #[default]
    Hex,
    HexUpper,
    Base64,
}

pub fn show(bytes: &[u8], f: Fmt) -> String {
    match f {
        Fmt::Hex => hex::encode(bytes),
        Fmt::HexUpper => hex::encode_upper(bytes),
        Fmt::Base64 => crate::util::b64(bytes),
    }
}

/// does `expected` (hex or base64, any case, spaces ignored) equal `bytes`?
pub fn matches(bytes: &[u8], expected: &str) -> bool {
    let e = expected.trim();
    !e.is_empty() && crate::util::bytes_from_text(e).is_ok_and(|b| b == bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(a: Algo, data: &str) -> String {
        hex::encode(hash_one(a, data.as_bytes()))
    }

    #[test]
    fn known_answers() {
        // published test vectors (FIPS 180-4 / 202 examples, RFC 1320 / 1321, BLAKE2 / BLAKE3 / SM3 reference)
        assert_eq!(h(Algo::Md4, "abc"), "a448017aaf21d8525fc10ae87aa6729d");
        assert_eq!(h(Algo::Md5, "abc"), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(h(Algo::Sha1, "abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(h(Algo::Sha224, "abc"), "23097d223405d8228642a477bda255b32aadbce4bda0b3f7e36c9da7");
        assert_eq!(h(Algo::Sha256, "abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(
            h(Algo::Sha384, "abc"),
            "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed8086072ba1e7cc2358baeca134c825a7"
        );
        assert_eq!(
            h(Algo::Sha512, "abc"),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
        assert_eq!(h(Algo::Sha3_256, "abc"), "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532");
        assert_eq!(h(Algo::Keccak256, ""), "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470");
        assert_eq!(h(Algo::Shake128, ""), "7f9c2ba4e88f827d616045507605853ed73b8093f6efbc88eb1a6eacfa66ef26");
        assert_eq!(h(Algo::Ripemd160, "abc"), "8eb208f7e05d987a9b044a8e98c6b087f15a0bfc");
        assert_eq!(
            h(Algo::Blake2b, "abc"),
            "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d17d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923"
        );
        assert_eq!(h(Algo::Blake3, "abc"), "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85");
        assert_eq!(h(Algo::Sm3, "abc"), "66c7f0f462eeedd9d1f2d46bdc10e4e24167c4875cf2f7a2297da02b8f4ba8e0");
        assert_eq!(h(Algo::Crc32, "123456789"), "cbf43926");
        assert_eq!(h(Algo::Xxh64, ""), "ef46db3751d8e999");
        assert_eq!(h(Algo::Xxh3_64, ""), "2d06800538d394c2");
        assert_eq!(h(Algo::Ntlm, "password"), "8846f7eaee8fb117ad06bdd830b7586c");
        for a in Algo::ALL {
            assert_eq!(hash_one(a, b"x").len(), a.out_len(), "{}", a.name());
        }
    }

    #[test]
    fn hmac_rfc4231_and_wikipedia() {
        let msg = b"The quick brown fox jumps over the lazy dog";
        let r = hmac_reader(&[Algo::Sha256, Algo::Crc32, Algo::Md5], b"key", &msg[..], None).unwrap();
        assert_eq!(hex::encode(r[0].as_ref().unwrap()), "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8");
        assert!(r[1].is_none());
        assert_eq!(hex::encode(r[2].as_ref().unwrap()), "80070713463e7749b90c2dc24911e275");
    }

    #[test]
    fn streaming_equals_one_shot() {
        // bigger than one block, so the reader thread hands over several
        let data: Vec<u8> = (0..(BLOCK * 2 + 12345)).map(|i| (i * 31 % 251) as u8).collect();
        let algos = [Algo::Sha256, Algo::Blake3, Algo::Crc32, Algo::Shake256];
        let p = Progress::default();
        assert_eq!(hash_reader(&algos, &data[..], Some(&p)).unwrap(), hash_bytes(&algos, &data));
        assert_eq!(p.done.load(std::sync::atomic::Ordering::Relaxed), data.len() as u64);
    }

    #[test]
    fn names_parse_back() {
        for a in Algo::ALL {
            assert_eq!(Algo::parse(a.name()), Some(a));
        }
        assert_eq!(Algo::parse("sha256"), Some(Algo::Sha256));
        assert_eq!(Algo::parse("SHA2-512"), Some(Algo::Sha512));
        assert_eq!(Algo::parse("sha3_256"), Some(Algo::Sha3_256));
        assert_eq!(Algo::parse("blake2b"), Some(Algo::Blake2b));
        assert_eq!(Algo::parse("nope"), None);
    }

    #[test]
    fn compare() {
        let d = hash_one(Algo::Sha256, b"abc");
        assert!(matches(&d, "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD"));
        assert!(matches(&d, &crate::util::b64(&d)));
        assert!(!matches(&d, "ba78"));
    }
}
