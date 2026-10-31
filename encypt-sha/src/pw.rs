// Passwords: hash + verify the way websites / Linux store them (Argon2, bcrypt, scrypt, PBKDF2,
// sha512-crypt ...), generate random ones and estimate how strong one is.

use crate::util::{err, Res};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use rand::rngs::OsRng;
use rand::seq::SliceRandom;
use rand::Rng;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PwAlgo {
    Argon2id,
    Argon2i,
    Argon2d,
    Bcrypt,
    Scrypt,
    Pbkdf2Sha256,
    Pbkdf2Sha512,
    Sha512Crypt,
    Sha256Crypt,
    Md5Crypt,
}

impl PwAlgo {
    pub const ALL: [PwAlgo; 10] = [
        PwAlgo::Argon2id,
        PwAlgo::Argon2i,
        PwAlgo::Argon2d,
        PwAlgo::Bcrypt,
        PwAlgo::Scrypt,
        PwAlgo::Pbkdf2Sha256,
        PwAlgo::Pbkdf2Sha512,
        PwAlgo::Sha512Crypt,
        PwAlgo::Sha256Crypt,
        PwAlgo::Md5Crypt,
    ];

    pub fn name(self) -> &'static str {
        match self {
            PwAlgo::Argon2id => "Argon2id",
            PwAlgo::Argon2i => "Argon2i",
            PwAlgo::Argon2d => "Argon2d",
            PwAlgo::Bcrypt => "bcrypt",
            PwAlgo::Scrypt => "scrypt",
            PwAlgo::Pbkdf2Sha256 => "PBKDF2-SHA256",
            PwAlgo::Pbkdf2Sha512 => "PBKDF2-SHA512",
            PwAlgo::Sha512Crypt => "sha512-crypt",
            PwAlgo::Sha256Crypt => "sha256-crypt",
            PwAlgo::Md5Crypt => "md5-crypt",
        }
    }

    /// settings used, shown next to the name
    pub fn settings(self) -> &'static str {
        match self {
            PwAlgo::Argon2id | PwAlgo::Argon2i | PwAlgo::Argon2d => "19 MiB, 2 passes (OWASP)",
            PwAlgo::Bcrypt => "cost 12, max. 72 bytes",
            PwAlgo::Scrypt => "N=2^17, r=8, p=1",
            PwAlgo::Pbkdf2Sha256 => "600,000 rounds (OWASP)",
            PwAlgo::Pbkdf2Sha512 => "210,000 rounds (OWASP)",
            PwAlgo::Sha512Crypt | PwAlgo::Sha256Crypt => "Linux /etc/shadow ($6$ / $5$)",
            PwAlgo::Md5Crypt => "legacy ($1$), fast to crack",
        }
    }

    pub fn parse(s: &str) -> Option<PwAlgo> {
        let norm = |x: &str| x.to_ascii_lowercase().replace(['-', '_', ' '], "");
        let want = match norm(s).as_str() {
            "argon2" | "argon" => "argon2id".to_string(),
            "pbkdf2" => "pbkdf2sha256".into(),
            "sha512crypt" | "shacrypt" | "shadow" | "6" => "sha512crypt".into(),
            other => other.to_string(),
        };
        PwAlgo::ALL.into_iter().find(|a| norm(a.name()) == want)
    }
}

// the old crypt schemes are offered to match existing systems, not for new ones
#[allow(deprecated)]
pub fn hash(algo: PwAlgo, password: &str) -> Res<String> {
    let pw = password.as_bytes();
    let salt = SaltString::generate(&mut OsRng);
    let argon = |a: argon2::Algorithm| {
        argon2::Argon2::new(a, argon2::Version::V0x13, argon2::Params::default()).hash_password(pw, &salt).map(|h| h.to_string()).map_err(err)
    };
    let pbkdf2 = |a: pbkdf2::Algorithm, rounds: u32, len: usize| {
        let params = pbkdf2::Params { rounds, output_length: len };
        pbkdf2::Pbkdf2.hash_password_customized(pw, Some(a.ident()), None, params, &salt).map(|h| h.to_string()).map_err(err)
    };
    match algo {
        PwAlgo::Argon2id => argon(argon2::Algorithm::Argon2id),
        PwAlgo::Argon2i => argon(argon2::Algorithm::Argon2i),
        PwAlgo::Argon2d => argon(argon2::Algorithm::Argon2d),
        PwAlgo::Bcrypt => {
            if pw.len() > 72 {
                return Err("bcrypt only uses the first 72 bytes of a password - pick Argon2id for longer ones".into());
            }
            bcrypt::hash(pw, 12).map_err(err)
        }
        PwAlgo::Scrypt => scrypt::Scrypt.hash_password(pw, &salt).map(|h| h.to_string()).map_err(err),
        PwAlgo::Pbkdf2Sha256 => pbkdf2(pbkdf2::Algorithm::Pbkdf2Sha256, 600_000, 32),
        PwAlgo::Pbkdf2Sha512 => pbkdf2(pbkdf2::Algorithm::Pbkdf2Sha512, 210_000, 64),
        PwAlgo::Sha512Crypt => pwhash::sha512_crypt::hash(pw).map_err(err),
        PwAlgo::Sha256Crypt => pwhash::sha256_crypt::hash(pw).map_err(err),
        PwAlgo::Md5Crypt => pwhash::md5_crypt::hash(pw).map_err(err),
    }
}

