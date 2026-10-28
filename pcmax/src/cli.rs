// `pcm` - the command line. Every GUI feature is also here, for scripts and .bat files.

use crate::cpu::{Eco, Prio};
use crate::gpu::GpuPref;
use crate::io::IoPrio;
use crate::mem::RamMode;
use crate::profile::{self, Profile, Report};
use crate::win::{fmt_bytes, parse_size};
use crate::boost::{self, Measure};
use crate::settings::Settings;
use crate::{bench, copy, engine, extract, fast, fsx, power, procs, shell, topo, undo};
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(
    name = "pcm",
    version,
    about = "pcmax command line: give a process / window more or less CPU, RAM, GPU and I/O, boost the PC, unzip / copy / print fast",
    after_help = "TARGET: a PID, an exe name (all matching processes), 'pick' (click a window), 'fg' (the window in front after 3 s) or 'shell' (the shell pcm runs in)\n\nExamples:\n  pcm set pick --cpu 10-50\n  pcm set chrome --cpu 20 --eco on\n  pcm set shell --boost\n  pcm run --cores vcache --prio high -- game.exe\n  pcm fast -- pwsh -NoProfile -c \"1..10000000\""
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// List processes, CPU % measured over 1 s
    List {
        /// only names containing this
        filter: Option<String>,
        #[arg(long, default_value_t = 25)]
        top: usize,
    },
    /// Show what a process has right now (priority, cores, Efficiency Mode, I/O, GPU, RAM, job limits)
    Info { target: String },
    /// Set limits / boost on running processes (+ their child processes)
    Set {
        target: String,
        #[command(flatten)]
        p: ProfileArgs,
    },
    /// Start a program with limits from the first moment (everything it starts gets them too)
    Run {
        #[command(flatten)]
        p: ProfileArgs,
        /// the program and its arguments, after --
        #[arg(last = true, required = true)]
        cmd: Vec<String>,
    },
    /// Everything back to normal
    Reset {
        target: String,
        /// only this process, not its children
        #[arg(long)]
        no_tree: bool,
    },
    /// Cores, CCDs, 3D V-Cache, P-/E-cores
    Topo,
    /// Test loads to see that limits work
    Bench {
        #[command(subcommand)]
        what: BenchCmd,
    },
    /// Unpack on all cores: zip, 7z, tar(.gz/.zst/.xz/.bz2), gz, zst, xz, bz2 - rar via WinRAR / 7-Zip
    Extract {
        archive: PathBuf,
        /// target folder (default: next to the archive, named like it)
        dest: Option<PathBuf>,
        #[arg(long)]
        threads: Option<usize>,
        /// replace files that already exist
        #[arg(long)]
        overwrite: bool,
        /// Defender skips the folder while unpacking (admin, removed right after)
        #[arg(long)]
        defender_skip: bool,
        /// no Defender scan afterwards (default: scan when --defender-skip is used)
        #[arg(long)]
        no_scan: bool,
    },
    /// Copy a file / folder INTO a folder on all cores (many small files: much faster than Explorer)
    Copy {
        src: PathBuf,
        dest_dir: PathBuf,
        #[arg(long)]
        threads: Option<usize>,
        #[arg(long)]
        overwrite: bool,
        /// compare every byte afterwards
        #[arg(long)]
        verify: bool,
    },
    /// Run a command and print its output in big blocks instead of line by line
    Fast {
        /// only show the last N lines (+ live counter)
        #[arg(long)]
        tail: Option<usize>,
        /// write the output into a file
        #[arg(long)]
        file: Option<PathBuf>,
        /// a PowerShell command, e.g. --ps "1..10000000" (pwsh itself prints line by line - this makes it one block)
        #[arg(long)]
        ps: Option<String>,
        /// the program and its arguments, after --
        #[arg(last = true, required_unless_present = "ps")]
        cmd: Vec<String>,
    },
    /// Set up: right-click menu, autostart, `fast` in the PowerShell profile
    Install {
        #[command(subcommand)]
        what: InstallCmd,
    },
    /// Remove what install set up
    Uninstall {
        #[command(subcommand)]
        what: UninstallCmd,
    },
    /// Revert everything in the undo journal (power plan, services, registry, Defender exclusions)
    Undo,
    /// The general boost. With pcmax running in the tray all measures work, without it only the system ones
    Boost {
        #[command(subcommand)]
        what: BoostCmd,
    },
    /// Run the engine (rules, boost, hotkeys) in this console until Ctrl+C - pcmax.exe does this in the tray
    Engine,
}

