// Passwords: generator, strength check, hash for storing (Argon2, bcrypt ...) and verify a stored hash.

use super::theme::*;
use super::{copy_btn, error_line, finished, job_view, password_row, t, Job, Shared};
use eframe::egui::{self, RichText, Ui};
use encsha::pw::{self, GenOpts, PwAlgo};
use encsha::util::Res;
use zeroize::Zeroizing;

pub struct PwTab {
    opts: GenOpts,
    phrase: bool,
    words: usize,
    generated: String,
    check: String,
    show_check: bool,
    algo: PwAlgo,
    hash_pw: String,
    show_hash_pw: bool,
    hash_job: Option<Job<String>>,
    hash_out: Option<Res<String>>,
    stored: String,
    verify_pw: String,
    show_verify_pw: bool,
    verify_job: Option<Job<bool>>,
    verify_out: Option<Res<bool>>,
}

impl Default for PwTab {
    fn default() -> PwTab {
        let opts = GenOpts::default();
        PwTab {
            generated: pw::generate(&opts).unwrap_or_default(),
            opts,
            phrase: false,
            words: 5,
            check: String::new(),
            show_check: false,
            algo: PwAlgo::Argon2id,
            hash_pw: String::new(),
            show_hash_pw: false,
            hash_job: None,
            hash_out: None,
            stored: String::new(),
            verify_pw: String::new(),
            show_verify_pw: false,
            verify_job: None,
            verify_out: None,
        }
    }
}

fn level_name(de: bool, level: u8) -> &'static str {
    match level {
        0 => t(de, "sehr schwach", "very weak"),
        1 => t(de, "schwach", "weak"),
        2 => t(de, "okay", "okay"),
        3 => t(de, "stark", "strong"),
        _ => t(de, "sehr stark", "very strong"),
    }
}

fn settings(de: bool, a: PwAlgo) -> &'static str {
    if !de {
        return a.settings();
    }
    match a {
        PwAlgo::Argon2id | PwAlgo::Argon2i | PwAlgo::Argon2d => "19 MiB, 2 Durchläufe (OWASP)",
        PwAlgo::Bcrypt => "Kosten 12, max. 72 Bytes",
        PwAlgo::Scrypt => "N=2^17, r=8, p=1",
        PwAlgo::Pbkdf2Sha256 => "600.000 Runden (OWASP)",
        PwAlgo::Pbkdf2Sha512 => "210.000 Runden (OWASP)",
        PwAlgo::Sha512Crypt | PwAlgo::Sha256Crypt => "Linux /etc/shadow ($6$ / $5$)",
        PwAlgo::Md5Crypt => "veraltet ($1$), schnell zu knacken",
    }
}

impl PwTab {
    fn regenerate(&mut self) {
        self.generated = if self.phrase { pw::passphrase(self.words) } else { pw::generate(&self.opts).unwrap_or_default() };
    }

