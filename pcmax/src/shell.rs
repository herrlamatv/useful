// Windows integration: right-click menu (HKCU, no admin), autostart (Run key or a task with admin
// rights), the `fast` function in the PowerShell profile.

use crate::reg::{self, Root, Value};
use crate::win::run_hidden;
use std::path::PathBuf;

const CLASSES: &str = r"Software\Classes";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const TASK: &str = "pcmax";

/// pcmax.exe next to the running exe (pcm.exe and pcmax.exe live in the same folder)
pub fn gui_exe() -> PathBuf {
    let me = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("pcmax.exe"));
    me.with_file_name("pcmax.exe")
}

fn notify_shell() {
    use windows_sys::Win32::UI::Shell::{SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_IDLIST};
    unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED as i32, SHCNF_IDLIST, std::ptr::null(), std::ptr::null()) };
}

fn menu_keys() -> Vec<String> {
    let mut k: Vec<String> =
        crate::extract::EXTENSIONS.iter().map(|e| format!(r"{CLASSES}\SystemFileAssociations\{e}\shell\pcmax.extract")).collect();
    k.push(format!(r"{CLASSES}\Directory\shell\pcmax.copy"));
    k
}

/// "Unpack with pcmax" on archives, "Copy with pcmax to ..." on folders (Win 11: under "Show more options")
pub fn install_menu(german: bool) -> Result<(), String> {
    let exe = gui_exe();
    let exe = exe.to_string_lossy();
    let (unpack, copy) = if german {
        ("Mit pcmax entpacken (schnell)", "Mit pcmax kopieren nach ...")
    } else {
        ("Unpack with pcmax (fast)", "Copy with pcmax to ...")
    };
    for key in menu_keys() {
        let (text, verb) = if key.ends_with("pcmax.copy") { (copy, "copy") } else { (unpack, "extract") };
        reg::set(Root::Hkcu, &key, "", &Value::Sz(text.into()))?;
        reg::set(Root::Hkcu, &key, "Icon", &Value::Sz(format!("\"{exe}\",0")))?;
        reg::set(Root::Hkcu, &format!(r"{key}\command"), "", &Value::Sz(format!("\"{exe}\" {verb} \"%1\"")))?;
    }
    notify_shell();
    Ok(())
}

pub fn uninstall_menu() -> Result<(), String> {
    for key in menu_keys() {
        reg::delete_tree(Root::Hkcu, &key)?;
    }
    notify_shell();
    Ok(())
}

pub fn menu_installed() -> bool {
    reg::key_exists(Root::Hkcu, &menu_keys()[0])
}

// ---------------- autostart ----------------

pub fn autostart_installed() -> bool {
    reg::get_sz(Root::Hkcu, RUN_KEY, "pcmax").is_some() || task_installed()
}

pub fn task_installed() -> bool {
    run_hidden("schtasks.exe", &["/Query", "/TN", TASK]).0
}

/// with_admin: a scheduled task "at logon, highest rights" (needs admin once, no UAC prompt later)
pub fn install_autostart(with_admin: bool) -> Result<(), String> {
    let exe = gui_exe().to_string_lossy().to_string();
    if with_admin {
        if !crate::win::is_elevated() {
            return Err("autostart with admin rights has to be set up as admin".into());
        }
        let xml = task_xml(&exe);
        let path = std::env::temp_dir().join("pcmax-task.xml");
        // schtasks wants UTF-16 with BOM
        let mut bytes = vec![0xFF, 0xFE];
        for u in xml.encode_utf16() {
            bytes.extend_from_slice(&u.to_le_bytes());
        }
        std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
        let (ok, out) = run_hidden("schtasks.exe", &["/Create", "/TN", TASK, "/XML", &path.to_string_lossy(), "/F"]);
        let _ = std::fs::remove_file(&path);
        if !ok {
            return Err(format!("schtasks: {out}"));
        }
        // only one way to start
        reg::delete_value(Root::Hkcu, RUN_KEY, "pcmax")
    } else {
        if task_installed() && crate::win::is_elevated() {
            run_hidden("schtasks.exe", &["/Delete", "/TN", TASK, "/F"]);
        }
        reg::set(Root::Hkcu, RUN_KEY, "pcmax", &Value::Sz(format!("\"{exe}\" --tray")))
    }
}

