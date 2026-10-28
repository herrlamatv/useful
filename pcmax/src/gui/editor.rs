// Profile editor: the sliders / switches for one profile (processes tab + rules tab).
// "—" everywhere means "don't touch".

use super::theme::*;
use super::t;
use eframe::egui::{self, RichText, Ui};
use pcmax::cpu::{Eco, Prio};
use pcmax::gpu::GpuPref;
use pcmax::io::IoPrio;
use pcmax::mem::RamMode;
use pcmax::profile::Profile;
use pcmax::topo::Topo;

fn prio_names(de: bool) -> [&'static str; 7] {
    if de {
        ["—", "Leerlauf", "Niedriger", "Normal", "Höher", "Hoch", "Echtzeit"]
    } else {
        ["—", "Idle", "Below", "Normal", "Above", "High", "Realtime"]
    }
}

/// index 0 = None, 1.. = Prio::ALL
fn prio_seg(ui: &mut Ui, de: bool, v: &mut Option<Prio>) -> bool {
    let sel = v.map(|p| p.index() + 1).unwrap_or(0);
    if let Some(i) = segmented(ui, &prio_names(de), sel, true) {
        *v = if i == 0 { None } else { Some(Prio::ALL[i - 1]) };
        return true;
    }
    false
}

fn row_label(ui: &mut Ui, text: &str, admin: bool) {
    ui.horizontal(|ui| {
        label(ui, text);
        if admin {
            ui.label(RichText::new("🛡").size(11.0).color(TEXT_3)).on_hover_text("admin");
        }
    });
}

