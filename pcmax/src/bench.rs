// Test loads to check that limits work: burn CPU, allocate RAM, measure a process tree.

use crate::procs;
use crate::topo;
use crate::win::{fmt_bytes, Handle};
use std::hint::black_box;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

/// spin `threads` threads for `secs` seconds, returns the used share of the whole PC in %
pub fn burn(threads: usize, secs: f64) -> f64 {
    let stop = Arc::new(AtomicBool::new(false));
    let me = Handle(unsafe { GetCurrentProcess() });
    let cpu0 = procs::times(&me).map(|t| t.1).unwrap_or(0);
    let t0 = Instant::now();
    let handles: Vec<_> = (0..threads.max(1))
        .map(|i| {
            let stop = stop.clone();
            std::thread::spawn(move || {
                let mut x = i as u64 | 1;
                while !stop.load(Ordering::Relaxed) {
                    for _ in 0..10_000 {
                        x = black_box(x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407));
                    }
                }
                x
            })
        })
        .collect();
    std::thread::sleep(Duration::from_secs_f64(secs));
    stop.store(true, Ordering::Relaxed);
    for h in handles {
        let _ = h.join();
    }
    let wall = t0.elapsed().as_secs_f64();
    let cpu1 = procs::times(&me).map(|t| t.1).unwrap_or(0);
    // pseudo handle, must not be closed
    std::mem::forget(me);
    (cpu1 - cpu0) as f64 / 1e7 / wall / topo::logical_count() as f64 * 100.0
}

/// allocate + touch `size` bytes in 64 MB steps; returns how much worked, the RAM really used
/// at the end (working set, measured while still holding it) and the error if it stopped
pub fn alloc(size: u64, hold_secs: f64, mut progress: impl FnMut(u64)) -> (u64, u64, Result<(), String>) {
    const STEP: usize = 64 << 20;
    let mut blocks: Vec<Vec<u8>> = vec![];
    let mut done = 0u64;
    let mut result = Ok(());
    while done < size {
        let n = STEP.min((size - done) as usize);
        let mut v: Vec<u8> = Vec::new();
        if let Err(e) = v.try_reserve_exact(n) {
            result = Err(format!("allocation failed after {}: {e}", fmt_bytes(done)));
            break;
        }
        // touch every page so it really uses RAM
        v.resize(n, 1);
        for i in (0..n).step_by(4096) {
            v[i] = (i & 0xff) as u8;
        }
        blocks.push(black_box(v));
        done += n as u64;
        progress(done);
    }
    std::thread::sleep(Duration::from_secs_f64(hold_secs));
    black_box(&blocks);
    let ws = procs::memory(procs::own_pid()).map(|m| m.0).unwrap_or(0);
    (done, ws, result)
}

/// CPU share of the whole PC of a process tree, measured over `secs`
pub fn measure(root: u32, secs: f64) -> f64 {
    let pids = procs::tree(&procs::snapshot(), root);
    let c0 = procs::cpu_seconds(&pids);
    let t0 = Instant::now();
    std::thread::sleep(Duration::from_secs_f64(secs));
    let pids = procs::tree(&procs::snapshot(), root);
    let c1 = procs::cpu_seconds(&pids);
    (c1 - c0).max(0.0) / t0.elapsed().as_secs_f64() / topo::logical_count() as f64 * 100.0
}