pub fn uninstall_autostart() -> Result<(), String> {
    reg::delete_value(Root::Hkcu, RUN_KEY, "pcmax")?;
    if task_installed() {
        let (ok, out) = run_hidden("schtasks.exe", &["/Delete", "/TN", TASK, "/F"]);
        if !ok {
            return Err(format!("the admin autostart task can only be removed as admin: {out}"));
        }
    }
    Ok(())
}

fn task_xml(exe: &str) -> String {
    let user = std::env::var("USERDOMAIN").map(|d| format!("{d}\\")).unwrap_or_default() + &std::env::var("USERNAME").unwrap_or_default();
    let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    let (user, exe) = (esc(&user), esc(exe));
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo><Description>pcmax tray (boost, rules, limits)</Description></RegistrationInfo>
  <Triggers><LogonTrigger><Enabled>true</Enabled><UserId>{user}</UserId></LogonTrigger></Triggers>
  <Principals><Principal id="Author"><UserId>{user}</UserId><LogonType>InteractiveToken</LogonType><RunLevel>HighestAvailable</RunLevel></Principal></Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>5</Priority>
  </Settings>
  <Actions Context="Author"><Exec><Command>{exe}</Command><Arguments>--tray</Arguments></Exec></Actions>
</Task>"#
    )
}

// ---------------- PowerShell profile ----------------

const MARK_START: &str = "# >>> pcmax >>>";
const MARK_END: &str = "# <<< pcmax <<<";

/// profile files of PowerShell 7 and Windows PowerShell that exist on this PC
pub fn pwsh_profiles() -> Vec<PathBuf> {
    let mut out = vec![];
    for shell in ["pwsh.exe", "powershell.exe"] {
        let (ok, p) = run_hidden(shell, &["-NoProfile", "-NonInteractive", "-Command", "$PROFILE"]);
        if ok && !p.trim().is_empty() {
            out.push(PathBuf::from(p.trim()));
        }
    }
    out
}

fn without_snippet(text: &str) -> String {
    match (text.find(MARK_START), text.find(MARK_END)) {
        (Some(a), Some(b)) if b > a => {
            let end = b + MARK_END.len();
            let end = text[end..].find('\n').map(|n| end + n + 1).unwrap_or(text.len());
            format!("{}{}", &text[..a], &text[end..])
        }
        _ => text.to_string(),
    }
}

pub fn install_pwsh() -> Result<Vec<PathBuf>, String> {
    let profiles = pwsh_profiles();
    if profiles.is_empty() {
        return Err("no PowerShell found".into());
    }
    for p in &profiles {
        let old = std::fs::read_to_string(p).unwrap_or_default();
        let mut text = without_snippet(&old);
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(crate::fast::PWSH_SNIPPET);
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
        }
        std::fs::write(p, text).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    Ok(profiles)
}

pub fn uninstall_pwsh() -> Result<(), String> {
    for p in pwsh_profiles() {
        if let Ok(old) = std::fs::read_to_string(&p) {
            if old.contains(MARK_START) {
                std::fs::write(&p, without_snippet(&old)).map_err(|e| format!("{}: {e}", p.display()))?;
            }
        }
    }
    Ok(())
}

pub fn pwsh_installed() -> bool {
    pwsh_profiles().iter().any(|p| std::fs::read_to_string(p).is_ok_and(|t| t.contains(MARK_START)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snippet_removal() {
        let t = format!("a\n{}\nb\n", crate::fast::PWSH_SNIPPET.trim_end());
        assert_eq!(without_snippet(&t), "a\nb\n");
        assert_eq!(without_snippet("x\n"), "x\n");
    }
}
