// Power plans (switch + remember), core parking, timer resolution.

use std::ptr::null_mut;
use windows_sys::core::GUID;
use windows_sys::Win32::Foundation::{LocalFree, HLOCAL};
use windows_sys::Win32::Media::{timeBeginPeriod, timeEndPeriod};
use windows_sys::Win32::System::Power::{
    PowerEnumerate, PowerGetActiveScheme, PowerReadACValueIndex, PowerReadDCValueIndex, PowerReadFriendlyName,
    PowerSetActiveScheme, PowerWriteACValueIndex, PowerWriteDCValueIndex, ACCESS_SCHEME,
};

pub const HIGH_PERFORMANCE: &str = "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c";
pub const BALANCED: &str = "381b4222-f694-41f0-9685-ff5bb260df2e";
const SUB_PROCESSOR: GUID = GUID::from_u128(0x54533251_82be_4824_96c1_47b60b740d00);
/// "processor performance core parking min cores" (100 = no parking)
const CP_MIN_CORES: GUID = GUID::from_u128(0x0cc5b647_c1df_4637_891a_dec35c318583);

pub fn guid_str(g: &GUID) -> String {
    format!(
        "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        g.data1, g.data2, g.data3, g.data4[0], g.data4[1], g.data4[2], g.data4[3], g.data4[4], g.data4[5], g.data4[6], g.data4[7]
    )
}

pub fn parse_guid(s: &str) -> Option<GUID> {
    let hex: String = s.trim().trim_matches(['{', '}']).chars().filter(|c| *c != '-').collect();
    (hex.len() == 32).then(|| u128::from_str_radix(&hex, 16).ok().map(GUID::from_u128)).flatten()
}

fn err(code: u32) -> String {
    std::io::Error::from_raw_os_error(code as i32).to_string()
}

pub fn active() -> Option<String> {
    let mut p: *mut GUID = null_mut();
    unsafe {
        if PowerGetActiveScheme(null_mut(), &mut p) != 0 || p.is_null() {
            return None;
        }
        let s = guid_str(&*p);
        LocalFree(p as HLOCAL);
        Some(s)
    }
}

pub fn set_active(guid: &str) -> Result<(), String> {
    let g = parse_guid(guid).ok_or("bad power plan id")?;
    match unsafe { PowerSetActiveScheme(null_mut(), &g) } {
        0 => Ok(()),
        e => Err(err(e)),
    }
}

/// (id, name) of all power plans
pub fn list() -> Vec<(String, String)> {
    let mut out = vec![];
    for i in 0..64 {
        let mut g: GUID = GUID::from_u128(0);
        let mut size = std::mem::size_of::<GUID>() as u32;
        let r = unsafe { PowerEnumerate(null_mut(), std::ptr::null(), std::ptr::null(), ACCESS_SCHEME, i, &mut g as *mut GUID as *mut u8, &mut size) };
        if r != 0 {
            break;
        }
        out.push((guid_str(&g), name(&g)));
    }
    out
}

fn name(g: &GUID) -> String {
    let mut size = 0u32;
    unsafe {
        PowerReadFriendlyName(null_mut(), g, std::ptr::null(), std::ptr::null(), null_mut(), &mut size);
        let mut buf = vec![0u16; size as usize / 2 + 1];
        size = (buf.len() * 2) as u32;
        if PowerReadFriendlyName(null_mut(), g, std::ptr::null(), std::ptr::null(), buf.as_mut_ptr() as *mut u8, &mut size) != 0 {
            return String::new();
        }
        crate::win::from_wide(&buf)
    }
}

pub fn name_of(guid: &str) -> String {
    parse_guid(guid).map(|g| name(&g)).unwrap_or_default()
}

/// the fastest plan there is: "Ultimate Performance" (any language), else "High performance"
pub fn fastest() -> Option<(String, String)> {
    let all = list();
    all.iter()
        .find(|(_, n)| {
            let n = n.to_lowercase();
            n.contains("ultimat") || n.contains("optimales")
        })
        .or_else(|| all.iter().find(|(g, _)| g == HIGH_PERFORMANCE))
        .cloned()
}

/// (AC, DC) "min cores unparked" of a plan in %
pub fn parking(guid: &str) -> Option<(u32, u32)> {
    let g = parse_guid(guid)?;
    let (mut ac, mut dc) = (0u32, 0u32);
    unsafe {
        if PowerReadACValueIndex(null_mut(), &g, &SUB_PROCESSOR, &CP_MIN_CORES, &mut ac) != 0 {
            return None;
        }
        PowerReadDCValueIndex(null_mut(), &g, &SUB_PROCESSOR, &CP_MIN_CORES, &mut dc);
    }
    Some((ac, dc))
}

/// needs admin; re-activates the plan so it takes effect
pub fn set_parking(guid: &str, ac: u32, dc: u32) -> Result<(), String> {
    let g = parse_guid(guid).ok_or("bad power plan id")?;
    unsafe {
        let r = PowerWriteACValueIndex(null_mut(), &g, &SUB_PROCESSOR, &CP_MIN_CORES, ac);
        if r != 0 {
            return Err(err(r));
        }
        PowerWriteDCValueIndex(null_mut(), &g, &SUB_PROCESSOR, &CP_MIN_CORES, dc);
    }
    if active().as_deref() == Some(guid) {
        set_active(guid)?;
    }
    Ok(())
}

/// 1 ms timer for this process (Windows 11: only affects processes that ask for it themselves,
/// system-wide only with GlobalTimerResolutionRequests, see boost.rs)
pub fn timer_1ms(on: bool) {
    unsafe {
        if on {
            timeBeginPeriod(1);
        } else {
            timeEndPeriod(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guids() {
        let g = parse_guid(BALANCED).unwrap();
        assert_eq!(guid_str(&g), BALANCED);
        assert!(parse_guid("nope").is_none());
    }

    #[test]
    fn plans_are_readable() {
        let a = active().unwrap();
        assert!(list().iter().any(|(g, _)| *g == a));
    }
}
