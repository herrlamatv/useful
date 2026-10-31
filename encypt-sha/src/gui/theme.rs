// Off-white look, taken from chroofer/ui/style.css (same colors, radii and shadows).
// Copied from pcmax (monitor charts removed), logo = padlock, plus text areas / copy / file rows.

use eframe::egui::{
    self, epaint::Shadow, Color32, FontData, FontDefinitions, FontFamily, FontId, Id, Margin, Rect, Response, RichText,
    Rounding, Sense, Stroke, TextStyle, Ui, Vec2,
};

pub const BG: Color32 = Color32::from_rgb(0xf3, 0xef, 0xe5);
pub const BG_TINT: Color32 = Color32::from_rgb(0xeb, 0xe5, 0xd7);
pub const SURFACE: Color32 = Color32::from_rgb(0xff, 0xfd, 0xf8);
pub const SURFACE_2: Color32 = Color32::from_rgb(0xfa, 0xf6, 0xec);
pub const LINE: Color32 = Color32::from_rgb(0xe7, 0xdf, 0xd0);
pub const LINE_SOFT: Color32 = Color32::from_rgb(0xef, 0xe9, 0xdc);
pub const TEXT: Color32 = Color32::from_rgb(0x1f, 0x1d, 0x18);
pub const TEXT_2: Color32 = Color32::from_rgb(0x7c, 0x74, 0x66);
pub const TEXT_3: Color32 = Color32::from_rgb(0xa2, 0x9a, 0x8a);
pub const ACCENT: Color32 = Color32::from_rgb(0x00, 0x71, 0xe3);
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(0xe8, 0xf1, 0xfd);
pub const GREEN: Color32 = Color32::from_rgb(0x34, 0xc7, 0x59);
pub const ORANGE: Color32 = Color32::from_rgb(0xff, 0x95, 0x00);
pub const RED: Color32 = Color32::from_rgb(0xe5, 0x48, 0x4d);
pub const TRACK: Color32 = Color32::from_rgb(0xde, 0xd7, 0xc7);
/// the red/orange of rust-lang
pub const RUST: Color32 = Color32::from_rgb(0xce, 0x42, 0x2b);

pub const R_LG: f32 = 18.0;
pub const R_MD: f32 = 12.0;
pub const R_SM: f32 = 9.0;

pub fn semibold() -> FontFamily {
    FontFamily::Name("semibold".into())
}

/// Segoe UI like in chroofer (falls back to the egui font if it's missing), Cascadia / Consolas for hashes
pub fn fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let dir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    let load = |f: &str| std::fs::read(format!("{dir}\\Fonts\\{f}")).ok();

    if let Some(regular) = load("segoeui.ttf") {
        fonts.font_data.insert("segoe".into(), FontData::from_owned(regular));
        fonts.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "segoe".into());
    }
    // symbols (🔒 ✓ …) that Segoe UI doesn't have
    if let Some(sym) = load("seguisym.ttf") {
        fonts.font_data.insert("segoe-symbol".into(), FontData::from_owned(sym));
        fonts.families.get_mut(&FontFamily::Proportional).unwrap().push("segoe-symbol".into());
    }
    if let Some(mono) = load("CascadiaMono.ttf").or_else(|| load("consola.ttf")) {
        fonts.font_data.insert("mono".into(), FontData::from_owned(mono));
        fonts.families.get_mut(&FontFamily::Monospace).unwrap().insert(0, "mono".into());
    }
    let mut semi = fonts.families[&FontFamily::Proportional].clone();
    if let Some(bold) = load("seguisb.ttf") {
        fonts.font_data.insert("segoe-semibold".into(), FontData::from_owned(bold));
        semi.insert(0, "segoe-semibold".into());
    }
    fonts.families.insert(semibold(), semi);
    ctx.set_fonts(fonts);
}

