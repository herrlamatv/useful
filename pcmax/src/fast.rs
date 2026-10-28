// Console fix. `1..10000000` in pwsh takes minutes not because counting is slow (0.3 s) but because
// every single line travels pwsh -> console host -> terminal on its own, on one thread.
// `pcm fast -- <command>` lets the command write into a pipe (fast) and puts the text on the
// screen in big blocks, or only shows the last lines / writes a file.

use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::os::windows::io::AsRawHandle;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::Storage::FileSystem::WriteFile;
use windows_sys::Win32::System::Pipes::PeekNamedPipe;
use windows_sys::Win32::System::Console::{
    GetConsoleMode, GetConsoleOutputCP, GetStdHandle, SetConsoleOutputCP, WriteConsoleW, STD_OUTPUT_HANDLE,
};

#[derive(Clone, Debug, Default)]
pub struct Options {
    /// only show the last N lines (+ a live line counter)
    pub tail: Option<usize>,
    /// write everything into this file instead of the screen
    pub file: Option<PathBuf>,
}

pub struct Stats {
    pub lines: u64,
    pub bytes: u64,
    pub secs: f64,
    pub code: i32,
}

/// stdout, written in big blocks
struct Out {
    h: HANDLE,
    console: bool,
    /// UTF-8 bytes of a character cut in half at the end of the last chunk
    pending: Vec<u8>,
    wide: Vec<u16>,
}

impl Out {
    fn new() -> Out {
        let h = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
        let mut mode = 0;
        let console = unsafe { GetConsoleMode(h, &mut mode) } != 0;
        Out { h, console, pending: vec![], wide: vec![] }
    }

    fn write(&mut self, data: &[u8]) {
        if !self.console {
            // redirected: just pass the bytes on
            let mut done = 0;
            while done < data.len() {
                let mut n = 0u32;
                let chunk = (data.len() - done).min(1 << 30);
                if unsafe { WriteFile(self.h, data[done..].as_ptr(), chunk as u32, &mut n, std::ptr::null_mut()) } == 0 || n == 0 {
                    return;
                }
                done += n as usize;
            }
            return;
        }
        // console: UTF-8 -> UTF-16, one WriteConsoleW per 256K characters
        self.pending.extend_from_slice(data);
        let valid = match std::str::from_utf8(&self.pending) {
            Ok(_) => self.pending.len(),
            Err(e) if e.error_len().is_none() => e.valid_up_to(),
            Err(_) => self.pending.len(),
        };
        let text = String::from_utf8_lossy(&self.pending[..valid]).into_owned();
        self.pending.drain(..valid);
        self.wide.clear();
        self.wide.extend(text.encode_utf16());
        for chunk in self.wide.chunks(256 * 1024) {
            let mut n = 0u32;
            unsafe { WriteConsoleW(self.h, chunk.as_ptr(), chunk.len() as u32, &mut n, std::ptr::null()) };
        }
    }
}

/// bytes waiting in the pipe (0 also on error)
fn pipe_available(h: HANDLE) -> u32 {
    let mut avail = 0u32;
    let ok = unsafe { PeekNamedPipe(h, std::ptr::null_mut(), 0, std::ptr::null_mut(), &mut avail, std::ptr::null_mut()) };
    if ok == 0 {
        0
    } else {
        avail
    }
}

