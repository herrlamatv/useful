// What a scan found, packed tight so a whole drive (millions of files) fits in memory:
// folders in DFS pre-order (parent < child, a subtree is folders[f..subtree_end]),
// files grouped per folder in that same order, and every name in one shared string.

use std::cmp::Ordering;
use std::ops::Range;
use std::time::Duration;

pub const NONE: u32 = u32::MAX;

// Folder.flags
pub const F_HIDDEN: u8 = 1;
pub const F_SYSTEM: u8 = 2;
pub const F_CLOUD: u8 = 4;
pub const F_DENIED: u8 = 8;
pub const F_REPO: u8 = 16;
pub const F_ROOT: u8 = 32;
pub const F_SKIPPED: u8 = 64;

// FileRec.flags
pub const FL_HIDDEN: u8 = 1;
pub const FL_SYSTEM: u8 = 2;
pub const FL_CLOUD: u8 = 4;

// windows file attributes
pub const ATTR_HIDDEN: u32 = 0x2;
pub const ATTR_SYSTEM: u32 = 0x4;
pub const ATTR_OFFLINE: u32 = 0x1000;
pub const ATTR_RECALL_ON_OPEN: u32 = 0x4_0000;
pub const ATTR_RECALL_ON_DATA_ACCESS: u32 = 0x40_0000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Folder {
    pub parent: u32,
    pub name_off: u32,
    pub name_len: u16,
    /// exclusive end of this folder's subtree in `Index::folders`
    pub subtree_end: u32,
    /// files directly inside
    pub file_start: u32,
    pub file_end: u32,
    /// all files below (unfiltered)
    pub sub_files: u32,
    pub sub_bytes: u64,
    pub mtime: i64,
    /// .html / .htm files directly inside
    pub html: u16,
    /// hide rules this folder hits by itself (rules::R_*)
    pub rules: u16,
    pub flags: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FileRec {
    pub size: u64,
    /// unix seconds, 0 = unknown
    pub mtime: i64,
    pub ctime: i64,
    pub folder: u32,
    pub name_off: u32,
    pub name_len: u16,
    pub ext_len: u8,
    pub cats: u16,
    pub flags: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Root {
    pub path: String,
    pub folder: u32,
}

#[derive(Default, Debug)]
pub struct Index {
    pub roots: Vec<Root>,
    pub folders: Vec<Folder>,
    pub files: Vec<FileRec>,
    pub names: String,
    /// unix seconds when the scan started
    pub started: i64,
    pub duration: Duration,
    pub denied: u32,
    pub cancelled: bool,
}

impl Index {
    pub fn folder_name(&self, f: u32) -> &str {
        let d = &self.folders[f as usize];
        &self.names[d.name_off as usize..d.name_off as usize + d.name_len as usize]
    }

    pub fn file_name(&self, i: u32) -> &str {
        let r = &self.files[i as usize];
        &self.names[r.name_off as usize..r.name_off as usize + r.name_len as usize]
    }

    /// extension without the dot, as written ("JPG")
    pub fn file_ext(&self, i: u32) -> &str {
        let r = &self.files[i as usize];
        let name = self.file_name(i);
        &name[name.len() - r.ext_len as usize..]
    }

    /// full path, roots store their whole path as name
    pub fn folder_path(&self, f: u32) -> String {
        let mut parts = Vec::new();
        let mut cur = f;
        while cur != NONE {
            parts.push(self.folder_name(cur));
            cur = self.folders[cur as usize].parent;
        }
        let mut s = String::new();
        for p in parts.iter().rev() {
            if !s.is_empty() && !s.ends_with('\\') {
                s.push('\\');
            }
            s.push_str(p);
        }
        s
    }

    pub fn file_path(&self, i: u32) -> String {
        let mut s = self.folder_path(self.files[i as usize].folder);
        if !s.ends_with('\\') {
            s.push('\\');
        }
        s.push_str(self.file_name(i));
        s
    }

    pub fn is_root(&self, f: u32) -> bool {
        self.folders[f as usize].parent == NONE
    }

    pub fn root_of(&self, mut f: u32) -> u32 {
        while self.folders[f as usize].parent != NONE {
            f = self.folders[f as usize].parent;
        }
        f
    }

    /// all files of the subtree (they are contiguous)
    pub fn subtree_files(&self, f: u32) -> Range<usize> {
        let d = &self.folders[f as usize];
        let end = self.folders.get(d.subtree_end as usize).map(|n| n.file_start as usize).unwrap_or(self.files.len());
        d.file_start as usize..end
    }

    /// direct child folders, in name order
    pub fn children(&self, f: u32) -> Children<'_> {
        Children { index: self, next: f + 1, end: self.folders[f as usize].subtree_end }
    }

    /// folder index for a full path (case-insensitive), None if it's not in this scan
    pub fn find_folder(&self, path: &str) -> Option<u32> {
        let path = path.replace('/', "\\");
        let path = path.trim_end_matches('\\');
        for root in &self.roots {
            let rp = root.path.trim_end_matches('\\');
            if path.len() < rp.len() || !path[..rp.len()].eq_ignore_ascii_case(rp) {
                continue;
            }
            let rest = &path[rp.len()..];
            if !rest.is_empty() && !rest.starts_with('\\') {
                continue;
            }
            let mut cur = root.folder;
            for part in rest.split('\\').filter(|p| !p.is_empty()) {
                let low = part.to_lowercase();
                cur = self.children(cur).find(|&c| self.folder_name(c).to_lowercase() == low)?;
            }
            return Some(cur);
        }
        None
    }
}

pub struct Children<'a> {
    index: &'a Index,
    next: u32,
    end: u32,
}

