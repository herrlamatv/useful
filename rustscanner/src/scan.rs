// Parallel scan: a few threads share a stack of folders to list. Every folder is listed and sorted
// on its own, then one pass puts everything together in name order -> the same result no matter
// how many threads ran or in which order they finished.

use crate::model::*;
use crate::rules::{self, Siblings};
use std::fs;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering::Relaxed};
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

/// never entered, like in filescanner.bat
const SKIP_NAMES: [&str; 2] = ["$Recycle.Bin", "System Volume Information"];

#[derive(Default)]
pub struct Progress {
    pub folders: AtomicU64,
    pub files: AtomicU64,
    pub bytes: AtomicU64,
    pub denied: AtomicU64,
    pub cancel: AtomicBool,
    /// folder that is listed right now (updated every ~50 ms)
    pub current: Mutex<String>,
}

pub struct Options {
    /// from normalize_roots()
    pub roots: Vec<String>,
    /// don't enter Windows / Program Files / ProgramData / AppData
    pub skip_system: bool,
    pub threads: usize,
}

pub fn default_threads() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(2, 12)
}

/// Existing folders as full paths, without duplicates and without folders that are inside
/// another chosen one (same as filescanner.bat). Returns (roots, dropped because nested).
pub fn normalize_roots(list: &[String]) -> (Vec<String>, Vec<String>) {
    let mut found: Vec<String> = Vec::new();
    for p in list {
        let p = p.trim().trim_matches('"').trim();
        if p.is_empty() || !Path::new(p).is_dir() {
            continue;
        }
        let mut full = std::path::absolute(p).map(|x| x.to_string_lossy().into_owned()).unwrap_or_else(|_| p.into());
        if full.len() > 3 {
            full = full.trim_end_matches('\\').to_string();
        }
        if full.len() >= 2 && full.as_bytes()[1] == b':' {
            full = full[..1].to_ascii_uppercase() + &full[1..];
        }
        if full.len() == 2 {
            full.push('\\');
        }
        found.push(full);
    }
    found.sort_by_key(|s| s.len());
    let mut roots: Vec<String> = Vec::new();
    let mut nested = Vec::new();
    for f in found {
        if roots.iter().any(|r| is_inside(&f, r)) {
            if !roots.iter().any(|r| r.eq_ignore_ascii_case(&f)) {
                nested.push(f);
            }
        } else {
            roots.push(f);
        }
    }
    roots.sort_by(|a, b| natural_cmp(a, b));
    (roots, nested)
}

/// child == parent or child is somewhere below parent
pub fn is_inside(child: &str, parent: &str) -> bool {
    let c = child.trim_end_matches('\\').to_lowercase() + "\\";
    let p = parent.trim_end_matches('\\').to_lowercase() + "\\";
    c.starts_with(&p)
}

struct Job {
    path: PathBuf,
    id: u32,
}

struct Sub {
    name_off: u32,
    name_len: u16,
    /// NONE = not entered
    id: u32,
    rules: u16,
    flags: u8,
    mtime: i64,
}

#[derive(Default)]
struct DirResult {
    names: String,
    files: Vec<FileRec>,
    subs: Vec<Sub>,
    html: u16,
    rules: u16,
    flags: u8,
}

struct Queue {
    state: Mutex<QState>,
    cv: Condvar,
}

struct QState {
    jobs: Vec<Job>,
    /// queued + being listed right now; 0 = all done
    pending: usize,
}

pub fn scan(opts: &Options, prog: &Progress) -> Index {
    let started = unix_now();
    let clock = Instant::now();
    let roots = &opts.roots;
    let next_id = AtomicU32::new(roots.len() as u32);
    let queue = Queue {
        state: Mutex::new(QState {
            jobs: roots.iter().enumerate().rev().map(|(i, r)| Job { path: PathBuf::from(r), id: i as u32 }).collect(),
            pending: roots.len(),
        }),
        cv: Condvar::new(),
    };

    let mut results: Vec<(u32, DirResult)> = Vec::new();
    std::thread::scope(|s| {
        let workers: Vec<_> =
            (0..opts.threads.max(1)).map(|_| s.spawn(|| worker(&queue, &next_id, opts, prog))).collect();
        for w in workers {
            if let Ok(mut r) = w.join() {
                results.append(&mut r);
            }
        }
    });

    let mut index = assemble(roots, results, next_id.load(Relaxed) as usize);
    index.started = started;
    index.duration = clock.elapsed();
    index.denied = prog.denied.load(Relaxed) as u32;
    index.cancelled = prog.cancel.load(Relaxed);
    index
}

