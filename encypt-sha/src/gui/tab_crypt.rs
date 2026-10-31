// Encrypt / decrypt text or files with a password or a key pair. Decrypting needs no settings:
// cipher, Argon2 cost and lock type are read from the data.

use super::theme::*;
use super::{copy_btn, error_line, file_row, finished, job_view, password_row, show_in_folder, t, Job, KeyInfo, Shared};
use eframe::egui::{self, RichText, Ui};
use encsha::crypt::{self, Cipher, Info, Kdf, Lock, LockKind, Unlock};
use encsha::keys;
use encsha::pw;
use encsha::util::Res;
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

enum Done {
    Text(String, Option<Info>),
    File(PathBuf, Option<Info>),
}

pub struct CryptTab {
    file_mode: bool,
    text: String,
    out_text: String,
    src: String,
    dst: String,
    /// what the chosen file / pasted text was encrypted with (Err = not encrypted by encsha)
    info: Option<Res<Info>>,
    key_mode: bool,
    pw: String,
    pw2: String,
    show_pw: bool,
    pub_key: String,
    priv_key: String,
    key_pw: String,
    show_key_pw: bool,
    cipher: Cipher,
    job: Option<Job<Done>>,
    done: Option<Res<Done>>,
    pub_info: KeyInfo,
    priv_info: KeyInfo,
}

impl Default for CryptTab {
    fn default() -> CryptTab {
        CryptTab {
            file_mode: false,
            text: String::new(),
            out_text: String::new(),
            src: String::new(),
            dst: String::new(),
            info: None,
            key_mode: false,
            pw: String::new(),
            pw2: String::new(),
            show_pw: false,
            pub_key: String::new(),
            priv_key: String::new(),
            key_pw: String::new(),
            show_key_pw: false,
            cipher: Cipher::Aes256Gcm,
            job: None,
            done: None,
            pub_info: KeyInfo::default(),
            priv_info: KeyInfo::default(),
        }
    }
}

/// the private key from a file, unlocked with the key password if it has one
fn load_private(path: &str, key_pw: &str) -> Res<keys::PrivKey> {
    let text = keys::read_key_file(Path::new(path))?;
    keys::parse_private(&text, Some(key_pw))
}

impl CryptTab {
    pub fn drop(&mut self, p: &Path) -> bool {
        if p.is_file() {
            self.file_mode = true;
            self.src = p.to_string_lossy().to_string();
            self.src_changed();
            true
        } else {
            false
        }
    }

    fn src_changed(&mut self) {
        self.done = None;
        let p = Path::new(&self.src);
        if !p.is_file() {
            self.info = None;
            return;
        }
        self.info = Some(crypt::peek_file(p));
        let encrypted = matches!(self.info, Some(Ok(_)));
        self.dst = crypt::unused(&crypt::default_out(p, !encrypted)).to_string_lossy().to_string();
        if let Some(Ok(i)) = self.info {
            self.key_mode = i.lock != LockKind::Password;
        }
    }

    fn run(&mut self, encrypting: bool) {
        self.done = None;
        let cipher = self.cipher;
        let key_mode = self.key_mode;
        let pw = Zeroizing::new(self.pw.clone());
        let pub_key = self.pub_key.clone();
        let priv_key = self.priv_key.clone();
        let key_pw = Zeroizing::new(self.key_pw.clone());
        let text = Zeroizing::new(self.text.clone());
        let (src, dst) = (PathBuf::from(&self.src), PathBuf::from(&self.dst));
        let file_mode = self.file_mode;
        self.job = Some(Job::start(move |p| {
            if encrypting {
                let recipient = if key_mode { Some(keys::parse_public(&keys::read_key_file(Path::new(&pub_key))?)?) } else { None };
                let lock = match &recipient {
                    Some(k) => k.lock()?,
                    None => Lock::Password(pw.as_bytes(), Kdf::default()),
                };
                if file_mode {
                    crypt::encrypt_file(cipher, &lock, &src, &dst, false, Some(p))?;
                    Ok(Done::File(dst, None))
                } else {
                    Ok(Done::Text(crypt::encrypt_text(cipher, &lock, &text)?, None))
                }
            } else {
                let info = if file_mode { crypt::peek_file(&src)? } else { crypt::peek_text(&text)? };
                let key = if info.lock == LockKind::Password { None } else { Some(load_private(&priv_key, &key_pw)?) };
                let un = Unlock { password: (info.lock == LockKind::Password).then_some(pw.as_bytes()), key: key.as_ref() };
                if file_mode {
                    let info = crypt::decrypt_file(&un, &src, &dst, false, Some(p))?;
                    Ok(Done::File(dst, Some(info)))
                } else {
                    let (plain, info) = crypt::decrypt_text(&un, &text)?;
                    let (shown, _) = encsha::encode::show(&plain);
                    Ok(Done::Text(shown, Some(info)))
                }
            }
        }));
    }

