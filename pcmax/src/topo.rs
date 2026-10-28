// CPU topology: cores, SMT, L3 groups. Finds the 3D V-Cache CCD on X3D Ryzens (bigger L3)
// and P- / E-cores on Intel hybrids. Only processor group 0 (up to 64 threads) - plenty for desktops.

use std::sync::OnceLock;
use windows_sys::Win32::System::SystemInformation::{
    GetLogicalProcessorInformationEx, RelationAll, RelationCache, RelationProcessorCore,
    SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
};

#[derive(Clone, Debug)]
pub struct Core {
    /// logical processors of this core (2 bits with SMT)
    pub mask: u64,
    pub efficiency_class: u8,
}

#[derive(Clone, Debug)]
pub struct L3 {
    pub mask: u64,
    pub size: u64,
}

#[derive(Clone, Debug, Default)]
pub struct Topo {
    pub logical: u32,
    pub all: u64,
    pub cores: Vec<Core>,
    pub l3: Vec<L3>,
    /// CCD with 3D V-Cache (X3D with two CCDs)
    pub vcache: Option<u64>,
    /// the other CCD(s), clocks higher
    pub freq: Option<u64>,
    /// Intel hybrid: performance / efficient cores
    pub pcores: Option<u64>,
    pub ecores: Option<u64>,
}

impl Topo {
    pub fn from_parts(cores: Vec<Core>, mut l3: Vec<L3>) -> Topo {
        let all = cores.iter().fold(0u64, |a, c| a | c.mask);
        l3.sort_by_key(|c| c.mask.trailing_zeros());
        let (mut vcache, mut freq) = (None, None);
        if l3.len() >= 2 {
            let max = l3.iter().map(|c| c.size).max().unwrap_or(0);
            let min = l3.iter().map(|c| c.size).min().unwrap_or(0);
            // X3D: one CCD has 96 MB, the other 32 MB
            if max >= min * 2 {
                vcache = Some(l3.iter().filter(|c| c.size == max).fold(0, |a, c| a | c.mask));
                freq = Some(l3.iter().filter(|c| c.size != max).fold(0, |a, c| a | c.mask));
            }
        }
        let (mut pcores, mut ecores) = (None, None);
        let max_class = cores.iter().map(|c| c.efficiency_class).max().unwrap_or(0);
        if cores.iter().any(|c| c.efficiency_class != max_class) {
            pcores = Some(cores.iter().filter(|c| c.efficiency_class == max_class).fold(0, |a, c| a | c.mask));
            ecores = Some(cores.iter().filter(|c| c.efficiency_class != max_class).fold(0, |a, c| a | c.mask));
        }
        Topo { logical: all.count_ones(), all, cores, l3, vcache, freq, pcores, ecores }
    }

    pub fn detect() -> Topo {
        let mut len = 0u32;
        unsafe { GetLogicalProcessorInformationEx(RelationAll, std::ptr::null_mut(), &mut len) };
        let mut buf = vec![0u64; len as usize / 8 + 1];
        let ok = unsafe {
            GetLogicalProcessorInformationEx(RelationAll, buf.as_mut_ptr() as *mut SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX, &mut len)
        };
        if ok == 0 {
            let n = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1).min(64) as u32;
            let all = if n == 64 { u64::MAX } else { (1u64 << n) - 1 };
            return Topo { logical: n, all, ..Default::default() };
        }
        let (mut cores, mut l3) = (vec![], vec![]);
        let base = buf.as_ptr() as *const u8;
        let mut off = 0usize;
        while off < len as usize {
            let info = unsafe { &*(base.add(off) as *const SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX) };
            if info.Relationship == RelationProcessorCore {
                let p = unsafe { &info.Anonymous.Processor };
                let g = p.GroupMask[0];
                if g.Group == 0 {
                    cores.push(Core { mask: g.Mask as u64, efficiency_class: p.EfficiencyClass });
                }
            } else if info.Relationship == RelationCache {
                let c = unsafe { &info.Anonymous.Cache };
                let g = unsafe { c.Anonymous.GroupMask };
                if c.Level == 3 && g.Group == 0 {
                    l3.push(L3 { mask: g.Mask as u64, size: c.CacheSize as u64 });
                }
            }
            if info.Size == 0 {
                break;
            }
            off += info.Size as usize;
        }
        Topo::from_parts(cores, l3)
    }

    /// "vcache", "freq", "p", "e", "all" or a list like "0-5,12-17"
    pub fn mask_for(&self, spec: &str) -> Result<u64, String> {
        let s = spec.trim().to_ascii_lowercase();
        let named = |m: Option<u64>, what: &str| m.ok_or_else(|| format!("this CPU has no {what}"));
        match s.as_str() {
            "" | "all" | "alle" => Ok(self.all),
            "vcache" | "v-cache" | "cache" | "x3d" => named(self.vcache, "3D V-Cache CCD"),
            "freq" | "takt" | "frequency" => named(self.freq, "second CCD without V-Cache"),
            "p" | "pcores" | "p-cores" => named(self.pcores, "P-/E-core split"),
            "e" | "ecores" | "e-cores" => named(self.ecores, "P-/E-core split"),
            _ => {
                let m = parse_list(&s)?;
                let m = m & self.all;
                if m == 0 {
                    Err(format!("'{spec}': no existing processor in it (0-{})", self.logical.saturating_sub(1)))
                } else {
                    Ok(m)
                }
            }
        }
    }

    /// which named group a mask is (for display)
    pub fn describe(&self, mask: u64) -> String {
        if mask == self.all {
            return "all".into();
        }
        for (m, n) in [(self.vcache, "vcache"), (self.freq, "freq"), (self.pcores, "p"), (self.ecores, "e")] {
            if m == Some(mask) {
                return format!("{n} ({})", format_list(mask));
            }
        }
        format_list(mask)
    }

    pub fn is_x3d(&self) -> bool {
        self.vcache.is_some()
    }
}

