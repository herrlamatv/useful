// Key pairs (Ed25519, X25519, RSA, P-256): generate + save, and sign / verify text or files.

use super::theme::*;
use super::{copy_btn, error_line, file_row, finished, job_view, password_row, show_in_folder, t, Job, KeyInfo, Shared};
use eframe::egui::{self, Ui};
use encsha::keys::{self, KeyKind, PrivKey};
use encsha::util::{b64, bytes_from_text, Res};
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

const KINDS: [KeyKind; 4] = [KeyKind::Ed25519, KeyKind::X25519, KeyKind::Rsa(4096), KeyKind::P256];

pub struct Generated {
    key: PrivKey,
    private_pem: Zeroizing<String>,
    public_pem: String,
    fingerprint: String,
}

enum SvDone {
    Signed(String),
    Verified(bool, String),
}

pub struct KeysTab {
    kind: usize,
    bits: usize,
    name: String,
    lock_pw: String,
    show_lock_pw: bool,
    gen_job: Option<Job<Generated>>,
    generated: Option<Res<Generated>>,
    show_private: bool,
    saved: Option<PathBuf>,
    // sign / verify
    file_mode: bool,
    text: String,
    file: String,
    sign_key: String,
    sign_key_pw: String,
    show_sign_key_pw: bool,
    verify_key: String,
    sig: String,
    sv_job: Option<Job<SvDone>>,
    sv_done: Option<Res<SvDone>>,
    sign_info: KeyInfo,
    verify_info: KeyInfo,
}

impl Default for KeysTab {
    fn default() -> KeysTab {
        KeysTab {
            kind: 0,
            bits: 4096,
            name: String::new(),
            lock_pw: String::new(),
            show_lock_pw: false,
            gen_job: None,
            generated: None,
            show_private: false,
            saved: None,
            file_mode: false,
            text: String::new(),
            file: String::new(),
            sign_key: String::new(),
            sign_key_pw: String::new(),
            show_sign_key_pw: false,
            verify_key: String::new(),
            sig: String::new(),
            sv_job: None,
            sv_done: None,
            sign_info: KeyInfo::default(),
            verify_info: KeyInfo::default(),
        }
    }
}

impl KeysTab {
    pub fn drop(&mut self, p: &Path) -> bool {
        let name = p.to_string_lossy().to_string();
        if name.ends_with(".sig") {
            self.sig = std::fs::read_to_string(p).unwrap_or_default().trim().to_string();
            return true;
        }
        if name.ends_with(".pem") {
            if std::fs::read_to_string(p).is_ok_and(|t| t.contains("PRIVATE KEY")) {
                self.sign_key = name;
            } else {
                self.verify_key = name;
            }
            return true;
        }
        if p.is_file() {
            self.file_mode = true;
            self.file = name;
            self.load_sig_next_to_file();
            return true;
        }
        false
    }

    fn load_sig_next_to_file(&mut self) {
        let sig = format!("{}.sig", self.file);
        if let Ok(s) = std::fs::read_to_string(&sig) {
            self.sig = s.trim().to_string();
        }
        self.sv_done = None;
    }

    fn kind(&self) -> KeyKind {
        match KINDS[self.kind] {
            KeyKind::Rsa(_) => KeyKind::Rsa(self.bits),
            k => k,
        }
    }

    pub fn show(&mut self, ui: &mut Ui, s: &mut Shared) {
        if let Some(r) = finished(&mut self.gen_job) {
            self.generated = Some(r);
            self.saved = None;
            self.show_private = false;
        }
        if let Some(r) = finished(&mut self.sv_job) {
            if let Ok(SvDone::Signed(sig)) = &r {
                self.sig = sig.clone();
            }
            self.sv_done = Some(r);
        }
        panel(ui, |ui| self.generate_ui(ui, s));
        ui.add_space(12.0);
        panel(ui, |ui| self.sign_ui(ui, s));
    }

