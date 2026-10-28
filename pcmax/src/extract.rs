// Turbo unpacker. Explorer unpacks with one thread, one file after the other - with thousands of
// small files most of the time goes into creating files (NTFS + Defender), not into decompressing.
// Here all cores create files at the same time:
//   zip       every entry on its own thread (archive memory mapped, no seeking between threads)
//   7z        every block on its own thread (+ multi threaded LZMA2 inside a block)
//   tar.*     one thread decompresses, all others write the files
//   rar       the installed WinRAR / UnRAR with -mt and boosted priority
// Defender can skip the target folder while unpacking (needs admin) and scans it right after.

use crate::fsx::{self, safe_join, DirCache, Progress, Summary};
use crate::profile::{self, Profile};
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::mpsc::sync_channel;
use std::time::{Instant, SystemTime};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Comp {
    None,
    Gz,
    Zst,
    Xz,
    Bz2,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Zip,
    SevenZ,
    Rar,
    Tar(Comp),
    /// one compressed file, no archive
    Single(Comp),
}

#[derive(Clone, Debug)]
pub struct Options {
    pub threads: usize,
    pub overwrite: bool,
    /// temporary Defender exclusion for the target folder (admin)
    pub defender_skip: bool,
    /// Defender scan of the target folder afterwards
    pub scan_after: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options { threads: fsx::default_threads(), overwrite: false, defender_skip: false, scan_after: false }
    }
}

/// all endings the context menu is registered for
pub const EXTENSIONS: [&str; 13] =
    [".zip", ".7z", ".rar", ".tar", ".tgz", ".gz", ".tzst", ".zst", ".txz", ".xz", ".tbz2", ".bz2", ".jar"];

pub fn kind_of(path: &Path) -> Option<Kind> {
    let n = path.file_name()?.to_string_lossy().to_ascii_lowercase();
    let by_name = if n.ends_with(".tar.gz") || n.ends_with(".tgz") {
        Some(Kind::Tar(Comp::Gz))
    } else if n.ends_with(".tar.zst") || n.ends_with(".tzst") {
        Some(Kind::Tar(Comp::Zst))
    } else if n.ends_with(".tar.xz") || n.ends_with(".txz") {
        Some(Kind::Tar(Comp::Xz))
    } else if n.ends_with(".tar.bz2") || n.ends_with(".tbz2") || n.ends_with(".tbz") {
        Some(Kind::Tar(Comp::Bz2))
    } else if n.ends_with(".tar") {
        Some(Kind::Tar(Comp::None))
    } else if n.ends_with(".zip") || n.ends_with(".jar") || n.ends_with(".nupkg") || n.ends_with(".vsix") {
        Some(Kind::Zip)
    } else if n.ends_with(".7z") {
        Some(Kind::SevenZ)
    } else if n.ends_with(".rar") {
        Some(Kind::Rar)
    } else if n.ends_with(".gz") {
        Some(Kind::Single(Comp::Gz))
    } else if n.ends_with(".zst") {
        Some(Kind::Single(Comp::Zst))
    } else if n.ends_with(".xz") {
        Some(Kind::Single(Comp::Xz))
    } else if n.ends_with(".bz2") {
        Some(Kind::Single(Comp::Bz2))
    } else {
        None
    };
    by_name.or_else(|| sniff(path))
}

/// unknown ending: look at the first bytes
fn sniff(path: &Path) -> Option<Kind> {
    let mut head = [0u8; 8];
    File::open(path).ok()?.read_exact(&mut head).ok()?;
    Some(match head {
        [b'P', b'K', 3, 4, ..] => Kind::Zip,
        [b'7', b'z', 0xBC, 0xAF, 0x27, 0x1C, ..] => Kind::SevenZ,
        [b'R', b'a', b'r', b'!', ..] => Kind::Rar,
        [0x1f, 0x8b, ..] => Kind::Single(Comp::Gz),
        [0x28, 0xb5, 0x2f, 0xfd, ..] => Kind::Single(Comp::Zst),
        [0xfd, b'7', b'z', b'X', b'Z', 0, ..] => Kind::Single(Comp::Xz),
        [b'B', b'Z', b'h', ..] => Kind::Single(Comp::Bz2),
        _ => return None,
    })
}