pub fn style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (TextStyle::Heading, FontId::new(17.0, semibold())),
        (TextStyle::Body, FontId::proportional(13.5)),
        (TextStyle::Button, FontId::proportional(13.0)),
        (TextStyle::Small, FontId::proportional(11.5)),
        (TextStyle::Monospace, FontId::monospace(12.5)),
    ]
    .into();
    style.spacing.item_spacing = Vec2::new(10.0, 8.0);
    style.spacing.button_padding = Vec2::new(16.0, 6.0);
    style.spacing.scroll.bar_width = 6.0;
    style.spacing.menu_margin = Margin::same(6.0);

    let v = &mut style.visuals;
    *v = egui::Visuals::light();
    v.panel_fill = BG;
    v.window_fill = SURFACE;
    v.extreme_bg_color = SURFACE;
    v.faint_bg_color = SURFACE_2;
    v.override_text_color = Some(TEXT);
    v.hyperlink_color = ACCENT;
    v.selection.bg_fill = ACCENT_SOFT;
    v.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    v.window_rounding = Rounding::same(R_MD);
    v.menu_rounding = Rounding::same(R_MD);
    v.window_stroke = Stroke::new(1.0_f32, LINE);
    v.window_shadow =
        Shadow { offset: Vec2::new(0.0, 6.0), blur: 22.0, spread: 0.0, color: Color32::from_rgba_unmultiplied(74, 62, 40, 30) };
    v.popup_shadow = v.window_shadow;

    let w = &mut v.widgets;
    for (wv, fill) in [
        (&mut w.noninteractive, SURFACE),
        (&mut w.inactive, SURFACE),
        (&mut w.hovered, SURFACE_2),
        (&mut w.active, SURFACE_2),
        (&mut w.open, SURFACE_2),
    ] {
        wv.bg_fill = fill;
        wv.weak_bg_fill = fill;
        wv.bg_stroke = Stroke::new(1.0_f32, LINE);
        wv.fg_stroke = Stroke::new(1.0_f32, TEXT);
        wv.rounding = Rounding::same(999.0);
        wv.expansion = 0.0;
    }
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, LINE_SOFT);
    // bg_fill is the slider rail (buttons use weak_bg_fill); no trailing fill, egui adds the
    // corner radius (999 here) to its end
    w.inactive.bg_fill = TRACK;
    style.spacing.slider_width = 220.0;
    // egui 0.29 keeps a style per theme and eframe follows the Windows theme after the first frame:
    // set_style() alone would only change the theme active right now (sliders, combo boxes looked default)
    ctx.set_theme(egui::Theme::Light);
    ctx.set_style_of(egui::Theme::Light, style.clone());
    ctx.set_style_of(egui::Theme::Dark, style);
}

fn card_shadow() -> Shadow {
    Shadow { offset: Vec2::new(0.0, 6.0), blur: 22.0, spread: 0.0, color: Color32::from_rgba_unmultiplied(74, 62, 40, 16) }
}

/// white card with border + soft shadow (.panel)
pub fn panel(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    egui::Frame::none()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0_f32, LINE))
        .rounding(R_LG)
        .inner_margin(Margin::symmetric(16.0, 14.0))
        .shadow(card_shadow())
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            // ui.columns() hands out justified layouts: labels would be spread out like block text
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), add);
        });
}

/// panel title + gray text below (.panel-head)
pub fn panel_head(ui: &mut Ui, title: &str, sub: &str) {
    ui.label(RichText::new(title).size(15.0).family(semibold()));
    ui.add_space(-6.0);
    ui.label(RichText::new(sub).size(12.0).color(TEXT_2));
}

/// small uppercase gray label (.label)
pub fn label(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text.to_uppercase()).size(11.0).family(semibold()).color(TEXT_3));
}

/// gray hint text (.hint)
pub fn hint(ui: &mut Ui, text: &str) {
    ui.add(egui::Label::new(RichText::new(text).size(11.5).color(TEXT_2)).wrap());
}

/// thin separator line (.sep)
pub fn sep(ui: &mut Ui) {
    ui.add_space(2.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, LINE_SOFT);
    ui.add_space(2.0);
}