/// which scheme a stored hash uses, by its prefix
pub fn detect(stored: &str) -> Option<PwAlgo> {
    let s = stored.trim();
    Some(match () {
        _ if s.starts_with("$argon2id$") => PwAlgo::Argon2id,
        _ if s.starts_with("$argon2i$") => PwAlgo::Argon2i,
        _ if s.starts_with("$argon2d$") => PwAlgo::Argon2d,
        _ if ["$2a$", "$2b$", "$2x$", "$2y$"].iter().any(|p| s.starts_with(p)) => PwAlgo::Bcrypt,
        _ if s.starts_with("$scrypt$") => PwAlgo::Scrypt,
        _ if s.starts_with("$pbkdf2-sha256$") || s.starts_with("$pbkdf2$") => PwAlgo::Pbkdf2Sha256,
        _ if s.starts_with("$pbkdf2-sha512$") => PwAlgo::Pbkdf2Sha512,
        _ if s.starts_with("$6$") => PwAlgo::Sha512Crypt,
        _ if s.starts_with("$5$") => PwAlgo::Sha256Crypt,
        _ if s.starts_with("$1$") => PwAlgo::Md5Crypt,
        _ => return None,
    })
}

/// does the password belong to the stored hash?
pub fn verify(stored: &str, password: &str) -> Res<bool> {
    let s = stored.trim();
    let pw = password.as_bytes();
    let algo = detect(s).ok_or("unknown hash format (Argon2, bcrypt, scrypt, PBKDF2 and $1$ / $5$ / $6$ crypt work)")?;
    let phc = || PasswordHash::new(s).map_err(err);
    Ok(match algo {
        PwAlgo::Argon2id | PwAlgo::Argon2i | PwAlgo::Argon2d => argon2::Argon2::default().verify_password(pw, &phc()?).is_ok(),
        PwAlgo::Scrypt => scrypt::Scrypt.verify_password(pw, &phc()?).is_ok(),
        PwAlgo::Pbkdf2Sha256 | PwAlgo::Pbkdf2Sha512 => pbkdf2::Pbkdf2.verify_password(pw, &phc()?).is_ok(),
        PwAlgo::Bcrypt => bcrypt::verify(pw, s).map_err(err)?,
        PwAlgo::Sha512Crypt | PwAlgo::Sha256Crypt | PwAlgo::Md5Crypt => pwhash::unix::verify(pw, s),
    })
}

// ---------------- generator ----------------

const LOWER: &str = "abcdefghijklmnopqrstuvwxyz";
const UPPER: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &str = "0123456789";
const SYMBOLS: &str = "!#$%&()*+,-./:;<=>?@[]^_{|}~";
const AMBIGUOUS: &str = "Il1O0o";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GenOpts {
    pub len: usize,
    pub lower: bool,
    pub upper: bool,
    pub digits: bool,
    pub symbols: bool,
    /// leave out I l 1 O 0 o (easier to type off paper)
    pub no_ambiguous: bool,
}

impl Default for GenOpts {
    fn default() -> GenOpts {
        GenOpts { len: 20, lower: true, upper: true, digits: true, symbols: true, no_ambiguous: false }
    }
}

fn sets(o: &GenOpts) -> Vec<Vec<char>> {
    [(o.lower, LOWER), (o.upper, UPPER), (o.digits, DIGITS), (o.symbols, SYMBOLS)]
        .into_iter()
        .filter(|(on, _)| *on)
        .map(|(_, s)| s.chars().filter(|c| !(o.no_ambiguous && AMBIGUOUS.contains(*c))).collect())
        .collect()
}

/// bits of randomness in a generated password
pub fn gen_bits(o: &GenOpts) -> f64 {
    let n: usize = sets(o).iter().map(|s| s.len()).sum();
    if n < 2 {
        return 0.0;
    }
    o.len as f64 * (n as f64).log2()
}

