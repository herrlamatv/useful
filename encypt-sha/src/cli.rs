// esh.exe - encsha on the command line. `esh <command> --help` explains each command.
// Exit code 0 = ok, 1 = error, 2 = mismatch (hash differs, signature invalid, wrong password in pw verify).

use crate::crypt::{self, Cipher, Kdf, Lock, Unlock};
use crate::encode::{self, Method};
use crate::hash::{self, Algo, Class, Fmt};
use crate::keys::{self, KeyKind, PrivKey};
use crate::pw::{self, GenOpts, PwAlgo};
use crate::util::{b64, bytes_from_text, fmt_bytes, Progress, Res};
use crate::{identify, sums, util};
use clap::{Args, Parser, Subcommand};
use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use zeroize::Zeroizing;

const EXAMPLES: &str = "\
Examples:
  esh hash -a sha256,blake3 file.iso       hash files (-a all = every algorithm)
  esh hash -t \"hello\" -a md5                hash text
  esh check SHA256SUMS                     check a checksum file
  esh enc -i secret.pdf                    encrypt with a password -> secret.pdf.esh
  esh dec -i secret.pdf.esh                decrypt (cipher + settings come from the file)
  esh enc -t \"meet at 8\" -c chacha20       encrypt text -> Base64
  esh keygen x25519 -n bob                 bob.key.pem + bob.pub.pem
  esh enc -i x.zip --to bob.pub.pem        encrypt for bob (no password needed)
  esh keygen ed25519 -n me && esh sign -k me.key.pem file.exe && esh verify -k me.pub.pem file.exe
  esh pw gen -l 24 -c 5                    five random passwords
  esh pw hash -a argon2id                  hash a password for storing
  esh encode base64 \"hello\"   /   esh encode base64 -d aGVsbG8=
  esh id 5f4dcc3b5aa765d61d8327deb882cf99 -t password
  esh list                                 every algorithm";

#[derive(Parser)]
#[command(name = "esh", version, about = "Hash, encrypt, sign, encode - encsha on the command line", after_help = EXAMPLES)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Hash text or files (SHA-256 by default, -a all = every algorithm)
    Hash(HashArgs),
    /// Check a checksum file (SHA256SUMS, x.iso.sha256, *.md5, BSD style, .sfv)
    Check {
        sums: PathBuf,
    },
    /// Encrypt with a password (asked for) or for a public key (--to)
    Enc(EncArgs),
    /// Decrypt (cipher and settings are stored in the data)
    Dec(DecArgs),
    /// Create a key pair: ed25519 (sign), x25519 (encrypt), rsa (both), p256 (sign)
    Keygen(KeygenArgs),
    /// Sign a file or text with a private key
    Sign(SignArgs),
    /// Check a signature with a public key
    Verify(VerifyArgs),
    /// Password hashes (Argon2, bcrypt ...), generator, strength check
    #[command(subcommand)]
    Pw(PwCmd),
    /// Encode / decode: base64 base64url base32 base58 hex url binary caesar rot13 rot47 atbash vigenere xor morse
    Encode(EncodeArgs),
    /// What kind of hash is this? (-t tests a guess for the plaintext)
    Id {
        hash: String,
        #[arg(short, long)]
        text: Option<String>,
    },
    /// Random bytes, keys or UUIDs
    Rand(RandArgs),
    /// Every algorithm encsha knows
    List,
}

#[derive(Args)]
struct HashArgs {
    /// files (none: -t or stdin)
    files: Vec<PathBuf>,
    /// hash this text instead of files
    #[arg(short, long)]
    text: Option<String>,
    /// comma separated (sha256,md5,blake3 ...), "common" or "all"
    #[arg(short, long, default_value = "sha256")]
    algo: String,
    /// HMAC with this key instead of a plain hash
    #[arg(long)]
    hmac: Option<String>,
    /// compare with this hash (hex or Base64), exit code 2 when nothing matches
    #[arg(short, long)]
    compare: Option<String>,
    /// show Base64 instead of hex
    #[arg(long)]
    b64: bool,
    /// HEX IN CAPITALS
    #[arg(long)]
    upper: bool,
}