impl Iterator for Children<'_> {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.next >= self.end {
            return None;
        }
        let c = self.next;
        self.next = self.index.folders[c as usize].subtree_end;
        Some(c)
    }
}

// ---------------- categories (same groups as the filescanner presets) ----------------

pub const CAT_WEB: u16 = 1 << 0;
pub const CAT_IMAGES: u16 = 1 << 1;
pub const CAT_VIDEOS: u16 = 1 << 2;
pub const CAT_AUDIO: u16 = 1 << 3;
pub const CAT_DOCS: u16 = 1 << 4;
pub const CAT_CODE: u16 = 1 << 5;
pub const CAT_ARCHIVES: u16 = 1 << 6;
pub const CAT_PROGRAMS: u16 = 1 << 7;
pub const CAT_OTHER: u16 = 1 << 8;
pub const CAT_COUNT: usize = 9;
pub const CAT_ALL: u16 = (1 << CAT_COUNT) - 1;

/// extension (without dot, any case) -> category bits. `.js` is web and code, like in the filescanner presets.
pub fn ext_cats(ext: &str) -> u16 {
    if ext.is_empty() || ext.len() > 10 {
        return CAT_OTHER;
    }
    let mut buf = [0u8; 10];
    for (i, b) in ext.bytes().enumerate() {
        buf[i] = b.to_ascii_lowercase();
    }
    let c = match &buf[..ext.len()] {
        b"html" | b"htm" | b"xhtml" | b"css" | b"scss" | b"sass" | b"less" => CAT_WEB,
        b"js" | b"mjs" | b"cjs" => CAT_WEB | CAT_CODE,
        b"jpg" | b"jpeg" | b"jfif" | b"png" | b"gif" | b"webp" | b"svg" | b"bmp" | b"ico" | b"heic" | b"heif"
        | b"tif" | b"tiff" | b"avif" | b"psd" | b"raw" | b"cr2" | b"nef" | b"dng" | b"arw" => CAT_IMAGES,
        b"mp4" | b"mkv" | b"mov" | b"avi" | b"webm" | b"wmv" | b"flv" | b"m4v" | b"mpg" | b"mpeg" | b"3gp" => {
            CAT_VIDEOS
        }
        b"mp3" | b"wav" | b"flac" | b"aac" | b"ogg" | b"m4a" | b"opus" | b"wma" | b"aiff" | b"mid" | b"midi" => {
            CAT_AUDIO
        }
        b"pdf" | b"doc" | b"docx" | b"xls" | b"xlsx" | b"ppt" | b"pptx" | b"odt" | b"ods" | b"odp" | b"rtf"
        | b"txt" | b"md" | b"csv" | b"epub" => CAT_DOCS,
        b"py" | b"ts" | b"jsx" | b"tsx" | b"java" | b"kt" | b"c" | b"cpp" | b"cc" | b"h" | b"hpp" | b"cs"
        | b"rs" | b"go" | b"php" | b"rb" | b"lua" | b"swift" | b"bat" | b"cmd" | b"ps1" | b"sh" | b"vue"
        | b"svelte" | b"sql" | b"ipynb" => CAT_CODE,
        b"zip" | b"rar" | b"7z" | b"tar" | b"gz" | b"tgz" | b"bz2" | b"xz" | b"zst" | b"iso" | b"cab" => {
            CAT_ARCHIVES
        }
        b"exe" | b"msi" | b"msix" | b"msixbundle" | b"appx" | b"appxbundle" | b"apk" | b"jar" => CAT_PROGRAMS,
        _ => 0,
    };
    if c == 0 {
        CAT_OTHER
    } else {
        c
    }
}

