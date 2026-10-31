// encsha.exe - the window. `esh.exe` is the same on the command line.
//   encsha.exe              window
//   encsha.exe <file>       open with a file: .esh files go to "Encrypt", others to "Hash"
//   encsha.exe --tab keys   open on a tab (hash, crypt, keys, pw, encode, tools)

#![windows_subsystem = "windows"] // no console window next to the app

mod gui;

use eframe::egui;
use gui::theme::app_icon;
use gui::{App, Tab, NAME};
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    // release builds abort on panic without a window -> leave a note
    std::panic::set_hook(Box::new(|info| {
        if let Some(d) = std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("encsha")) {
            let _ = std::fs::create_dir_all(&d);
            let bt = std::backtrace::Backtrace::force_capture();
            let _ = std::fs::write(d.join("crash.txt"), format!("{info}\n\n{bt}"));
        }
    }));
    let args: Vec<String> = std::env::args().skip(1).collect();
    let tab = args.iter().position(|a| a == "--tab").and_then(|i| args.get(i + 1)).and_then(|t| {
        Some(match t.to_ascii_lowercase().as_str() {
            "hash" => Tab::Hash,
            "crypt" | "encrypt" => Tab::Crypt,
            "keys" | "sign" => Tab::Keys,
            "pw" | "passwords" => Tab::Pw,
            "encode" => Tab::Encode,
            "tools" => Tab::Tools,
            _ => return None,
        })
    });
    let open = args.iter().find(|a| !a.starts_with("--") && PathBuf::from(a).exists()).map(PathBuf::from);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(NAME)
            .with_inner_size([1060.0, 820.0])
            .with_min_inner_size([760.0, 560.0])
            .with_drag_and_drop(true)
            .with_icon(app_icon()),
        ..Default::default()
    };
    eframe::run_native(
        NAME,
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, tab, open)))),
    )
}