#[derive(Subcommand)]
enum BoostCmd {
    On {
        /// only these measures (plan, parking, timer, background, services, foreground, x3d, gpupref), comma separated
        #[arg(long)]
        only: Option<String>,
    },
    Off,
    Status,
}

#[derive(Subcommand)]
enum InstallCmd {
    /// "Unpack with pcmax" on archives, "Copy with pcmax to ..." on folders
    Menu,
    /// start pcmax in the tray at logon
    Autostart {
        /// as a task with admin rights (set up as admin once, no UAC prompt later)
        #[arg(long)]
        admin: bool,
    },
    /// `fast { ... }` in the PowerShell 7 + Windows PowerShell profile
    Pwsh,
}

#[derive(Subcommand)]
enum UninstallCmd {
    Menu,
    Autostart,
    Pwsh,
    /// all of it
    All,
}

#[derive(Subcommand)]
enum BenchCmd {
    /// Busy loop on N threads, prints the CPU share it really got
    Burn {
        #[arg(long)]
        threads: Option<usize>,
        #[arg(long, default_value_t = 10.0)]
        secs: f64,
    },
    /// Allocate + touch RAM (e.g. 8G), then hold it
    Alloc {
        size: String,
        #[arg(long, default_value_t = 5.0)]
        secs: f64,
    },
    /// CPU share of a process tree over some seconds
    Measure {
        target: String,
        #[arg(long, default_value_t = 10.0)]
        secs: f64,
    },
}

#[derive(Args, Clone, Default)]
pub struct ProfileArgs {
    /// CPU share of the whole PC in %: "50" = at most 50, "10-50" = at least 10, at most 50
    #[arg(long)]
    cpu: Option<String>,
    /// idle | below | normal | above | high | realtime
    #[arg(long)]
    prio: Option<String>,
    /// allow realtime priority (CPU and GPU) - can freeze the PC
    #[arg(long)]
    yes_realtime: bool,
    /// all | vcache | freq | p | e | list like 0-5,12-17
    #[arg(long)]
    cores: Option<String>,
    /// Efficiency Mode: on | off | auto
    #[arg(long)]
    eco: Option<String>,
    /// RAM limit, e.g. 4G or 512M
    #[arg(long)]
    ram: Option<String>,
    /// soft = page out above the limit (per process), hard = out of memory (all together)
    #[arg(long, default_value = "soft")]
    ram_mode: String,
    /// RAM priority 1 (very low) .. 5 (normal)
    #[arg(long)]
    ram_prio: Option<u8>,
    /// ask Windows to keep the current RAM of the process (trim it last)
    #[arg(long)]
    keep_ram: bool,
    /// I/O priority: verylow | low | normal | high
    #[arg(long)]
    io: Option<String>,
    /// GPU priority: idle | below | normal | above | high | realtime
    #[arg(long)]
    gpu: Option<String>,
    /// which GPU the app uses next start: auto | saving (iGPU) | high (RTX)
    #[arg(long)]
    gpu_pref: Option<String>,
    /// only the process itself, not its children
    #[arg(long)]
    no_tree: bool,
    /// shortcut: priority high, Efficiency Mode off, I/O high, GPU high
    #[arg(long)]
    boost: bool,
    /// shortcut: below normal, Efficiency Mode on, I/O low, GPU below normal
    #[arg(long)]
    calm: bool,
}