/// detected once
pub fn get() -> &'static Topo {
    static T: OnceLock<Topo> = OnceLock::new();
    T.get_or_init(Topo::detect)
}

pub fn logical_count() -> u32 {
    get().logical.max(1)
}

/// "0-5,12-17" -> mask
pub fn parse_list(s: &str) -> Result<u64, String> {
    let mut m = 0u64;
    for part in s.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let (a, b) = match part.split_once('-') {
            Some((a, b)) => (a.trim(), b.trim()),
            None => (part, part),
        };
        let a: u32 = a.parse().map_err(|_| format!("'{part}' is not a number / range"))?;
        let b: u32 = b.parse().map_err(|_| format!("'{part}' is not a number / range"))?;
        if a > b || b > 63 {
            return Err(format!("'{part}': use 0-63, small to big"));
        }
        for i in a..=b {
            m |= 1 << i;
        }
    }
    if m == 0 {
        Err("empty core list".into())
    } else {
        Ok(m)
    }
}

/// mask -> "0-5,12-17"
pub fn format_list(mask: u64) -> String {
    let mut out = vec![];
    let mut i = 0;
    while i < 64 {
        if mask & (1 << i) != 0 {
            let start = i;
            while i + 1 < 64 && mask & (1 << (i + 1)) != 0 {
                i += 1;
            }
            out.push(if start == i { start.to_string() } else { format!("{start}-{i}") });
        }
        i += 1;
    }
    out.join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn x3d() -> Topo {
        // 9900X3D like: 12 cores with SMT, CCD0 = logical 0-11 with 96 MB, CCD1 = 12-23 with 32 MB
        let cores = (0..12).map(|i| Core { mask: 0b11 << (i * 2), efficiency_class: 0 }).collect();
        let l3 = vec![L3 { mask: 0xfff000, size: 32 << 20 }, L3 { mask: 0xfff, size: 96 << 20 }];
        Topo::from_parts(cores, l3)
    }

    #[test]
    fn x3d_split() {
        let t = x3d();
        assert_eq!(t.logical, 24);
        assert_eq!(t.vcache, Some(0xfff));
        assert_eq!(t.freq, Some(0xfff000));
        assert_eq!(t.pcores, None);
        assert_eq!(t.mask_for("vcache").unwrap(), 0xfff);
        assert_eq!(t.mask_for("0-1,12").unwrap(), 0b11 | 1 << 12);
        assert!(t.mask_for("30").is_err());
        assert_eq!(t.describe(0xfff), "vcache (0-11)");
    }

    #[test]
    fn same_l3_is_no_x3d() {
        let cores = (0..16).map(|i| Core { mask: 0b11 << (i * 2), efficiency_class: 0 }).collect();
        let l3 = vec![L3 { mask: 0xffff, size: 32 << 20 }, L3 { mask: 0xffff0000, size: 32 << 20 }];
        assert!(!Topo::from_parts(cores, l3).is_x3d());
    }

    #[test]
    fn hybrid() {
        let mut cores: Vec<Core> = (0..8).map(|i| Core { mask: 0b11 << (i * 2), efficiency_class: 1 }).collect();
        cores.extend((16..32).map(|i| Core { mask: 1 << i, efficiency_class: 0 }));
        let t = Topo::from_parts(cores, vec![]);
        assert_eq!(t.pcores, Some(0xffff));
        assert_eq!(t.ecores, Some(0xffff0000));
    }

    #[test]
    fn lists() {
        assert_eq!(format_list(0xfff | 0xfff000), "0-23");
        assert_eq!(format_list(0b1011), "0-1,3");
        assert_eq!(parse_list("0-1,3").unwrap(), 0b1011);
        assert!(parse_list("5-2").is_err());
    }

    #[test]
    fn detect_real() {
        let t = Topo::detect();
        assert!(t.logical >= 1);
        assert_eq!(t.all.count_ones(), t.logical);
    }
}
