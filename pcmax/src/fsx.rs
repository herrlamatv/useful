// File helpers for extract + copy: progress counters, safe paths, fast writes, read-only mmap.

use crate::win::{last_error, Handle};
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use windows_sys::Win32::Foundation::{FILETIME, SYSTEMTIME};
use windows_sys::Win32::System::Memory::{
    CreateFileMappingW, MapViewOfFile, UnmapViewOfFile, FILE_MAP_READ, MEMORY_MAPPED_VIEW_ADDRESS, PAGE_READONLY,
};
use windows_sys::Win32::System::Time::{SystemTimeToFileTime, TzSpecificLocalTimeToSystemTime};

/// shared between the worker threads and the UI / console
#[derive(Default)]
pub struct Progress {
    pub total_bytes: AtomicU64,
    pub done_bytes: AtomicU64,
    pub total_files: AtomicU64,
    pub done_files: AtomicU64,
    pub skipped: AtomicU64,
    pub cancel: AtomicBool,
    pub phase: Mutex<String>,
    pub errors: Mutex<Vec<String>>,
}

impl Progress {
    pub fn set_phase(&self, s: &str) {
        *self.phase.lock().unwrap() = s.to_string();
    }
    pub fn phase(&self) -> String {
        self.phase.lock().unwrap().clone()
    }
    pub fn error(&self, e: String) {
        let mut v = self.errors.lock().unwrap();
        // thousands of identical errors help nobody
        if v.len() < 200 {
            v.push(e);
        }
    }
    pub fn errors(&self) -> Vec<String> {
        self.errors.lock().unwrap().clone()
    }
    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
    pub fn add_bytes(&self, n: u64) {
        self.done_bytes.fetch_add(n, Ordering::Relaxed);
    }
    pub fn file_done(&self) {
        self.done_files.fetch_add(1, Ordering::Relaxed);
    }
    /// 0..1, None while the total is unknown
    pub fn fraction(&self) -> Option<f32> {
        let t = self.total_bytes.load(Ordering::Relaxed);
        if t > 0 {
            return Some((self.done_bytes.load(Ordering::Relaxed) as f64 / t as f64).min(1.0) as f32);
        }
        let tf = self.total_files.load(Ordering::Relaxed);
        (tf > 0).then(|| (self.done_files.load(Ordering::Relaxed) as f64 / tf as f64).min(1.0) as f32)
    }
}

pub struct Summary {
    pub files: u64,
    pub bytes: u64,
    pub secs: f64,
    pub dest: PathBuf,
    pub notes: Vec<String>,
    pub errors: Vec<String>,
}

impl Summary {
    pub fn line(&self) -> String {
        let speed = if self.secs > 0.0 { self.bytes as f64 / self.secs } else { 0.0 };
        format!(
            "{} files, {} in {:.1} s ({}/s)",
            self.files,
            crate::win::fmt_bytes(self.bytes),
            self.secs,
            crate::win::fmt_bytes(speed as u64)
        )
    }
}

/// `dest` + an untrusted archive path; None if it would leave `dest` (zip slip) or is absolute
pub fn safe_join(dest: &Path, name: &str) -> Option<PathBuf> {
    let norm = name.replace('\\', "/");
    let mut out = dest.to_path_buf();
    let mut any = false;
    for c in Path::new(&norm).components() {
        match c {
            Component::Normal(p) => {
                out.push(p);
                any = true;
            }
            Component::CurDir => {}
            _ => return None,
        }
    }
    any.then_some(out)
}

/// create_dir_all with a cache, so 50 000 files in one folder don't ask the disk 50 000 times
#[derive(Default)]
pub struct DirCache(Mutex<HashSet<PathBuf>>);

impl DirCache {
    pub fn ensure(&self, dir: &Path) -> std::io::Result<()> {
        if self.0.lock().unwrap().contains(dir) {
            return Ok(());
        }
        std::fs::create_dir_all(dir)?;
        self.0.lock().unwrap().insert(dir.to_path_buf());
        Ok(())
    }
    pub fn ensure_parent(&self, file: &Path) -> std::io::Result<()> {
        match file.parent() {
            Some(p) => self.ensure(p),
            None => Ok(()),
        }
    }
}

pub const BUF: usize = 1 << 20;

