// A profile = everything you can set for a process (tree): CPU range, priority, cores, Efficiency Mode,
// RAM, I/O, GPU. Used by the CLI, the GUI, rules and the boost. Stored as "cpu=10-50;prio=high;...".

use crate::cpu::{self, Eco, Prio};
use crate::gpu::{self, GpuPref};
use crate::io::{self, IoPrio};
use crate::job::{self, Job, JobLimits};
use crate::mem::{self, RamMode};
use crate::procs::{self, ProcInfo};
use crate::topo;
use crate::win::{fmt_bytes, open_process, parse_size, Handle};
use windows_sys::Win32::System::Threading::{
    PROCESS_QUERY_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_INFORMATION, PROCESS_SET_QUOTA,
};

#[derive(Clone, PartialEq, Debug)]
pub struct Profile {
    /// guaranteed share of the whole PC in %
    pub cpu_min: u8,
    /// max share of the whole PC in % (100 = no limit)
    pub cpu_max: u8,
    pub prio: Option<Prio>,
    /// "all", "vcache", "freq", "p", "e" or "0-5,12-17" (resolved on the PC it runs on)
    pub cores: Option<String>,
    pub eco: Option<Eco>,
    pub ram: Option<u64>,
    pub ram_mode: RamMode,
    /// 1..=5
    pub ram_prio: Option<u8>,
    pub keep_ram: bool,
    pub io: Option<IoPrio>,
    pub gpu: Option<Prio>,
    pub gpu_pref: Option<GpuPref>,
    /// also all child processes
    pub tree: bool,
}

impl Default for Profile {
    fn default() -> Self {
        Profile {
            cpu_min: 0,
            cpu_max: 100,
            prio: None,
            cores: None,
            eco: None,
            ram: None,
            ram_mode: RamMode::Soft,
            ram_prio: None,
            keep_ram: false,
            io: None,
            gpu: None,
            gpu_pref: None,
            tree: true,
        }
    }
}

impl Profile {
    /// "max boost": high priority, no throttling, high I/O + GPU priority
    pub fn boost() -> Profile {
        Profile { prio: Some(Prio::High), eco: Some(Eco::Off), io: Some(IoPrio::High), gpu: Some(Prio::High), ..Default::default() }
    }

    /// "calm down": efficiency mode, below normal, low I/O
    pub fn calm() -> Profile {
        Profile { prio: Some(Prio::BelowNormal), eco: Some(Eco::On), io: Some(IoPrio::Low), gpu: Some(Prio::BelowNormal), ..Default::default() }
    }

    pub fn is_empty(&self) -> bool {
        let d = Profile::default();
        Profile { tree: d.tree, ..self.clone() } == d
    }

    pub fn encode(&self) -> String {
        let mut p = vec![];
        if self.cpu_min > 0 || self.cpu_max < 100 {
            p.push(format!("cpu={}-{}", self.cpu_min, self.cpu_max));
        }
        if let Some(x) = self.prio {
            p.push(format!("prio={}", x.key()));
        }
        if let Some(x) = &self.cores {
            p.push(format!("cores={x}"));
        }
        if let Some(x) = self.eco {
            p.push(format!("eco={}", x.key()));
        }
        if let Some(x) = self.ram {
            p.push(format!("ram={}M", x >> 20));
            p.push(format!("ram_mode={}", self.ram_mode.key()));
        }
        if let Some(x) = self.ram_prio {
            p.push(format!("ram_prio={x}"));
        }
        if self.keep_ram {
            p.push("keep_ram=1".into());
        }
        if let Some(x) = self.io {
            p.push(format!("io={}", x.key()));
        }
        if let Some(x) = self.gpu {
            p.push(format!("gpu={}", x.key()));
        }
        if let Some(x) = self.gpu_pref {
            p.push(format!("gpu_pref={}", x.key()));
        }
        if !self.tree {
            p.push("tree=0".into());
        }
        p.join(";")
    }

