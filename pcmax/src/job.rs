// Job Objects: the only Windows way to give a process (+ everything it starts later) a CPU range
// like "10-50 %", a RAM limit for all of it together and fixed cores.
//
// The job is named "Local\pcmax-<pid of the root>". A handle to it is also copied into the root
// process itself, so the name stays alive after `pcm` exits and a later `pcm reset` finds it again.

use crate::cpu::Prio;
use crate::win::{last_error, open_process, wide, Handle};
use std::ffi::c_void;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{
    DuplicateHandle, BOOL, DUPLICATE_SAME_ACCESS, ERROR_ALREADY_EXISTS, ERROR_INVALID_PARAMETER, HANDLE,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, IsProcessInJob, JobObjectBasicAccountingInformation,
    JobObjectCpuRateControlInformation, JobObjectExtendedLimitInformation, OpenJobObjectW, QueryInformationJobObject,
    SetInformationJobObject, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION, JOBOBJECT_CPU_RATE_CONTROL_INFORMATION,
    JOBOBJECT_CPU_RATE_CONTROL_INFORMATION_0, JOBOBJECT_CPU_RATE_CONTROL_INFORMATION_0_0,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_CPU_RATE_CONTROL_ENABLE, JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP,
    JOB_OBJECT_CPU_RATE_CONTROL_MIN_MAX_RATE, JOB_OBJECT_LIMIT_AFFINITY, JOB_OBJECT_LIMIT_JOB_MEMORY,
    JOB_OBJECT_LIMIT_PRIORITY_CLASS,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, PROCESS_DUP_HANDLE, PROCESS_SET_QUOTA, PROCESS_TERMINATE};

/// JOB_OBJECT_ALL_ACCESS
const JOB_ALL: u32 = 0x1F_003F;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct JobLimits {
    /// guaranteed share of the whole PC in % (0 = none)
    pub cpu_min: u8,
    /// at most this share of the whole PC in % (100 = no limit)
    pub cpu_max: u8,
    pub affinity: Option<u64>,
    /// all processes of the job together
    pub mem: Option<u64>,
    /// priority for all processes, also the ones started later (needs admin)
    pub prio: Option<Prio>,
}

impl JobLimits {
    pub fn none() -> JobLimits {
        JobLimits { cpu_min: 0, cpu_max: 100, affinity: None, mem: None, prio: None }
    }
    pub fn cpu_limited(&self) -> bool {
        self.cpu_min > 0 || self.cpu_max < 100
    }
    pub fn is_empty(&self) -> bool {
        !self.cpu_limited() && self.affinity.is_none() && self.mem.is_none() && self.prio.is_none()
    }
}

pub fn job_name(root: u32) -> String {
    format!("Local\\pcmax-{root}")
}

pub struct Job {
    pub handle: Handle,
    pub name: String,
    /// existed already (someone else holds it alive)
    pub existed: bool,
}

pub fn open(root: u32) -> Option<Job> {
    let name = job_name(root);
    let h = Handle::new(unsafe { OpenJobObjectW(JOB_ALL, 0, wide(&name).as_ptr()) })?;
    Some(Job { handle: h, name, existed: true })
}

pub fn open_or_create(root: u32) -> Result<Job, String> {
    let name = job_name(root);
    let h = unsafe { CreateJobObjectW(null(), wide(&name).as_ptr()) };
    let existed = crate::win::last_error_code() == ERROR_ALREADY_EXISTS;
    let handle = Handle::new(h).ok_or_else(|| format!("CreateJobObject: {}", last_error()))?;
    Ok(Job { handle, name, existed })
}