/// None = skipped because it exists and overwrite is off
fn open_out(path: &Path, overwrite: bool, size_hint: u64, prog: &Progress) -> Result<Option<File>, String> {
    let mut o = OpenOptions::new();
    o.write(true);
    if overwrite {
        o.create(true).truncate(true);
    } else {
        o.create_new(true);
    }
    match o.open(path) {
        Ok(f) => Ok(Some(f)),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            prog.skipped.fetch_add(1, Ordering::Relaxed);
            // keep the progress bar honest
            prog.add_bytes(size_hint);
            Ok(None)
        }
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// Ok(false) = skipped because it exists and overwrite is off
pub fn write_stream(
    path: &Path,
    src: &mut dyn Read,
    size_hint: u64,
    mtime: Option<SystemTime>,
    overwrite: bool,
    buf: &mut Vec<u8>,
    prog: &Progress,
) -> Result<bool, String> {
    let Some(mut f) = open_out(path, overwrite, size_hint, prog)? else { return Ok(false) };
    // reserve the space once: less fragmentation, fewer metadata updates
    if size_hint >= 4 << 20 {
        let _ = f.set_len(size_hint);
    }
    if buf.len() < BUF {
        buf.resize(BUF, 0);
    }
    let mut written = 0u64;
    loop {
        if prog.cancelled() {
            drop(f);
            let _ = std::fs::remove_file(path);
            return Err("cancelled".into());
        }
        let n = match src.read(buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(format!("{}: {e}", path.display())),
        };
        f.write_all(&buf[..n]).map_err(|e| format!("{}: {e}", path.display()))?;
        written += n as u64;
        prog.add_bytes(n as u64);
    }
    if written != size_hint && size_hint >= 4 << 20 {
        let _ = f.set_len(written);
    }
    if let Some(t) = mtime {
        let _ = f.set_modified(t);
    }
    prog.file_done();
    Ok(true)
}

/// small file that is already in memory
pub fn write_bytes(path: &Path, data: &[u8], mtime: Option<SystemTime>, overwrite: bool, prog: &Progress) -> Result<bool, String> {
    let Some(mut f) = open_out(path, overwrite, data.len() as u64, prog)? else { return Ok(false) };
    f.write_all(data).map_err(|e| format!("{}: {e}", path.display()))?;
    prog.add_bytes(data.len() as u64);
    if let Some(t) = mtime {
        let _ = f.set_modified(t);
    }
    prog.file_done();
    Ok(true)
}

/// zip / DOS time (local time) -> SystemTime
pub fn local_to_system(y: u16, mo: u8, d: u8, h: u8, mi: u8, s: u8) -> Option<SystemTime> {
    if y < 1980 || mo == 0 || d == 0 {
        return None;
    }
    let local = SYSTEMTIME {
        wYear: y,
        wMonth: mo as u16,
        wDayOfWeek: 0,
        wDay: d as u16,
        wHour: h as u16,
        wMinute: mi as u16,
        wSecond: s.min(59) as u16,
        wMilliseconds: 0,
    };
    let mut utc: SYSTEMTIME = unsafe { std::mem::zeroed() };
    let mut ft = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    unsafe {
        if TzSpecificLocalTimeToSystemTime(std::ptr::null(), &local, &mut utc) == 0 || SystemTimeToFileTime(&utc, &mut ft) == 0 {
            return None;
        }
    }
    filetime_to_system((ft.dwHighDateTime as u64) << 32 | ft.dwLowDateTime as u64)
}

/// 100 ns since 1601 -> SystemTime
pub fn filetime_to_system(ft: u64) -> Option<SystemTime> {
    const EPOCH_DIFF: u64 = 116_444_736_000_000_000;
    (ft >= EPOCH_DIFF).then(|| UNIX_EPOCH + Duration::from_nanos((ft - EPOCH_DIFF) * 100))
}

pub fn unix_to_system(secs: u64) -> Option<SystemTime> {
    (secs > 0).then(|| UNIX_EPOCH + Duration::from_secs(secs))
}

/// read-only memory map of a whole file (lets every thread read the archive without seeking)
pub struct Mmap {
    ptr: *const u8,
    len: usize,
    _map: Handle,
}

unsafe impl Send for Mmap {}
unsafe impl Sync for Mmap {}

impl Mmap {
    pub fn open(file: &File) -> Result<Mmap, String> {
        use std::os::windows::io::AsRawHandle;
        let len = file.metadata().map_err(|e| e.to_string())?.len() as usize;
        if len == 0 {
            return Err("empty file".into());
        }
        let map = unsafe { CreateFileMappingW(file.as_raw_handle() as _, std::ptr::null(), PAGE_READONLY, 0, 0, std::ptr::null()) };
        let map = Handle::new(map).ok_or_else(|| format!("map: {}", last_error()))?;
        let view = unsafe { MapViewOfFile(map.raw(), FILE_MAP_READ, 0, 0, 0) };
        if view.Value.is_null() {
            return Err(format!("map view: {}", last_error()));
        }
        Ok(Mmap { ptr: view.Value as *const u8, len, _map: map })
    }
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl Drop for Mmap {
    fn drop(&mut self) {
        unsafe { UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS { Value: self.ptr as *mut _ }) };
    }
}

/// a rayon pool with exactly `threads` threads
pub fn pool(threads: usize) -> rayon::ThreadPool {
    rayon::ThreadPoolBuilder::new().num_threads(threads.max(1)).build().expect("thread pool")
}

pub fn default_threads() -> usize {
    crate::topo::logical_count() as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slip() {
        let d = Path::new(r"C:\out");
        assert_eq!(safe_join(d, "a/b.txt"), Some(PathBuf::from(r"C:\out\a\b.txt")));
        assert_eq!(safe_join(d, "./a"), Some(PathBuf::from(r"C:\out\a")));
        assert_eq!(safe_join(d, "../evil"), None);
        assert_eq!(safe_join(d, "a/../../evil"), None);
        assert_eq!(safe_join(d, r"C:\Windows\x"), None);
        assert_eq!(safe_join(d, "/etc/passwd"), None);
        assert_eq!(safe_join(d, ""), None);
    }

    #[test]
    fn times() {
        let t = local_to_system(2024, 6, 1, 12, 0, 0).unwrap();
        let secs = t.duration_since(UNIX_EPOCH).unwrap().as_secs();
        // 2024-06-01 12:00 local is within a day of 1717243200 (UTC noon)
        assert!((secs as i64 - 1_717_243_200).abs() < 86_400);
        assert!(local_to_system(1970, 1, 1, 0, 0, 0).is_none());
    }
}
