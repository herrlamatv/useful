// Tools: what kind of hash is this, check a checksum file, random keys / UUIDs.

use super::theme::*;
use super::{copy_btn, error_line, file_row, finished, job_view, t, Job, Shared};
use eframe::egui::{self, RichText, Ui};
use encsha::sums::{self, Outcome};
use encsha::util::{self, Res};
use encsha::identify;
use std::path::Path;

pub struct ToolsTab {
    id_input: String,
    id_text: String,
    sums_path: String,
    sums_job: Option<Job<Vec<(String, String, Outcome)>>>,
    sums_out: Option<Res<Vec<(String, String, Outcome)>>>,
    rand_n: usize,
    rand_fmt: usize,
    rand_out: String,
}

impl Default for ToolsTab {
    fn default() -> ToolsTab {
        ToolsTab {
            id_input: String::new(),
            id_text: String::new(),
            sums_path: String::new(),
            sums_job: None,
            sums_out: None,
            rand_n: 32,
            rand_fmt: 0,
            rand_out: hex::encode(util::random(32)),
        }
    }
}

impl ToolsTab {
    pub fn drop(&mut self, p: &Path) -> bool {
        if sums::parse(p).is_ok() {
            self.sums_path = p.to_string_lossy().to_string();
            self.check_sums();
            return true;
        }
        false
    }

    fn check_sums(&mut self) {
        let path = self.sums_path.clone();
        self.sums_out = None;
        self.sums_job = Some(Job::start(move |p| {
            let entries = sums::parse(Path::new(&path))?;
            let outcomes = sums::check(&entries, Some(p));
            if outcomes.iter().any(|o| *o == Outcome::Error(util::CANCELLED.into())) {
                return Err(util::CANCELLED.into());
            }
            Ok(entries.into_iter().zip(outcomes).map(|(e, o)| (e.name, e.algo.name().to_string(), o)).collect())
        }));
    }

    fn make_random(&mut self) {
        self.rand_out = match self.rand_fmt {
            0 => hex::encode(util::random(self.rand_n)),
            1 => util::b64(&util::random(self.rand_n)),
            _ => util::uuid_v4(),
        };
    }