fn set_cpu(job: &Handle, min: u8, max: u8) -> Result<(), String> {
    let (min, max) = (min.min(100) as u16, max.clamp(1, 100) as u16);
    let info = if min == 0 && max >= 100 {
        // ControlFlags 0 = rate control off
        JOBOBJECT_CPU_RATE_CONTROL_INFORMATION { ControlFlags: 0, Anonymous: JOBOBJECT_CPU_RATE_CONTROL_INFORMATION_0 { CpuRate: 0 } }
    } else if min == 0 {
        JOBOBJECT_CPU_RATE_CONTROL_INFORMATION {
            ControlFlags: JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP,
            Anonymous: JOBOBJECT_CPU_RATE_CONTROL_INFORMATION_0 { CpuRate: max as u32 * 100 },
        }
    } else {
        JOBOBJECT_CPU_RATE_CONTROL_INFORMATION {
            ControlFlags: JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_MIN_MAX_RATE,
            Anonymous: JOBOBJECT_CPU_RATE_CONTROL_INFORMATION_0 {
                Anonymous: JOBOBJECT_CPU_RATE_CONTROL_INFORMATION_0_0 { MinRate: min.min(max) * 100, MaxRate: max * 100 },
            },
        }
    };
    let ok = unsafe {
        SetInformationJobObject(
            job.raw(),
            JobObjectCpuRateControlInformation,
            &info as *const _ as *const c_void,
            std::mem::size_of::<JOBOBJECT_CPU_RATE_CONTROL_INFORMATION>() as u32,
        )
    };
    // turning off a rate control that was never on gives ERROR_INVALID_PARAMETER - nothing to do then
    if ok == 0 && info.ControlFlags == 0 && crate::win::last_error_code() == ERROR_INVALID_PARAMETER {
        return Ok(());
    }
    if ok == 0 {
        Err(format!("CPU limit: {}", last_error()))
    } else {
        Ok(())
    }
}