    pub fn decode(s: &str) -> Result<Profile, String> {
        let mut p = Profile::default();
        for part in s.split(';').map(str::trim).filter(|x| !x.is_empty()) {
            let (k, v) = part.split_once('=').ok_or_else(|| format!("'{part}' is not key=value"))?;
            let bad = || format!("bad value in '{part}'");
            match k.trim() {
                "cpu" => (p.cpu_min, p.cpu_max) = parse_cpu(v)?,
                "prio" => p.prio = Some(Prio::parse(v).ok_or_else(bad)?),
                "cores" => p.cores = Some(v.trim().to_string()),
                "eco" => p.eco = Some(Eco::parse(v).ok_or_else(bad)?),
                "ram" => p.ram = Some(parse_size(v).ok_or_else(bad)?),
                "ram_mode" => p.ram_mode = RamMode::parse(v).ok_or_else(bad)?,
                "ram_prio" => p.ram_prio = Some(v.trim().parse::<u8>().map_err(|_| bad())?.clamp(1, 5)),
                "keep_ram" => p.keep_ram = v.trim() == "1",
                "io" => p.io = Some(IoPrio::parse(v).ok_or_else(bad)?),
                "gpu" => p.gpu = Some(Prio::parse(v).ok_or_else(bad)?),
                "gpu_pref" => p.gpu_pref = Some(GpuPref::parse(v).ok_or_else(bad)?),
                "tree" => p.tree = v.trim() != "0",
                _ => return Err(format!("unknown setting '{k}'")),
            }
        }
        Ok(p)
    }

    /// short human text, e.g. "CPU 10-50 % · high · V-Cache · Eco off"
    pub fn summary(&self) -> String {
        let mut p = vec![];
        if self.cpu_min > 0 || self.cpu_max < 100 {
            p.push(format!("CPU {}-{} %", self.cpu_min, self.cpu_max));
        }
        if let Some(x) = self.prio {
            p.push(format!("prio {}", x.key()));
        }
        if let Some(x) = &self.cores {
            p.push(format!("cores {x}"));
        }
        if let Some(x) = self.eco {
            p.push(format!("eco {}", x.key()));
        }
        if let Some(x) = self.ram {
            p.push(format!("RAM {} {}", fmt_bytes(x), self.ram_mode.key()));
        }
        if let Some(x) = self.ram_prio {
            p.push(format!("RAM prio {x}"));
        }
        if self.keep_ram {
            p.push("keep RAM".into());
        }
        if let Some(x) = self.io {
            p.push(format!("I/O {}", x.key()));
        }
        if let Some(x) = self.gpu {
            p.push(format!("GPU {}", x.key()));
        }
        if let Some(x) = self.gpu_pref {
            p.push(format!("GPU pref {}", x.key()));
        }
        if p.is_empty() {
            "-".into()
        } else {
            p.join(" · ")
        }
    }
}

/// "10-50" -> (10, 50), "50" -> (0, 50)
pub fn parse_cpu(s: &str) -> Result<(u8, u8), String> {
    let s = s.trim().trim_end_matches('%');
    let num = |x: &str| x.trim().trim_end_matches('%').parse::<u8>().map_err(|_| format!("'{s}': use e.g. 50 or 10-50"));
    let (a, b) = match s.split_once('-') {
        Some((a, b)) => (num(a)?, num(b)?),
        None => (0, num(s)?),
    };
    if b == 0 || b > 100 || a > b {
        return Err(format!("'{s}': 0 <= min <= max <= 100, max > 0"));
    }
    Ok((a, b))
}

#[derive(Default)]
pub struct Report {
    pub ok: Vec<String>,
    pub warn: Vec<String>,
    pub err: Vec<String>,
    /// keep this alive as long as the limits should be changeable
    pub job: Option<Job>,
    pub pids: Vec<u32>,
}

impl Report {
    pub fn lines(&self) -> Vec<String> {
        let mut out: Vec<String> = self.ok.iter().map(|s| format!("✓ {s}")).collect();
        out.extend(self.warn.iter().map(|s| format!("! {s}")));
        out.extend(self.err.iter().map(|s| format!("✗ {s}")));
        out
    }
    pub fn is_ok(&self) -> bool {
        self.err.is_empty()
    }
}

fn open_rw(pid: u32) -> Option<Handle> {
    open_process(pid, PROCESS_SET_INFORMATION | PROCESS_QUERY_INFORMATION | PROCESS_SET_QUOTA)
        .or_else(|| open_process(pid, PROCESS_SET_INFORMATION | PROCESS_QUERY_LIMITED_INFORMATION))
}