/// iOS like segmented control (.seg) over the full width, returns the new index if clicked
pub fn segmented(ui: &mut Ui, items: &[&str], selected: usize, enabled: bool) -> Option<usize> {
    let mut clicked = None;
    egui::Frame::none().fill(BG_TINT).rounding(10.0).inner_margin(Margin::same(2.0)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.x = 2.0;
        let w = (ui.available_width() - 2.0 * (items.len() as f32 - 1.0)) / items.len() as f32;
        ui.horizontal(|ui| {
            for (i, item) in items.iter().enumerate() {
                if seg_button(ui, item, i == selected, enabled, Vec2::new(w, 26.0), 12.5).clicked() && i != selected {
                    clicked = Some(i);
                }
            }
        });
    });
    clicked
}

fn fit_widths(ui: &Ui, items: &[&str], size: f32) -> Vec<f32> {
    // measured semibold, so the segments don't jump when the selection changes
    items
        .iter()
        .map(|it| ui.fonts(|f| f.layout_no_wrap(it.to_string(), FontId::new(size, semibold()), TEXT).size().x) + size * 1.6)
        .collect()
}

/// width segmented_fit() will take
pub fn segmented_fit_width(ui: &Ui, items: &[&str], size: f32) -> f32 {
    fit_widths(ui, items, size).iter().sum::<f32>() + 2.0 * (items.len() as f32 - 1.0) + 4.0
}

/// segmented control where every segment is as wide as its text (toolbar, language switch).
/// Always left-to-right, also inside right-to-left layouts.
pub fn segmented_fit(ui: &mut Ui, items: &[&str], selected: usize, size: f32, height: f32) -> Option<usize> {
    let mut clicked = None;
    let widths = fit_widths(ui, items, size);
    let total = segmented_fit_width(ui, items, size);
    ui.allocate_ui_with_layout(Vec2::new(total, height + 4.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
        egui::Frame::none().fill(BG_TINT).rounding(10.0).inner_margin(Margin::same(2.0)).show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.spacing_mut().button_padding = Vec2::new(size * 0.8, 2.0);
            for (i, item) in items.iter().enumerate() {
                if seg_button(ui, item, i == selected, true, Vec2::new(widths[i], height), size).clicked() && i != selected {
                    clicked = Some(i);
                }
            }
        });
    });
    clicked
}

fn seg_button(ui: &mut Ui, text: &str, on: bool, enabled: bool, size: Vec2, font: f32) -> Response {
    let text = if on {
        RichText::new(text).size(font).family(semibold()).color(TEXT)
    } else {
        RichText::new(text).size(font).color(TEXT_2)
    };
    let b = egui::Button::new(text)
        .fill(if on { SURFACE } else { Color32::TRANSPARENT })
        .stroke(Stroke::NONE)
        .rounding(8.0)
        .min_size(size);
    let r = ui.add_enabled(enabled, b);
    if on {
        // small shadow under the selected segment
        ui.painter().rect_stroke(r.rect, 8.0, Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(60, 50, 30, 22)));
    }
    r
}

/// pill button (.btn)
pub fn btn(ui: &mut Ui, text: &str, enabled: bool) -> Response {
    ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(text).size(12.5))
            .fill(SURFACE)
            .stroke(Stroke::new(1.0_f32, LINE))
            .rounding(999.0)
            .min_size(Vec2::new(0.0, 30.0)),
    )
}

/// small pill button for rows
pub fn btn_small(ui: &mut Ui, text: &str) -> Response {
    ui.add(
        egui::Button::new(RichText::new(text).size(11.5))
            .fill(SURFACE)
            .stroke(Stroke::new(1.0_f32, LINE))
            .rounding(999.0)
            .min_size(Vec2::new(0.0, 22.0)),
    )
}

/// blue pill button (.btn.primary)
pub fn btn_primary(ui: &mut Ui, text: &str, width: f32, enabled: bool) -> Response {
    let b = egui::Button::new(RichText::new(text).size(13.5).family(semibold()).color(Color32::WHITE))
        .fill(if enabled { ACCENT } else { TRACK })
        .stroke(Stroke::NONE)
        .rounding(999.0);
    // add_sized centers the text in the wide button
    ui.add_enabled_ui(enabled, |ui| ui.add_sized(Vec2::new(width, 36.0), b)).inner
}

