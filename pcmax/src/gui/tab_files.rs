// Unpack + copy on all cores, with progress. Archives / folders can also be dropped on the window.

use super::theme::*;
use super::{t, App, Job};
use eframe::egui::{self, RichText, Ui};
use pcmax::fsx::Summary;
use pcmax::win::fmt_bytes;
use pcmax::{copy, extract};
use std::path::PathBuf;
use std::sync::atomic::Ordering;

fn open_folder(p: &std::path::Path) {
    let _ = std::process::Command::new("explorer.exe").arg(p).spawn();
}

fn threads_slider(ui: &mut Ui, de: bool, v: &mut usize, max: usize) -> bool {
    let mut n = if *v == 0 { max } else { (*v).min(max) } as f32;
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new("Threads").size(12.0).color(TEXT_2));
        if ui.add(egui::Slider::new(&mut n, 1.0..=max as f32).step_by(1.0).fixed_decimals(0)).changed() {
            *v = if n as usize >= max { 0 } else { n as usize };
            changed = true;
        }
        if *v == 0 {
            hint(ui, t(de, "(alle)", "(all)"));
        }
    });
    changed
}

/// progress / result of a running or finished job; returns true when `done_label` was clicked
pub(super) fn job_view(ui: &mut Ui, de: bool, job: &mut Job, done_label: &str) -> bool {
    let running = job.poll();
    let p = &job.prog;
    let mut clear = false;
    if running {
        progress(ui, p.fraction());
        let secs = job.started.elapsed().as_secs_f64().max(0.001);
        let done = p.done_bytes.load(Ordering::Relaxed);
        ui.label(
            RichText::new(format!(
                "{} / {} {} · {} · {}/s · {}",
                p.done_files.load(Ordering::Relaxed),
                p.total_files.load(Ordering::Relaxed),
                t(de, "Dateien", "files"),
                fmt_bytes(done),
                fmt_bytes((done as f64 / secs) as u64),
                p.phase()
            ))
            .size(12.0)
            .color(TEXT_2),
        );
        if btn(ui, t(de, "Abbrechen", "Cancel"), true).clicked() {
            p.cancel.store(true, Ordering::Relaxed);
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
    } else if let Some(r) = &job.result {
        let dest = match r {
            Ok(s) => {
                summary(ui, de, s);
                Some(s.dest.clone())
            }
            Err(e) => {
                chip_red(ui, t(de, "Fehler", "Error"));
                ui.label(RichText::new(e).size(12.0).color(RED));
                None
            }
        };
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if let Some(d) = dest {
                if btn(ui, t(de, "Ordner öffnen", "Open folder"), true).clicked() {
                    open_folder(&d);
                }
            }
            if btn(ui, done_label, true).clicked() {
                clear = true;
            }
        });
    }
    clear
}

/// "50000 Dateien, 163 MB in 13.7 s (11.9 MB/s)"
fn summary_line(de: bool, s: &Summary) -> String {
    let speed = if s.secs > 0.0 { s.bytes as f64 / s.secs } else { 0.0 };
    format!("{} {}, {} in {:.1} s ({}/s)", s.files, t(de, "Dateien", "files"), fmt_bytes(s.bytes), s.secs, fmt_bytes(speed as u64))
}