#[derive(Args)]
struct EncArgs {
    /// encrypt this text (output: Base64)
    #[arg(short, long)]
    text: Option<String>,
    /// encrypt this file
    #[arg(short, long)]
    input: Option<PathBuf>,
    /// output file (default: <input>.esh)
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// aes-256-gcm, chacha20-poly1305, xchacha20-poly1305, aes-256-gcm-siv, aes-256-cbc, twofish ... (esh list)
    #[arg(short, long, default_value = "aes-256-gcm")]
    cipher: String,
    /// encrypt for this public key (X25519 or RSA) instead of a password
    #[arg(long)]
    to: Option<PathBuf>,
    /// the password (better: leave it out and type it when asked - this lands in the shell history)
    #[arg(short, long)]
    password: Option<String>,
    /// Argon2id memory in MiB
    #[arg(long, default_value_t = 64)]
    kdf_mib: u32,
    /// overwrite the output file
    #[arg(short, long)]
    force: bool,
}

#[derive(Args)]
struct DecArgs {
    /// decrypt this Base64 text
    #[arg(short, long)]
    text: Option<String>,
    /// decrypt this file
    #[arg(short, long)]
    input: Option<PathBuf>,
    /// output file (default: <input> without .esh)
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// private key file (when it was encrypted for a key)
    #[arg(short, long)]
    key: Option<PathBuf>,
    #[arg(short, long)]
    password: Option<String>,
    #[arg(short, long)]
    force: bool,
}

#[derive(Args)]
struct KeygenArgs {
    /// ed25519 | x25519 | rsa | p256
    kind: String,
    /// RSA size
    #[arg(short, long, default_value_t = 4096)]
    bits: usize,
    /// file name without extension (default: the key type)
    #[arg(short, long)]
    name: Option<String>,
    /// folder for the two files
    #[arg(short, long, default_value = ".")]
    dir: PathBuf,
    /// lock the private key with a password (asked for)
    #[arg(long)]
    protect: bool,
    /// print both keys instead of writing files
    #[arg(long)]
    print: bool,
}

