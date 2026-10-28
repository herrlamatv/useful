// The engine: runs on its own thread inside pcmax.exe (also when the window is hidden in the tray).
//  - rules for programs that start (+ the ones already running when pcmax starts)
//  - the dynamic part of the boost: window in front, background programs, X3D cores
//  - hotkeys Ctrl+Alt+Up / Down / 0 for the window in front
//  - named events, so `pcm boost on|off` can switch the running engine

use crate::boost::{self, Measure};
use crate::cpu::{self, Eco, Prio};
use crate::job::Job;
use crate::procs::{self, ProcInfo};
use crate::profile::{self, Profile};
use crate::rules;
use crate::settings::Settings;
use crate::topo;
use crate::undo;
use crate::win::{last_error_code, open_process, wide, Handle};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{ERROR_ALREADY_EXISTS, WAIT_OBJECT_0};
use windows_sys::Win32::System::Threading::{
    CreateEventW, CreateMutexW, OpenEventW, OpenMutexW, SetEvent, EVENT_MODIFY_STATE, PROCESS_QUERY_INFORMATION,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_INFORMATION, SYNCHRONIZATION_SYNCHRONIZE,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT};
use windows_sys::Win32::UI::WindowsAndMessaging::{MsgWaitForMultipleObjects, PeekMessageW, MSG, PM_REMOVE, QS_ALLINPUT, WM_HOTKEY};

pub const MUTEX: &str = "Local\\pcmax-engine";
pub const EV_BOOST_ON: &str = "Local\\pcmax-boost-on";
pub const EV_BOOST_OFF: &str = "Local\\pcmax-boost-off";
pub const EV_RELOAD: &str = "Local\\pcmax-reload";
/// the tray thread listens to this one (second start of pcmax.exe -> show the window)
pub const EV_SHOW: &str = "Local\\pcmax-show";

const HK_BOOST: i32 = 1;
const HK_CALM: i32 = 2;
const HK_RESET: i32 = 3;

pub enum Cmd {
    Boost(bool),
    Settings(Box<Settings>),
    /// apply a profile to a process from the GUI
    Apply(u32, Box<Profile>, bool),
    Quit,
}

/// what the GUI shows
#[derive(Default)]
pub struct Shared {
    pub boost_on: bool,
    pub outcomes: Vec<(Measure, Result<String, String>)>,
    pub log: VecDeque<String>,
    pub foreground: String,
    pub damped: usize,
    pub pinned_games: usize,
    pub pinned_work: usize,
    pub rules_applied: usize,
    /// result lines of the last Cmd::Apply
    pub last_apply: Vec<String>,
}

