// Find your old files again: scan drives / folders once, then filter live in a tree.
// Same off-white look as chroofer, English + German.

#![windows_subsystem = "windows"] // no console window next to the app

mod export;
mod filter;
mod i18n;
mod model;
mod rules;
mod scan;
mod settings;
mod theme;
mod tree;
mod ui_side;
mod ui_tree;
mod win;

use eframe::egui::{self, Align, Key, Layout, Modifiers, RichText};
use filter::{DateFilter, FilterParams, FilterResult, Filterer, View, OLDER_MONTHS, SIZE_STEPS};
use i18n::{fill, LangPref, T};
use model::*;
use scan::Progress;
use settings::Settings;
use std::collections::{HashMap, HashSet};
use std::os::windows::fs::MetadataExt;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use theme::*;
use tree::{NodeKey, Row, RowKind, TreeState};

const NAME: &str = "rustscanner";
/// wait this long after typing before filtering
const TYPE_DELAY: Duration = Duration::from_millis(150);

pub struct Scanning {
    pub progress: Arc<Progress>,
    handle: Option<JoinHandle<Index>>,
    pub started: Instant,
    /// used space of the scanned drives, for a % (only when whole drives are scanned)
    pub used_bytes: Option<u64>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sel {
    File(u32),
    Folder(u32),
}

pub struct Detail {
    pub sel: Sel,
    pub created: i64,
    pub modified: i64,
    pub accessed: i64,
}

#[derive(Default)]
pub struct ExportState {
    pub running: bool,
    pub done: Option<Result<String, String>>,
}

/// things rows / menus want to do, applied after drawing
pub enum Action {
    Toggle(NodeKey),
    Select(Sel, Option<usize>),
    Open(Sel),
    Explorer(Sel),
    Copy(Sel),
    More(NodeKey),
    ShowAnyway(u32),
    HideAgain(u32),
    HideFolder(u32),
    ExpandBelow(u32),
}

pub struct App {
    pub t: &'static T,
    t_pref: LangPref,
    pub s: Settings,
    saved: Settings,
    dirty_since: Option<Instant>,
    pub drives: Arc<Mutex<Vec<win::Drive>>>,
    /// (index into T::known, path)
    pub known: Vec<(usize, String)>,

    pub scan: Option<Scanning>,
    pub index: Option<Arc<Index>>,
    /// locations the current index was scanned with
    pub scanned_locations: Vec<String>,

    filterer: Filterer,
    gen: u64,
    result_gen: u64,
    pub sent: Option<FilterParams>,
    pub result: Option<Arc<FilterResult>>,
    pub search: String,
    search_applied: String,
    exts_applied: String,
    pub typed_at: Option<Instant>,
    pub whitelist_ids: Arc<HashSet<u32>>,
    pub manual_ids: Arc<HashSet<u32>>,

    pub states: HashMap<View, TreeState>,
    pub rows: Vec<Row>,
    pub rows_dirty: bool,
    pub selected: Option<Sel>,
    pub sel_row: Option<usize>,
    pub detail: Option<Detail>,
    pub scroll_to: Option<usize>,
    pub scroll_state: (f32, f32),

    pub export: Arc<Mutex<ExportState>>,
    pub toast: Option<(String, Instant)>,
    pub focus_search: bool,
    pub actions: Vec<Action>,
}

impl App {
    fn new(cc: &eframe::CreationContext) -> Self {
        fonts(&cc.egui_ctx);
        style(&cc.egui_ctx);
        let mut s = settings::load();
        let known: Vec<(usize, String)> = win::Known::ALL
            .iter()
            .enumerate()
            .filter_map(|(i, k)| win::known_folder(*k).map(|p| (i, p)))
            .collect();
        // first start: the user folder (AppData is skipped)
        if s.locations.is_empty() {
            if let Some((_, home)) = known.iter().find(|(i, _)| *i == 6) {
                s.locations.push(home.clone());
            }
        }
        let ctx = cc.egui_ctx.clone();
        let app = Self {
            t: i18n::texts(s.lang.resolve()),
            t_pref: s.lang,
            exts_applied: s.exts.clone(),
            saved: s.clone(),
            s,
            dirty_since: None,
            drives: Arc::default(),
            known,
            scan: None,
            index: None,
            scanned_locations: Vec::new(),
            filterer: Filterer::new(move || ctx.request_repaint()),
            gen: 0,
            result_gen: 0,
            sent: None,
            result: None,
            search: String::new(),
            search_applied: String::new(),
            typed_at: None,
            whitelist_ids: Arc::default(),
            manual_ids: Arc::default(),
            states: HashMap::new(),
            rows: Vec::new(),
            rows_dirty: true,
            selected: None,
            sel_row: None,
            detail: None,
            scroll_to: None,
            scroll_state: (0.0, 0.0),
            export: Arc::default(),
            toast: None,
            focus_search: false,
            actions: Vec::new(),
        };
        app.load_drives(&cc.egui_ctx);
        app
    }