/// borderless small button (button.icon), e.g. the x to remove a file
pub fn icon_btn(ui: &mut Ui, text: &str, enabled: bool) -> Response {
    ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(text).size(13.0).color(TEXT_3))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::NONE)
            .rounding(8.0),
    )
}

/// pill that switches on / off (blue when on), for picking algorithms
pub fn pick(ui: &mut Ui, on: &mut bool, text: &str) -> Response {
    let (fill, stroke, color) = if *on { (ACCENT_SOFT, ACCENT, Color32::from_rgb(0x0a, 0x5b, 0xb5)) } else { (SURFACE, LINE, TEXT_2) };
    let b = egui::Button::new(RichText::new(text).size(12.0).color(color)).fill(fill).stroke(Stroke::new(1.0_f32, stroke)).rounding(999.0).min_size(Vec2::new(0.0, 24.0));
    // a scope here would break horizontal_wrapped(): callers set a tighter button_padding instead
    let r = ui.add(b);
    if r.clicked() {
        *on = !*on;
    }
    r
}

/// status chip (.chip / .chip.on / ...); one piece, so wrapped rows break between chips, not inside
pub fn chip(ui: &mut Ui, text: &str, fill: Color32, color: Color32) {
    let galley = ui.fonts(|f| f.layout_no_wrap(text.to_string(), FontId::new(11.0, semibold()), color));
    let size = Vec2::new(galley.size().x + 18.0, galley.size().y + 6.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect_filled(rect, 999.0, fill);
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, color);
}

pub fn chip_gray(ui: &mut Ui, t: &str) {
    chip(ui, t, BG_TINT, TEXT_2)
}
pub fn chip_on(ui: &mut Ui, t: &str) {
    chip(ui, t, Color32::from_rgb(0xe6, 0xf7, 0xec), Color32::from_rgb(0x1c, 0x7a, 0x3e))
}
pub fn chip_blue(ui: &mut Ui, t: &str) {
    chip(ui, t, ACCENT_SOFT, Color32::from_rgb(0x0a, 0x5b, 0xb5))
}
pub fn chip_orange(ui: &mut Ui, t: &str) {
    chip(ui, t, Color32::from_rgb(0xfd, 0xf3, 0xe2), Color32::from_rgb(0x9a, 0x5b, 0x00))
}
pub fn chip_red(ui: &mut Ui, t: &str) {
    chip(ui, t, Color32::from_rgb(0xfd, 0xec, 0xed), RED)
}

/// text input (.add-row input): white, rounded, blue ring when focused
pub fn text_input(ui: &mut Ui, id: Id, text: &mut String, hint_text: &str, width: f32, password: bool) -> Response {
    let focused = ui.memory(|m| m.has_focus(id));
    let (stroke, ring) = if focused {
        (Stroke::new(1.0_f32, ACCENT), Shadow { offset: Vec2::ZERO, blur: 0.0, spread: 3.0, color: ACCENT_SOFT })
    } else {
        (Stroke::new(1.0_f32, LINE), Shadow::NONE)
    };
    egui::Frame::none()
        .fill(SURFACE)
        .stroke(stroke)
        .rounding(R_SM)
        .shadow(ring)
        .inner_margin(Margin::symmetric(10.0, 6.0))
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(text)
                    .id(id)
                    .frame(false)
                    .password(password)
                    .hint_text(RichText::new(hint_text).color(TEXT_3))
                    // the frame is the padding, TextEdit would add 4 px of its own on each side
                    .margin(Margin::ZERO)
                    .desired_width(width - 22.0)
                    .font(FontId::proportional(13.0)),
            )
        })
        .inner
}