#[derive(Args)]
struct SignArgs {
    /// private key (.key.pem)
    #[arg(short, long)]
    key: PathBuf,
    /// file to sign
    file: Option<PathBuf>,
    /// sign this text instead (prints the signature)
    #[arg(short, long)]
    text: Option<String>,
    /// signature file (default: <file>.sig)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Args)]
struct VerifyArgs {
    /// public key (.pub.pem)
    #[arg(short, long)]
    key: PathBuf,
    /// signed file
    file: Option<PathBuf>,
    /// signed text
    #[arg(short, long)]
    text: Option<String>,
    /// signature: a .sig file or the Base64 / hex itself (default: <file>.sig)
    #[arg(short, long)]
    sig: Option<String>,
}

#[derive(Subcommand)]
enum PwCmd {
    /// Hash a password for storing: argon2id argon2i argon2d bcrypt scrypt pbkdf2 pbkdf2-sha512 sha512-crypt sha256-crypt md5-crypt
    Hash {
        #[arg(short, long, default_value = "argon2id")]
        algo: String,
        /// asked for when left out
        password: Option<String>,
    },
    /// Does a password belong to a stored hash? ($argon2id$..., $2b$..., $6$...)
    Verify {
        hash: String,
        password: Option<String>,
    },
    /// Random passwords
    Gen {
        #[arg(short, long, default_value_t = 20)]
        len: usize,
        #[arg(short, long, default_value_t = 1)]
        count: usize,
        #[arg(long)]
        no_symbols: bool,
        #[arg(long)]
        no_upper: bool,
        #[arg(long)]
        no_lower: bool,
        #[arg(long)]
        no_digits: bool,
        /// leave out I l 1 O 0 o
        #[arg(long)]
        easy: bool,
        /// pronounceable pass phrase with this many parts instead
        #[arg(long)]
        words: Option<usize>,
    },
    /// How strong is a password?
    Check {
        password: Option<String>,
    },
}

#[derive(Args)]
struct EncodeArgs {
    method: String,
    /// text (default: stdin)
    text: Option<String>,
    #[arg(short, long)]
    decode: bool,
    /// Caesar shift, Vigenère word or XOR key
    #[arg(short, long, default_value = "")]
    key: String,
}

#[derive(Args)]
struct RandArgs {
    /// number of bytes
    #[arg(short, long, default_value_t = 32)]
    n: usize,
    /// how many
    #[arg(short, long, default_value_t = 1)]
    count: usize,
    #[arg(long)]
    b64: bool,
    #[arg(long)]
    uuid: bool,
}

/// exit codes
enum Out {
    Ok,
    Mismatch,
}

pub fn main() -> ExitCode {
    let cli = Cli::parse();
    let res = match cli.cmd {
        Cmd::Hash(a) => cmd_hash(a),
        Cmd::Check { sums } => cmd_check(&sums),
        Cmd::Enc(a) => cmd_enc(a),
        Cmd::Dec(a) => cmd_dec(a),
        Cmd::Keygen(a) => cmd_keygen(a),
        Cmd::Sign(a) => cmd_sign(a),
        Cmd::Verify(a) => cmd_verify(a),
        Cmd::Pw(c) => cmd_pw(c),
        Cmd::Encode(a) => cmd_encode(a),
        Cmd::Id { hash, text } => cmd_id(&hash, text.as_deref()),
        Cmd::Rand(a) => cmd_rand(a),
        Cmd::List => cmd_list(),
    };
    match res {
        Ok(Out::Ok) => ExitCode::SUCCESS,
        Ok(Out::Mismatch) => ExitCode::from(2),
        Err(e) => {
            eprintln!("esh: {e}");
            ExitCode::FAILURE
        }
    }
}

// ---------------- helpers ----------------

/// progress line on stderr for long jobs (only on a console, only after half a second)
fn with_progress<T>(label: &str, work: impl FnOnce(&Progress) -> Res<T>) -> Res<T> {
    let p = Progress::default();
    let done = AtomicBool::new(false);
    std::thread::scope(|s| {
        if std::io::stderr().is_terminal() {
            s.spawn(|| {
                let start = Instant::now();
                let mut shown = false;
                while !done.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(100));
                    if start.elapsed() < Duration::from_millis(500) || done.load(Ordering::Relaxed) {
                        continue;
                    }
                    let d = p.done.load(Ordering::Relaxed);
                    let speed = d as f64 / start.elapsed().as_secs_f64();
                    let pct = p.fraction().map(|f| format!("{:3.0} %  ", f * 100.0)).unwrap_or_default();
                    eprint!("\r{label}  {pct}{}  {}/s     ", fmt_bytes(d), fmt_bytes(speed as u64));
                    shown = true;
                }
                if shown {
                    eprint!("\r{}\r", " ".repeat(70));
                }
            });
        }
        let r = work(&p);
        done.store(true, Ordering::Relaxed);
        r
    })
}

fn ask(prompt: &str) -> Res<Zeroizing<String>> {
    rpassword::prompt_password(prompt).map(Zeroizing::new).map_err(|e| format!("can't read the password: {e}"))
}

fn ask_new_password() -> Res<Zeroizing<String>> {
    let a = ask("Password: ")?;
    if a.is_empty() {
        return Err("empty password".into());
    }
    let s = pw::strength(&a);
    if s.level < 2 {
        eprintln!("note: weak password (~{:.0} bits, time to crack: {})", s.bits, pw::duration(s.crack_secs, false));
    }
    if *ask("Again:    ")? != *a {
        return Err("the passwords are different".into());
    }
    Ok(a)
}