/// returns true when something changed
pub fn edit(ui: &mut Ui, de: bool, p: &mut Profile, topo: &Topo, ram_total: u64, id: &str) -> bool {
    let mut changed = false;
    let threads = topo.logical as f32;

    // ---- CPU range ----
    row_label(ui, t(de, "CPU-Anteil am ganzen PC", "CPU share of the whole PC"), false);
    let (mut min, mut max) = (p.cpu_min as f32, p.cpu_max as f32);
    ui.horizontal(|ui| {
        ui.label(RichText::new(t(de, "Minimum", "Minimum")).size(12.0).color(TEXT_2));
        let r = ui.add(egui::Slider::new(&mut min, 0.0..=100.0).step_by(1.0).fixed_decimals(0).suffix(" %"));
        if r.changed() {
            changed = true;
            if min > max {
                max = min.max(1.0);
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new(t(de, "Maximum", "Maximum")).size(12.0).color(TEXT_2));
        let r = ui.add(egui::Slider::new(&mut max, 1.0..=100.0).step_by(1.0).fixed_decimals(0).suffix(" %"));
        if r.changed() {
            changed = true;
            if max < min {
                min = max;
            }
        }
    });
    p.cpu_min = min as u8;
    p.cpu_max = max as u8;
    if p.cpu_min > 0 || p.cpu_max < 100 {
        hint(
            ui,
            &format!(
                "≈ {:.1} – {:.1} {} {} {}",
                min / 100.0 * threads,
                max / 100.0 * threads,
                t(de, "von", "of"),
                topo.logical,
                t(de, "Threads. Das Minimum hebt die Priorität auf „Höher“, sonst würde Windows es nicht einhalten.", "threads. The minimum also lifts the priority to 'above', Windows wouldn't keep it otherwise.")
            ),
        );
    } else {
        hint(ui, t(de, "100 % = kein Limit", "100 % = no limit"));
    }
    ui.add_space(4.0);

    // ---- priority ----
    row_label(ui, t(de, "CPU-Priorität", "CPU priority"), p.prio == Some(Prio::Realtime));
    changed |= prio_seg(ui, de, &mut p.prio);
    if p.prio == Some(Prio::Realtime) {
        note(ui, t(de, "Echtzeit kann den ganzen PC einfrieren (Maus, Ton, Eingaben). Braucht Admin und wird vor dem Anwenden bestätigt.", "Realtime can freeze the whole PC (mouse, sound, input). Needs admin and is confirmed before it is applied."));
    }

    // ---- cores ----
    let mut cores: Vec<(&str, &str)> = vec![("—", ""), (t(de, "Alle", "All"), "all")];
    if topo.is_x3d() {
        cores.push(("V-Cache", "vcache"));
        cores.push((t(de, "Takt", "Clock"), "freq"));
    }
    if topo.pcores.is_some() {
        cores.push(("P-Cores", "p"));
        cores.push(("E-Cores", "e"));
    }
    cores.push((t(de, "Eigene", "Custom"), "custom"));
    row_label(ui, t(de, "Kerne", "Cores"), false);
    let cur = p.cores.clone().unwrap_or_default();
    let known = cores.iter().position(|(_, k)| *k == cur.as_str());
    let sel = match (&p.cores, known) {
        (None, _) => 0,
        (Some(_), Some(i)) => i,
        (Some(_), None) => cores.len() - 1,
    };
    let names: Vec<&str> = cores.iter().map(|c| c.0).collect();
    if let Some(i) = segmented(ui, &names, sel, true) {
        p.cores = match cores[i].1 {
            "" => None,
            "custom" => Some(format!("0-{}", topo.logical.saturating_sub(1))),
            k => Some(k.to_string()),
        };
        changed = true;
    }
    if sel == cores.len() - 1 || (p.cores.is_some() && known.is_none()) {
        let mut text = p.cores.clone().unwrap_or_default();
        ui.horizontal(|ui| {
            if text_input(ui, egui::Id::new((id, "cores")), &mut text, "0-5,12-17", 180.0).changed() {
                p.cores = Some(text.clone());
                changed = true;
            }
            match topo.mask_for(&text) {
                Ok(m) => hint(ui, &format!("{} {}", m.count_ones(), t(de, "Threads", "threads"))),
                Err(e) => {
                    ui.label(RichText::new(e).size(11.5).color(RED));
                }
            }
        });
    }
    if topo.is_x3d() {
        hint(ui, t(de, "V-Cache = 3D-Cache-Kerne (Spiele), Takt = die höher taktenden Kerne (Entpacken, Compiler).", "V-Cache = 3D cache cores (games), Clock = the higher clocked cores (unpacking, compilers)."));
    }
    ui.add_space(4.0);

    // ---- efficiency mode ----
    row_label(ui, t(de, "Effizienzmodus (Drosselung)", "Efficiency Mode (throttling)"), false);
    let eco_names = if de { ["—", "Windows", "Aus", "An"] } else { ["—", "Windows", "Off", "On"] };
    let sel = match p.eco {
        None => 0,
        Some(Eco::Auto) => 1,
        Some(Eco::Off) => 2,
        Some(Eco::On) => 3,
    };
    if let Some(i) = segmented(ui, &eco_names, sel, true) {
        p.eco = [None, Some(Eco::Auto), Some(Eco::Off), Some(Eco::On)][i];
        changed = true;
    }
    ui.add_space(4.0);

    // ---- RAM ----
    row_label(ui, "RAM", p.ram.is_some() && p.ram_mode == RamMode::Hard && false);
    let mut limit = p.ram.is_some();
    ui.horizontal(|ui| {
        if toggle(ui, &mut limit, true).changed() {
            p.ram = if limit { Some(4 << 30) } else { None };
            changed = true;
        }
        ui.label(RichText::new(t(de, "RAM begrenzen", "Limit RAM")).size(12.5));
    });
    if let Some(bytes) = p.ram {
        let total_gb = (ram_total as f64 / (1u64 << 30) as f64).max(1.0) as f32;
        let mut gb = bytes as f32 / (1u64 << 30) as f32;
        if ui.add(egui::Slider::new(&mut gb, 0.25..=total_gb).logarithmic(true).suffix(" GB").max_decimals(2)).changed() {
            p.ram = Some((gb as f64 * (1u64 << 30) as f64) as u64);
            changed = true;
        }
        let modes = if de { ["Weich (auslagern)", "Hart (Speicherfehler)"] } else { ["Soft (page out)", "Hard (out of memory)"] };
        let sel = if p.ram_mode == RamMode::Soft { 0 } else { 1 };
        if let Some(i) = segmented(ui, &modes, sel, true) {
            p.ram_mode = if i == 0 { RamMode::Soft } else { RamMode::Hard };
            changed = true;
        }
        hint(
            ui,
            if p.ram_mode == RamMode::Soft {
                t(de, "Weich: pro Prozess, darüber lagert Windows aus – langsamer, aber kein Absturz.", "Soft: per process, above it Windows pages out - slower, but no crash.")
            } else {
                t(de, "Hart: für alle Prozesse zusammen, darüber bekommt das Programm keinen Speicher mehr (kann abstürzen).", "Hard: all processes together, above it the program gets no more memory (may crash).")
            },
        );
    }
    let rp = ["—", "1", "2", "3", "4", "5 (normal)"];
    ui.horizontal(|ui| {
        ui.label(RichText::new(t(de, "RAM-Priorität", "RAM priority")).size(12.0).color(TEXT_2));
    });
    let sel = p.ram_prio.map(|x| x as usize).unwrap_or(0);
    if let Some(i) = segmented(ui, &rp, sel, true) {
        p.ram_prio = if i == 0 { None } else { Some(i as u8) };
        changed = true;
    }
    ui.horizontal(|ui| {
        if toggle(ui, &mut p.keep_ram, true).changed() {
            changed = true;
        }
        ui.label(RichText::new(t(de, "Im RAM halten (Windows lagert es zuletzt aus)", "Keep in RAM (Windows pages it out last)")).size(12.5));
    });
    ui.add_space(4.0);

    // ---- I/O ----
    row_label(ui, t(de, "Festplatten-Priorität (I/O)", "Disk priority (I/O)"), p.io == Some(IoPrio::High));
    let io_names = if de { ["—", "Sehr niedrig", "Niedrig", "Normal", "Hoch"] } else { ["—", "Very low", "Low", "Normal", "High"] };
    let sel = p.io.map(|x| x.index() + 1).unwrap_or(0);
    if let Some(i) = segmented(ui, &io_names, sel, true) {
        p.io = if i == 0 { None } else { Some(IoPrio::ALL[i - 1]) };
        changed = true;
    }
    ui.add_space(4.0);

    // ---- GPU ----
    row_label(ui, t(de, "GPU-Priorität", "GPU priority"), p.gpu == Some(Prio::Realtime));
    changed |= prio_seg(ui, de, &mut p.gpu);
    row_label(ui, t(de, "Welche GPU (ab dem nächsten Start der App)", "Which GPU (from the next start of the app)"), false);
    let gp = if de { ["—", "Windows", "iGPU (sparsam)", "Dedizierte GPU"] } else { ["—", "Windows", "iGPU (saving)", "Dedicated GPU"] };
    let sel = match p.gpu_pref {
        None => 0,
        Some(GpuPref::Auto) => 1,
        Some(GpuPref::Saving) => 2,
        Some(GpuPref::High) => 3,
    };
    if let Some(i) = segmented(ui, &gp, sel, true) {
        p.gpu_pref = [None, Some(GpuPref::Auto), Some(GpuPref::Saving), Some(GpuPref::High)][i];
        changed = true;
    }
    hint(ui, t(de, "Ein GPU-Limit in % gibt es in Windows nicht – nur Priorität und Wahl der Grafikkarte.", "Windows has no GPU limit in % - only priority and the choice of the graphics card."));
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        if toggle(ui, &mut p.tree, true).changed() {
            changed = true;
        }
        ui.label(RichText::new(t(de, "Auch für Kindprozesse (z. B. die Shell im Terminal-Fenster)", "Also for child processes (e.g. the shell in a terminal window)")).size(12.5));
    });
    changed
}
