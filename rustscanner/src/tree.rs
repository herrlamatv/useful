// Turns a filter result into the flat list of rows the UI draws (only what is expanded),
// so even millions of files scroll smoothly: the UI only paints the rows on screen.

use crate::filter::{FilterResult, View};
use crate::model::*;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

/// files shown per folder before a "+ N more" row
pub const FILES_PER_FOLDER: u32 = 1000;
/// stop adding rows here (keeps scrolling precise and memory small)
pub const MAX_ROWS: usize = 300_000;
/// "Expand all" opens folders until about this many rows
const EXPAND_ALL_ROWS: usize = 50_000;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sort {
    Name,
    Size,
    Newest,
    Oldest,
}

impl Sort {
    pub const ALL: [Sort; 4] = [Sort::Name, Sort::Size, Sort::Newest, Sort::Oldest];
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum NodeKey {
    Folder(u32),
    Group(i32),
    GroupFolder(i32, u32),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RowKind {
    /// full_path: show the whole path instead of the name (flat lists)
    Folder { f: u32, key: NodeKey, open: bool, full_path: bool, count: u32, bytes: u64, date: i64 },
    File(u32),
    Group { gi: u32, open: bool },
    More { key: NodeKey, hidden: u32 },
    Hidden { f: u32, shown_anyway: bool },
    /// 0 = hidden folders, 1 = shown anyway
    Section(u8),
    /// 0 = too many rows, 1 = nothing found
    Note(u8),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Row {
    pub depth: u16,
    pub kind: RowKind,
}

impl Row {
    /// stable id for egui (clicks / context menus survive scrolling)
    pub fn key(&self) -> (u8, u64) {
        match self.kind {
            RowKind::Folder { key, .. } => node_id(key),
            RowKind::File(i) => (10, i as u64),
            RowKind::Group { gi, .. } => (11, gi as u64),
            RowKind::More { key, .. } => {
                let (a, b) = node_id(key);
                (20 + a, b)
            }
            RowKind::Hidden { f, .. } => (12, f as u64),
            RowKind::Section(s) => (13, s as u64),
            RowKind::Note(n) => (14, n as u64),
        }
    }
}

fn node_id(k: NodeKey) -> (u8, u64) {
    match k {
        NodeKey::Folder(f) => (1, f as u64),
        NodeKey::Group(g) => (2, g as u32 as u64),
        NodeKey::GroupFolder(g, f) => (3, ((g as u32 as u64) << 32) | f as u64),
    }
}

/// open/closed state per view. Each node has a default (roots open, rest closed);
/// `toggled` holds the ones the user flipped, so new groups after a filter change get their default.
#[derive(Default, Clone, Debug)]
pub struct TreeState {
    pub toggled: HashSet<NodeKey>,
    pub more: HashMap<NodeKey, u32>,
}

impl TreeState {
    pub fn is_open(&self, key: NodeKey, default_open: bool) -> bool {
        default_open ^ self.toggled.contains(&key)
    }
    pub fn toggle(&mut self, key: NodeKey) {
        if !self.toggled.remove(&key) {
            self.toggled.insert(key);
        }
    }
    pub fn set_open(&mut self, key: NodeKey, default_open: bool, open: bool) {
        if open != default_open {
            self.toggled.insert(key);
        } else {
            self.toggled.remove(&key);
        }
    }
    fn limit(&self, key: NodeKey) -> u32 {
        self.more.get(&key).copied().unwrap_or(FILES_PER_FOLDER)
    }
}

pub fn default_open(idx: &Index, key: NodeKey) -> bool {
    match key {
        NodeKey::Folder(f) => idx.is_root(f),
        // Types / Years start as a compact list of groups with their counts
        NodeKey::Group(_) => false,
        NodeKey::GroupFolder(..) => false,
    }
}

struct Ctx<'a> {
    idx: &'a Index,
    res: &'a FilterResult,
    sort: Sort,
    st: &'a TreeState,
    rows: Vec<Row>,
}

impl Ctx<'_> {
    fn full(&self) -> bool {
        self.rows.len() >= MAX_ROWS
    }

    fn date(&self, i: u32) -> i64 {
        let r = &self.idx.files[i as usize];
        match self.res.date_field {
            crate::filter::DateField::Modified => r.mtime,
            crate::filter::DateField::Created => r.ctime,
        }
    }

    fn sort_files(&self, v: &mut [u32]) {
        let files = &self.idx.files;
        match self.sort {
            Sort::Name => {} // already in name order from the scan
            Sort::Size => v.sort_by(|a, b| files[*b as usize].size.cmp(&files[*a as usize].size)),
            Sort::Newest => v.sort_by_key(|&i| std::cmp::Reverse(self.date(i))),
            Sort::Oldest => v.sort_by_key(|&i| oldest_key(self.date(i))),
        }
    }

    fn sort_folders(&self, v: &mut [u32]) {
        let agg = &self.res.agg;
        match self.sort {
            Sort::Name => {}
            Sort::Size => v.sort_by(|a, b| agg[*b as usize].bytes.cmp(&agg[*a as usize].bytes)),
            Sort::Newest => v.sort_by_key(|&f| std::cmp::Reverse(agg[f as usize].newest)),
            Sort::Oldest => v.sort_by_key(|&f| oldest_key(agg[f as usize].oldest)),
        }
    }

    /// files (sorted, capped) with a "+ N more" row
    fn push_files(&mut self, mut files: Vec<u32>, depth: u16, key: NodeKey) {
        self.sort_files(&mut files);
        let limit = self.st.limit(key) as usize;
        for &i in files.iter().take(limit) {
            if self.full() {
                return;
            }
            self.rows.push(Row { depth, kind: RowKind::File(i) });
        }
        if files.len() > limit && !self.full() {
            self.rows.push(Row { depth, kind: RowKind::More { key, hidden: (files.len() - limit) as u32 } });
        }
    }

    fn direct_matches(&self, f: u32) -> Vec<u32> {
        let d = &self.idx.folders[f as usize];
        (d.file_start..d.file_end).filter(|&i| self.res.is_match(i)).collect()
    }

    fn visible_children(&self, f: u32) -> Vec<u32> {
        self.idx
            .children(f)
            .filter(|&c| self.res.hide[c as usize] == 0 && self.res.agg[c as usize].count > 0)
            .collect()
    }

    fn folder_row(&self, f: u32, key: NodeKey, open: bool, full_path: bool, depth: u16, direct_only: bool) -> Row {
        let a = self.res.agg[f as usize];
        let (count, bytes, date) = if direct_only {
            let d = &self.idx.folders[f as usize];
            let mut c = (0u32, 0u64, 0i64);
            for i in d.file_start..d.file_end {
                if self.res.is_match(i) {
                    c.0 += 1;
                    c.1 += self.idx.files[i as usize].size;
                    c.2 = c.2.max(self.date(i));
                }
            }
            c
        } else {
            (a.count, a.bytes, a.newest)
        };
        Row { depth, kind: RowKind::Folder { f, key, open, full_path, count, bytes, date } }
    }

    fn tree(&mut self, f: u32, depth: u16) {
        if self.full() {
            return;
        }
        let key = NodeKey::Folder(f);
        let open = self.st.is_open(key, default_open(self.idx, key));
        self.rows.push(self.folder_row(f, key, open, depth == 0, depth, false));
        if !open {
            return;
        }
        let mut kids = self.visible_children(f);
        self.sort_folders(&mut kids);
        for c in kids {
            self.tree(c, depth + 1);
        }
        let files = self.direct_matches(f);
        self.push_files(files, depth + 1, key);
    }
}

fn oldest_key(t: i64) -> i64 {
    if t > 0 {
        t
    } else {
        i64::MAX // unknown dates last
    }
}

pub fn flatten(idx: &Index, res: &FilterResult, view: View, sort: Sort, st: &TreeState, whitelist: &[u32]) -> Vec<Row> {
    let mut c = Ctx { idx, res, sort, st, rows: Vec::new() };
    match view {
        View::Tree => {
            for r in &idx.roots {
                c.tree(r.folder, 0);
            }
        }
        View::Folders => {
            let mut list: Vec<u32> = (0..idx.folders.len() as u32).filter(|&f| res.direct[f as usize] > 0).collect();
            // sort by what is directly inside
            let rows: Vec<Row> =
                list.iter().map(|&f| c.folder_row(f, NodeKey::Folder(f), false, true, 0, true)).collect();
            if sort != Sort::Name {
                let mut by: Vec<(u32, Row)> = list.iter().copied().zip(rows).collect();
                by.sort_by(|(_, a), (_, b)| cmp_folder_rows(a, b, sort));
                list = by.into_iter().map(|(f, _)| f).collect();
            }
            for f in list {
                if c.full() {
                    break;
                }
                let key = NodeKey::Folder(f);
                let open = st.is_open(key, false);
                c.rows.push(c.folder_row(f, key, open, true, 0, true));
                if open {
                    let files = c.direct_matches(f);
                    c.push_files(files, 1, key);
                }
            }
        }
        View::Types | View::Years => {
            let mut order: Vec<usize> = (0..res.groups.len()).collect();
            if sort == Sort::Size {
                order.sort_by_key(|&g| std::cmp::Reverse(res.groups[g].bytes));
            } else if view == View::Years && sort == Sort::Oldest {
                // oldest year first, unknown stays last
                order.sort_by_key(|&g| {
                    let k = res.groups[g].key;
                    if k == crate::filter::UNKNOWN_YEAR {
                        i32::MAX
                    } else {
                        k
                    }
                });
            }
            for gi in order {
                if c.full() {
                    break;
                }
                let g = &res.groups[gi];
                let gkey = NodeKey::Group(g.key);
                let open = st.is_open(gkey, false);
                c.rows.push(Row { depth: 0, kind: RowKind::Group { gi: gi as u32, open } });
                if !open {
                    continue;
                }
                let mut runs: Vec<usize> = (0..g.runs.len()).collect();
                let run_rows: Vec<Row> = g
                    .runs
                    .iter()
                    .map(|r| {
                        let files = &g.files[r.start as usize..r.end as usize];
                        let bytes = files.iter().map(|&i| idx.files[i as usize].size).sum();
                        let date = files.iter().map(|&i| c.date(i)).max().unwrap_or(0);
                        let key = NodeKey::GroupFolder(g.key, r.folder);
                        Row {
                            depth: 1,
                            kind: RowKind::Folder {
                                f: r.folder,
                                key,
                                open: st.is_open(key, false),
                                full_path: true,
                                count: r.end - r.start,
                                bytes,
                                date,
                            },
                        }
                    })
                    .collect();
                if sort != Sort::Name {
                    runs.sort_by(|&a, &b| cmp_folder_rows(&run_rows[a], &run_rows[b], sort));
                }
                for ri in runs {
                    if c.full() {
                        break;
                    }
                    let row = run_rows[ri];
                    c.rows.push(row);
                    if let RowKind::Folder { key, open: true, .. } = row.kind {
                        let r = g.runs[ri];
                        let files = g.files[r.start as usize..r.end as usize].to_vec();
                        c.push_files(files, 2, key);
                    }
                }
            }
        }
        View::Hidden => {
            c.rows.push(Row { depth: 0, kind: RowKind::Section(0) });
            let mut top = res.hidden_top.clone();
            match sort {
                Sort::Name => {}
                Sort::Size => top.sort_by_key(|&f| std::cmp::Reverse(idx.folders[f as usize].sub_bytes)),
                Sort::Newest => top.sort_by_key(|&f| std::cmp::Reverse(idx.folders[f as usize].mtime)),
                Sort::Oldest => top.sort_by_key(|&f| oldest_key(idx.folders[f as usize].mtime)),
            }
            for f in top {
                if c.full() {
                    break;
                }
                c.rows.push(Row { depth: 0, kind: RowKind::Hidden { f, shown_anyway: false } });
            }
            if !whitelist.is_empty() {
                c.rows.push(Row { depth: 0, kind: RowKind::Section(1) });
                for &f in whitelist {
                    c.rows.push(Row { depth: 0, kind: RowKind::Hidden { f, shown_anyway: true } });
                }
            }
            return c.rows;
        }
    }
    if c.full() {
        c.rows.push(Row { depth: 0, kind: RowKind::Note(0) });
    } else if res.total.count == 0 {
        c.rows.push(Row { depth: 0, kind: RowKind::Note(1) });
    }
    c.rows
}

fn cmp_folder_rows(a: &Row, b: &Row, sort: Sort) -> Ordering {
    let (RowKind::Folder { bytes: ba, date: da, .. }, RowKind::Folder { bytes: bb, date: db, .. }) = (a.kind, b.kind)
    else {
        return Ordering::Equal;
    };
    match sort {
        Sort::Name => Ordering::Equal,
        Sort::Size => bb.cmp(&ba),
        Sort::Newest => db.cmp(&da),
        Sort::Oldest => oldest_key(da).cmp(&oldest_key(db)),
    }
}

/// opens folders breadth-first until about EXPAND_ALL_ROWS rows would be shown
pub fn expand_all(idx: &Index, res: &FilterResult, view: View, st: &mut TreeState) {
    let mut rows = 0usize;
    match view {
        View::Tree => {
            let mut queue: std::collections::VecDeque<u32> = idx.roots.iter().map(|r| r.folder).collect();
            while let Some(f) = queue.pop_front() {
                if res.hide[f as usize] != 0 || res.agg[f as usize].count == 0 {
                    continue;
                }
                let key = NodeKey::Folder(f);
                st.set_open(key, default_open(idx, key), true);
                rows += 1 + res.direct[f as usize].min(FILES_PER_FOLDER) as usize;
                if rows > EXPAND_ALL_ROWS {
                    break;
                }
                queue.extend(idx.children(f));
            }
        }
        View::Folders => {
            for f in 0..idx.folders.len() as u32 {
                if res.direct[f as usize] > 0 {
                    st.set_open(NodeKey::Folder(f), false, true);
                    rows += 1 + res.direct[f as usize].min(FILES_PER_FOLDER) as usize;
                    if rows > EXPAND_ALL_ROWS {
                        break;
                    }
                }
            }
        }
        View::Types | View::Years => {
            'outer: for g in &res.groups {
                st.set_open(NodeKey::Group(g.key), false, true);
                for r in &g.runs {
                    st.set_open(NodeKey::GroupFolder(g.key, r.folder), false, true);
                    rows += 1 + (r.end - r.start).min(FILES_PER_FOLDER) as usize;
                    if rows > EXPAND_ALL_ROWS {
                        break 'outer;
                    }
                }
            }
        }
        View::Hidden => {}
    }
}