/// collect "x processes: error" instead of one line per process
#[derive(Default)]
struct Tally {
    ok: usize,
    errs: Vec<String>,
}

impl Tally {
    fn add(&mut self, r: Result<(), String>) {
        match r {
            Ok(()) => self.ok += 1,
            Err(e) => self.errs.push(e),
        }
    }
    fn report(self, what: String, rep: &mut Report, total: usize) {
        if self.ok > 0 && self.errs.is_empty() {
            rep.ok.push(if total > 1 { format!("{what} ({} processes)", self.ok) } else { what });
        } else if self.ok > 0 {
            rep.warn.push(format!("{what}: {} ok, {} failed ({})", self.ok, self.errs.len(), self.errs[0]));
        } else if let Some(e) = self.errs.first() {
            rep.err.push(format!("{what}: {e}"));
        }
    }
}

/// apply to `root` (+ children if profile.tree). Realtime only with `allow_realtime`.
pub fn apply(root: u32, p: &Profile, allow_realtime: bool) -> Report {
    let mut rep = Report::default();
    let all = procs::snapshot();
    let Some(me) = all.iter().find(|x| x.pid == root) else {
        rep.err.push(format!("no process with PID {root}"));
        return rep;
    };
    let name = me.name.clone();
    let targets = if p.tree { procs::tree(&all, root) } else { vec![root] };
    rep.pids = targets.clone();
    let n = targets.len();
    if crate::win::is_elevated() {
        crate::win::enable_admin_privileges();
    }

    let mut prio = p.prio;
    if prio == Some(Prio::Realtime) && !allow_realtime {
        rep.warn.push("realtime can freeze the PC - used 'high' instead (confirm realtime explicitly)".into());
        prio = Some(Prio::High);
    }
    let mut gpu_prio = p.gpu;
    if gpu_prio == Some(Prio::Realtime) && !allow_realtime {
        gpu_prio = Some(Prio::High);
    }

    let topo = topo::get();
    let mask = match &p.cores {
        Some(spec) => match topo.mask_for(spec) {
            Ok(m) => Some(m),
            Err(e) => {
                rep.err.push(format!("cores: {e}"));
                None
            }
        },
        None => None,
    };

    // Windows reserves the job minimum only against other rate limited jobs (measured: a normal
    // process with more threads still wins) -> a minimum also lifts the priority to at least "above"
    if p.cpu_min > 0 && prio.is_none_or(|x| x < Prio::AboveNormal) {
        prio = Some(Prio::AboveNormal);
        rep.ok.push("priority above, so the minimum also holds against normal programs".into());
    }

    // ---- job: CPU range, RAM for the whole tree, cores + priority also for later children ----
    let hard_ram = p.ram.filter(|_| p.ram_mode == RamMode::Hard);
    let limits = JobLimits {
        cpu_min: p.cpu_min,
        cpu_max: p.cpu_max,
        affinity: mask.filter(|_| p.tree),
        mem: hard_ram,
        prio: prio.filter(|_| p.tree),
    };
    let mut outside: Vec<u32> = vec![];
    let need_job = limits.cpu_limited() || limits.mem.is_some() || (p.tree && (mask.is_some() || prio.is_some()));
    let existing = job::find_for(&all, root).map(|(_, j)| j);
    if need_job || existing.is_some() {
        let job = match existing {
            Some(j) => Ok(j),
            None => job::open_or_create(root),
        };
        match job {
            Ok(job) => {
                match job::set_limits(&job.handle, &limits) {
                    Ok(w) => rep.warn.extend(w),
                    Err(e) => rep.err.push(e),
                }
                for &pid in &targets {
                    if job::assign(&job.handle, pid).is_err() {
                        outside.push(pid);
                    }
                }
                if !outside.is_empty() && limits.cpu_limited() {
                    rep.warn.push(format!(
                        "{} of {n} processes are in a foreign job (sandbox / launcher) - the CPU range doesn't apply to them",
                        outside.len()
                    ));
                }
                if !job.existed && !outside.contains(&root) {
                    // keeps the job name alive after pcm exits, so `pcm reset` finds it
                    let _ = job::keep_alive_in(&job.handle, root);
                }
                if limits.cpu_limited() {
                    let threads = |pct: u8| pct as f32 / 100.0 * topo.logical as f32;
                    rep.ok.push(format!(
                        "CPU {}-{} % of the PC (≈ {:.1}-{:.1} of {} threads)",
                        p.cpu_min,
                        p.cpu_max,
                        threads(p.cpu_min),
                        threads(p.cpu_max),
                        topo.logical
                    ));
                }
                if let Some(m) = hard_ram {
                    rep.ok.push(format!("RAM hard limit {} for all together", fmt_bytes(m)));
                }
                rep.job = Some(job);
            }
            Err(e) => rep.err.push(e),
        }
    }

    // ---- per process ----
    let (mut t_prio, mut t_aff, mut t_eco, mut t_io, mut t_gpu, mut t_mp, mut t_soft, mut t_keep) =
        (Tally::default(), Tally::default(), Tally::default(), Tally::default(), Tally::default(), Tally::default(), Tally::default(), Tally::default());
    let mut denied = 0;
    for &pid in &targets {
        let Some(h) = open_rw(pid) else {
            denied += 1;
            continue;
        };
        if let Some(x) = prio {
            t_prio.add(cpu::set_priority(&h, x));
        }
        if let Some(m) = mask {
            t_aff.add(cpu::set_affinity(&h, m));
        }
        if let Some(e) = p.eco {
            t_eco.add(cpu::set_eco(&h, e));
        }
        if let Some(x) = p.io {
            t_io.add(io::set(&h, x));
        }
        if let Some(x) = gpu_prio {
            t_gpu.add(gpu::set_priority(&h, x));
        }
        if let Some(x) = p.ram_prio {
            t_mp.add(mem::set_priority(&h, x));
        }
        if let (Some(m), RamMode::Soft) = (p.ram, p.ram_mode) {
            t_soft.add(mem::set_soft_limit(&h, m));
        }
        if p.keep_ram {
            let ws = procs::memory(pid).map(|m| m.0).unwrap_or(0);
            t_keep.add(mem::set_priority(&h, 5).and_then(|_| mem::set_keep(&h, ws)));
        }
    }
    if let Some(x) = prio {
        t_prio.report(format!("priority {}", x.key()), &mut rep, n);
    }
    if let Some(m) = mask {
        t_aff.report(format!("cores {}", topo.describe(m)), &mut rep, n);
    }
    if let Some(e) = p.eco {
        t_eco.report(format!("Efficiency Mode {}", e.key()), &mut rep, n);
    }
    if let Some(x) = p.io {
        t_io.report(format!("I/O priority {}", x.key()), &mut rep, n);
    }
    if let Some(x) = gpu_prio {
        t_gpu.report(format!("GPU priority {}", x.key()), &mut rep, n);
    }
    if let Some(x) = p.ram_prio {
        t_mp.report(format!("RAM priority {x}"), &mut rep, n);
    }
    if let (Some(m), RamMode::Soft) = (p.ram, p.ram_mode) {
        t_soft.report(format!("RAM soft limit {} per process", fmt_bytes(m)), &mut rep, n);
    }
    if p.keep_ram {
        t_keep.report("keep in RAM".into(), &mut rep, n);
    }
    if denied > 0 {
        let hint = if crate::win::is_elevated() { "protected / system processes" } else { "try as admin" };
        rep.warn.push(format!("{denied} of {n} processes: access denied ({hint})"));
    }

    if let Some(g) = p.gpu_pref {
        match procs::exe_path(root) {
            Some(exe) => match gpu::set_pref(&exe, g) {
                Ok(()) => rep.ok.push(format!("GPU preference {} for {name} (from the next start of the app)", g.key())),
                Err(e) => rep.err.push(format!("GPU preference: {e}")),
            },
            None => rep.err.push("GPU preference: exe path not readable".into()),
        }
    }
    if rep.ok.is_empty() && rep.err.is_empty() && rep.warn.is_empty() {
        rep.warn.push("nothing to change".into());
    }
    rep
}