    pub fn show(&mut self, ui: &mut Ui, s: &mut Shared) {
        let de = s.de;
        if let Some(r) = finished(&mut self.job) {
            if let Ok(Done::Text(out, _)) = &r {
                self.out_text = out.clone();
            }
            if let Ok(Done::File(..)) = &r {
                // the output exists now: suggest a fresh name for the next run
                self.src_changed_keep_done();
            }
            self.done = Some(r);
        }
        let running = self.job.is_some();

        panel(ui, |ui| {
            panel_head(
                ui,
                t(de, "Ver- und entschlüsseln", "Encrypt and decrypt"),
                t(de, "Text oder Dateien, mit Passwort oder für den öffentlichen Schlüssel einer Person", "Text or files, with a password or for someone's public key"),
            );
            ui.add_space(4.0);
            if let Some(i) = segmented(ui, &["Text", t(de, "Datei", "File")], self.file_mode as usize, !running) {
                self.file_mode = i == 1;
                self.done = None;
            }
            ui.add_space(2.0);
            if self.file_mode {
                label(ui, t(de, "Datei", "File"));
                if file_row(ui, "crypt-src", &mut self.src, t(de, "Datei wählen oder auf das Fenster ziehen", "Pick a file or drop it on the window"), false) {
                    self.src_changed();
                }
                match &self.info {
                    Some(Ok(i)) => chip_blue(ui, &format!("🔒 {} · {} · {}", t(de, "verschlüsselt", "encrypted"), i.cipher.name(), lock_name(de, i.lock))),
                    Some(Err(_)) => chip_gray(ui, t(de, "nicht verschlüsselt", "not encrypted")),
                    None => {}
                }
                label(ui, t(de, "Ziel", "Output"));
                file_row(ui, "crypt-dst", &mut self.dst, t(de, "wird automatisch vorgeschlagen", "suggested automatically"), true);
            } else {
                label(ui, t(de, "Eingabe", "Input"));
                if text_area(ui, egui::Id::new("crypt-in"), &mut self.text, t(de, "Klartext zum Verschlüsseln – oder verschlüsselten Text (Base64) zum Entschlüsseln", "Plain text to encrypt – or encrypted text (Base64) to decrypt"), 5, true)
                    .changed()
                {
                    self.info = Some(crypt::peek_text(&self.text)).filter(|i| i.is_ok());
                }
                if let Some(Ok(i)) = &self.info {
                    chip_blue(ui, &format!("🔒 {} · {} · {}", t(de, "verschlüsselt", "encrypted"), i.cipher.name(), lock_name(de, i.lock)));
                }
            }
        });
        ui.add_space(12.0);

        panel(ui, |ui| {
            ui.label(RichText::new(t(de, "Womit", "With")).size(15.0).family(semibold()));
            if let Some(i) = segmented(ui, &[t(de, "Passwort", "Password"), t(de, "Schlüsselpaar", "Key pair")], self.key_mode as usize, !running) {
                self.key_mode = i == 1;
            }
            ui.add_space(2.0);
            if self.key_mode {
                label(ui, t(de, "Öffentlicher Schlüssel des Empfängers (zum Verschlüsseln)", "Recipient's public key (to encrypt)"));
                file_row(ui, "crypt-pub", &mut self.pub_key, "….pub.pem (X25519 / RSA)", false);
                self.pub_info.update(&self.pub_key, de).chip(ui);
                label(ui, t(de, "Dein privater Schlüssel (zum Entschlüsseln)", "Your private key (to decrypt)"));
                file_row(ui, "crypt-priv", &mut self.priv_key, "….key.pem", false);
                self.priv_info.update(&self.priv_key, de).chip(ui);
                if self.priv_info.locked() {
                    password_row(ui, "crypt-keypw", &mut self.key_pw, &mut self.show_key_pw, t(de, "Passwort des privaten Schlüssels", "Private key password"));
                }
                hint(ui, t(de, "Schlüsselpaare erzeugst du im Tab „Schlüssel & Signaturen“ (X25519 oder RSA).", "Make key pairs in the \"Keys & signatures\" tab (X25519 or RSA)."));
            } else {
                label(ui, t(de, "Passwort", "Password"));
                password_row(ui, "crypt-pw", &mut self.pw, &mut self.show_pw, t(de, "Passwort", "Password"));
                if !self.pw.is_empty() {
                    let st = pw::strength(&self.pw);
                    ui.horizontal(|ui| {
                        meter(ui, st.level, 160.0);
                        ui.label(RichText::new(format!("~{:.0} bit · {} {}", st.bits, t(de, "knackbar in", "cracked in"), pw::duration(st.crack_secs, de))).size(11.5).color(TEXT_2));
                    });
                }
                password_row(ui, "crypt-pw2", &mut self.pw2, &mut self.show_pw, t(de, "Wiederholen (nur zum Verschlüsseln)", "Repeat (only to encrypt)"));
                if !self.pw2.is_empty() && self.pw2 != self.pw {
                    chip_red(ui, t(de, "Die Passwörter sind verschieden", "The passwords are different"));
                }
            }
            ui.add_space(4.0);
            label(ui, t(de, "Verfahren (nur zum Verschlüsseln)", "Cipher (only to encrypt)"));
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("crypt-cipher").width(230.0).selected_text(self.cipher.name()).show_ui(ui, |ui| {
                    ui.label(RichText::new(t(de, "Empfohlen (authentifiziert)", "Recommended (authenticated)")).size(11.0).color(TEXT_3));
                    for c in Cipher::ALL.into_iter().filter(|c| c.aead()) {
                        ui.selectable_value(&mut self.cipher, c, c.name());
                    }
                    ui.separator();
                    ui.label(RichText::new(t(de, "Blockchiffren + HMAC-SHA256", "Block ciphers + HMAC-SHA256")).size(11.0).color(TEXT_3));
                    for c in Cipher::ALL.into_iter().filter(|c| !c.aead()) {
                        ui.selectable_value(&mut self.cipher, c, c.name());
                    }
                });
                match warning(de, self.cipher) {
                    Some(w) => chip_orange(ui, w),
                    None => hint(ui, t(de, "Schlüssel aus dem Passwort per Argon2id (64 MiB)", "Key from the password via Argon2id (64 MiB)")),
                }
            });
        });
        ui.add_space(12.0);

        panel(ui, |ui| {
            let have_input = if self.file_mode { Path::new(&self.src).is_file() && !self.dst.trim().is_empty() } else { !self.text.is_empty() };
            let can_enc = have_input && !running && if self.key_mode { !self.pub_key.is_empty() } else { !self.pw.is_empty() && self.pw == self.pw2 };
            let can_dec = have_input && !running && if self.key_mode { !self.priv_key.is_empty() } else { !self.pw.is_empty() };
            ui.horizontal(|ui| {
                if btn_primary(ui, t(de, "🔒 Verschlüsseln", "🔒 Encrypt"), 170.0, can_enc).clicked() {
                    self.run(true);
                }
                if btn_primary(ui, t(de, "🔓 Entschlüsseln", "🔓 Decrypt"), 170.0, can_dec).clicked() {
                    self.run(false);
                }
                if !self.key_mode && !self.pw.is_empty() && self.pw2.is_empty() {
                    hint(ui, t(de, "Zum Verschlüsseln das Passwort wiederholen.", "Repeat the password to encrypt."));
                }
            });
            if let Some(j) = &self.job {
                job_view(ui, de, j, t(de, "Arbeite (Argon2 zuerst, dann die Daten)", "Working (Argon2 first, then the data)"));
            }
            let mut use_as_input = false;
            match &self.done {
                Some(Ok(Done::File(p, info))) => {
                    ui.horizontal(|ui| {
                        chip_on(ui, &format!("✓ {} {}", t(de, "Fertig:", "Done:"), p.display()));
                        if let Some(i) = info {
                            chip_gray(ui, &format!("{} · {}", i.cipher.name(), lock_name(de, i.lock)));
                        }
                        if btn_small(ui, t(de, "Im Ordner zeigen", "Show in folder")).clicked() {
                            show_in_folder(p);
                        }
                    });
                }
                Some(Ok(Done::Text(_, info))) => {
                    let info = *info;
                    ui.horizontal(|ui| {
                        label(ui, t(de, "Ausgabe", "Output"));
                        if let Some(i) = info {
                            chip_gray(ui, &format!("{} · {}", i.cipher.name(), lock_name(de, i.lock)));
                        }
                        copy_btn(ui, s, &self.out_text);
                        use_as_input = btn_small(ui, t(de, "↑ als Eingabe", "↑ use as input")).clicked();
                    });
                    let mut out = self.out_text.clone();
                    text_area(ui, egui::Id::new("crypt-out"), &mut out, "", 5, false);
                }
                Some(Err(e)) => error_line(ui, de, e),
                None => {}
            }
            if use_as_input {
                self.text = std::mem::take(&mut self.out_text);
                self.info = Some(crypt::peek_text(&self.text)).filter(|i| i.is_ok());
                self.done = None;
            }
        });
    }

    /// like src_changed, but keeps the result shown
    fn src_changed_keep_done(&mut self) {
        let done = self.done.take();
        self.src_changed();
        self.done = done;
    }
}

fn warning(de: bool, c: Cipher) -> Option<&'static str> {
    let w = c.warning()?;
    if !de {
        return Some(w);
    }
    Some(match c {
        Cipher::DesCbc => "gebrochen: 56-Bit-Schlüssel, in Stunden geknackt",
        Cipher::TripleDesCbc | Cipher::BlowfishCbc | Cipher::Cast5Cbc => "veraltet: 64-Bit-Blöcke, nur für alte Systeme",
        _ => "hier langsam (3–10 MB/s): gut für Text und kleine Dateien",
    })
}

fn lock_name(de: bool, l: LockKind) -> &'static str {
    match l {
        LockKind::Password => t(de, "Passwort", "password"),
        LockKind::X25519 => t(de, "X25519-Schlüssel", "X25519 key"),
        LockKind::Rsa => t(de, "RSA-Schlüssel", "RSA key"),
    }
}
