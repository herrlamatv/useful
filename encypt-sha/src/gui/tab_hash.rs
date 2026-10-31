// Hash text (live while typing) or files with any number of algorithms, HMAC with a key,
// compare with an expected value, save a checksum file.

use super::theme::*;
use super::{copy_btn, error_line, finished, job_view, show_in_folder, t, Job, Shared};
use eframe::egui::{self, RichText, Ui};
use encsha::hash::{self, Algo, Class, Fmt};
use encsha::util::{fmt_bytes, Res};
use std::path::PathBuf;

type Digests = Vec<(Algo, Option<Vec<u8>>)>;

pub struct FileResult {
    path: PathBuf,
    size: u64,
    digests: Res<Digests>,
}

pub struct HashTab {
    files_mode: bool,
    text: String,
    files: Vec<PathBuf>,
    on: Vec<Algo>,
    hmac_key: String,
    compare: String,
    fmt: Fmt,
    /// text results and what they were made from
    text_out: Digests,
    text_for: Option<(String, Vec<Algo>, String)>,
    job: Option<Job<Vec<FileResult>>>,
    results: Vec<FileResult>,
}

impl Default for HashTab {
    fn default() -> HashTab {
        HashTab {
            files_mode: false,
            text: String::new(),
            files: vec![],
            on: Algo::COMMON.to_vec(),
            hmac_key: String::new(),
            compare: String::new(),
            fmt: Fmt::Hex,
            text_out: vec![],
            text_for: None,
            job: None,
            results: vec![],
        }
    }
}

/// hashes or HMACs of one input for the picked algorithms (None = no HMAC / text only)
fn digest(algos: &[Algo], key: &str, r: impl std::io::Read + Send, prog: Option<&encsha::util::Progress>) -> Res<Digests> {
    if key.is_empty() {
        let d = hash::hash_reader(algos, r, prog)?;
        Ok(algos.iter().copied().zip(d.into_iter().map(Some)).collect())
    } else {
        let d = hash::hmac_reader(algos, key.as_bytes(), r, prog)?;
        Ok(algos.iter().copied().zip(d).collect())
    }
}

impl HashTab {
    pub fn drop(&mut self, paths: Vec<PathBuf>) {
        self.files_mode = true;
        for p in paths.into_iter().filter(|p| p.is_file()) {
            if !self.files.contains(&p) {
                self.files.push(p);
            }
        }
    }

    fn start(&mut self) {
        let files = self.files.clone();
        // NTLM hashes passwords, not files
        let algos: Vec<Algo> = self.on.iter().copied().filter(|a| !a.text_only()).collect();
        let key = self.hmac_key.clone();
        self.results.clear();
        self.job = Some(Job::start(move |p| {
            p.grow(files.iter().map(|f| f.metadata().map(|m| m.len()).unwrap_or(0)).sum());
            let mut out = vec![];
            for f in files {
                let size = f.metadata().map(|m| m.len()).unwrap_or(0);
                let digests = std::fs::File::open(&f).map_err(|e| e.to_string()).and_then(|file| digest(&algos, &key, file, Some(p)));
                if digests.as_ref().is_err_and(|e| e == encsha::util::CANCELLED) {
                    return Err(encsha::util::CANCELLED.into());
                }
                out.push(FileResult { path: f, size, digests });
            }
            Ok(out)
        }));
    }

    pub fn show(&mut self, ui: &mut Ui, s: &mut Shared) {
        let de = s.de;
        if let Some(r) = finished(&mut self.job) {
            match r {
                Ok(res) => self.results = res,
                Err(e) => s.toast(if e == encsha::util::CANCELLED { t(de, "Abgebrochen", "Cancelled").to_string() } else { e }, false),
            }
        }

        panel(ui, |ui| {
            panel_head(
                ui,
                t(de, "Hash berechnen", "Calculate hashes"),
                t(de, "Fingerabdruck von Text oder Dateien – z. B. um einen Download zu prüfen", "Fingerprint of text or files – e.g. to check a download"),
            );
            ui.add_space(4.0);
            if let Some(i) = segmented(ui, &["Text", t(de, "Dateien", "Files")], self.files_mode as usize, true) {
                self.files_mode = i == 1;
            }
            ui.add_space(2.0);
            if self.files_mode {
                self.files_ui(ui, de);
            } else {
                text_area(ui, egui::Id::new("hash-text"), &mut self.text, t(de, "Text hier eingeben – die Hashes erscheinen sofort", "Type text here – the hashes show up right away"), 4, true);
            }
            ui.add_space(4.0);
            ui.columns(2, |c| {
                let w = c[0].available_width();
                label(&mut c[0], t(de, "HMAC-Schlüssel (optional)", "HMAC key (optional)"));
                text_input(&mut c[0], egui::Id::new("hash-hmac"), &mut self.hmac_key, t(de, "leer = normaler Hash", "empty = plain hash"), w, false);
                label(&mut c[1], t(de, "Vergleichen mit", "Compare with"));
                text_input(&mut c[1], egui::Id::new("hash-compare"), &mut self.compare, t(de, "erwarteten Hash einfügen (hex oder Base64)", "paste the expected hash (hex or Base64)"), w, false);
            });
        });
        ui.add_space(12.0);

        panel(ui, |ui| self.algos_ui(ui, de));
        ui.add_space(12.0);

        panel(ui, |ui| self.results_ui(ui, s));
    }