/// "C:\dl\project.tar.gz" -> "C:\dl\project" (archives) or "C:\dl" (single files)
pub fn default_dest(archive: &Path) -> PathBuf {
    let parent = archive.parent().map(Path::to_path_buf).unwrap_or_default();
    if let Some(Kind::Single(_)) = kind_of(archive) {
        return parent;
    }
    let name = archive.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let lower = name.to_ascii_lowercase();
    let mut stem = name.as_str();
    for ext in [".tar.gz", ".tar.zst", ".tar.xz", ".tar.bz2", ".tgz", ".tzst", ".txz", ".tbz2", ".tar", ".zip", ".7z", ".rar", ".jar"] {
        if lower.ends_with(ext) {
            stem = &name[..name.len() - ext.len()];
            break;
        }
    }
    if stem.is_empty() || stem == name {
        stem = archive.file_stem().and_then(|s| s.to_str()).unwrap_or("extracted");
    }
    parent.join(stem)
}

pub fn extract(archive: &Path, dest: &Path, opts: &Options, prog: &Progress) -> Result<Summary, String> {
    let kind = kind_of(archive).ok_or_else(|| format!("{}: unknown archive type", archive.display()))?;
    std::fs::create_dir_all(dest).map_err(|e| format!("{}: {e}", dest.display()))?;
    let dest = dest.canonicalize().map(strip_verbatim).unwrap_or_else(|_| dest.to_path_buf());
    let mut notes = vec![];
    let excluded = if opts.defender_skip {
        prog.set_phase("Defender exclusion ...");
        crate::defender::exclude(&dest)?;
        notes.push("Defender skipped the folder while unpacking".to_string());
        true
    } else {
        false
    };

    let t0 = Instant::now();
    let result = match kind {
        Kind::Zip => zip(archive, &dest, opts, prog),
        Kind::SevenZ => seven_z(archive, &dest, opts, prog),
        Kind::Rar => rar(archive, &dest, opts, prog),
        Kind::Tar(c) => tar(archive, c, &dest, opts, prog),
        Kind::Single(c) => single(archive, c, &dest, opts, prog),
    };
    let secs = t0.elapsed().as_secs_f64();

    // the hole is closed no matter what happened
    if excluded {
        if let Err(e) = crate::defender::unexclude(&dest.to_string_lossy()) {
            notes.push(e);
        }
    }
    if opts.scan_after && !prog.cancelled() {
        prog.set_phase("Defender scan ...");
        match crate::defender::scan(&dest) {
            Ok(s) => notes.push(s),
            Err(e) => notes.push(format!("⚠ {e}")),
        }
    }
    result?;
    let skipped = prog.skipped.load(Ordering::Relaxed);
    if skipped > 0 {
        notes.push(format!("{skipped} files already existed and were kept (overwrite is off)"));
    }
    Ok(Summary {
        files: prog.done_files.load(Ordering::Relaxed),
        bytes: prog.done_bytes.load(Ordering::Relaxed),
        secs,
        dest,
        notes,
        errors: prog.errors(),
    })
}

/// "\\?\C:\x" -> "C:\x" (canonicalize adds it, other programs don't like it)
fn strip_verbatim(p: PathBuf) -> PathBuf {
    let s = p.to_string_lossy();
    match s.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC") => PathBuf::from(rest),
        _ => p,
    }
}

// ---------------- zip ----------------

struct ZipItem {
    index: usize,
    path: PathBuf,
    size: u64,
    mtime: Option<SystemTime>,
}

