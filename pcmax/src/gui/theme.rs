// Off-white look, taken from chroofer/ui/style.css (same colors, radii and shadows).
// Copied from rustscanner (tree icons removed), logo = lightning bolt, plus bars / sparklines for the monitor.

use eframe::egui::{
    self, epaint::Shadow, Color32, FontData, FontDefinitions, FontFamily, FontId, Id, Margin, Pos2, Rect,
    Response, RichText, Rounding, Sense, Stroke, TextStyle, Ui, Vec2,
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
pub const RED: Color32 = Color32::from_rgb(0xe5, 0x48, 0x4d);
pub const TRACK: Color32 = Color32::from_rgb(0xde, 0xd7, 0xc7);
/// the red/orange of rust-lang
pub const RUST: Color32 = Color32::from_rgb(0xce, 0x42, 0x2b);
/// CCD with 3D V-Cache in the monitor
pub const VCACHE: Color32 = Color32::from_rgb(0x00, 0x71, 0xe3);
/// the higher clocked CCD
pub const FREQ: Color32 = Color32::from_rgb(0xff, 0x95, 0x00);

pub const R_LG: f32 = 18.0;
pub const R_MD: f32 = 12.0;
pub const R_SM: f32 = 9.0;

pub fn semibold() -> FontFamily {
    FontFamily::Name("semibold".into())
}

/// Segoe UI like in chroofer (falls back to the egui font if it's missing)
pub fn fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let dir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    let load = |f: &str| std::fs::read(format!("{dir}\\Fonts\\{f}")).ok();

    if let Some(regular) = load("segoeui.ttf") {
        fonts.font_data.insert("segoe".into(), FontData::from_owned(regular));
        fonts.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "segoe".into());
    }
    // symbols (☁ ✕ …) that Segoe UI doesn't have
    if let Some(sym) = load("seguisym.ttf") {
        fonts.font_data.insert("segoe-symbol".into(), FontData::from_owned(sym));
        fonts.families.get_mut(&FontFamily::Proportional).unwrap().push("segoe-symbol".into());
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
        (TextStyle::Monospace, FontId::monospace(11.5)),
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
    ctx.set_style(style);
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

/// same card, but without padding (for the process list)
pub fn panel_flush(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    egui::Frame::none()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0_f32, LINE))
        .rounding(R_LG)
        .inner_margin(Margin::symmetric(6.0, 6.0))
        .shadow(card_shadow())
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_height(ui.available_height());
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
    ui.label(RichText::new(text).size(11.5).color(TEXT_2));
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

/// red text pill button (.btn.danger)
pub fn btn_danger(ui: &mut Ui, text: &str, width: f32) -> Response {
    let b = egui::Button::new(RichText::new(text).size(13.0).color(RED)).fill(SURFACE).stroke(Stroke::new(1.0_f32, LINE)).rounding(999.0);
    ui.add_sized(Vec2::new(width, 36.0), b)
}

/// borderless small button (button.icon), e.g. the x to remove a folder
pub fn icon_btn(ui: &mut Ui, text: &str, enabled: bool) -> Response {
    ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(text).size(13.0).color(TEXT_3))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::NONE)
            .rounding(8.0),
    )
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
pub fn chip_red(ui: &mut Ui, t: &str) {
    chip(ui, t, Color32::from_rgb(0xfd, 0xec, 0xed), RED)
}
/// iOS switch (.toggle): gray track, green when on, white knob that slides
pub fn toggle(ui: &mut Ui, on: &mut bool, enabled: bool) -> Response {
    let size = Vec2::new(40.0, 24.0);
    let (rect, mut resp) = ui.allocate_exact_size(size, if enabled { Sense::click() } else { Sense::hover() });
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = ui.ctx().animate_bool_with_time(resp.id, *on, 0.18);
    let alpha = if enabled { 1.0 } else { 0.4 };
    let p = ui.painter();
    p.rect_filled(rect, rect.height() / 2.0, lerp_color(TRACK, GREEN, t).gamma_multiply(alpha));
    let r = rect.height() / 2.0 - 2.0;
    let x = egui::lerp((rect.left() + 2.0 + r)..=(rect.right() - 2.0 - r), t);
    let c = egui::pos2(x, rect.center().y);
    p.circle_filled(c + Vec2::new(0.0, 1.0), r, Color32::from_rgba_unmultiplied(60, 50, 30, (60.0 * alpha) as u8));
    p.circle_filled(c, r, Color32::WHITE.gamma_multiply(if enabled { 1.0 } else { 0.85 }));
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

/// text input (.add-row input): white, rounded, blue ring when focused
pub fn text_input(ui: &mut Ui, id: Id, text: &mut String, hint_text: &str, width: f32) -> Response {
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
                    .hint_text(RichText::new(hint_text).color(TEXT_3))
                    .desired_width(width - 20.0)
                    .font(FontId::proportional(13.0)),
            )
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

/// one row in a list: title + gray text on the left, `right` on the right (.item)
pub fn item(ui: &mut Ui, first: bool, title: &str, sub: &str, right: impl FnOnce(&mut Ui)) {
    if !first {
        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
        ui.painter().rect_filled(r, 0.0, LINE_SOFT);
    }
    ui.add_space(7.0);
    ui.horizontal(|ui| {
        let right_w = 52.0;
        ui.vertical(|ui| {
            ui.set_width(ui.available_width() - right_w);
            ui.spacing_mut().item_spacing.y = 1.0;
            ui.add(egui::Label::new(RichText::new(title).size(13.0)).wrap());
            if !sub.is_empty() {
                ui.add(egui::Label::new(RichText::new(sub).size(11.5).color(TEXT_2)).wrap());
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), right);
    });
    ui.add_space(7.0);
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
            // unknown total: a block that slides back and forth
            let t = ui.input(|i| i.time) as f32;
            let w = rect.width() * 0.25;
            let x = rect.left() + (rect.width() - w) * (0.5 + 0.5 * (t * 1.8).sin());
            p.rect_filled(Rect::from_min_size(egui::pos2(x, rect.top()), Vec2::new(w, rect.height())), 3.0, ACCENT);
            ui.ctx().request_repaint();
        }
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
            ui.label(RichText::new(text).size(11.5).color(Color32::from_rgb(0x6b, 0x5a, 0x33)));
        });
}

