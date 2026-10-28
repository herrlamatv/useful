// Boost: one main switch, every measure on its own switch, the program lists, live status + log.

use super::theme::*;
use super::{t, App};
use eframe::egui::{self, Align, Layout, RichText, Ui};
use pcmax::boost::Measure;
use pcmax::services;

fn texts(de: bool, m: Measure) -> (&'static str, &'static str) {
    match m {
        Measure::Plan => (
            t(de, "Energieplan „Höchstleistung“", "Power plan 'max performance'"),
            t(de, "Schaltet auf „Ultimative Leistung“ (oder „Höchstleistung“), beim Ausschalten zurück auf deinen Plan.", "Switches to 'Ultimate Performance' (or 'High performance'), back to your plan when switched off."),
        ),
        Measure::Parking => (
            t(de, "Keine Kerne schlafen legen", "No parked cores"),
            t(de, "Core-Parking aus: alle Kerne sofort bereit (weniger Aufwach-Verzögerung).", "Core parking off: all cores ready at once (less wake-up delay)."),
        ),
        Measure::Timer => (
            t(de, "Timer 1 ms", "1 ms timer"),
            t(de, "Feinerer Windows-Takt. Hilft bei Latenz (Spiele, Audio), nicht beim Durchsatz.", "Finer Windows tick. Helps latency (games, audio), not throughput."),
        ),
        Measure::Background => (
            t(de, "Hintergrund dämpfen", "Calm the background"),
            t(de, "Programme aus der Liste unten bekommen im Hintergrund Effizienzmodus + niedrigere Priorität, im Vordergrund wieder normal.", "Programs from the list below get Efficiency Mode + lower priority in the background, normal again in front."),
        ),
        Measure::Services => (
            t(de, "Dienste pausieren", "Pause services"),
            t(de, "Windows-Suche (Indexer), SysMain und Übermittlungsoptimierung (Update-Downloads) – beim Ausschalten wieder an.", "Windows Search (indexer), SysMain and Delivery Optimization (update downloads) - back on when switched off."),
        ),
        Measure::Foreground => (
            t(de, "Vordergrund bevorzugen", "Prefer the window in front"),
            t(de, "Das aktive Fenster (+ seine Kindprozesse, z. B. die Shell im Terminal) bekommt „Höher“ und keine Drosselung. Mit Admin zusätzlich den Windows-Vordergrund-Boost auf Maximum.", "The active window (+ its child processes, e.g. the shell in a terminal) gets 'above' and no throttling. As admin also the Windows foreground boost at maximum."),
        ),
        Measure::X3d => (
            t(de, "X3D-Kerne aufteilen", "Split X3D cores"),
            t(de, "Spiele → Kerne mit 3D-V-Cache, Arbeit (Entpacken, Compiler, ffmpeg) → die höher taktenden Kerne.", "Games -> cores with 3D V-Cache, work (unpacking, compilers, ffmpeg) -> the higher clocked cores."),
        ),
        Measure::GpuPref => (
            t(de, "Apps auf die dedizierte GPU", "Apps on the dedicated GPU"),
            t(de, "Die Apps aus der GPU-Liste unten nutzen die starke Grafikkarte statt der iGPU (ab ihrem nächsten Start).", "The apps from the GPU list below use the strong graphics card instead of the iGPU (from their next start)."),
        ),
    }
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let de = app.de;
    let (on, outcomes, log, fg, damped, games, work, rules) = match app.shared() {
        Some(s) => (
            s.boost_on,
            s.outcomes.clone(),
            s.log.iter().rev().take(14).cloned().collect::<Vec<_>>(),
            s.foreground.clone(),
            s.damped,
            s.pinned_games,
            s.pinned_work,
            s.rules_applied,
        ),
        None => (false, vec![], vec![], String::new(), 0, 0, 0, 0),
    };
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.columns(2, |cols| {
            let ui = &mut cols[0];
            panel(ui, |ui| {
                ui.horizontal(|ui| {
                    let mut b = on;
                    if toggle(ui, &mut b, true).changed() {
                        app.s.boost_on = b;
                        app.save();
                    }
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(RichText::new(if on { t(de, "Boost ist an", "Boost is on") } else { t(de, "Boost ist aus", "Boost is off") }).size(16.0).family(semibold()));
                        hint(ui, t(de, "Ausschalten setzt alles zurück. Auch nach einem Absturz beim nächsten Start.", "Switching off puts everything back. Also after a crash, on the next start."));
                    });
                });
                if app.topo.is_x3d() && services::is_running("amd3dvcache") {
                    ui.add_space(6.0);
                    note(ui, t(de, "Dein Ryzen X3D: AMDs Treiber legt für Spiele die zweite CCD schlafen und braucht dafür „Ausbalanciert“ + Core-Parking. Darum sind Energieplan und Core-Parking hier aus – „X3D-Kerne aufteilen“ erledigt das gezielter.", "Your Ryzen X3D: AMD's driver parks the second CCD for games and needs 'Balanced' + core parking for that. That's why power plan and core parking are off here - 'Split X3D cores' does it more precisely."));
                }
            });
            ui.add_space(10.0);
            panel(ui, |ui| {
                panel_head(ui, t(de, "Maßnahmen", "Measures"), t(de, "Jede einzeln an/aus. 🛡 = braucht Admin.", "Each one on/off. 🛡 = needs admin."));
                ui.add_space(6.0);
                list(ui, |ui| {
                    for (i, m) in Measure::ALL.into_iter().enumerate() {
                        let (title, desc) = texts(de, m);
                        let shield = if m.needs_admin() || m.partly_admin() { " 🛡" } else { "" };
                        let result = outcomes.iter().find(|o| o.0 == m).map(|o| o.1.clone());
                        let sub = match (&result, on && app.s.has(m)) {
                            (Some(Ok(r)), true) => format!("✓ {r}"),
                            (Some(Err(e)), true) => format!("✗ {e}"),
                            _ => desc.to_string(),
                        };
                        let mut v = app.s.has(m);
                        let mut changed = false;
                        item(ui, i == 0, &format!("{title}{shield}"), &sub, |ui| {
                            changed = toggle(ui, &mut v, true).changed();
                        });
                        if changed {
                            app.s.set_measure(m, v);
                            app.save();
                        }
                    }
                });
            });

            let ui = &mut cols[1];
            panel(ui, |ui| {
                panel_head(ui, t(de, "Jetzt", "Now"), t(de, "Was die Engine gerade macht", "What the engine does right now"));
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    if !fg.is_empty() {
                        chip_blue(ui, &format!("{} {fg}", t(de, "Vorne:", "In front:")));
                    }
                    chip_gray(ui, &format!("{damped} {}", t(de, "gedämpft", "calmed")));
                    if app.topo.is_x3d() {
                        chip_gray(ui, &format!("{games} {} · {work} {}", t(de, "Spiele auf V-Cache", "games on V-Cache"), t(de, "Arbeit auf Takt", "work on clock")));
                    }
                    chip_gray(ui, &format!("{rules} {}", t(de, "Regeln angewendet", "rules applied")));
                });
                ui.add_space(6.0);
                egui::Frame::none().fill(SURFACE_2).rounding(R_MD).inner_margin(egui::Margin::same(10.0)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    if log.is_empty() {
                        hint(ui, "–");
                    }
                    for l in &log {
                        ui.label(RichText::new(l).monospace().size(11.0).color(TEXT_2));
                    }
                });
            });
            ui.add_space(10.0);
            panel(ui, |ui| {
                panel_head(ui, t(de, "Listen", "Lists"), t(de, "Exe-Namen mit Komma getrennt, * als Platzhalter", "Exe names separated by commas, * as wildcard"));
                let mut changed = false;
                label(ui, t(de, "Hintergrund dämpfen", "Calm in the background"));
                changed |= ui.add(egui::TextEdit::multiline(&mut app.s.background).desired_rows(2).desired_width(f32::INFINITY)).lost_focus();
                if app.topo.is_x3d() {
                    label(ui, t(de, "Arbeit → Takt-Kerne", "Work -> clock cores"));
                    changed |= ui.add(egui::TextEdit::multiline(&mut app.s.work).desired_rows(2).desired_width(f32::INFINITY)).lost_focus();
                    label(ui, t(de, "Spiele → V-Cache-Kerne", "Games -> V-Cache cores"));
                    changed |= ui
                        .add(egui::TextEdit::multiline(&mut app.s.games).desired_rows(1).desired_width(f32::INFINITY).hint_text("game.exe, *shooter*"))
                        .lost_focus();
                    ui.horizontal(|ui| {
                        if toggle(ui, &mut app.s.auto_games, true).changed() {
                            changed = true;
                        }
                        ui.label(RichText::new(t(de, "Steam-/Epic-/GOG-Spiele automatisch erkennen", "Detect Steam / Epic / GOG games automatically")).size(12.5));
                    });
                    if app.s.auto_games {
                        note(ui, t(de, "Achtung: Anti-Cheat (Vanguard, EAC, BattlEye) kann schon das Öffnen eines Spiel-Prozesses melden. Bei Online-Spielen mit Anti-Cheat lieber aus lassen.", "Careful: anti-cheat (Vanguard, EAC, BattlEye) may already flag opening a game process. Better keep it off for online games with anti-cheat."));
                    }
                }
                label(ui, t(de, "Auf die dedizierte GPU", "On the dedicated GPU"));
                let mut remove = None;
                for (i, g) in app.s.gpu_apps.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(g).size(12.0));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if icon_btn(ui, "✕", true).clicked() {
                                remove = Some(i);
                            }
                        });
                    });
                }
                if let Some(i) = remove {
                    app.s.gpu_apps.remove(i);
                    changed = true;
                }
                if btn(ui, t(de, "+ Programm (.exe) hinzufügen", "+ Add program (.exe)"), true).clicked() {
                    if let Some(f) = rfd::FileDialog::new().add_filter("exe", &["exe"]).pick_file() {
                        app.s.gpu_apps.push(f.to_string_lossy().to_string());
                        changed = true;
                    }
                }
                if changed {
                    app.save();
                }
            });
        });
    });
}