fn zip(archive: &Path, dest: &Path, opts: &Options, prog: &Progress) -> Result<(), String> {
    prog.set_phase("reading the zip directory ...");
    let file = File::open(archive).map_err(|e| e.to_string())?;
    let map = fsx::Mmap::open(&file)?;
    let mut za = zip::ZipArchive::new(Cursor::new(map.as_slice())).map_err(|e| format!("zip: {e}"))?;

    let mut dirs = vec![];
    let mut files = vec![];
    for i in 0..za.len() {
        let e = za.by_index_raw(i).map_err(|e| format!("zip: {e}"))?;
        let Some(rel) = e.enclosed_name() else {
            prog.error(format!("skipped unsafe path: {}", e.name().unwrap_or_default()));
            continue;
        };
        let path = dest.join(rel);
        if e.is_dir() {
            dirs.push(path);
            continue;
        }
        if e.encrypted() {
            return Err("this zip is password protected - not supported, use 7-Zip / WinRAR".into());
        }
        let mtime = e.last_modified().and_then(|d| fsx::local_to_system(d.year(), d.month(), d.day(), d.hour(), d.minute(), d.second()));
        files.push(ZipItem { index: i, path, size: e.size(), mtime });
    }
    prog.total_files.store(files.len() as u64, Ordering::Relaxed);
    prog.total_bytes.store(files.iter().map(|f| f.size).sum(), Ordering::Relaxed);

    // folders first (one thread, they depend on each other), then all files in parallel
    prog.set_phase("creating folders ...");
    let cache = DirCache::default();
    for d in &dirs {
        cache.ensure(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    for f in &files {
        let _ = cache.ensure_parent(&f.path);
    }
    // big files first, so no thread ends up alone with a huge one at the end
    files.sort_by_key(|f| std::cmp::Reverse(f.size));

    prog.set_phase("unpacking ...");
    fsx::pool(opts.threads).install(|| {
        files.par_iter().for_each_init(
            || (za.clone(), Vec::new()),
            |(za, buf), f| {
                if prog.cancelled() {
                    return;
                }
                let r = match za.by_index(f.index) {
                    Ok(mut entry) => fsx::write_stream(&f.path, &mut entry, f.size, f.mtime, opts.overwrite, buf, prog).map(|_| ()),
                    Err(e) => Err(format!("{}: {e}", f.path.display())),
                };
                if let Err(e) = r {
                    prog.error(e);
                }
            },
        )
    });
    if prog.cancelled() {
        return Err("cancelled".into());
    }
    Ok(())
}

// ---------------- writer pool (7z solid blocks + tar) ----------------

/// small files are read into memory by the decoder thread and written by all other threads
const SMALL: u64 = 16 << 20;

/// hands a small file (path, content, time) to the writer threads
type SendFile<'a> = &'a dyn Fn(PathBuf, Vec<u8>, Option<SystemTime>);

struct Job {
    path: PathBuf,
    data: Vec<u8>,
    mtime: Option<SystemTime>,
}

/// runs `decode` on this thread; it hands small files to `threads` writer threads via the sender
fn with_writers<F>(threads: usize, opts: &Options, prog: &Progress, cache: &DirCache, decode: F) -> Result<(), String>
where
    F: FnOnce(SendFile) -> Result<(), String>,
{
    let (tx, rx) = sync_channel::<Job>(256);
    let rx = std::sync::Mutex::new(rx);
    std::thread::scope(|s| {
        for _ in 0..threads.max(1) {
            s.spawn(|| loop {
                let job = match rx.lock().unwrap().recv() {
                    Ok(j) => j,
                    Err(_) => break,
                };
                let r = cache
                    .ensure_parent(&job.path)
                    .map_err(|e| format!("{}: {e}", job.path.display()))
                    .and_then(|_| fsx::write_bytes(&job.path, &job.data, job.mtime, opts.overwrite, prog));
                if let Err(e) = r {
                    prog.error(e);
                }
            });
        }
        let send = |path: PathBuf, data: Vec<u8>, mtime: Option<SystemTime>| {
            let _ = tx.send(Job { path, data, mtime });
        };
        let r = decode(&send);
        drop(tx);
        r
    })
}

// ---------------- 7z ----------------

fn seven_z(archive: &Path, dest: &Path, opts: &Options, prog: &Progress) -> Result<(), String> {
    use sevenz_rust2::{Archive, BlockDecoder, Password};
    prog.set_phase("reading the 7z directory ...");
    let pw = Password::empty();
    let mut f = File::open(archive).map_err(|e| e.to_string())?;
    let ar = Archive::read(&mut f, &pw).map_err(|e| format!("7z: {e} (password protected 7z is not supported)"))?;
    let files: Vec<_> = ar.files.iter().filter(|e| !e.is_anti_item).collect();
    prog.total_files.store(files.iter().filter(|e| !e.is_directory()).count() as u64, Ordering::Relaxed);
    prog.total_bytes.store(files.iter().map(|e| e.size()).sum(), Ordering::Relaxed);

    let cache = DirCache::default();
    // folders + empty files are not inside any block
    for e in &files {
        let Some(p) = safe_join(dest, e.name()) else {
            prog.error(format!("skipped unsafe path: {}", e.name()));
            continue;
        };
        if e.is_directory() {
            cache.ensure(&p).map_err(|x| format!("{}: {x}", p.display()))?;
        } else if !e.has_stream {
            let _ = cache.ensure_parent(&p);
            if let Err(x) = fsx::write_bytes(&p, &[], mtime_7z(e), opts.overwrite, prog) {
                prog.error(x);
            }
        }
    }

    let blocks = ar.blocks.len();
    prog.set_phase(&format!("unpacking ({blocks} blocks) ..."));
    let write_entry = |entry: &sevenz_rust2::ArchiveEntry, reader: &mut dyn Read, buf: &mut Vec<u8>, send: Option<SendFile>| {
        if prog.cancelled() {
            return Ok(false);
        }
        if entry.is_directory() {
            return Ok(true);
        }
        let Some(p) = safe_join(dest, entry.name()) else { return Ok(true) };
        let size = entry.size();
        match send {
            Some(send) if size <= SMALL => {
                let mut data = Vec::with_capacity(size as usize);
                reader.read_to_end(&mut data)?;
                send(p, data, mtime_7z(entry));
            }
            _ => {
                let _ = cache.ensure_parent(&p);
                if let Err(e) = fsx::write_stream(&p, reader, size, mtime_7z(entry), opts.overwrite, buf, prog) {
                    prog.error(e);
                }
            }
        }
        Ok(true)
    };

    if blocks > 1 {
        // independent blocks: one per thread
        let inner = (opts.threads / blocks).max(1) as u32;
        fsx::pool(opts.threads).install(|| {
            (0..blocks).into_par_iter().for_each_init(Vec::new, |buf, bi| {
                if prog.cancelled() {
                    return;
                }
                let mut src = match File::open(archive) {
                    Ok(f) => BufReader::with_capacity(1 << 20, f),
                    Err(e) => return prog.error(e.to_string()),
                };
                let dec = BlockDecoder::new(inner, bi, &ar, &pw, &mut src);
                if let Err(e) = dec.for_each_entries(&mut |entry, reader| write_entry(entry, reader, buf, None)) {
                    prog.error(format!("7z block {bi}: {e}"));
                }
            })
        });
    } else if blocks == 1 {
        // solid: one decoder (multi threaded LZMA2 if the archive allows it), all others write
        let writers = opts.threads.saturating_sub(1).max(1);
        with_writers(writers, opts, prog, &cache, |send| {
            let mut src = BufReader::with_capacity(1 << 20, File::open(archive).map_err(|e| e.to_string())?);
            let mut buf = Vec::new();
            let dec = BlockDecoder::new(opts.threads as u32, 0, &ar, &pw, &mut src);
            dec.for_each_entries(&mut |entry, reader| write_entry(entry, reader, &mut buf, Some(send)))
                .map(|_| ())
                .map_err(|e| format!("7z: {e}"))
        })?;
    }
    if prog.cancelled() {
        return Err("cancelled".into());
    }
    Ok(())
}

fn mtime_7z(e: &sevenz_rust2::ArchiveEntry) -> Option<SystemTime> {
    e.has_last_modified_date.then(|| SystemTime::from(e.last_modified_date))
}

// ---------------- tar + single compressed files ----------------

fn decoder(path: &Path, c: Comp, threads: usize) -> Result<Box<dyn Read + Send>, String> {
    let f = File::open(path).map_err(|e| e.to_string())?;
    Ok(match c {
        Comp::None => Box::new(BufReader::with_capacity(1 << 20, f)),
        Comp::Gz => Box::new(flate2::read::MultiGzDecoder::new(BufReader::with_capacity(1 << 20, f))),
        Comp::Zst => Box::new(zstd::stream::read::Decoder::new(f).map_err(|e| e.to_string())?),
        // xz made with -T0 has many blocks -> decoded on all cores
        Comp::Xz => Box::new(lzma_rust2::XzReaderMt::new(BufReader::with_capacity(1 << 20, f), true, threads as u32).map_err(|e| e.to_string())?),
        Comp::Bz2 => Box::new(bzip2::read::MultiBzDecoder::new(BufReader::with_capacity(1 << 20, f))),
    })
}

fn tar(archive: &Path, c: Comp, dest: &Path, opts: &Options, prog: &Progress) -> Result<(), String> {
    // tar has no directory up front: totals grow while reading
    let cache = DirCache::default();
    let writers = opts.threads.saturating_sub(1).max(1);
    prog.set_phase("unpacking ...");
    with_writers(writers, opts, prog, &cache, |send| {
        let mut ar = tar::Archive::new(decoder(archive, c, opts.threads)?);
        let mut buf = Vec::new();
        for entry in ar.entries().map_err(|e| format!("tar: {e}"))? {
            if prog.cancelled() {
                return Err("cancelled".into());
            }
            let mut entry = entry.map_err(|e| format!("tar: {e}"))?;
            let name = entry.path().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
            let Some(p) = safe_join(dest, &name) else {
                if !name.is_empty() && name != "./" {
                    prog.error(format!("skipped unsafe path: {name}"));
                }
                continue;
            };
            let kind = entry.header().entry_type();
            let mtime = entry.header().mtime().ok().and_then(fsx::unix_to_system);
            if kind.is_dir() {
                cache.ensure(&p).map_err(|e| format!("{}: {e}", p.display()))?;
            } else if kind.is_file() {
                let size = entry.size();
                prog.total_files.fetch_add(1, Ordering::Relaxed);
                prog.total_bytes.fetch_add(size, Ordering::Relaxed);
                if size <= SMALL {
                    let mut data = Vec::with_capacity(size as usize);
                    entry.read_to_end(&mut data).map_err(|e| format!("tar: {e}"))?;
                    send(p, data, mtime);
                } else {
                    let _ = cache.ensure_parent(&p);
                    if let Err(e) = fsx::write_stream(&p, &mut entry, size, mtime, opts.overwrite, &mut buf, prog) {
                        prog.error(e);
                    }
                }
            } else if kind.is_symlink() || kind.is_hard_link() {
                prog.error(format!("skipped link: {name}"));
            }
        }
        Ok(())
    })
}

fn single(archive: &Path, c: Comp, dest: &Path, opts: &Options, prog: &Progress) -> Result<(), String> {
    let name = archive.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "out".into());
    let out = dest.join(name);
    prog.total_files.store(1, Ordering::Relaxed);
    prog.set_phase("unpacking ...");
    let mut d = decoder(archive, c, opts.threads)?;
    let mut buf = Vec::new();
    fsx::write_stream(&out, &mut d, 0, None, opts.overwrite, &mut buf, prog).map(|_| ())
}

// ---------------- rar: the installed WinRAR does it ----------------

pub fn unrar_exe() -> Option<PathBuf> {
    let mut c = vec![];
    for var in ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)"] {
        if let Some(pf) = std::env::var_os(var) {
            let pf = PathBuf::from(pf);
            c.push(pf.join(r"WinRAR\UnRAR.exe"));
            c.push(pf.join(r"7-Zip\7z.exe"));
        }
    }
    c.into_iter().find(|p| p.exists())
}

