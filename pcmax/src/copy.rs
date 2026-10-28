// Fast copy: read the tree once, create all folders, then copy the files on all cores
// (like robocopy /MT). Small files go through CopyFileExW (keeps attributes + times),
// big ones through our own loop so the progress bar moves.

use crate::fsx::{self, Progress, Summary};
use rayon::prelude::*;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct Options {
    pub threads: usize,
    pub overwrite: bool,
    /// compare every byte afterwards
    pub verify: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options { threads: fsx::default_threads().min(16), overwrite: false, verify: false }
    }
}

struct Item {
    from: PathBuf,
    to: PathBuf,
    size: u64,
}

const BIG: u64 = 64 << 20;

/// copies `src` (file or folder) INTO the folder `dst_dir`, like Explorer
pub fn copy(src: &Path, dst_dir: &Path, opts: &Options, prog: &Progress) -> Result<Summary, String> {
    let src = src.canonicalize().map_err(|e| format!("{}: {e}", src.display()))?;
    std::fs::create_dir_all(dst_dir).map_err(|e| format!("{}: {e}", dst_dir.display()))?;
    let dst_dir = dst_dir.canonicalize().map_err(|e| format!("{}: {e}", dst_dir.display()))?;
    let name = src.file_name().ok_or("can't copy a drive root")?;
    let target = dst_dir.join(name);
    if dst_dir.starts_with(&src) {
        return Err("the target is inside the source folder".into());
    }
    if target == src {
        return Err("source and target are the same".into());
    }

    let t0 = Instant::now();
    prog.set_phase("reading the folder ...");
    let mut dirs = vec![];
    let mut files = vec![];
    let mut links = 0;
    if src.is_file() {
        let size = src.metadata().map(|m| m.len()).unwrap_or(0);
        files.push(Item { from: src.clone(), to: target.clone(), size });
    } else {
        let mut stack = vec![(src.clone(), target.clone())];
        while let Some((from, to)) = stack.pop() {
            dirs.push(to.clone());
            let rd = match std::fs::read_dir(&from) {
                Ok(r) => r,
                Err(e) => {
                    prog.error(format!("{}: {e}", from.display()));
                    continue;
                }
            };
            for e in rd.flatten() {
                let Ok(t) = e.file_type() else { continue };
                if t.is_symlink() {
                    // junctions / links are not followed (could point anywhere, even in a loop)
                    links += 1;
                } else if t.is_dir() {
                    stack.push((e.path(), to.join(e.file_name())));
                } else {
                    let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                    files.push(Item { from: e.path(), to: to.join(e.file_name()), size });
                }
            }
            if prog.cancelled() {
                return Err("cancelled".into());
            }
        }
    }
    prog.total_files.store(files.len() as u64, Ordering::Relaxed);
    prog.total_bytes.store(files.iter().map(|f| f.size).sum(), Ordering::Relaxed);

    prog.set_phase("creating folders ...");
    dirs.sort();
    for d in &dirs {
        std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    files.sort_by_key(|f| std::cmp::Reverse(f.size));

    prog.set_phase("copying ...");
    let pool = fsx::pool(opts.threads);
    pool.install(|| {
        files.par_iter().for_each_init(Vec::new, |buf, f| {
            if prog.cancelled() {
                return;
            }
            if let Err(e) = copy_one(f, opts.overwrite, buf, prog) {
                prog.error(e);
            }
        })
    });
    if prog.cancelled() {
        return Err("cancelled".into());
    }

    let mut notes = vec![];
    if links > 0 {
        notes.push(format!("{links} links / junctions were not followed"));
    }
    let skipped = prog.skipped.load(Ordering::Relaxed);
    if skipped > 0 {
        notes.push(format!("{skipped} files already existed and were kept (overwrite is off)"));
    }
    let secs = t0.elapsed().as_secs_f64();
    if opts.verify {
        prog.set_phase("comparing ...");
        let bad: Vec<String> = pool.install(|| {
            files.par_iter().filter(|f| !same_content(&f.from, &f.to)).map(|f| f.to.display().to_string()).collect()
        });
        if bad.is_empty() {
            notes.push("verified: every byte is the same".into());
        } else {
            for b in bad.iter().take(50) {
                prog.error(format!("differs: {b}"));
            }
            notes.push(format!("⚠ {} files differ", bad.len()));
        }
    }
    Ok(Summary {
        files: prog.done_files.load(Ordering::Relaxed),
        bytes: prog.done_bytes.load(Ordering::Relaxed),
        secs,
        dest: target,
        notes,
        errors: prog.errors(),
    })
}

fn copy_one(f: &Item, overwrite: bool, buf: &mut Vec<u8>, prog: &Progress) -> Result<(), String> {
    if !overwrite && f.to.exists() {
        prog.skipped.fetch_add(1, Ordering::Relaxed);
        prog.add_bytes(f.size);
        return Ok(());
    }
    let err = |e: std::io::Error| format!("{}: {e}", f.from.display());
    if f.size < BIG {
        // CopyFileExW: data + attributes + timestamps
        let n = std::fs::copy(&f.from, &f.to).map_err(err)?;
        prog.add_bytes(n);
        prog.file_done();
        return Ok(());
    }
    let meta = f.from.metadata().map_err(err)?;
    let mut src = File::open(&f.from).map_err(err)?;
    let mut dst = File::create(&f.to).map_err(|e| format!("{}: {e}", f.to.display()))?;
    let _ = dst.set_len(f.size);
    if buf.len() < 4 << 20 {
        buf.resize(4 << 20, 0);
    }
    loop {
        if prog.cancelled() {
            drop(dst);
            let _ = std::fs::remove_file(&f.to);
            return Err("cancelled".into());
        }
        let n = src.read(buf).map_err(err)?;
        if n == 0 {
            break;
        }
        dst.write_all(&buf[..n]).map_err(|e| format!("{}: {e}", f.to.display()))?;
        prog.add_bytes(n as u64);
    }
    if let Ok(t) = meta.modified() {
        let _ = dst.set_modified(t);
    }
    drop(dst);
    let _ = std::fs::set_permissions(&f.to, meta.permissions());
    prog.file_done();
    Ok(())
}

fn same_content(a: &Path, b: &Path) -> bool {
    let (Ok(mut fa), Ok(mut fb)) = (File::open(a), File::open(b)) else { return false };
    if fa.metadata().map(|m| m.len()).ok() != fb.metadata().map(|m| m.len()).ok() {
        return false;
    }
    let (mut ba, mut bb) = (vec![0u8; 1 << 20], vec![0u8; 1 << 20]);
    loop {
        let na = match fa.read(&mut ba) {
            Ok(n) => n,
            Err(_) => return false,
        };
        if na == 0 {
            return true;
        }
        if fb.read_exact(&mut bb[..na]).is_err() || ba[..na] != bb[..na] {
            return false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_tree() {
        let d = std::env::temp_dir().join(format!("pcmax-copy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let src = d.join("src");
        for i in 0..500 {
            let p = src.join(format!("a{}/b/f{i}.txt", i % 9));
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, format!("{i}").repeat(i + 1)).unwrap();
        }
        std::fs::create_dir_all(src.join("empty")).unwrap();
        let big: Vec<u8> = (0..70_000_000u32).map(|i| (i % 253) as u8).collect();
        std::fs::write(src.join("big.bin"), &big).unwrap();

        let prog = Progress::default();
        let s = copy(&src, &d.join("dst"), &Options { threads: 8, verify: true, ..Default::default() }, &prog).unwrap();
        assert!(s.errors.is_empty(), "{:?}", s.errors);
        assert_eq!(s.files, 501);
        assert!(s.notes.iter().any(|n| n.starts_with("verified")));
        assert!(d.join("dst/src/empty").is_dir());
        assert_eq!(std::fs::read(d.join("dst/src/a3/b/f12.txt")).unwrap(), "12".repeat(13).into_bytes());
        // into itself is refused
        assert!(copy(&src, &src.join("x"), &Options::default(), &Progress::default()).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
