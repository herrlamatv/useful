// Small Win32 helpers: drives, known folders, opening files, local time, UI language.

use std::os::windows::process::CommandExt;
use std::process::Command;
use std::ptr::{null, null_mut};
use std::sync::OnceLock;
use windows_sys::Win32::Foundation::{FILETIME, SYSTEMTIME};
use windows_sys::Win32::Storage::FileSystem::{GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW};
use windows_sys::Win32::System::Diagnostics::Debug::{SetErrorMode, SEM_FAILCRITICALERRORS};
use windows_sys::Win32::System::Time::{
    FileTimeToSystemTime, SystemTimeToFileTime, SystemTimeToTzSpecificLocalTime, TzSpecificLocalTimeToSystemTime,
};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn from_wide(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DriveKind {
    Local,
    Removable,
    Network,
    Cd,
    Ram,
}

#[derive(Clone, Debug)]
pub struct Drive {
    /// "C:\"
    pub root: String,
    pub label: String,
    pub kind: DriveKind,
    pub total: u64,
    pub free: u64,
}

/// ready drives only (empty card readers / DVD drives are left out, without "no disk" popups)
pub fn drives() -> Vec<Drive> {
    let mut out = Vec::new();
    unsafe {
        SetErrorMode(SEM_FAILCRITICALERRORS);
        let mask = GetLogicalDrives();
        for i in 0..26 {
            if mask & (1 << i) == 0 {
                continue;
            }
            let root = format!("{}:\\", (b'A' + i as u8) as char);
            let w = wide(&root);
            let kind = match GetDriveTypeW(w.as_ptr()) {
                2 => DriveKind::Removable,
                3 => DriveKind::Local,
                4 => DriveKind::Network,
                5 => DriveKind::Cd,
                6 => DriveKind::Ram,
                _ => continue,
            };
            let mut label = [0u16; 261];
            let ok = GetVolumeInformationW(
                w.as_ptr(),
                label.as_mut_ptr(),
                label.len() as u32,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                0,
            );
            if ok == 0 {
                continue; // not ready
            }
            let (mut avail, mut total, mut free) = (0u64, 0u64, 0u64);
            GetDiskFreeSpaceExW(w.as_ptr(), &mut avail, &mut total, &mut free);
            out.push(Drive { root, label: from_wide(&label), kind, total, free: avail });
        }
    }
    out
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Known {
    Desktop,
    Documents,
    Downloads,
    Pictures,
    Videos,
    Music,
    Home,
    OneDrive,
}

impl Known {
    pub const ALL: [Known; 8] = [
        Known::Desktop,
        Known::Documents,
        Known::Downloads,
        Known::Pictures,
        Known::Videos,
        Known::Music,
        Known::Home,
        Known::OneDrive,
    ];
}

/// real path of a known folder (Desktop can be redirected to OneDrive etc.)
pub fn known_folder(k: Known) -> Option<String> {
    use windows_sys::Win32::UI::Shell::*;
    let id = match k {
        Known::Desktop => FOLDERID_Desktop,
        Known::Documents => FOLDERID_Documents,
        Known::Downloads => FOLDERID_Downloads,
        Known::Pictures => FOLDERID_Pictures,
        Known::Videos => FOLDERID_Videos,
        Known::Music => FOLDERID_Music,
        Known::Home => FOLDERID_Profile,
        Known::OneDrive => {
            return std::env::var("OneDrive").ok().filter(|p| std::path::Path::new(p).is_dir());
        }
    };
    unsafe {
        let mut p = null_mut();
        let hr = SHGetKnownFolderPath(&id, 0, null_mut(), &mut p);
        let out = if hr >= 0 && !p.is_null() {
            let mut len = 0;
            while *p.add(len) != 0 {
                len += 1;
            }
            Some(String::from_utf16_lossy(std::slice::from_raw_parts(p, len)))
        } else {
            None
        };
        if !p.is_null() {
            windows_sys::Win32::System::Com::CoTaskMemFree(p as _);
        }
        out.filter(|p| std::path::Path::new(p).is_dir())
    }
}

/// opens a file / folder with its default program
pub fn open(path: &str) -> bool {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let (op, p) = (wide("open"), wide(path));
    let r = unsafe { ShellExecuteW(null_mut(), op.as_ptr(), p.as_ptr(), null(), null(), SW_SHOWNORMAL) };
    r as isize > 32
}

/// Explorer with the file selected. raw_arg because explorer can't handle the usual quoting.
pub fn show_in_explorer(path: &str) {
    let _ = Command::new("explorer.exe").raw_arg(format!("/select,\"{path}\"")).spawn();
}

/// German Windows -> German UI (for "Auto")
pub fn ui_is_german() -> bool {
    let lang = unsafe { windows_sys::Win32::Globalization::GetUserDefaultUILanguage() };
    lang & 0x3ff == 0x07
}

// ---------------- local time ----------------

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DateTime {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl DateTime {
    /// 2019-05-03 14:22 (same in every language, like filescanner)
    pub fn ymd_hm(&self) -> String {
        format!("{:04}-{:02}-{:02} {:02}:{:02}", self.year, self.month, self.day, self.hour, self.minute)
    }
    pub fn ymd(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
    pub fn ymd_hms(&self) -> String {
        format!("{} {:02}:{:02}:{:02}", self.ymd(), self.hour, self.minute, self.second)
    }
    /// for file names: 2026-10-10_19-10-19
    pub fn stamp(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}_{:02}-{:02}-{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

fn unix_to_filetime(secs: i64) -> FILETIME {
    let ft = ((secs + 11_644_473_600).max(0) as u64) * 10_000_000;
    FILETIME { dwLowDateTime: ft as u32, dwHighDateTime: (ft >> 32) as u32 }
}

/// unix seconds -> local date/time (with the daylight saving time of that date)
pub fn local(secs: i64) -> Option<DateTime> {
    if secs <= 0 {
        return None;
    }
    unsafe {
        let ft = unix_to_filetime(secs);
        let mut utc: SYSTEMTIME = std::mem::zeroed();
        let mut loc: SYSTEMTIME = std::mem::zeroed();
        if FileTimeToSystemTime(&ft, &mut utc) == 0 || SystemTimeToTzSpecificLocalTime(null(), &utc, &mut loc) == 0 {
            return None;
        }
        Some(DateTime {
            year: loc.wYear as i32,
            month: loc.wMonth as u32,
            day: loc.wDay as u32,
            hour: loc.wHour as u32,
            minute: loc.wMinute as u32,
            second: loc.wSecond as u32,
        })
    }
}

/// unix seconds of January 1st 00:00 local time
fn local_year_start(year: i32) -> i64 {
    unsafe {
        let loc = SYSTEMTIME { wYear: year as u16, wMonth: 1, wDayOfWeek: 0, wDay: 1, wHour: 0, wMinute: 0, wSecond: 0, wMilliseconds: 0 };
        let mut utc: SYSTEMTIME = std::mem::zeroed();
        let mut ft: FILETIME = std::mem::zeroed();
        if TzSpecificLocalTimeToSystemTime(null(), &loc, &mut utc) == 0 || SystemTimeToFileTime(&utc, &mut ft) == 0 {
            // fallback: UTC
            let days = (year as i64 - 1970) * 365 + ((year as i64 - 1969) / 4);
            return days * 86_400;
        }
        let v = ((ft.dwHighDateTime as u64) << 32) | ft.dwLowDateTime as u64;
        (v / 10_000_000) as i64 - 11_644_473_600
    }
}

const FIRST_YEAR: i32 = 1970;
const LAST_YEAR: i32 = 2100;

fn year_starts() -> &'static [i64] {
    static T: OnceLock<Vec<i64>> = OnceLock::new();
    T.get_or_init(|| (FIRST_YEAR..=LAST_YEAR + 1).map(local_year_start).collect())
}

/// local year of a timestamp, None if unknown
pub fn year_of(secs: i64) -> Option<i32> {
    if secs <= 0 {
        return None;
    }
    let t = year_starts();
    let i = t.partition_point(|&s| s <= secs);
    if i == 0 {
        return Some(FIRST_YEAR);
    }
    Some((FIRST_YEAR + i as i32 - 1).min(LAST_YEAR))
}

pub fn year_start(year: i32) -> i64 {
    let t = year_starts();
    let i = (year.clamp(FIRST_YEAR, LAST_YEAR + 1) - FIRST_YEAR) as usize;
    t[i]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn years() {
        // 2019-07-01 12:00 UTC
        assert_eq!(year_of(1_561_982_400), Some(2019));
        assert_eq!(year_of(0), None);
        assert!(year_start(2020) > year_start(2019));
        assert_eq!(year_of(year_start(2020)), Some(2020));
        assert_eq!(year_of(year_start(2020) - 1), Some(2019));
        let d = local(1_561_982_400).unwrap();
        assert_eq!((d.year, d.month, d.day), (2019, 7, 1));
    }
}