    fn files_ui(&mut self, ui: &mut Ui, de: bool) {
        let running = self.job.is_some();
        if self.files.is_empty() {
            hint(ui, t(de, "Noch keine Dateien – „Hinzufügen“ oder Dateien auf das Fenster ziehen.", "No files yet – \"Add\" or drop files on the window."));
        } else {
            let mut remove = None;
            list(ui, |ui| {
                for (i, f) in self.files.iter().enumerate() {
                    if i > 0 {
                        row_line(ui);
                    }
                    ui.horizontal(|ui| {
                        ui.set_min_height(28.0);
                        let name = f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                        ui.label(RichText::new(name).size(13.0));
                        ui.label(RichText::new(fmt_bytes(f.metadata().map(|m| m.len()).unwrap_or(0))).size(11.5).color(TEXT_2));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if icon_btn(ui, "✕", !running).clicked() {
                                remove = Some(i);
                            }
                        });
                    });
                }
            });
            if let Some(i) = remove {
                self.files.remove(i);
            }
        }
        ui.horizontal(|ui| {
            if btn(ui, t(de, "+ Hinzufügen …", "+ Add …"), !running).clicked() {
                if let Some(ps) = rfd::FileDialog::new().pick_files() {
                    self.drop(ps);
                }
            }
            if btn(ui, t(de, "Leeren", "Clear"), !running && !self.files.is_empty()).clicked() {
                self.files.clear();
                self.results.clear();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let ok = !running && !self.files.is_empty() && !self.on.is_empty();
                if btn_primary(ui, t(de, "Berechnen", "Calculate"), 140.0, ok).clicked() {
                    self.start();
                }
            });
        });
        if let Some(j) = &self.job {
            job_view(ui, de, j, t(de, "Hashen", "Hashing"));
        }
    }

    fn algos_ui(&mut self, ui: &mut Ui, de: bool) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(t(de, "Algorithmen", "Algorithms")).size(15.0).family(semibold()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let fmts = [Fmt::Hex, Fmt::HexUpper, Fmt::Base64];
                let sel = fmts.iter().position(|f| *f == self.fmt).unwrap_or(0);
                if let Some(i) = segmented_fit(ui, &["hex", "HEX", "Base64"], sel, 12.0, 22.0) {
                    self.fmt = fmts[i];
                }
                ui.add_space(8.0);
                if btn_small(ui, t(de, "Keine", "None")).clicked() {
                    self.on.clear();
                }
                if btn_small(ui, t(de, "Alle", "All")).clicked() {
                    self.on = Algo::ALL.to_vec();
                }
                if btn_small(ui, t(de, "Übliche", "Common")).clicked() {
                    self.on = Algo::COMMON.to_vec();
                }
            });
        });
        for (class, title) in [
            (Class::Secure, t(de, "Sicher", "Secure")),
            (Class::Broken, t(de, "Gebrochen – nur noch als Prüfsumme", "Broken – only good as a checksum")),
            (Class::Checksum, t(de, "Prüfsummen – erkennen nur Fehler, keine Absicht", "Checksums – catch errors, not attacks")),
        ] {
            label(ui, title);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                ui.spacing_mut().button_padding = egui::vec2(10.0, 3.0);
                for a in Algo::ALL.into_iter().filter(|a| a.class() == class) {
                    let mut on = self.on.contains(&a);
                    let r = pick(ui, &mut on, a.name());
                    if a.text_only() {
                        r.on_hover_text(t(de, "Windows-Passwort-Hash (MD4 über UTF-16) – nur für Text", "Windows password hash (MD4 of UTF-16) – text only"));
                    }
                    if on != self.on.contains(&a) {
                        if on {
                            self.on.push(a);
                        } else {
                            self.on.retain(|x| *x != a);
                        }
                        // keep the list order
                        self.on.sort_by_key(|x| Algo::ALL.iter().position(|y| y == x));
                    }
                }
            });
        }
    }

    fn results_ui(&mut self, ui: &mut Ui, s: &mut Shared) {
        let de = s.de;
        ui.label(RichText::new(t(de, "Ergebnis", "Result")).size(15.0).family(semibold()));
        if self.on.is_empty() {
            hint(ui, t(de, "Mindestens einen Algorithmus wählen.", "Pick at least one algorithm."));
            return;
        }
        if !self.hmac_key.is_empty() {
            chip_blue(ui, t(de, "HMAC mit Schlüssel", "HMAC with key"));
        }
        let mut matched = vec![];
        if !self.files_mode {
            if self.text.is_empty() {
                hint(ui, t(de, "Oben Text eingeben.", "Type some text above."));
                return;
            }
            let want = (self.text.clone(), self.on.clone(), self.hmac_key.clone());
            if self.text_for.as_ref() != Some(&want) {
                self.text_out = digest(&self.on, &self.hmac_key, self.text.as_bytes(), None).unwrap_or_default();
                self.text_for = Some(want);
            }
            let out = std::mem::take(&mut self.text_out);
            self.rows(ui, s, &out, None, &mut matched);
            self.text_out = out;
        } else if self.results.is_empty() {
            hint(ui, t(de, "Dateien wählen und „Berechnen“ drücken.", "Pick files and press \"Calculate\"."));
            return;
        } else {
            let results = std::mem::take(&mut self.results);
            for (i, r) in results.iter().enumerate() {
                if i > 0 {
                    ui.add_space(6.0);
                }
                ui.horizontal(|ui| {
                    let name = r.path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    ui.label(RichText::new(&name).size(13.5).family(semibold()));
                    ui.label(RichText::new(fmt_bytes(r.size)).size(11.5).color(TEXT_2));
                    if icon_btn(ui, "📂", true).on_hover_text(t(de, "Im Ordner zeigen", "Show in folder")).clicked() {
                        show_in_folder(&r.path);
                    }
                });
                match &r.digests {
                    Ok(d) => self.rows(ui, s, d, Some(&name_of(&r.path)), &mut matched),
                    Err(e) => error_line(ui, de, e),
                }
            }
            self.results = results;
            ui.add_space(6.0);
            self.save_sums_ui(ui, s);
        }
        if !self.compare.trim().is_empty() {
            ui.add_space(4.0);
            if matched.is_empty() {
                chip_red(ui, t(de, "✗ Keine Übereinstimmung mit dem erwarteten Hash", "✗ Nothing matches the expected hash"));
            } else {
                chip_on(ui, &format!("✓ {} {}", t(de, "Stimmt überein:", "Matches:"), matched.join(", ")));
            }
        }
    }

    fn rows(&self, ui: &mut Ui, s: &mut Shared, digests: &Digests, file: Option<&str>, matched: &mut Vec<String>) {
        let de = s.de;
        list(ui, |ui| {
            for (i, (a, d)) in digests.iter().enumerate() {
                if i > 0 {
                    row_line(ui);
                }
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(egui::vec2(104.0, 20.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.set_min_width(104.0);
                        ui.label(RichText::new(a.name()).size(12.5).family(semibold()));
                    });
                    match d {
                        Some(d) => {
                            let shown = hash::show(d, self.fmt);
                            copy_btn(ui, s, &shown);
                            if hash::matches(d, &self.compare) {
                                chip_on(ui, "✓");
                                matched.push(file.map(|f| format!("{} ({f})", a.name())).unwrap_or_else(|| a.name().to_string()));
                            }
                            ui.add(egui::Label::new(RichText::new(shown).monospace().size(12.5)).wrap().selectable(true));
                        }
                        None => {
                            let why = if a.text_only() {
                                t(de, "nur für Text", "text only")
                            } else {
                                t(de, "kein HMAC möglich", "no HMAC for this one")
                            };
                            ui.label(RichText::new(why).size(12.0).color(TEXT_3));
                        }
                    }
                });
                ui.add_space(5.0);
            }
        });
    }

    fn save_sums_ui(&mut self, ui: &mut Ui, s: &mut Shared) {
        let de = s.de;
        let algos: Vec<Algo> = self.on.iter().copied().filter(|a| !a.text_only()).collect();
        let one = algos.len() == 1 && self.hmac_key.is_empty();
        let r = btn(ui, t(de, "Prüfsummendatei speichern …", "Save checksum file …"), one);
        let r = if one {
            r
        } else {
            r.on_disabled_hover_text(t(de, "Geht mit genau einem Algorithmus (ohne HMAC)", "Works with exactly one algorithm (no HMAC)"))
        };
        if !r.clicked() {
            return;
        }
        let a = algos[0];
        let ext = a.name().to_ascii_lowercase().replace(['-', '/'], "");
        let dir = self.results[0].path.parent().map(|p| p.to_path_buf());
        let mut d = rfd::FileDialog::new().set_file_name(format!("{}SUMS.{ext}", a.name().replace(['-', '/'], ""))).add_filter(a.name(), &[&ext, "txt"]);
        if let Some(dir) = dir {
            d = d.set_directory(dir);
        }
        let Some(out) = d.save_file() else { return };
        let base = out.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        let text: String = self
            .results
            .iter()
            .filter_map(|r| {
                let d = r.digests.as_ref().ok()?.iter().find(|(x, _)| *x == a)?.1.clone()?;
                // relative to the checksum file when possible
                let name = r.path.strip_prefix(&base).map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_else(|_| r.path.display().to_string());
                Some(format!("{}\n", encsha::sums::format_line(&d, &name)))
            })
            .collect();
        match std::fs::write(&out, text) {
            Ok(_) => s.toast(format!("{} {}", t(de, "Gespeichert:", "Saved:"), out.display()), true),
            Err(e) => s.toast(e.to_string(), false),
        }
    }
}

fn name_of(p: &std::path::Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
}