    pub fn show(&mut self, ui: &mut Ui, s: &mut Shared) {
        let de = s.de;
        if let Some(r) = finished(&mut self.hash_job) {
            self.hash_out = Some(r);
        }
        if let Some(r) = finished(&mut self.verify_job) {
            self.verify_out = Some(r);
        }

        panel(ui, |ui| {
            panel_head(ui, t(de, "Passwort-Generator", "Password generator"), t(de, "Zufällig aus dem sicheren Zufallsgenerator von Windows", "Random from the Windows secure random generator"));
            ui.add_space(4.0);
            let mut changed = false;
            if let Some(i) = segmented(ui, &[t(de, "Zeichen", "Characters"), t(de, "Silben (aussprechbar)", "Syllables (pronounceable)")], self.phrase as usize, true) {
                self.phrase = i == 1;
                changed = true;
            }
            if self.phrase {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t(de, "Teile", "Parts")).size(12.0).color(TEXT_2));
                    changed |= ui.add(egui::Slider::new(&mut self.words, 2..=12)).changed();
                });
            } else {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t(de, "Länge", "Length")).size(12.0).color(TEXT_2));
                    changed |= ui.add(egui::Slider::new(&mut self.opts.len, 6..=128)).changed();
                });
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    ui.spacing_mut().button_padding = egui::vec2(10.0, 3.0);
                    for (on, name) in [
                        (&mut self.opts.lower, "a-z"),
                        (&mut self.opts.upper, "A-Z"),
                        (&mut self.opts.digits, "0-9"),
                        (&mut self.opts.symbols, "!#$%…"),
                        (&mut self.opts.no_ambiguous, t(de, "ohne I l 1 O 0", "no I l 1 O 0")),
                    ] {
                        changed |= pick(ui, on, name).clicked();
                    }
                });
            }
            if changed {
                self.regenerate();
            }
            ui.add_space(4.0);
            egui::Frame::none().fill(SURFACE_2).stroke(egui::Stroke::new(1.0_f32, LINE)).rounding(R_MD).inner_margin(egui::Margin::symmetric(14.0, 12.0)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.add(egui::Label::new(RichText::new(&self.generated).monospace().size(17.0)).wrap().selectable(true));
            });
            ui.horizontal(|ui| {
                if btn(ui, t(de, "↻ Neu", "↻ New"), true).clicked() {
                    self.regenerate();
                }
                copy_btn(ui, s, &self.generated);
                let bits = if self.phrase { pw::passphrase_bits(self.words) } else { pw::gen_bits(&self.opts) };
                let lvl = pw::strength(&self.generated).level.max(if bits >= 90.0 { 4 } else if bits >= 60.0 { 3 } else { 2 });
                meter(ui, lvl, 120.0);
                hint(ui, &format!("{bits:.0} bit {}", t(de, "echter Zufall", "of real randomness")));
            });
        });
        ui.add_space(12.0);

        panel(ui, |ui| {
            panel_head(ui, t(de, "Wie stark ist mein Passwort?", "How strong is my password?"), t(de, "Wird nur hier im Programm geprüft, nichts verlässt den PC", "Checked only in this program, nothing leaves the PC"));
            ui.add_space(4.0);
            password_row(ui, "pw-check", &mut self.check, &mut self.show_check, t(de, "Passwort eingeben", "Type a password"));
            if !self.check.is_empty() {
                let st = pw::strength(&self.check);
                ui.horizontal(|ui| {
                    meter(ui, st.level, 160.0);
                    ui.label(RichText::new(level_name(de, st.level)).size(12.5).family(semibold()));
                    ui.label(
                        RichText::new(format!(
                            "~{:.0} bit · {} {}",
                            st.bits,
                            t(de, "Knacken bei 10 Mrd. Versuchen/s (eine Grafikkarte, schneller Hash):", "cracking at 10 billion guesses/s (one GPU, fast hash):"),
                            pw::duration(st.crack_secs, de)
                        ))
                        .size(11.5)
                        .color(TEXT_2),
                    );
                });
            }
        });
        ui.add_space(12.0);

        panel(ui, |ui| {
            panel_head(
                ui,
                t(de, "Passwort-Hash zum Speichern", "Password hash for storing"),
                t(de, "So speichern Webseiten und Linux Passwörter – langsam mit Absicht, mit Salz", "How websites and Linux store passwords – slow on purpose, salted"),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("pw-algo").width(170.0).selected_text(self.algo.name()).show_ui(ui, |ui| {
                    for a in PwAlgo::ALL {
                        ui.selectable_value(&mut self.algo, a, a.name());
                    }
                });
                hint(ui, settings(de, self.algo));
            });
            password_row(ui, "pw-hash", &mut self.hash_pw, &mut self.show_hash_pw, t(de, "Passwort", "Password"));
            let running = self.hash_job.is_some();
            if btn_primary(ui, t(de, "Hash erzeugen", "Make hash"), 150.0, !running && !self.hash_pw.is_empty()).clicked() {
                let (a, p) = (self.algo, Zeroizing::new(self.hash_pw.clone()));
                self.hash_out = None;
                self.hash_job = Some(Job::start(move |_| pw::hash(a, &p)));
            }
            if let Some(j) = &self.hash_job {
                job_view(ui, de, j, t(de, "Rechne (absichtlich langsam)", "Working (slow on purpose)"));
            }
            match &self.hash_out {
                Some(Ok(h)) => {
                    let h = h.clone();
                    ui.horizontal(|ui| {
                        copy_btn(ui, s, &h);
                        if btn_small(ui, t(de, "↓ zum Prüfen", "↓ to verify")).clicked() {
                            self.stored = h.clone();
                        }
                    });
                    ui.add(egui::Label::new(RichText::new(&h).monospace()).wrap().selectable(true));
                }
                Some(Err(e)) => error_line(ui, de, e),
                None => {}
            }
            sep(ui);
            label(ui, t(de, "Gespeicherten Hash prüfen", "Check a stored hash"));
            text_input(ui, egui::Id::new("pw-stored"), &mut self.stored, "$argon2id$… / $2b$… / $6$… / $pbkdf2-sha256$…", ui.available_width(), false);
            if !self.stored.trim().is_empty() {
                match pw::detect(&self.stored) {
                    Some(a) => chip_blue(ui, a.name()),
                    None => chip_red(ui, t(de, "unbekanntes Format", "unknown format")),
                }
            }
            password_row(ui, "pw-verify", &mut self.verify_pw, &mut self.show_verify_pw, t(de, "Passwort", "Password"));
            let running = self.verify_job.is_some();
            if btn_primary(ui, t(de, "Prüfen", "Verify"), 150.0, !running && pw::detect(&self.stored).is_some()).clicked() {
                let (h, p) = (self.stored.clone(), Zeroizing::new(self.verify_pw.clone()));
                self.verify_out = None;
                self.verify_job = Some(Job::start(move |_| pw::verify(&h, &p)));
            }
            if let Some(j) = &self.verify_job {
                job_view(ui, de, j, t(de, "Rechne", "Working"));
            }
            match &self.verify_out {
                Some(Ok(true)) => chip_on(ui, t(de, "✓ Passwort stimmt", "✓ Correct password")),
                Some(Ok(false)) => chip_red(ui, t(de, "✗ Falsches Passwort", "✗ Wrong password")),
                Some(Err(e)) => error_line(ui, de, e),
                None => {}
            }
        });
    }
}
