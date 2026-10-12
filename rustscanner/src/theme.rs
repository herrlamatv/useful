// Off-white look, taken from chroofer/ui/style.css (same colors, radii and shadows).
// Base copied from FastForwardRustverter, plus toggle / input / chips / tree icons for rustscanner.

use eframe::egui::{
    self, epaint::Shadow, Color32, FontData, FontDefinitions, FontFamily, FontId, Id, Margin, Painter, Pos2, Rect,
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
pub const FOLDER_BACK: Color32 = Color32::from_rgb(0xd9, 0x9a, 0x2b);
pub const FOLDER_FRONT: Color32 = Color32::from_rgb(0xf0, 0xb9, 0x4a);

/// one color per file type (same order as model::CAT_*)
pub const CAT_COLORS: [Color32; 9] = [
    Color32::from_rgb(0x00, 0x71, 0xe3), // web
    Color32::from_rgb(0x34, 0xa8, 0x53), // images
    Color32::from_rgb(0xaf, 0x52, 0xde), // videos
    Color32::from_rgb(0xff, 0x95, 0x00), // audio
    Color32::from_rgb(0x58, 0x56, 0xd6), // documents
    Color32::from_rgb(0xce, 0x42, 0x2b), // code
    Color32::from_rgb(0xa2, 0x84, 0x5e), // archives
    Color32::from_rgb(0x8e, 0x8e, 0x93), // programs
    Color32::from_rgb(0xa2, 0x9a, 0x8a), // other
];

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
            add(ui)
        });
}

/// same card, but without padding (for the tree)
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
            add(ui)
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