/// everything closed again except the roots
pub fn collapse_all(st: &mut TreeState) {
    st.toggled.clear();
}

/// opens everything below one folder (context menu)
pub fn expand_subtree(idx: &Index, res: &FilterResult, f: u32, st: &mut TreeState) {
    let end = idx.folders[f as usize].subtree_end;
    let mut rows = 0usize;
    for c in f..end {
        if res.hide[c as usize] != 0 || res.agg[c as usize].count == 0 {
            continue;
        }
        let key = NodeKey::Folder(c);
        st.set_open(key, default_open(idx, key), true);
        rows += 1 + res.direct[c as usize].min(FILES_PER_FOLDER) as usize;
        if rows > EXPAND_ALL_ROWS {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::tests::filt;
    use crate::filter::FilterParams;
    use crate::scan::tests::{fixture, scan_dir};

    fn count(rows: &[Row], pred: impl Fn(&RowKind) -> bool) -> usize {
        rows.iter().filter(|r| pred(&r.kind)).count()
    }

    #[test]
    fn flatten_views() {
        let fx = fixture("tree");
        let idx = scan_dir(&fx.dir, 4);
        let res = filt(&idx, &FilterParams::default());
        let mut st = TreeState::default();

        // only the root is open: its visible sub folders + its direct files
        let rows = flatten(&idx, &res, View::Tree, Sort::Name, &st, &[]);
        let root_direct = res.direct[0] as usize;
        let kids = idx.children(0).filter(|&c| res.hide[c as usize] == 0 && res.agg[c as usize].count > 0).count();
        assert_eq!(rows.len(), 1 + kids + root_direct);
        assert!(matches!(rows[0].kind, RowKind::Folder { open: true, full_path: true, .. }));

        // expand all -> every visible match is a row
        expand_all(&idx, &res, View::Tree, &mut st);
        let rows = flatten(&idx, &res, View::Tree, Sort::Name, &st, &[]);
        assert_eq!(count(&rows, |k| matches!(k, RowKind::File(_))), res.total.count as usize);
        // collapse again
        collapse_all(&mut st);
        assert_eq!(flatten(&idx, &res, View::Tree, Sort::Name, &st, &[]).len(), 1 + kids + root_direct);

        // sort by size: biggest file of the root first
        let rows = flatten(&idx, &res, View::Tree, Sort::Size, &st, &[]);
        let first_file = rows.iter().find_map(|r| if let RowKind::File(i) = r.kind { Some(i) } else { None }).unwrap();
        let max = (idx.folders[0].file_start..idx.folders[0].file_end)
            .filter(|&i| res.is_match(i))
            .map(|i| idx.files[i as usize].size)
            .max()
            .unwrap();
        assert_eq!(idx.files[first_file as usize].size, max);

        // flat folder list: one row per folder with direct matches
        let rows = flatten(&idx, &res, View::Folders, Sort::Newest, &st, &[]);
        assert_eq!(rows.len(), res.folders_hit as usize);

        // types: only the groups at first, opening one shows its folders
        let res_t = filt(&idx, &FilterParams { view: View::Types, ..Default::default() });
        let rows = flatten(&idx, &res_t, View::Types, Sort::Name, &st, &[]);
        assert_eq!(rows.len(), res_t.groups.len());
        let g0 = &res_t.groups[0];
        st.toggle(NodeKey::Group(g0.key));
        let rows = flatten(&idx, &res_t, View::Types, Sort::Name, &st, &[]);
        assert_eq!(rows.len(), res_t.groups.len() + g0.runs.len());

        // hidden view lists the outermost hidden folders
        let rows = flatten(&idx, &res, View::Hidden, Sort::Name, &st, &[]);
        assert_eq!(count(&rows, |k| matches!(k, RowKind::Hidden { .. })), res.hidden_top.len());
    }

    #[test]
    fn more_rows() {
        let dir = std::env::temp_dir().join(format!("rustscanner_test_more_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for i in 0..1005 {
            std::fs::write(dir.join(format!("f{i}.txt")), b"x").unwrap();
        }
        let idx = scan_dir(&dir, 2);
        let res = filt(&idx, &FilterParams::default());
        let mut st = TreeState::default();
        let rows = flatten(&idx, &res, View::Tree, Sort::Name, &st, &[]);
        assert_eq!(rows.len(), 1 + 1000 + 1);
        assert!(matches!(rows.last().unwrap().kind, RowKind::More { hidden: 5, .. }));
        // natural order: f0, f1, f2 ... f9, f10
        assert!(matches!(rows[11].kind, RowKind::File(i) if idx.file_name(i) == "f10.txt"));
        st.more.insert(NodeKey::Folder(0), 2000);
        assert_eq!(flatten(&idx, &res, View::Tree, Sort::Name, &st, &[]).len(), 1 + 1005);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