fn set_extended(job: &Handle, l: &JobLimits, with_prio: bool) -> Result<(), String> {
    let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
    let b = &mut info.BasicLimitInformation;
    if let Some(a) = l.affinity {
        b.LimitFlags |= JOB_OBJECT_LIMIT_AFFINITY;
        b.Affinity = a as usize;
    }
    if let Some(m) = l.mem {
        b.LimitFlags |= JOB_OBJECT_LIMIT_JOB_MEMORY;
        info.JobMemoryLimit = m as usize;
    }
    if let (Some(p), true) = (l.prio, with_prio) {
        info.BasicLimitInformation.LimitFlags |= JOB_OBJECT_LIMIT_PRIORITY_CLASS;
        info.BasicLimitInformation.PriorityClass = p.class();
    }
    let ok = unsafe {
        SetInformationJobObject(
            job.raw(),
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const c_void,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    };
    if ok == 0 {
        Err(last_error())
    } else {
        Ok(())
    }
}

/// returns warnings (things that only partly worked)
pub fn set_limits(job: &Handle, l: &JobLimits) -> Result<Vec<String>, String> {
    let mut warn = vec![];
    set_cpu(job, l.cpu_min, l.cpu_max)?;
    if set_extended(job, l, true).is_err() {
        // priority for the whole job needs SeIncreaseBasePriorityPrivilege (admin)
        set_extended(job, l, false).map_err(|e| format!("RAM / core limit: {e}"))?;
        if l.prio.is_some() {
            warn.push("priority only for the running processes (new child processes need admin)".into());
        }
    }
    Ok(warn)
}

pub fn contains(job: &Handle, pid: u32) -> bool {
    let Some(h) = open_process(pid, windows_sys::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION) else {
        return false;
    };
    let mut r: BOOL = 0;
    unsafe { IsProcessInJob(h.raw(), job.raw(), &mut r) != 0 && r != 0 }
}

pub fn assign(job: &Handle, pid: u32) -> Result<(), String> {
    if contains(job, pid) {
        return Ok(());
    }
    let h = open_process(pid, PROCESS_SET_QUOTA | PROCESS_TERMINATE).ok_or_else(last_error)?;
    if unsafe { AssignProcessToJobObject(job.raw(), h.raw()) } == 0 {
        Err(last_error())
    } else {
        Ok(())
    }
}

/// copy the job handle into `pid`, keeps the job name alive as long as that process lives
pub fn keep_alive_in(job: &Handle, pid: u32) -> Result<(), String> {
    let target = open_process(pid, PROCESS_DUP_HANDLE).ok_or_else(last_error)?;
    let mut out: HANDLE = null_mut();
    let ok = unsafe { DuplicateHandle(GetCurrentProcess(), job.raw(), target.raw(), &mut out, 0, 0, DUPLICATE_SAME_ACCESS) };
    if ok == 0 {
        Err(last_error())
    } else {
        Ok(())
    }
}

/// all limits off (processes can't leave a job, but an empty job does nothing)
pub fn clear(job: &Handle) -> Result<(), String> {
    set_cpu(job, 0, 100)?;
    set_extended(job, &JobLimits::none(), false)
}

pub fn query(job: &Handle) -> Option<JobLimits> {
    let mut l = JobLimits::none();
    let mut cpu: JOBOBJECT_CPU_RATE_CONTROL_INFORMATION = unsafe { std::mem::zeroed() };
    let ok = unsafe {
        QueryInformationJobObject(
            job.raw(),
            JobObjectCpuRateControlInformation,
            &mut cpu as *mut _ as *mut c_void,
            std::mem::size_of::<JOBOBJECT_CPU_RATE_CONTROL_INFORMATION>() as u32,
            null_mut(),
        )
    };
    if ok != 0 && cpu.ControlFlags & JOB_OBJECT_CPU_RATE_CONTROL_ENABLE != 0 {
        unsafe {
            if cpu.ControlFlags & JOB_OBJECT_CPU_RATE_CONTROL_MIN_MAX_RATE != 0 {
                l.cpu_min = (cpu.Anonymous.Anonymous.MinRate / 100) as u8;
                l.cpu_max = (cpu.Anonymous.Anonymous.MaxRate / 100) as u8;
            } else {
                l.cpu_max = (cpu.Anonymous.CpuRate / 100) as u8;
            }
        }
    }
    let mut ext: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
    let ok = unsafe {
        QueryInformationJobObject(
            job.raw(),
            JobObjectExtendedLimitInformation,
            &mut ext as *mut _ as *mut c_void,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            null_mut(),
        )
    };
    if ok != 0 {
        let f = ext.BasicLimitInformation.LimitFlags;
        if f & JOB_OBJECT_LIMIT_AFFINITY != 0 {
            l.affinity = Some(ext.BasicLimitInformation.Affinity as u64);
        }
        if f & JOB_OBJECT_LIMIT_JOB_MEMORY != 0 {
            l.mem = Some(ext.JobMemoryLimit as u64);
        }
        if f & JOB_OBJECT_LIMIT_PRIORITY_CLASS != 0 {
            l.prio = Prio::from_class(ext.BasicLimitInformation.PriorityClass);
        }
    }
    Some(l)
}

/// CPU seconds of everything that ever ran in the job + number of active processes
pub fn accounting(job: &Handle) -> Option<(f64, u32)> {
    let mut a: JOBOBJECT_BASIC_ACCOUNTING_INFORMATION = unsafe { std::mem::zeroed() };
    let ok = unsafe {
        QueryInformationJobObject(
            job.raw(),
            JobObjectBasicAccountingInformation,
            &mut a as *mut _ as *mut c_void,
            std::mem::size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
            null_mut(),
        )
    };
    (ok != 0).then(|| ((a.TotalUserTime + a.TotalKernelTime) as f64 / 1e7, a.ActiveProcesses))
}

/// the job a process got from pcmax: its own one or the one of a parent
pub fn find_for(all: &[crate::procs::ProcInfo], pid: u32) -> Option<(u32, Job)> {
    let mut cur = pid;
    for _ in 0..16 {
        if let Some(j) = open(cur) {
            if contains(&j.handle, pid) {
                return Some((cur, j));
            }
        }
        cur = all.iter().find(|p| p.pid == cur)?.ppid;
        if cur == 0 {
            return None;
        }
    }
    None
}
