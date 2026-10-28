// Live values for the monitor: CPU per logical processor (same counter as Task Manager),
// GPU per process + total, RAM. Uses PDH; call sample() about once a second.

use crate::win::{from_wide, wide};
use std::collections::HashMap;
use windows_sys::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW,
    PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_MORE_DATA,
};
use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

/// PDH_FMT_NOCAP100: "% Processor Utility" goes above 100 with boost clock
const NOCAP100: u32 = 0x0000_8000;

#[derive(Clone, Default, Debug)]
pub struct Sample {
    /// per logical processor, 0..100
    pub cores: Vec<f32>,
    pub cpu_total: f32,
    pub gpu_total: f32,
    pub gpu_by_pid: HashMap<u32, f32>,
    pub ram_used: u64,
    pub ram_total: u64,
}

pub struct Monitor {
    query: isize,
    cpu: isize,
    gpu: isize,
    primed: bool,
}

unsafe impl Send for Monitor {}

impl Monitor {
    pub fn new() -> Option<Monitor> {
        let mut q = 0isize;
        unsafe {
            if PdhOpenQueryW(std::ptr::null(), 0, &mut q) != 0 {
                return None;
            }
            let (mut cpu, mut gpu) = (0isize, 0isize);
            PdhAddEnglishCounterW(q, wide(r"\Processor Information(*)\% Processor Utility").as_ptr(), 0, &mut cpu);
            PdhAddEnglishCounterW(q, wide(r"\GPU Engine(*)\Utilization Percentage").as_ptr(), 0, &mut gpu);
            PdhCollectQueryData(q);
            Some(Monitor { query: q, cpu, gpu, primed: false })
        }
    }

    fn values(counter: isize, cap: bool) -> Vec<(String, f64)> {
        if counter == 0 {
            return vec![];
        }
        let fmt = PDH_FMT_DOUBLE | if cap { 0 } else { NOCAP100 };
        let (mut size, mut count) = (0u32, 0u32);
        unsafe {
            let r = PdhGetFormattedCounterArrayW(counter, fmt, &mut size, &mut count, std::ptr::null_mut());
            if r != PDH_MORE_DATA || size == 0 {
                return vec![];
            }
            // the strings live in the same buffer, behind the items
            let mut buf = vec![0u64; size as usize / 8 + 1];
            let items = buf.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
            if PdhGetFormattedCounterArrayW(counter, fmt, &mut size, &mut count, items) != 0 {
                return vec![];
            }
            (0..count as usize)
                .filter_map(|i| {
                    let it = &*items.add(i);
                    if it.FmtValue.CStatus > 1 {
                        return None;
                    }
                    let mut len = 0;
                    while *it.szName.add(len) != 0 {
                        len += 1;
                    }
                    let name = from_wide(std::slice::from_raw_parts(it.szName, len));
                    Some((name, it.FmtValue.Anonymous.doubleValue))
                })
                .collect()
        }
    }

    pub fn sample(&mut self, logical: usize) -> Sample {
        let mut s = Sample { cores: vec![0.0; logical], ..Default::default() };
        unsafe { PdhCollectQueryData(self.query) };
        if self.primed {
            // "0,5" = group 0, processor 5; "0,_Total" / "_Total" are sums
            for (name, v) in Monitor::values(self.cpu, false) {
                if name == "_Total" {
                    s.cpu_total = v.min(100.0) as f32;
                    continue;
                }
                if let Some((g, n)) = name.split_once(',') {
                    if let (Ok(0), Ok(n)) = (g.parse::<u32>(), n.parse::<usize>()) {
                        if n < logical {
                            s.cores[n] = v.min(100.0) as f32;
                        }
                    }
                }
            }
            // "pid_1234_luid_0x0_0xD1F1_phys_0_eng_0_engtype_3D": per process the busiest engine,
            // in total the busiest engine of all (like Task Manager)
            let mut per_engine: HashMap<String, f64> = HashMap::new();
            let mut per_pid_engine: HashMap<(u32, String), f64> = HashMap::new();
            for (name, v) in Monitor::values(self.gpu, false) {
                let Some(rest) = name.strip_prefix("pid_") else { continue };
                let Some((pid, rest)) = rest.split_once('_') else { continue };
                let Ok(pid) = pid.parse::<u32>() else { continue };
                // engine = everything after the pid without the process part
                let engine = rest.to_string();
                *per_engine.entry(engine.clone()).or_default() += v;
                *per_pid_engine.entry((pid, engine)).or_default() += v;
            }
            s.gpu_total = per_engine.values().cloned().fold(0.0, f64::max).min(100.0) as f32;
            for ((pid, _), v) in per_pid_engine {
                let e = s.gpu_by_pid.entry(pid).or_default();
                *e = e.max(v.min(100.0) as f32);
            }
        }
        self.primed = true;
        let mut m: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
        m.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        if unsafe { GlobalMemoryStatusEx(&mut m) } != 0 {
            s.ram_total = m.ullTotalPhys;
            s.ram_used = m.ullTotalPhys - m.ullAvailPhys;
        }
        s
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        unsafe { PdhCloseQuery(self.query) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples() {
        let mut m = Monitor::new().unwrap();
        let n = crate::topo::logical_count() as usize;
        m.sample(n);
        std::thread::sleep(std::time::Duration::from_millis(300));
        let s = m.sample(n);
        assert_eq!(s.cores.len(), n);
        assert!(s.ram_total > 0 && s.ram_used <= s.ram_total);
        assert!(s.cpu_total >= 0.0);
    }
}