/// everything back to Windows defaults (GPU preference stays, it's a setting)
pub fn reset(root: u32, tree: bool) -> Report {
    let mut rep = Report::default();
    let all = procs::snapshot();
    if !all.iter().any(|x| x.pid == root) {
        rep.err.push(format!("no process with PID {root}"));
        return rep;
    }
    let targets = if tree { procs::tree(&all, root) } else { vec![root] };
    rep.pids = targets.clone();
    if let Some((jroot, j)) = job::find_for(&all, root) {
        match job::clear(&j.handle) {
            Ok(()) => rep.ok.push(format!("job limits off (job of PID {jroot})")),
            Err(e) => rep.err.push(format!("job: {e}")),
        }
    }
    let all_cores = topo::get().all;
    let (mut ok, mut denied) = (0, 0);
    for &pid in &targets {
        let Some(h) = open_rw(pid) else {
            denied += 1;
            continue;
        };
        let _ = cpu::set_priority(&h, Prio::Normal);
        let _ = cpu::set_affinity(&h, all_cores);
        let _ = cpu::set_eco(&h, Eco::Auto);
        let _ = io::set(&h, IoPrio::Normal);
        let _ = gpu::set_priority(&h, Prio::Normal);
        let _ = mem::set_priority(&h, 5);
        let _ = mem::clear_limits(&h);
        ok += 1;
    }
    if ok > 0 {
        rep.ok.push(format!("priority, cores, Efficiency Mode, I/O, GPU, RAM back to normal ({ok} processes)"));
    }
    if denied > 0 {
        rep.warn.push(format!("{denied} processes: access denied"));
    }
    rep
}

