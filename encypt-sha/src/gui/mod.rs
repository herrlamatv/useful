// The encsha window: header with tabs, every tab lives in its own file and keeps its own state.
// Slow work (files, Argon2, RSA key generation) runs on a thread as a Job, so the window never hangs.

mod tab_crypt;
mod tab_encode;
mod tab_hash;
mod tab_keys;
mod tab_pw;
mod tab_tools;
pub mod theme;

use eframe::egui::{self, Align, Layout, RichText, Ui};
use encsha::util::{fmt_bytes, Progress, Res, CANCELLED};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use theme::*;

pub const NAME: &str = "encsha";

/// German or English text
pub fn t(de: bool, german: &'static str, english: &'static str) -> &'static str {
    if de {
        german
    } else {
        english
    }
}

fn settings_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("encsha"))
}

/// the language picked last time, else the Windows display language
fn load_german() -> bool {
    match settings_dir().and_then(|d| std::fs::read_to_string(d.join("lang")).ok()).as_deref().map(str::trim) {
        Some("de") => true,
        Some("en") => false,
        _ => {
            let lang = unsafe { windows_sys::Win32::Globalization::GetUserDefaultUILanguage() };
            lang & 0x3ff == 0x07
        }
    }
}

fn save_german(de: bool) {
    if let Some(d) = settings_dir() {
        let _ = std::fs::create_dir_all(&d);
        let _ = std::fs::write(d.join("lang"), if de { "de" } else { "en" });
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Hash,
    Crypt,
    Keys,
    Pw,
    Encode,
    Tools,
}

/// what every tab may touch besides its own state
pub struct Shared {
    pub de: bool,
    pub toast: Option<(String, bool, Instant)>,
}

impl Shared {
    pub fn toast(&mut self, text: impl Into<String>, ok: bool) {
        self.toast = Some((text.into(), ok, Instant::now()));
    }
}

/// work on a thread with progress + cancel
pub struct Job<T> {
    pub prog: Arc<Progress>,
    handle: Option<JoinHandle<Res<T>>>,
    pub started: Instant,
}

impl<T: Send + 'static> Job<T> {
    pub fn start(work: impl FnOnce(&Progress) -> Res<T> + Send + 'static) -> Job<T> {
        let prog = Arc::new(Progress::default());
        let p = prog.clone();
        let handle = std::thread::spawn(move || work(&p));
        Job { prog, handle: Some(handle), started: Instant::now() }
    }

    /// the result once the thread is done
    pub fn poll(&mut self) -> Option<Res<T>> {
        if self.handle.as_ref().is_some_and(|h| h.is_finished()) {
            let h = self.handle.take().unwrap();
            return Some(h.join().unwrap_or_else(|_| Err("crashed".into())));
        }
        None
    }
}

/// poll a job slot: Some(result) when it just finished; shows nothing
pub fn finished<T: Send + 'static>(job: &mut Option<Job<T>>) -> Option<Res<T>> {
    let r = job.as_mut()?.poll();
    if r.is_some() {
        *job = None;
    }
    r
}

/// progress bar + numbers + cancel for a running job
pub fn job_view<T: Send + 'static>(ui: &mut Ui, de: bool, job: &Job<T>, what: &str) {
    let p = &job.prog;
    progress(ui, p.fraction());
    ui.horizontal(|ui| {
        let done = p.done.load(Ordering::Relaxed);
        let secs = job.started.elapsed().as_secs_f64().max(0.001);
        let text = if p.total.load(Ordering::Relaxed) > 0 {
            format!("{what} · {} / {} · {}/s", fmt_bytes(done), fmt_bytes(p.total.load(Ordering::Relaxed)), fmt_bytes((done as f64 / secs) as u64))
        } else {
            format!("{what} …")
        };
        ui.label(RichText::new(text).size(12.0).color(TEXT_2));
        if btn_small(ui, t(de, "Abbrechen", "Cancel")).clicked() {
            p.cancel.store(true, Ordering::Relaxed);
        }
    });
    ui.ctx().request_repaint_after(Duration::from_millis(100));
}

/// error text in red; "cancelled" in gray
pub fn error_line(ui: &mut Ui, de: bool, e: &str) {
    if e == CANCELLED {
        chip_gray(ui, t(de, "Abgebrochen", "Cancelled"));
    } else {
        ui.add(egui::Label::new(RichText::new(e).size(12.5).color(RED)).wrap());
    }
}

