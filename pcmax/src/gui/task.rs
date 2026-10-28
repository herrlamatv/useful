// Small window for the right-click menu: "pcmax.exe extract <archive>" / "pcmax.exe copy <folder>".
// Starts right away with the options from the settings, shows progress, then "open folder".

use super::tab_files::job_view;
use super::theme::*;
use super::{german, t, Job};
use eframe::egui::{self, RichText};
use pcmax::settings::Settings;
use pcmax::{copy, extract};
use std::path::PathBuf;

pub enum Kind {
    Extract,
    Copy,
}

pub struct TaskApp {
    de: bool,
    title: String,
    from: PathBuf,
    to: PathBuf,
    job: Option<Job>,
}

impl TaskApp {
    /// None = the user cancelled the target folder dialog
    pub fn new(cc: &eframe::CreationContext<'_>, kind: Kind, src: PathBuf) -> TaskApp {
        fonts(&cc.egui_ctx);
        style(&cc.egui_ctx);
        let s = Settings::load();
        let de = german(&s.lang);
        let threads = pcmax::fsx::default_threads();
        let admin = pcmax::win::is_elevated();
        match kind {
            Kind::Extract => {
                let dest = extract::default_dest(&src);
                let opts = extract::Options {
                    threads: if s.extract_threads == 0 { threads } else { s.extract_threads },
                    overwrite: s.overwrite,
                    defender_skip: s.defender_skip && admin,
                    scan_after: s.defender_skip && admin && s.scan_after,
                };
                let (a, d) = (src.clone(), dest.clone());
                TaskApp {
                    de,
                    title: t(de, "Entpacken", "Unpacking").into(),
                    from: src,
                    to: dest,
                    job: Some(Job::start(move |p| extract::extract(&a, &d, &opts, p))),
                }
            }
            Kind::Copy => {
                let target = rfd::FileDialog::new().set_title(t(de, "In welchen Ordner kopieren?", "Copy into which folder?")).pick_folder();
                let Some(target) = target else { std::process::exit(0) };
                let opts = copy::Options {
                    threads: if s.copy_threads == 0 { threads.min(16) } else { s.copy_threads },
                    overwrite: s.overwrite,
                    verify: s.verify_copy,
                };
                let (a, d) = (src.clone(), target.clone());
                TaskApp {
                    de,
                    title: t(de, "Kopieren", "Copying").into(),
                    from: src,
                    to: target,
                    job: Some(Job::start(move |p| copy::copy(&a, &d, &opts, p))),
                }
            }
        }
    }
}

impl eframe::App for TaskApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let de = self.de;
        egui::CentralPanel::default().frame(egui::Frame::none().fill(BG).inner_margin(egui::Margin::same(16.0))).show(ctx, |ui| {
            panel(ui, |ui| {
                ui.horizontal(|ui| {
                    logo(ui, 24.0);
                    ui.label(RichText::new(&self.title).size(15.0).family(semibold()));
                });
                hint(ui, &self.from.to_string_lossy());
                hint(ui, &format!("→ {}", self.to.display()));
                ui.add_space(6.0);
                if let Some(j) = self.job.as_mut() {
                    if job_view(ui, de, j, t(de, "Schließen", "Close")) {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            });
        });
        ctx.request_repaint_after(std::time::Duration::from_millis(150));
    }
}
