// The pcmax window: header with tabs, the tabs live in their own files.
// The engine (rules, boost, hotkeys) runs on its own thread, so closing the window to the tray
// keeps everything working.

mod editor;
mod tab_boost;
mod tab_files;
mod tab_monitor;
mod tab_procs;
mod tab_rules;
mod tab_settings;
pub mod task;
pub mod theme;
pub mod tray;

use eframe::egui::{self, Align, Layout, RichText};
use pcmax::engine::{self, Cmd, EngineHandle};
use pcmax::fsx::{Progress, Summary};
use pcmax::monitor::{Monitor, Sample};
use pcmax::procs::{self, CpuSampler};
use pcmax::profile::{self, Profile};
use pcmax::settings::Settings;
use pcmax::topo::{self, Topo};
use std::collections::VecDeque;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use theme::*;

pub const NAME: &str = "pcmax";

/// German or English text
pub fn t(de: bool, german: &'static str, english: &'static str) -> &'static str {
    if de {
        german
    } else {
        english
    }
}

pub fn german(lang: &str) -> bool {
    match lang {
        "de" => true,
        "en" => false,
        _ => pcmax::win::ui_is_german(),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Procs,
    Boost,
    Rules,
    Files,
    Monitor,
    Settings,
}

impl Tab {
    pub fn parse(s: &str) -> Option<Tab> {
        Some(match s.to_ascii_lowercase().as_str() {
            "procs" | "processes" | "prozesse" => Tab::Procs,
            "boost" => Tab::Boost,
            "rules" | "regeln" => Tab::Rules,
            "files" | "extract" | "copy" => Tab::Files,
            "monitor" => Tab::Monitor,
            "settings" | "einstellungen" => Tab::Settings,
            _ => return None,
        })
    }
}

#[derive(Clone)]
pub struct Row {
    pub pid: u32,
    pub name: String,
    pub cpu: f32,
    pub ram: u64,
    pub gpu: f32,
    pub title: String,
}

/// unpack / copy running in the background
pub struct Job {
    pub prog: Arc<Progress>,
    pub handle: Option<JoinHandle<Result<Summary, String>>>,
    pub result: Option<Result<Summary, String>>,
    pub started: Instant,
}

impl Job {
    pub fn start(work: impl FnOnce(&Progress) -> Result<Summary, String> + Send + 'static) -> Job {
        let prog = Arc::new(Progress::default());
        let p = prog.clone();
        let handle = std::thread::spawn(move || work(&p));
        Job { prog, handle: Some(handle), result: None, started: Instant::now() }
    }
    /// true while running
    pub fn poll(&mut self) -> bool {
        if let Some(h) = &self.handle {
            if h.is_finished() {
                let h = self.handle.take().unwrap();
                self.result = Some(h.join().unwrap_or_else(|_| Err("crashed".into())));
            }
        }
        self.handle.is_some()
    }
}

pub enum Pick {
    /// waiting for the mouse button to go up (the click on our own button)
    Release,
    Click,
}

pub struct App {
    pub s: Settings,
    pub de: bool,
    pub tab: Tab,
    pub engine: Option<EngineHandle>,
    pub admin: bool,
    pub topo: &'static Topo,
    // processes + monitor
    pub rows: Vec<Row>,
    pub refreshed: Option<Instant>,
    pub sampler: CpuSampler,
    pub monitor: Option<Monitor>,
    pub sample: Sample,
    pub cpu_hist: VecDeque<f32>,
    pub gpu_hist: VecDeque<f32>,
    pub ram_hist: VecDeque<f32>,
    pub filter: String,
    pub selected: Option<u32>,
    pub sel_state: Option<profile::State>,
    pub editor: Profile,
    pub results: Vec<String>,
    pub confirm_rt: bool,
    pub pick: Option<Pick>,
    pub kept_jobs: Vec<pcmax::job::Job>,
    // rules
    pub rule_open: Option<usize>,
    // files
    pub ex_path: String,
    pub ex_dest: String,
    pub ex_job: Option<Job>,
    pub cp_src: String,
    pub cp_dst: String,
    pub cp_job: Option<Job>,
    // settings tab
    pub shell_state: Option<tab_settings::ShellState>,
    pub shell_loading: Option<JoinHandle<tab_settings::ShellState>>,
    pub toast: Option<(String, bool, Instant)>,
    pub frames: u64,
    /// autostart: hide the window once eframe has shown it (it ignores with_visible(false))
    pub hide_pending: bool,
    /// settings changed, save once nothing changed for a moment
    pub dirty: Option<Instant>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> App {
        fonts(&cc.egui_ctx);
        style(&cc.egui_ctx);
        let s = Settings::load();
        let de = german(&s.lang);
        let admin = pcmax::win::is_elevated();
        let ctx = cc.egui_ctx.clone();
        let eng = engine::start(
            s.clone(),
            Box::new(move |m| {
                tray::notify(m);
                ctx.request_repaint();
            }),
        );
        tray::spawn(cc.egui_ctx.clone(), eng.tx.clone(), eng.boost.clone(), eng.stopped.clone(), de);
        App {
            s,
            de,
            tab: Tab::Procs,
            engine: Some(eng),
            admin,
            topo: topo::get(),
            rows: vec![],
            refreshed: None,
            sampler: CpuSampler::new(),
            monitor: Monitor::new(),
            sample: Sample::default(),
            cpu_hist: VecDeque::new(),
            gpu_hist: VecDeque::new(),
            ram_hist: VecDeque::new(),
            filter: String::new(),
            selected: None,
            sel_state: None,
            editor: Profile::default(),
            results: vec![],
            confirm_rt: false,
            pick: None,
            kept_jobs: vec![],
            rule_open: None,
            ex_path: String::new(),
            ex_dest: String::new(),
            ex_job: None,
            cp_src: String::new(),
            cp_dst: String::new(),
            cp_job: None,
            shell_state: None,
            shell_loading: None,
            toast: None,
            frames: 0,
            hide_pending: false,
            dirty: None,
        }
    }

    pub fn send(&self, c: Cmd) {
        if let Some(e) = &self.engine {
            e.send(c);
        }
    }

    /// save + tell the engine
    pub fn save(&mut self) {
        if let Err(e) = self.s.save() {
            self.toast(e, false);
        }
        self.send(Cmd::Settings(Box::new(self.s.clone())));
    }

    pub fn save_later(&mut self) {
        self.dirty = Some(Instant::now());
    }

    pub fn toast(&mut self, text: impl Into<String>, ok: bool) {
        self.toast = Some((text.into(), ok, Instant::now()));
    }

    pub fn shared(&self) -> Option<std::sync::MutexGuard<'_, engine::Shared>> {
        self.engine.as_ref().map(|e| e.shared.lock().unwrap())
    }

    /// processes + monitor values, once a second while visible
    fn refresh(&mut self) {
        if self.refreshed.is_some_and(|t| t.elapsed() < Duration::from_millis(1000)) {
            return;
        }
        self.refreshed = Some(Instant::now());
        let all = procs::snapshot();
        let cpu = self.sampler.sample(all.iter().map(|p| p.pid));
        if let Some(m) = self.monitor.as_mut() {
            self.sample = m.sample(self.topo.logical as usize);
        }
        let titles = procs::window_titles();
        let mut rows: Vec<Row> = all
            .iter()
            .filter(|p| p.pid > 4)
            .map(|p| Row {
                pid: p.pid,
                name: p.name.clone(),
                cpu: cpu.get(&p.pid).copied().unwrap_or(0.0),
                ram: procs::memory(p.pid).map(|m| m.0).unwrap_or(0),
                gpu: self.sample.gpu_by_pid.get(&p.pid).copied().unwrap_or(0.0),
                title: titles.get(&p.pid).cloned().unwrap_or_default(),
            })
            .collect();
        rows.sort_by(|a, b| b.cpu.total_cmp(&a.cpu).then(b.ram.cmp(&a.ram)));
        self.rows = rows;
        for (h, v) in [
            (&mut self.cpu_hist, self.sample.cpu_total),
            (&mut self.gpu_hist, self.sample.gpu_total),
            (&mut self.ram_hist, if self.sample.ram_total > 0 { self.sample.ram_used as f32 / self.sample.ram_total as f32 * 100.0 } else { 0.0 }),
        ] {
            h.push_back(v);
            while h.len() > 90 {
                h.pop_front();
            }
        }
        if let Some(pid) = self.selected {
            self.sel_state = profile::state(&all, pid);
            if self.sel_state.is_none() {
                self.selected = None;
            }
        }
    }

    fn header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            logo(ui, 30.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(RichText::new(NAME).size(18.0).family(semibold()));
                ui.label(RichText::new(t(self.de, "Mehr oder weniger Leistung pro Programm", "More or less power per program")).size(11.5).color(TEXT_2));
            });
            ui.add_space(14.0);
            let tabs = [
                (Tab::Procs, t(self.de, "Prozesse", "Processes")),
                (Tab::Boost, "Boost"),
                (Tab::Rules, t(self.de, "Regeln", "Rules")),
                (Tab::Files, t(self.de, "Entpacken & Kopieren", "Unpack & copy")),
                (Tab::Monitor, "Monitor"),
                (Tab::Settings, t(self.de, "Einstellungen", "Settings")),
            ];
            let names: Vec<&str> = tabs.iter().map(|x| x.1).collect();
            let sel = tabs.iter().position(|x| x.0 == self.tab).unwrap_or(0);
            if let Some(i) = segmented_fit(ui, &names, sel, 13.0, 28.0) {
                self.tab = tabs[i].0;
                if self.tab == Tab::Settings {
                    self.load_shell_state();
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if self.admin {
                    chip_on(ui, "Admin");
                } else if btn(ui, t(self.de, "🛡 Als Admin neu starten", "🛡 Restart as admin"), true)
                    .on_hover_text(t(
                        self.de,
                        "Für Echtzeit, hohe I/O-Priorität, Dienste, Core-Parking und die Defender-Ausnahme",
                        "For realtime, high I/O priority, services, core parking and the Defender exclusion",
                    ))
                    .clicked()
                {
                    self.restart_as_admin();
                }
                let on = self.shared().is_some_and(|s| s.boost_on);
                if on {
                    chip(ui, "⚡ Boost", RUST, egui::Color32::WHITE);
                }
            });
        });
    }

    pub fn restart_as_admin(&mut self) {
        // the engine must give everything back first, the new process applies it again
        if let Some(e) = self.engine.take() {
            e.stop();
        }
        // the new process needs the single-instance mutex
        let single = crate::SINGLE.lock().unwrap().take();
        if pcmax::win::relaunch_as_admin("--after-restart") {
            tray::remove();
            std::process::exit(0);
        }
        // UAC said no: keep running as before
        *crate::SINGLE.lock().unwrap() = single;
        let eng = engine::start(self.s.clone(), Box::new(tray::notify));
        tray::set_engine(eng.tx.clone(), eng.boost.clone(), eng.stopped.clone());
        self.engine = Some(eng);
        self.toast(t(self.de, "Admin-Rechte abgelehnt", "Admin rights declined"), false);
    }

    pub fn load_shell_state(&mut self) {
        if self.shell_loading.is_none() {
            self.shell_loading = Some(std::thread::spawn(tab_settings::ShellState::read));
        }
    }

    fn poll_picker(&mut self, ctx: &egui::Context) {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_ESCAPE, VK_LBUTTON};
        let Some(p) = &self.pick else { return };
        let down = |vk: u16| unsafe { GetAsyncKeyState(vk as i32) } as u16 & 0x8000 != 0;
        if down(VK_ESCAPE) {
            self.pick = None;
            return;
        }
        match p {
            Pick::Release => {
                if !down(VK_LBUTTON) {
                    self.pick = Some(Pick::Click);
                }
            }
            Pick::Click => {
                if down(VK_LBUTTON) {
                    let pid = procs::pid_at_cursor();
                    self.pick = None;
                    if pid != 0 && pid != procs::own_pid() {
                        self.select(pid);
                        self.tab = Tab::Procs;
                    }
                }
            }
        }
        ctx.request_repaint_after(Duration::from_millis(15));
    }

    pub fn select(&mut self, pid: u32) {
        self.selected = Some(pid);
        self.results.clear();
        self.refreshed = None;
        // start from what fits the process: an existing rule or nothing
        let name = self.rows.iter().find(|r| r.pid == pid).map(|r| r.name.clone()).unwrap_or_default();
        self.editor = self.s.rules.iter().find(|r| r.matches(&name, None)).map(|r| r.profile.clone()).unwrap_or_default();
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frames += 1;
        if self.hide_pending {
            if self.frames >= 2 {
                tray::hide_main();
                self.hide_pending = false;
            } else {
                ctx.request_repaint();
            }
        }
        // close button -> tray (the engine keeps running)
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.s.close_to_tray {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                tray::hide_main();
            } else {
                if let Some(e) = self.engine.take() {
                    e.stop();
                }
                tray::remove();
            }
        }
        self.poll_picker(ctx);
        if self.dirty.is_some_and(|d| d.elapsed() > Duration::from_millis(700)) {
            self.dirty = None;
            self.save();
        }
        if let Some(h) = &self.shell_loading {
            if h.is_finished() {
                let h = self.shell_loading.take().unwrap();
                self.shell_state = h.join().ok();
            }
        }
        if matches!(self.tab, Tab::Procs | Tab::Monitor) {
            self.refresh();
            ctx.request_repaint_after(Duration::from_millis(1000));
        } else {
            ctx.request_repaint_after(Duration::from_millis(500));
        }

        egui::TopBottomPanel::top("header")
            .frame(egui::Frame::none().fill(BG).inner_margin(egui::Margin { left: 18.0, right: 18.0, top: 12.0, bottom: 8.0 }))
            .show_separator_line(false)
            .show(ctx, |ui| self.header(ui));

        if let Some((text, ok, at)) = &self.toast {
            if at.elapsed() > Duration::from_secs(6) {
                self.toast = None;
            } else {
                let (text, ok) = (text.clone(), *ok);
                egui::TopBottomPanel::bottom("toast").frame(egui::Frame::none().fill(BG).inner_margin(egui::Margin::symmetric(18.0, 6.0))).show_separator_line(false).show(ctx, |ui| {
                    if ok {
                        chip_on(ui, &text);
                    } else {
                        chip_red(ui, &text);
                    }
                });
            }
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(BG).inner_margin(egui::Margin { left: 18.0, right: 18.0, top: 4.0, bottom: 14.0 }))
            .show(ctx, |ui| match self.tab {
                Tab::Procs => tab_procs::show(self, ui),
                Tab::Boost => tab_boost::show(self, ui),
                Tab::Rules => tab_rules::show(self, ui),
                Tab::Files => tab_files::show(self, ui),
                Tab::Monitor => tab_monitor::show(self, ui),
                Tab::Settings => tab_settings::show(self, ui),
            });

        if self.confirm_rt {
            tab_procs::confirm_realtime(self, ctx);
        }

        // files dropped on the window: archives -> unpack tab
        let dropped: Vec<std::path::PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if let Some(p) = dropped.into_iter().next() {
            if p.is_dir() {
                self.cp_src = p.to_string_lossy().to_string();
            } else {
                self.ex_path = p.to_string_lossy().to_string();
                self.ex_dest = pcmax::extract::default_dest(&p).to_string_lossy().to_string();
            }
            self.tab = Tab::Files;
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.dirty.take().is_some() {
            let _ = self.s.save();
        }
        if let Some(e) = self.engine.take() {
            e.stop();
        }
        tray::remove();
    }
}