/// multi-line box for texts / hashes / keys, monospace, as wide as there is room
pub fn text_area(ui: &mut Ui, id: Id, text: &mut String, hint_text: &str, rows: usize, editable: bool) -> Response {
    let focused = ui.memory(|m| m.has_focus(id));
    egui::Frame::none()
        .fill(if editable { SURFACE } else { SURFACE_2 })
        .stroke(Stroke::new(1.0_f32, if focused { ACCENT } else { LINE }))
        .rounding(R_SM)
        .inner_margin(Margin::symmetric(10.0, 8.0))
        .show(ui, |ui| {
            egui::ScrollArea::vertical().id_salt(id).max_height(rows as f32 * 17.0 + 4.0).auto_shrink([false, true]).show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(text)
                        .id(id)
                        .frame(false)
                        .interactive(editable)
                        .margin(Margin::ZERO)
                        .hint_text(RichText::new(hint_text).color(TEXT_3))
                        .desired_width(f32::INFINITY)
                        .desired_rows(rows)
                        .font(TextStyle::Monospace),
                )
            })
            .inner
        })
        .inner
}

/// list box with soft separators (.list)
pub fn list(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    egui::Frame::none()
        .fill(SURFACE_2)
        .stroke(Stroke::new(1.0_f32, LINE_SOFT))
        .rounding(R_MD)
        .inner_margin(Margin::symmetric(12.0, 4.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            add(ui)
        });
}

/// soft line between rows of a list()
pub fn row_line(ui: &mut Ui) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(r, 0.0, LINE_SOFT);
}

/// slim progress bar
pub fn progress(ui: &mut Ui, value: Option<f32>) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 5.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 3.0, TRACK);
    match value {
        Some(v) => {
            let mut fill = rect;
            fill.set_width(rect.width() * v.clamp(0.0, 1.0));
            p.rect_filled(fill, 3.0, ACCENT);
        }
        None => {
            // unknown total (key generation, Argon2): a block that slides back and forth
            let t = ui.input(|i| i.time) as f32;
            let w = rect.width() * 0.25;
            let x = rect.left() + (rect.width() - w) * (0.5 + 0.5 * (t * 1.8).sin());
            p.rect_filled(Rect::from_min_size(egui::pos2(x, rect.top()), Vec2::new(w, rect.height())), 3.0, ACCENT);
            ui.ctx().request_repaint();
        }
    }
}

/// strength meter: 5 segments, red .. green
pub fn meter(ui: &mut Ui, level: u8, width: f32) {
    let colors = [RED, ORANGE, Color32::from_rgb(0xe0, 0xb1, 0x00), Color32::from_rgb(0x7c, 0xc4, 0x4d), GREEN];
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 6.0), Sense::hover());
    let w = (rect.width() - 4.0 * 4.0) / 5.0;
    for i in 0..5 {
        let r = Rect::from_min_size(rect.min + Vec2::new(i as f32 * (w + 4.0), 0.0), Vec2::new(w, 6.0));
        ui.painter().rect_filled(r, 3.0, if i <= level as usize { colors[level as usize] } else { TRACK });
    }
}

/// yellow hint box (.note)
pub fn note(ui: &mut Ui, text: &str) {
    egui::Frame::none()
        .fill(Color32::from_rgb(0xfd, 0xf6, 0xe6))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0xf0, 0xe2, 0xc2)))
        .rounding(R_MD)
        .inner_margin(Margin::symmetric(12.0, 9.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add(egui::Label::new(RichText::new(text).size(11.5).color(Color32::from_rgb(0x6b, 0x5a, 0x33))).wrap());
        });
}

// ---------------- logo ----------------

// padlock in 64x64 coordinates
const BODY: (f32, f32, f32, f32) = (16.0, 29.0, 48.0, 55.0);
const SHACKLE_C: (f32, f32) = (32.0, 29.0);
const SHACKLE_R: f32 = 11.5;
const SHACKLE_W: f32 = 6.0;
const HOLE_C: (f32, f32) = (32.0, 39.5);