impl ProfileArgs {
    pub fn to_profile(&self) -> Result<Profile, String> {
        let mut p = if self.boost {
            Profile::boost()
        } else if self.calm {
            Profile::calm()
        } else {
            Profile::default()
        };
        let bad = |what: &str, v: &str| format!("--{what} {v}: unknown value");
        if let Some(c) = &self.cpu {
            (p.cpu_min, p.cpu_max) = profile::parse_cpu(c)?;
        }
        if let Some(v) = &self.prio {
            p.prio = Some(Prio::parse(v).ok_or_else(|| bad("prio", v))?);
        }
        if let Some(v) = &self.cores {
            topo::get().mask_for(v)?;
            p.cores = Some(v.clone());
        }
        if let Some(v) = &self.eco {
            p.eco = Some(Eco::parse(v).ok_or_else(|| bad("eco", v))?);
        }
        if let Some(v) = &self.ram {
            p.ram = Some(parse_size(v).ok_or_else(|| bad("ram", v))?);
        }
        p.ram_mode = RamMode::parse(&self.ram_mode).ok_or_else(|| bad("ram-mode", &self.ram_mode))?;
        if let Some(v) = self.ram_prio {
            p.ram_prio = Some(v.clamp(1, 5));
        }
        p.keep_ram = self.keep_ram;
        if let Some(v) = &self.io {
            p.io = Some(IoPrio::parse(v).ok_or_else(|| bad("io", v))?);
        }
        if let Some(v) = &self.gpu {
            p.gpu = Some(Prio::parse(v).ok_or_else(|| bad("gpu", v))?);
        }
        if let Some(v) = &self.gpu_pref {
            p.gpu_pref = Some(GpuPref::parse(v).ok_or_else(|| bad("gpu-pref", v))?);
        }
        p.tree = !self.no_tree;
        Ok(p)
    }
}

/// PID, exe name, "pick", "fg" or "shell" -> PIDs
pub fn resolve(target: &str) -> Result<Vec<u32>, String> {
    let t = target.trim();
    if let Ok(pid) = t.parse::<u32>() {
        return Ok(vec![pid]);
    }
    match t.to_ascii_lowercase().as_str() {
        "pick" => pick().map(|p| vec![p]),
        "fg" | "foreground" => {
            for i in (1..=3).rev() {
                eprint!("\rswitch to the window you mean ... {i} ");
                std::thread::sleep(Duration::from_secs(1));
            }
            eprintln!();
            let pid = procs::foreground_pid();
            if pid == 0 {
                Err("no window in front".into())
            } else {
                Ok(vec![pid])
            }
        }
        "shell" | "parent" => {
            let all = procs::snapshot();
            let me = all.iter().find(|p| p.pid == procs::own_pid()).ok_or("own process not found")?;
            Ok(vec![me.ppid])
        }
        _ => {
            let found = procs::find_by_name(&procs::snapshot(), t);
            if found.is_empty() {
                Err(format!("no running process '{t}'"))
            } else {
                Ok(found)
            }
        }
    }
}

