// Windows Defender: a temporary exclusion for one folder (extracting thousands of small files is
// mostly Defender scanning each one) + a scan of that folder afterwards, so nothing slips through.

use crate::undo::{self, Entry};
use crate::win::{powershell, ps_quote, run_hidden};
use std::path::Path;

/// needs admin; goes into the undo journal first, so a crash can't leave the hole open
pub fn exclude(dir: &Path) -> Result<(), String> {
    if !crate::win::is_elevated() {
        return Err("the Defender exclusion needs admin rights".into());
    }
    let d = dir.to_string_lossy().to_string();
    undo::add(&Entry::Defender(d.clone()));
    let (ok, out) = powershell(&format!("Add-MpPreference -ExclusionPath {}", ps_quote(&d)));
    if ok {
        Ok(())
    } else {
        undo::remove(&Entry::Defender(d));
        Err(format!("Defender exclusion: {out}"))
    }
}

pub fn unexclude(dir: &str) -> Result<(), String> {
    let (ok, out) = powershell(&format!("Remove-MpPreference -ExclusionPath {}", ps_quote(dir)));
    if ok {
        undo::remove(&Entry::Defender(dir.to_string()));
        Ok(())
    } else {
        Err(format!("removing the Defender exclusion: {out}"))
    }
}

fn mpcmdrun() -> String {
    let pf = std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".into());
    format!(r"{pf}\Windows Defender\MpCmdRun.exe")
}

/// custom scan of one folder; Ok(text) = clean, Err(text) = threats found (Defender removes them) or failed
pub fn scan(dir: &Path) -> Result<String, String> {
    let d = dir.to_string_lossy().to_string();
    let (ok, out) = run_hidden(&mpcmdrun(), &["-Scan", "-ScanType", "3", "-File", &d]);
    let last = out.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").trim().to_string();
    if ok {
        Ok(format!("Defender scan: clean ({last})"))
    } else {
        Err(format!("Defender scan: {last}"))
    }
}
