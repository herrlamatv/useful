// Monitor: every logical processor (X3D: V-Cache CCD blue, clock CCD orange), CPU / GPU / RAM history,
// the programs that use the most.

use super::theme::*;
use super::{t, App};
use eframe::egui::{self, Align, Layout, RichText, Ui, Vec2};
use pcmax::win::fmt_bytes;

pub fn show(app: &mut App, ui: &mut Ui) {
    let de = app.de;
    let s = app.sample.clone();
    let topo = app.topo;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        panel(ui, |ui| {
            ui.horizontal(|ui| {
                panel_head(ui, &format!("CPU {:.0} %", s.cpu_total), &format!("{} {} · {} {}", topo.logical, t(de, "Threads", "threads"), topo.cores.len(), t(de, "Kerne", "cores")));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if topo.is_x3d() {
                        chip(ui, t(de, "Takt-CCD", "clock CCD"), FREQ.gamma_multiply(0.2), egui::Color32::from_rgb(0x8a, 0x52, 0x00));
                        chip(ui, "V-Cache CCD", ACCENT_SOFT, egui::Color32::from_rgb(0x0a, 0x5b, 0xb5));
                    }
                });
            });
            ui.add_space(6.0);
            let n = s.cores.len().max(1);
            let gap = 4.0;
            let w = ((ui.available_width() - gap * (n as f32 - 1.0)) / n as f32).clamp(6.0, 40.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for (i, v) in s.cores.iter().enumerate() {
                    let bit = 1u64 << i;
                    let color = if topo.vcache.is_some_and(|m| m & bit != 0) {
                        VCACHE
                    } else if topo.freq.is_some_and(|m| m & bit != 0) {
                        FREQ
                    } else {
                        ACCENT
                    };
                    bar(ui, Vec2::new(w, 90.0), *v, color).on_hover_text(format!("{} {i}: {:.0} %", t(de, "Prozessor", "processor"), v));
                }
            });
            ui.add_space(8.0);
            sparkline(ui, Vec2::new(ui.available_width(), 70.0), &app.cpu_hist.iter().copied().collect::<Vec<_>>(), RUST);
            hint(ui, t(de, "Letzte 90 Sekunden", "Last 90 seconds"));
        });
        ui.add_space(10.0);
        ui.columns(2, |cols| {
            panel(&mut cols[0], |ui| {
                panel_head(ui, &format!("GPU {:.0} %", s.gpu_total), t(de, "Am stärksten ausgelastete GPU-Einheit (wie im Task-Manager)", "Busiest GPU engine (like Task Manager)"));
                ui.add_space(6.0);
                sparkline(ui, Vec2::new(ui.available_width(), 60.0), &app.gpu_hist.iter().copied().collect::<Vec<_>>(), ACCENT);
            });
            panel(&mut cols[1], |ui| {
                panel_head(ui, &format!("RAM {} / {}", fmt_bytes(s.ram_used), fmt_bytes(s.ram_total)), t(de, "Belegt", "In use"));
                ui.add_space(6.0);
                sparkline(ui, Vec2::new(ui.available_width(), 60.0), &app.ram_hist.iter().copied().collect::<Vec<_>>(), GREEN);
            });
        });
        ui.add_space(10.0);
        panel(ui, |ui| {
            panel_head(ui, t(de, "Größte Verbraucher", "Top users"), t(de, "Klick = im Tab Prozesse öffnen", "Click = open in the processes tab"));
            ui.add_space(4.0);
            let mut open = None;
            egui::Grid::new("top").num_columns(5).spacing(Vec2::new(18.0, 6.0)).striped(false).show(ui, |ui| {
                for h in [t(de, "Programm", "Program"), "PID", "CPU", "RAM", "GPU"] {
                    ui.label(RichText::new(h).size(11.0).color(TEXT_3));
                }
                ui.end_row();
                for r in app.rows.iter().take(12) {
                    if ui.link(RichText::new(&r.name).size(12.5)).clicked() {
                        open = Some(r.pid);
                    }
                    ui.label(RichText::new(r.pid.to_string()).size(12.0).color(TEXT_2));
                    ui.label(RichText::new(format!("{:.1} %", r.cpu)).size(12.0));
                    ui.label(RichText::new(fmt_bytes(r.ram)).size(12.0).color(TEXT_2));
                    ui.label(RichText::new(if r.gpu >= 0.5 { format!("{:.0} %", r.gpu) } else { "–".into() }).size(12.0).color(TEXT_2));
                    ui.end_row();
                }
            });
            if let Some(pid) = open {
                app.select(pid);
                app.tab = super::Tab::Procs;
            }
        });
    });
}
