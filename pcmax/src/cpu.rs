// Per process CPU knobs: priority class, cores (affinity), Efficiency Mode (EcoQoS).

use crate::win::{last_error, Handle};
use std::ffi::c_void;
use windows_sys::Win32::System::Threading::{
    GetPriorityClass, GetProcessAffinityMask, GetProcessInformation, ProcessPowerThrottling, SetPriorityClass,
    SetProcessAffinityMask, SetProcessInformation, ABOVE_NORMAL_PRIORITY_CLASS, BELOW_NORMAL_PRIORITY_CLASS,
    HIGH_PRIORITY_CLASS, IDLE_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS, PROCESS_POWER_THROTTLING_CURRENT_VERSION,
    PROCESS_POWER_THROTTLING_EXECUTION_SPEED, PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION,
    PROCESS_POWER_THROTTLING_STATE, REALTIME_PRIORITY_CLASS,
};

/// also used for the GPU priority (same six steps)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Prio {
    Idle,
    BelowNormal,
    Normal,
    AboveNormal,
    High,
    Realtime,
}

impl Prio {
    pub const ALL: [Prio; 6] = [Prio::Idle, Prio::BelowNormal, Prio::Normal, Prio::AboveNormal, Prio::High, Prio::Realtime];

    pub fn key(self) -> &'static str {
        match self {
            Prio::Idle => "idle",
            Prio::BelowNormal => "below",
            Prio::Normal => "normal",
            Prio::AboveNormal => "above",
            Prio::High => "high",
            Prio::Realtime => "realtime",
        }
    }

    pub fn parse(s: &str) -> Option<Prio> {
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "idle" | "low" | "niedrig" => Prio::Idle,
            "below" | "belownormal" | "below-normal" => Prio::BelowNormal,
            "normal" => Prio::Normal,
            "above" | "abovenormal" | "above-normal" => Prio::AboveNormal,
            "high" | "hoch" => Prio::High,
            "realtime" | "echtzeit" => Prio::Realtime,
            _ => return None,
        })
    }

    pub fn class(self) -> u32 {
        match self {
            Prio::Idle => IDLE_PRIORITY_CLASS,
            Prio::BelowNormal => BELOW_NORMAL_PRIORITY_CLASS,
            Prio::Normal => NORMAL_PRIORITY_CLASS,
            Prio::AboveNormal => ABOVE_NORMAL_PRIORITY_CLASS,
            Prio::High => HIGH_PRIORITY_CLASS,
            Prio::Realtime => REALTIME_PRIORITY_CLASS,
        }
    }

    pub fn from_class(c: u32) -> Option<Prio> {
        Prio::ALL.into_iter().find(|p| p.class() == c)
    }

    pub fn index(self) -> usize {
        Prio::ALL.iter().position(|&p| p == self).unwrap_or(2)
    }
}

pub fn set_priority(h: &Handle, p: Prio) -> Result<(), String> {
    if unsafe { SetPriorityClass(h.raw(), p.class()) } == 0 {
        return Err(last_error());
    }
    // without the admin privilege Windows quietly makes "realtime" "high"
    if p == Prio::Realtime && get_priority(h) != Some(Prio::Realtime) {
        return Err("realtime needs admin rights - got 'high'".into());
    }
    Ok(())
}

pub fn get_priority(h: &Handle) -> Option<Prio> {
    Prio::from_class(unsafe { GetPriorityClass(h.raw()) })
}

pub fn set_affinity(h: &Handle, mask: u64) -> Result<(), String> {
    if unsafe { SetProcessAffinityMask(h.raw(), mask as usize) } == 0 {
        Err(last_error())
    } else {
        Ok(())
    }
}

pub fn get_affinity(h: &Handle) -> Option<u64> {
    let (mut p, mut s) = (0usize, 0usize);
    (unsafe { GetProcessAffinityMask(h.raw(), &mut p, &mut s) } != 0).then_some(p as u64)
}

/// Efficiency Mode / EcoQoS: Some(true) = force on, Some(false) = force off (no throttling), None = Windows decides
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Eco {
    Auto,
    Off,
    On,
}

impl Eco {
    pub fn key(self) -> &'static str {
        match self {
            Eco::Auto => "auto",
            Eco::Off => "off",
            Eco::On => "on",
        }
    }
    pub fn parse(s: &str) -> Option<Eco> {
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "auto" | "default" => Eco::Auto,
            "off" | "aus" | "0" => Eco::Off,
            "on" | "an" | "1" => Eco::On,
            _ => return None,
        })
    }
}

fn set_throttle(h: &Handle, control: u32, state: u32) -> Result<(), String> {
    let s = PROCESS_POWER_THROTTLING_STATE { Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION, ControlMask: control, StateMask: state };
    let ok = unsafe {
        SetProcessInformation(
            h.raw(),
            ProcessPowerThrottling,
            &s as *const _ as *const c_void,
            std::mem::size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32,
        )
    };
    if ok == 0 {
        Err(last_error())
    } else {
        Ok(())
    }
}

pub fn set_eco(h: &Handle, eco: Eco) -> Result<(), String> {
    let speed = PROCESS_POWER_THROTTLING_EXECUTION_SPEED;
    match eco {
        Eco::Auto => set_throttle(h, 0, 0),
        Eco::Off => set_throttle(h, speed, 0),
        Eco::On => set_throttle(h, speed, speed),
    }
}

/// a hidden / minimized process may still ask for a fine timer (Windows 11 ignores it by default)
pub fn keep_timer_resolution(h: &Handle) -> Result<(), String> {
    set_throttle(h, PROCESS_POWER_THROTTLING_EXECUTION_SPEED | PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION, 0)
}

pub fn get_eco(h: &Handle) -> Option<Eco> {
    let mut s = PROCESS_POWER_THROTTLING_STATE { Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION, ControlMask: 0, StateMask: 0 };
    let ok = unsafe {
        GetProcessInformation(
            h.raw(),
            ProcessPowerThrottling,
            &mut s as *mut _ as *mut c_void,
            std::mem::size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32,
        )
    };
    if ok == 0 {
        return None;
    }
    let speed = PROCESS_POWER_THROTTLING_EXECUTION_SPEED;
    Some(if s.ControlMask & speed == 0 {
        Eco::Auto
    } else if s.StateMask & speed != 0 {
        Eco::On
    } else {
        Eco::Off
    })
}
