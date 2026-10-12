// Live filtering on top of a finished scan: hide rules, search, types, date, size.
// Runs in its own thread; a newer request makes an older run stop early.

use crate::model::*;
use crate::rules::*;
use crate::win;
use std::collections::{BTreeMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Default)]
pub enum DateField {
    #[default]
    Modified,
    Created,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DateFilter {
    Any,
    /// older than n months
    OlderMonths(u32),
    /// from..=to (local years)
    Years(i32, i32),
}

pub const OLDER_MONTHS: [u32; 5] = [1, 6, 12, 24, 60];
pub const SIZE_STEPS: [u64; 3] = [1 << 20, 100 << 20, 1 << 30];

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Default)]
pub enum View {
    #[default]
    Tree,
    Folders,
    Types,
    Years,
    Hidden,
}

impl View {
    pub const ALL: [View; 5] = [View::Tree, View::Folders, View::Types, View::Years, View::Hidden];
}

#[derive(Clone, PartialEq, Debug)]
pub struct FilterParams {
    pub search: String,
    /// a matching folder name matches everything inside it
    pub match_folders: bool,
    /// 0 = all types
    pub cats: u16,
    /// lowercase, without dot. empty = any
    pub exts: Vec<String>,
    pub git_only: bool,
    pub date_field: DateField,
    pub date: DateFilter,
    /// minimum size, 0 = any
    pub min_size: u64,
    /// enabled rules (R_*)
    pub rules: u16,
    pub html_n: u16,
    /// folder indices of the current index
    pub whitelist: Arc<HashSet<u32>>,
    pub manual: Arc<HashSet<u32>>,
    /// which grouping to build (Types / Years)
    pub view: View,
}

impl Default for FilterParams {
    fn default() -> Self {
        Self {
            search: String::new(),
            match_folders: true,
            cats: 0,
            exts: Vec::new(),
            git_only: false,
            date_field: DateField::Modified,
            date: DateFilter::Any,
            min_size: 0,
            rules: DEFAULT_RULES,
            html_n: 2,
            whitelist: Arc::default(),
            manual: Arc::default(),
            view: View::Tree,
        }
    }
}