    // ---------------- locations ----------------

    pub fn load_drives(&self, ctx: &egui::Context) {
        let (drives, ctx) = (self.drives.clone(), ctx.clone());
        std::thread::spawn(move || {
            *drives.lock().unwrap() = win::drives();
            ctx.request_repaint();
        });
    }

    pub fn has_location(&self, p: &str) -> bool {
        self.s.locations.iter().any(|l| same_path(l, p))
    }

    pub fn set_location(&mut self, p: &str, on: bool) {
        if on {
            if !self.has_location(p) {
                self.s.locations.push(p.to_string());
            }
        } else {
            self.s.locations.retain(|l| !same_path(l, p));
        }
    }

    pub fn locations_changed(&self) -> bool {
        let norm = |v: &[String]| {
            let mut v: Vec<String> = v.iter().map(|l| l.trim_end_matches('\\').to_lowercase()).collect();
            v.sort();
            v
        };
        norm(&self.s.locations) != norm(&self.scanned_locations)
    }

    // ---------------- scan ----------------

    pub fn start_scan(&mut self, ctx: &egui::Context) {
        if self.scan.is_some() {
            return;
        }
        let (roots, _) = scan::normalize_roots(&self.s.locations);
        if roots.is_empty() {
            return;
        }
        let used_bytes = if !self.s.skip_system && roots.iter().all(|r| r.len() == 3) {
            let drives = self.drives.lock().unwrap();
            let used: Option<Vec<u64>> = roots
                .iter()
                .map(|r| drives.iter().find(|d| d.root.eq_ignore_ascii_case(r)).map(|d| d.total.saturating_sub(d.free)))
                .collect();
            used.map(|u| u.iter().sum())
        } else {
            None
        };
        let progress = Arc::new(Progress::default());
        let opts = scan::Options { roots, skip_system: self.s.skip_system, threads: scan::default_threads() };
        let (p2, ctx2) = (progress.clone(), ctx.clone());
        let handle = std::thread::spawn(move || {
            let idx = scan::scan(&opts, &p2);
            ctx2.request_repaint();
            idx
        });
        // free the old index right away, a full drive can be a few hundred MB
        self.index = None;
        self.result = None;
        self.rows.clear();
        self.scanned_locations = self.s.locations.clone();
        self.scan = Some(Scanning { progress, handle: Some(handle), started: Instant::now(), used_bytes });
    }

    pub fn cancel_scan(&self) {
        if let Some(sc) = &self.scan {
            sc.progress.cancel.store(true, Ordering::SeqCst);
        }
    }

