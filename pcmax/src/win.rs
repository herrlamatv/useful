// Small Win32 helpers: wide strings, owned handles, admin check / restart, privileges.

use std::ffi::c_void;
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE, LUID};
use windows_sys::Win32::Security::{
    AdjustTokenPrivileges, GetTokenInformation, LookupPrivilegeValueW, TokenElevation, LUID_AND_ATTRIBUTES,
    SE_PRIVILEGE_ENABLED, TOKEN_ADJUST_PRIVILEGES, TOKEN_ELEVATION, TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcess, OpenProcessToken};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

/// no console window for helper programs (powershell, schtasks, ...)
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

pub fn from_wide(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

/// text of the last Win32 error
pub fn last_error() -> String {
    std::io::Error::last_os_error().to_string()
}

pub fn last_error_code() -> u32 {
    unsafe { GetLastError() }
}

/// HANDLE that is closed on drop
pub struct Handle(pub HANDLE);

// kernel handles can be used from any thread
unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}

impl Handle {
    pub fn new(h: HANDLE) -> Option<Handle> {
        if h.is_null() || h == INVALID_HANDLE_VALUE {
            None
        } else {
            Some(Handle(h))
        }
    }
    pub fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

pub fn open_process(pid: u32, access: u32) -> Option<Handle> {
    Handle::new(unsafe { OpenProcess(access, 0, pid) })
}

/// runs pcmax with admin rights right now?
pub fn is_elevated() -> bool {
    unsafe {
        let mut token: HANDLE = null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let token = Handle(token);
        let mut e = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut len = 0u32;
        let ok = GetTokenInformation(
            token.raw(),
            TokenElevation,
            &mut e as *mut _ as *mut c_void,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut len,
        );
        ok != 0 && e.TokenIsElevated != 0
    }
}

/// turn on a privilege the token already has (only admins have the interesting ones)
pub fn enable_privilege(name: &str) -> bool {
    unsafe {
        let mut token: HANDLE = null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let token = Handle(token);
        let mut luid = LUID { LowPart: 0, HighPart: 0 };
        if LookupPrivilegeValueW(null(), wide(name).as_ptr(), &mut luid) == 0 {
            return false;
        }
        let tp = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES { Luid: luid, Attributes: SE_PRIVILEGE_ENABLED }],
        };
        AdjustTokenPrivileges(token.raw(), 0, &tp, 0, null_mut(), null_mut());
        // ERROR_NOT_ALL_ASSIGNED (1300) = the token doesn't have it
        GetLastError() == 0
    }
}

/// enable everything that helps setting priorities (works only as admin, harmless otherwise)
pub fn enable_admin_privileges() {
    for p in ["SeIncreaseBasePriorityPrivilege", "SeDebugPrivilege", "SeIncreaseQuotaPrivilege", "SeProfileSingleProcessPrivilege"] {
        enable_privilege(p);
    }
}

/// start this exe again with UAC ("runas"), returns false if the user said no
pub fn relaunch_as_admin(args: &str) -> bool {
    let Ok(exe) = std::env::current_exe() else { return false };
    let r = unsafe {
        ShellExecuteW(
            null_mut(),
            wide("runas").as_ptr(),
            wide(&exe.to_string_lossy()).as_ptr(),
            wide(args).as_ptr(),
            null(),
            SW_SHOWNORMAL,
        )
    };
    r as isize > 32
}

/// Windows display language is German?
pub fn ui_is_german() -> bool {
    let lang = unsafe { windows_sys::Win32::Globalization::GetUserDefaultUILanguage() };
    lang & 0x3ff == 0x07
}

/// run a helper program hidden and return (success, stdout + stderr)
pub fn run_hidden(exe: &str, args: &[&str]) -> (bool, String) {
    match Command::new(exe).args(args).creation_flags(CREATE_NO_WINDOW).output() {
        Ok(o) => {
            let mut s = String::from_utf8_lossy(&o.stdout).into_owned();
            s.push_str(&String::from_utf8_lossy(&o.stderr));
            (o.status.success(), s.trim().to_string())
        }
        Err(e) => (false, e.to_string()),
    }
}

/// Windows PowerShell 5.1 (always there, has the Defender cmdlets)
pub fn powershell(script: &str) -> (bool, String) {
    run_hidden("powershell.exe", &["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script])
}

/// 'text' for PowerShell single quotes
pub fn ps_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

pub fn fmt_bytes(b: u64) -> String {
    const U: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = b as f64;
    let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{b} B")
    } else if v >= 100.0 {
        format!("{v:.0} {}", U[i])
    } else {
        format!("{v:.1} {}", U[i])
    }
}

/// "4G", "512M", "2048" (MB), "1.5g" -> bytes
pub fn parse_size(s: &str) -> Option<u64> {
    let s = s.trim().to_ascii_lowercase();
    let s = s.trim_end_matches('b');
    let (num, mul) = match s.chars().last()? {
        'k' => (&s[..s.len() - 1], 1u64 << 10),
        'm' => (&s[..s.len() - 1], 1 << 20),
        'g' => (&s[..s.len() - 1], 1 << 30),
        't' => (&s[..s.len() - 1], 1 << 40),
        _ => (s, 1 << 20),
    };
    let v: f64 = num.trim().replace(',', ".").parse().ok()?;
    (v > 0.0).then_some((v * mul as f64) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(parse_size("4G"), Some(4 << 30));
        assert_eq!(parse_size("512m"), Some(512 << 20));
        assert_eq!(parse_size("2048"), Some(2048 << 20));
        assert_eq!(parse_size("1,5gb"), Some(3 << 29));
        assert_eq!(parse_size("x"), None);
        assert_eq!(fmt_bytes(1536), "1.5 KB");
    }
}