/// runs `cmd`, returns how much it printed
pub fn run(cmd: &[String], opts: &Options) -> Result<Stats, String> {
    let (exe, args) = cmd.split_first().ok_or("no command given")?;
    // children write UTF-8 into the pipe (pwsh / cmd follow the console code page)
    let old_cp = unsafe { GetConsoleOutputCP() };
    unsafe { SetConsoleOutputCP(65001) };
    let child = Command::new(exe).args(args).stdout(Stdio::piped()).stderr(Stdio::inherit()).stdin(Stdio::inherit()).spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            unsafe { SetConsoleOutputCP(old_cp) };
            return Err(format!("{exe}: {e}"));
        }
    };
    let mut pipe = child.stdout.take().ok_or("no pipe")?;
    let pipe_h = pipe.as_raw_handle() as HANDLE;

    let t0 = Instant::now();
    let mut buf = vec![0u8; 1 << 20];
    let (mut lines, mut bytes) = (0u64, 0u64);
    let mut out = Out::new();
    let mut file = match &opts.file {
        Some(p) => Some(BufWriter::with_capacity(8 << 20, File::create(p).map_err(|e| format!("{}: {e}", p.display()))?)),
        None => None,
    };
    let mut tail: VecDeque<Vec<u8>> = VecDeque::new();
    let mut partial: Vec<u8> = vec![];
    // screen output waits here until the pipe is empty or 1 MB came together
    let mut pending: Vec<u8> = Vec::with_capacity(2 << 20);
    let mut last_status = Instant::now();
    let show_status = opts.tail.is_some() || opts.file.is_some();

    loop {
        // most programs write line by line: collect while more is waiting in the pipe,
        // write when it is empty (= the program is thinking) or the block is big
        if !pending.is_empty() && (pending.len() >= 1 << 20 || pipe_available(pipe_h) == 0) {
            out.write(&pending);
            pending.clear();
        }
        let n = match pipe.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        };
        let chunk = &buf[..n];
        bytes += n as u64;
        lines += chunk.iter().filter(|&&b| b == b'\n').count() as u64;
        if let Some(f) = file.as_mut() {
            f.write_all(chunk).map_err(|e| e.to_string())?;
        } else if let Some(keep) = opts.tail {
            for piece in chunk.split_inclusive(|&b| b == b'\n') {
                partial.extend_from_slice(piece);
                if piece.ends_with(b"\n") {
                    tail.push_back(std::mem::take(&mut partial));
                    if tail.len() > keep {
                        tail.pop_front();
                    }
                }
            }
        } else {
            pending.extend_from_slice(chunk);
        }
        if show_status && last_status.elapsed() > Duration::from_millis(200) {
            eprint!("\r{lines} lines ...");
            last_status = Instant::now();
        }
    }
    if !pending.is_empty() {
        out.write(&pending);
    }
    let code = child.wait().map(|s| s.code().unwrap_or(1)).unwrap_or(1);
    if let Some(mut f) = file {
        f.flush().map_err(|e| e.to_string())?;
    }
    if opts.tail.is_some() {
        if !partial.is_empty() {
            tail.push_back(partial);
        }
        if show_status {
            eprint!("\r");
        }
        let all: Vec<u8> = tail.into_iter().flatten().collect();
        out.write(&all);
    }
    unsafe { SetConsoleOutputCP(old_cp) };
    Ok(Stats { lines, bytes, secs: t0.elapsed().as_secs_f64(), code })
}

/// pwsh (or Windows PowerShell) running `script` and printing its result as ONE block:
/// pwsh writes every object on its own (300 s for 10 million lines even into a pipe), joined it's ~2 s
pub fn ps_command(script: &str) -> Vec<String> {
    let shell = if crate::win::run_hidden("pwsh.exe", &["-NoProfile", "-Command", "1"]).0 { "pwsh.exe" } else { "powershell.exe" };
    let wrapped = format!(
        "$o = @(& {{ {script} }}); if ($o.Count) {{ if ($o[0] -is [string] -or $o[0] -is [ValueType]) {{ [Console]::Out.Write([string]::Join(\"`n\", $o) + \"`n\") }} else {{ [Console]::Out.Write(($o | Out-String)) }} }}"
    );
    vec![shell.into(), "-NoProfile".into(), "-NonInteractive".into(), "-Command".into(), wrapped]
}

/// PowerShell function for the profile: `fast { 1..10000000 }`
pub const PWSH_SNIPPET: &str = r#"# >>> pcmax >>>
# fast { 1..10000000 }  -> prints everything in one go instead of line by line (pcmax)
function fast {
    param([Parameter(Mandatory, Position = 0)][scriptblock]$Script)
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $out = @(& $Script)
    if ($out.Count -eq 0) { return }
    if ($out[0] -is [string] -or $out[0] -is [ValueType]) {
        $text = [string]::Join("`n", $out) + "`n"
    } else {
        $text = $out | Out-String
    }
    $bytes = [Console]::OutputEncoding.GetBytes($text)
    $stdout = [Console]::OpenStandardOutput()
    $stdout.Write($bytes, 0, $bytes.Length)
    $stdout.Flush()
    [Console]::Error.WriteLine("fast: $($out.Count) items in $([math]::Round($sw.Elapsed.TotalSeconds, 2)) s")
}
# <<< pcmax <<<
"#;