    fn poll_scan(&mut self, ctx: &egui::Context) {
        let Some(sc) = &mut self.scan else { return };
        if sc.handle.as_ref().is_some_and(|h| h.is_finished()) {
            let idx = sc.handle.take().and_then(|h| h.join().ok());
            self.scan = None;
            if let Some(idx) = idx {
                self.index = Some(Arc::new(idx));
                self.states.clear();
                self.selected = None;
                self.sel_row = None;
                self.detail = None;
                self.rows.clear();
                self.result = None;
                self.sent = None;
                self.resolve_lists();
            }
        } else {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    /// whitelist / manual paths -> folder indices of the current scan
    fn resolve_lists(&mut self) {
        let Some(idx) = &self.index else { return };
        let ids = |list: &[String]| -> Arc<HashSet<u32>> { Arc::new(list.iter().filter_map(|p| idx.find_folder(p)).collect()) };
        self.whitelist_ids = ids(&self.s.whitelist);
        self.manual_ids = ids(&self.s.manual);
    }

    // ---------------- filtering ----------------

    pub fn params(&self) -> FilterParams {
        let s = &self.s;
        FilterParams {
            search: self.search_applied.clone(),
            match_folders: s.match_folders,
            cats: s.cats,
            exts: filter::parse_exts(&self.exts_applied),
            git_only: s.git_only,
            date_field: s.date_field,
            date: match s.date_mode {
                1 => DateFilter::OlderMonths(OLDER_MONTHS[s.older_idx.min(OLDER_MONTHS.len() - 1)]),
                2 => DateFilter::Years(s.year_from, s.year_to),
                _ => DateFilter::Any,
            },
            min_size: if s.size_idx == 0 { 0 } else { SIZE_STEPS[(s.size_idx - 1).min(SIZE_STEPS.len() - 1)] },
            rules: s.rules,
            html_n: s.html_n,
            whitelist: self.whitelist_ids.clone(),
            manual: self.manual_ids.clone(),
            // only Types / Years need their groups built
            view: if matches!(s.view, View::Types | View::Years) { s.view } else { View::Tree },
        }
    }

    pub fn filtering(&self) -> bool {
        self.result_gen != self.gen
    }

    fn refilter(&mut self, ctx: &egui::Context) {
        if let Some(t) = self.typed_at {
            let left = TYPE_DELAY.saturating_sub(t.elapsed());
            if left.is_zero() {
                self.search_applied = self.search.clone();
                self.exts_applied = self.s.exts.clone();
                self.typed_at = None;
            } else {
                ctx.request_repaint_after(left);
            }
        }
        let Some(idx) = &self.index else { return };
        let p = self.params();
        if self.sent.as_ref() != Some(&p) {
            self.gen += 1;
            self.filterer.request(self.gen, idx.clone(), p.clone());
            self.sent = Some(p);
        }
        if let Some((gen, res)) = self.filterer.take() {
            if gen == self.gen {
                self.result = Some(res);
                self.result_gen = gen;
                self.rows_dirty = true;
            }
        }
    }

    fn rebuild_rows(&mut self) {
        let (Some(idx), Some(res)) = (&self.index, &self.result) else { return };
        if !self.rows_dirty {
            return;
        }
        // the groups of Types / Years come with the filter result, wait for the right one
        if matches!(self.s.view, View::Types | View::Years) && res.view != self.s.view {
            return;
        }
        let mut wl: Vec<u32> = self.whitelist_ids.iter().copied().collect();
        wl.sort_unstable();
        let st = self.states.entry(self.s.view).or_default();
        self.rows = tree::flatten(idx, res, self.s.view, self.s.sort, st, &wl);
        self.rows_dirty = false;
        // the selection may have moved
        self.sel_row = self.selected.and_then(|s| self.rows.iter().position(|r| row_sel(r) == Some(s)));
    }

    pub fn state(&mut self) -> &mut TreeState {
        self.rows_dirty = true;
        self.states.entry(self.s.view).or_default()
    }

    // ---------------- selection + actions ----------------

    pub fn path_of(&self, sel: Sel) -> Option<String> {
        let idx = self.index.as_ref()?;
        Some(match sel {
            Sel::File(i) => idx.file_path(i),
            Sel::Folder(f) => idx.folder_path(f),
        })
    }

    fn select(&mut self, sel: Sel, row: Option<usize>) {
        self.selected = Some(sel);
        self.sel_row = row.or_else(|| self.rows.iter().position(|r| row_sel(r) == Some(sel)));
        let md = self.path_of(sel).and_then(|p| std::fs::metadata(p).ok());
        self.detail = Some(Detail {
            sel,
            created: md.as_ref().map(|m| filetime_to_unix(m.creation_time())).unwrap_or(0),
            modified: md.as_ref().map(|m| filetime_to_unix(m.last_write_time())).unwrap_or(0),
            accessed: md.as_ref().map(|m| filetime_to_unix(m.last_access_time())).unwrap_or(0),
        });
    }

    fn apply(&mut self, a: Action, ctx: &egui::Context) {
        let t = self.t;
        match a {
            Action::Toggle(key) => self.state().toggle(key),
            Action::Select(sel, row) => self.select(sel, row),
            Action::Open(sel) => {
                if let Some(p) = self.path_of(sel) {
                    if !win::open(&p) {
                        self.toast = Some((t.open_failed.into(), Instant::now()));
                    }
                }
            }
            Action::Explorer(sel) => {
                if let Some(p) = self.path_of(sel) {
                    win::show_in_explorer(&p);
                }
            }
            Action::Copy(sel) => {
                if let Some(p) = self.path_of(sel) {
                    ctx.copy_text(p);
                    self.toast = Some((t.copied.into(), Instant::now()));
                }
            }
            Action::More(key) => {
                let st = self.state();
                let cur = st.more.get(&key).copied().unwrap_or(tree::FILES_PER_FOLDER);
                st.more.insert(key, cur + cur.max(tree::FILES_PER_FOLDER));
            }
            Action::ShowAnyway(f) => {
                if let Some(p) = self.path_of(Sel::Folder(f)) {
                    self.s.manual.retain(|m| !same_path(m, &p));
                    if !self.s.whitelist.iter().any(|w| same_path(w, &p)) {
                        self.s.whitelist.push(p);
                    }
                    self.resolve_lists();
                }
            }
            Action::HideAgain(f) => {
                if let Some(p) = self.path_of(Sel::Folder(f)) {
                    self.s.whitelist.retain(|w| !same_path(w, &p));
                    self.resolve_lists();
                }
            }
            Action::HideFolder(f) => {
                if let Some(p) = self.path_of(Sel::Folder(f)) {
                    self.s.whitelist.retain(|w| !same_path(w, &p));
                    if !self.s.manual.iter().any(|m| same_path(m, &p)) {
                        self.s.manual.push(p);
                    }
                    self.resolve_lists();
                }
            }
            Action::ExpandBelow(f) => {
                if let (Some(idx), Some(res)) = (self.index.clone(), self.result.clone()) {
                    tree::expand_subtree(&idx, &res, f, self.state());
                }
            }
        }
    }

    // ---------------- keyboard ----------------

    fn keyboard(&mut self, ctx: &egui::Context) {
        if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::F)) {
            self.focus_search = true;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F5)) {
            self.start_scan(ctx);
        }
        let typing = ctx.memory(|m| m.focused().is_some());
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            if self.scan.is_some() {
                self.cancel_scan();
            } else if !self.search.is_empty() {
                self.search.clear();
                self.typed_at = Some(Instant::now() - TYPE_DELAY);
            }
            ctx.memory_mut(|m| {
                if let Some(id) = m.focused() {
                    m.surrender_focus(id)
                }
            });
        }
        if typing || self.rows.is_empty() {
            return;
        }
        let pressed = |k: Key| ctx.input_mut(|i| i.consume_key(Modifiers::NONE, k));
        let cur = self.sel_row.filter(|&r| r < self.rows.len() && row_sel(&self.rows[r]) == self.selected);
        let step = |from: Option<usize>, down: bool, rows: &[Row]| -> Option<usize> {
            let mut i = from.map(|f| f as isize).unwrap_or(if down { -1 } else { rows.len() as isize });
            loop {
                i += if down { 1 } else { -1 };
                if i < 0 || i >= rows.len() as isize {
                    return None;
                }
                if row_sel(&rows[i as usize]).is_some() {
                    return Some(i as usize);
                }
            }
        };
        let mut go = None;
        if pressed(Key::ArrowDown) {
            go = step(cur, true, &self.rows);
        }
        if pressed(Key::ArrowUp) {
            go = step(cur, false, &self.rows);
        }
        if let Some(r) = cur {
            let row = self.rows[r];
            let folder = match row.kind {
                RowKind::Folder { key, open, .. } => Some((key, open)),
                _ => None,
            };
            if pressed(Key::ArrowRight) {
                if let Some((key, false)) = folder {
                    self.actions.push(Action::Toggle(key));
                }
            }
            if pressed(Key::ArrowLeft) {
                match folder {
                    Some((key, true)) => self.actions.push(Action::Toggle(key)),
                    // jump to the parent row
                    _ => go = (0..r).rev().find(|&i| self.rows[i].depth < row.depth && row_sel(&self.rows[i]).is_some()),
                }
            }
            if pressed(Key::Enter) {
                match (folder, row_sel(&row)) {
                    (Some((key, _)), _) => self.actions.push(Action::Toggle(key)),
                    (None, Some(sel)) => self.actions.push(Action::Open(sel)),
                    _ => {}
                }
            }
            // egui-winit turns Ctrl+C into a Copy event
            let copy = ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Copy)));
            if copy || ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::C)) {
                if let Some(sel) = row_sel(&row) {
                    self.actions.push(Action::Copy(sel));
                }
            }
        }
        if let Some(r) = go {
            if let Some(sel) = row_sel(&self.rows[r]) {
                self.actions.push(Action::Select(sel, Some(r)));
                self.scroll_to = Some(r);
            }
        }
    }

    // ---------------- header ----------------

    fn header(&mut self, ui: &mut egui::Ui) {
        let t = self.t;
        ui.horizontal(|ui| {
            logo(ui, 32.0);
            ui.add_space(1.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(RichText::new(NAME).size(17.0).family(semibold()).color(RUST));
                ui.label(RichText::new(t.app_by).size(11.5).color(TEXT_3));
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let items = [t.lang_auto, "DE", "EN"];
                let cur = LangPref::ALL.iter().position(|l| *l == self.s.lang).unwrap_or(0);
                let r = ui.scope(|ui| segmented_fit(ui, &items, cur, 11.5, 22.0));
                if let Some(i) = r.inner {
                    self.s.lang = LangPref::ALL[i];
                }
                r.response.on_hover_text(t.lang_auto_tip);
                ui.add_space(4.0);
                if self.scan.is_some() {
                    chip_blue(ui, t.chip_scanning);
                } else if let Some(idx) = &self.index {
                    if self.filtering() {
                        chip_gray(ui, t.chip_updating);
                    }
                    if idx.cancelled {
                        chip_yellow(ui, t.chip_cancelled);
                    }
                    let n = fill(t.chip_files, &[("n", &t.int(idx.files.len() as u64))]);
                    chip_on(ui, &format!("{n} · {}", t.duration(idx.duration)));
                }
            });
        });
    }

    fn start_export(&mut self, ctx: &egui::Context) {
        let (Some(idx), Some(res), Some(params)) = (self.index.clone(), self.result.clone(), self.sent.clone()) else {
            return;
        };
        let t = self.t;
        let stamp = win::local(unix_now()).map(|d| d.stamp()).unwrap_or_default();
        let mut dlg = rfd::FileDialog::new()
            .set_title(t.export)
            .set_file_name(format!("filescan_{stamp}.md"))
            .add_filter("Markdown", &["md"]);
        if let Some(d) = win::known_folder(win::Known::Documents) {
            dlg = dlg.set_directory(d);
        }
        let Some(path) = dlg.save_file() else { return };
        let (state, ctx, view) = (self.export.clone(), ctx.clone(), self.s.view);
        *state.lock().unwrap() = ExportState { running: true, done: None };
        std::thread::spawn(move || {
            let info = export::Info { t, idx: &idx, res: &res, params: &params, view };
            let r = export::write(&path, &info).map(|_| path.to_string_lossy().into_owned()).map_err(|e| e.to_string());
            *state.lock().unwrap() = ExportState { running: false, done: Some(r) };
            ctx.request_repaint();
        });
    }

    fn save_settings_later(&mut self, ctx: &egui::Context) {
        if self.s == self.saved {
            self.dirty_since = None;
            return;
        }
        let since = *self.dirty_since.get_or_insert_with(Instant::now);
        if since.elapsed() >= Duration::from_secs(1) {
            settings::save(&self.s);
            self.saved = self.s.clone();
            self.dirty_since = None;
        } else {
            ctx.request_repaint_after(Duration::from_secs(1));
        }
    }
}