// ---------------- logo ----------------

/// lightning bolt in 64x64 coordinates (concave, drawn as two convex parts)
const BOLT: [(f32, f32); 7] = [(28.0, 6.0), (46.0, 6.0), (36.0, 28.0), (48.0, 28.0), (22.0, 58.0), (28.0, 36.0), (16.0, 36.0)];
const BOLT_TOP: [usize; 5] = [0, 1, 2, 5, 6];
const BOLT_BOTTOM: [usize; 4] = [2, 3, 4, 5];

/// logo: rounded rust-red square with a white lightning bolt
pub fn logo(ui: &mut Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect.translate(Vec2::new(0.0, 1.0)), 8.0, Color32::from_rgba_unmultiplied(74, 62, 40, 20));
    p.rect_filled(rect, 8.0, RUST);
    let s = size / 64.0;
    let at = |i: usize| rect.min + Vec2::new(BOLT[i].0 * s, BOLT[i].1 * s);
    for part in [&BOLT_TOP[..], &BOLT_BOTTOM[..]] {
        p.add(egui::Shape::convex_polygon(part.iter().map(|&i| at(i)).collect(), Color32::WHITE, Stroke::NONE));
    }
}

fn in_bolt(x: f32, y: f32) -> bool {
    // even-odd rule over the whole outline
    let mut inside = false;
    let n = BOLT.len();
    for i in 0..n {
        let (xi, yi) = BOLT[i];
        let (xj, yj) = BOLT[(i + n - 1) % n];
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
    }
    inside
}

/// same logo as RGBA (window / taskbar / tray icon), drawn in code, 4x4 supersampled
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
                        if in_bolt(x, y) {
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

// ---------------- monitor ----------------

/// vertical bar 0..100 %
pub fn bar(ui: &mut Ui, size: Vec2, pct: f32, color: Color32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 3.0, BG_TINT);
    let h = rect.height() * (pct / 100.0).clamp(0.0, 1.0);
    if h > 0.5 {
        p.rect_filled(Rect::from_min_max(egui::pos2(rect.left(), rect.bottom() - h), rect.max), 3.0, color);
    }
    resp
}

/// filled line chart of the last values, 0..100
pub fn sparkline(ui: &mut Ui, size: Vec2, values: &[f32], color: Color32) {
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, R_SM, SURFACE_2);
    for f in [0.25, 0.5, 0.75] {
        let y = rect.bottom() - rect.height() * f;
        p.line_segment([egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)], Stroke::new(1.0_f32, LINE_SOFT));
    }
    if values.len() < 2 {
        return;
    }
    let n = values.len();
    let pts: Vec<Pos2> = values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            egui::pos2(
                rect.left() + rect.width() * i as f32 / (n - 1) as f32,
                rect.bottom() - rect.height() * (v / 100.0).clamp(0.0, 1.0),
            )
        })
        .collect();
    // area under the line as thin quads (the area itself is not convex)
    let fill = color.gamma_multiply(0.18);
    for w in pts.windows(2) {
        p.add(egui::Shape::convex_polygon(
            vec![w[0], w[1], egui::pos2(w[1].x, rect.bottom()), egui::pos2(w[0].x, rect.bottom())],
            fill,
            Stroke::NONE,
        ));
    }
    p.add(egui::Shape::line(pts, Stroke::new(1.6_f32, color)));
}