pub fn copy_btn(ui: &mut Ui, s: &mut Shared, text: &str) {
    if btn_small(ui, t(s.de, "Kopieren", "Copy")).clicked() {
        ui.ctx().copy_text(text.to_string());
        s.toast(t(s.de, "In die Zwischenablage kopiert", "Copied to the clipboard"), true);
    }
}

/// path field + "…" button; returns true when the path changed
pub fn file_row(ui: &mut Ui, id: &str, path: &mut String, hint_text: &str, save: bool) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        let w = ui.available_width() - 40.0 - ui.spacing().item_spacing.x;
        changed |= text_input(ui, egui::Id::new(id), path, hint_text, w, false).changed();
        let pick = egui::Button::new(RichText::new("…").size(13.0)).fill(SURFACE).stroke(egui::Stroke::new(1.0_f32, LINE)).rounding(999.0);
        // the default 16 px button padding would make it wider than the 40 px set aside
        ui.spacing_mut().button_padding = egui::vec2(4.0, 4.0);
        if ui.add_sized([40.0, 30.0], pick).clicked() {
            let mut d = rfd::FileDialog::new();
            let cur = Path::new(path.as_str());
            if let Some(dir) = cur.parent().filter(|d| d.is_dir()) {
                d = d.set_directory(dir);
            }
            if let Some(name) = cur.file_name() {
                d = d.set_file_name(name.to_string_lossy());
            }
            if let Some(p) = if save { d.save_file() } else { d.pick_file() } {
                *path = p.to_string_lossy().to_string();
                changed = true;
            }
        }
    });
    changed
}

/// password field with an eye to show it
pub fn password_row(ui: &mut Ui, id: &str, pw: &mut String, show: &mut bool, hint_text: &str) -> egui::Response {
    ui.horizontal(|ui| {
        let w = ui.available_width() - 34.0 - ui.spacing().item_spacing.x;
        let r = text_input(ui, egui::Id::new(id), pw, hint_text, w, !*show);
        let eye = egui::Button::new(RichText::new(if *show { "🙈" } else { "👁" }).size(16.0).color(TEXT_2)).fill(egui::Color32::TRANSPARENT).stroke(egui::Stroke::NONE);
        ui.spacing_mut().button_padding = egui::vec2(4.0, 4.0);
        if ui.add_sized([34.0, 30.0], eye).clicked() {
            *show = !*show;
        }
        r
    })
    .inner
}

/// what a key file is (type + fingerprint), read once per path instead of every frame
#[derive(Default)]
pub struct KeyInfo {
    path: String,
    desc: Option<Result<String, String>>,
    locked: bool,
}

impl KeyInfo {
    pub fn update(&mut self, path: &str, de: bool) -> &mut KeyInfo {
        if path != self.path {
            self.path = path.to_string();
            self.locked = false;
            self.desc = match encsha::keys::read_key_file(Path::new(path)) {
                _ if path.trim().is_empty() => None,
                Err(e) => Some(Err(e)),
                Ok(text) if encsha::keys::is_protected(&text) => {
                    self.locked = true;
                    Some(Ok(t(de, "🔒 privater Schlüssel mit Passwort", "🔒 private key with password").to_string()))
                }
                Ok(text) => Some(encsha::keys::parse_public(&text).map(|k| {
                    let private = text.contains("PRIVATE KEY");
                    format!("{} {} · {}", k.kind_name(), if private { t(de, "privat", "private") } else { t(de, "öffentlich", "public") }, k.fingerprint())
                })),
            };
        }
        self
    }

    pub fn locked(&self) -> bool {
        self.locked
    }

    pub fn chip(&self, ui: &mut Ui) {
        match &self.desc {
            Some(Ok(d)) => chip_blue(ui, d),
            Some(Err(e)) => chip_red(ui, e),
            None => {}
        }
    }
}

/// Explorer with the file selected
pub fn show_in_folder(p: &Path) {
    let _ = std::process::Command::new("explorer.exe").arg(format!("/select,{}", p.display())).spawn();
}