/// "html, .css *.js" -> ["html", "css", "js"]  (like New-ExtSet in filescanner.bat)
pub fn parse_exts(text: &str) -> Vec<String> {
    let mut v: Vec<String> = text
        .split(|c: char| c.is_whitespace() || c == ',' || c == ';')
        .map(|t| t.trim_start_matches('*').trim_start_matches('.').to_lowercase())
        .filter(|t| !t.is_empty())
        .collect();
    v.sort();
    v.dedup();
    v
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Agg {
    pub count: u32,
    pub bytes: u64,
    /// by the chosen date field, 0 = none
    pub newest: i64,
    pub oldest: i64,
}

impl Agg {
    fn add_file(&mut self, size: u64, t: i64) {
        self.count += 1;
        self.bytes += size;
        if t > 0 {
            self.newest = self.newest.max(t);
            self.oldest = if self.oldest == 0 { t } else { self.oldest.min(t) };
        }
    }
    fn merge(&mut self, o: &Agg) {
        self.count += o.count;
        self.bytes += o.bytes;
        self.newest = self.newest.max(o.newest);
        if o.oldest > 0 {
            self.oldest = if self.oldest == 0 { o.oldest } else { self.oldest.min(o.oldest) };
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RuleStat {
    /// outermost folders the rule hits
    pub folders: u32,
    pub files: u64,
    pub bytes: u64,
}

/// files of one folder inside a group: group.files[start..end]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Run {
    pub folder: u32,
    pub start: u32,
    pub end: u32,
}

#[derive(Clone, Debug, Default)]
pub struct Group {
    /// Types: category bit index, Years: year (i32::MIN = unknown)
    pub key: i32,
    pub files: Vec<u32>,
    pub runs: Vec<Run>,
    pub bytes: u64,
}

pub const UNKNOWN_YEAR: i32 = i32::MIN;

#[derive(Default, Debug)]
pub struct FilterResult {
    /// per folder: hide reason (R_* bit, | INHERITED when it comes from a parent), 0 = visible
    pub hide: Vec<u16>,
    /// outermost hidden folders
    pub hidden_top: Vec<u32>,
    /// bitset over files
    pub matched: Vec<u64>,
    /// matches in the subtree
    pub agg: Vec<Agg>,
    /// matches directly inside
    pub direct: Vec<u32>,
    /// Types / Years view
    pub groups: Vec<Group>,
    /// per category, with all other filters applied
    pub cat_counts: [u32; CAT_COUNT],
    /// per rule (bit index), independent of the other rules
    pub rule_stats: [RuleStat; 16],
    pub total: Agg,
    pub folders_hit: u32,
    pub hidden_files: u64,
    pub hidden_bytes: u64,
    pub date_field: DateField,
    /// grouping that was built (Types / Years), Tree otherwise
    pub view: View,
}

impl FilterResult {
    pub fn is_match(&self, i: u32) -> bool {
        self.matched[(i / 64) as usize] & (1 << (i % 64)) != 0
    }
}

// ---------------- name matching ----------------

#[derive(Clone, Debug)]
pub struct Matcher {
    kind: MatchKind,
    pat: Vec<u8>,
    ascii: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MatchKind {
    All,
    Contains,
    Wild,
}

impl Matcher {
    /// plain text = name contains it, with * or ? = wildcard for the whole name (like -like in PowerShell)
    pub fn new(query: &str) -> Self {
        let q = query.trim();
        if q.is_empty() {
            return Self { kind: MatchKind::All, pat: Vec::new(), ascii: true };
        }
        let ascii = q.is_ascii();
        let pat = if ascii { q.to_ascii_lowercase() } else { q.to_lowercase() }.into_bytes();
        let kind = if q.contains('*') || q.contains('?') { MatchKind::Wild } else { MatchKind::Contains };
        Self { kind, pat, ascii }
    }

    pub fn is_all(&self) -> bool {
        self.kind == MatchKind::All
    }

    /// `buf` is reused for lowercasing non-ascii names
    pub fn matches(&self, name: &str, buf: &mut String) -> bool {
        let hay: &[u8] = match self.kind {
            MatchKind::All => return true,
            _ if self.ascii => name.as_bytes(),
            _ => {
                buf.clear();
                buf.extend(name.chars().flat_map(char::to_lowercase));
                buf.as_bytes()
            }
        };
        match self.kind {
            MatchKind::Contains => find_fold(hay, &self.pat).is_some(),
            _ => wild(&self.pat, hay),
        }
    }

    /// byte range of the hit inside `name` (for highlighting plain-text searches)
    pub fn find(&self, name: &str) -> Option<std::ops::Range<usize>> {
        if self.kind != MatchKind::Contains || !self.ascii {
            return None;
        }
        find_fold(name.as_bytes(), &self.pat).map(|i| i..i + self.pat.len())
    }
}

/// ascii case-insensitive substring search (the needle is lowercase already)
fn find_fold(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.len() > hay.len() {
        return None;
    }
    let first = needle[0];
    (0..=hay.len() - needle.len()).find(|&i| {
        hay[i].to_ascii_lowercase() == first
            && hay[i + 1..i + needle.len()].iter().zip(&needle[1..]).all(|(a, b)| a.to_ascii_lowercase() == *b)
    })
}

fn utf8_len(b: u8) -> usize {
    match b {
        0..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        _ => 4,
    }
}

/// * = anything, ? = one character. Iterative with one backtrack point, never exponential.
fn wild(p: &[u8], s: &[u8]) -> bool {
    let (mut pi, mut si) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while si < s.len() {
        if pi < p.len() && p[pi] == b'*' {
            star = Some((pi, si));
            pi += 1;
        } else if pi < p.len() && p[pi] == b'?' {
            pi += 1;
            si = (si + utf8_len(s[si])).min(s.len());
        } else if pi < p.len() && p[pi] == s[si].to_ascii_lowercase() {
            pi += 1;
            si += 1;
        } else if let Some((sp, ss)) = star {
            let next = (ss + utf8_len(s[ss])).min(s.len());
            star = Some((sp, next));
            pi = sp + 1;
            si = next;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == b'*' {
        pi += 1;
    }
    pi == p.len()
}

// ---------------- the filter run ----------------

const MONTH: i64 = 2_629_746;

/// None = cancelled because a newer request came in
pub fn run(idx: &Index, p: &FilterParams, now: i64, stale: &dyn Fn() -> bool) -> Option<FilterResult> {
    let nf = idx.folders.len();
    let mut res = FilterResult {
        hide: vec![0; nf],
        matched: vec![0; idx.files.len().div_ceil(64)],
        agg: vec![Agg::default(); nf],
        direct: vec![0; nf],
        date_field: p.date_field,
        view: p.view,
        ..Default::default()
    };

    // what a folder hits by itself (scan-time rules + html count + manual + windows attributes),
    // worked out once because every rule below needs it
    let own: Vec<u16> = idx
        .folders
        .iter()
        .enumerate()
        .map(|(f, d)| {
            if d.parent == NONE || p.whitelist.contains(&(f as u32)) {
                return 0;
            }
            let mut b = d.rules;
            if d.html > p.html_n {
                b |= R_HTML;
            }
            if p.manual.contains(&(f as u32)) {
                b |= R_MANUAL;
            }
            if d.flags & (F_HIDDEN | F_SYSTEM) != 0 {
                b |= R_ATTR;
            }
            b
        })
        .collect();

    // 1) hidden folders (parents come first in pre-order)
    for (f, &own_f) in own.iter().enumerate() {
        let parent = idx.folders[f].parent;
        res.hide[f] = if parent != NONE && res.hide[parent as usize] != 0 {
            (res.hide[parent as usize] & !INHERITED) | INHERITED
        } else {
            let bits = own_f & p.rules;
            if bits != 0 {
                res.hidden_top.push(f as u32);
                res.hidden_files += idx.folders[f].sub_files as u64;
                res.hidden_bytes += idx.folders[f].sub_bytes;
                1 << bits.trailing_zeros()
            } else {
                0
            }
        };
    }
    if stale() {
        return None;
    }

    // 2) per-rule numbers for the toggles, each rule on its own
    for def in RULES.iter() {
        let st = &mut res.rule_stats[bit_index(def.bit)];
        let mut f = 0;
        while f < nf {
            if own[f] & def.bit != 0 {
                st.folders += 1;
                st.files += idx.folders[f].sub_files as u64;
                st.bytes += idx.folders[f].sub_bytes;
                f = idx.folders[f].subtree_end as usize;
            } else {
                f += 1;
            }
        }
    }

    // 3) files
    let matcher = Matcher::new(&p.search);
    let use_search = !matcher.is_all();
    let mut buf = String::new();
    let mut folder_hit = vec![false; nf]; // folder name matches (inherited down)
    let mut in_repo = vec![false; nf];
    let hide_attr_files = p.rules & R_ATTR != 0;
    let (date_min, date_max) = match p.date {
        DateFilter::Any => (i64::MIN, i64::MAX),
        DateFilter::OlderMonths(m) => (1, now - m as i64 * MONTH),
        DateFilter::Years(a, b) => (win::year_start(a.min(b)), win::year_start(a.max(b) + 1) - 1),
    };
    let use_date = p.date != DateFilter::Any;
    let cat_mask = if p.cats == 0 { CAT_ALL } else { p.cats };
    let attr_stat = bit_index(R_ATTR);

    let mut f = 0;
    while f < nf {
        if f % 4096 == 0 && stale() {
            return None;
        }
        let d = idx.folders[f];
        if res.hide[f] != 0 {
            f = d.subtree_end as usize;
            continue;
        }
        let parent = d.parent;
        let up = |v: &Vec<bool>| parent != NONE && v[parent as usize];
        in_repo[f] = up(&in_repo) || d.flags & F_REPO != 0;
        folder_hit[f] = up(&folder_hit)
            || (use_search && p.match_folders && parent != NONE && matcher.matches(idx.folder_name(f as u32), &mut buf));

        if p.git_only && !in_repo[f] {
            f += 1;
            continue;
        }
        for i in d.file_start..d.file_end {
            let r = &idx.files[i as usize];
            if hide_attr_files && r.flags & (FL_HIDDEN | FL_SYSTEM) != 0 {
                res.hidden_files += 1;
                res.hidden_bytes += r.size;
                res.rule_stats[attr_stat].files += 1;
                res.rule_stats[attr_stat].bytes += r.size;
                continue;
            }
            if r.size < p.min_size {
                continue;
            }
            let t = if p.date_field == DateField::Modified { r.mtime } else { r.ctime };
            if use_date && (t < date_min || t > date_max) {
                continue;
            }
            if use_search && !folder_hit[f] && !matcher.matches(idx.file_name(i), &mut buf) {
                continue;
            }
            if !p.exts.is_empty() {
                let ext = idx.file_ext(i);
                if !p.exts.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
                    continue;
                }
            }
            // type counts are taken before the type filter, so every chip shows what it would add
            let mut c = r.cats;
            while c != 0 {
                res.cat_counts[c.trailing_zeros() as usize] += 1;
                c &= c - 1;
            }
            if r.cats & cat_mask == 0 {
                continue;
            }
            res.matched[(i / 64) as usize] |= 1 << (i % 64);
            res.agg[f].add_file(r.size, t);
            res.direct[f] += 1;
        }
        if res.direct[f] > 0 {
            res.folders_hit += 1;
        }
        f += 1;
    }
    if stale() {
        return None;
    }

    // 4) sums up the tree
    for f in (0..nf).rev() {
        let parent = idx.folders[f].parent;
        if parent != NONE {
            let a = res.agg[f];
            res.agg[parent as usize].merge(&a);
        }
    }
    for r in &idx.roots {
        let a = res.agg[r.folder as usize];
        res.total.merge(&a);
    }

    // 5) groups for the Types / Years views. Matches come in folder order -> runs per folder are consecutive.
    match p.view {
        View::Types => {
            let mut groups: Vec<Group> = (0..CAT_COUNT).map(|k| Group { key: k as i32, ..Default::default() }).collect();
            for_each_match(idx, &res, |i| {
                let r = &idx.files[i as usize];
                let g = &mut groups[(r.cats & cat_mask).trailing_zeros() as usize];
                push_run(g, i, r.folder, r.size);
            });
            res.groups = groups.into_iter().filter(|g| !g.files.is_empty()).collect();
        }
        View::Years => {
            let mut groups: BTreeMap<i32, Group> = BTreeMap::new();
            for_each_match(idx, &res, |i| {
                let r = &idx.files[i as usize];
                let t = if p.date_field == DateField::Modified { r.mtime } else { r.ctime };
                let key = win::year_of(t).unwrap_or(UNKNOWN_YEAR);
                let g = groups.entry(key).or_insert_with(|| Group { key, ..Default::default() });
                push_run(g, i, r.folder, r.size);
            });
            // newest year first, unknown last
            res.groups = groups.into_values().rev().collect();
            if res.groups.first().is_some_and(|g| g.key == UNKNOWN_YEAR) {
                let g = res.groups.remove(0);
                res.groups.push(g);
            }
        }
        _ => {}
    }
    Some(res)
}

fn for_each_match(idx: &Index, res: &FilterResult, mut f: impl FnMut(u32)) {
    for (w, &bits) in res.matched.iter().enumerate() {
        let mut b = bits;
        while b != 0 {
            let i = w as u32 * 64 + b.trailing_zeros();
            if (i as usize) < idx.files.len() {
                f(i);
            }
            b &= b - 1;
        }
    }
}

fn push_run(g: &mut Group, i: u32, folder: u32, size: u64) {
    let n = g.files.len() as u32;
    match g.runs.last_mut() {
        Some(r) if r.folder == folder => r.end = n + 1,
        _ => g.runs.push(Run { folder, start: n, end: n + 1 }),
    }
    g.files.push(i);
    g.bytes += size;
}

// ---------------- worker thread ----------------

/// newest finished result with its request number
type Slot = Arc<Mutex<Option<(u64, Arc<FilterResult>)>>>;

pub struct Filterer {
    tx: Sender<(u64, Arc<Index>, FilterParams)>,
    latest: Arc<AtomicU64>,
    slot: Slot,
}

impl Filterer {
    /// `repaint` is called whenever a new result is ready
    pub fn new(repaint: impl Fn() + Send + 'static) -> Self {
        let (tx, rx) = channel::<(u64, Arc<Index>, FilterParams)>();
        let latest = Arc::new(AtomicU64::new(0));
        let slot: Slot = Arc::default();
        let (latest2, slot2) = (latest.clone(), slot.clone());
        std::thread::spawn(move || {
            while let Ok(mut job) = rx.recv() {
                // only the newest request counts
                while let Ok(j) = rx.try_recv() {
                    job = j;
                }
                let (gen, idx, params) = job;
                let stale = || latest2.load(Relaxed) != gen;
                if let Some(res) = run(&idx, &params, unix_now(), &stale) {
                    if !stale() {
                        *slot2.lock().unwrap() = Some((gen, Arc::new(res)));
                        repaint();
                    }
                }
            }
        });
        Self { tx, latest, slot }
    }

    pub fn request(&self, gen: u64, idx: Arc<Index>, params: FilterParams) {
        self.latest.store(gen, Relaxed);
        let _ = self.tx.send((gen, idx, params));
    }

    pub fn take(&self) -> Option<(u64, Arc<FilterResult>)> {
        self.slot.lock().unwrap().take()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::scan::tests::{fixture, scan_dir};

    pub fn filt(idx: &Index, p: &FilterParams) -> FilterResult {
        run(idx, p, unix_now(), &|| false).unwrap()
    }

    pub fn matched_names(idx: &Index, r: &FilterResult) -> Vec<String> {
        let mut v: Vec<String> =
            (0..idx.files.len() as u32).filter(|&i| r.is_match(i)).map(|i| idx.file_name(i).to_string()).collect();
        v.sort();
        v
    }

    fn folder(idx: &Index, base: &std::path::Path, p: &str) -> u32 {
        idx.find_folder(&base.join(p).to_string_lossy()).unwrap()
    }

    #[test]
    fn matcher() {
        let mut b = String::new();
        assert!(Matcher::new("urlaub").matches("Urlaub.JPG", &mut b));
        assert!(Matcher::new("ärger").matches("Ärger.jpeg", &mut b));
        assert!(Matcher::new("*.jp?g").matches("Ärger.jpeg", &mut b));
        assert!(!Matcher::new("*.jp?g").matches("x.jpg", &mut b));
        assert!(Matcher::new("*.jpg").matches("URLAUB.JPG", &mut b));
        assert!(Matcher::new("?rger*").matches("Ärger.jpeg", &mut b));
        assert!(!Matcher::new("abc").matches("ab", &mut b));
        assert!(Matcher::new("a*b*c").matches("axxbyyc", &mut b));
        assert!(!Matcher::new("a*b*c").matches("axxbyy", &mut b));
        assert_eq!(Matcher::new("LAUB").find("Urlaub.jpg"), Some(2..6));
        assert_eq!(parse_exts("html, .CSS *.js js"), vec!["css", "html", "js"]);
    }

    #[test]
    fn rules_hide_and_whitelist() {
        let fx = fixture("filter");
        let idx = scan_dir(&fx.dir, 4);
        let all = filt(&idx, &FilterParams::default());
        let names = matched_names(&idx, &all);
        // root has 3 html but roots are never hidden
        assert!(names.contains(&"a.html".to_string()));
        // Downloads (3 html > 2) is hidden including sub/x.pdf
        assert!(!names.contains(&"x.pdf".to_string()));
        assert!(!names.contains(&"app.exe".to_string()));
        assert!(names.contains(&"keep.txt".to_string()));
        assert!(names.contains(&"main.rs".to_string()));
        assert!(names.contains(&"main.cpp".to_string()));
        for gone in ["x.js", "workspace.xml", "out.o", "mod.py", "m.pyc", "HEAD", "style.css", "hid.txt", "pyvenv.cfg"] {
            assert!(!names.contains(&gone.to_string()), "{gone} should be hidden");
        }
        let dl = folder(&idx, &fx.dir, "Downloads");
        assert_eq!(all.hide[dl as usize], R_HTML);
        let sub = folder(&idx, &fx.dir, "Downloads/sub");
        assert_eq!(all.hide[sub as usize], R_HTML | INHERITED);
        assert!(all.hidden_top.contains(&dl));
        assert!(!all.hidden_top.contains(&sub));
        let st = all.rule_stats[bit_index(R_HTML)];
        assert_eq!((st.folders, st.files), (1, 4));
        assert_eq!(all.rule_stats[bit_index(R_NODE)].files, 2);

        // N = 3 -> visible
        let p3 = FilterParams { html_n: 3, ..Default::default() };
        assert!(matched_names(&idx, &filt(&idx, &p3)).contains(&"x.pdf".to_string()));
        // whitelisted -> visible
        let pw = FilterParams { whitelist: Arc::new([dl].into_iter().collect()), ..Default::default() };
        assert!(matched_names(&idx, &filt(&idx, &pw)).contains(&"x.pdf".to_string()));
        // rule off -> visible
        let pn = FilterParams { rules: DEFAULT_RULES & !R_NODE, ..Default::default() };
        assert!(matched_names(&idx, &filt(&idx, &pn)).contains(&"x.js".to_string()));
        // manual
        let fotos = folder(&idx, &fx.dir, "Fotos");
        let pm = FilterParams { manual: Arc::new([fotos].into_iter().collect()), ..Default::default() };
        let rm = filt(&idx, &pm);
        assert!(!matched_names(&idx, &rm).contains(&"Urlaub.JPG".to_string()));
        assert_eq!(rm.hide[fotos as usize], R_MANUAL);
        // nothing hidden at all
        let none = filt(&idx, &FilterParams { rules: 0, ..Default::default() });
        assert_eq!(none.total.count as usize, idx.files.len());
    }

    #[test]
    fn search_types_date_size() {
        let fx = fixture("search");
        let idx = scan_dir(&fx.dir, 4);
        let names = |p: FilterParams| matched_names(&idx, &filt(&idx, &p));
        assert_eq!(names(FilterParams { search: "URLAUB".into(), ..Default::default() }), vec!["Urlaub.JPG"]);
        assert_eq!(names(FilterParams { search: "*.jp?g".into(), ..Default::default() }), vec!["Ärger.jpeg"]);
        // folder name hit -> everything inside
        assert_eq!(names(FilterParams { search: "2019".into(), ..Default::default() }), vec!["Urlaub.JPG", "Ärger.jpeg"]);
        assert!(names(FilterParams { search: "2019".into(), match_folders: false, ..Default::default() }).is_empty());
        // .js is web and code
        let web = names(FilterParams { cats: CAT_WEB, ..Default::default() });
        let code = names(FilterParams { cats: CAT_CODE, ..Default::default() });
        assert!(web.contains(&"app.js".to_string()) && code.contains(&"app.js".to_string()));
        assert!(web.contains(&"a.html".to_string()) && !code.contains(&"a.html".to_string()));
        assert_eq!(names(FilterParams { exts: vec!["jpeg".into()], ..Default::default() }), vec!["Ärger.jpeg"]);
        assert_eq!(names(FilterParams { min_size: 1 << 20, ..Default::default() }), vec!["Urlaub.JPG"]);
        assert_eq!(names(FilterParams { date: DateFilter::OlderMonths(12), ..Default::default() }), vec!["Urlaub.JPG"]);
        assert_eq!(names(FilterParams { date: DateFilter::Years(2018, 2019), ..Default::default() }), vec!["Urlaub.JPG"]);
        assert!(names(FilterParams { date: DateFilter::Years(2020, 2021), ..Default::default() }).is_empty());
        assert_eq!(
            names(FilterParams { git_only: true, rules: 0, ..Default::default() }),
            vec!["HEAD", "m.pyc", "main.py", "mod.py", "pyvenv.cfg"]
        );
        let r = filt(&idx, &FilterParams { view: View::Years, ..Default::default() });
        assert_eq!(r.groups.last().map(|g| g.key), Some(2019));
        assert_eq!(r.groups.last().unwrap().files.len(), 1);
        let r = filt(&idx, &FilterParams { view: View::Types, ..Default::default() });
        let imgs = r.groups.iter().find(|g| g.key == bit_index(CAT_IMAGES) as i32).unwrap();
        assert_eq!(imgs.files.len(), 2);
        assert_eq!(imgs.runs.len(), 1);
        assert_eq!(r.cat_counts[bit_index(CAT_IMAGES)], 2);
    }
}

#[cfg(test)]
mod bench {
    use super::*;
    use crate::scan::{normalize_roots, scan, Options, Progress};

    /// cargo test --release bench_c_drive -- --ignored --nocapture
    #[test]
    #[ignore]
    fn bench_c_drive() {
        for skip_system in [true, false] {
            let t = std::time::Instant::now();
            let (roots, _) = normalize_roots(&["C:\\".into()]);
            let idx = scan(&Options { roots, skip_system, threads: crate::scan::default_threads() }, &Progress::default());
            let scan_time = t.elapsed();
            let t = std::time::Instant::now();
            let r = run(&idx, &FilterParams::default(), unix_now(), &|| false).unwrap();
            let f1 = t.elapsed();
            let t = std::time::Instant::now();
            let p = FilterParams { search: "bewerbung".into(), view: View::Years, ..Default::default() };
            let r2 = run(&idx, &p, unix_now(), &|| false).unwrap();
            let f2 = t.elapsed();
            let t = std::time::Instant::now();
            let p = FilterParams { search: "*.jp?g".into(), view: View::Types, rules: 0, ..Default::default() };
            let r3 = run(&idx, &p, unix_now(), &|| false).unwrap();
            let f3 = t.elapsed();
            let mem = idx.folders.len() * std::mem::size_of::<Folder>() + idx.files.len() * std::mem::size_of::<FileRec>() + idx.names.len();
            println!(
                "skip_system={skip_system}: {} folders, {} files, {} denied, scan {:?} | filter default {:?} ({} hits, {} hidden top) | search {:?} ({} hits) | wildcard {:?} ({} hits) | index ~{} MB",
                idx.folders.len(), idx.files.len(), idx.denied, scan_time, f1, r.total.count, r.hidden_top.len(), f2, r2.total.count, f3, r3.total.count, mem >> 20
            );
        }
    }
}
