// Registry: read / write / delete single values and whole keys (HKCU or HKLM).

use crate::win::{from_wide, wide};
use std::ffi::c_void;
use std::ptr::{null, null_mut};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteKeyValueW, RegDeleteTreeW, RegGetValueW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_WRITE, REG_DWORD, REG_OPTION_NON_VOLATILE, REG_SZ, RRF_RT_REG_DWORD,
    RRF_RT_REG_SZ,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Root {
    Hkcu,
    Hklm,
}

impl Root {
    fn hkey(self) -> HKEY {
        match self {
            Root::Hkcu => HKEY_CURRENT_USER,
            Root::Hklm => HKEY_LOCAL_MACHINE,
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Root::Hkcu => "HKCU",
            Root::Hklm => "HKLM",
        }
    }
    pub fn from_key(s: &str) -> Option<Root> {
        match s {
            "HKCU" => Some(Root::Hkcu),
            "HKLM" => Some(Root::Hklm),
            _ => None,
        }
    }
}

/// a registry value as we need it (old values go into the undo journal)
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Value {
    Dword(u32),
    Sz(String),
}

impl Value {
    /// "d:38" / "s:text" for the journal
    pub fn encode(&self) -> String {
        match self {
            Value::Dword(d) => format!("d:{d}"),
            Value::Sz(s) => format!("s:{s}"),
        }
    }
    pub fn decode(s: &str) -> Option<Value> {
        if let Some(d) = s.strip_prefix("d:") {
            return d.parse().ok().map(Value::Dword);
        }
        s.strip_prefix("s:").map(|t| Value::Sz(t.to_string()))
    }
}

fn err(code: u32) -> String {
    std::io::Error::from_raw_os_error(code as i32).to_string()
}

pub fn get_dword(root: Root, path: &str, name: &str) -> Option<u32> {
    let mut v = 0u32;
    let mut len = 4u32;
    let r = unsafe {
        RegGetValueW(
            root.hkey(),
            wide(path).as_ptr(),
            wide(name).as_ptr(),
            RRF_RT_REG_DWORD,
            null_mut(),
            &mut v as *mut u32 as *mut c_void,
            &mut len,
        )
    };
    (r == 0).then_some(v)
}

pub fn get_sz(root: Root, path: &str, name: &str) -> Option<String> {
    let (p, n) = (wide(path), wide(name));
    let mut len = 0u32;
    unsafe {
        if RegGetValueW(root.hkey(), p.as_ptr(), n.as_ptr(), RRF_RT_REG_SZ, null_mut(), null_mut(), &mut len) != 0 {
            return None;
        }
        let mut buf = vec![0u16; len as usize / 2 + 1];
        len = (buf.len() * 2) as u32;
        if RegGetValueW(root.hkey(), p.as_ptr(), n.as_ptr(), RRF_RT_REG_SZ, null_mut(), buf.as_mut_ptr() as *mut c_void, &mut len) != 0 {
            return None;
        }
        Some(from_wide(&buf))
    }
}

pub fn get(root: Root, path: &str, name: &str) -> Option<Value> {
    get_dword(root, path, name).map(Value::Dword).or_else(|| get_sz(root, path, name).map(Value::Sz))
}

fn with_key<T>(root: Root, path: &str, f: impl FnOnce(HKEY) -> Result<T, String>) -> Result<T, String> {
    let mut key: HKEY = null_mut();
    let r = unsafe {
        RegCreateKeyExW(root.hkey(), wide(path).as_ptr(), 0, null(), REG_OPTION_NON_VOLATILE, KEY_WRITE, null(), &mut key, null_mut())
    };
    if r != 0 {
        return Err(err(r));
    }
    let out = f(key);
    unsafe { RegCloseKey(key) };
    out
}

/// creates the key if needed
pub fn set(root: Root, path: &str, name: &str, value: &Value) -> Result<(), String> {
    with_key(root, path, |key| {
        let n = wide(name);
        let r = match value {
            Value::Dword(d) => unsafe { RegSetValueExW(key, n.as_ptr(), 0, REG_DWORD, d as *const u32 as *const u8, 4) },
            Value::Sz(s) => {
                let w = wide(s);
                unsafe { RegSetValueExW(key, n.as_ptr(), 0, REG_SZ, w.as_ptr() as *const u8, (w.len() * 2) as u32) }
            }
        };
        if r == 0 {
            Ok(())
        } else {
            Err(err(r))
        }
    })
}

/// deleting something that isn't there counts as success
pub fn delete_value(root: Root, path: &str, name: &str) -> Result<(), String> {
    let r = unsafe { RegDeleteKeyValueW(root.hkey(), wide(path).as_ptr(), wide(name).as_ptr()) };
    if r == 0 || r == 2 {
        Ok(())
    } else {
        Err(err(r))
    }
}

/// the key with everything below it
pub fn delete_tree(root: Root, path: &str) -> Result<(), String> {
    let r = unsafe { RegDeleteTreeW(root.hkey(), wide(path).as_ptr()) };
    if r == 0 || r == 2 {
        Ok(())
    } else {
        Err(err(r))
    }
}

pub fn key_exists(root: Root, path: &str) -> bool {
    use windows_sys::Win32::System::Registry::{RegOpenKeyExW, KEY_READ};
    let mut key: HKEY = null_mut();
    let r = unsafe { RegOpenKeyExW(root.hkey(), wide(path).as_ptr(), 0, KEY_READ, &mut key) };
    if r == 0 {
        unsafe { RegCloseKey(key) };
    }
    r == 0
}

/// write `value` (or delete when None)
pub fn restore(root: Root, path: &str, name: &str, value: Option<&Value>) -> Result<(), String> {
    match value {
        Some(v) => set(root, path, name, v),
        None => delete_value(root, path, name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_hkcu() {
        let path = r"Software\pcmax-test";
        set(Root::Hkcu, path, "d", &Value::Dword(38)).unwrap();
        set(Root::Hkcu, path, "s", &Value::Sz("GpuPreference=2;".into())).unwrap();
        assert_eq!(get(Root::Hkcu, path, "d"), Some(Value::Dword(38)));
        assert_eq!(get(Root::Hkcu, path, "s"), Some(Value::Sz("GpuPreference=2;".into())));
        delete_value(Root::Hkcu, path, "s").unwrap();
        assert_eq!(get(Root::Hkcu, path, "s"), None);
        set(Root::Hkcu, &format!(r"{path}\sub\deeper"), "x", &Value::Dword(1)).unwrap();
        delete_tree(Root::Hkcu, path).unwrap();
        assert_eq!(get(Root::Hkcu, path, "d"), None);
        assert!(!key_exists(Root::Hkcu, path));
        assert_eq!(Value::decode(&Value::Dword(2).encode()), Some(Value::Dword(2)));
    }
}