/// every picked character class shows up at least once; uniform choice from the OS random generator
pub fn generate(o: &GenOpts) -> Res<String> {
    let sets = sets(o);
    if sets.is_empty() {
        return Err("pick at least one kind of character".into());
    }
    if o.len < sets.len() || o.len > 1024 {
        return Err(format!("length must be {} to 1024", sets.len()));
    }
    let all: Vec<char> = sets.concat();
    let mut rng = OsRng;
    // redraw until every class is in it (keeps the result uniform among valid passwords)
    loop {
        let pw: Vec<char> = (0..o.len).map(|_| all[rng.gen_range(0..all.len())]).collect();
        if sets.iter().all(|s| pw.iter().any(|c| s.contains(c))) {
            return Ok(pw.into_iter().collect());
        }
    }
}

/// random words would need a word list; this makes pronounceable syllables instead
pub fn passphrase(words: usize) -> String {
    const CONS: &[u8] = b"bcdfghjklmnprstvwz";
    const VOW: &[u8] = b"aeiou";
    let mut rng = OsRng;
    let word = |rng: &mut OsRng| -> String {
        (0..3).flat_map(|_| [*CONS.choose(rng).unwrap() as char, *VOW.choose(rng).unwrap() as char]).collect()
    };
    (0..words.max(1)).map(|_| word(&mut rng)).collect::<Vec<_>>().join("-")
}

pub fn passphrase_bits(words: usize) -> f64 {
    words.max(1) as f64 * 3.0 * ((18.0f64).log2() + (5.0f64).log2())
}

// ---------------- strength ----------------

