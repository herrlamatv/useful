// Encodings and classic ciphers, converted live while typing.

use super::theme::*;
use super::{copy_btn, t, Shared};
use eframe::egui::{self, RichText, Ui};
use encsha::encode::{self, Method};

pub struct EncodeTab {
    input: String,
    method: Method,
    key: String,
    decode: bool,
    output: String,
    /// error or "not text, shown as hex"
    note: Option<(String, bool)>,
    made_from: Option<(String, Method, String, bool)>,
}

impl Default for EncodeTab {
    fn default() -> EncodeTab {
        EncodeTab { input: String::new(), method: Method::Base64, key: String::new(), decode: false, output: String::new(), note: None, made_from: None }
    }
}

impl EncodeTab {
    fn convert(&mut self, de: bool) {
        let now = (self.input.clone(), self.method, self.key.clone(), self.decode);
        if self.made_from.as_ref() == Some(&now) {
            return;
        }
        self.made_from = Some(now);
        self.note = None;
        if self.input.is_empty() {
            self.output.clear();
            return;
        }
        let r = if self.decode {
            encode::decode(self.method, &self.input, &self.key).map(|b| {
                let (text, is_text) = encode::show(&b);
                if !is_text {
                    self.note = Some((t(de, "Kein Text – als Hex angezeigt", "Not text – shown as hex").into(), false));
                }
                text
            })
        } else {
            encode::encode(self.method, &self.input, &self.key)
        };
        match r {
            Ok(o) => self.output = o,
            Err(e) => {
                self.output.clear();
                self.note = Some((e, true));
            }
        }
    }

    pub fn show(&mut self, ui: &mut Ui, s: &mut Shared) {
        let de = s.de;
        panel(ui, |ui| {
            panel_head(
                ui,
                t(de, "Kodieren & klassische Chiffren", "Encodings & classic ciphers"),
                t(de, "Base64, Hex, URL … und Caesar, Vigenère, Morse – Rätsel, kein Schutz", "Base64, hex, URL … and Caesar, Vigenère, Morse – puzzles, not protection"),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("enc-method").width(170.0).selected_text(self.method.name()).show_ui(ui, |ui| {
                    ui.label(RichText::new(t(de, "Kodierungen", "Encodings")).size(11.0).color(TEXT_3));
                    for m in Method::ALL.into_iter().filter(|m| !m.classic()) {
                        ui.selectable_value(&mut self.method, m, m.name());
                    }
                    ui.separator();
                    ui.label(RichText::new(t(de, "Klassische Chiffren", "Classic ciphers")).size(11.0).color(TEXT_3));
                    for m in Method::ALL.into_iter().filter(|m| m.classic()) {
                        ui.selectable_value(&mut self.method, m, m.name());
                    }
                });
                let w = segmented_fit_width(ui, &["→ Kodieren", "← Dekodieren"], 12.5);
                ui.allocate_ui(egui::vec2(w, 30.0), |ui| {
                    if let Some(i) = segmented_fit(ui, &[t(de, "→ Kodieren", "→ Encode"), t(de, "← Dekodieren", "← Decode")], self.decode as usize, 12.5, 24.0) {
                        self.decode = i == 1;
                    }
                });
                if let Some(h) = self.method.key_hint() {
                    let hint_text = match self.method {
                        Method::Caesar => t(de, "Verschiebung (3)", "shift (3)"),
                        Method::Vigenere => t(de, "Schlüsselwort", "key word"),
                        _ => h,
                    };
                    text_input(ui, egui::Id::new("enc-key"), &mut self.key, hint_text, 180.0, false);
                }
            });
            label(ui, t(de, "Eingabe", "Input"));
            text_area(ui, egui::Id::new("enc-in"), &mut self.input, t(de, "Text hier eingeben", "Type text here"), 5, true);
            self.convert(de);
            ui.horizontal(|ui| {
                label(ui, t(de, "Ausgabe", "Output"));
                copy_btn(ui, s, &self.output);
                if btn_small(ui, t(de, "⇅ Tauschen", "⇅ Swap")).clicked() {
                    self.input = std::mem::take(&mut self.output);
                    self.decode = !self.decode;
                }
                match &self.note {
                    Some((n, true)) => chip_red(ui, n),
                    Some((n, false)) => chip_gray(ui, n),
                    None => {}
                }
            });
            let mut out = self.output.clone();
            text_area(ui, egui::Id::new("enc-out"), &mut out, "", 5, false);
        });
    }
}