pub struct EngineHandle {
    pub tx: Sender<Cmd>,
    pub shared: Arc<Mutex<Shared>>,
    /// boost on? (for the tray menu, without locking)
    pub boost: Arc<AtomicBool>,
    /// set when the engine has put everything back and ended
    pub stopped: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl EngineHandle {
    pub fn send(&self, c: Cmd) {
        let _ = self.tx.send(c);
    }
    /// restores everything the engine changed, then stops it
    pub fn stop(mut self) {
        let _ = self.tx.send(Cmd::Quit);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// None if another pcmax engine already runs
pub fn claim_single_instance() -> Option<Handle> {
    let h = unsafe { CreateMutexW(std::ptr::null(), 0, wide(MUTEX).as_ptr()) };
    let exists = last_error_code() == ERROR_ALREADY_EXISTS;
    let h = Handle::new(h)?;
    if exists {
        None
    } else {
        Some(h)
    }
}

pub fn running() -> bool {
    Handle::new(unsafe { OpenMutexW(SYNCHRONIZATION_SYNCHRONIZE, 0, wide(MUTEX).as_ptr()) }).is_some()
}

/// wake the running engine / tray (from pcm or a second pcmax.exe)
pub fn signal(name: &str) -> bool {
    match Handle::new(unsafe { OpenEventW(EVENT_MODIFY_STATE, 0, wide(name).as_ptr()) }) {
        Some(h) => unsafe { SetEvent(h.raw()) != 0 },
        None => false,
    }
}

pub fn event(name: &str) -> Option<Handle> {
    Handle::new(unsafe { CreateEventW(std::ptr::null(), 0, 0, wide(name).as_ptr()) })
}

#[derive(Clone, Copy)]
struct Orig {
    prio: Option<Prio>,
    eco: Option<Eco>,
}

#[derive(Clone, Copy, PartialEq)]
enum Pin {
    Game,
    Work,
}

pub type Notify = Box<dyn Fn(&str) + Send>;

struct Engine {
    s: Settings,
    boost: bool,
    boost_flag: Arc<AtomicBool>,
    shared: Arc<Mutex<Shared>>,
    notify: Notify,
    known: HashMap<u32, String>,
    /// processes the boost (X3D) already looked at
    boost_seen: HashSet<u32>,
    jobs: Vec<(u32, Job)>,
    fg_root: u32,
    fg_tree: Vec<u32>,
    fg_orig: HashMap<u32, Orig>,
    damped: HashMap<u32, Orig>,
    pinned: HashMap<u32, Pin>,
    hotkeys: bool,
    me: u32,
}

pub fn start(settings: Settings, notify: Notify) -> EngineHandle {
    let (tx, rx) = channel();
    let shared = Arc::new(Mutex::new(Shared::default()));
    let sh = shared.clone();
    let boost = Arc::new(AtomicBool::new(false));
    let stopped = Arc::new(AtomicBool::new(false));
    let (b2, st2) = (boost.clone(), stopped.clone());
    let thread = std::thread::Builder::new()
        .name("pcmax-engine".into())
        .spawn(move || {
            let mut e = Engine {
                s: settings,
                boost: false,
                boost_flag: b2,
                shared: sh,
                notify,
                known: HashMap::new(),
                boost_seen: HashSet::new(),
                jobs: vec![],
                fg_root: 0,
                fg_tree: vec![],
                fg_orig: HashMap::new(),
                damped: HashMap::new(),
                pinned: HashMap::new(),
                hotkeys: false,
                me: procs::own_pid(),
            };
            e.run(rx);
            st2.store(true, Ordering::SeqCst);
        })
        .expect("engine thread");
    EngineHandle { tx, shared, boost, stopped, thread: Some(thread) }
}

fn open_rw(pid: u32) -> Option<Handle> {
    open_process(pid, PROCESS_SET_INFORMATION | PROCESS_QUERY_INFORMATION)
        .or_else(|| open_process(pid, PROCESS_SET_INFORMATION | PROCESS_QUERY_LIMITED_INFORMATION))
}

fn read_orig(h: &Handle) -> Orig {
    Orig { prio: cpu::get_priority(h), eco: cpu::get_eco(h) }
}

fn restore(pid: u32, o: Orig) {
    if let Some(h) = open_rw(pid) {
        if let Some(p) = o.prio {
            let _ = cpu::set_priority(&h, p);
        }
        let _ = cpu::set_eco(&h, o.eco.unwrap_or(Eco::Auto));
    }
}

impl Engine {
    fn log(&self, s: impl Into<String>) {
        let mut sh = self.shared.lock().unwrap();
        let t = local_time();
        sh.log.push_back(format!("{t}  {}", s.into()));
        while sh.log.len() > 300 {
            sh.log.pop_front();
        }
    }

    fn publish(&self) {
        self.boost_flag.store(self.boost, Ordering::SeqCst);
        let mut sh = self.shared.lock().unwrap();
        sh.boost_on = self.boost;
        sh.damped = self.damped.len();
        sh.pinned_games = self.pinned.values().filter(|p| **p == Pin::Game).count();
        sh.pinned_work = self.pinned.values().filter(|p| **p == Pin::Work).count();
        sh.foreground = self.known.get(&self.fg_root).cloned().unwrap_or_default();
    }

    fn run(&mut self, rx: Receiver<Cmd>) {
        // leftovers of a crash / kill / reboot first
        let left = undo::load();
        if !left.is_empty() {
            for (what, r) in undo::revert_all() {
                match r {
                    Ok(()) => self.log(format!("restored after the last session: {what}")),
                    Err(e) => self.log(format!("could not restore: {what} ({e})")),
                }
            }
        }
        self.log(format!("engine started{}", if crate::win::is_elevated() { " (admin)" } else { "" }));
        if crate::win::is_elevated() {
            crate::win::enable_admin_privileges();
        }
        let events: Vec<Handle> = [EV_BOOST_ON, EV_BOOST_OFF, EV_RELOAD].iter().filter_map(|n| event(n)).collect();
        let raw: Vec<_> = events.iter().map(|h| h.raw()).collect();
        self.set_hotkeys(self.s.hotkeys);
        if self.s.boost_on {
            self.set_boost(true);
        }
        let mut last_scan = Instant::now() - Duration::from_secs(10);
        let mut last_fg = Instant::now();
        loop {
            let w = unsafe { MsgWaitForMultipleObjects(raw.len() as u32, raw.as_ptr(), 0, 250, QS_ALLINPUT) };
            let fired = w.wrapping_sub(WAIT_OBJECT_0) as usize;
            if fired < raw.len() {
                match fired {
                    0 => self.set_boost(true),
                    1 => self.set_boost(false),
                    _ => {
                        let s = Settings::load();
                        self.apply_settings(s);
                    }
                }
            }
            let mut msg: MSG = unsafe { std::mem::zeroed() };
            while unsafe { PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) } != 0 {
                if msg.message == WM_HOTKEY {
                    self.hotkey(msg.wParam as i32);
                }
            }
            loop {
                match rx.try_recv() {
                    Ok(Cmd::Boost(on)) => self.set_boost(on),
                    Ok(Cmd::Settings(s)) => self.apply_settings(*s),
                    Ok(Cmd::Apply(pid, p, rt)) => {
                        let rep = profile::apply(pid, &p, rt);
                        let lines = rep.lines();
                        if let Some(j) = rep.job {
                            self.jobs.push((pid, j));
                        }
                        self.shared.lock().unwrap().last_apply = lines;
                    }
                    Ok(Cmd::Quit) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.shutdown();
                        return;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                }
            }
            if last_scan.elapsed() >= Duration::from_millis(500) {
                self.scan();
                last_scan = Instant::now();
            }
            if last_fg.elapsed() >= Duration::from_millis(250) {
                self.foreground();
                last_fg = Instant::now();
            }
            self.publish();
        }
    }

    fn apply_settings(&mut self, s: Settings) {
        let boost_changed = s.boost_on != self.boost;
        let measures_changed = s.measures != self.s.measures || s.gpu_apps != self.s.gpu_apps;
        let lists_changed = s.background != self.s.background || s.games != self.s.games || s.work != self.s.work || s.auto_games != self.s.auto_games;
        let hotkeys = s.hotkeys;
        let rules_changed = s.rules != self.s.rules;
        let want = s.boost_on;
        // re-run the boost with the new measures
        if self.boost && (measures_changed || lists_changed) {
            self.set_boost(false);
            self.s = s;
            self.set_boost(true);
        } else {
            self.s = s;
            if boost_changed {
                self.set_boost(want);
            }
        }
        if rules_changed {
            // new rules count for running programs too
            self.known.clear();
        }
        self.set_hotkeys(hotkeys);
    }

    fn set_hotkeys(&mut self, on: bool) {
        if on == self.hotkeys {
            return;
        }
        let m = MOD_CONTROL | MOD_ALT | MOD_NOREPEAT;
        unsafe {
            if on {
                let mut failed = vec![];
                for (id, vk, name) in [(HK_BOOST, 0x26u32, "Ctrl+Alt+Up"), (HK_CALM, 0x28, "Ctrl+Alt+Down"), (HK_RESET, 0x30, "Ctrl+Alt+0")] {
                    if RegisterHotKey(std::ptr::null_mut(), id, m, vk) == 0 {
                        failed.push(name);
                    }
                }
                if failed.is_empty() {
                    self.log("hotkeys: Ctrl+Alt+Up boost, Ctrl+Alt+Down calm, Ctrl+Alt+0 reset (window in front)");
                } else {
                    self.log(format!("hotkeys already used by another program: {}", failed.join(", ")));
                }
            } else {
                for id in [HK_BOOST, HK_CALM, HK_RESET] {
                    UnregisterHotKey(std::ptr::null_mut(), id);
                }
            }
        }
        self.hotkeys = on;
    }

    fn set_boost(&mut self, on: bool) {
        if on == self.boost {
            return;
        }
        if on {
            let statics: Vec<Measure> = self.s.measures.to_vec();
            let out = boost::apply_static(&statics, &self.s.gpu_apps);
            let bad = out.iter().filter(|o| o.1.is_err()).count();
            for (m, r) in &out {
                match r {
                    Ok(t) => self.log(format!("boost {}: {t}", m.key())),
                    Err(e) => self.log(format!("boost {}: ✗ {e}", m.key())),
                }
            }
            self.shared.lock().unwrap().outcomes = out;
            self.boost = true;
            // dynamic parts for everything that already runs
            self.boost_seen.clear();
            self.fg_root = 0;
            let all = procs::snapshot();
            self.redamp(&all);
            (self.notify)(&if bad == 0 { "Boost on".to_string() } else { format!("Boost on ({bad} measures need admin / failed)") });
        } else {
            self.undo_dynamic();
            for (what, r) in boost::revert_static() {
                match r {
                    Ok(()) => self.log(format!("boost off: {what}")),
                    Err(e) => self.log(format!("boost off: ✗ {what}: {e}")),
                }
            }
            self.shared.lock().unwrap().outcomes.clear();
            self.boost = false;
            (self.notify)("Boost off - everything is back");
        }
        self.s.boost_on = on;
        let mut saved = Settings::load();
        saved.boost_on = on;
        let _ = saved.save();
        self.publish();
    }

    fn undo_dynamic(&mut self) {
        for (pid, o) in self.fg_orig.drain() {
            restore(pid, o);
        }
        for (pid, o) in self.damped.drain() {
            restore(pid, o);
        }
        let all = topo::get().all;
        for (pid, _) in self.pinned.drain() {
            if let Some(h) = open_rw(pid) {
                let _ = cpu::set_affinity(&h, all);
            }
        }
        self.fg_tree.clear();
        self.fg_root = 0;
    }

    fn shutdown(&mut self) {
        self.set_hotkeys(false);
        if self.boost {
            // statics stay in the journal only if we crash; a clean exit puts everything back
            self.undo_dynamic();
            boost::revert_static();
        }
        self.log("engine stopped");
    }

    fn is_game(&self, name: &str, path: Option<&str>) -> bool {
        let mut list = self.s.games.clone();
        if self.s.auto_games {
            list.push_str(r", \steamapps\common\, \Epic Games\, \GOG Galaxy\Games\");
        }
        rules::matches_list(&list, name, path)
    }

    fn scan(&mut self) {
        let all = procs::snapshot();
        let alive: HashSet<u32> = all.iter().map(|p| p.pid).collect();
        self.known.retain(|pid, _| alive.contains(pid));
        self.boost_seen.retain(|pid| alive.contains(pid));
        self.damped.retain(|pid, _| alive.contains(pid));
        self.pinned.retain(|pid, _| alive.contains(pid));
        self.fg_orig.retain(|pid, _| alive.contains(pid));
        self.jobs.retain(|(pid, _)| alive.contains(pid));

        let real = |p: &&ProcInfo| p.pid > 4 && p.pid != self.me;
        let rule_new: Vec<&ProcInfo> =
            all.iter().filter(real).filter(|p| self.known.get(&p.pid).is_none_or(|n| *n != p.name)).collect();
        let x3d = self.boost && self.s.has(Measure::X3d) && topo::get().is_x3d();
        let boost_new: Vec<&ProcInfo> =
            if x3d { all.iter().filter(real).filter(|p| !self.boost_seen.contains(&p.pid)).collect() } else { vec![] };
        let need_path = self.s.rules.iter().any(|r| r.enabled && r.pattern.contains('\\')) || self.s.auto_games || self.s.games.contains('\\');
        let path_of = |pid: u32| if need_path { procs::exe_path(pid) } else { None };

        let mut applied = 0;
        for p in &rule_new {
            let path = path_of(p.pid);
            if let Some(r) = self.s.rules.iter().filter(|r| r.enabled).find(|r| r.matches(&p.name, path.as_deref())) {
                let rep = profile::apply(p.pid, &r.profile, false);
                let state = if rep.err.is_empty() { "✓".to_string() } else { format!("partly: {}", rep.err.join("; ")) };
                self.log(format!("rule {} -> {} ({state})", p.name, r.profile.summary()));
                if let Some(j) = rep.job {
                    self.jobs.push((p.pid, j));
                }
                applied += 1;
            }
            self.known.insert(p.pid, p.name.clone());
        }
        for p in &boost_new {
            self.boost_seen.insert(p.pid);
            let path = path_of(p.pid);
            let t = topo::get();
            let pin = if self.is_game(&p.name, path.as_deref()) {
                Some((Pin::Game, t.vcache))
            } else if rules::matches_list(&self.s.work, &p.name, path.as_deref()) {
                Some((Pin::Work, t.freq))
            } else {
                None
            };
            if let Some((kind, Some(mask))) = pin {
                if let Some(h) = open_rw(p.pid) {
                    if cpu::set_affinity(&h, mask).is_ok() {
                        self.pinned.insert(p.pid, kind);
                        let what = if kind == Pin::Game { "V-Cache CCD" } else { "clock CCD" };
                        self.log(format!("{} -> {what} ({})", p.name, topo::format_list(mask)));
                    }
                }
            }
        }
        if self.boost && self.s.has(Measure::Background) && (!rule_new.is_empty() || !boost_new.is_empty()) {
            self.redamp(&all);
        }
        self.shared.lock().unwrap().rules_applied += applied;
    }

    /// background list programs: damp when not in front, undamp when in front
    fn redamp(&mut self, all: &[ProcInfo]) {
        if !(self.boost && self.s.has(Measure::Background)) {
            return;
        }
        let front: HashSet<u32> = self.fg_tree.iter().copied().collect();
        for p in all {
            if p.pid <= 4 || p.pid == self.me || !rules::matches_list(&self.s.background, &p.name, None) {
                continue;
            }
            let in_front = front.contains(&p.pid);
            if in_front {
                if let Some(o) = self.damped.remove(&p.pid) {
                    restore(p.pid, o);
                }
            } else if let std::collections::hash_map::Entry::Vacant(slot) = self.damped.entry(p.pid) {
                let Some(h) = open_rw(p.pid) else { continue };
                let o = read_orig(&h);
                if o.prio.is_some_and(|x| x > Prio::Normal) {
                    // someone wanted it fast on purpose (a rule, the user)
                    continue;
                }
                let _ = cpu::set_eco(&h, Eco::On);
                if o.prio == Some(Prio::Normal) {
                    let _ = cpu::set_priority(&h, Prio::BelowNormal);
                }
                slot.insert(o);
            }
        }
    }

    fn fg_tree_of(all: &[ProcInfo], root: u32) -> Vec<u32> {
        let name = all.iter().find(|p| p.pid == root).map(|p| p.name.to_ascii_lowercase()).unwrap_or_default();
        // Explorer started almost every program -> its tree would be "everything"
        if name == "explorer.exe" {
            return vec![root];
        }
        let mut t = procs::tree(all, root);
        t.truncate(64);
        t
    }

    fn foreground(&mut self) {
        let fg_on = self.boost && self.s.has(Measure::Foreground);
        let bg_on = self.boost && self.s.has(Measure::Background);
        if !fg_on && !bg_on {
            return;
        }
        let root = procs::foreground_pid();
        if root == 0 || root == self.fg_root {
            return;
        }
        let all = procs::snapshot();
        let tree: Vec<u32> = Engine::fg_tree_of(&all, root).into_iter().filter(|p| *p != self.me).collect();
        // the previous window goes back to what it had
        let keep: HashSet<u32> = tree.iter().copied().collect();
        let old: Vec<u32> = self.fg_orig.keys().copied().filter(|p| !keep.contains(p)).collect();
        for pid in old {
            if let Some(o) = self.fg_orig.remove(&pid) {
                restore(pid, o);
            }
        }
        self.fg_root = root;
        self.fg_tree = tree.clone();
        // undamp first, so the orig we save for the foreground is the real one
        self.redamp(&all);
        if fg_on {
            for pid in tree {
                if self.fg_orig.contains_key(&pid) {
                    continue;
                }
                let Some(h) = open_rw(pid) else { continue };
                let o = read_orig(&h);
                if o.prio.is_none_or(|p| p < Prio::AboveNormal) {
                    let _ = cpu::set_priority(&h, Prio::AboveNormal);
                }
                let _ = cpu::set_eco(&h, Eco::Off);
                self.fg_orig.insert(pid, o);
            }
        }
        if let Some(p) = all.iter().find(|p| p.pid == root) {
            self.known.entry(root).or_insert_with(|| p.name.clone());
        }
    }

    fn hotkey(&mut self, id: i32) {
        let root = procs::foreground_pid();
        if root == 0 || root == self.me {
            return;
        }
        let all = procs::snapshot();
        let name = all.iter().find(|p| p.pid == root).map(|p| p.name.clone()).unwrap_or_default();
        let explorer = name.eq_ignore_ascii_case("explorer.exe");
        // our own foreground boost must not "restore" over what the user just asked for
        for pid in Engine::fg_tree_of(&all, root) {
            self.fg_orig.remove(&pid);
            self.damped.remove(&pid);
        }
        let (rep, what) = match id {
            HK_BOOST => (profile::apply(root, &Profile { tree: !explorer, ..Profile::boost() }, false), "boosted"),
            HK_CALM => (profile::apply(root, &Profile { tree: !explorer, ..Profile::calm() }, false), "calmed down"),
            _ => (profile::reset(root, !explorer), "back to normal"),
        };
        if let Some(j) = rep.job {
            self.jobs.push((root, j));
        }
        let msg = format!("{name}: {what}");
        self.log(format!("hotkey: {msg} ({} processes)", rep.pids.len()));
        (self.notify)(&msg);
    }
}

/// "14:03:22"
pub fn local_time() -> String {
    let mut st: windows_sys::Win32::Foundation::SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe { windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut st) };
    format!("{:02}:{:02}:{:02}", st.wHour, st.wMinute, st.wSecond)
}
