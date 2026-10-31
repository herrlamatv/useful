// Checksum files (SHA256SUMS, file.iso.sha256, *.md5 ...): read them and check every listed file,
// or write one. Understands the GNU form `<hex>  name` / `<hex> *name` and the BSD form `SHA256 (name) = <hex>`.

use crate::hash::{hash_file, Algo};
use crate::util::{hex_decode, is_hex, Progress, Res};
use rayon::prelude::*;
use std::path::{Path, PathBuf};

pub struct Entry {
    pub path: PathBuf,
    /// as written in the sums file
    pub name: String,
    pub algo: Algo,
    pub expected: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Ok,
    Mismatch,
    Error(String),
}

/// algorithm from the file name (`x.sha256`, `SHA512SUMS`) or else from the digest length
fn algo_for(sums: &Path, digest_len: usize) -> Option<Algo> {
    let name = sums.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    let by_name = [
        ("sha512", Algo::Sha512),
        ("sha384", Algo::Sha384),
        ("sha256", Algo::Sha256),
        ("sha224", Algo::Sha224),
        ("sha1", Algo::Sha1),
        ("md5", Algo::Md5),
        ("b3", Algo::Blake3),
        ("blake3", Algo::Blake3),
        ("b2", Algo::Blake2b),
        ("crc32", Algo::Crc32),
        ("sfv", Algo::Crc32),
    ];
    if let Some((_, a)) = by_name.iter().find(|(k, _)| name.contains(k)) {
        if a.out_len() == digest_len {
            return Some(*a);
        }
    }
    Some(match digest_len {
        4 => Algo::Crc32,
        16 => Algo::Md5,
        20 => Algo::Sha1,
        28 => Algo::Sha224,
        32 => Algo::Sha256,
        48 => Algo::Sha384,
        64 => Algo::Sha512,
        _ => return None,
    })
}

pub fn parse(sums: &Path) -> Res<Vec<Entry>> {
    let text = std::fs::read_to_string(sums).map_err(|e| format!("{}: {e}", sums.display()))?;
    let dir = sums.parent().unwrap_or(Path::new("."));
    let mut out = vec![];
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with(';')) {
        // BSD: SHA256 (name) = hex
        if let (Some(open), Some(eq)) = (line.find(" ("), line.rfind(") = ")) {
            let algo = Algo::parse(&line[..open]).ok_or(format!("unknown algorithm in: {line}"))?;
            let name = line[open + 2..eq].to_string();
            out.push(Entry { path: dir.join(&name), name, algo, expected: hex_decode(&line[eq + 4..])? });
            continue;
        }
        // GNU: hex  name / hex *name
        let (digest, name) = match line.split_once(char::is_whitespace) {
            Some((d, n)) => (d, n.trim_start().trim_start_matches('*').to_string()),
            // only a digest: the sums file belongs to the file with the same name minus the extension
            None => (line, sums.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()),
        };
        if !is_hex(digest) {
            // SFV: name crc32
            match line.rsplit_once(char::is_whitespace) {
                Some((n, d)) if is_hex(d) && d.len() == 8 => {
                    out.push(Entry { path: dir.join(n.trim()), name: n.trim().to_string(), algo: Algo::Crc32, expected: hex_decode(d)? });
                    continue;
                }
                _ => return Err(format!("can't read this line: {line}")),
            }
        }
        let expected = hex_decode(digest)?;
        let algo = algo_for(sums, expected.len()).ok_or(format!("no known hash is {} bytes long", expected.len()))?;
        out.push(Entry { path: dir.join(&name), name, algo, expected });
    }
    if out.is_empty() {
        return Err("no checksums found in the file".into());
    }
    Ok(out)
}

/// several files at once (fast on SSDs)
pub fn check(entries: &[Entry], prog: Option<&Progress>) -> Vec<Outcome> {
    entries
        .par_iter()
        .map(|e| match hash_file(&[e.algo], &e.path, prog) {
            Ok(d) if d[0] == e.expected => Outcome::Ok,
            Ok(_) => Outcome::Mismatch,
            Err(err) if err == crate::util::CANCELLED => Outcome::Error(err),
            Err(err) => Outcome::Error(err.rsplit(": ").next().unwrap_or(&err).to_string()),
        })
        .collect()
}

/// GNU format, readable by `sha256sum -c` and by parse()
pub fn format_line(digest: &[u8], name: &str) -> String {
    format!("{}  {}", hex::encode(digest), name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gnu_bsd_and_single() {
        let dir = std::env::temp_dir().join(format!("encsha-sums-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), "abc").unwrap();
        std::fs::write(dir.join("b.txt"), "abd").unwrap();
        let sha = |s: &str| hex::encode(crate::hash::hash_one(Algo::Sha256, s.as_bytes()));
        let sums = dir.join("SHA256SUMS");
        std::fs::write(&sums, format!("# comment\n{}  a.txt\n{} *b.txt\n{}  missing.txt\n", sha("abc"), sha("abc"), sha("x"))).unwrap();
        let e = parse(&sums).unwrap();
        assert_eq!(e.len(), 3);
        let r = check(&e, None);
        assert_eq!(r[0], Outcome::Ok);
        assert_eq!(r[1], Outcome::Mismatch);
        assert!(matches!(r[2], Outcome::Error(_)));

        let bsd = dir.join("list.txt");
        std::fs::write(&bsd, format!("MD5 (a.txt) = 900150983cd24fb0d6963f7d28e17f72\nSHA256 (b.txt) = {}\n", sha("abd"))).unwrap();
        assert!(check(&parse(&bsd).unwrap(), None).iter().all(|o| *o == Outcome::Ok));

        let single = dir.join("a.txt.sha256");
        std::fs::write(&single, sha("abc")).unwrap();
        let e = parse(&single).unwrap();
        assert_eq!(e[0].name, "a.txt");
        assert_eq!(check(&e, None), vec![Outcome::Ok]);

        let sfv = dir.join("x.sfv");
        std::fs::write(&sfv, "; made by hand\na.txt 352441c2\n").unwrap();
        assert_eq!(check(&parse(&sfv).unwrap(), None), vec![Outcome::Ok]);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