/// what a process has right now
#[derive(Debug, Clone)]
pub struct State {
    pub pid: u32,
    pub name: String,
    pub threads: u32,
    pub prio: Option<Prio>,
    pub affinity: Option<u64>,
    pub eco: Option<Eco>,
    pub io: Option<IoPrio>,
    pub gpu: Option<Prio>,
    pub ram_prio: Option<u8>,
    pub working_set: Option<u64>,
    pub private: Option<u64>,
    pub job: Option<(u32, JobLimits)>,
    pub exe: Option<String>,
    pub gpu_pref: Option<GpuPref>,
}

pub fn state(all: &[ProcInfo], pid: u32) -> Option<State> {
    let info = all.iter().find(|p| p.pid == pid)?;
    let h = open_process(pid, PROCESS_QUERY_INFORMATION).or_else(|| open_process(pid, PROCESS_QUERY_LIMITED_INFORMATION));
    let mem = procs::memory(pid);
    let exe = procs::exe_path(pid);
    Some(State {
        pid,
        name: info.name.clone(),
        threads: info.threads,
        prio: h.as_ref().and_then(cpu::get_priority),
        affinity: h.as_ref().and_then(cpu::get_affinity),
        eco: h.as_ref().and_then(cpu::get_eco),
        io: h.as_ref().and_then(io::get),
        // reading the GPU priority needs the right to set it
        gpu: open_process(pid, PROCESS_QUERY_INFORMATION | PROCESS_SET_INFORMATION).as_ref().and_then(gpu::get_priority),
        ram_prio: h.as_ref().and_then(mem::get_priority),
        working_set: mem.map(|m| m.0),
        private: mem.map(|m| m.1),
        job: job::find_for(all, pid).and_then(|(r, j)| job::query(&j.handle).map(|l| (r, l))),
        gpu_pref: exe.as_deref().map(gpu::get_pref),
        exe,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let p = Profile {
            cpu_min: 10,
            cpu_max: 50,
            prio: Some(Prio::High),
            cores: Some("0-5,12-17".into()),
            eco: Some(Eco::Off),
            ram: Some(4 << 30),
            ram_mode: RamMode::Hard,
            ram_prio: Some(5),
            keep_ram: true,
            io: Some(IoPrio::High),
            gpu: Some(Prio::AboveNormal),
            gpu_pref: Some(GpuPref::High),
            tree: false,
        };
        assert_eq!(Profile::decode(&p.encode()).unwrap(), p);
        assert_eq!(Profile::decode("").unwrap(), Profile::default());
        assert!(Profile::default().is_empty());
        assert!(!Profile::boost().is_empty());
        assert!(Profile::decode("cpu=60-50").is_err());
        assert!(Profile::decode("foo=1").is_err());
    }

    #[test]
    fn cpu_ranges() {
        assert_eq!(parse_cpu("10-50").unwrap(), (10, 50));
        assert_eq!(parse_cpu("50%").unwrap(), (0, 50));
        assert!(parse_cpu("0").is_err());
        assert!(parse_cpu("101").is_err());
    }
}