fn password_or_ask(given: Option<String>, prompt: &str) -> Res<Zeroizing<String>> {
    match given {
        Some(p) => Ok(Zeroizing::new(p)),
        None => ask(prompt),
    }
}

/// all of stdin; a single trailing newline from `echo` is dropped
fn stdin_text() -> Res<String> {
    if std::io::stdin().is_terminal() {
        return Err("nothing to work on - give a text / file (see --help) or pipe something in".into());
    }
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).map_err(util::err)?;
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
    Ok(s)
}

fn load_private(path: &Path) -> Res<PrivKey> {
    let text = keys::read_key_file(path)?;
    if keys::is_protected(&text) {
        let pw = ask("Key password: ")?;
        keys::parse_private(&text, Some(&pw))
    } else {
        keys::parse_private(&text, None)
    }
}

fn parse_algos(list: &str) -> Res<Vec<Algo>> {
    match list.to_ascii_lowercase().as_str() {
        "all" => return Ok(Algo::ALL.to_vec()),
        "common" => return Ok(Algo::COMMON.to_vec()),
        _ => {}
    }
    list.split(',').filter(|s| !s.trim().is_empty()).map(|s| Algo::parse(s.trim()).ok_or(format!("unknown hash \"{s}\" (esh list)"))).collect()
}

// ---------------- commands ----------------

fn cmd_hash(a: HashArgs) -> Res<Out> {
    let algos = parse_algos(&a.algo)?;
    if algos.is_empty() {
        return Err("no algorithm".into());
    }
    let fmt = if a.b64 {
        Fmt::Base64
    } else if a.upper {
        Fmt::HexUpper
    } else {
        Fmt::Hex
    };
    let key = a.hmac.as_deref().map(str::as_bytes);
    if let Some(bad) = algos.iter().find(|x| key.is_some() && !x.hmac_ok()) {
        return Err(format!("{} has no HMAC", bad.name()));
    }
    if a.text.is_some() || a.files.is_empty() {
        let text = match a.text {
            Some(t) => t,
            None => stdin_text()?,
        };
        let digests = digest_all(&algos, key, text.as_bytes(), None)?;
        return print_hashes(&algos, &[(None, digests)], fmt, a.compare.as_deref());
    }
    let file_algos: Vec<Algo> = algos.iter().copied().filter(|x| !x.text_only()).collect();
    if file_algos.len() < algos.len() {
        eprintln!("note: NTLM is for passwords, skipped for files");
    }
    if file_algos.is_empty() {
        return Err("nothing left to hash files with".into());
    }
    // (file name, digests)
    let mut results = vec![];
    for f in &a.files {
        if f.is_dir() {
            eprintln!("esh: {} is a folder, skipped", f.display());
            continue;
        }
        let name = f.display().to_string();
        let digests = with_progress(&name, |p| {
            let file = std::fs::File::open(f).map_err(|e| format!("{name}: {e}"))?;
            p.grow(file.metadata().map(|m| m.len()).unwrap_or(0));
            digest_all(&file_algos, key, file, Some(p))
        })?;
        results.push((Some(name), digests));
    }
    print_hashes(&file_algos, &results, fmt, a.compare.as_deref())
}

/// plain hashes, or HMACs when there is a key (every algorithm was checked to have one)
fn digest_all(algos: &[Algo], key: Option<&[u8]>, r: impl Read + Send, p: Option<&Progress>) -> Res<Vec<Vec<u8>>> {
    match key {
        Some(k) => Ok(hash::hmac_reader(algos, k, r, p)?.into_iter().map(|d| d.expect("hmac_ok")).collect()),
        None => hash::hash_reader(algos, r, p),
    }
}