fn summary(ui: &mut Ui, de: bool, s: &Summary) {
    ui.horizontal(|ui| {
        chip_on(ui, t(de, "Fertig", "Done"));
        ui.label(RichText::new(summary_line(de, s)).size(12.5));
    });
    for n in &s.notes {
        hint(ui, n);
    }
    for e in s.errors.iter().take(8) {
        ui.label(RichText::new(format!("✗ {e}")).size(11.5).color(RED));
    }
    if s.errors.len() > 8 {
        hint(ui, &format!("… {} {}", s.errors.len() - 8, t(de, "weitere Fehler", "more errors")));
    }
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let de = app.de;
    let max = app.topo.logical as usize;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.columns(2, |cols| {
            // ---------------- unpack ----------------
            let ui = &mut cols[0];
            panel(ui, |ui| {
                panel_head(
                    ui,
                    t(de, "Entpacken – mit allen Kernen", "Unpack - on all cores"),
                    "zip · 7z · tar(.gz/.zst/.xz/.bz2) · gz · zst · xz · bz2 · rar (WinRAR / 7-Zip)",
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    text_input(ui, egui::Id::new("ex-path"), &mut app.ex_path, t(de, "Archiv (oder aufs Fenster ziehen)", "Archive (or drop it on the window)"), ui.available_width() - 110.0);
                    if btn(ui, t(de, "Wählen …", "Choose ..."), true).clicked() {
                        let exts: Vec<&str> = extract::EXTENSIONS.iter().map(|e| e.trim_start_matches('.')).collect();
                        if let Some(f) = rfd::FileDialog::new().add_filter("Archive", &exts).pick_file() {
                            app.ex_dest = extract::default_dest(&f).to_string_lossy().to_string();
                            app.ex_path = f.to_string_lossy().to_string();
                        }
                    }
                });
                ui.horizontal(|ui| {
                    text_input(ui, egui::Id::new("ex-dest"), &mut app.ex_dest, t(de, "Ziel (leer = neben dem Archiv)", "Target (empty = next to the archive)"), ui.available_width() - 110.0);
                    if btn(ui, t(de, "Ziel …", "Target ..."), true).clicked() {
                        if let Some(d) = rfd::FileDialog::new().pick_folder() {
                            app.ex_dest = d.to_string_lossy().to_string();
                        }
                    }
                });
                ui.add_space(4.0);
                let mut changed = threads_slider(ui, de, &mut app.s.extract_threads, max);
                ui.horizontal(|ui| {
                    changed |= toggle(ui, &mut app.s.overwrite, true).changed();
                    ui.label(RichText::new(t(de, "Vorhandene Dateien überschreiben", "Overwrite existing files")).size(12.5));
                });
                ui.horizontal(|ui| {
                    let mut v = app.s.defender_skip && app.admin;
                    if toggle(ui, &mut v, app.admin).changed() {
                        app.s.defender_skip = v;
                        changed = true;
                    }
                    ui.label(RichText::new(t(de, "Defender überspringt den Zielordner beim Entpacken 🛡", "Defender skips the target folder while unpacking 🛡")).size(12.5));
                });
                if app.s.defender_skip && app.admin {
                    ui.horizontal(|ui| {
                        changed |= toggle(ui, &mut app.s.scan_after, true).changed();
                        ui.label(RichText::new(t(de, "Danach den Ordner mit Defender scannen", "Scan the folder with Defender afterwards")).size(12.5));
                    });
                    note(ui, t(de, "Nur für Archive, denen du vertraust. Die Ausnahme wird direkt danach wieder entfernt (auch nach einem Absturz beim nächsten Start).", "Only for archives you trust. The exclusion is removed right after (also after a crash, on the next start)."));
                } else if !app.admin {
                    hint(ui, t(de, "Die Defender-Ausnahme braucht Admin-Rechte (oben rechts).", "The Defender exclusion needs admin rights (top right)."));
                }
                if changed {
                    app.save();
                }
                ui.add_space(6.0);
                let busy = app.ex_job.as_ref().is_some_and(|j| j.handle.is_some());
                if app.ex_job.is_none() || !busy {
                    let ok = !app.ex_path.trim().is_empty();
                    if btn_primary(ui, t(de, "Entpacken", "Unpack"), 160.0, ok && !busy).clicked() {
                        let src = PathBuf::from(app.ex_path.trim());
                        let dest = if app.ex_dest.trim().is_empty() { extract::default_dest(&src) } else { PathBuf::from(app.ex_dest.trim()) };
                        let opts = extract::Options {
                            threads: if app.s.extract_threads == 0 { max } else { app.s.extract_threads },
                            overwrite: app.s.overwrite,
                            defender_skip: app.s.defender_skip && app.admin,
                            scan_after: app.s.defender_skip && app.admin && app.s.scan_after,
                        };
                        app.ex_job = Some(Job::start(move |p| extract::extract(&src, &dest, &opts, p)));
                    }
                }
                if let Some(j) = app.ex_job.as_mut() {
                    ui.add_space(6.0);
                    if job_view(ui, de, j, "OK") {
                        app.ex_job = None;
                    }
                }
            });

            // ---------------- copy ----------------
            let ui = &mut cols[1];
            panel(ui, |ui| {
                panel_head(
                    ui,
                    t(de, "Kopieren – mit allen Kernen", "Copy - on all cores"),
                    t(de, "Viele kleine Dateien: deutlich schneller als der Explorer", "Many small files: much faster than Explorer"),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    text_input(ui, egui::Id::new("cp-src"), &mut app.cp_src, t(de, "Quelle (Ordner oder Datei)", "Source (folder or file)"), ui.available_width() - 110.0);
                    if btn(ui, t(de, "Ordner …", "Folder ..."), true).clicked() {
                        if let Some(d) = rfd::FileDialog::new().pick_folder() {
                            app.cp_src = d.to_string_lossy().to_string();
                        }
                    }
                });
                ui.horizontal(|ui| {
                    text_input(ui, egui::Id::new("cp-dst"), &mut app.cp_dst, t(de, "In diesen Ordner", "Into this folder"), ui.available_width() - 110.0);
                    if btn(ui, t(de, "Ziel …", "Target ..."), true).clicked() {
                        if let Some(d) = rfd::FileDialog::new().pick_folder() {
                            app.cp_dst = d.to_string_lossy().to_string();
                        }
                    }
                });
                ui.add_space(4.0);
                let mut changed = threads_slider(ui, de, &mut app.s.copy_threads, max);
                ui.horizontal(|ui| {
                    changed |= toggle(ui, &mut app.s.verify_copy, true).changed();
                    ui.label(RichText::new(t(de, "Danach jedes Byte vergleichen", "Compare every byte afterwards")).size(12.5));
                });
                if changed {
                    app.save();
                }
                ui.add_space(6.0);
                let busy = app.cp_job.as_ref().is_some_and(|j| j.handle.is_some());
                let ok = !app.cp_src.trim().is_empty() && !app.cp_dst.trim().is_empty();
                if btn_primary(ui, t(de, "Kopieren", "Copy"), 160.0, ok && !busy).clicked() {
                    let (src, dst) = (PathBuf::from(app.cp_src.trim()), PathBuf::from(app.cp_dst.trim()));
                    let opts = copy::Options {
                        threads: if app.s.copy_threads == 0 { max.min(16) } else { app.s.copy_threads },
                        overwrite: app.s.overwrite,
                        verify: app.s.verify_copy,
                    };
                    app.cp_job = Some(Job::start(move |p| copy::copy(&src, &dst, &opts, p)));
                }
                if let Some(j) = app.cp_job.as_mut() {
                    ui.add_space(6.0);
                    if job_view(ui, de, j, "OK") {
                        app.cp_job = None;
                    }
                }
            });
        });
    });
}
