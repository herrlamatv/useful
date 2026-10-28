// Disk I/O priority per process (undocumented but stable since Vista: NtSetInformationProcess class 33).

use crate::win::Handle;
use std::ffi::c_void;
use windows_sys::Win32::Foundation::{HANDLE, NTSTATUS};

const PROCESS_IO_PRIORITY: u32 = 33;

#[link(name = "ntdll")]
extern "system" {
    fn NtSetInformationProcess(process: HANDLE, class: u32, info: *const c_void, len: u32) -> NTSTATUS;
    fn NtQueryInformationProcess(process: HANDLE, class: u32, info: *mut c_void, len: u32, ret: *mut u32) -> NTSTATUS;
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum IoPrio {
    VeryLow,
    Low,
    Normal,
    /// needs admin
    High,
}

impl IoPrio {
    pub const ALL: [IoPrio; 4] = [IoPrio::VeryLow, IoPrio::Low, IoPrio::Normal, IoPrio::High];

    pub fn key(self) -> &'static str {
        match self {
            IoPrio::VeryLow => "verylow",
            IoPrio::Low => "low",
            IoPrio::Normal => "normal",
            IoPrio::High => "high",
        }
    }
    pub fn parse(s: &str) -> Option<IoPrio> {
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "verylow" | "very-low" | "background" => IoPrio::VeryLow,
            "low" | "niedrig" => IoPrio::Low,
            "normal" => IoPrio::Normal,
            "high" | "hoch" => IoPrio::High,
            _ => return None,
        })
    }
    pub fn index(self) -> usize {
        self as usize
    }
}

fn status_text(s: NTSTATUS) -> String {
    match s as u32 {
        0xC000_0061 => "needs admin rights (privilege not held)".into(),
        0xC000_0022 => "access denied".into(),
        x => format!("NTSTATUS 0x{x:08X}"),
    }
}

pub fn set(h: &Handle, p: IoPrio) -> Result<(), String> {
    if p == IoPrio::High {
        crate::win::enable_privilege("SeIncreaseBasePriorityPrivilege");
    }
    let v = p as u32;
    let s = unsafe { NtSetInformationProcess(h.raw(), PROCESS_IO_PRIORITY, &v as *const u32 as *const c_void, 4) };
    if s < 0 {
        Err(status_text(s))
    } else {
        Ok(())
    }
}

pub fn get(h: &Handle) -> Option<IoPrio> {
    let mut v = 0u32;
    let s = unsafe { NtQueryInformationProcess(h.raw(), PROCESS_IO_PRIORITY, &mut v as *mut u32 as *mut c_void, 4, std::ptr::null_mut()) };
    if s < 0 {
        return None;
    }
    IoPrio::ALL.get(v as usize).copied()
}
