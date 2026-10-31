// Encodings (Base64, Base32, Base58, hex, URL, binary) and the classic ciphers (Caesar, ROT13,
// Vigenère, XOR, Morse ...). The classic ones are puzzles, not protection.

use crate::util::{b64, b64_decode, hex_decode, Res};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Method {
    Base64,
    Base64Url,
    Base32,
    Base58,
    Hex,
    Url,
    Binary,
    Caesar,
    Rot13,
    Rot47,
    Atbash,
    Vigenere,
    Xor,
    Morse,
}

impl Method {
    pub const ALL: [Method; 14] = [
        Method::Base64,
        Method::Base64Url,
        Method::Base32,
        Method::Base58,
        Method::Hex,
        Method::Url,
        Method::Binary,
        Method::Caesar,
        Method::Rot13,
        Method::Rot47,
        Method::Atbash,
        Method::Vigenere,
        Method::Xor,
        Method::Morse,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Method::Base64 => "Base64",
            Method::Base64Url => "Base64 URL",
            Method::Base32 => "Base32",
            Method::Base58 => "Base58",
            Method::Hex => "Hex",
            Method::Url => "URL (%20)",
            Method::Binary => "Binary",
            Method::Caesar => "Caesar",
            Method::Rot13 => "ROT13",
            Method::Rot47 => "ROT47",
            Method::Atbash => "Atbash",
            Method::Vigenere => "Vigenère",
            Method::Xor => "XOR",
            Method::Morse => "Morse",
        }
    }

    pub fn parse(s: &str) -> Option<Method> {
        let want = s.to_ascii_lowercase().replace(['-', '_', ' '], "");
        Some(match want.as_str() {
            "base64" | "b64" => Method::Base64,
            "base64url" | "b64url" | "url64" => Method::Base64Url,
            "base32" | "b32" => Method::Base32,
            "base58" | "b58" => Method::Base58,
            "hex" | "base16" => Method::Hex,
            "url" | "percent" | "urlencode" => Method::Url,
            "binary" | "bin" | "bits" => Method::Binary,
            "caesar" | "cesar" | "shift" => Method::Caesar,
            "rot13" => Method::Rot13,
            "rot47" => Method::Rot47,
            "atbash" => Method::Atbash,
            "vigenere" | "vigenère" => Method::Vigenere,
            "xor" => Method::Xor,
            "morse" => Method::Morse,
            _ => return None,
        })
    }

    /// classic cipher (true) or encoding (false)
    pub fn classic(self) -> bool {
        matches!(self, Method::Caesar | Method::Rot13 | Method::Rot47 | Method::Atbash | Method::Vigenere | Method::Xor | Method::Morse)
    }

    /// what the key field means, None = no key
    pub fn key_hint(self) -> Option<&'static str> {
        match self {
            Method::Caesar => Some("shift (3)"),
            Method::Vigenere => Some("key word"),
            Method::Xor => Some("key"),
            _ => None,
        }
    }
}

pub fn encode(m: Method, input: &str, key: &str) -> Res<String> {
    let b = input.as_bytes();
    Ok(match m {
        Method::Base64 => b64(b),
        Method::Base64Url => {
            use base64::Engine;
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b)
        }
        Method::Base32 => data_encoding::BASE32.encode(b),
        Method::Base58 => bs58::encode(b).into_string(),
        Method::Hex => hex::encode(b),
        Method::Url => url_encode(input),
        Method::Binary => b.iter().map(|x| format!("{x:08b}")).collect::<Vec<_>>().join(" "),
        Method::Caesar => shift(input, caesar_key(key)?),
        Method::Rot13 => shift(input, 13),
        Method::Rot47 => rot47(input),
        Method::Atbash => atbash(input),
        Method::Vigenere => vigenere(input, key, true)?,
        Method::Xor => hex::encode(xor(b, key)?),
        Method::Morse => morse_encode(input),
    })
}