/// which file / folder a row stands for
pub fn row_sel(r: &Row) -> Option<Sel> {
    match r.kind {
        RowKind::File(i) => Some(Sel::File(i)),
        RowKind::Folder { f, .. } | RowKind::Hidden { f, .. } => Some(Sel::Folder(f)),
        _ => None,
    }
}

pub fn same_path(a: &str, b: &str) -> bool {
    a.trim_end_matches('\\').eq_ignore_ascii_case(b.trim_end_matches('\\'))
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.t_pref != self.s.lang {
            self.t_pref = self.s.lang;
            self.t = i18n::texts(self.s.lang.resolve());
            self.rows_dirty = true;
        }
        self.poll_scan(ctx);
        self.keyboard(ctx);
        self.refilter(ctx);
        self.rebuild_rows();

        // drag & drop: folders become locations (a file adds its folder)
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if self.scan.is_none() {
            for p in dropped {
                let dir = if p.is_dir() { Some(p) } else { p.parent().map(|x| x.to_path_buf()) };
                if let Some(d) = dir {
                    self.set_location(&d.to_string_lossy(), true);
                }
            }
        }
        let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());

        egui::TopBottomPanel::top("header")
            .frame(egui::Frame::none().fill(BG).inner_margin(egui::Margin { left: 18.0, right: 18.0, top: 12.0, bottom: 8.0 }))
            .show_separator_line(false)
            .show(ctx, |ui| self.header(ui));

        if self.selected.is_some() && self.index.is_some() && self.scan.is_none() {
            egui::TopBottomPanel::bottom("detail")
                .frame(egui::Frame::none().fill(BG).inner_margin(egui::Margin { left: 8.0, right: 18.0, top: 4.0, bottom: 14.0 }))
                .show_separator_line(false)
                .show(ctx, |ui| self.detail_panel(ui));
        }

        egui::SidePanel::left("side")
            .resizable(true)
            .default_width(350.0)
            .width_range(310.0..=480.0)
            .show_separator_line(false)
            .frame(egui::Frame::none().fill(BG).inner_margin(egui::Margin { left: 18.0, right: 8.0, top: 4.0, bottom: 14.0 }))
            .show(ctx, |ui| self.side_panel(ui, ctx));

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(BG).inner_margin(egui::Margin { left: 8.0, right: 18.0, top: 4.0, bottom: 14.0 }))
            .show(ctx, |ui| self.central(ui, ctx));

        for a in std::mem::take(&mut self.actions) {
            self.apply(a, ctx);
        }
        if self.rows_dirty {
            ctx.request_repaint();
        }

        // overlay while folders are dragged over the window
        if hovering {
            let rect = ctx.screen_rect().shrink(10.0);
            let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("drop")));
            painter.rect(rect, R_LG, egui::Color32::from_rgba_unmultiplied(232, 241, 253, 235), egui::Stroke::new(2.0_f32, ACCENT));
            painter.text(rect.center(), egui::Align2::CENTER_CENTER, self.t.drop_overlay, egui::FontId::new(17.0, semibold()), ACCENT);
        }
        self.save_settings_later(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.cancel_scan();
        settings::save(&self.s);
    }
}

fn main() -> eframe::Result<()> {
    // release builds abort on panic without a window -> leave a note
    std::panic::set_hook(Box::new(|info| {
        if let Some(d) = settings::dir() {
            let _ = std::fs::create_dir_all(&d);
            let bt = std::backtrace::Backtrace::force_capture();
            let _ = std::fs::write(d.join("crash.txt"), format!("{info}\n\n{bt}"));
        }
    }));
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(NAME)
            .with_inner_size([1280.0, 840.0])
            .with_min_inner_size([1000.0, 640.0])
            .with_drag_and_drop(true)
            .with_icon(app_icon()),
        ..Default::default()
    };
    eframe::run_native(NAME, options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