fn print_hashes(algos: &[Algo], results: &[(Option<String>, Vec<Vec<u8>>)], fmt: Fmt, compare: Option<&str>) -> Res<Out> {
    let width = algos.iter().map(|a| a.name().len()).max().unwrap_or(0);
    let mut matched = vec![];
    for (name, digests) in results {
        if algos.len() == 1 {
            // sha256sum format, so `esh hash *.iso > SHA256SUMS` works with `esh check` / `sha256sum -c`
            let d = hash::show(&digests[0], fmt);
            match name {
                Some(n) => println!("{d}  {n}"),
                None => println!("{d}"),
            }
        } else {
            if let Some(n) = name {
                println!("{n}");
            }
            for (a, d) in algos.iter().zip(digests) {
                let indent = if name.is_some() { "  " } else { "" };
                println!("{indent}{:width$}  {}", a.name(), hash::show(d, fmt));
            }
        }
        if let Some(c) = compare {
            for (a, d) in algos.iter().zip(digests) {
                if hash::matches(d, c) {
                    matched.push(format!("{}{}", a.name(), name.as_ref().map(|n| format!(" of {n}")).unwrap_or_default()));
                }
            }
        }
    }
    if compare.is_some() {
        if matched.is_empty() {
            eprintln!("no match");
            return Ok(Out::Mismatch);
        }
        eprintln!("match: {}", matched.join(", "));
    }
    Ok(Out::Ok)
}

fn cmd_check(path: &Path) -> Res<Out> {
    let entries = sums::parse(path)?;
    let outcomes = with_progress("checking", |p| Ok(sums::check(&entries, Some(p))))?;
    let mut bad = 0;
    for (e, o) in entries.iter().zip(&outcomes) {
        match o {
            sums::Outcome::Ok => println!("{}: OK ({})", e.name, e.algo.name()),
            sums::Outcome::Mismatch => {
                bad += 1;
                println!("{}: FAILED", e.name);
            }
            sums::Outcome::Error(err) => {
                bad += 1;
                println!("{}: {err}", e.name);
            }
        }
    }
    if bad > 0 {
        eprintln!("{bad} of {} files did NOT match", entries.len());
        return Ok(Out::Mismatch);
    }
    eprintln!("all {} files match", entries.len());
    Ok(Out::Ok)
}

fn cmd_enc(a: EncArgs) -> Res<Out> {
    let cipher = Cipher::parse(&a.cipher).ok_or(format!("unknown cipher \"{}\" (esh list)", a.cipher))?;
    if let Some(w) = cipher.warning() {
        eprintln!("note: {} is {w}", cipher.name());
    }
    let recipient = a.to.as_deref().map(|p| keys::parse_public(&keys::read_key_file(p)?)).transpose()?;
    let password = match &recipient {
        Some(_) => None,
        None => Some(match a.password {
            Some(p) => Zeroizing::new(p),
            None => ask_new_password()?,
        }),
    };
    let kdf = Kdf { m_kib: a.kdf_mib.clamp(8, 4096) * 1024, ..Kdf::default() };
    let lock = match (&recipient, &password) {
        (Some(k), _) => k.lock()?,
        (None, Some(p)) => Lock::Password(p.as_bytes(), kdf),
        _ => unreachable!(),
    };
    match (a.text, a.input) {
        (Some(_), Some(_)) => Err("either -t TEXT or -i FILE".into()),
        (Some(t), None) => {
            println!("{}", crypt::encrypt_text(cipher, &lock, &t)?);
            Ok(Out::Ok)
        }
        (None, Some(src)) => {
            let dst = a.output.unwrap_or_else(|| crypt::default_out(&src, true));
            with_progress("encrypting", |p| crypt::encrypt_file(cipher, &lock, &src, &dst, a.force, Some(p)))?;
            eprintln!("{} -> {} ({})", src.display(), dst.display(), cipher.name());
            Ok(Out::Ok)
        }
        (None, None) => {
            println!("{}", crypt::encrypt_text(cipher, &lock, &stdin_text()?)?);
            Ok(Out::Ok)
        }
    }
}