/// status chip (.chip / .chip.on / ...)
pub fn chip(ui: &mut Ui, text: &str, fill: Color32, color: Color32) {
    egui::Frame::none().fill(fill).rounding(999.0).inner_margin(Margin::symmetric(9.0, 3.0)).show(ui, |ui| {
        ui.label(RichText::new(text).size(11.0).family(semibold()).color(color));
    });
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
pub fn chip_yellow(ui: &mut Ui, t: &str) {
    chip(ui, t, Color32::from_rgb(0xfd, 0xf6, 0xe6), Color32::from_rgb(0x8a, 0x6a, 0x1f))
}

/// clickable filter chip with optional colored dot and a count
pub fn chip_toggle(ui: &mut Ui, text: &str, count: &str, on: bool, dot: Option<Color32>) -> Response {
    let font = FontId::proportional(12.0);
    let (tc, cc) = if on { (Color32::from_rgb(0x0a, 0x5b, 0xb5), Color32::from_rgb(0x5a, 0x8f, 0xd0)) } else { (TEXT, TEXT_3) };
    let g_text = ui.fonts(|f| f.layout_no_wrap(text.to_string(), font.clone(), tc));
    let g_count = ui.fonts(|f| f.layout_no_wrap(count.to_string(), FontId::proportional(11.0), cc));
    let pad = 10.0;
    let dot_w = if dot.is_some() { 12.0 } else { 0.0 };
    let gap = if count.is_empty() { 0.0 } else { 6.0 };
    let size = Vec2::new(pad + dot_w + g_text.size().x + gap + g_count.size().x + pad, 26.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let p = ui.painter();
    let (fill, stroke) = if on {
        (ACCENT_SOFT, Stroke::new(1.0_f32, Color32::from_rgb(0xb8, 0xd4, 0xf6)))
    } else if resp.hovered() {
        (SURFACE_2, Stroke::new(1.0_f32, LINE))
    } else {
        (SURFACE, Stroke::new(1.0_f32, LINE))
    };
    p.rect(rect, 999.0, fill, stroke);
    let mut x = rect.left() + pad;
    if let Some(c) = dot {
        p.circle_filled(egui::pos2(x + 4.0, rect.center().y), 4.0, c);
        x += dot_w;
    }
    p.galley(egui::pos2(x, rect.center().y - g_text.size().y / 2.0), g_text.clone(), tc);
    x += g_text.size().x + gap;
    p.galley(egui::pos2(x, rect.center().y - g_count.size().y / 2.0), g_count, cc);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
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

/// "−  2  +" stepper, returns true when the value changed
pub fn stepper(ui: &mut Ui, value: &mut u16, min: u16, max: u16) -> bool {
    let mut changed = false;
    egui::Frame::none().fill(BG_TINT).rounding(10.0).inner_margin(Margin::same(2.0)).show(ui, |ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        ui.horizontal(|ui| {
            let b = |t: &str| {
                egui::Button::new(RichText::new(t).size(13.0).family(semibold()).color(TEXT_2))
                    .fill(SURFACE)
                    .stroke(Stroke::NONE)
                    .rounding(8.0)
                    .min_size(Vec2::new(24.0, 22.0))
            };
            if ui.add_enabled(*value > min, b("−")).clicked() {
                *value -= 1;
                changed = true;
            }
            ui.add_sized(Vec2::new(26.0, 22.0), egui::Label::new(RichText::new(value.to_string()).size(12.5).family(semibold())));
            if ui.add_enabled(*value < max, b("+")).clicked() {
                *value += 1;
                changed = true;
            }
        });
    });
    changed
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

// ---------------- tree icons (painted, Segoe UI has no good glyphs for these) ----------------

/// ">" that turns into "v" while `open` goes 0 -> 1
pub fn paint_chevron(p: &Painter, c: Pos2, open: f32, color: Color32) {
    let a = open * std::f32::consts::FRAC_PI_2;
    let (s, co) = a.sin_cos();
    let rot = |x: f32, y: f32| egui::pos2(c.x + x * co - y * s, c.y + x * s + y * co);
    let pts = vec![rot(-1.8, -3.8), rot(2.2, 0.0), rot(-1.8, 3.8)];
    p.add(egui::Shape::line(pts, Stroke::new(1.6_f32, color)));
}

/// small folder: back with tab + lighter front
pub fn paint_folder(p: &Painter, c: Pos2, open: bool) {
    let r = Rect::from_center_size(c, Vec2::new(15.0, 12.0));
    let tab = Rect::from_min_size(r.min, Vec2::new(6.5, 3.0));
    p.rect_filled(tab, Rounding { nw: 1.5, ne: 1.5, sw: 0.0, se: 0.0 }, FOLDER_BACK);
    p.rect_filled(Rect::from_min_max(egui::pos2(r.left(), r.top() + 2.0), r.max), 2.0, FOLDER_BACK);
    let front = if open {
        // tilted front when open
        egui::Shape::convex_polygon(
            vec![
                egui::pos2(r.left() + 2.5, r.top() + 5.0),
                egui::pos2(r.right() + 1.0, r.top() + 5.0),
                egui::pos2(r.right() - 1.0, r.bottom()),
                egui::pos2(r.left(), r.bottom()),
            ],
            FOLDER_FRONT,
            Stroke::NONE,
        )
    } else {
        egui::Shape::rect_filled(Rect::from_min_max(egui::pos2(r.left(), r.top() + 4.0), r.max), 2.0, FOLDER_FRONT)
    };
    p.add(front);
}

/// small page with a folded corner in the type color
pub fn paint_file(p: &Painter, c: Pos2, color: Color32) {
    let r = Rect::from_center_size(c, Vec2::new(11.0, 14.0));
    let fold = 3.5;
    let body = vec![
        r.left_top(),
        egui::pos2(r.right() - fold, r.top()),
        egui::pos2(r.right(), r.top() + fold),
        r.right_bottom(),
        r.left_bottom(),
    ];
    p.add(egui::Shape::convex_polygon(body, color.gamma_multiply(0.16), Stroke::new(1.2_f32, color)));
    p.add(egui::Shape::convex_polygon(
        vec![egui::pos2(r.right() - fold, r.top()), egui::pos2(r.right(), r.top() + fold), egui::pos2(r.right() - fold, r.top() + fold)],
        color,
        Stroke::NONE,
    ));
}

/// cloud shape for OneDrive files that are not on this PC
pub fn paint_cloud(p: &Painter, c: Pos2, color: Color32) {
    p.circle_filled(c + Vec2::new(-3.0, 1.0), 3.0, color);
    p.circle_filled(c + Vec2::new(1.0, -1.0), 4.0, color);
    p.circle_filled(c + Vec2::new(4.5, 1.5), 2.5, color);
    p.rect_filled(Rect::from_min_max(c + Vec2::new(-3.0, 1.0), c + Vec2::new(4.5, 4.0)), 0.0, color);
}

// ---------------- logo ----------------

/// logo: rounded rust-red square with a white magnifier
pub fn logo(ui: &mut Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect.translate(Vec2::new(0.0, 1.0)), 8.0, Color32::from_rgba_unmultiplied(74, 62, 40, 20));
    p.rect_filled(rect, 8.0, RUST);
    let s = size / 64.0;
    let at = |x: f32, y: f32| rect.min + Vec2::new(x * s, y * s);
    let (ring_c, ring_r, ring_w) = (at(MAG.0, MAG.1), MAG.2 * s, MAG.3 * s);
    p.circle_stroke(ring_c, ring_r, Stroke::new(ring_w, Color32::WHITE));
    let (a, b, w) = (at(HANDLE.0, HANDLE.1), at(HANDLE.2, HANDLE.3), HANDLE.4 * s);
    p.line_segment([a, b], Stroke::new(w, Color32::WHITE));
    p.circle_filled(b, w / 2.0, Color32::WHITE);
}

/// magnifier in 64x64 coordinates: ring (center x, y, radius, width), handle (x1, y1, x2, y2, width)
const MAG: (f32, f32, f32, f32) = (27.5, 27.5, 12.5, 5.5);
const HANDLE: (f32, f32, f32, f32, f32) = (37.0, 37.0, 47.5, 47.5, 7.0);

/// same logo as a 64x64 window / taskbar icon (drawn in code, 4x4 supersampled)
pub fn app_icon() -> egui::IconData {
    const S: usize = 64;
    let inside_rounded = |x: f32, y: f32| {
        let r = 14.0;
        let cx = x.clamp(r, S as f32 - r);
        let cy = y.clamp(r, S as f32 - r);
        (x - cx).powi(2) + (y - cy).powi(2) <= r * r
    };
    let in_magnifier = |x: f32, y: f32| {
        let d = ((x - MAG.0).powi(2) + (y - MAG.1).powi(2)).sqrt();
        if (d - MAG.2).abs() <= MAG.3 / 2.0 {
            return true;
        }
        // distance to the handle segment
        let (ax, ay, bx, by, w) = HANDLE;
        let (dx, dy) = (bx - ax, by - ay);
        let t = (((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
        let (px, py) = (ax + t * dx, ay + t * dy);
        ((x - px).powi(2) + (y - py).powi(2)).sqrt() <= w / 2.0
    };
    let mut rgba = vec![0u8; S * S * 4];
    for py in 0..S {
        for px in 0..S {
            let (mut cover, mut white) = (0.0, 0.0);
            for sy in 0..4 {
                for sx in 0..4 {
                    let (x, y) = (px as f32 + (sx as f32 + 0.5) / 4.0, py as f32 + (sy as f32 + 0.5) / 4.0);
                    if inside_rounded(x, y) {
                        cover += 1.0 / 16.0;
                        if in_magnifier(x, y) {
                            white += 1.0 / 16.0;
                        }
                    }
                }
            }
            let w = if cover > 0.0 { white / cover } else { 0.0 };
            let mix = |a: u8| (a as f32 * (1.0 - w) + 255.0 * w) as u8;
            let i = (py * S + px) * 4;
            rgba[i..i + 4].copy_from_slice(&[mix(RUST.r()), mix(RUST.g()), mix(RUST.b()), (cover * 255.0) as u8]);
        }
    }
    egui::IconData { rgba, width: S as u32, height: S as u32 }
}
