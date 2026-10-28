// pcmax.exe - the window + tray. `pcm.exe` is the same on the command line.
//   pcmax.exe                  window (a second start just shows the first one)
//   pcmax.exe --tray           start hidden in the tray (autostart)
//   pcmax.exe --tab boost      open on a tab (procs, boost, rules, files, monitor, settings)
//   pcmax.exe --select chrome  open with a process selected (name or PID)
//   pcmax.exe extract <file>   small progress window (right-click menu)
//   pcmax.exe copy <folder>    asks for the target folder, then copies

#![windows_subsystem = "windows"] // no console window next to the app

mod gui;

use eframe::egui;
use gui::theme::app_icon;
use gui::{App, NAME};
use pcmax::engine;
use std::path::PathBuf;
use std::sync::Mutex;

/// the single-instance mutex, released before a restart as admin
pub static SINGLE: Mutex<Option<pcmax::win::Handle>> = Mutex::new(None);

fn main() -> eframe::Result<()> {
    // release builds abort on panic without a window -> leave a note
    std::panic::set_hook(Box::new(|info| {
        if let Some(d) = pcmax::settings::dir() {
            let _ = std::fs::create_dir_all(&d);
            let bt = std::backtrace::Backtrace::force_capture();
            let _ = std::fs::write(d.join("crash.txt"), format!("{info}\n\n{bt}"));
        }
    }));
    let args: Vec<String> = std::env::args().skip(1).collect();
    match (args.first().map(String::as_str), args.get(1)) {
        (Some("extract"), Some(p)) => task(gui::task::Kind::Extract, PathBuf::from(p)),
        (Some("copy"), Some(p)) => task(gui::task::Kind::Copy, PathBuf::from(p)),
        _ => {
            let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
            let tab = value("--tab").and_then(|t| gui::Tab::parse(&t));
            window(args.iter().any(|a| a == "--tray"), args.iter().any(|a| a == "--after-restart"), tab, value("--select"))
        }
    }
}

fn window(tray: bool, after_restart: bool, tab: Option<gui::Tab>, select: Option<String>) -> eframe::Result<()> {
    // after "restart as admin" the old process may still hold the mutex for a moment
    let tries = if after_restart { 50 } else { 1 };
    let mut single = None;
    for _ in 0..tries {
        single = engine::claim_single_instance();
        if single.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let Some(single) = single else {
        // already running: bring that window to the front
        engine::signal(engine::EV_SHOW);
        return Ok(());
    };
    *SINGLE.lock().unwrap() = Some(single);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(NAME)
            .with_inner_size([1240.0, 840.0])
            .with_min_inner_size([980.0, 620.0])
            .with_drag_and_drop(true)
            // autostart: only the tray icon, the window comes with a click on it
            .with_visible(!tray)
            .with_icon(app_icon()),
        ..Default::default()
    };
    eframe::run_native(
        NAME,
        options,
        Box::new(move |cc| {
            let mut app = App::new(cc);
            app.hide_pending = tray;
            if let Some(t) = tab {
                app.tab = t;
                if t == gui::Tab::Settings {
                    app.load_shell_state();
                }
            }
            if let Some(sel) = select {
                let pid = sel.parse::<u32>().ok().or_else(|| pcmax::procs::find_by_name(&pcmax::procs::snapshot(), &sel).first().copied());
                if let Some(pid) = pid {
                    app.tab = gui::Tab::Procs;
                    app.select(pid);
                }
            }
            Ok(Box::new(app))
        }),
    )
}

fn task(kind: gui::task::Kind, src: PathBuf) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("pcmax")
            .with_inner_size([560.0, 300.0])
            .with_resizable(false)
            .with_icon(app_icon()),
        ..Default::default()
    };
    eframe::run_native("pcmax-task", options, Box::new(move |cc| Ok(Box::new(gui::task::TaskApp::new(cc, kind, src)))))
}