    fn generate_ui(&mut self, ui: &mut Ui, s: &mut Shared) {
        let de = s.de;
        panel_head(
            ui,
            t(de, "Neues Schlüsselpaar", "New key pair"),
            t(de, "Den privaten Schlüssel behältst du, den öffentlichen gibst du weiter", "You keep the private key, the public one you hand out"),
        );
        ui.add_space(4.0);
        let running = self.gen_job.is_some();
        if let Some(i) = segmented(ui, &["Ed25519", "X25519", "RSA", "ECDSA P-256"], self.kind, !running) {
            self.kind = i;
        }
        hint(
            ui,
            match self.kind {
                0 => t(de, "Signieren – modern, schnell, kleine Schlüssel (empfohlen zum Signieren)", "Signing – modern, fast, small keys (recommended for signing)"),
                1 => t(de, "Verschlüsseln – andere verschlüsseln für dich ohne gemeinsames Passwort (empfohlen)", "Encryption – others encrypt for you without a shared password (recommended)"),
                2 => t(de, "Signieren und verschlüsseln – der Klassiker, überall unterstützt, große Schlüssel", "Signing and encryption – the classic, supported everywhere, big keys"),
                _ => t(de, "Signieren – der NIST-Standard aus TLS, JWT (ES256) und WebCrypto", "Signing – the NIST standard used by TLS, JWT (ES256) and WebCrypto"),
            },
        );
        ui.horizontal(|ui| {
            if self.kind == 2 {
                egui::ComboBox::from_id_salt("rsa-bits").width(110.0).selected_text(format!("{} bit", self.bits)).show_ui(ui, |ui| {
                    for b in [2048, 3072, 4096] {
                        ui.selectable_value(&mut self.bits, b, format!("{b} bit"));
                    }
                });
            }
            let w = (ui.available_width() - 180.0).max(160.0) / 2.0;
            text_input(ui, egui::Id::new("key-name"), &mut self.name, t(de, "Name (z. B. dein Name)", "Name (e.g. your name)"), w, false);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if btn_primary(ui, t(de, "Erzeugen", "Generate"), 140.0, !running).clicked() {
                    let kind = self.kind();
                    let pw = Zeroizing::new(self.lock_pw.clone());
                    self.gen_job = Some(Job::start(move |_| {
                        let key = keys::generate(kind)?;
                        let private_pem = key.to_pem(Some(&pw))?;
                        let public = key.public();
                        Ok(Generated { public_pem: public.to_pem()?, fingerprint: public.fingerprint(), private_pem, key })
                    }));
                }
            });
        });
        password_row(ui, "key-lockpw", &mut self.lock_pw, &mut self.show_lock_pw, t(de, "Passwort für den privaten Schlüssel (optional, empfohlen)", "Password for the private key (optional, recommended)"));
        if let Some(j) = &self.gen_job {
            job_view(ui, de, j, t(de, "Erzeuge Schlüssel (RSA dauert ein paar Sekunden)", "Making the key (RSA takes a few seconds)"));
        }
        match &self.generated {
            Some(Ok(g)) => {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    chip_on(ui, &format!("✓ {}", g.key.kind_name()));
                    chip_gray(ui, &format!("{} {}", t(de, "Fingerabdruck", "Fingerprint"), g.fingerprint));
                });
                ui.horizontal(|ui| {
                    label(ui, t(de, "Öffentlicher Schlüssel – zum Weitergeben", "Public key – share it"));
                    copy_btn(ui, s, &g.public_pem);
                });
                let mut p = g.public_pem.clone();
                text_area(ui, egui::Id::new("key-pub"), &mut p, "", 4, false);
                ui.horizontal(|ui| {
                    label(ui, t(de, "Privater Schlüssel – geheim halten", "Private key – keep it secret"));
                    if btn_small(ui, if self.show_private { t(de, "Verbergen", "Hide") } else { t(de, "Anzeigen", "Show") }).clicked() {
                        self.show_private = !self.show_private;
                    }
                    copy_btn(ui, s, &g.private_pem);
                });
                if self.show_private {
                    let mut p = g.private_pem.to_string();
                    text_area(ui, egui::Id::new("key-priv"), &mut p, "", 6, false);
                }
                ui.horizontal(|ui| {
                    if btn(ui, t(de, "In Ordner speichern …", "Save to folder …"), true).clicked() {
                        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                            let name = if self.name.trim().is_empty() { g.key.kind_name().to_ascii_lowercase().replace(' ', "-") } else { self.name.trim().to_string() };
                            match keys::write_pair(&dir, &name, &g.key, Some(&self.lock_pw)) {
                                Ok((private, _)) => {
                                    s.toast(format!("{} {}", t(de, "Gespeichert:", "Saved:"), dir.display()), true);
                                    self.saved = Some(private);
                                }
                                Err(e) => s.toast(e, false),
                            }
                        }
                    }
                    if let Some(p) = &self.saved {
                        let file = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                        chip_on(ui, &format!("✓ {file} + {}", file.replace(".key.pem", ".pub.pem")));
                        if btn_small(ui, t(de, "Im Ordner zeigen", "Show in folder")).clicked() {
                            show_in_folder(p);
                        }
                    }
                });
                if self.lock_pw.is_empty() {
                    note(ui, t(de, "Der private Schlüssel wird ohne Passwort gespeichert (Standard-PEM, auch für OpenSSL lesbar). Mit Passwort ist er geschützt, aber nur encsha kann ihn öffnen.", "The private key is saved without a password (standard PEM, OpenSSL reads it too). With a password it is protected, but only encsha can open it."));
                }
            }
            Some(Err(e)) => error_line(ui, de, e),
            None => {}
        }
    }

    fn sign_ui(&mut self, ui: &mut Ui, s: &mut Shared) {
        let de = s.de;
        panel_head(
            ui,
            t(de, "Signieren & prüfen", "Sign & verify"),
            t(de, "Beweist, von wem etwas stammt und dass es nicht verändert wurde", "Proves who something comes from and that it wasn't changed"),
        );
        ui.add_space(4.0);
        let running = self.sv_job.is_some();
        if let Some(i) = segmented(ui, &["Text", t(de, "Datei", "File")], self.file_mode as usize, !running) {
            self.file_mode = i == 1;
            self.sv_done = None;
        }
        if self.file_mode {
            if file_row(ui, "sv-file", &mut self.file, t(de, "Datei wählen oder auf das Fenster ziehen", "Pick a file or drop it on the window"), false) {
                self.load_sig_next_to_file();
            }
        } else if text_area(ui, egui::Id::new("sv-text"), &mut self.text, t(de, "Nachricht", "Message"), 3, true).changed() {
            self.sv_done = None;
        }
        ui.add_space(4.0);
        let has_input = if self.file_mode { Path::new(&self.file).is_file() } else { !self.text.is_empty() };
        let top = egui::Layout::top_down(egui::Align::Min);
        ui.columns(2, |c| {
            // columns are justified: a top-down layout keeps the buttons at their own width
            c[0].with_layout(top, |ui| {
                label(ui, t(de, "Signieren mit privatem Schlüssel", "Sign with private key"));
                file_row(ui, "sv-priv", &mut self.sign_key, "….key.pem", false);
                self.sign_info.update(&self.sign_key, de).chip(ui);
                if self.sign_info.locked() {
                    password_row(ui, "sv-privpw", &mut self.sign_key_pw, &mut self.show_sign_key_pw, t(de, "Passwort des Schlüssels", "Key password"));
                }
                if btn_primary(ui, t(de, "✍ Signieren", "✍ Sign"), 150.0, !running && has_input && !self.sign_key.is_empty()).clicked() {
                    let (key, pw, file_mode, file, text) = (self.sign_key.clone(), Zeroizing::new(self.sign_key_pw.clone()), self.file_mode, PathBuf::from(&self.file), self.text.clone());
                    self.sv_job = Some(Job::start(move |p| {
                        let k = keys::parse_private(&keys::read_key_file(Path::new(&key))?, Some(&pw))?;
                        let sig = if file_mode {
                            let f = std::fs::File::open(&file).map_err(|e| e.to_string())?;
                            p.grow(f.metadata().map(|m| m.len()).unwrap_or(0));
                            keys::sign(&k, f, Some(p))?
                        } else {
                            keys::sign(&k, text.as_bytes(), None)?
                        };
                        Ok(SvDone::Signed(b64(&sig)))
                    }));
                }
            });
            c[1].with_layout(top, |ui| {
                label(ui, t(de, "Prüfen mit öffentlichem Schlüssel", "Verify with public key"));
                file_row(ui, "sv-pub", &mut self.verify_key, "….pub.pem", false);
                self.verify_info.update(&self.verify_key, de).chip(ui);
                if btn_primary(ui, t(de, "🔍 Prüfen", "🔍 Verify"), 150.0, !running && has_input && !self.verify_key.is_empty() && !self.sig.trim().is_empty()).clicked() {
                    let (key, sig, file_mode, file, text) = (self.verify_key.clone(), self.sig.clone(), self.file_mode, PathBuf::from(&self.file), self.text.clone());
                    self.sv_job = Some(Job::start(move |p| {
                        let k = keys::parse_public(&keys::read_key_file(Path::new(&key))?)?;
                        let sig = bytes_from_text(sig.trim())?;
                        let ok = if file_mode {
                            let f = std::fs::File::open(&file).map_err(|e| e.to_string())?;
                            p.grow(f.metadata().map(|m| m.len()).unwrap_or(0));
                            keys::verify(&k, f, &sig, Some(p))?
                        } else {
                            keys::verify(&k, text.as_bytes(), &sig, None)?
                        };
                        Ok(SvDone::Verified(ok, format!("{} · {}", k.kind().sig_name(), k.fingerprint())))
                    }));
                }
            });
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            label(ui, t(de, "Signatur (Base64)", "Signature (Base64)"));
            if !self.sig.is_empty() {
                copy_btn(ui, s, &self.sig);
                if self.file_mode && Path::new(&self.file).is_file() && btn_small(ui, t(de, "Als .sig speichern", "Save as .sig")).clicked() {
                    let out = encsha::crypt::unused(Path::new(&format!("{}.sig", self.file)));
                    match std::fs::write(&out, format!("{}\n", self.sig.trim())) {
                        Ok(_) => s.toast(format!("{} {}", t(de, "Gespeichert:", "Saved:"), out.display()), true),
                        Err(e) => s.toast(e.to_string(), false),
                    }
                }
            }
        });
        if text_area(ui, egui::Id::new("sv-sig"), &mut self.sig, t(de, "Signatur einfügen – oder „Signieren“ drücken (eine .sig neben der Datei wird automatisch geladen)", "Paste a signature – or press \"Sign\" (a .sig next to the file is loaded automatically)"), 2, true).changed() {
            self.sv_done = None;
        }
        if let Some(j) = &self.sv_job {
            job_view(ui, de, j, t(de, "Lese die Daten", "Reading the data"));
        }
        match &self.sv_done {
            Some(Ok(SvDone::Signed(_))) => chip_on(ui, t(de, "✓ Signiert – Signatur steht oben", "✓ Signed – the signature is above")),
            Some(Ok(SvDone::Verified(true, how))) => chip_on(ui, &format!("✓ {} · {how}", t(de, "Signatur gültig", "Valid signature"))),
            Some(Ok(SvDone::Verified(false, _))) => {
                chip_red(ui, t(de, "✗ Signatur UNGÜLTIG – Daten oder Signatur verändert, oder anderer Schlüssel", "✗ INVALID signature – data or signature changed, or a different key"))
            }
            Some(Err(e)) => error_line(ui, de, e),
            None => {}
        }
    }
}