pub struct App {
    pub s: Shared,
    pub tab: Tab,
    hash: tab_hash::HashTab,
    crypt: tab_crypt::CryptTab,
    keys: tab_keys::KeysTab,
    pw: tab_pw::PwTab,
    encode: tab_encode::EncodeTab,
    tools: tab_tools::ToolsTab,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, tab: Option<Tab>, open: Option<PathBuf>) -> App {
        fonts(&cc.egui_ctx);
        style(&cc.egui_ctx);
        let mut app = App {
            s: Shared { de: load_german(), toast: None },
            tab: tab.unwrap_or(Tab::Hash),
            hash: Default::default(),
            crypt: Default::default(),
            keys: Default::default(),
            pw: Default::default(),
            encode: Default::default(),
            tools: Default::default(),
        };
        if let Some(p) = open {
            app.dropped(vec![p]);
        }
        app
    }

    /// files dropped on the window / given on the command line go to the tab that can use them
    fn dropped(&mut self, paths: Vec<PathBuf>) {
        let first = &paths[0];
        let is_esh = encsha::crypt::peek_file(first).is_ok();
        let handled = match self.tab {
            Tab::Crypt => self.crypt.drop(first),
            Tab::Keys => self.keys.drop(first),
            Tab::Tools => self.tools.drop(first),
            _ => false,
        };
        if handled {
            return;
        }
        let name = first.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        let ext = first.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        let sums_file = name.contains("sums") || ["md5", "sha1", "sha224", "sha256", "sha384", "sha512", "sfv", "b3"].contains(&ext.as_str());
        if is_esh {
            self.tab = Tab::Crypt;
            self.crypt.drop(first);
        } else if sums_file && self.tools.drop(first) {
            self.tab = Tab::Tools;
        } else {
            self.tab = Tab::Hash;
            self.hash.drop(paths);
        }
    }

    fn header(&mut self, ui: &mut Ui) {
        let de = self.s.de;
        ui.horizontal(|ui| {
            logo(ui, 30.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(RichText::new(NAME).size(18.0).family(semibold()));
                ui.label(RichText::new(t(de, "Hashen, verschlüsseln, signieren", "Hash, encrypt, sign")).size(11.5).color(TEXT_2));
            });
            ui.add_space(14.0);
            let tabs = [
                (Tab::Hash, "Hash"),
                (Tab::Crypt, t(de, "Verschlüsseln", "Encrypt")),
                (Tab::Keys, t(de, "Schlüssel & Signaturen", "Keys & signatures")),
                (Tab::Pw, t(de, "Passwörter", "Passwords")),
                (Tab::Encode, t(de, "Kodieren", "Encode")),
                (Tab::Tools, t(de, "Werkzeuge", "Tools")),
            ];
            let names: Vec<&str> = tabs.iter().map(|x| x.1).collect();
            let sel = tabs.iter().position(|x| x.0 == self.tab).unwrap_or(0);
            if let Some(i) = segmented_fit(ui, &names, sel, 13.0, 28.0) {
                self.tab = tabs[i].0;
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if let Some(i) = segmented_fit(ui, &["DE", "EN"], if de { 0 } else { 1 }, 12.0, 24.0) {
                    self.s.de = i == 0;
                    save_german(self.s.de);
                }
            });
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("header")
            .frame(egui::Frame::none().fill(BG).inner_margin(egui::Margin { left: 18.0, right: 18.0, top: 12.0, bottom: 8.0 }))
            .show_separator_line(false)
            .show(ctx, |ui| self.header(ui));

        if let Some((text, ok, at)) = &self.s.toast {
            if at.elapsed() > Duration::from_secs(4) {
                self.s.toast = None;
            } else {
                let (text, ok) = (text.clone(), *ok);
                egui::TopBottomPanel::bottom("toast")
                    .frame(egui::Frame::none().fill(BG).inner_margin(egui::Margin::symmetric(18.0, 6.0)))
                    .show_separator_line(false)
                    .show(ctx, |ui| if ok { chip_on(ui, &text) } else { chip_red(ui, &text) });
                ctx.request_repaint_after(Duration::from_millis(500));
            }
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(BG).inner_margin(egui::Margin { left: 18.0, right: 18.0, top: 4.0, bottom: 14.0 }))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    // room for the card shadows
                    ui.add_space(4.0);
                    match self.tab {
                        Tab::Hash => self.hash.show(ui, &mut self.s),
                        Tab::Crypt => self.crypt.show(ui, &mut self.s),
                        Tab::Keys => self.keys.show(ui, &mut self.s),
                        Tab::Pw => self.pw.show(ui, &mut self.s),
                        Tab::Encode => self.encode.show(ui, &mut self.s),
                        Tab::Tools => self.tools.show(ui, &mut self.s),
                    }
                    ui.add_space(8.0);
                });
            });

        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if !dropped.is_empty() {
            self.dropped(dropped);
        }
    }
}
