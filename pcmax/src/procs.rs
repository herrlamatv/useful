// Processes: list, tree, find by name, window -> process, CPU % / RAM / I/O per process.

use crate::win::{from_wide, open_process, Handle};
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use windows_sys::Win32::Foundation::{BOOL, FILETIME, HWND, LPARAM, POINT};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX};
use windows_sys::Win32::System::Threading::{
    GetProcessIoCounters, GetProcessTimes, QueryFullProcessImageNameW, IO_COUNTERS, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetAncestor, GetCursorPos, GetForegroundWindow, GetWindow, GetWindowTextLengthW, GetWindowTextW,
    GetWindowThreadProcessId, IsWindowVisible, WindowFromPoint, GA_ROOT, GW_OWNER,
};

#[derive(Clone, Debug)]
pub struct ProcInfo {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub threads: u32,
}

pub fn snapshot() -> Vec<ProcInfo> {
    let mut out = Vec::with_capacity(400);
    unsafe {
        let Some(snap) = Handle::new(CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)) else { return out };
        let mut e: PROCESSENTRY32W = std::mem::zeroed();
        e.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut ok = Process32FirstW(snap.raw(), &mut e);
        while ok != 0 {
            out.push(ProcInfo {
                pid: e.th32ProcessID,
                ppid: e.th32ParentProcessID,
                name: from_wide(&e.szExeFile),
                threads: e.cntThreads,
            });
            ok = Process32NextW(snap.raw(), &mut e);
        }
    }
    out
}

fn ft(f: &FILETIME) -> u64 {
    (f.dwHighDateTime as u64) << 32 | f.dwLowDateTime as u64
}

/// (creation time, kernel + user time) in 100 ns units
pub fn times(h: &Handle) -> Option<(u64, u64)> {
    let z = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let (mut c, mut e, mut k, mut u) = (z, z, z, z);
    let ok = unsafe { GetProcessTimes(h.raw(), &mut c, &mut e, &mut k, &mut u) };
    (ok != 0).then(|| (ft(&c), ft(&k) + ft(&u)))
}

pub fn creation_time(pid: u32) -> Option<u64> {
    let h = open_process(pid, PROCESS_QUERY_LIMITED_INFORMATION)?;
    times(&h).map(|t| t.0)
}

/// root + all children, grandchildren, ... (a child must be younger than its parent, PIDs get reused)
pub fn tree(all: &[ProcInfo], root: u32) -> Vec<u32> {
    let mut kids: HashMap<u32, Vec<u32>> = HashMap::new();
    for p in all {
        if p.pid != p.ppid && p.pid != 0 {
            kids.entry(p.ppid).or_default().push(p.pid);
        }
    }
    let mut out = vec![root];
    let mut seen: HashSet<u32> = HashSet::from([root]);
    let mut i = 0;
    let mut born: HashMap<u32, Option<u64>> = HashMap::new();
    let mut birth = |pid: u32| *born.entry(pid).or_insert_with(|| creation_time(pid));
    while i < out.len() {
        let parent = out[i];
        let parent_born = birth(parent);
        if let Some(children) = kids.get(&parent) {
            for &c in children {
                if seen.contains(&c) {
                    continue;
                }
                let older_than_parent = matches!((birth(c), parent_born), (Some(cb), Some(pb)) if cb < pb);
                if !older_than_parent {
                    seen.insert(c);
                    out.push(c);
                }
            }
        }
        i += 1;
    }
    out
}

fn base_name(n: &str) -> String {
    let n = n.to_ascii_lowercase();
    n.strip_suffix(".exe").map(|s| s.to_string()).unwrap_or(n)
}

/// exe name, with or without ".exe", any case; `*` as wildcard ("steam*")
pub fn name_matches(pattern: &str, name: &str) -> bool {
    let p = base_name(pattern.trim());
    let n = base_name(name);
    if !p.contains('*') {
        return p == n;
    }
    wildcard(&p, &n)
}

fn wildcard(p: &str, s: &str) -> bool {
    let parts: Vec<&str> = p.split('*').collect();
    let mut pos = 0;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 {
            if !s.starts_with(part) {
                return false;
            }
            pos = part.len();
        } else if i == parts.len() - 1 {
            return s.len() >= pos + part.len() && s[pos..].ends_with(part);
        } else {
            match s[pos..].find(part) {
                Some(f) => pos += f + part.len(),
                None => return false,
            }
        }
    }
    p.ends_with('*') || pos == s.len()
}

pub fn find_by_name(all: &[ProcInfo], name: &str) -> Vec<u32> {
    all.iter().filter(|p| name_matches(name, &p.name)).map(|p| p.pid).collect()
}

pub fn exe_path(pid: u32) -> Option<String> {
    let h = open_process(pid, PROCESS_QUERY_LIMITED_INFORMATION)?;
    let mut buf = vec![0u16; 1024];
    let mut len = buf.len() as u32;
    let ok = unsafe { QueryFullProcessImageNameW(h.raw(), PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len) };
    (ok != 0).then(|| String::from_utf16_lossy(&buf[..len as usize]))
}

/// (working set, private bytes)
pub fn memory(pid: u32) -> Option<(u64, u64)> {
    let h = open_process(pid, PROCESS_QUERY_LIMITED_INFORMATION)?;
    let mut m: PROCESS_MEMORY_COUNTERS_EX = unsafe { std::mem::zeroed() };
    m.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
    let ok = unsafe { K32GetProcessMemoryInfo(h.raw(), &mut m as *mut _ as *mut PROCESS_MEMORY_COUNTERS, m.cb) };
    (ok != 0).then_some((m.WorkingSetSize as u64, m.PrivateUsage as u64))
}

