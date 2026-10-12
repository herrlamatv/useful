// Left side: where to scan, filters, hide rules.

use crate::filter::DateField;
use crate::i18n::fill;
use crate::model::bit_index;
use crate::rules::{R_HTML, R_MANUAL, RULES};
use crate::theme::*;
use crate::{scan, win, App};
use eframe::egui::{self, Id, RichText, Vec2};
use std::time::Instant;

impl App {
    pub(crate) fn side_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            self.locations_panel(ui, ctx);
            ui.add_space(6.0);
            self.filter_panel(ui);
            ui.add_space(6.0);
            self.hide_panel(ui);
            ui.add_space(6.0);
        });
    }

    fn locations_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let t = self.t;
        let scanning = self.scan.is_some();
        panel(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_max_width(ui.available_width() - 40.0);
                    panel_head(ui, t.loc_title, t.loc_sub);
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    if icon_btn(ui, "⟳", true).on_hover_text(t.refresh_drives).clicked() {
                        self.load_drives(ctx);
                    }
                });
            });
            ui.add_space(2.0);

            label(ui, t.drives);
            let drives = self.drives.lock().unwrap().clone();
            list(ui, |ui| {
                for (i, d) in drives.iter().enumerate() {
                    let letter = d.root.trim_end_matches('\\');
                    let title = if d.label.is_empty() { letter.to_string() } else { format!("{letter}  {}", d.label) };
                    let kind = t.drive_kinds[d.kind as usize];
                    let free = fill(t.drive_free, &[("free", &t.size(d.free)), ("total", &t.size(d.total))]);
                    let mut on = self.has_location(&d.root);
                    item(ui, i == 0, &title, &format!("{kind} · {free}"), |ui| {
                        if toggle(ui, &mut on, !scanning).changed() {
                            self.set_location(&d.root, on);
                        }
                    });
                }
            });

            label(ui, t.quick);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                for (k, path) in self.known.clone() {
                    let on = self.has_location(&path);
                    if chip_toggle(ui, t.known[k], "", on, None).on_hover_text(&path).clicked() && !scanning {
                        self.set_location(&path, !on);
                    }
                }
            });

            // chosen folders that are not whole drives
            let custom: Vec<String> = self.s.locations.iter().filter(|l| l.trim_end_matches('\\').len() > 2).cloned().collect();
            if !custom.is_empty() {
                label(ui, t.folders);
                let mut remove = None;
                list(ui, |ui| {
                    for (i, l) in custom.iter().enumerate() {
                        let name = l.trim_end_matches('\\').rsplit('\\').next().unwrap_or(l).to_string();
                        let parent = self.s.locations.iter().find(|o| !crate::same_path(o, l) && scan::is_inside(l, o));
                        let mut sub = l.clone();
                        if let Some(p) = parent {
                            sub = format!("{sub}\n{}", fill(t.nested, &[("p", p)]));
                        } else if !std::path::Path::new(l).is_dir() {
                            sub = format!("{sub}  ✕");
                        }
                        item(ui, i == 0, &name, &sub, |ui| {
                            if icon_btn(ui, "✕", !scanning).on_hover_text(t.remove).clicked() {
                                remove = Some(l.clone());
                            }
                        });
                    }
                });
                if let Some(r) = remove {
                    self.set_location(&r, false);
                }
            }

            ui.horizontal(|ui| {
                if btn(ui, t.add_folder, !scanning).clicked() {
                    if let Some(dirs) = rfd::FileDialog::new().set_title(t.add_folder.trim_start_matches(['+', ' '])).pick_folders() {
                        for d in dirs {
                            self.set_location(&d.to_string_lossy(), true);
                        }
                    }
                }
                hint(ui, t.drop_hint);
            });

            sep(ui);
            item(ui, true, t.skip_system, t.skip_system_sub, |ui| {
                toggle(ui, &mut self.s.skip_system, !scanning);
            });
            ui.add_space(2.0);

            let w = ui.available_width();
            if scanning {
                if btn_danger(ui, t.cancel, w).clicked() {
                    self.cancel_scan();
                }
            } else {
                let text = if self.index.is_some() { t.scan_again } else { t.scan };
                if btn_primary(ui, text, w, !self.s.locations.is_empty()).on_hover_text("F5").clicked() {
                    self.start_scan(ctx);
                }
            }
            if self.s.locations.is_empty() {
                hint(ui, t.no_location);
            } else if self.index.is_some() && !scanning && self.locations_changed() {
                note(ui, t.loc_changed);
            }
        });
    }

    fn filter_panel(&mut self, ui: &mut egui::Ui) {
        let t = self.t;
        let res = self.result.clone();
        panel(ui, |ui| {
            panel_head(ui, t.filter_title, t.filter_sub);
            ui.add_space(2.0);

            label(ui, t.types);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                for (i, &color) in CAT_COLORS.iter().enumerate() {
                    let bit = 1u16 << i;
                    let count = res.as_ref().map(|r| t.int(r.cat_counts[i] as u64)).unwrap_or_default();
                    if chip_toggle(ui, t.cats[i], &count, self.s.cats & bit != 0, Some(color)).clicked() {
                        self.s.cats ^= bit;
                    }
                }
                if chip_toggle(ui, t.git_repos, "", self.s.git_only, None).clicked() {
                    self.s.git_only = !self.s.git_only;
                }
            });

            label(ui, t.ext);
            let w = ui.available_width();
            if text_input(ui, Id::new("exts"), &mut self.s.exts, t.ext_hint, w).changed() {
                self.typed_at = Some(Instant::now());
            }

            label(ui, t.date);
            let field = (self.s.date_field == DateField::Created) as usize;
            if let Some(i) = segmented(ui, &t.date_fields, field, true) {
                self.s.date_field = if i == 1 { DateField::Created } else { DateField::Modified };
            }
            if let Some(i) = segmented(ui, &t.date_modes, self.s.date_mode, true) {
                self.s.date_mode = i;
            }
            match self.s.date_mode {
                1 => {
                    if let Some(i) = segmented(ui, &t.older_short, self.s.older_idx, true) {
                        self.s.older_idx = i;
                    }
                }
                2 => {
                    let this_year = win::year_of(crate::model::unix_now()).unwrap_or(2026);
                    ui.horizontal(|ui| {
                        hint(ui, t.year_from);
                        ui.add(egui::DragValue::new(&mut self.s.year_from).range(1980..=this_year).speed(0.05));
                        hint(ui, t.year_to);
                        ui.add(egui::DragValue::new(&mut self.s.year_to).range(1980..=this_year).speed(0.05));
                    });
                }
                _ => {}
            }
            if self.s.date_field == DateField::Created {
                hint(ui, t.created_hint);
            }

            label(ui, t.size);
            if let Some(i) = segmented(ui, &t.size_opts, self.s.size_idx, true) {
                self.s.size_idx = i;
            }

            sep(ui);
            item(ui, true, t.match_folders, t.match_folders_sub, |ui| {
                toggle(ui, &mut self.s.match_folders, true);
            });

            let active = self.s.cats != 0
                || self.s.git_only
                || !self.s.exts.trim().is_empty()
                || self.s.date_mode != 0
                || self.s.size_idx != 0
                || !self.search.is_empty();
            if active && btn(ui, t.reset, true).clicked() {
                self.s.cats = 0;
                self.s.git_only = false;
                self.s.exts.clear();
                self.s.date_mode = 0;
                self.s.size_idx = 0;
                self.search.clear();
                self.typed_at = Some(Instant::now() - std::time::Duration::from_secs(1));
            }
        });
    }

    fn hide_panel(&mut self, ui: &mut egui::Ui) {
        let t = self.t;
        let res = self.result.clone();
        panel(ui, |ui| {
            panel_head(ui, t.hide_title, t.hide_sub);
            ui.add_space(2.0);
            list(ui, |ui| {
                for (k, def) in RULES.iter().enumerate() {
                    let name = t.rule_name(def.bit, self.s.html_n);
                    let examples = match def.bit {
                        R_HTML => t.html_examples,
                        R_MANUAL => t.manual_examples,
                        _ => def.examples,
                    };
                    let stat = res.as_ref().map(|r| {
                        let st = r.rule_stats[bit_index(def.bit)];
                        if st.folders == 0 && st.files == 0 {
                            t.rule_none.to_string()
                        } else {
                            fill(
                                t.rule_stat,
                                &[("f", &t.int(st.folders as u64)), ("n", &t.int(st.files)), ("s", &t.size(st.bytes))],
                            )
                        }
                    });
                    let sub = match stat {
                        Some(s) => format!("{examples}\n{s}"),
                        None => examples.to_string(),
                    };
                    let mut on = self.s.rules & def.bit != 0;
                    item(ui, k == 0, &name, &sub, |ui| {
                        if toggle(ui, &mut on, true).changed() {
                            self.s.rules ^= def.bit;
                        }
                    });
                    if def.bit == R_HTML {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("N =").size(11.5).color(TEXT_2));
                            stepper(ui, &mut self.s.html_n, 1, 50);
                        });
                        ui.add_space(8.0);
                    }
                }
            });
            if let Some(r) = &res {
                let text = fill(t.show_list, &[("n", &t.int(r.hidden_top.len() as u64))]);
                if btn(ui, &text, true).clicked() {
                    self.s.view = crate::filter::View::Hidden;
                    self.rows_dirty = true;
                }
            }
        });
    }
}