/// wait for a mouse click on any window
pub fn pick() -> Result<u32, String> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_ESCAPE, VK_LBUTTON};
    let down = |vk: u16| unsafe { GetAsyncKeyState(vk as i32) } as u16 & 0x8000 != 0;
    eprintln!("click on the window you mean (Esc = cancel) ...");
    while down(VK_LBUTTON) {
        std::thread::sleep(Duration::from_millis(15));
    }
    let start = Instant::now();
    loop {
        if down(VK_ESCAPE) {
            return Err("cancelled".into());
        }
        if down(VK_LBUTTON) {
            let pid = procs::pid_at_cursor();
            return if pid == 0 { Err("no window there".into()) } else { Ok(pid) };
        }
        if start.elapsed() > Duration::from_secs(60) {
            return Err("no click within 60 s".into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn name_of(pid: u32) -> String {
    procs::snapshot().into_iter().find(|p| p.pid == pid).map(|p| p.name).unwrap_or_else(|| "?".into())
}

fn print_report(title: &str, rep: &Report) {
    println!("{title}");
    for l in rep.lines() {
        println!("  {l}");
    }
}

fn fail(e: impl std::fmt::Display) -> ExitCode {
    eprintln!("pcm: {e}");
    ExitCode::FAILURE
}

pub fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::List { filter, top } => list(filter, top),
        Cmd::Info { target } => match resolve(&target) {
            Ok(pids) => {
                let all = procs::snapshot();
                for pid in pids {
                    info(&all, pid);
                }
                ExitCode::SUCCESS
            }
            Err(e) => fail(e),
        },
        Cmd::Set { target, p } => {
            let prof = match p.to_profile() {
                Ok(x) => x,
                Err(e) => return fail(e),
            };
            if prof.is_empty() {
                return fail("nothing to set - see pcm set --help");
            }
            let pids = match resolve(&target) {
                Ok(x) => x,
                Err(e) => return fail(e),
            };
            let mut ok = true;
            for pid in pids {
                let rep = profile::apply(pid, &prof, p.yes_realtime);
                ok &= rep.is_ok();
                print_report(&format!("{} (PID {pid}, {} processes)", name_of(pid), rep.pids.len()), &rep);
            }
            if ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Cmd::Run { p, cmd } => run(&p, &cmd),
        Cmd::Reset { target, no_tree } => match resolve(&target) {
            Ok(pids) => {
                for pid in pids {
                    let name = name_of(pid);
                    print_report(&format!("{name} (PID {pid})"), &profile::reset(pid, !no_tree));
                }
                ExitCode::SUCCESS
            }
            Err(e) => fail(e),
        },
        Cmd::Topo => {
            print_topo();
            ExitCode::SUCCESS
        }
        Cmd::Bench { what } => bench_cmd(what),
        Cmd::Extract { archive, dest, threads, overwrite, defender_skip, no_scan } => {
            let dest = dest.unwrap_or_else(|| extract::default_dest(&archive));
            let opts = extract::Options {
                threads: threads.unwrap_or_else(fsx::default_threads),
                overwrite,
                defender_skip,
                scan_after: defender_skip && !no_scan,
            };
            println!("{} -> {}", archive.display(), dest.display());
            with_progress(|prog| extract::extract(&archive, &dest, &opts, prog))
        }
        Cmd::Copy { src, dest_dir, threads, overwrite, verify } => {
            let opts = copy::Options { threads: threads.unwrap_or(copy::Options::default().threads), overwrite, verify };
            println!("{} -> {}", src.display(), dest_dir.display());
            with_progress(|prog| copy::copy(&src, &dest_dir, &opts, prog))
        }
        Cmd::Fast { tail, file, ps, cmd } => match fast::run(&ps.map(|s| fast::ps_command(&s)).unwrap_or(cmd), &fast::Options { tail, file }) {
            Ok(s) => {
                let rate = if s.secs > 0.0 { s.lines as f64 / s.secs } else { 0.0 };
                eprintln!("pcm fast: {} lines ({}) in {:.2} s = {:.0} lines/s", s.lines, fmt_bytes(s.bytes), s.secs, rate);
                ExitCode::from(s.code.clamp(0, 255) as u8)
            }
            Err(e) => fail(e),
        },
        Cmd::Install { what } => {
            let r = match what {
                InstallCmd::Menu => shell::install_menu(crate::win::ui_is_german()).map(|_| "right-click menu installed".to_string()),
                InstallCmd::Autostart { admin } => {
                    shell::install_autostart(admin).map(|_| format!("autostart{} installed", if admin { " (admin task)" } else { "" }))
                }
                InstallCmd::Pwsh => shell::install_pwsh().map(|p| {
                    let list: Vec<String> = p.iter().map(|x| x.display().to_string()).collect();
                    format!("`fast {{ ... }}` added to {} (open a new shell)", list.join(", "))
                }),
            };
            match r {
                Ok(s) => {
                    println!("{s}");
                    ExitCode::SUCCESS
                }
                Err(e) => fail(e),
            }
        }
        Cmd::Uninstall { what } => {
            let r = match what {
                UninstallCmd::Menu => shell::uninstall_menu(),
                UninstallCmd::Autostart => shell::uninstall_autostart(),
                UninstallCmd::Pwsh => shell::uninstall_pwsh(),
                UninstallCmd::All => shell::uninstall_menu().and(shell::uninstall_autostart()).and(shell::uninstall_pwsh()),
            };
            match r {
                Ok(()) => {
                    println!("removed");
                    ExitCode::SUCCESS
                }
                Err(e) => fail(e),
            }
        }
        Cmd::Boost { what } => boost_cmd(what),
        Cmd::Engine => engine_cmd(),
        Cmd::Undo => {
            let r = undo::revert_all();
            if r.is_empty() {
                println!("nothing to undo");
            }
            let mut ok = true;
            for (what, res) in r {
                match res {
                    Ok(()) => println!("✓ {what}"),
                    Err(e) => {
                        ok = false;
                        println!("✗ {what}: {e}");
                    }
                }
            }
            if ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
    }
}

fn boost_cmd(what: BoostCmd) -> ExitCode {
    let mut s = Settings::load();
    let running = engine::running();
    match what {
        BoostCmd::On { only } => {
            if let Some(list) = only {
                let mut ms = vec![];
                for k in list.split(',') {
                    match Measure::parse(k) {
                        Some(m) => ms.push(m),
                        None => return fail(format!("unknown measure '{k}'")),
                    }
                }
                s.measures = ms;
            }
            s.boost_on = true;
            if let Err(e) = s.save() {
                return fail(e);
            }
            if running {
                // the engine reloads the settings (measures) and switches on
                engine::signal(engine::EV_RELOAD);
                println!("boost on (pcmax in the tray does it, see its window for details)");
                return ExitCode::SUCCESS;
            }
            for (m, r) in boost::apply_static(&s.measures, &s.gpu_apps) {
                if m.dynamic() {
                    println!("  - {}: needs pcmax running (tray) - not active", m.key());
                    continue;
                }
                match r {
                    Ok(t) => println!("  ✓ {}: {t}", m.key()),
                    Err(e) => println!("  ✗ {}: {e}", m.key()),
                }
            }
            println!("boost on - `pcm boost off` puts everything back");
            ExitCode::SUCCESS
        }
        BoostCmd::Off => {
            s.boost_on = false;
            let _ = s.save();
            if running {
                engine::signal(engine::EV_BOOST_OFF);
                println!("boost off (pcmax in the tray puts everything back)");
                return ExitCode::SUCCESS;
            }
            for (what, r) in boost::revert_static() {
                match r {
                    Ok(()) => println!("  ✓ {what}"),
                    Err(e) => println!("  ✗ {what}: {e}"),
                }
            }
            println!("boost off");
            ExitCode::SUCCESS
        }
        BoostCmd::Status => {
            println!("boost: {}   engine (pcmax tray): {}", if s.boost_on { "on" } else { "off" }, if running { "running" } else { "not running" });
            for m in Measure::ALL {
                let admin = if m.needs_admin() { " (admin)" } else if m.partly_admin() { " (partly admin)" } else { "" };
                let dynm = if m.dynamic() { " [needs the engine]" } else { "" };
                println!("  [{}] {}{admin}{dynm}", if s.has(m) { "x" } else { " " }, m.key());
            }
            println!("power plan: {}", power::active().map(|g| power::name_of(&g)).unwrap_or_default());
            let j = undo::load();
            if !j.is_empty() {
                println!("undo journal:");
                for e in j {
                    println!("  {}", e.describe());
                }
            }
            ExitCode::SUCCESS
        }
    }
}

fn engine_cmd() -> ExitCode {
    use std::sync::atomic::{AtomicBool, Ordering};
    static QUIT: AtomicBool = AtomicBool::new(false);
    unsafe extern "system" fn on_ctrl(_: u32) -> windows_sys::Win32::Foundation::BOOL {
        QUIT.store(true, Ordering::SeqCst);
        1
    }
    let Some(_single) = engine::claim_single_instance() else {
        return fail("pcmax already runs (tray) - only one engine at a time");
    };
    unsafe { windows_sys::Win32::System::Console::SetConsoleCtrlHandler(Some(on_ctrl), 1) };
    let h = engine::start(Settings::load(), Box::new(|m| println!(">> {m}")));
    let mut shown = 0;
    while !QUIT.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(200));
        let sh = h.shared.lock().unwrap();
        let total = sh.log.len();
        // log is capped at 300 lines, print what's new
        for l in sh.log.iter().skip(shown.min(total)) {
            println!("{l}");
        }
        shown = total;
    }
    println!("stopping - putting everything back ...");
    h.stop();
    ExitCode::SUCCESS
}

/// runs `work` on a thread and shows a progress line until it's done
fn with_progress(work: impl FnOnce(&fsx::Progress) -> Result<fsx::Summary, String> + Send) -> ExitCode {
    let prog = fsx::Progress::default();
    let t0 = Instant::now();
    let result = std::thread::scope(|s| {
        let h = s.spawn(|| work(&prog));
        while !h.is_finished() {
            std::thread::sleep(Duration::from_millis(200));
            let secs = t0.elapsed().as_secs_f64().max(0.001);
            let done = prog.done_bytes.load(std::sync::atomic::Ordering::Relaxed);
            let files = prog.done_files.load(std::sync::atomic::Ordering::Relaxed);
            let total = prog.total_files.load(std::sync::atomic::Ordering::Relaxed);
            let pct = prog.fraction().map(|f| format!("{:>3.0} %  ", f * 100.0)).unwrap_or_default();
            eprint!(
                "\r{pct}{files}/{total} files  {}  {}/s  {}        ",
                fmt_bytes(done),
                fmt_bytes((done as f64 / secs) as u64),
                prog.phase()
            );
        }
        h.join().unwrap_or_else(|_| Err("worker crashed".into()))
    });
    eprintln!();
    match result {
        Ok(s) => {
            println!("done: {}", s.line());
            println!("  -> {}", s.dest.display());
            for n in &s.notes {
                println!("  {n}");
            }
            for e in s.errors.iter().take(20) {
                println!("  ✗ {e}");
            }
            if s.errors.len() > 20 {
                println!("  ... {} more errors", s.errors.len() - 20);
            }
            if s.errors.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(e) => fail(e),
    }
}

fn list(filter: Option<String>, top: usize) -> ExitCode {
    let all = procs::snapshot();
    let mut s = procs::CpuSampler::new();
    s.sample(all.iter().map(|p| p.pid));
    std::thread::sleep(Duration::from_millis(1000));
    let cpu = s.sample(all.iter().map(|p| p.pid));
    let titles = procs::window_titles();
    let f = filter.map(|x| x.to_ascii_lowercase());
    let mut rows: Vec<_> = all
        .iter()
        .filter(|p| p.pid != 0 && f.as_ref().is_none_or(|f| p.name.to_ascii_lowercase().contains(f)))
        .map(|p| (p, cpu.get(&p.pid).copied().unwrap_or(0.0)))
        .collect();
    rows.sort_by(|a, b| b.1.total_cmp(&a.1));
    println!("{:>7}  {:<28} {:>6}  {:>9}  {:>4}  window", "PID", "name", "CPU %", "RAM", "thr");
    for (p, c) in rows.into_iter().take(top) {
        let ram = procs::memory(p.pid).map(|m| fmt_bytes(m.0)).unwrap_or_default();
        let title: String = titles.get(&p.pid).map(|t| t.chars().take(40).collect()).unwrap_or_default();
        println!("{:>7}  {:<28} {:>6.1}  {:>9}  {:>4}  {title}", p.pid, p.name, c, ram, p.threads);
    }
    ExitCode::SUCCESS
}

fn info(all: &[procs::ProcInfo], pid: u32) {
    let Some(s) = profile::state(all, pid) else {
        println!("PID {pid}: not found");
        return;
    };
    let t = topo::get();
    let opt = |o: Option<&str>| o.unwrap_or("?").to_string();
    println!("{} (PID {}, {} threads)", s.name, s.pid, s.threads);
    if let Some(e) = &s.exe {
        println!("  exe              {e}");
    }
    println!("  priority         {}", opt(s.prio.map(|p| p.key())));
    println!("  cores            {}", s.affinity.map(|m| t.describe(m)).unwrap_or_else(|| "?".into()));
    println!("  Efficiency Mode  {}", opt(s.eco.map(|e| e.key())));
    println!("  I/O priority     {}", opt(s.io.map(|e| e.key())));
    println!("  GPU priority     {}", opt(s.gpu.map(|e| e.key())));
    println!("  GPU preference   {}", opt(s.gpu_pref.map(|e| e.key())));
    println!("  RAM priority     {}", s.ram_prio.map(|p| p.to_string()).unwrap_or_else(|| "?".into()));
    println!(
        "  RAM              {} in RAM, {} private",
        s.working_set.map(fmt_bytes).unwrap_or_default(),
        s.private.map(fmt_bytes).unwrap_or_default()
    );
    match s.job {
        Some((root, l)) => {
            let mut parts = vec![];
            if l.cpu_limited() {
                parts.push(format!("CPU {}-{} %", l.cpu_min, l.cpu_max));
            }
            if let Some(a) = l.affinity {
                parts.push(format!("cores {}", t.describe(a)));
            }
            if let Some(m) = l.mem {
                parts.push(format!("RAM {}", fmt_bytes(m)));
            }
            if let Some(p) = l.prio {
                parts.push(format!("prio {}", p.key()));
            }
            let what = if parts.is_empty() { "no limits".to_string() } else { parts.join(", ") };
            println!("  pcmax job        {what} (job of PID {root})");
        }
        None => println!("  pcmax job        -"),
    }
}

fn run(p: &ProfileArgs, cmd: &[String]) -> ExitCode {
    use std::os::windows::io::AsRawHandle;
    use std::os::windows::process::CommandExt;
    #[link(name = "ntdll")]
    extern "system" {
        fn NtResumeProcess(h: windows_sys::Win32::Foundation::HANDLE) -> i32;
    }
    const CREATE_SUSPENDED: u32 = 0x4;
    let prof = match p.to_profile() {
        Ok(x) => x,
        Err(e) => return fail(e),
    };
    let mut child = match std::process::Command::new(&cmd[0]).args(&cmd[1..]).creation_flags(CREATE_SUSPENDED).spawn() {
        Ok(c) => c,
        Err(e) => return fail(format!("{}: {e}", cmd[0])),
    };
    let pid = child.id();
    // limits first, then let it run: so every child process starts inside the job
    let rep = if prof.is_empty() { Report::default() } else { profile::apply(pid, &prof, p.yes_realtime) };
    for l in rep.lines() {
        eprintln!("pcm: {l}");
    }
    unsafe { NtResumeProcess(child.as_raw_handle() as _) };
    let code = child.wait().map(|s| s.code().unwrap_or(1)).unwrap_or(1);
    drop(rep);
    ExitCode::from(code.clamp(0, 255) as u8)
}

fn print_topo() {
    let t = topo::get();
    println!("{} logical processors, {} cores", t.logical, t.cores.len());
    for (i, c) in t.l3.iter().enumerate() {
        let tag = if t.vcache == Some(c.mask) {
            "  <- 3D V-Cache (games: `--cores vcache`)"
        } else if t.freq.is_some_and(|f| f & c.mask == c.mask) {
            "  <- higher clock (work: `--cores freq`)"
        } else {
            ""
        };
        println!("  L3 #{i}: {:>4} MB  logical {}{tag}", c.size >> 20, topo::format_list(c.mask));
    }
    if let (Some(p), Some(e)) = (t.pcores, t.ecores) {
        println!("  P-cores: {}   E-cores: {}", topo::format_list(p), topo::format_list(e));
    }
    if !t.is_x3d() && t.pcores.is_none() {
        println!("  all cores are the same kind");
    }
}

fn bench_cmd(what: BenchCmd) -> ExitCode {
    match what {
        BenchCmd::Burn { threads, secs } => {
            let n = threads.unwrap_or(topo::logical_count() as usize);
            eprintln!("burning {n} threads for {secs} s ...");
            let pct = bench::burn(n, secs);
            println!("got {pct:.1} % of the PC (≈ {:.1} threads)", pct / 100.0 * topo::logical_count() as f64);
            ExitCode::SUCCESS
        }
        BenchCmd::Alloc { size, secs } => {
            let Some(bytes) = parse_size(&size) else { return fail("size like 8G or 512M") };
            let mut last = Instant::now();
            let (done, ws, r) = bench::alloc(bytes, secs, |d| {
                if last.elapsed() > Duration::from_millis(250) {
                    eprint!("\r{} ", fmt_bytes(d));
                    last = Instant::now();
                }
            });
            eprintln!();
            println!("allocated {} of {}, in RAM now: {}", fmt_bytes(done), fmt_bytes(bytes), fmt_bytes(ws));
            match r {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    println!("{e}");
                    ExitCode::from(2)
                }
            }
        }
        BenchCmd::Measure { target, secs } => match resolve(&target) {
            Ok(pids) => {
                for pid in pids {
                    let pct = bench::measure(pid, secs);
                    println!("{} (PID {pid}): {pct:.1} % of the PC", name_of(pid));
                }
                ExitCode::SUCCESS
            }
            Err(e) => fail(e),
        },
    }
}