/// like Path::extension: "a.tar.gz" -> "gz", ".gitignore" -> "", "x." -> ""
pub fn ext_of(name: &str) -> &str {
    match name.rfind('.') {
        Some(0) | None => "",
        Some(i) => &name[i + 1..],
    }
}

pub fn is_html_ext(ext: &str) -> bool {
    ext.eq_ignore_ascii_case("html") || ext.eq_ignore_ascii_case("htm")
}

/// lowest set bit as index (0..16)
pub fn bit_index(bits: u16) -> usize {
    bits.trailing_zeros() as usize
}

// ---------------- sorting ----------------

/// Explorer-like order: case-insensitive, numbers by value ("file2" < "file10"),
/// exact bytes as the last tie breaker so the order is always the same.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    natural_cmp_fold(a, b).then_with(|| a.cmp(b))
}

fn natural_cmp_fold(a: &str, b: &str) -> Ordering {
    let mut ia = a.chars().peekable();
    let mut ib = b.chars().peekable();
    loop {
        match (ia.peek().copied(), ib.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ca), Some(cb)) if ca.is_ascii_digit() && cb.is_ascii_digit() => {
                let da = take_digits(&mut ia);
                let db = take_digits(&mut ib);
                let (ta, tb) = (da.trim_start_matches('0'), db.trim_start_matches('0'));
                let o = ta.len().cmp(&tb.len()).then_with(|| ta.cmp(tb)).then_with(|| da.len().cmp(&db.len()));
                if o != Ordering::Equal {
                    return o;
                }
            }
            (Some(ca), Some(cb)) => {
                let o = fold(ca).cmp(&fold(cb));
                if o != Ordering::Equal {
                    return o;
                }
                ia.next();
                ib.next();
            }
        }
    }
}

fn take_digits(it: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut s = String::new();
    while let Some(&c) = it.peek() {
        if !c.is_ascii_digit() {
            break;
        }
        s.push(c);
        it.next();
    }
    s
}

fn fold(c: char) -> char {
    if c.is_ascii() {
        c.to_ascii_lowercase()
    } else {
        c.to_lowercase().next().unwrap_or(c)
    }
}

// ---------------- time ----------------

/// FILETIME (100 ns since 1601) -> unix seconds, 0 = unknown
pub fn filetime_to_unix(ft: u64) -> i64 {
    if ft == 0 {
        return 0;
    }
    let s = (ft / 10_000_000) as i64 - 11_644_473_600;
    s.max(0)
}

pub fn unix_now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_order() {
        let mut v = vec!["file10.txt", "File2.txt", "file1.txt", "Äpfel", "apfel", "b", "file02.txt"];
        v.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(v, vec!["apfel", "b", "file1.txt", "File2.txt", "file02.txt", "file10.txt", "Äpfel"]);
    }

    #[test]
    fn categories() {
        assert_eq!(ext_cats("JS"), CAT_WEB | CAT_CODE);
        assert_eq!(ext_cats("jpg"), CAT_IMAGES);
        assert_eq!(ext_cats(""), CAT_OTHER);
        assert_eq!(ext_cats("verylongextension"), CAT_OTHER);
        assert_eq!(ext_of("a.tar.gz"), "gz");
        assert_eq!(ext_of(".gitignore"), "");
        assert_eq!(ext_of("x."), "");
    }
}