/// bytes, because decoded Base64 / hex / XOR doesn't have to be text
pub fn decode(m: Method, input: &str, key: &str) -> Res<Vec<u8>> {
    Ok(match m {
        Method::Base64 | Method::Base64Url => b64_decode(input)?,
        Method::Base32 => {
            let clean: String = input.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_ascii_uppercase();
            let spec = if clean.contains('=') { data_encoding::BASE32 } else { data_encoding::BASE32_NOPAD };
            spec.decode(clean.as_bytes()).map_err(|e| format!("not valid Base32: {e}"))?
        }
        Method::Base58 => bs58::decode(input.trim()).into_vec().map_err(|e| format!("not valid Base58: {e}"))?,
        Method::Hex => hex_decode(input)?,
        Method::Url => url_decode(input)?,
        Method::Binary => {
            let bits: String = input.chars().filter(|c| !c.is_whitespace()).collect();
            if !bits.len().is_multiple_of(8) || !bits.chars().all(|c| c == '0' || c == '1') {
                return Err("binary needs groups of 8 zeros / ones".into());
            }
            bits.as_bytes().chunks(8).map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap(), 2).unwrap()).collect()
        }
        Method::Caesar => shift(input, -caesar_key(key)?).into_bytes(),
        Method::Rot13 => shift(input, 13).into_bytes(),
        Method::Rot47 => rot47(input).into_bytes(),
        Method::Atbash => atbash(input).into_bytes(),
        Method::Vigenere => vigenere(input, key, false)?.into_bytes(),
        Method::Xor => xor(&hex_decode(input)?, key)?,
        Method::Morse => morse_decode(input)?.into_bytes(),
    })
}

/// decoded bytes as text, or as hex when they aren't text
pub fn show(bytes: &[u8]) -> (String, bool) {
    match std::str::from_utf8(bytes) {
        Ok(s) if !s.chars().any(|c| c.is_control() && !"\n\r\t".contains(c)) => (s.to_string(), true),
        _ => (hex::encode(bytes), false),
    }
}

fn url_encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn url_decode(s: &str) -> Res<Vec<u8>> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let h = s.get(i + 1..i + 3).ok_or("% must be followed by two hex digits")?;
            out.push(u8::from_str_radix(h, 16).map_err(|_| format!("bad escape %{h}"))?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    Ok(out)
}

fn caesar_key(key: &str) -> Res<i32> {
    let k = key.trim();
    if k.is_empty() {
        return Ok(3);
    }
    k.parse::<i32>().map_err(|_| "the Caesar key is a number (the shift)".to_string())
}

/// shift A-Z / a-z, everything else stays
fn shift(s: &str, by: i32) -> String {
    let by = by.rem_euclid(26) as u8;
    s.chars()
        .map(|c| match c {
            'a'..='z' => ((c as u8 - b'a' + by) % 26 + b'a') as char,
            'A'..='Z' => ((c as u8 - b'A' + by) % 26 + b'A') as char,
            c => c,
        })
        .collect()
}

fn rot47(s: &str) -> String {
    s.chars().map(|c| if ('!'..='~').contains(&c) { (33 + (c as u8 - 33 + 47) % 94) as char } else { c }).collect()
}

fn atbash(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'a'..='z' => (b'z' - (c as u8 - b'a')) as char,
            'A'..='Z' => (b'Z' - (c as u8 - b'A')) as char,
            c => c,
        })
        .collect()
}

fn vigenere(s: &str, key: &str, enc: bool) -> Res<String> {
    let k: Vec<u8> = key.chars().filter(|c| c.is_ascii_alphabetic()).map(|c| c.to_ascii_lowercase() as u8 - b'a').collect();
    if k.is_empty() {
        return Err("Vigenère needs a key word (letters)".into());
    }
    let mut i = 0;
    Ok(s
        .chars()
        .map(|c| {
            if !c.is_ascii_alphabetic() {
                return c;
            }
            // the key only moves on letters
            let by = k[i % k.len()] as i32;
            i += 1;
            shift(&c.to_string(), if enc { by } else { -by }).chars().next().unwrap()
        })
        .collect())
}

fn xor(data: &[u8], key: &str) -> Res<Vec<u8>> {
    if key.is_empty() {
        return Err("XOR needs a key".into());
    }
    let k = key.as_bytes();
    Ok(data.iter().zip(k.iter().cycle()).map(|(d, k)| d ^ k).collect())
}

