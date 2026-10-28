// The general boost: one switch, every measure can be turned off on its own.
// "Static" measures change the system once (power plan, services, registry) and go into the undo journal.
// "Dynamic" measures need the running engine (pcmax in the tray): they follow the foreground window,
// background programs and new processes.

use crate::reg::{self, Root, Value};
use crate::undo::{self, Entry};
use crate::{gpu, power, services, topo};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Measure {
    /// fastest power plan
    Plan,
    /// no parked cores
    Parking,
    /// 1 ms timer
    Timer,
    /// background programs: Efficiency Mode + below normal
    Background,
    /// pause Search / SysMain / Delivery Optimization
    Services,
    /// the window in front (+ its children) gets priority, Windows quantum for foreground up
    Foreground,
    /// games -> 3D V-Cache CCD, work -> the other CCD
    X3d,
    /// listed apps -> dedicated GPU
    GpuPref,
}

impl Measure {
    pub const ALL: [Measure; 8] = [
        Measure::Plan,
        Measure::Parking,
        Measure::Timer,
        Measure::Background,
        Measure::Services,
        Measure::Foreground,
        Measure::X3d,
        Measure::GpuPref,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Measure::Plan => "plan",
            Measure::Parking => "parking",
            Measure::Timer => "timer",
            Measure::Background => "background",
            Measure::Services => "services",
            Measure::Foreground => "foreground",
            Measure::X3d => "x3d",
            Measure::GpuPref => "gpupref",
        }
    }

    pub fn parse(s: &str) -> Option<Measure> {
        Measure::ALL.into_iter().find(|m| m.key() == s.trim())
    }

    /// fully (true) or partly needs admin
    pub fn needs_admin(self) -> bool {
        matches!(self, Measure::Parking | Measure::Services)
    }

    pub fn partly_admin(self) -> bool {
        matches!(self, Measure::Timer | Measure::Foreground)
    }

    /// needs the engine running (tray)
    pub fn dynamic(self) -> bool {
        matches!(self, Measure::Background | Measure::Foreground | Measure::X3d)
    }

    /// default on/off. X3D + AMD's driver: no plan switch / no unparking (the driver parks the 2nd CCD for games)
    pub fn default_on(self) -> bool {
        let x3d_driver = topo::get().is_x3d() && services::is_running("amd3dvcache");
        match self {
            Measure::Plan | Measure::Parking => !x3d_driver,
            Measure::Timer => false,
            Measure::X3d => topo::get().is_x3d(),
            _ => true,
        }
    }
}

pub fn default_measures() -> Vec<Measure> {
    Measure::ALL.into_iter().filter(|m| m.default_on()).collect()
}

pub const DEFAULT_BACKGROUND: &str =
    "OneDrive.exe, chrome.exe, msedge.exe, firefox.exe, opera.exe, brave.exe, Discord.exe, Spotify.exe, steamwebhelper.exe, EpicGamesLauncher.exe, Teams.exe, ms-teams.exe, Dropbox.exe, GoogleDriveFS.exe";
pub const DEFAULT_WORK: &str =
    "7z.exe, 7zG.exe, WinRAR.exe, UnRAR.exe, Rar.exe, tar.exe, cargo.exe, rustc.exe, cl.exe, link.exe, msbuild.exe, ffmpeg.exe, HandBrakeCLI.exe, clang.exe, gcc.exe";

const PRIO_KEY: &str = r"SYSTEM\CurrentControlSet\Control\PriorityControl";
const PRIO_NAME: &str = "Win32PrioritySeparation";
/// short, variable quantum, foreground gets 3x - "programs" with the maximum boost
const PRIO_FOREGROUND: u32 = 0x26;
const TIMER_KEY: &str = r"SYSTEM\CurrentControlSet\Control\Session Manager\kernel";
const TIMER_NAME: &str = "GlobalTimerResolutionRequests";

pub type Outcome = (Measure, Result<String, String>);

fn reg_change(root: Root, path: &str, name: &str, value: Value) -> Result<String, String> {
    let old = reg::get(root, path, name);
    if old.as_ref() == Some(&value) {
        return Ok("already set".into());
    }
    undo::add(&Entry::Reg { root, path: path.into(), name: name.into(), old });
    reg::set(root, path, name, &value).map(|_| "set".to_string())
}

