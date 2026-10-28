// GPU: scheduling priority per process (D3DKMT, like Windows does it for the foreground app)
// and which GPU an app uses (same registry value as Settings > Display > Graphics).
// A GPU limit in % doesn't exist in Windows - only priority and the choice of the GPU.

use crate::cpu::Prio;
use crate::reg::{self, Root, Value};
use crate::win::Handle;
use windows_sys::Wdk::Graphics::Direct3D::{D3DKMTGetProcessSchedulingPriorityClass, D3DKMTSetProcessSchedulingPriorityClass};

const PREF_KEY: &str = r"Software\Microsoft\DirectX\UserGpuPreferences";

fn class(p: Prio) -> i32 {
    // D3DKMT_SCHEDULINGPRIORITYCLASS_IDLE = 0 ... REALTIME = 5, same order as Prio
    p as i32
}

pub fn set_priority(h: &Handle, p: Prio) -> Result<(), String> {
    if p == Prio::Realtime {
        crate::win::enable_privilege("SeIncreaseBasePriorityPrivilege");
    }
    let s = unsafe { D3DKMTSetProcessSchedulingPriorityClass(h.raw(), class(p)) };
    if s < 0 {
        Err(match s as u32 {
            0xC000_0061 | 0xC000_0022 => "needs admin rights".into(),
            x => format!("NTSTATUS 0x{x:08X}"),
        })
    } else {
        Ok(())
    }
}

pub fn get_priority(h: &Handle) -> Option<Prio> {
    let mut c = 0i32;
    let s = unsafe { D3DKMTGetProcessSchedulingPriorityClass(h.raw(), &mut c) };
    (s >= 0).then(|| Prio::ALL.get(c as usize).copied()).flatten()
}

/// Windows "graphics performance preference" per app
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum GpuPref {
    /// Windows decides
    Auto,
    /// integrated GPU (Radeon iGPU)
    Saving,
    /// dedicated GPU (RTX)
    High,
}

impl GpuPref {
    pub fn key(self) -> &'static str {
        match self {
            GpuPref::Auto => "auto",
            GpuPref::Saving => "saving",
            GpuPref::High => "high",
        }
    }
    pub fn parse(s: &str) -> Option<GpuPref> {
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "auto" | "0" => GpuPref::Auto,
            "saving" | "igpu" | "power" | "sparen" | "1" => GpuPref::Saving,
            "high" | "dgpu" | "rtx" | "nvidia" | "performance" | "2" => GpuPref::High,
            _ => return None,
        })
    }
    fn number(self) -> u8 {
        match self {
            GpuPref::Auto => 0,
            GpuPref::Saving => 1,
            GpuPref::High => 2,
        }
    }
}

/// "SwapEffectUpgradeEnable=1;GpuPreference=2;" with GpuPreference replaced / removed
fn merge(old: &str, pref: GpuPref) -> String {
    let mut parts: Vec<String> =
        old.split(';').map(str::trim).filter(|p| !p.is_empty() && !p.starts_with("GpuPreference=")).map(String::from).collect();
    if pref != GpuPref::Auto {
        parts.push(format!("GpuPreference={}", pref.number()));
    }
    if parts.is_empty() {
        String::new()
    } else {
        parts.join(";") + ";"
    }
}

pub fn get_pref_raw(exe: &str) -> Option<String> {
    reg::get_sz(Root::Hkcu, PREF_KEY, exe)
}

pub fn get_pref(exe: &str) -> GpuPref {
    let raw = get_pref_raw(exe).unwrap_or_default();
    for p in raw.split(';') {
        if let Some(n) = p.trim().strip_prefix("GpuPreference=") {
            return GpuPref::parse(n).unwrap_or(GpuPref::Auto);
        }
    }
    GpuPref::Auto
}

/// takes effect the next time the app starts
pub fn set_pref(exe: &str, pref: GpuPref) -> Result<(), String> {
    let old = get_pref_raw(exe).unwrap_or_default();
    let new = merge(&old, pref);
    if new.is_empty() {
        reg::delete_value(Root::Hkcu, PREF_KEY, exe)
    } else {
        reg::set(Root::Hkcu, PREF_KEY, exe, &Value::Sz(new))
    }
}

/// put back what was there before (undo journal)
pub fn restore_pref(exe: &str, old: Option<&str>) -> Result<(), String> {
    match old {
        Some(v) if !v.is_empty() => reg::set(Root::Hkcu, PREF_KEY, exe, &Value::Sz(v.to_string())),
        _ => reg::delete_value(Root::Hkcu, PREF_KEY, exe),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merging() {
        assert_eq!(merge("", GpuPref::High), "GpuPreference=2;");
        assert_eq!(merge("SwapEffectUpgradeEnable=1;GpuPreference=1;", GpuPref::High), "SwapEffectUpgradeEnable=1;GpuPreference=2;");
        assert_eq!(merge("GpuPreference=2;", GpuPref::Auto), "");
        assert_eq!(merge("SwapEffectUpgradeEnable=1;", GpuPref::Auto), "SwapEffectUpgradeEnable=1;");
    }
}
