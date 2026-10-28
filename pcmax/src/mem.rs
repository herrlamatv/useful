// RAM per process: soft limit (working set cap -> pages out instead of crashing), keep in RAM,
// memory priority, empty working set. The hard limit lives in job.rs (commit limit for the job).

use crate::win::{last_error, Handle};
use std::ffi::c_void;
use windows_sys::Win32::System::Memory::{
    SetProcessWorkingSetSizeEx, QUOTA_LIMITS_HARDWS_MAX_DISABLE, QUOTA_LIMITS_HARDWS_MAX_ENABLE,
    QUOTA_LIMITS_HARDWS_MIN_DISABLE,
};
use windows_sys::Win32::System::ProcessStatus::K32EmptyWorkingSet;
use windows_sys::Win32::System::Threading::{
    GetProcessInformation, ProcessMemoryPriority, SetProcessInformation, MEMORY_PRIORITY_INFORMATION,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum RamMode {
    /// commit limit: the program gets "out of memory" (may crash)
    Hard,
    /// working set cap: Windows pages out, slower but safe
    Soft,
}

impl RamMode {
    pub fn key(self) -> &'static str {
        match self {
            RamMode::Hard => "hard",
            RamMode::Soft => "soft",
        }
    }
    pub fn parse(s: &str) -> Option<RamMode> {
        match s.trim().to_ascii_lowercase().as_str() {
            "hard" | "hart" => Some(RamMode::Hard),
            "soft" | "weich" => Some(RamMode::Soft),
            _ => None,
        }
    }
}

/// at most `max` bytes in RAM, the rest goes to the page file
pub fn set_soft_limit(h: &Handle, max: u64) -> Result<(), String> {
    let min = (max / 4).clamp(1 << 20, 64 << 20) as usize;
    let ok = unsafe {
        SetProcessWorkingSetSizeEx(h.raw(), min, max.max(min as u64 + (1 << 20)) as usize, QUOTA_LIMITS_HARDWS_MAX_ENABLE | QUOTA_LIMITS_HARDWS_MIN_DISABLE)
    };
    if ok == 0 {
        Err(last_error())
    } else {
        Ok(())
    }
}

/// soft "please keep at least this much in RAM" (Windows trims it last)
pub fn set_keep(h: &Handle, keep: u64) -> Result<(), String> {
    let keep = keep.max(16 << 20) as usize;
    let ok = unsafe {
        SetProcessWorkingSetSizeEx(h.raw(), keep, keep * 2, QUOTA_LIMITS_HARDWS_MAX_DISABLE | QUOTA_LIMITS_HARDWS_MIN_DISABLE)
    };
    if ok == 0 {
        Err(last_error())
    } else {
        Ok(())
    }
}

/// back to Windows defaults (no caps)
pub fn clear_limits(h: &Handle) -> Result<(), String> {
    // (-1, -1) is not allowed with the Ex flags -> use the classic defaults (200 / 1380 pages)
    let ok = unsafe {
        SetProcessWorkingSetSizeEx(h.raw(), 200 * 4096, 1380 * 4096, QUOTA_LIMITS_HARDWS_MAX_DISABLE | QUOTA_LIMITS_HARDWS_MIN_DISABLE)
    };
    if ok == 0 {
        Err(last_error())
    } else {
        Ok(())
    }
}

/// 1 (very low) ..= 5 (normal); higher than normal doesn't exist in Windows
pub fn set_priority(h: &Handle, p: u8) -> Result<(), String> {
    let info = MEMORY_PRIORITY_INFORMATION { MemoryPriority: p.clamp(1, 5) as u32 };
    let ok = unsafe {
        SetProcessInformation(
            h.raw(),
            ProcessMemoryPriority,
            &info as *const _ as *const c_void,
            std::mem::size_of::<MEMORY_PRIORITY_INFORMATION>() as u32,
        )
    };
    if ok == 0 {
        Err(last_error())
    } else {
        Ok(())
    }
}

pub fn get_priority(h: &Handle) -> Option<u8> {
    let mut info = MEMORY_PRIORITY_INFORMATION { MemoryPriority: 0 };
    let ok = unsafe {
        GetProcessInformation(
            h.raw(),
            ProcessMemoryPriority,
            &mut info as *mut _ as *mut c_void,
            std::mem::size_of::<MEMORY_PRIORITY_INFORMATION>() as u32,
        )
    };
    (ok != 0).then_some(info.MemoryPriority as u8)
}

/// push everything to the page file / standby list (comes back on use)
pub fn empty_working_set(h: &Handle) -> Result<(), String> {
    if unsafe { K32EmptyWorkingSet(h.raw()) } == 0 {
        Err(last_error())
    } else {
        Ok(())
    }
}