/// logo: rounded rust-red square with a white padlock
pub fn logo(ui: &mut Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect.translate(Vec2::new(0.0, 1.0)), 8.0, Color32::from_rgba_unmultiplied(74, 62, 40, 20));
    p.rect_filled(rect, 8.0, RUST);
    let s = size / 64.0;
    let at = |x: f32, y: f32| rect.min + Vec2::new(x * s, y * s);
    // shackle: upper half circle + short legs into the body
    let mut arc: Vec<egui::Pos2> = (0..=24)
        .map(|i| {
            let a = std::f32::consts::PI * (1.0 + i as f32 / 24.0);
            at(SHACKLE_C.0 + SHACKLE_R * a.cos(), SHACKLE_C.1 + SHACKLE_R * a.sin())
        })
        .collect();
    arc.insert(0, at(SHACKLE_C.0 - SHACKLE_R, SHACKLE_C.1 + 2.0));
    arc.push(at(SHACKLE_C.0 + SHACKLE_R, SHACKLE_C.1 + 2.0));
    p.add(egui::Shape::line(arc, Stroke::new(SHACKLE_W * s, Color32::WHITE)));
    p.rect_filled(Rect::from_min_max(at(BODY.0, BODY.1), at(BODY.2, BODY.3)), 4.0 * s, Color32::WHITE);
    p.circle_filled(at(HOLE_C.0, HOLE_C.1), 3.6 * s, RUST);
    p.rect_filled(Rect::from_min_max(at(HOLE_C.0 - 1.5, HOLE_C.1), at(HOLE_C.0 + 1.5, HOLE_C.1 + 8.0)), 1.0 * s, RUST);
}

fn in_lock(x: f32, y: f32) -> bool {
    let rounded = |x: f32, y: f32, (x0, y0, x1, y1): (f32, f32, f32, f32), r: f32| {
        let cx = x.clamp(x0 + r, x1 - r);
        let cy = y.clamp(y0 + r, y1 - r);
        x >= x0 && x <= x1 && y >= y0 && y <= y1 && (x - cx).powi(2) + (y - cy).powi(2) <= r * r
    };
    let body = rounded(x, y, BODY, 4.0);
    let d = ((x - SHACKLE_C.0).powi(2) + (y - SHACKLE_C.1).powi(2)).sqrt();
    let ring = y <= SHACKLE_C.1 && (d - SHACKLE_R).abs() <= SHACKLE_W / 2.0;
    let legs = y > SHACKLE_C.1 && y < BODY.1 + 2.0 && ((x - (SHACKLE_C.0 - SHACKLE_R)).abs() <= SHACKLE_W / 2.0 || (x - (SHACKLE_C.0 + SHACKLE_R)).abs() <= SHACKLE_W / 2.0);
    let hole = (x - HOLE_C.0).powi(2) + (y - HOLE_C.1).powi(2) <= 3.6f32.powi(2) || ((x - HOLE_C.0).abs() <= 1.5 && y >= HOLE_C.1 && y <= HOLE_C.1 + 8.0);
    (body && !hole) || ring || legs
}

/// same logo as RGBA (window / taskbar icon), drawn in code, 4x4 supersampled
pub fn icon_rgba(size: usize) -> Vec<u8> {
    let k = 64.0 / size as f32;
    let inside_rounded = |x: f32, y: f32| {
        let r = 14.0;
        let cx = x.clamp(r, 64.0 - r);
        let cy = y.clamp(r, 64.0 - r);
        (x - cx).powi(2) + (y - cy).powi(2) <= r * r
    };
    let mut rgba = vec![0u8; size * size * 4];
    for py in 0..size {
        for px in 0..size {
            let (mut cover, mut white) = (0.0, 0.0);
            for sy in 0..4 {
                for sx in 0..4 {
                    let x = (px as f32 + (sx as f32 + 0.5) / 4.0) * k;
                    let y = (py as f32 + (sy as f32 + 0.5) / 4.0) * k;
                    if inside_rounded(x, y) {
                        cover += 1.0 / 16.0;
                        if in_lock(x, y) {
                            white += 1.0 / 16.0;
                        }
                    }
                }
            }
            let w = if cover > 0.0 { white / cover } else { 0.0 };
            let mix = |a: u8| (a as f32 * (1.0 - w) + 255.0 * w) as u8;
            let i = (py * size + px) * 4;
            rgba[i..i + 4].copy_from_slice(&[mix(RUST.r()), mix(RUST.g()), mix(RUST.b()), (cover * 255.0) as u8]);
        }
    }
    rgba
}

pub fn app_icon() -> egui::IconData {
    egui::IconData { rgba: icon_rgba(64), width: 64, height: 64 }
}
