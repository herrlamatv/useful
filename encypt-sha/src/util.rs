// Small helpers everything uses: progress + cancel, reading full buffers, randomness, base64 / hex input.

use base64::engine::general_purpose::{GeneralPurpose, GeneralPurposeConfig};
use base64::engine::DecodePaddingMode;
use base64::{alphabet, Engine};
use rand::rngs::OsRng;
use rand::RngCore;
use std::io::{self, Read};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

pub type Res<T> = Result<T, String>;

pub const CANCELLED: &str = "cancelled";

pub fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// shared between a worker thread and the window / console
#[derive(Default)]
pub struct Progress {
    pub done: AtomicU64,
    pub total: AtomicU64,
    pub cancel: AtomicBool,
}

impl Progress {
    /// count bytes, stop with an error once cancel was pressed
    pub fn add(&self, n: usize) -> Res<()> {
        self.done.fetch_add(n as u64, Ordering::Relaxed);
        if self.cancel.load(Ordering::Relaxed) {
            return Err(CANCELLED.into());
        }
        Ok(())
    }

    pub fn grow(&self, n: u64) {
        self.total.fetch_add(n, Ordering::Relaxed);
    }

    pub fn fraction(&self) -> Option<f32> {
        let total = self.total.load(Ordering::Relaxed);
        (total > 0).then(|| self.done.load(Ordering::Relaxed) as f32 / total as f32)
    }
}

pub fn tick(p: Option<&Progress>, n: usize) -> Res<()> {
    p.map_or(Ok(()), |p| p.add(n))
}

/// read until `buf` is full or the input ends (a plain read() may return less)
pub fn read_full(r: &mut impl Read, buf: &mut [u8]) -> io::Result<usize> {
    let mut n = 0;
    while n < buf.len() {
        match r.read(&mut buf[n..]) {
            Ok(0) => break,
            Ok(k) => n += k,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(n)
}

/// bytes from the operating system's secure random generator
pub fn random(n: usize) -> Vec<u8> {
    let mut v = vec![0u8; n];
    OsRng.fill_bytes(&mut v);
    v
}

pub fn uuid_v4() -> String {
    let mut b = random(16);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h = hex::encode(b);
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

pub fn b64(data: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(data)
}

/// base64 as people paste it: with or without padding, url-safe or not, with line breaks
pub fn b64_decode(text: &str) -> Res<Vec<u8>> {
    let clean: String = text
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| match c {
            '-' => '+',
            '_' => '/',
            c => c,
        })
        .collect();
    let engine = GeneralPurpose::new(&alphabet::STANDARD, GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent));
    engine.decode(clean.trim_end_matches('=')).map_err(|e| format!("not valid Base64: {e}"))
}

/// hex with spaces, colons or a 0x in front
pub fn hex_decode(text: &str) -> Res<Vec<u8>> {
    let t = text.trim();
    let t = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")).unwrap_or(t);
    let clean: String = t.chars().filter(|c| !c.is_whitespace() && *c != ':').collect();
    hex::decode(clean).map_err(|e| format!("not valid hex: {e}"))
}

pub fn is_hex(text: &str) -> bool {
    let t = text.trim();
    !t.is_empty() && t.len().is_multiple_of(2) && t.chars().all(|c| c.is_ascii_hexdigit())
}

/// hex if it looks like hex, otherwise base64
pub fn bytes_from_text(text: &str) -> Res<Vec<u8>> {
    if is_hex(text) {
        hex_decode(text)
    } else {
        b64_decode(text)
    }
}

pub fn fmt_bytes(n: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < units.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{n} B")
    } else {
        format!("{v:.1} {}", units[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_lenient() {
        assert_eq!(b64_decode("aGVsbG8=").unwrap(), b"hello");
        assert_eq!(b64_decode("aGVsbG8").unwrap(), b"hello");
        assert_eq!(b64_decode("aGVs\r\nbG8=").unwrap(), b"hello");
        assert_eq!(b64_decode("-_8").unwrap(), b64_decode("+/8=").unwrap());
        assert!(b64_decode("!!").is_err());
    }

    #[test]
    fn hex_lenient() {
        assert_eq!(hex_decode("0xDE:AD be ef").unwrap(), vec![0xde, 0xad, 0xbe, 0xef]);
        assert!(is_hex("00ff") && !is_hex("0ff") && !is_hex("zz"));
    }

    #[test]
    fn uuid_format() {
        let u = uuid_v4();
        assert_eq!(u.len(), 36);
        assert_eq!(&u[14..15], "4");
        assert!("89ab".contains(&u[19..20]));
    }
}