fn worker(q: &Queue, next_id: &AtomicU32, opts: &Options, prog: &Progress) -> Vec<(u32, DirResult)> {
    let mut out = Vec::new();
    let mut last_report = Instant::now() - Duration::from_secs(1);
    loop {
        let job = {
            let mut st = q.state.lock().unwrap();
            loop {
                if prog.cancel.load(Relaxed) {
                    return out;
                }
                if let Some(j) = st.jobs.pop() {
                    break j;
                }
                if st.pending == 0 {
                    return out;
                }
                st = q.cv.wait_timeout(st, Duration::from_millis(50)).unwrap().0;
            }
        };
        if last_report.elapsed() >= Duration::from_millis(50) {
            last_report = Instant::now();
            *prog.current.lock().unwrap() = job.path.to_string_lossy().into_owned();
        }

        let mut children = Vec::new();
        let res = list_dir(&job.path, opts, prog, next_id, &mut children);
        out.push((job.id, res));

        let n = children.len();
        let mut st = q.state.lock().unwrap();
        // reversed, so the first child is popped first (roughly alphabetical, keeps the stack small)
        st.jobs.extend(children.into_iter().rev());
        st.pending = st.pending + n - 1;
        let all_done = st.pending == 0;
        drop(st);
        if all_done || n > 1 {
            q.cv.notify_all();
        }
    }
}

fn list_dir(path: &Path, opts: &Options, prog: &Progress, next_id: &AtomicU32, jobs: &mut Vec<Job>) -> DirResult {
    let mut r = DirResult::default();
    prog.folders.fetch_add(1, Relaxed);
    let rd = match fs::read_dir(path) {
        Ok(rd) => rd,
        Err(_) => {
            prog.denied.fetch_add(1, Relaxed);
            r.flags |= F_DENIED;
            return r;
        }
    };

    let (mut count, mut bytes) = (0u64, 0u64);
    for entry in rd.flatten() {
        // file_type + metadata come from the directory listing on Windows, no extra disk access
        let Ok(ft) = entry.file_type() else { continue };
        // junctions + symlinks ("Application Data", "My Documents" ...) would loop or scan things twice.
        // is_symlink() is only true for those, OneDrive placeholders are normal files/folders.
        if ft.is_symlink() {
            continue;
        }
        let Ok(md) = entry.metadata() else { continue };
        let os_name = entry.file_name();
        let name = os_name.to_string_lossy();
        let attrs = md.file_attributes();
        let name_off = r.names.len() as u32;

        if ft.is_dir() {
            if SKIP_NAMES.iter().any(|s| name.eq_ignore_ascii_case(s)) {
                continue;
            }
            if name.eq_ignore_ascii_case(".git") {
                r.flags |= F_REPO;
            }
            r.names.push_str(&name);
            r.subs.push(Sub {
                name_off,
                name_len: name.len() as u16,
                id: NONE,
                rules: 0,
                flags: folder_flags(attrs),
                mtime: filetime_to_unix(md.last_write_time()),
            });
        } else {
            if name.eq_ignore_ascii_case(".git") {
                r.flags |= F_REPO; // worktrees / submodules have a .git file
            }
            let ext = ext_of(&name);
            if is_html_ext(ext) {
                r.html = r.html.saturating_add(1);
            }
            let size = md.file_size();
            r.files.push(FileRec {
                size,
                mtime: filetime_to_unix(md.last_write_time()),
                ctime: filetime_to_unix(md.creation_time()),
                folder: 0,
                name_off,
                name_len: name.len() as u16,
                ext_len: if ext.len() <= 255 { ext.len() as u8 } else { 0 },
                cats: ext_cats(ext),
                flags: file_flags(attrs),
            });
            r.names.push_str(&name);
            count += 1;
            bytes += size;
        }
    }
    prog.files.fetch_add(count, Relaxed);
    prog.bytes.fetch_add(bytes, Relaxed);

    let DirResult { names, files, subs, rules: own_rules, .. } = &mut r;
    let name_at = |off: u32, len: u16| &names[off as usize..off as usize + len as usize];
    files.sort_unstable_by(|a, b| natural_cmp(name_at(a.name_off, a.name_len), name_at(b.name_off, b.name_len)));
    subs.sort_unstable_by(|a, b| natural_cmp(name_at(a.name_off, a.name_len), name_at(b.name_off, b.name_len)));

    let sib = Siblings { names, files };
    *own_rules = rules::detect_own(&sib);
    let drive_root = path.parent().is_none();
    let parent_is_profile =
        path.parent().and_then(|p| p.file_name()).is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("Users"));
    for s in subs.iter_mut() {
        let name = name_at(s.name_off, s.name_len);
        s.rules = rules::detect_child(name, drive_root, parent_is_profile, &sib);
        if opts.skip_system && s.rules & rules::R_SYSTEM != 0 {
            s.flags |= F_SKIPPED;
            continue;
        }
        s.id = next_id.fetch_add(1, Relaxed);
        jobs.push(Job { path: path.join(name), id: s.id });
    }
    r
}

