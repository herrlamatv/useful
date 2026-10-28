// Pause / resume Windows services (needs admin): Windows Search, SysMain, Delivery Optimization.

use crate::win::{last_error, wide};
use std::ptr::null;
use std::time::{Duration, Instant};
use windows_sys::Win32::System::Services::{
    CloseServiceHandle, ControlService, OpenSCManagerW, OpenServiceW, QueryServiceStatus, StartServiceW,
    SC_HANDLE, SC_MANAGER_CONNECT, SERVICE_CONTROL_STOP, SERVICE_QUERY_STATUS, SERVICE_RUNNING, SERVICE_START,
    SERVICE_STATUS, SERVICE_STOP, SERVICE_STOPPED,
};

/// (service name, what it is)
pub const PAUSABLE: [(&str, &str); 3] = [
    ("WSearch", "Windows Search (indexer)"),
    ("SysMain", "SysMain (Superfetch)"),
    ("DoSvc", "Delivery Optimization (update downloads)"),
];

struct Svc(SC_HANDLE);

impl Drop for Svc {
    fn drop(&mut self) {
        unsafe { CloseServiceHandle(self.0) };
    }
}

fn open(name: &str, access: u32) -> Result<(Svc, Svc), String> {
    unsafe {
        let scm = OpenSCManagerW(null(), null(), SC_MANAGER_CONNECT);
        if scm.is_null() {
            return Err(last_error());
        }
        let scm = Svc(scm);
        let s = OpenServiceW(scm.0, wide(name).as_ptr(), access);
        if s.is_null() {
            return Err(format!("{name}: {}", last_error()));
        }
        Ok((scm, Svc(s)))
    }
}

fn state(s: &Svc) -> Option<u32> {
    let mut st: SERVICE_STATUS = unsafe { std::mem::zeroed() };
    (unsafe { QueryServiceStatus(s.0, &mut st) } != 0).then_some(st.dwCurrentState)
}

/// works without admin (only asks for the status)
pub fn is_running(name: &str) -> bool {
    open(name, SERVICE_QUERY_STATUS).ok().and_then(|(_, s)| state(&s)) == Some(SERVICE_RUNNING)
}

/// Ok(true) = was running and is stopped now, Ok(false) = wasn't running
pub fn stop(name: &str) -> Result<bool, String> {
    let (_scm, s) = open(name, SERVICE_STOP | SERVICE_QUERY_STATUS)?;
    if state(&s) != Some(SERVICE_RUNNING) {
        return Ok(false);
    }
    let mut st: SERVICE_STATUS = unsafe { std::mem::zeroed() };
    if unsafe { ControlService(s.0, SERVICE_CONTROL_STOP, &mut st) } == 0 {
        return Err(format!("{name}: {}", last_error()));
    }
    let t = Instant::now();
    while state(&s) != Some(SERVICE_STOPPED) && t.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok(true)
}

pub fn start(name: &str) -> Result<(), String> {
    let (_scm, s) = open(name, SERVICE_START | SERVICE_QUERY_STATUS)?;
    if state(&s) == Some(SERVICE_RUNNING) {
        return Ok(());
    }
    if unsafe { StartServiceW(s.0, 0, null()) } == 0 {
        // 1056 = already running
        if crate::win::last_error_code() != 1056 {
            return Err(format!("{name}: {}", last_error()));
        }
    }
    Ok(())
}
