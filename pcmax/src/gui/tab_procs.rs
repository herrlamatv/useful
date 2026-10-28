// Processes: list (CPU / RAM / GPU live), pick a window with the mouse, set a profile for it.

use super::theme::*;
use super::{editor, t, App, Pick};
use eframe::egui::{self, Align, Layout, RichText, Sense, Ui, Vec2};
use pcmax::cpu::Prio;
use pcmax::profile::{self, Profile};
use pcmax::rules::Rule;
use pcmax::win::fmt_bytes;

pub fn show(app: &mut App, ui: &mut Ui) {
    let h = ui.available_height();
    ui.columns(2, |cols| {
        cols[0].set_height(h);
        proc_list(app, &mut cols[0]);
        details(app, &mut cols[1]);
    });
}

fn proc_list(app: &mut App, ui: &mut Ui) {
    let de = app.de;
    panel_flush(ui, |ui| {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_space(6.0);
            text_input(ui, egui::Id::new("proc-filter"), &mut app.filter, t(de, "Suchen …", "Search ..."), 220.0);
            let picking = app.pick.is_some();
            let label = if picking { t(de, "Jetzt auf ein Fenster klicken … (Esc)", "Click a window now ... (Esc)") } else { t(de, "🎯 Fenster anklicken", "🎯 Pick a window") };
            if btn(ui, label, !picking).on_hover_text(t(de, "Danach auf irgendein Fenster klicken – dessen Programm wird ausgewählt", "Then click any window - its program gets selected")).clicked() {
                app.pick = Some(Pick::Release);
            }
        });
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            let w = ui.available_width() - 12.0;
            ui.allocate_ui(Vec2::new(w, 16.0), |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t(de, "PROGRAMM", "PROGRAM")).size(10.5).color(TEXT_3));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        for (txt, wd) in [("GPU", 44.0), ("RAM", 70.0), ("CPU", 50.0)] {
                            ui.add_sized(Vec2::new(wd, 14.0), egui::Label::new(RichText::new(txt).size(10.5).color(TEXT_3)));
                        }
                    });
                });
            });
        });
        let f = app.filter.to_lowercase();
        let rows: Vec<super::Row> = app
            .rows
            .iter()
            .filter(|r| f.is_empty() || r.name.to_lowercase().contains(&f) || r.title.to_lowercase().contains(&f) || r.pid.to_string() == f)
            .cloned()
            .collect();
        let row_h = 34.0;
        let mut clicked = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, row_h, rows.len(), |ui, range| {
            for r in &rows[range] {
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), row_h), Sense::click());
                let sel = app.selected == Some(r.pid);
                let p = ui.painter();
                if sel {
                    p.rect_filled(rect.shrink2(Vec2::new(4.0, 1.0)), R_SM, ACCENT_SOFT);
                } else if resp.hovered() {
                    p.rect_filled(rect.shrink2(Vec2::new(4.0, 1.0)), R_SM, SURFACE_2);
                }
                let left = rect.left() + 12.0;
                let font = egui::FontId::proportional(13.0);
                let small = egui::FontId::proportional(11.0);
                p.text(egui::pos2(left, rect.top() + 9.0), egui::Align2::LEFT_CENTER, &r.name, font.clone(), TEXT);
                let sub = if r.title.is_empty() { format!("PID {}", r.pid) } else { format!("PID {} · {}", r.pid, r.title) };
                let sub: String = sub.chars().take(60).collect();
                p.text(egui::pos2(left, rect.top() + 24.0), egui::Align2::LEFT_CENTER, sub, small.clone(), TEXT_2);
                let right = rect.right() - 12.0;
                let gpu = if r.gpu >= 0.5 { format!("{:.0} %", r.gpu) } else { "–".into() };
                p.text(egui::pos2(right, rect.center().y), egui::Align2::RIGHT_CENTER, gpu, small.clone(), TEXT_2);
                p.text(egui::pos2(right - 50.0, rect.center().y), egui::Align2::RIGHT_CENTER, fmt_bytes(r.ram), small.clone(), TEXT_2);
                let cpu_col = if r.cpu >= 10.0 { RUST } else if r.cpu >= 1.0 { TEXT } else { TEXT_3 };
                p.text(egui::pos2(right - 126.0, rect.center().y), egui::Align2::RIGHT_CENTER, format!("{:.1} %", r.cpu), font, cpu_col);
                if resp.clicked() {
                    clicked = Some(r.pid);
                }
            }
        });
        if let Some(pid) = clicked {
            app.select(pid);
        }
    });
}

