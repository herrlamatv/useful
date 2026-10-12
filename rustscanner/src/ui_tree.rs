// Middle + bottom: toolbar, the tree (only the rows on screen are drawn), details of the selection,
// the welcome card and the scan progress.

use crate::filter::{DateField, FilterResult, View, UNKNOWN_YEAR};
use crate::i18n::fill;
use crate::model::*;
use crate::rules::INHERITED;
use crate::theme::*;
use crate::tree::{self, RowKind, Sort};
use crate::{row_sel, win, Action, App, Sel};
use eframe::egui::text::{LayoutJob, TextFormat, TextWrapping};
use eframe::egui::{self, Align, Align2, Color32, FontId, Galley, Id, Layout, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

pub const ROW_H: f32 = 24.0;
const INDENT: f32 = 18.0;
const COL_DATE: f32 = 190.0;
const COL_SIZE: f32 = 78.0;
const COL_COUNT: f32 = 64.0;
const ACCENT_DARK: Color32 = Color32::from_rgb(0x0a, 0x5b, 0xb5);

/// x positions of the columns in a row
struct Cols {
    name_end: f32,
    count_r: f32,
    size_r: f32,
    date_l: f32,
}

fn cols(rect: Rect) -> Cols {
    let date_l = rect.right() - 10.0 - COL_DATE;
    let size_r = date_l - 16.0;
    let count_r = size_r - COL_SIZE - 6.0;
    Cols { name_end: count_r - COL_COUNT, count_r, size_r, date_l }
}

fn font(size: f32) -> FontId {
    FontId::proportional(size)
}

fn single_line(job: &mut LayoutJob, max_w: f32) {
    job.wrap = TextWrapping { max_width: max_w.max(8.0), max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
}

fn galley(ui: &Ui, text: &str, f: FontId, color: Color32, max_w: f32) -> Arc<Galley> {
    let mut job = LayoutJob::single_section(text.to_owned(), TextFormat::simple(f, color));
    single_line(&mut job, max_w);
    ui.fonts(|fo| fo.layout_job(job))
}

/// text vertically centered at y, returns its width
fn paint_text(ui: &Ui, x: f32, y: f32, text: &str, f: FontId, color: Color32, max_w: f32) -> f32 {
    let g = galley(ui, text, f, color, max_w);
    let w = g.size().x;
    ui.painter().galley(egui::pos2(x, y - g.size().y / 2.0), g, color);
    w
}

fn paint_right(ui: &Ui, right: f32, y: f32, text: &str, f: FontId, color: Color32) {
    ui.painter().text(egui::pos2(right, y), Align2::RIGHT_CENTER, text, f, color);
}

/// name with the search hit marked
fn name_galley(ui: &Ui, name: &str, hit: Option<std::ops::Range<usize>>, f: FontId, color: Color32, max_w: f32) -> Arc<Galley> {
    let plain = TextFormat::simple(f.clone(), color);
    let mut job = LayoutJob::default();
    match hit {
        Some(r) if r.end <= name.len() && name.is_char_boundary(r.start) && name.is_char_boundary(r.end) => {
            job.append(&name[..r.start], 0.0, plain.clone());
            job.append(
                &name[r.clone()],
                0.0,
                TextFormat { font_id: FontId::new(f.size, semibold()), color: ACCENT_DARK, background: ACCENT_SOFT, ..Default::default() },
            );
            job.append(&name[r.end..], 0.0, plain);
        }
        _ => job.append(name, 0.0, plain),
    }
    single_line(&mut job, max_w);
    ui.fonts(|fo| fo.layout_job(job))
}

/// "C:\Users\me\Projects" with the last part dark, cut at the start when too long ("…\me\Projects")
fn path_galley(ui: &Ui, path: &str, size: f32, max_w: f32, dim: bool) -> Arc<Galley> {
    let path = path.trim_end_matches('\\');
    let (parent, name) = match path.rfind('\\') {
        Some(i) if i + 1 < path.len() => (&path[..=i], &path[i + 1..]),
        _ => ("", path),
    };
    let name_color = if dim { TEXT_2 } else { TEXT };
    let make = |p: &str| {
        let mut job = LayoutJob::default();
        job.append(p, 0.0, TextFormat::simple(font(size), TEXT_3));
        job.append(name, 0.0, TextFormat::simple(font(size), name_color));
        single_line(&mut job, f32::INFINITY);
        ui.fonts(|fo| fo.layout_job(job))
    };
    let mut g = make(parent);
    if g.size().x <= max_w {
        return g;
    }
    let chars: Vec<char> = parent.chars().collect();
    let mut cut = (((g.size().x - max_w) / (size * 0.5)).ceil() as usize + 1).min(chars.len());
    for _ in 0..8 {
        if cut >= chars.len() {
            break;
        }
        let p: String = std::iter::once('…').chain(chars[cut..].iter().copied()).collect();
        g = make(&p);
        if g.size().x <= max_w {
            return g;
        }
        cut += 3 + cut / 4;
    }
    galley(ui, name, font(size), name_color, max_w)
}

impl App {
    pub(crate) fn central(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        if self.scan.is_some() {
            self.scan_card(ui);
            return;
        }
        let Some(idx) = self.index.clone() else {
            self.welcome(ui, ctx);
            return;
        };
        self.toolbar(ui);
        ui.add_space(2.0);
        self.second_bar(ui, ctx, &idx);
        ui.add_space(6.0);
        let res = self.result.clone();
        panel_flush(ui, |ui| match res {
            Some(res) => self.tree_view(ui, &idx, &res),
            None => {
                ui.add_space(40.0);
                ui.vertical_centered(|ui| {
                    ui.add(egui::Spinner::new().color(TEXT_3));
                    ui.label(RichText::new(self.t.chip_updating).color(TEXT_2));
                });
            }
        });
    }

    /// search + views
    fn toolbar(&mut self, ui: &mut Ui) {
        let t = self.t;
        let hidden_n = self.result.as_ref().map(|r| r.hidden_top.len()).unwrap_or(0) as u64;
        let hidden_label = format!("{} ({})", t.views[4], t.int(hidden_n));
        let items = [t.views[0], t.views[1], t.views[2], t.views[3], hidden_label.as_str()];
        ui.horizontal(|ui| {
            let seg_w = segmented_fit_width(ui, &items, 12.5);
            let id = Id::new("search");
            if self.focus_search {
                ui.memory_mut(|m| m.request_focus(id));
                self.focus_search = false;
            }
            let w = ui.available_width() - seg_w - ui.spacing().item_spacing.x;
            if text_input(ui, id, &mut self.search, t.search_hint, w).on_hover_text("Ctrl+F").changed() {
                self.typed_at = Some(std::time::Instant::now());
            }
            let cur = View::ALL.iter().position(|v| *v == self.s.view).unwrap_or(0);
            if let Some(i) = segmented_fit(ui, &items, cur, 12.5, 26.0) {
                self.s.view = View::ALL[i];
                self.rows_dirty = true;
            }
        });
    }

    /// sort, expand / collapse, summary, export
    fn second_bar(&mut self, ui: &mut Ui, ctx: &egui::Context, idx: &Index) {
        let t = self.t;
        ui.horizontal(|ui| {
            let cur = Sort::ALL.iter().position(|s| *s == self.s.sort).unwrap_or(0);
            if let Some(i) = segmented_fit(ui, &t.sorts, cur, 12.0, 24.0) {
                self.s.sort = Sort::ALL[i];
                self.rows_dirty = true;
            }
            let can_expand = !matches!(self.s.view, View::Hidden);
            if ui.add_enabled(can_expand, small_icon_button("⊞")).on_hover_text(t.expand_all).clicked() {
                if let Some(res) = self.result.clone() {
                    let view = self.s.view;
                    tree::expand_all(idx, &res, view, self.state());
                }
            }
            if ui.add_enabled(can_expand, small_icon_button("⊟")).on_hover_text(t.collapse_all).clicked() {
                tree::collapse_all(self.state());
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let ex = self.export.lock().unwrap().running;
                if btn(ui, t.export, self.result.is_some() && !ex && !self.filtering()).clicked() {
                    self.start_export(ctx);
                }
                let done = self.export.lock().unwrap().done.clone();
                match done {
                    _ if ex => chip_gray(ui, t.ex_writing),
                    Some(Ok(path)) => {
                        if icon_btn(ui, "✕", true).clicked() {
                            self.export.lock().unwrap().done = None;
                        }
                        if btn_small(ui, t.ex_show).on_hover_text(&path).clicked() {
                            win::show_in_explorer(&path);
                        }
                        if btn_small(ui, t.open).clicked() {
                            win::open(&path);
                        }
                        chip_on(ui, t.ex_saved);
                    }
                    Some(Err(e)) => {
                        if icon_btn(ui, "✕", true).clicked() {
                            self.export.lock().unwrap().done = None;
                        }
                        chip_red(ui, &fill(t.ex_failed, &[("e", &e)]));
                    }
                    None => {}
                }
                if let Some((msg, at)) = &self.toast {
                    if at.elapsed() < Duration::from_millis(1600) {
                        chip_blue(ui, msg);
                        ctx.request_repaint_after(Duration::from_millis(200));
                    } else {
                        self.toast = None;
                    }
                }
                if self.filtering() {
                    ui.add(egui::Spinner::new().size(14.0).color(TEXT_3));
                }
                // summary fills the rest from the left
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    if let Some(res) = &self.result {
                        let mut s = fill(
                            t.summary,
                            &[("n", &t.int(res.total.count as u64)), ("s", &t.size(res.total.bytes)), ("f", &t.int(res.folders_hit as u64))],
                        );
                        if res.hidden_files > 0 {
                            s.push_str("  ·  ");
                            s.push_str(&fill(t.summary_hidden, &[("n", &t.int(res.hidden_files))]));
                        }
                        ui.add(egui::Label::new(RichText::new(s).size(12.0).color(TEXT_2)).truncate());
                    }
                });
            });
        });
    }

    fn tree_view(&mut self, ui: &mut Ui, idx: &Arc<Index>, res: &Arc<FilterResult>) {
        let t = self.t;
        // column titles
        let (hdr, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 24.0), Sense::hover());
        let c = cols(hdr);
        let y = hdr.center().y;
        let f = FontId::new(11.0, semibold());
        let date_title = t.date_fields[(res.date_field == DateField::Created) as usize];
        paint_text(ui, hdr.left() + 40.0, y, &t.col_name.to_uppercase(), f.clone(), TEXT_3, 200.0);
        paint_right(ui, c.count_r, y, &t.col_files.to_uppercase(), f.clone(), TEXT_3);
        paint_right(ui, c.size_r, y, &t.size.to_uppercase(), f.clone(), TEXT_3);
        paint_text(ui, c.date_l, y, &date_title.to_uppercase(), f, TEXT_3, COL_DATE);
        ui.painter().hline(hdr.x_range().shrink(4.0), hdr.bottom(), Stroke::new(1.0_f32, LINE_SOFT));
        ui.add_space(2.0);

        let mut area = egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt("tree");
        if let Some(r) = self.scroll_to.take() {
            let (off, h) = self.scroll_state;
            let top = r as f32 * ROW_H;
            if top < off {
                area = area.vertical_scroll_offset(top);
            } else if top + ROW_H > off + h {
                area = area.vertical_scroll_offset(top + ROW_H - h);
            }
        }
        ui.spacing_mut().item_spacing.y = 0.0;
        let n = self.rows.len();
        let out = area.show_rows(ui, ROW_H, n, |ui, range| {
            for i in range {
                self.row_ui(ui, idx, res, i);
            }
        });
        self.scroll_state = (out.state.offset.y, out.inner_rect.height());
    }

    fn row_ui(&mut self, ui: &mut Ui, idx: &Index, res: &FilterResult, i: usize) {
        let t = self.t;
        let row = self.rows[i];
        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW_H), Sense::hover());
        let (a, b) = row.key();
        let id = Id::new(("row", a, b));
        let clickable = !matches!(row.kind, RowKind::Section(_) | RowKind::Note(_));
        let resp = ui.interact(rect, id, if clickable { Sense::click() } else { Sense::hover() });
        let sel = row_sel(&row);
        let selected = sel.is_some() && sel == self.selected && self.sel_row.is_none_or(|r| r == i);
        let p = ui.painter().clone();
        let bg = rect.shrink2(Vec2::new(2.0, 1.0));
        if selected {
            p.rect_filled(bg, 6.0, ACCENT_SOFT);
        } else if resp.hovered() && clickable {
            p.rect_filled(bg, 6.0, SURFACE_2);
        }
        let x0 = rect.left() + 10.0 + row.depth as f32 * INDENT;
        for d in 0..row.depth {
            let x = rect.left() + 16.0 + d as f32 * INDENT;
            p.vline(x, rect.y_range(), Stroke::new(1.0_f32, LINE_SOFT));
        }
        let cy = rect.center().y;
        let c = cols(rect);
        let name_x = x0 + 30.0;
        let date_text = |ts: i64| -> Option<(String, String)> {
            let d = win::local(ts)?;
            Some((d.ymd(), t.age(unix_now() - ts)))
        };
        let paint_date = |ts: i64| {
            if let Some((d, age)) = date_text(ts) {
                let w = paint_text(ui, c.date_l, cy, &d, font(12.5), TEXT_2, COL_DATE);
                paint_text(ui, c.date_l + w + 8.0, cy, &age, font(11.5), TEXT_3, COL_DATE - w - 8.0);
            }
        };
        let chevron = |open: bool| {
            let tt = ui.ctx().animate_bool_with_time(id.with("chev"), open, 0.15);
            paint_chevron(&p, egui::pos2(x0 + 6.0, cy), tt, TEXT_3);
        };
        // a click on the chevron only toggles, a click on the row selects (and toggles folders)
        let clicked = resp.clicked();
        let double = resp.double_clicked();

        match row.kind {
            RowKind::Folder { f, key, open, full_path, count, bytes, date } => {
                chevron(open);
                paint_folder(&p, egui::pos2(x0 + 21.0, cy), open);
                let d = &idx.folders[f as usize];
                let mut badge_w = 0.0;
                if d.flags & F_REPO != 0 {
                    badge_w += 34.0;
                }
                let max_w = c.name_end - name_x - badge_w;
                let g = if full_path {
                    path_galley(ui, &idx.folder_path(f), 13.0, max_w, false)
                } else {
                    galley(ui, idx.folder_name(f), font(13.0), TEXT, max_w)
                };
                let gw = g.size().x;
                p.galley(egui::pos2(name_x, cy - g.size().y / 2.0), g, TEXT);
                if d.flags & F_REPO != 0 {
                    paint_badge(ui, name_x + gw + 8.0, cy, "git", RUST);
                }
                paint_right(ui, c.count_r, cy, &t.int(count as u64), font(12.0), TEXT_3);
                paint_right(ui, c.size_r, cy, &t.size(bytes), font(12.5), TEXT_2);
                paint_date(date);
                if clicked {
                    self.actions.push(Action::Select(Sel::Folder(f), Some(i)));
                    if !double {
                        self.actions.push(Action::Toggle(key));
                    }
                }
            }
            RowKind::File(fi) => {
                let r = &idx.files[fi as usize];
                let cat = (r.cats.trailing_zeros() as usize).min(CAT_COUNT - 1);
                paint_file(&p, egui::pos2(x0 + 21.0, cy), CAT_COLORS[cat]);
                let name = idx.file_name(fi);
                let hit = self.sent.as_ref().and_then(|s| crate::filter::Matcher::new(&s.search).find(name));
                let cloud_w = if r.flags & FL_CLOUD != 0 { 20.0 } else { 0.0 };
                let g = name_galley(ui, name, hit, font(13.0), TEXT, c.name_end - name_x - cloud_w);
                let gw = g.size().x;
                p.galley(egui::pos2(name_x, cy - g.size().y / 2.0), g, TEXT);
                if r.flags & FL_CLOUD != 0 {
                    paint_cloud(&p, egui::pos2(name_x + gw + 10.0, cy), Color32::from_rgb(0x5a, 0x9b, 0xe6));
                }
                paint_right(ui, c.size_r, cy, &t.size(r.size), font(12.5), TEXT_2);
                let ts = if res.date_field == DateField::Created { r.ctime } else { r.mtime };
                paint_date(ts);
                if double {
                    self.actions.push(Action::Open(Sel::File(fi)));
                } else if clicked {
                    self.actions.push(Action::Select(Sel::File(fi), Some(i)));
                }
            }
            RowKind::Group { gi, open } => {
                let Some(g) = res.groups.get(gi as usize) else { return };
                chevron(open);
                let (title, color) = match res.view {
                    View::Types => (t.cats[g.key as usize].to_string(), Some(CAT_COLORS[g.key as usize])),
                    _ if g.key == UNKNOWN_YEAR => (t.unknown_year.to_string(), None),
                    _ => (g.key.to_string(), None),
                };
                let mut x = x0 + 16.0;
                if let Some(col) = color {
                    p.circle_filled(egui::pos2(x + 5.0, cy), 5.0, col);
                    x += 16.0;
                }
                paint_text(ui, x, cy, &title, FontId::new(14.0, semibold()), TEXT, c.name_end - x);
                paint_right(ui, c.count_r, cy, &t.int(g.files.len() as u64), font(12.0), TEXT_3);
                paint_right(ui, c.size_r, cy, &t.size(g.bytes), font(12.5), TEXT_2);
                if clicked {
                    self.actions.push(Action::Toggle(tree::NodeKey::Group(g.key)));
                }
            }
            RowKind::More { key, hidden } => {
                let text = fill(t.more, &[("n", &t.int(hidden as u64))]);
                let col = if resp.hovered() { ACCENT_DARK } else { ACCENT };
                paint_text(ui, name_x, cy, &text, font(12.5), col, c.name_end - name_x);
                if clicked {
                    self.actions.push(Action::More(key));
                }
            }
            RowKind::Hidden { f, shown_anyway } => {
                let d = &idx.folders[f as usize];
                paint_folder(&p, egui::pos2(x0 + 21.0, cy), false);
                // reason chip right of the path
                let reason_bits = if shown_anyway { 0 } else { res.hide[f as usize] & !INHERITED };
                let reason = if reason_bits != 0 { t.rule_name(reason_bits, self.s.html_n) } else { String::new() };
                let rg = galley(ui, &reason, FontId::new(11.0, semibold()), TEXT_2, 220.0);
                let chip_w = if reason.is_empty() { 0.0 } else { rg.size().x + 16.0 };
                let max_w = c.name_end - name_x - chip_w - 8.0;
                let g = path_galley(ui, &idx.folder_path(f), 13.0, max_w, true);
                let gw = g.size().x;
                p.galley(egui::pos2(name_x, cy - g.size().y / 2.0), g, TEXT);
                if !reason.is_empty() {
                    let r = Rect::from_min_size(egui::pos2(name_x + gw + 8.0, cy - 9.0), Vec2::new(chip_w, 18.0));
                    p.rect_filled(r, 999.0, BG_TINT);
                    p.galley(egui::pos2(r.left() + 8.0, cy - rg.size().y / 2.0), rg, TEXT_2);
                }
                paint_right(ui, c.count_r, cy, &t.int(d.sub_files as u64), font(12.0), TEXT_3);
                paint_right(ui, c.size_r, cy, &t.size(d.sub_bytes), font(12.5), TEXT_2);
                let label = if shown_anyway { t.hide_again } else { t.show_anyway };
                let bw = ui.fonts(|fo| fo.layout_no_wrap(label.to_string(), font(11.5), TEXT).size().x) + 24.0;
                let br = Rect::from_min_size(egui::pos2(c.date_l, cy - 10.0), Vec2::new(bw, 20.0));
                let b = ui.put(
                    br,
                    egui::Button::new(RichText::new(label).size(11.5)).fill(SURFACE).stroke(Stroke::new(1.0_f32, LINE)).rounding(999.0),
                );
                if b.clicked() {
                    self.actions.push(if shown_anyway { Action::HideAgain(f) } else { Action::ShowAnyway(f) });
                } else if double {
                    self.actions.push(Action::Explorer(Sel::Folder(f)));
                } else if clicked {
                    self.actions.push(Action::Select(Sel::Folder(f), Some(i)));
                }
            }
            RowKind::Section(s) => {
                let text = if s == 0 { t.sec_hidden } else { t.sec_shown };
                let w = paint_text(ui, rect.left() + 12.0, cy, &text.to_uppercase(), FontId::new(11.0, semibold()), TEXT_3, 400.0);
                if s == 0 && res.hidden_top.is_empty() {
                    paint_text(ui, rect.left() + 24.0 + w, cy, t.no_hidden, font(12.0), TEXT_2, 300.0);
                }
            }
            RowKind::Note(n) => {
                let text = if n == 0 { t.too_many } else { t.nothing };
                p.text(rect.center(), Align2::CENTER_CENTER, text, font(12.5), TEXT_2);
            }
        }

        if let Some(sel) = sel {
            let in_tree = self.s.view == View::Tree;
            resp.context_menu(|ui| {
                ui.set_min_width(210.0);
                if ui.button(t.open).clicked() {
                    self.actions.push(Action::Open(sel));
                    ui.close_menu();
                }
                if ui.button(t.show_explorer).clicked() {
                    self.actions.push(Action::Explorer(sel));
                    ui.close_menu();
                }
                if ui.button(t.copy_path).clicked() {
                    self.actions.push(Action::Copy(sel));
                    ui.close_menu();
                }
                if let Sel::Folder(f) = sel {
                    ui.separator();
                    if in_tree && ui.button(t.expand_below).clicked() {
                        self.actions.push(Action::ExpandBelow(f));
                        ui.close_menu();
                    }
                    if self.whitelist_ids.contains(&f) {
                        if ui.button(t.hide_again).clicked() {
                            self.actions.push(Action::HideAgain(f));
                            ui.close_menu();
                        }
                    } else if res.hide[f as usize] & !INHERITED != 0 && res.hide[f as usize] & INHERITED == 0 {
                        if ui.button(t.show_anyway).clicked() {
                            self.actions.push(Action::ShowAnyway(f));
                            ui.close_menu();
                        }
                    } else if !idx.is_root(f) && res.hide[f as usize] == 0 && ui.button(t.hide_folder).clicked() {
                        self.actions.push(Action::HideFolder(f));
                        ui.close_menu();
                    }
                }
            });
        }
    }

    pub(crate) fn detail_panel(&mut self, ui: &mut Ui) {
        let (Some(idx), Some(sel)) = (self.index.clone(), self.selected) else { return };
        let t = self.t;
        let res = self.result.clone();
        let Some(path) = self.path_of(sel) else { return };
        panel(ui, |ui| {
            ui.horizontal(|ui| {
                let (icon_rect, _) = ui.allocate_exact_size(Vec2::new(22.0, 22.0), Sense::hover());
                let name = match sel {
                    Sel::File(i) => {
                        let r = &idx.files[i as usize];
                        let cat = (r.cats.trailing_zeros() as usize).min(CAT_COUNT - 1);
                        paint_file(ui.painter(), icon_rect.center(), CAT_COLORS[cat]);
                        idx.file_name(i).to_string()
                    }
                    Sel::Folder(f) => {
                        paint_folder(ui.painter(), icon_rect.center(), false);
                        idx.folder_name(f).to_string()
                    }
                };
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if btn(ui, t.copy_path, true).clicked() {
                        self.actions.push(Action::Copy(sel));
                    }
                    if btn(ui, t.show_explorer, true).clicked() {
                        self.actions.push(Action::Explorer(sel));
                    }
                    if btn(ui, t.open, true).clicked() {
                        self.actions.push(Action::Open(sel));
                    }
                    if let (Sel::Folder(f), Some(res)) = (sel, &res) {
                        let h = res.hide[f as usize];
                        if self.whitelist_ids.contains(&f) {
                            if btn(ui, t.hide_again, true).clicked() {
                                self.actions.push(Action::HideAgain(f));
                            }
                        } else if h != 0 && h & INHERITED == 0 {
                            if btn(ui, t.show_anyway, true).clicked() {
                                self.actions.push(Action::ShowAnyway(f));
                            }
                        } else if h == 0 && !idx.is_root(f) && btn(ui, t.hide_folder, true).clicked() {
                            self.actions.push(Action::HideFolder(f));
                        }
                    }
                    ui.with_layout(Layout::top_down(Align::Min), |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.add(egui::Label::new(RichText::new(&name).size(14.0).family(semibold())).truncate());
                        let w = ui.available_width();
                        let g = path_galley(ui, &path, 12.0, w, true);
                        ui.label(g);
                    });
                });
            });
            ui.add_space(2.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                let date = |ts: i64| win::local(ts).map(|d| format!("{} · {}", d.ymd_hm(), t.age(unix_now() - ts)));
                match sel {
                    Sel::File(i) => {
                        let r = &idx.files[i as usize];
                        chip_gray(ui, &t.size(r.size));
                        let cat = (r.cats.trailing_zeros() as usize).min(CAT_COUNT - 1);
                        chip(ui, t.cats[cat], CAT_COLORS[cat].gamma_multiply(0.14), CAT_COLORS[cat]);
                        if r.flags & FL_CLOUD != 0 {
                            chip_blue(ui, t.cloud);
                        }
                    }
                    Sel::Folder(f) => {
                        let d = &idx.folders[f as usize];
                        chip_gray(ui, &fill(t.d_files, &[("n", &t.int(d.sub_files as u64)), ("s", &t.size(d.sub_bytes))]));
                        if let Some(res) = &res {
                            let h = res.hide[f as usize] & !INHERITED;
                            if h != 0 {
                                chip_yellow(ui, &fill(t.hidden_because, &[("r", &t.rule_name(h, self.s.html_n))]));
                            }
                        }
                    }
                }
                if let Some(dt) = self.detail.as_ref().filter(|d| d.sel == sel) {
                    for (label_text, ts) in [(t.date_fields[0], dt.modified), (t.date_fields[1], dt.created), (t.d_accessed, dt.accessed)] {
                        if let Some(s) = date(ts) {
                            ui.label(RichText::new(format!("{label_text}:")).size(11.5).color(TEXT_3));
                            ui.label(RichText::new(s).size(11.5).color(TEXT_2));
                            ui.add_space(6.0);
                        }
                    }
                }
            });
        });
    }

    fn welcome(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let t = self.t;
        panel(ui, |ui| {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| {
                logo(ui, 64.0);
                ui.add_space(10.0);
                ui.label(RichText::new(t.welcome_title).size(22.0).family(semibold()));
                ui.add_space(2.0);
                ui.set_max_width(520.0);
                ui.label(RichText::new(t.welcome_sub).size(13.0).color(TEXT_2));
                ui.add_space(14.0);
                if btn_primary(ui, t.scan, 220.0, !self.s.locations.is_empty()).clicked() {
                    self.start_scan(ctx);
                }
                if self.s.locations.is_empty() {
                    hint(ui, t.no_location);
                }
                ui.add_space(18.0);
            });
            ui.vertical_centered(|ui| {
                ui.set_max_width(520.0);
                list(ui, |ui| {
                    for (i, tip) in t.welcome_tips.iter().enumerate() {
                        item(ui, i == 0, tip, "", |_| {});
                    }
                });
            });
            ui.add_space(30.0);
        });
    }

    fn scan_card(&mut self, ui: &mut Ui) {
        let t = self.t;
        let Some(sc) = &self.scan else { return };
        let pr = &sc.progress;
        let folders = pr.folders.load(Ordering::Relaxed);
        let files = pr.files.load(Ordering::Relaxed);
        let bytes = pr.bytes.load(Ordering::Relaxed);
        let denied = pr.denied.load(Ordering::Relaxed);
        let current = pr.current.lock().unwrap().clone();
        let elapsed = sc.started.elapsed();
        let pct = sc.used_bytes.filter(|&u| u > 0).map(|u| (bytes as f32 / u as f32).min(0.99));
        let cancelling = pr.cancel.load(Ordering::Relaxed);
        let mut cancel = false;
        panel(ui, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new().size(18.0).color(ACCENT));
                ui.label(RichText::new(t.scanning_title).size(17.0).family(semibold()));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    chip_gray(ui, &t.duration(elapsed));
                    if let Some(p) = pct {
                        chip_blue(ui, &format!("{:.0} %", p * 100.0));
                    }
                });
            });
            ui.add_space(6.0);
            let stats = fill(t.rule_stat, &[("f", &t.int(folders)), ("n", &t.int(files)), ("s", &t.size(bytes))]);
            ui.label(RichText::new(stats).size(14.0));
            if denied > 0 {
                hint(ui, &fill(t.denied, &[("n", &t.int(denied))]));
            }
            ui.add_space(4.0);
            progress(ui, pct);
            ui.add_space(4.0);
            let w = ui.available_width();
            let g = path_galley(ui, &current, 12.0, w, true);
            ui.label(g);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if !cancelling && btn_danger(ui, t.cancel, 140.0).on_hover_text("Esc").clicked() {
                    cancel = true;
                }
                hint(ui, t.scan_hint);
            });
            ui.add_space(4.0);
        });
        if cancel {
            self.cancel_scan();
        }
    }
}

fn small_icon_button(text: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text.to_string()).size(15.0).color(TEXT_2))
        .fill(SURFACE)
        .stroke(Stroke::new(1.0_f32, LINE))
        .rounding(8.0)
        .min_size(Vec2::new(30.0, 26.0))
}

/// tiny colored label next to a name ("git")
fn paint_badge(ui: &Ui, x: f32, cy: f32, text: &str, color: Color32) {
    let g = galley(ui, text, FontId::new(10.0, semibold()), color, 80.0);
    let r = Rect::from_min_size(Pos2::new(x, cy - 8.0), Vec2::new(g.size().x + 10.0, 16.0));
    ui.painter().rect_filled(r, 999.0, color.gamma_multiply(0.12));
    ui.painter().galley(egui::pos2(r.left() + 5.0, cy - g.size().y / 2.0), g, color);
}