const COMMON: &[&str] = &[
    "123456", "password", "12345678", "qwerty", "123456789", "12345", "1234", "111111", "1234567", "dragon", "123123", "baseball",
    "abc123", "football", "monkey", "letmein", "696969", "shadow", "master", "666666", "qwertyuiop", "123321", "mustang",
    "1234567890", "michael", "654321", "superman", "1qaz2wsx", "7777777", "121212", "000000", "qazwsx", "123qwe", "killer",
    "trustno1", "jordan", "jennifer", "zxcvbnm", "asdfgh", "hunter", "buster", "soccer", "harley", "batman", "andrew", "tigger",
    "sunshine", "iloveyou", "2000", "charlie", "robert", "thomas", "hockey", "ranger", "daniel", "starwars", "klaster", "112233",
    "george", "computer", "michelle", "jessica", "pepper", "1111", "zxcvbn", "555555", "11111111", "131313", "freedom", "777777",
    "pass", "maggie", "159753", "aaaaaa", "ginger", "princess", "joshua", "cheese", "amanda", "summer", "love", "ashley", "nicole",
    "chelsea", "biteme", "matthew", "access", "yankees", "987654321", "dallas", "austin", "thunder", "taylor", "matrix", "passwort",
    "hallo", "hallo123", "schatz", "ficken", "geheim", "admin", "welcome", "login", "passw0rd", "p@ssw0rd", "qwertz", "asdf",
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Strength {
    pub bits: f64,
    /// 0 very weak .. 4 very strong
    pub level: u8,
    /// seconds for an offline attack at 10 billion guesses per second (fast hash, one GPU)
    pub crack_secs: f64,
}

/// rough estimate: character pool x length, minus repeats, sequences and known passwords
pub fn strength(pw: &str) -> Strength {
    let chars: Vec<char> = pw.chars().collect();
    if chars.is_empty() {
        return Strength { bits: 0.0, level: 0, crack_secs: 0.0 };
    }
    let lower = pw.to_lowercase();
    let base = lower.trim_end_matches(|c: char| c.is_ascii_digit() || "!?.".contains(c));
    if COMMON.contains(&lower.as_str()) || COMMON.contains(&base) {
        return Strength { bits: 4.0, level: 0, crack_secs: 0.0 };
    }
    let mut pool = 0u32;
    if chars.iter().any(|c| c.is_ascii_lowercase()) {
        pool += 26;
    }
    if chars.iter().any(|c| c.is_ascii_uppercase()) {
        pool += 26;
    }
    if chars.iter().any(|c| c.is_ascii_digit()) {
        pool += 10;
    }
    if chars.iter().any(|c| c.is_ascii_punctuation() || *c == ' ') {
        pool += 33;
    }
    if chars.iter().any(|c| !c.is_ascii()) {
        pool += 100;
    }
    // a character equal to or one step from the previous one adds almost nothing ("aaaa", "1234", "abcd")
    let mut effective = 1.0;
    for w in chars.windows(2) {
        let d = (w[1] as i64 - w[0] as i64).abs();
        effective += if d <= 1 { 0.25 } else { 1.0 };
    }
    let bits = effective * (pool.max(2) as f64).log2();
    let level = match bits {
        b if b < 28.0 => 0,
        b if b < 40.0 => 1,
        b if b < 60.0 => 2,
        b if b < 90.0 => 3,
        _ => 4,
    };
    Strength { bits, level, crack_secs: 2f64.powf(bits - 1.0) / 1e10 }
}

/// "3 hours", "200 years", ...
pub fn duration(secs: f64, de: bool) -> String {
    let t = |g: &'static str, e: &'static str| if de { g } else { e };
    if secs < 1.0 {
        return t("sofort", "instantly").into();
    }
    let units = [
        (60.0, t("Sekunden", "seconds")),
        (60.0, t("Minuten", "minutes")),
        (24.0, t("Stunden", "hours")),
        (365.25, t("Tage", "days")),
        (1e3, t("Jahre", "years")),
        (1e3, t("Jahrtausende", "millennia")),
        (1e3, t("Mio. Jahre", "million years")),
    ];
    let mut v = secs;
    for (step, name) in units {
        if v < step {
            return format!("{:.0} {name}", v.max(1.0));
        }
        v /= step;
    }
    // past ~10^9 years
    t("länger als das Universum alt ist", "longer than the universe exists").into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_every_scheme() {
        for a in PwAlgo::ALL {
            let h = hash(a, "correct horse").unwrap();
            assert_eq!(detect(&h), Some(a), "{} -> {h}", a.name());
            assert!(verify(&h, "correct horse").unwrap(), "{}", a.name());
            assert!(!verify(&h, "correct horsE").unwrap(), "{}", a.name());
        }
    }

    #[test]
    fn known_hashes_from_other_tools() {
        // `openssl passwd -6 -salt saltsalt password`
        assert!(verify("$6$saltsalt$qFmFH.bQmmtXzyBY0s9v7Oicd2z4XSIecDzlB5KiA2/jctKu9YterLp8wwnSq.qc.eoxqOmSuNp2xS0ktL3nh/", "password").unwrap());
        // `openssl passwd -5 -salt saltsalt password`
        assert!(verify("$5$saltsalt$gOjOtoMpVhru2uyjeJSEc/JaLQWOXMNmlOnj6T4AtC.", "password").unwrap());
        // `openssl passwd -1 -salt saltsalt password`
        assert!(verify("$1$saltsalt$qjXMvbEw8oaL.CzflDtaK/", "password").unwrap());
        assert!(!verify("$1$saltsalt$qjXMvbEw8oaL.CzflDtaK/", "Password").unwrap());
        // bcrypt reference ($2a$, cost 5)
        assert!(verify("$2a$05$CCCCCCCCCCCCCCCCCCCCC.E5YPO9kmyuRGyh0XouQYb4YMJKvyOeW", "U*U").unwrap());
        assert!(verify("$argon2id$v=19$m=65536,t=2,p=1$c29tZXNhbHQ$CTFhFdXPJO1aFaMaO6Mm5c8y7cJHAph8ArZWb2GRPPc", "password").unwrap());
    }

    #[test]
    fn generator() {
        let o = GenOpts { len: 12, lower: true, upper: false, digits: true, symbols: false, no_ambiguous: true };
        for _ in 0..200 {
            let p = generate(&o).unwrap();
            assert_eq!(p.chars().count(), 12);
            assert!(p.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
            assert!(p.chars().any(|c| c.is_ascii_digit()) && p.chars().any(|c| c.is_ascii_lowercase()));
            assert!(!p.contains(['l', '1', '0', 'o']));
        }
        assert!(generate(&GenOpts { lower: false, upper: false, digits: false, symbols: false, ..o }).is_err());
        assert!((gen_bits(&GenOpts::default()) - 20.0 * 90f64.log2()).abs() < 1e-9);
        assert_eq!(passphrase(4).split('-').count(), 4);
    }

    #[test]
    fn strength_ranks_sensibly() {
        assert_eq!(strength("password1").level, 0);
        assert_eq!(strength("aaaaaaaaaaaa").level, 0);
        assert!(strength("Tr0ub4dor&3").level >= 2);
        assert_eq!(strength(&generate(&GenOpts::default()).unwrap()).level, 4);
        assert_eq!(duration(0.5, false), "instantly");
        assert_eq!(duration(7200.0, false), "2 hours");
    }
}