fn folder_flags(attrs: u32) -> u8 {
    let mut f = 0;
    if attrs & ATTR_HIDDEN != 0 {
        f |= F_HIDDEN;
    }
    if attrs & ATTR_SYSTEM != 0 {
        f |= F_SYSTEM;
    }
    if attrs & (ATTR_OFFLINE | ATTR_RECALL_ON_OPEN | ATTR_RECALL_ON_DATA_ACCESS) != 0 {
        f |= F_CLOUD;
    }
    f
}

fn file_flags(attrs: u32) -> u8 {
    let mut f = 0;
    if attrs & ATTR_HIDDEN != 0 {
        f |= FL_HIDDEN;
    }
    if attrs & ATTR_SYSTEM != 0 {
        f |= FL_SYSTEM;
    }
    if attrs & (ATTR_OFFLINE | ATTR_RECALL_ON_DATA_ACCESS) != 0 {
        f |= FL_CLOUD;
    }
    f
}

/// puts the per-folder results together in pre-order (name order), then sums up the subtrees
fn assemble(roots: &[String], results: Vec<(u32, DirResult)>, ids: usize) -> Index {
    let mut slots: Vec<Option<DirResult>> = (0..ids).map(|_| None).collect();
    for (id, r) in results {
        slots[id as usize] = Some(r);
    }

    struct Pending {
        id: u32,
        parent: u32,
        /// roots: their path (pushed into names first), others: offset into names
        root_path: Option<String>,
        name_off: u32,
        name_len: u16,
        rules: u16,
        flags: u8,
        mtime: i64,
    }

    let mut idx = Index::default();
    let mut stack: Vec<Pending> = roots
        .iter()
        .enumerate()
        .rev()
        .map(|(i, r)| {
            let md = fs::metadata(r).ok();
            Pending {
                id: i as u32,
                parent: NONE,
                root_path: Some(r.clone()),
                name_off: 0,
                name_len: 0,
                rules: 0,
                flags: F_ROOT | md.as_ref().map(|m| folder_flags(m.file_attributes())).unwrap_or(0),
                mtime: md.map(|m| filetime_to_unix(m.last_write_time())).unwrap_or(0),
            }
        })
        .collect();

    while let Some(p) = stack.pop() {
        let f = idx.folders.len() as u32;
        let (name_off, name_len) = match p.root_path {
            Some(path) => {
                idx.roots.push(Root { path: path.clone(), folder: f });
                let off = idx.names.len() as u32;
                idx.names.push_str(&path);
                (off, path.len() as u16)
            }
            None => (p.name_off, p.name_len),
        };
        let start = idx.files.len() as u32;
        let mut d = Folder {
            parent: p.parent,
            name_off,
            name_len,
            subtree_end: f + 1,
            file_start: start,
            file_end: start,
            mtime: p.mtime,
            rules: p.rules,
            flags: p.flags,
            ..Default::default()
        };
        if let Some(r) = slots.get_mut(p.id as usize).and_then(Option::take) {
            d.html = r.html;
            d.rules |= r.rules;
            d.flags |= r.flags;
            let base = idx.names.len() as u32;
            idx.names.push_str(&r.names);
            for mut file in r.files {
                file.folder = f;
                file.name_off += base;
                d.sub_bytes += file.size;
                idx.files.push(file);
            }
            d.file_end = idx.files.len() as u32;
            d.sub_files = d.file_end - d.file_start;
            for s in r.subs.iter().rev() {
                stack.push(Pending {
                    id: s.id,
                    parent: f,
                    root_path: None,
                    name_off: base + s.name_off,
                    name_len: s.name_len,
                    rules: s.rules,
                    flags: s.flags,
                    mtime: s.mtime,
                });
            }
        }
        idx.folders.push(d);
    }

    // children have higher indices than their parent -> one backwards pass sums everything up
    for f in (0..idx.folders.len()).rev() {
        let d = idx.folders[f];
        if d.parent != NONE {
            let p = &mut idx.folders[d.parent as usize];
            p.subtree_end = p.subtree_end.max(d.subtree_end);
            p.sub_files += d.sub_files;
            p.sub_bytes += d.sub_bytes;
        }
    }
    idx
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::rules::*;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    /// a small folder tree with every case the rules have to handle
    pub struct Fixture {
        pub dir: PathBuf,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            // removes the junction itself, never what it points to
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    pub fn touch(p: &Path, size: usize) {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        File::create(p).unwrap().write_all(&vec![b'x'; size]).unwrap();
    }

    pub fn set_year(p: &Path, year: i32) {
        // mid of the year, so time zones can't move it to another year
        let secs = ((year - 1970) as u64) * 31_556_952 + 180 * 86_400;
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(secs);
        use std::os::windows::fs::FileTimesExt;
        let f = File::options().write(true).open(p).unwrap();
        f.set_times(fs::FileTimes::new().set_modified(t).set_accessed(t).set_created(t)).unwrap();
    }

    pub fn fixture(tag: &str) -> Fixture {
        let dir = std::env::temp_dir().join(format!("rustscanner_test_{tag}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let d = |s: &str| dir.join(s);
        for f in ["a.html", "b.html", "c.html", "notes.txt", "app.js"] {
            touch(&d(f), 10);
        }
        for f in ["Downloads/1.html", "Downloads/2.html", "Downloads/3.html", "Downloads/sub/x.pdf"] {
            touch(&d(f), 100);
        }
        touch(&d("Proj/rs/Cargo.toml"), 5);
        touch(&d("Proj/rs/src/main.rs"), 50);
        touch(&d("Proj/rs/target/debug/app.exe"), 1000);
        touch(&d("Proj/other/target/keep.txt"), 7);
        touch(&d("Proj/web/index.js"), 20);
        touch(&d("Proj/web/node_modules/lib/x.js"), 20);
        touch(&d("Proj/web/node_modules/lib/y.js"), 20);
        touch(&d("Proj/c/.idea/workspace.xml"), 20);
        touch(&d("Proj/c/cmake-build-debug/out.o"), 20);
        touch(&d("Proj/c/main.cpp"), 20);
        touch(&d("Proj/py/venv2/pyvenv.cfg"), 5);
        touch(&d("Proj/py/venv2/Lib/mod.py"), 5);
        touch(&d("Proj/py/__pycache__/m.pyc"), 5);
        touch(&d("Proj/py/main.py"), 5);
        touch(&d("Proj/py/.git/HEAD"), 5);
        touch(&d("Seite.html"), 30);
        touch(&d("Seite_files/style.css"), 30);
        touch(&d("Fotos/2019/Urlaub.JPG"), 2_000_000);
        touch(&d("Fotos/2019/Ärger.jpeg"), 10);
        set_year(&d("Fotos/2019/Urlaub.JPG"), 2019);
        touch(&d("hid.txt"), 3);
        let wide: Vec<u16> = d("hid.txt").as_os_str().encode_wide_z();
        unsafe { windows_sys::Win32::Storage::FileSystem::SetFileAttributesW(wide.as_ptr(), ATTR_HIDDEN) };
        // junction to Proj -> must not be followed (no admin needed for /J)
        std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(d("link"))
            .arg(d("Proj"))
            .output()
            .unwrap();
        assert!(d("link").exists(), "junction was not created");
        Fixture { dir }
    }

    trait WideZ {
        fn encode_wide_z(&self) -> Vec<u16>;
    }
    impl WideZ for std::ffi::OsStr {
        fn encode_wide_z(&self) -> Vec<u16> {
            use std::os::windows::ffi::OsStrExt;
            self.encode_wide().chain(Some(0)).collect()
        }
    }

    pub fn scan_dir(dir: &Path, threads: usize) -> Index {
        let (roots, _) = normalize_roots(&[dir.to_string_lossy().into_owned()]);
        scan(&Options { roots, skip_system: false, threads }, &Progress::default())
    }

    #[test]
    fn scan_counts_and_junction() {
        let fx = fixture("scan");
        let idx = scan_dir(&fx.dir, 4);
        assert_eq!(idx.roots.len(), 1);
        assert_eq!(idx.files.len(), 29);
        assert!(idx.folder_path(0).ends_with(&format!("rustscanner_test_scan_{}", std::process::id())));
        // the junction is skipped completely
        assert!(idx.find_folder(&fx.dir.join("link").to_string_lossy()).is_none());
        // pre-order, parents first, subtree sums
        for (i, f) in idx.folders.iter().enumerate() {
            if f.parent != NONE {
                assert!((f.parent as usize) < i);
            }
        }
        assert_eq!(idx.folders[0].sub_files as usize, idx.files.len());
        assert_eq!(idx.folders[0].subtree_end as usize, idx.folders.len());
        // natural name order inside a folder
        let root_files: Vec<&str> = (idx.folders[0].file_start..idx.folders[0].file_end).map(|i| idx.file_name(i)).collect();
        assert_eq!(root_files, vec!["a.html", "app.js", "b.html", "c.html", "hid.txt", "notes.txt", "Seite.html"]);
        let hid = (0..idx.files.len() as u32).find(|&i| idx.file_name(i) == "hid.txt").unwrap();
        assert_ne!(idx.files[hid as usize].flags & FL_HIDDEN, 0);
    }

    #[test]
    fn scan_is_deterministic() {
        let fx = fixture("det");
        let a = scan_dir(&fx.dir, 1);
        let b = scan_dir(&fx.dir, 8);
        // NTFS updates folder times in the parent's listing lazily, a fresh tree can differ by a second
        let no_time = |v: &[Folder]| v.iter().map(|f| Folder { mtime: 0, ..*f }).collect::<Vec<_>>();
        assert_eq!(no_time(&a.folders), no_time(&b.folders));
        assert_eq!(a.files, b.files);
        assert_eq!(a.names, b.names);
    }

    #[test]
    fn scan_time_rules() {
        let fx = fixture("rules");
        let idx = scan_dir(&fx.dir, 4);
        let rules_of = |p: &str| idx.folders[idx.find_folder(&fx.dir.join(p).to_string_lossy()).unwrap() as usize].rules;
        assert_eq!(rules_of("Proj/rs/target"), R_BUILD);
        assert_eq!(rules_of("Proj/other/target"), 0);
        assert_eq!(rules_of("Proj/web/node_modules"), R_NODE);
        assert_eq!(rules_of("Proj/c/.idea"), R_JETBRAINS);
        assert_eq!(rules_of("Proj/c/cmake-build-debug"), R_JETBRAINS);
        assert_eq!(rules_of("Proj/py/venv2"), R_PYTHON);
        assert_eq!(rules_of("Proj/py/__pycache__"), R_PYTHON);
        assert_eq!(rules_of("Proj/py/.git"), R_VCS);
        assert_eq!(rules_of("Seite_files"), R_SAVED);
        let py = idx.find_folder(&fx.dir.join("Proj/py").to_string_lossy()).unwrap();
        assert_ne!(idx.folders[py as usize].flags & F_REPO, 0);
        let dl = idx.find_folder(&fx.dir.join("downloads").to_string_lossy()).unwrap();
        assert_eq!(idx.folders[dl as usize].html, 3);
    }

    #[test]
    fn roots_are_deduped() {
        let fx = fixture("roots");
        let a = fx.dir.to_string_lossy().into_owned();
        let b = fx.dir.join("Proj").to_string_lossy().into_owned();
        let (roots, nested) = normalize_roots(&[b.clone(), a.clone(), a.clone() + "\\", "Z:\\does\\not\\exist".into()]);
        assert_eq!(roots, vec![a]);
        assert_eq!(nested, vec![b]);
    }
}
