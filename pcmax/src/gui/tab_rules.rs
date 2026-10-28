// Rules: when a program starts it gets its profile. Every rule can be switched off.

use super::theme::*;
use super::{editor, t, App};
use eframe::egui::{self, Align, Layout, RichText, Ui};
use pcmax::profile::Profile;
use pcmax::rules::Rule;

pub fn show(app: &mut App, ui: &mut Ui) {
    let de = app.de;
    let topo = app.topo;
    let ram = app.sample.ram_total.max(16 << 30);
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        panel(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    panel_head(
                        ui,
                        t(de, "Regeln", "Rules"),
                        t(de, "Startet ein passendes Programm, bekommt es automatisch seine Einstellungen (auch Programme, die schon laufen, wenn pcmax startet).", "When a matching program starts it gets its settings automatically (also programs already running when pcmax starts)."),
                    );
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if btn(ui, t(de, "+ Neue Regel", "+ New rule"), true).clicked() {
                        app.s.rules.push(Rule { enabled: true, pattern: "program.exe".into(), profile: Profile::default() });
                        app.rule_open = Some(app.s.rules.len() - 1);
                        app.save();
                    }
                });
            });
            hint(ui, t(de, "Muster: Exe-Namen mit Komma (7z.exe, WinRAR.exe), * als Platzhalter (steam*), oder ein Pfad-Teil mit \\ (\\steamapps\\common\\).", "Pattern: exe names with commas (7z.exe, WinRAR.exe), * as wildcard (steam*), or a path part with \\ (\\steamapps\\common\\)."));
        });
        ui.add_space(10.0);
        let mut delete = None;
        let mut changed = false;
        for i in 0..app.s.rules.len() {
            let open = app.rule_open == Some(i);
            panel(ui, |ui| {
                ui.horizontal(|ui| {
                    if toggle(ui, &mut app.s.rules[i].enabled, true).changed() {
                        changed = true;
                    }
                    let mut pat = app.s.rules[i].pattern.clone();
                    let r = text_input(ui, egui::Id::new(("rule-pat", i)), &mut pat, "program.exe", (ui.available_width() - 150.0).max(240.0));
                    if r.changed() {
                        app.s.rules[i].pattern = pat;
                    }
                    // saved when leaving the field, not on every key
                    if r.lost_focus() {
                        changed = true;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icon_btn(ui, "🗑", true).on_hover_text(t(de, "Löschen", "Delete")).clicked() {
                            delete = Some(i);
                        }
                        let label = if open { t(de, "Schließen", "Close") } else { t(de, "Bearbeiten", "Edit") };
                        if btn_small(ui, label).clicked() {
                            app.rule_open = if open { None } else { Some(i) };
                        }
                    });
                });
                let summary = app.s.rules[i].profile.summary();
                ui.label(RichText::new(summary).size(12.0).color(if app.s.rules[i].enabled { TEXT_2 } else { TEXT_3 }));
                if open {
                    ui.add_space(6.0);
                    sep(ui);
                    ui.add_space(4.0);
                    if editor::edit(ui, de, &mut app.s.rules[i].profile, topo, ram, &format!("rule{i}")) {
                        // sliders change every frame while dragging
                        app.save_later();
                    }
                }
            });
            ui.add_space(8.0);
        }
        if let Some(i) = delete {
            app.s.rules.remove(i);
            app.rule_open = None;
            changed = true;
        }
        if app.s.rules.is_empty() {
            hint(ui, t(de, "Keine Regeln.", "No rules."));
        }
        if changed {
            app.save();
        }
    });
}