fn cmd_dec(a: DecArgs) -> Res<Out> {
    let info = match (&a.text, &a.input) {
        (Some(_), Some(_)) => return Err("either -t TEXT or -i FILE".into()),
        (Some(t), None) => crypt::peek_text(t)?,
        (None, Some(f)) => crypt::peek_file(f)?,
        (None, None) => return Err("give -t BASE64 or -i FILE".into()),
    };
    let key = match info.lock {
        crypt::LockKind::Password => None,
        _ => Some(load_private(a.key.as_deref().ok_or(format!("this was encrypted for an {} - give it with -k", info.lock.name()))?)?),
    };
    let password = match info.lock {
        crypt::LockKind::Password => Some(password_or_ask(a.password, "Password: ")?),
        _ => None,
    };
    let un = Unlock { password: password.as_ref().map(|p| p.as_bytes()), key: key.as_ref() };
    if let Some(t) = a.text {
        let (plain, _) = crypt::decrypt_text(&un, &t)?;
        if let Some(out) = a.output {
            if out.exists() && !a.force {
                return Err(format!("{} already exists (-f overwrites)", out.display()));
            }
            std::fs::write(&out, &*plain).map_err(util::err)?;
        } else {
            let text = std::str::from_utf8(&plain).map_err(|_| "the result isn't text - write it to a file with -o".to_string())?;
            println!("{text}");
        }
        return Ok(Out::Ok);
    }
    let src = a.input.unwrap();
    let dst = a.output.unwrap_or_else(|| crypt::default_out(&src, false));
    let info = with_progress("decrypting", |p| crypt::decrypt_file(&un, &src, &dst, a.force, Some(p)))?;
    eprintln!("{} -> {} ({}, {})", src.display(), dst.display(), info.cipher.name(), info.lock.name());
    Ok(Out::Ok)
}

fn cmd_keygen(a: KeygenArgs) -> Res<Out> {
    let kind = KeyKind::parse(&a.kind, a.bits).ok_or("key type: ed25519, x25519, rsa or p256")?;
    if matches!(kind, KeyKind::Rsa(_)) {
        eprintln!("making a {}-bit RSA key, this takes a moment ...", a.bits);
    }
    let key = keys::generate(kind)?;
    let password = if a.protect { Some(ask_new_password()?) } else { None };
    let fp = key.public().fingerprint();
    if a.print {
        print!("{}", *key.to_pem(password.as_deref().map(|s| s.as_str()))?);
        print!("{}", key.public().to_pem()?);
    } else {
        let name = a.name.unwrap_or_else(|| kind.name().to_ascii_lowercase().replace(' ', "-"));
        let (private, public) = keys::write_pair(&a.dir, &name, &key, password.as_deref().map(|s| s.as_str()))?;
        println!("private key: {}  (keep it secret)", private.display());
        println!("public key:  {}  (give this one away)", public.display());
    }
    eprintln!("{} · fingerprint {fp}", kind.name());
    eprintln!(
        "{}",
        match kind {
            KeyKind::X25519 => "for: esh enc --to <public key>",
            KeyKind::Rsa(_) => "for: esh sign / verify and esh enc --to <public key>",
            _ => "for: esh sign / esh verify",
        }
    );
    Ok(Out::Ok)
}

fn cmd_sign(a: SignArgs) -> Res<Out> {
    let key = load_private(&a.key)?;
    match (a.text, a.file) {
        (Some(t), None) => {
            println!("{}", b64(&keys::sign(&key, t.as_bytes(), None)?));
            Ok(Out::Ok)
        }
        (None, Some(f)) => {
            let file = std::fs::File::open(&f).map_err(|e| format!("{}: {e}", f.display()))?;
            let size = file.metadata().map(|m| m.len()).unwrap_or(0);
            let sig = with_progress("signing", |p| {
                p.grow(size);
                keys::sign(&key, file, Some(p))
            })?;
            let out = a.output.unwrap_or_else(|| PathBuf::from(format!("{}.sig", f.display())));
            std::fs::write(&out, format!("{}\n", b64(&sig))).map_err(util::err)?;
            eprintln!("{} signature -> {}", key.kind().sig_name(), out.display());
            Ok(Out::Ok)
        }
        _ => Err("give a file or -t TEXT".into()),
    }
}