/// the system part of the boost; `gpu_apps` = full exe paths
pub fn apply_static(on: &[Measure], gpu_apps: &[String]) -> Vec<Outcome> {
    let admin = crate::win::is_elevated();
    let mut out = vec![];
    for &m in on {
        let r = match m {
            Measure::Plan => (|| {
                let old = power::active().ok_or("active plan not readable")?;
                let (fast, name) = power::fastest().ok_or("no 'High performance' / 'Ultimate Performance' plan")?;
                if old == fast {
                    return Ok(format!("'{name}' is already active"));
                }
                undo::add(&Entry::Plan(old));
                power::set_active(&fast).map(|_| format!("'{name}' active"))
            })(),
            Measure::Parking => (|| {
                if !admin {
                    return Err("needs admin".into());
                }
                let plan = power::active().ok_or("active plan not readable")?;
                let (ac, dc) = power::parking(&plan).ok_or("parking setting not readable")?;
                if ac == 100 && dc == 100 {
                    return Ok("no core is parked already".into());
                }
                undo::add(&Entry::Parking { plan: plan.clone(), ac, dc });
                power::set_parking(&plan, 100, 100).map(|_| "all cores awake".into())
            })(),
            Measure::Timer => {
                // our own request (lives as long as pcmax runs) + make it count system-wide (admin)
                power::timer_1ms(true);
                if admin {
                    reg_change(Root::Hklm, TIMER_KEY, TIMER_NAME, Value::Dword(1)).map(|_| "1 ms (system-wide after the next reboot)".into())
                } else {
                    Ok("1 ms for pcmax only (system-wide needs admin)".into())
                }
            }
            Measure::Services => (|| {
                if !admin {
                    return Err("needs admin".into());
                }
                let mut paused = vec![];
                for (name, _) in services::PAUSABLE {
                    if services::is_running(name) {
                        undo::add(&Entry::Service(name.into()));
                        match services::stop(name) {
                            Ok(true) => paused.push(name),
                            Ok(false) => undo::remove(&Entry::Service(name.into())),
                            Err(e) => {
                                undo::remove(&Entry::Service(name.into()));
                                return Err(e);
                            }
                        }
                    }
                }
                Ok(if paused.is_empty() { "nothing running".into() } else { format!("paused: {}", paused.join(", ")) })
            })(),
            Measure::Foreground => {
                if admin {
                    reg_change(Root::Hklm, PRIO_KEY, PRIO_NAME, Value::Dword(PRIO_FOREGROUND))
                        .map(|_| "window in front gets priority + Windows foreground boost 3x".into())
                } else {
                    Ok("window in front gets priority (the Windows foreground boost needs admin)".into())
                }
            }
            Measure::GpuPref => {
                let mut n = 0;
                let mut errs = vec![];
                for exe in gpu_apps.iter().filter(|e| !e.trim().is_empty()) {
                    let old = gpu::get_pref_raw(exe);
                    undo::add(&Entry::GpuPref { exe: exe.clone(), old });
                    match gpu::set_pref(exe, gpu::GpuPref::High) {
                        Ok(()) => n += 1,
                        Err(e) => errs.push(e),
                    }
                }
                if errs.is_empty() {
                    Ok(format!("{n} apps use the dedicated GPU (from their next start)"))
                } else {
                    Err(errs.join("; "))
                }
            }
            Measure::Background => Ok("background programs: Efficiency Mode + below normal".into()),
            Measure::X3d => Ok(if topo::get().is_x3d() { "games -> V-Cache CCD, work -> clock CCD".into() } else { "no X3D CPU".into() }),
        };
        out.push((m, r));
    }
    out
}

/// everything the boost changed (not the Defender exclusions of a running unpack)
pub fn revert_static() -> Vec<(String, Result<(), String>)> {
    power::timer_1ms(false);
    let admin = crate::win::is_elevated();
    let mut out = vec![];
    for e in undo::load().iter().rev() {
        if matches!(e, Entry::Defender(_)) {
            continue;
        }
        if e.needs_admin() && !admin {
            out.push((e.describe(), Err("needs admin".into())));
            continue;
        }
        let r = e.revert();
        if r.is_ok() {
            undo::remove(e);
        }
        out.push((e.describe(), r));
    }
    out
}