const MORSE: &[(char, &str)] = &[
    ('A', ".-"),
    ('B', "-..."),
    ('C', "-.-."),
    ('D', "-.."),
    ('E', "."),
    ('F', "..-."),
    ('G', "--."),
    ('H', "...."),
    ('I', ".."),
    ('J', ".---"),
    ('K', "-.-"),
    ('L', ".-.."),
    ('M', "--"),
    ('N', "-."),
    ('O', "---"),
    ('P', ".--."),
    ('Q', "--.-"),
    ('R', ".-."),
    ('S', "..."),
    ('T', "-"),
    ('U', "..-"),
    ('V', "...-"),
    ('W', ".--"),
    ('X', "-..-"),
    ('Y', "-.--"),
    ('Z', "--.."),
    ('0', "-----"),
    ('1', ".----"),
    ('2', "..---"),
    ('3', "...--"),
    ('4', "....-"),
    ('5', "....."),
    ('6', "-...."),
    ('7', "--..."),
    ('8', "---.."),
    ('9', "----."),
    ('.', ".-.-.-"),
    (',', "--..--"),
    ('?', "..--.."),
    ('\'', ".----."),
    ('!', "-.-.--"),
    ('/', "-..-."),
    ('(', "-.--."),
    (')', "-.--.-"),
    ('&', ".-..."),
    (':', "---..."),
    (';', "-.-.-."),
    ('=', "-...-"),
    ('+', ".-.-."),
    ('-', "-....-"),
    ('_', "..--.-"),
    ('"', ".-..-."),
    ('$', "...-..-"),
    ('@', ".--.-."),
    ('Ä', ".-.-"),
    ('Ö', "---."),
    ('Ü', "..--"),
    ('ß', "...--.."),
];

/// letters separated by spaces, words by " / "; unknown characters become '?'
fn morse_encode(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            w.chars()
                .map(|c| {
                    // ß first, its uppercase would be "SS"
                    let up = c.to_uppercase().next().unwrap_or(c);
                    let find = |x: char| MORSE.iter().find(|(ch, _)| *ch == x).map(|(_, m)| *m);
                    find(c).or_else(|| find(up)).unwrap_or("..--..")
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join(" / ")
}

fn morse_decode(s: &str) -> Res<String> {
    let mut out = String::new();
    for (i, word) in s.split('/').enumerate() {
        if i > 0 {
            out.push(' ');
        }
        for code in word.split_whitespace() {
            let code = code.replace(['·', '•'], ".").replace(['−', '–', '_'], "-");
            let c = MORSE.iter().find(|(_, m)| *m == code).map(|(c, _)| *c).ok_or(format!("unknown Morse code \"{code}\""))?;
            out.push(c);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round(m: Method, s: &str, key: &str) -> String {
        let e = encode(m, s, key).unwrap();
        String::from_utf8(decode(m, &e, key).unwrap()).unwrap()
    }

    #[test]
    fn every_method_round_trips() {
        let s = "Hello, World! Grüße 123";
        for m in Method::ALL {
            let key = match m {
                Method::Caesar => "5",
                Method::Vigenere => "LEMON",
                Method::Xor => "k3y",
                _ => "",
            };
            let back = round(m, s, key);
            if m == Method::Morse {
                // Morse has no lower case
                assert_eq!(back, "HELLO, WORLD! GRÜßE 123", "{}", m.name());
            } else {
                assert_eq!(back, s, "{}", m.name());
            }
        }
    }

    #[test]
    fn known_values() {
        assert_eq!(encode(Method::Base64, "hello", "").unwrap(), "aGVsbG8=");
        assert_eq!(encode(Method::Base32, "hello", "").unwrap(), "NBSWY3DP");
        assert_eq!(encode(Method::Base58, "hello", "").unwrap(), "Cn8eVZg");
        assert_eq!(encode(Method::Url, "a b&c=ü", "").unwrap(), "a%20b%26c%3D%C3%BC");
        assert_eq!(encode(Method::Binary, "Hi", "").unwrap(), "01001000 01101001");
        assert_eq!(encode(Method::Caesar, "abc XYZ", "").unwrap(), "def ABC");
        assert_eq!(encode(Method::Rot13, "Hello", "").unwrap(), "Uryyb");
        assert_eq!(encode(Method::Atbash, "abc", "").unwrap(), "zyx");
        // the textbook example
        assert_eq!(encode(Method::Vigenere, "ATTACKATDAWN", "LEMON").unwrap(), "LXFOPVEFRNHR");
        assert_eq!(encode(Method::Rot47, "Hello", "").unwrap(), "w6==@");
        assert_eq!(encode(Method::Morse, "SOS help", "").unwrap(), "... --- ... / .... . .-.. .--.");
        assert_eq!(show(&[0xff, 0x00]), ("ff00".to_string(), false));
    }
}