fn details(app: &mut App, ui: &mut Ui) {
    let de = app.de;
    let Some(pid) = app.selected else {
        panel(ui, |ui| {
            panel_head(ui, t(de, "Kein Programm gewählt", "No program selected"), t(de, "Links eines anklicken oder „Fenster anklicken“ und dann auf irgendein Fenster klicken.", "Click one on the left or 'Pick a window' and then click any window."));
            ui.add_space(8.0);
            note(ui, t(de, "Warum 1..10000000 in PowerShell so lange dauert: Das Zählen braucht 0,3 s – die Zeit geht für die Ausgabe Zeile für Zeile in die Konsole drauf (ein Thread). Dagegen hilft „pcm fast“ bzw. fast { … } (Einstellungen → PowerShell), nicht mehr CPU.", "Why 1..10000000 takes so long in PowerShell: counting takes 0.3 s - the time goes into printing line by line to the console (one thread). 'pcm fast' / fast { ... } helps (Settings -> PowerShell), more CPU doesn't."));
        });
        return;
    };
    let row = app.rows.iter().find(|r| r.pid == pid).cloned();
    let state = app.sel_state.clone();
    let topo = app.topo;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        panel(ui, |ui| {
            let name = state.as_ref().map(|s| s.name.clone()).unwrap_or_else(|| "?".into());
            ui.horizontal(|ui| {
                ui.label(RichText::new(&name).size(16.0).family(semibold()));
                ui.label(RichText::new(format!("PID {pid}")).size(12.0).color(TEXT_2));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icon_btn(ui, "✕", true).clicked() {
                        app.selected = None;
                    }
                });
            });
            if let Some(s) = &state {
                if let Some(exe) = &s.exe {
                    hint(ui, exe);
                }
                ui.horizontal_wrapped(|ui| {
                    let cpu = row.as_ref().map(|r| r.cpu).unwrap_or(0.0);
                    chip_gray(ui, &format!("CPU {:.1} %", cpu));
                    chip_gray(ui, &format!("{} {}", s.threads, t(de, "Threads", "threads")));
                    if let Some(p) = s.prio {
                        chip_gray(ui, &format!("{} {}", t(de, "Prio", "prio"), p.key()));
                    }
                    if let Some(e) = s.eco {
                        chip_gray(ui, &format!("Eco {}", e.key()));
                    }
                    if let Some(a) = s.affinity {
                        chip_gray(ui, &format!("{} {}", t(de, "Kerne", "cores"), topo.describe(a)));
                    }
                    if let Some(i) = s.io {
                        chip_gray(ui, &format!("I/O {}", i.key()));
                    }
                    if let Some(g) = s.gpu {
                        chip_gray(ui, &format!("GPU {}", g.key()));
                    }
                    if let Some(w) = s.working_set {
                        chip_gray(ui, &format!("RAM {}", fmt_bytes(w)));
                    }
                    if let Some((_, l)) = &s.job {
                        if l.cpu_limited() {
                            chip_blue(ui, &format!("CPU {}-{} %", l.cpu_min, l.cpu_max));
                        }
                        if let Some(m) = l.mem {
                            chip_blue(ui, &format!("RAM max {}", fmt_bytes(m)));
                        }
                    }
                });
                // one core at its limit: more "power" can't make it faster
                let one_core = 100.0 / topo.logical as f32;
                if let Some(r) = &row {
                    if r.cpu > one_core * 0.8 && r.cpu < one_core * 1.25 {
                        note(ui, &format!(
                            "{} {:.1} % = {}",
                            t(de, "Läuft gerade mit ≈", "Runs at ≈"),
                            r.cpu,
                            t(de, "ein Kern am Anschlag. Mehr Kerne helfen hier nicht – nur Priorität, kein Drosseln und weniger Konkurrenz.", "one core at its limit. More cores don't help here - only priority, no throttling and less competition.")
                        ));
                    }
                }
            }
        });
        ui.add_space(10.0);
        panel(ui, |ui| {
            ui.horizontal(|ui| {
                if btn(ui, t(de, "⚡ Boost-Vorlage", "⚡ Boost preset"), true).on_hover_text(t(de, "Hoch, kein Effizienzmodus, I/O + GPU hoch", "High, no Efficiency Mode, I/O + GPU high")).clicked() {
                    app.editor = Profile { tree: app.editor.tree, ..Profile::boost() };
                }
                if btn(ui, t(de, "🌙 Bremsen-Vorlage", "🌙 Calm preset"), true).on_hover_text(t(de, "Niedriger, Effizienzmodus, I/O niedrig", "Below normal, Efficiency Mode, I/O low")).clicked() {
                    app.editor = Profile { tree: app.editor.tree, ..Profile::calm() };
                }
                if btn(ui, t(de, "Leeren", "Clear"), true).clicked() {
                    app.editor = Profile::default();
                }
            });
            ui.add_space(6.0);
            let ram = app.sample.ram_total.max(1 << 30);
            editor::edit(ui, de, &mut app.editor, topo, ram, "procs");
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if btn_primary(ui, t(de, "Anwenden", "Apply"), 140.0, !app.editor.is_empty()).clicked() {
                    if app.editor.prio == Some(Prio::Realtime) || app.editor.gpu == Some(Prio::Realtime) {
                        app.confirm_rt = true;
                    } else {
                        apply(app, pid, false);
                    }
                }
                if btn(ui, t(de, "Zurücksetzen", "Reset"), true).on_hover_text(t(de, "Alles auf Windows-Standard", "Everything back to Windows defaults")).clicked() {
                    let rep = profile::reset(pid, app.editor.tree);
                    app.results = rep.lines();
                    app.refreshed = None;
                }
                if btn(ui, t(de, "Als Regel speichern", "Save as rule"), !app.editor.is_empty())
                    .on_hover_text(t(de, "Jedes Mal, wenn das Programm startet, bekommt es diese Einstellungen", "Every time the program starts it gets these settings"))
                    .clicked()
                {
                    if let Some(s) = &state {
                        let name = s.name.clone();
                        let prof = app.editor.clone();
                        if let Some(r) = app.s.rules.iter_mut().find(|r| r.pattern.eq_ignore_ascii_case(&name)) {
                            r.profile = prof;
                            r.enabled = true;
                        } else {
                            app.s.rules.push(Rule { enabled: true, pattern: name.clone(), profile: prof });
                        }
                        app.save();
                        app.toast(format!("{}: {name}", t(de, "Regel gespeichert", "Rule saved")), true);
                    }
                }
            });
            if !app.results.is_empty() {
                ui.add_space(8.0);
                list(ui, |ui| {
                    for l in &app.results {
                        let c = if l.starts_with('✓') { egui::Color32::from_rgb(0x1c, 0x7a, 0x3e) } else if l.starts_with('!') { egui::Color32::from_rgb(0x8a, 0x6a, 0x1f) } else { RED };
                        ui.add_space(3.0);
                        ui.label(RichText::new(l).size(12.0).color(c));
                        ui.add_space(3.0);
                    }
                });
            }
        });
    });
}

fn apply(app: &mut App, pid: u32, realtime: bool) {
    let rep = profile::apply(pid, &app.editor, realtime);
    app.results = rep.lines();
    if let Some(j) = rep.job {
        app.kept_jobs.push(j);
    }
    app.refreshed = None;
}

pub fn confirm_realtime(app: &mut App, ctx: &egui::Context) {
    let de = app.de;
    egui::Window::new(t(de, "Echtzeit-Priorität?", "Realtime priority?"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.label(t(de, "Ein Programm mit Echtzeit-Priorität kann Maus, Tastatur und Ton blockieren, bis es fertig ist.", "A program with realtime priority can block mouse, keyboard and sound until it is done."));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if btn_danger(ui, t(de, "Ja, Echtzeit", "Yes, realtime"), 140.0).clicked() {
                    app.confirm_rt = false;
                    if let Some(pid) = app.selected {
                        apply(app, pid, true);
                    }
                }
                if btn(ui, t(de, "Abbrechen", "Cancel"), true).clicked() {
                    app.confirm_rt = false;
                }
            });
        });
}
