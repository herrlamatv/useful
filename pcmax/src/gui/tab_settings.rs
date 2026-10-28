// Settings: language, tray, autostart, right-click menu, PowerShell `fast`, hotkeys, admin, undo journal.

use super::theme::*;
use super::{german, t, App};
use eframe::egui::{self, RichText, Ui};
use pcmax::{shell, topo, undo};

/// what is installed (reading the PowerShell profiles starts pwsh -> done on a thread)
#[derive(Clone, Default)]
pub struct ShellState {
    pub menu: bool,
    pub autostart: bool,
    pub task: bool,
    pub pwsh: bool,
}

impl ShellState {
    pub fn read() -> ShellState {
        ShellState { menu: shell::menu_installed(), autostart: shell::autostart_installed(), task: shell::task_installed(), pwsh: shell::pwsh_installed() }
    }
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let de = app.de;
    let st = app.shell_state.clone();
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.columns(2, |cols| {
            let ui = &mut cols[0];
            panel(ui, |ui| {
                panel_head(ui, t(de, "Allgemein", "General"), "");
                label(ui, t(de, "Sprache", "Language"));
                let langs = ["Auto", "Deutsch", "English"];
                let sel = match app.s.lang.as_str() {
                    "de" => 1,
                    "en" => 2,
                    _ => 0,
                };
                if let Some(i) = segmented(ui, &langs, sel, true) {
                    app.s.lang = ["auto", "de", "en"][i].into();
                    app.de = german(&app.s.lang);
                    app.save();
                }
                ui.add_space(6.0);
                list(ui, |ui| {
                    let mut v = app.s.close_to_tray;
                    let mut changed = false;
                    item(ui, true, t(de, "Schließen = in den Tray", "Close = to the tray"), t(de, "Regeln, Boost und Hotkeys laufen weiter. Beenden über das Tray-Menü.", "Rules, boost and hotkeys keep running. Quit from the tray menu."), |ui| {
                        changed = toggle(ui, &mut v, true).changed();
                    });
                    if changed {
                        app.s.close_to_tray = v;
                        app.save();
                    }
                    let mut v = app.s.hotkeys;
                    let mut changed = false;
                    item(ui, false, t(de, "Hotkeys", "Hotkeys"), t(de, "Strg+Alt+↑ Boost · Strg+Alt+↓ bremsen · Strg+Alt+0 normal – für das Fenster vorne", "Ctrl+Alt+Up boost · Ctrl+Alt+Down calm · Ctrl+Alt+0 normal - for the window in front"), |ui| {
                        changed = toggle(ui, &mut v, true).changed();
                    });
                    if changed {
                        app.s.hotkeys = v;
                        app.save();
                    }
                });
            });
            ui.add_space(10.0);
            panel(ui, |ui| {
                panel_head(ui, t(de, "Windows-Integration", "Windows integration"), t(de, "Alles ohne Admin, außer dem Autostart mit Admin-Rechten", "Everything without admin, except autostart with admin rights"));
                ui.add_space(4.0);
                let Some(st) = st.clone() else {
                    progress(ui, None);
                    hint(ui, t(de, "Lese den Status …", "Reading the status ..."));
                    return;
                };
                let mut reload = false;
                list(ui, |ui| {
                    let mut v = st.menu;
                    let mut changed = false;
                    item(ui, true, t(de, "Rechtsklick-Menü", "Right-click menu"), t(de, "„Mit pcmax entpacken“ auf Archiven, „Mit pcmax kopieren nach …“ auf Ordnern (Win 11: unter „Weitere Optionen“)", "'Unpack with pcmax' on archives, 'Copy with pcmax to ...' on folders (Win 11: under 'Show more options')"), |ui| {
                        changed = toggle(ui, &mut v, true).changed();
                    });
                    if changed {
                        let r = if v { shell::install_menu(de) } else { shell::uninstall_menu() };
                        report(app, r);
                        reload = true;
                    }
                    let mut v = st.autostart;
                    let mut changed = false;
                    let sub = if st.task { t(de, "Läuft mit Admin-Rechten (Aufgabenplanung)", "Runs with admin rights (task scheduler)") } else { t(de, "pcmax startet beim Anmelden im Tray", "pcmax starts in the tray at logon") };
                    item(ui, false, "Autostart", sub, |ui| {
                        changed = toggle(ui, &mut v, true).changed();
                    });
                    if changed {
                        let r = if v { shell::install_autostart(false) } else { shell::uninstall_autostart() };
                        report(app, r);
                        reload = true;
                    }
                    if app.admin {
                        let mut v = st.task;
                        let mut changed = false;
                        item(ui, false, t(de, "Autostart mit Admin-Rechten 🛡", "Autostart with admin rights 🛡"), t(de, "Ohne UAC-Abfrage, alle Boost-Maßnahmen verfügbar", "Without a UAC prompt, all boost measures available"), |ui| {
                            changed = toggle(ui, &mut v, true).changed();
                        });
                        if changed {
                            let r = if v { shell::install_autostart(true) } else { shell::uninstall_autostart() };
                            report(app, r);
                            reload = true;
                        }
                    }
                    let mut v = st.pwsh;
                    let mut changed = false;
                    item(ui, false, t(de, "PowerShell: fast { … }", "PowerShell: fast { ... }"), t(de, "fast { 1..10000000 } gibt alles in einem Rutsch aus statt Zeile für Zeile (PowerShell 7 + Windows PowerShell)", "fast { 1..10000000 } prints everything in one go instead of line by line (PowerShell 7 + Windows PowerShell)"), |ui| {
                        changed = toggle(ui, &mut v, true).changed();
                    });
                    if changed {
                        let r = if v { shell::install_pwsh().map(|_| ()) } else { shell::uninstall_pwsh() };
                        report(app, r);
                        reload = true;
                    }
                });
                if reload {
                    app.shell_state = None;
                    app.load_shell_state();
                }
            });

            let ui = &mut cols[1];
            panel(ui, |ui| {
                panel_head(ui, t(de, "Rechte", "Rights"), "");
                if app.admin {
                    chip_on(ui, t(de, "Läuft als Admin – alles verfügbar", "Runs as admin - everything available"));
                } else {
                    hint(ui, t(de, "Ohne Admin fehlen: Echtzeit, I/O „Hoch“, Dienste pausieren, Core-Parking, Windows-Vordergrund-Boost, Defender-Ausnahme, fremde/System-Prozesse.", "Without admin these are missing: realtime, I/O 'high', pausing services, core parking, Windows foreground boost, Defender exclusion, other users' / system processes."));
                    ui.add_space(4.0);
                    if btn(ui, t(de, "🛡 Als Admin neu starten", "🛡 Restart as admin"), true).clicked() {
                        app.restart_as_admin();
                    }
                }
            });
            ui.add_space(10.0);
            panel(ui, |ui| {
                panel_head(ui, t(de, "Rückgängig-Journal", "Undo journal"), t(de, "Jede Systemänderung steht hier, bevor sie gemacht wird – nach einem Absturz setzt pcmax sie beim nächsten Start zurück.", "Every system change is written here before it is made - after a crash pcmax reverts it on the next start."));
                let j = undo::load();
                ui.add_space(4.0);
                if j.is_empty() {
                    chip_on(ui, t(de, "Nichts offen", "Nothing open"));
                } else {
                    for e in &j {
                        ui.label(RichText::new(format!("• {}", e.describe())).size(12.0));
                    }
                    if btn(ui, t(de, "Jetzt alles zurücksetzen", "Revert everything now"), true).clicked() {
                        let bad: Vec<String> = undo::revert_all().into_iter().filter_map(|(w, r)| r.err().map(|e| format!("{w}: {e}"))).collect();
                        if bad.is_empty() {
                            app.toast(t(de, "Alles zurückgesetzt", "Everything reverted"), true);
                        } else {
                            app.toast(bad.join("; "), false);
                        }
                    }
                }
            });
            ui.add_space(10.0);
            panel(ui, |ui| {
                let tp = topo::get();
                panel_head(ui, t(de, "Dieser PC", "This PC"), "");
                hint(ui, &format!("{} {} · {} {}", tp.logical, t(de, "Threads", "threads"), tp.cores.len(), t(de, "Kerne", "cores")));
                for c in &tp.l3 {
                    let tag = if tp.vcache == Some(c.mask) { " – 3D V-Cache" } else { "" };
                    hint(ui, &format!("L3 {} MB: {}{tag}", c.size >> 20, topo::format_list(c.mask)));
                }
                if let Some(d) = pcmax::settings::dir() {
                    hint(ui, &format!("{}: {}", t(de, "Einstellungen", "Settings"), d.display()));
                }
                hint(ui, &format!("pcmax {}", env!("CARGO_PKG_VERSION")));
            });
        });
    });
}

fn report(app: &mut App, r: Result<(), String>) {
    match r {
        Ok(()) => app.toast(t(app.de, "Gespeichert", "Saved"), true),
        Err(e) => app.toast(e, false),
    }
}