fn cmd_verify(a: VerifyArgs) -> Res<Out> {
    let key = keys::parse_public(&keys::read_key_file(&a.key)?)?;
    let sig_text = match (&a.sig, &a.file) {
        (Some(s), _) if Path::new(s).is_file() => std::fs::read_to_string(s).map_err(util::err)?,
        (Some(s), _) => s.clone(),
        (None, Some(f)) => {
            let p = format!("{}.sig", f.display());
            std::fs::read_to_string(&p).map_err(|e| format!("{p}: {e} (give the signature with -s)"))?
        }
        (None, None) => return Err("give the signature with -s".into()),
    };
    let sig = bytes_from_text(sig_text.trim())?;
    let ok = match (a.text, a.file) {
        (Some(t), None) => keys::verify(&key, t.as_bytes(), &sig, None)?,
        (None, Some(f)) => {
            let file = std::fs::File::open(&f).map_err(|e| format!("{}: {e}", f.display()))?;
            let size = file.metadata().map(|m| m.len()).unwrap_or(0);
            with_progress("verifying", |p| {
                p.grow(size);
                keys::verify(&key, file, &sig, Some(p))
            })?
        }
        _ => return Err("give a file or -t TEXT".into()),
    };
    if ok {
        println!("valid signature ({}, key {})", key.kind().sig_name(), key.fingerprint());
        Ok(Out::Ok)
    } else {
        println!("INVALID signature - the data or the signature was changed, or it's a different key");
        Ok(Out::Mismatch)
    }
}

fn cmd_pw(c: PwCmd) -> Res<Out> {
    match c {
        PwCmd::Hash { algo, password } => {
            let a = PwAlgo::parse(&algo).ok_or(format!("unknown password hash \"{algo}\" (esh list)"))?;
            let pw = match password {
                Some(p) => Zeroizing::new(p),
                None => ask_new_password()?,
            };
            println!("{}", pw::hash(a, &pw)?);
            Ok(Out::Ok)
        }
        PwCmd::Verify { hash, password } => {
            let pw = password_or_ask(password, "Password: ")?;
            if pw::verify(&hash, &pw)? {
                println!("correct password ({})", pw::detect(&hash).map(|a| a.name()).unwrap_or("?"));
                Ok(Out::Ok)
            } else {
                println!("wrong password");
                Ok(Out::Mismatch)
            }
        }
        PwCmd::Gen { len, count, no_symbols, no_upper, no_lower, no_digits, easy, words } => {
            if let Some(w) = words {
                for _ in 0..count.max(1) {
                    println!("{}", pw::passphrase(w));
                }
                eprintln!("~{:.0} bits each", pw::passphrase_bits(w));
                return Ok(Out::Ok);
            }
            let o = GenOpts { len, lower: !no_lower, upper: !no_upper, digits: !no_digits, symbols: !no_symbols, no_ambiguous: easy };
            for _ in 0..count.max(1) {
                println!("{}", pw::generate(&o)?);
            }
            eprintln!("~{:.0} bits each", pw::gen_bits(&o));
            Ok(Out::Ok)
        }
        PwCmd::Check { password } => {
            let pw = password_or_ask(password, "Password: ")?;
            let s = pw::strength(&pw);
            let label = ["very weak", "weak", "okay", "strong", "very strong"][s.level as usize];
            println!("{label} · ~{:.0} bits · time to crack at 10 billion guesses/s: {}", s.bits, pw::duration(s.crack_secs, false));
            Ok(Out::Ok)
        }
    }
}