fn rar(archive: &Path, dest: &Path, opts: &Options, prog: &Progress) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let exe = unrar_exe().ok_or("for .rar WinRAR or 7-Zip has to be installed")?;
    let is_7z = exe.file_name().is_some_and(|n| n.eq_ignore_ascii_case("7z.exe"));
    let mut cmd = std::process::Command::new(&exe);
    let mut out_dir = dest.to_string_lossy().to_string();
    if is_7z {
        cmd.arg("x").arg(format!("-mmt{}", opts.threads)).arg(if opts.overwrite { "-aoa" } else { "-aos" }).arg("-y");
        cmd.arg(format!("-o{out_dir}")).arg(archive);
    } else {
        if !out_dir.ends_with('\\') {
            out_dir.push('\\');
        }
        cmd.arg("x").arg(format!("-mt{}", opts.threads.min(64))).arg(if opts.overwrite { "-o+" } else { "-o-" }).arg("-y").arg("-idq");
        cmd.arg(archive).arg(&out_dir);
    }
    prog.set_phase(&format!("unpacking with {} ...", exe.file_name().unwrap_or_default().to_string_lossy()));
    let mut child = cmd.creation_flags(crate::win::CREATE_NO_WINDOW).spawn().map_err(|e| format!("{}: {e}", exe.display()))?;
    // same boost as the rule for WinRAR / 7-Zip
    let _ = profile::apply(child.id(), &Profile { tree: true, ..Profile::boost() }, false);
    loop {
        if prog.cancelled() {
            let _ = child.kill();
            return Err("cancelled".into());
        }
        if let Some(st) = child.try_wait().map_err(|e| e.to_string())? {
            // UnRAR: 0 ok, 1 warning; 7z: 0 ok, 1 warning
            let code = st.code().unwrap_or(255);
            let (files, bytes) = count_tree(dest);
            prog.done_files.store(files, Ordering::Relaxed);
            prog.done_bytes.store(bytes, Ordering::Relaxed);
            return if code <= 1 { Ok(()) } else { Err(format!("{} ended with code {code}", exe.display())) };
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn count_tree(dir: &Path) -> (u64, u64) {
    let (mut files, mut bytes) = (0, 0);
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            match e.file_type() {
                Ok(t) if t.is_dir() => stack.push(e.path()),
                Ok(_) => {
                    files += 1;
                    bytes += e.metadata().map(|m| m.len()).unwrap_or(0);
                }
                Err(_) => {}
            }
        }
    }
    (files, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn names() {
        assert_eq!(kind_of(Path::new("a.tar.gz")), Some(Kind::Tar(Comp::Gz)));
        assert_eq!(kind_of(Path::new("a.ZIP")), Some(Kind::Zip));
        assert_eq!(kind_of(Path::new("a.txt.zst")), Some(Kind::Single(Comp::Zst)));
        assert_eq!(default_dest(Path::new(r"C:\dl\project.tar.gz")), PathBuf::from(r"C:\dl\project"));
        assert_eq!(default_dest(Path::new(r"C:\dl\x.zip")), PathBuf::from(r"C:\dl\x"));
        assert_eq!(default_dest(Path::new(r"C:\dl\log.txt.gz")), PathBuf::from(r"C:\dl"));
    }

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("pcmax-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// 300 small files in folders + one 5 MB file -> zip / tar.gz / tar.zst -> unpack -> same bytes
    fn sample() -> Vec<(String, Vec<u8>)> {
        let mut v: Vec<(String, Vec<u8>)> =
            (0..300).map(|i| (format!("dir{}/sub/f{i}.txt", i % 7), format!("file {i} ").repeat(i % 50 + 1).into_bytes())).collect();
        v.push(("big.bin".into(), (0..5_000_000u32).map(|i| (i * 7 % 251) as u8).collect()));
        v.push(("empty.txt".into(), vec![]));
        v
    }

    fn check(dir: &Path, files: &[(String, Vec<u8>)]) {
        for (n, data) in files {
            let got = std::fs::read(dir.join(n)).unwrap_or_else(|e| panic!("{n}: {e}"));
            assert_eq!(&got, data, "{n}");
        }
    }

    #[test]
    fn zip_roundtrip() {
        let d = tmp("zip");
        let files = sample();
        let zpath = d.join("t.zip");
        let mut w = zip::ZipWriter::new(File::create(&zpath).unwrap());
        let o: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default();
        w.add_directory("only-dir/", o).unwrap();
        for (n, data) in &files {
            w.start_file(n, o).unwrap();
            w.write_all(data).unwrap();
        }
        w.finish().unwrap();
        let prog = Progress::default();
        let out = d.join("out");
        let s = extract(&zpath, &out, &Options { threads: 8, ..Default::default() }, &prog).unwrap();
        assert!(s.errors.is_empty(), "{:?}", s.errors);
        check(&out, &files);
        assert!(out.join("only-dir").is_dir());
        // second run without overwrite keeps everything
        let prog = Progress::default();
        let s = extract(&zpath, &out, &Options { threads: 8, ..Default::default() }, &prog).unwrap();
        assert_eq!(prog.skipped.load(Ordering::Relaxed), files.len() as u64);
        assert!(s.notes.iter().any(|n| n.contains("kept")));
        let _ = std::fs::remove_dir_all(&d);
    }

    fn make_tar(files: &[(String, Vec<u8>)], w: impl Write) {
        let mut b = tar::Builder::new(w);
        for (n, data) in files {
            let mut h = tar::Header::new_gnu();
            h.set_size(data.len() as u64);
            h.set_mode(0o644);
            h.set_mtime(1_700_000_000);
            h.set_cksum();
            b.append_data(&mut h, n, data.as_slice()).unwrap();
        }
        b.into_inner().unwrap();
    }

    #[test]
    fn tar_gz_and_zst_roundtrip() {
        let d = tmp("tar");
        let files = sample();
        let gz = d.join("t.tar.gz");
        let enc = flate2::write::GzEncoder::new(File::create(&gz).unwrap(), flate2::Compression::fast());
        make_tar(&files, enc);
        let zs = d.join("t.tar.zst");
        let enc = zstd::stream::write::Encoder::new(File::create(&zs).unwrap(), 3).unwrap().auto_finish();
        make_tar(&files, enc);
        for a in [gz, zs] {
            let out = d.join(a.file_name().unwrap().to_string_lossy().replace('.', "_"));
            let prog = Progress::default();
            let s = extract(&a, &out, &Options { threads: 8, ..Default::default() }, &prog).unwrap();
            assert!(s.errors.is_empty(), "{:?}", s.errors);
            check(&out, &files);
            let t = std::fs::metadata(out.join("big.bin")).unwrap().modified().unwrap();
            assert_eq!(t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(), 1_700_000_000);
        }
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn seven_z_roundtrip() {
        let d = tmp("7z");
        let src = d.join("src");
        let files = sample();
        for (n, data) in &files {
            let p = src.join(n);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, data).unwrap();
        }
        let a = d.join("t.7z");
        sevenz_rust2::compress_to_path(&src, &a).unwrap();
        let out = d.join("out");
        let prog = Progress::default();
        let s = extract(&a, &out, &Options { threads: 8, ..Default::default() }, &prog).unwrap();
        assert!(s.errors.is_empty(), "{:?}", s.errors);
        check(&out, &files);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn rejects_zip_slip() {
        let d = tmp("slip");
        let zpath = d.join("evil.zip");
        let mut w = zip::ZipWriter::new(File::create(&zpath).unwrap());
        w.start_file("../evil.txt", zip::write::SimpleFileOptions::default()).unwrap();
        w.write_all(b"x").unwrap();
        w.start_file("ok.txt", zip::write::SimpleFileOptions::default()).unwrap();
        w.write_all(b"y").unwrap();
        w.finish().unwrap();
        let out = d.join("out");
        let prog = Progress::default();
        extract(&zpath, &out, &Options::default(), &prog).unwrap();
        assert!(!d.join("evil.txt").exists());
        assert!(out.join("ok.txt").exists());
        assert!(prog.errors().iter().any(|e| e.contains("unsafe")));
        let _ = std::fs::remove_dir_all(&d);
    }
}
