// "What kind of hash is this?" - by prefix, length and alphabet. With a guess for the plaintext
// it can also tell for sure, by hashing the guess with every algorithm.

use crate::hash::{hash_bytes, Algo};
use crate::util::{b64_decode, is_hex};

pub struct Guess {
    pub name: String,
    /// the usual suspect for this shape
    pub likely: bool,
}

fn g(name: &str, likely: bool) -> Guess {
    Guess { name: name.into(), likely }
}

/// candidates, most likely first
pub fn identify(input: &str) -> Vec<Guess> {
    let s = input.trim();
    if s.is_empty() {
        return vec![];
    }
    let prefixed: &[(&str, &str)] = &[
        ("$argon2id$", "Argon2id"),
        ("$argon2i$", "Argon2i"),
        ("$argon2d$", "Argon2d"),
        ("$2a$", "bcrypt"),
        ("$2b$", "bcrypt"),
        ("$2y$", "bcrypt (PHP)"),
        ("$2x$", "bcrypt (old PHP)"),
        ("$scrypt$", "scrypt (PHC)"),
        ("$7$", "scrypt (crypt)"),
        ("$pbkdf2-sha256$", "PBKDF2-SHA256"),
        ("$pbkdf2-sha512$", "PBKDF2-SHA512"),
        ("$pbkdf2$", "PBKDF2-SHA1"),
        ("pbkdf2_sha256$", "PBKDF2-SHA256 (Django)"),
        ("$6$", "sha512-crypt (Linux)"),
        ("$5$", "sha256-crypt (Linux)"),
        ("$1$", "md5-crypt"),
        ("$apr1$", "Apache md5-crypt (htpasswd)"),
        ("$y$", "yescrypt (Linux)"),
        ("$gy$", "gost-yescrypt"),
        ("$P$", "phpass (WordPress)"),
        ("$H$", "phpass (phpBB)"),
        ("{SSHA}", "salted SHA-1 (LDAP)"),
        ("{SHA}", "SHA-1 Base64 (LDAP)"),
        ("{SSHA512}", "salted SHA-512 (LDAP)"),
        ("sha1$", "SHA-1 (Django)"),
    ];
    if let Some((_, name)) = prefixed.iter().find(|(p, _)| s.starts_with(p)) {
        return vec![g(name, true)];
    }
    if s.len() == 41 && s.starts_with('*') && is_hex(&s[1..]) {
        return vec![g("MySQL 4.1+ (SHA-1 of SHA-1)", true)];
    }
    if is_hex(s) {
        return by_len(s.len() / 2, "");
    }
    // the same lengths, Base64 encoded
    if s.len() >= 8 && s.chars().all(|c| c.is_ascii_alphanumeric() || "+/=-_".contains(c)) {
        if let Ok(b) = b64_decode(s) {
            let found = by_len(b.len(), " (Base64)");
            if !found.is_empty() {
                return found;
            }
        }
    }
    vec![]
}

fn by_len(bytes: usize, suffix: &str) -> Vec<Guess> {
    let names: &[&str] = match bytes {
        4 => &["CRC32", "Adler-32", "FNV-1a 32"],
        8 => &["xxHash64 / XXH3-64", "CRC-64", "MySQL 3.23 (OLD_PASSWORD)", "half MD5"],
        16 => &["MD5", "NTLM (Windows)", "MD4", "XXH3-128", "RIPEMD-128", "LM (Windows, very old)"],
        20 => &["SHA-1", "RIPEMD-160", "MySQL 4.1+ without *", "Tiger-160"],
        24 => &["Tiger", "Tiger2"],
        28 => &["SHA-224", "SHA3-224", "SHA-512/224"],
        32 => &[
            "SHA-256",
            "SHA3-256",
            "Keccak-256 (Ethereum)",
            "BLAKE2s-256",
            "BLAKE3",
            "SHA-512/256",
            "SM3",
            "Streebog-256",
            "SHAKE128 (256 bits)",
            "RIPEMD-256",
        ],
        40 => &["RIPEMD-320"],
        48 => &["SHA-384", "SHA3-384"],
        64 => &["SHA-512", "SHA3-512", "BLAKE2b-512", "Whirlpool", "Streebog-512", "SHAKE256 (512 bits)"],
        _ => &[],
    };
    names.iter().enumerate().map(|(i, n)| g(&format!("{n}{suffix}"), i == 0)).collect()
}

/// which algorithms turn `text` into `hash` (hex or Base64; password hashes are verified)
pub fn find_match(hash: &str, text: &str) -> Vec<String> {
    let h = hash.trim();
    if crate::pw::detect(h).is_some() {
        return match crate::pw::verify(h, text) {
            Ok(true) => vec![crate::pw::detect(h).unwrap().name().to_string()],
            _ => vec![],
        };
    }
    if let Some(rest) = h.strip_prefix('*') {
        let inner = crate::hash::hash_one(Algo::Sha1, text.as_bytes());
        let outer = crate::hash::hash_one(Algo::Sha1, &inner);
        return if crate::hash::matches(&outer, rest) { vec!["MySQL 4.1+".into()] } else { vec![] };
    }
    let all = Algo::ALL;
    let digests = hash_bytes(&all, text.as_bytes());
    all.iter().zip(digests).filter(|(_, d)| crate::hash::matches(d, h)).map(|(a, _)| a.name().to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes() {
        assert_eq!(identify("5f4dcc3b5aa765d61d8327deb882cf99")[0].name, "MD5");
        assert_eq!(identify("5baa61e4c9b93f3f0682250b6cf8331b7ee68fd8")[0].name, "SHA-1");
        assert_eq!(identify("5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8")[0].name, "SHA-256");
        assert_eq!(identify("XohImNooBHFR0OVvjcYpJ3NgPQ1qq73WKhHvch0VQtg=")[0].name, "SHA-256 (Base64)");
        assert_eq!(identify("$2b$12$abcdefghijklmnopqrstuu")[0].name, "bcrypt");
        assert_eq!(identify("*2470C0C06DEE42FD1618BB99005ADCA2EC9D1E19")[0].name, "MySQL 4.1+ (SHA-1 of SHA-1)");
        assert!(identify("hello there").is_empty());
    }

    #[test]
    fn finds_the_algorithm() {
        assert_eq!(find_match("5f4dcc3b5aa765d61d8327deb882cf99", "password"), vec!["MD5"]);
        assert_eq!(find_match("8846F7EAEE8FB117AD06BDD830B7586C", "password"), vec!["NTLM"]);
        assert_eq!(find_match("*2470C0C06DEE42FD1618BB99005ADCA2EC9D1E19", "password"), vec!["MySQL 4.1+"]);
        assert!(find_match("5f4dcc3b5aa765d61d8327deb882cf99", "Password").is_empty());
        let stored = crate::pw::hash(crate::pw::PwAlgo::Md5Crypt, "pw").unwrap();
        assert_eq!(find_match(&stored, "pw"), vec!["md5-crypt"]);
    }
}
