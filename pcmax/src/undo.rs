// Undo journal (%APPDATA%\pcmax\undo.txt): every system change is written here BEFORE it is made.
// After a crash, a kill or a reboot, pcmax reverts what's left on the next start.

use crate::reg::{self, Root, Value};
use crate::{defender, gpu, power, services};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Entry {
    /// the power plan that was active before
    Plan(String),
    /// core parking values of a plan before
    Parking { plan: String, ac: u32, dc: u32 },
    /// a service we stopped
    Service(String),
    /// a registry value and what it was (None = didn't exist)
    Reg { root: Root, path: String, name: String, old: Option<Value> },
    /// a folder we excluded from Defender
    Defender(String),
    /// a GPU preference we set and what it was
    GpuPref { exe: String, old: Option<String> },
}

impl Entry {
    pub fn encode(&self) -> String {
        let opt = |o: &Option<String>| o.clone().unwrap_or_else(|| "-".into());
        match self {
            Entry::Plan(g) => format!("plan|{g}"),
            Entry::Parking { plan, ac, dc } => format!("parking|{plan}|{ac}|{dc}"),
            Entry::Service(s) => format!("service|{s}"),
            Entry::Reg { root, path, name, old } => {
                format!("reg|{}|{path}|{name}|{}", root.key(), opt(&old.as_ref().map(|v| v.encode())))
            }
            Entry::Defender(d) => format!("defender|{d}"),
            Entry::GpuPref { exe, old } => format!("gpupref|{exe}|{}", opt(old)),
        }
    }

    pub fn decode(line: &str) -> Option<Entry> {
        let p: Vec<&str> = line.trim().split('|').collect();
        let opt = |s: &str| (s != "-").then(|| s.to_string());
        Some(match p.as_slice() {
            ["plan", g] => Entry::Plan(g.to_string()),
            ["parking", plan, ac, dc] => Entry::Parking { plan: plan.to_string(), ac: ac.parse().ok()?, dc: dc.parse().ok()? },
            ["service", s] => Entry::Service(s.to_string()),
            ["reg", root, path, name, old] => Entry::Reg {
                root: Root::from_key(root)?,
                path: path.to_string(),
                name: name.to_string(),
                old: opt(old).and_then(|v| Value::decode(&v)),
            },
            ["defender", d] => Entry::Defender(d.to_string()),
            ["gpupref", exe, old] => Entry::GpuPref { exe: exe.to_string(), old: opt(old) },
            _ => return None,
        })
    }

    /// human text for the UI
    pub fn describe(&self) -> String {
        match self {
            Entry::Plan(g) => format!("power plan back to '{}'", power::name_of(g)),
            Entry::Parking { ac, .. } => format!("core parking back to {ac} %"),
            Entry::Service(s) => format!("start service {s} again"),
            Entry::Reg { path, name, .. } => format!("registry {path}\\{name} back"),
            Entry::Defender(d) => format!("remove Defender exclusion {d}"),
            Entry::GpuPref { exe, .. } => format!("GPU preference of {exe} back"),
        }
    }

    pub fn needs_admin(&self) -> bool {
        match self {
            Entry::Parking { .. } | Entry::Service(_) | Entry::Defender(_) => true,
            Entry::Reg { root, .. } => *root == Root::Hklm,
            Entry::Plan(_) | Entry::GpuPref { .. } => false,
        }
    }

    pub fn revert(&self) -> Result<(), String> {
        match self {
            Entry::Plan(g) => power::set_active(g),
            Entry::Parking { plan, ac, dc } => power::set_parking(plan, *ac, *dc),
            Entry::Service(s) => services::start(s),
            Entry::Reg { root, path, name, old } => reg::restore(*root, path, name, old.as_ref()),
            Entry::Defender(d) => defender::unexclude(d),
            Entry::GpuPref { exe, old } => gpu::restore_pref(exe, old.as_deref()),
        }
    }
}

/// pcm and pcmax can run at the same time - one writer inside a process at least
static LOCK: Mutex<()> = Mutex::new(());

fn file() -> Option<PathBuf> {
    crate::settings::dir().map(|d| d.join("undo.txt"))
}

pub fn load() -> Vec<Entry> {
    let Some(f) = file() else { return vec![] };
    std::fs::read_to_string(f).unwrap_or_default().lines().filter_map(Entry::decode).collect()
}

fn save(entries: &[Entry]) {
    let Some(f) = file() else { return };
    if entries.is_empty() {
        let _ = std::fs::remove_file(f);
        return;
    }
    if let Some(d) = f.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let text: String = entries.iter().map(|e| e.encode() + "\n").collect();
    let _ = std::fs::write(f, text);
}

/// the first "before" value of something wins: if the same thing is changed twice, keep the original
pub fn add(e: &Entry) {
    let _g = LOCK.lock().unwrap();
    let mut all = load();
    let same_target = |a: &Entry| match (a, e) {
        (Entry::Plan(_), Entry::Plan(_)) => true,
        (Entry::Parking { plan: p1, .. }, Entry::Parking { plan: p2, .. }) => p1 == p2,
        (Entry::Reg { root: r1, path: p1, name: n1, .. }, Entry::Reg { root: r2, path: p2, name: n2, .. }) => {
            r1 == r2 && p1.eq_ignore_ascii_case(p2) && n1.eq_ignore_ascii_case(n2)
        }
        (Entry::GpuPref { exe: a, .. }, Entry::GpuPref { exe: b, .. }) => a.eq_ignore_ascii_case(b),
        (a, b) => a == b,
    };
    if !all.iter().any(same_target) {
        all.push(e.clone());
        save(&all);
    }
}

pub fn remove(e: &Entry) {
    let _g = LOCK.lock().unwrap();
    let mut all = load();
    all.retain(|x| x != e);
    save(&all);
}

/// newest first; successful ones leave the journal. Returns (what, result) per entry.
pub fn revert_all() -> Vec<(String, Result<(), String>)> {
    let entries = load();
    let admin = crate::win::is_elevated();
    let mut out = vec![];
    for e in entries.iter().rev() {
        if e.needs_admin() && !admin {
            out.push((e.describe(), Err("needs admin rights".into())));
            continue;
        }
        let r = e.revert();
        if r.is_ok() {
            remove(e);
        }
        out.push((e.describe(), r));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codec() {
        let all = [
            Entry::Plan(power::BALANCED.into()),
            Entry::Parking { plan: power::BALANCED.into(), ac: 10, dc: 5 },
            Entry::Service("WSearch".into()),
            Entry::Reg {
                root: Root::Hklm,
                path: r"SYSTEM\CurrentControlSet\Control\PriorityControl".into(),
                name: "Win32PrioritySeparation".into(),
                old: Some(Value::Dword(2)),
            },
            Entry::Reg { root: Root::Hkcu, path: r"Software\x".into(), name: "y".into(), old: None },
            Entry::Defender(r"C:\Users\x\Downloads\big".into()),
            Entry::GpuPref { exe: r"C:\Games\game.exe".into(), old: Some("SwapEffectUpgradeEnable=1;".into()) },
            Entry::GpuPref { exe: r"C:\Games\b.exe".into(), old: None },
        ];
        for e in all {
            assert_eq!(Entry::decode(&e.encode()), Some(e));
        }
    }
}