/// read + written bytes so far
pub fn io_bytes(h: &Handle) -> Option<u64> {
    let mut c: IO_COUNTERS = unsafe { std::mem::zeroed() };
    let ok = unsafe { GetProcessIoCounters(h.raw(), &mut c) };
    (ok != 0).then(|| c.ReadTransferCount + c.WriteTransferCount + c.OtherTransferCount)
}

// ---------------- windows ----------------
// HWNDs are opaque handles: Windows checks them, nothing is dereferenced here

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn window_pid(hwnd: HWND) -> u32 {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    pid
}

pub fn foreground_pid() -> u32 {
    let h = unsafe { GetForegroundWindow() };
    if h.is_null() {
        0
    } else {
        window_pid(h)
    }
}

/// top level window under the mouse -> its process
pub fn pid_at_cursor() -> u32 {
    unsafe {
        let mut pt = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut pt) == 0 {
            return 0;
        }
        let w = WindowFromPoint(pt);
        if w.is_null() {
            return 0;
        }
        let root = GetAncestor(w, GA_ROOT);
        window_pid(if root.is_null() { w } else { root })
    }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn window_title(hwnd: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return String::new();
        }
        let mut buf = vec![0u16; len as usize + 1];
        let n = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
        String::from_utf16_lossy(&buf[..n.max(0) as usize])
    }
}

/// visible top level windows with a title: pid -> first title
pub fn window_titles() -> HashMap<u32, String> {
    unsafe extern "system" fn cb(hwnd: HWND, lp: LPARAM) -> BOOL {
        let map = &mut *(lp as *mut HashMap<u32, String>);
        if IsWindowVisible(hwnd) != 0 && GetWindow(hwnd, GW_OWNER).is_null() {
            let t = window_title(hwnd);
            if !t.is_empty() {
                map.entry(window_pid(hwnd)).or_insert(t);
            }
        }
        1
    }
    let mut map: HashMap<u32, String> = HashMap::new();
    unsafe { EnumWindows(Some(cb), &mut map as *mut _ as LPARAM) };
    map
}

/// all top level windows (also hidden ones) of a process
pub fn windows_of(pid: u32) -> Vec<HWND> {
    struct Ctx {
        pid: u32,
        out: Vec<HWND>,
    }
    unsafe extern "system" fn cb(hwnd: HWND, lp: LPARAM) -> BOOL {
        let c = &mut *(lp as *mut Ctx);
        if window_pid(hwnd) == c.pid {
            c.out.push(hwnd);
        }
        1
    }
    let mut c = Ctx { pid, out: vec![] };
    unsafe { EnumWindows(Some(cb), &mut c as *mut Ctx as LPARAM) };
    c.out
}

// ---------------- CPU % per process ----------------

/// CPU usage per process in % of the whole PC (like Task Manager), call sample() every ~1 s
pub struct CpuSampler {
    last: HashMap<u32, (u64, u64)>,
    last_at: Option<Instant>,
    logical: f64,
}

impl Default for CpuSampler {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuSampler {
    pub fn new() -> Self {
        Self { last: HashMap::new(), last_at: None, logical: crate::topo::logical_count() as f64 }
    }

    pub fn sample(&mut self, pids: impl Iterator<Item = u32>) -> HashMap<u32, f32> {
        let now = Instant::now();
        let wall = self.last_at.map(|t| now.duration_since(t).as_secs_f64()).unwrap_or(0.0);
        let mut next = HashMap::new();
        let mut out = HashMap::new();
        for pid in pids {
            let Some(h) = open_process(pid, PROCESS_QUERY_LIMITED_INFORMATION) else { continue };
            let Some((born, cpu)) = times(&h) else { continue };
            if let Some(&(b0, c0)) = self.last.get(&pid) {
                if b0 == born && wall > 0.0 && cpu >= c0 {
                    let pct = (cpu - c0) as f64 / 1e7 / wall / self.logical * 100.0;
                    out.insert(pid, pct.min(100.0) as f32);
                }
            }
            next.insert(pid, (born, cpu));
        }
        self.last = next;
        self.last_at = Some(now);
        out
    }
}

/// CPU time of a process tree in seconds (for measuring limits)
pub fn cpu_seconds(pids: &[u32]) -> f64 {
    pids.iter()
        .filter_map(|&p| open_process(p, PROCESS_QUERY_LIMITED_INFORMATION))
        .filter_map(|h| times(&h))
        .map(|(_, c)| c as f64 / 1e7)
        .sum()
}

pub fn own_pid() -> u32 {
    std::process::id()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert!(name_matches("7z", "7z.exe"));
        assert!(name_matches("WinRAR.exe", "winrar.EXE"));
        assert!(!name_matches("7z", "7zG.exe"));
        assert!(name_matches("7z*", "7zG.exe"));
        assert!(name_matches("*helper*", "steamwebhelper.exe"));
        assert!(name_matches("steam*", "steamwebhelper.exe"));
        assert!(!name_matches("*helper", "steamwebhelper2.exe"));
    }

    #[test]
    fn own_process_is_listed() {
        let all = snapshot();
        assert!(all.iter().any(|p| p.pid == own_pid()));
        let t = tree(&all, own_pid());
        assert_eq!(t[0], own_pid());
    }
}