    pub fn show(&mut self, ui: &mut Ui, s: &mut Shared) {
        let de = s.de;
        if let Some(r) = finished(&mut self.sums_job) {
            self.sums_out = Some(r);
        }

        panel(ui, |ui| {
            panel_head(ui, t(de, "Was für ein Hash ist das?", "What kind of hash is this?"), t(de, "Erkennung an Länge, Zeichen und Präfix", "Recognized by length, characters and prefix"));
            ui.add_space(4.0);
            text_input(ui, egui::Id::new("id-in"), &mut self.id_input, t(de, "Hash einfügen", "Paste a hash"), ui.available_width(), false);
            if !self.id_input.trim().is_empty() {
                let guesses = identify::identify(&self.id_input);
                if guesses.is_empty() {
                    chip_gray(ui, t(de, "Unbekannt – keine bekannte Hash-Form", "Unknown – not a known hash shape"));
                }
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    for g in guesses {
                        if g.likely {
                            chip_blue(ui, &format!("{} · {}", g.name, t(de, "am wahrscheinlichsten", "most likely")));
                        } else {
                            chip_gray(ui, &g.name);
                        }
                    }
                });
                label(ui, t(de, "Klartext testen (optional)", "Test a plaintext (optional)"));
                text_input(ui, egui::Id::new("id-text"), &mut self.id_text, t(de, "Vermutung, z. B. password", "a guess, e.g. password"), ui.available_width(), false);
                if !self.id_text.is_empty() {
                    let found = identify::find_match(&self.id_input, &self.id_text);
                    if found.is_empty() {
                        chip_red(ui, t(de, "✗ Kein Algorithmus macht aus diesem Text diesen Hash", "✗ No algorithm turns this text into this hash"));
                    } else {
                        chip_on(ui, &format!("✓ {}: {}", t(de, "Bestätigt", "Confirmed"), found.join(", ")));
                    }
                }
            }
        });
        ui.add_space(12.0);

        panel(ui, |ui| {
            panel_head(
                ui,
                t(de, "Prüfsummendatei prüfen", "Check a checksum file"),
                t(de, "SHA256SUMS, datei.iso.sha256, *.md5, BSD-Stil oder .sfv – alle gelisteten Dateien werden geprüft", "SHA256SUMS, file.iso.sha256, *.md5, BSD style or .sfv – every listed file is checked"),
            );
            ui.add_space(4.0);
            let running = self.sums_job.is_some();
            if file_row(ui, "sums-path", &mut self.sums_path, t(de, "Prüfsummendatei wählen oder auf das Fenster ziehen", "Pick a checksum file or drop it on the window"), false) && Path::new(&self.sums_path).is_file() {
                self.check_sums();
            }
            if btn_primary(ui, t(de, "Prüfen", "Check"), 140.0, !running && Path::new(&self.sums_path).is_file()).clicked() {
                self.check_sums();
            }
            if let Some(j) = &self.sums_job {
                job_view(ui, de, j, t(de, "Prüfe", "Checking"));
            }
            match &self.sums_out {
                Some(Ok(rows)) => {
                    let bad = rows.iter().filter(|r| r.2 != Outcome::Ok).count();
                    if bad == 0 {
                        chip_on(ui, &format!("✓ {} {}", rows.len(), t(de, "Dateien – alle stimmen", "files – all match")));
                    } else {
                        chip_red(ui, &format!("✗ {bad} / {} {}", rows.len(), t(de, "Dateien stimmen NICHT", "files do NOT match")));
                    }
                    list(ui, |ui| {
                        for (i, (name, algo, o)) in rows.iter().enumerate() {
                            if i > 0 {
                                row_line(ui);
                            }
                            ui.horizontal(|ui| {
                                ui.set_min_height(26.0);
                                match o {
                                    Outcome::Ok => chip_on(ui, "OK"),
                                    Outcome::Mismatch => chip_red(ui, t(de, "FALSCH", "FAILED")),
                                    Outcome::Error(e) => chip_orange(ui, e),
                                }
                                ui.label(RichText::new(name).size(12.5));
                                ui.label(RichText::new(algo).size(11.5).color(TEXT_3));
                            });
                        }
                    });
                }
                Some(Err(e)) => error_line(ui, de, e),
                None => {}
            }
        });
        ui.add_space(12.0);

        panel(ui, |ui| {
            panel_head(ui, t(de, "Zufall", "Random"), t(de, "Schlüssel, Tokens, Salze und UUIDs aus dem sicheren Zufallsgenerator", "Keys, tokens, salts and UUIDs from the secure random generator"));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let w = segmented_fit_width(ui, &["Hex", "Base64", "UUID v4"], 12.5);
                ui.allocate_ui(egui::vec2(w, 30.0), |ui| {
                    if let Some(i) = segmented_fit(ui, &["Hex", "Base64", "UUID v4"], self.rand_fmt, 12.5, 24.0) {
                        self.rand_fmt = i;
                        self.make_random();
                    }
                });
                if self.rand_fmt < 2 {
                    ui.label(RichText::new(t(de, "Bytes", "Bytes")).size(12.0).color(TEXT_2));
                    if ui.add(egui::Slider::new(&mut self.rand_n, 1..=256)).changed() {
                        self.make_random();
                    }
                    hint(ui, &format!("= {} bit", self.rand_n * 8));
                }
            });
            ui.add(egui::Label::new(RichText::new(&self.rand_out).monospace().size(13.5)).wrap().selectable(true));
            ui.horizontal(|ui| {
                if btn(ui, t(de, "↻ Neu", "↻ New"), true).clicked() {
                    self.make_random();
                }
                copy_btn(ui, s, &self.rand_out.clone());
            });
        });
    }
}