fn cmd_encode(a: EncodeArgs) -> Res<Out> {
    let m = Method::parse(&a.method).ok_or(format!("unknown method \"{}\" (esh list)", a.method))?;
    let input = match a.text {
        Some(t) => t,
        None => stdin_text()?,
    };
    if a.decode {
        let (text, is_text) = encode::show(&encode::decode(m, &input, &a.key)?);
        if !is_text {
            eprintln!("note: the result isn't text, shown as hex");
        }
        println!("{text}");
    } else {
        println!("{}", encode::encode(m, &input, &a.key)?);
    }
    Ok(Out::Ok)
}

fn cmd_id(hash: &str, text: Option<&str>) -> Res<Out> {
    let guesses = identify::identify(hash);
    if guesses.is_empty() {
        println!("unknown - not a hash shape I know");
    }
    for g in &guesses {
        println!("{}{}", g.name, if g.likely { "  (most likely)" } else { "" });
    }
    if let Some(t) = text {
        let found = identify::find_match(hash, t);
        if found.is_empty() {
            println!("\n\"{t}\" doesn't produce this hash with any algorithm here");
            return Ok(Out::Mismatch);
        }
        println!("\nconfirmed: {} of \"{t}\"", found.join(", "));
    }
    Ok(Out::Ok)
}

fn cmd_rand(a: RandArgs) -> Res<Out> {
    for _ in 0..a.count.max(1) {
        if a.uuid {
            println!("{}", util::uuid_v4());
        } else if a.b64 {
            println!("{}", b64(&util::random(a.n)));
        } else {
            println!("{}", hex::encode(util::random(a.n)));
        }
    }
    Ok(Out::Ok)
}

fn cmd_list() -> Res<Out> {
    println!("Hashes (esh hash -a ...):");
    for c in [Class::Secure, Class::Broken, Class::Checksum] {
        let names: Vec<&str> = Algo::ALL.iter().filter(|a| a.class() == c).map(|a| a.name()).collect();
        let label = match c {
            Class::Secure => "secure",
            Class::Broken => "broken (only for checksums)",
            Class::Checksum => "checksums (not cryptographic)",
        };
        println!("  {label}: {}", names.join(", "));
    }
    println!("\nCiphers (esh enc -c ...):");
    for c in Cipher::ALL {
        let kind = if c.aead() { "authenticated" } else { "+ HMAC-SHA256" };
        println!("  {:20} {}-bit key, {kind}{}", c.name(), c.key_len() * 8, c.warning().map(|w| format!(" - {w}")).unwrap_or_default());
    }
    println!("\nPassword hashes (esh pw hash -a ...):");
    for a in PwAlgo::ALL {
        println!("  {:15} {}", a.name(), a.settings());
    }
    println!("\nKey pairs (esh keygen ...):");
    for k in [KeyKind::Ed25519, KeyKind::X25519, KeyKind::Rsa(4096), KeyKind::P256] {
        let what = match (k.can_sign(), k.can_encrypt()) {
            (true, true) => "sign + encrypt",
            (true, false) => "sign",
            _ => "encrypt",
        };
        println!("  {:12} {what}{}", k.name().replace("-4096", ""), if k.can_sign() { format!(" ({})", k.sig_name()) } else { String::new() });
    }
    println!("\nEncodings + classic ciphers (esh encode ...):");
    println!("  {}", Method::ALL.iter().map(|m| m.name()).collect::<Vec<_>>().join(", "));
    Ok(Out::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn algo_lists() {
        assert_eq!(parse_algos("sha256, md5").unwrap(), vec![Algo::Sha256, Algo::Md5]);
        assert_eq!(parse_algos("all").unwrap().len(), Algo::ALL.len());
        assert!(parse_algos("sha256,nope").is_err());
    }

    #[test]
    fn cli_definition_is_valid() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
