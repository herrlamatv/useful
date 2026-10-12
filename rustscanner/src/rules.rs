// Hide rules: tool / IDE junk, Windows folders, folders full of .html ...
// Folder-name rules are detected while scanning (the parent's listing is at hand then),
// the html count and manual hides are checked when filtering, so they can change without a new scan.

use crate::model::FileRec;

// bit order = priority when a folder hits several rules (the lowest bit is shown as reason)
pub const R_SYSTEM: u16 = 1 << 0;
pub const R_MANUAL: u16 = 1 << 1;
pub const R_VCS: u16 = 1 << 2;
pub const R_JETBRAINS: u16 = 1 << 3;
pub const R_VS: u16 = 1 << 4;
pub const R_NODE: u16 = 1 << 5;
pub const R_PYTHON: u16 = 1 << 6;
pub const R_BUILD: u16 = 1 << 7;
pub const R_HTML: u16 = 1 << 8;
pub const R_SAVED: u16 = 1 << 9;
pub const R_ATTR: u16 = 1 << 10;
pub const R_CACHE: u16 = 1 << 11;
pub const RULE_COUNT: usize = 12;
/// set in FilterResult::hide when the reason comes from a parent folder
pub const INHERITED: u16 = 1 << 15;

pub const DEFAULT_RULES: u16 = (1 << RULE_COUNT) - 1;

pub struct RuleDef {
    pub bit: u16,
    /// name in settings.txt
    pub key: &'static str,
    /// what it matches, same in every language
    pub examples: &'static str,
}

/// order of the toggles in the "Hide" panel
pub const RULES: [RuleDef; RULE_COUNT] = [
    RuleDef { bit: R_HTML, key: "html", examples: "" },
    RuleDef { bit: R_SAVED, key: "saved_pages", examples: "Page_files, Seite-Dateien" },
    RuleDef { bit: R_JETBRAINS, key: "jetbrains", examples: ".idea, cmake-build-*, CMakeFiles, .fleet" },
    RuleDef { bit: R_VS, key: "visual_studio", examples: ".vs, .vscode, bin/obj (.csproj)" },
    RuleDef { bit: R_NODE, key: "node", examples: "node_modules, .next, .nuxt, .angular, .turbo" },
    RuleDef { bit: R_PYTHON, key: "python", examples: "__pycache__, .venv, venv (pyvenv.cfg), site-packages" },
    RuleDef { bit: R_BUILD, key: "build", examples: "target (Cargo), .gradle, build (Gradle/CMake), out (.iml)" },
    RuleDef { bit: R_CACHE, key: "caches", examples: ".cargo, .rustup, .m2, .npm, .nuget, .jdks, .cache" },
    RuleDef { bit: R_VCS, key: "vcs", examples: ".git, .svn, .hg" },
    RuleDef { bit: R_SYSTEM, key: "system", examples: "Windows, Program Files, ProgramData, AppData" },
    RuleDef { bit: R_ATTR, key: "hidden_attr", examples: "desktop.ini, Thumbs.db, $-folders" },
    RuleDef { bit: R_MANUAL, key: "manual", examples: "" },
];

/// top level folders of a drive that hold Windows / programs, not your files
const SYSTEM_TOP: [&str; 14] = [
    "Windows",
    "Program Files",
    "Program Files (x86)",
    "ProgramData",
    "PerfLogs",
    "Recovery",
    "$WinREAgent",
    "$Windows.~BT",
    "$Windows.~WS",
    "$SysReset",
    "Windows.old",
    "MSOCache",
    "Config.Msi",
    "OneDriveTemp",
];

/// the files of the folder that is being listed (names live in a local arena while scanning)
pub struct Siblings<'a> {
    pub names: &'a str,
    pub files: &'a [FileRec],
}

impl Siblings<'_> {
    fn any(&self, test: impl Fn(&str) -> bool) -> bool {
        self.files.iter().any(|f| test(&self.names[f.name_off as usize..f.name_off as usize + f.name_len as usize]))
    }
    fn has(&self, name: &str) -> bool {
        self.any(|n| n.eq_ignore_ascii_case(name))
    }
    fn has_ext(&self, ext: &str) -> bool {
        self.any(|n| ends_with_ci(n, ext))
    }
}

/// Rules a sub folder named `name` hits. `drive_root`: the parent is "X:\",
/// `parent_is_profile`: the parent's parent is called "Users" (C:\Users\me).
pub fn detect_child(name: &str, drive_root: bool, parent_is_profile: bool, sib: &Siblings) -> u16 {
    let is = |s: &str| name.eq_ignore_ascii_case(s);
    let mut r = 0;
    if (drive_root && SYSTEM_TOP.iter().any(|s| is(s))) || (parent_is_profile && is("AppData")) {
        r |= R_SYSTEM;
    }
    if is(".git") || is(".svn") || is(".hg") {
        r |= R_VCS;
    }
    if is(".idea") || is(".fleet") || is("CMakeFiles") || starts_with_ci(name, "cmake-build-") {
        r |= R_JETBRAINS;
    }
    if is(".vs") || is(".vscode") {
        r |= R_VS;
    }
    if (is("bin") || is("obj")) && [".csproj", ".vbproj", ".fsproj", ".vcxproj"].iter().any(|e| sib.has_ext(e)) {
        r |= R_VS;
    }
    if [
        "node_modules",
        "bower_components",
        ".next",
        ".nuxt",
        ".angular",
        ".parcel-cache",
        ".turbo",
        ".svelte-kit",
    ]
    .iter()
    .any(|s| is(s))
    {
        r |= R_NODE;
    }
    if [
        "__pycache__",
        ".venv",
        "site-packages",
        ".pytest_cache",
        ".mypy_cache",
        ".ruff_cache",
        ".tox",
        ".ipynb_checkpoints",
    ]
    .iter()
    .any(|s| is(s))
    {
        r |= R_PYTHON;
    }
    if [
        ".cargo",
        ".rustup",
        ".m2",
        ".npm",
        ".nuget",
        ".pnpm-store",
        ".yarn",
        ".conda",
        ".jdks",
        ".cache",
        ".pyenv",
        ".deno",
        ".bun",
        ".dotnet",
        ".android",
        ".gem",
    ]
    .iter()
    .any(|s| is(s))
    {
        r |= R_CACHE;
    }
    if is(".gradle")
        || (is("target") && (sib.has("Cargo.toml") || sib.has("pom.xml")))
        || (is("build") && (sib.has("build.gradle") || sib.has("build.gradle.kts") || sib.has("CMakeLists.txt")))
        || (is("out") && sib.has_ext(".iml"))
    {
        r |= R_BUILD;
    }
    // "Save page as" in browsers: Page.html + Page_files (German Windows: Page-Dateien)
    for suffix in ["_files", "-Dateien"] {
        if name.len() > suffix.len() && ends_with_ci(name, suffix) {
            let page = &name[..name.len() - suffix.len()];
            let html = |n: &str| {
                starts_with_ci(n, page) && {
                    let ext = &n[page.len()..];
                    ext.eq_ignore_ascii_case(".html") || ext.eq_ignore_ascii_case(".htm")
                }
            };
            if sib.any(html) {
                r |= R_SAVED;
            }
        }
    }
    r
}

/// rules a folder hits because of its own files (a venv has a pyvenv.cfg, whatever it's called)
pub fn detect_own(sib: &Siblings) -> u16 {
    if sib.has("pyvenv.cfg") {
        R_PYTHON
    } else {
        0
    }
}

fn ends_with_ci(s: &str, suffix: &str) -> bool {
    s.len() >= suffix.len()
        && s.is_char_boundary(s.len() - suffix.len())
        && s[s.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
}

fn starts_with_ci(s: &str, prefix: &str) -> bool {
    s.len() >= prefix.len() && s.is_char_boundary(prefix.len()) && s[..prefix.len()].eq_ignore_ascii_case(prefix)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sib<'a>(names: &'a str, files: &'a [FileRec]) -> Siblings<'a> {
        Siblings { names, files }
    }

    fn files_of(list: &[&str]) -> (String, Vec<FileRec>) {
        let mut names = String::new();
        let mut v = Vec::new();
        for n in list {
            v.push(FileRec { name_off: names.len() as u32, name_len: n.len() as u16, ..Default::default() });
            names.push_str(n);
        }
        (names, v)
    }

    #[test]
    fn detects() {
        let (n, f) = files_of(&["Cargo.toml", "Seite.html", "app.csproj"]);
        let s = sib(&n, &f);
        assert_eq!(detect_child("target", false, false, &s), R_BUILD);
        assert_eq!(detect_child("Seite-Dateien", false, false, &s), R_SAVED);
        assert_eq!(detect_child("Other_files", false, false, &s), 0);
        assert_eq!(detect_child("cmake-build-debug", false, false, &s), R_JETBRAINS);
        assert_eq!(detect_child("bin", false, false, &s), R_VS);
        assert_eq!(detect_child("Windows", true, false, &s), R_SYSTEM);
        assert_eq!(detect_child("Windows", false, false, &s), 0);
        assert_eq!(detect_child("AppData", false, true, &s), R_SYSTEM);
        assert_eq!(detect_child(".cargo", false, false, &s), R_CACHE);
        assert_eq!(detect_child(".M2", false, false, &s), R_CACHE);
        let (n2, f2) = files_of(&["readme.md"]);
        assert_eq!(detect_child("target", false, false, &sib(&n2, &f2)), 0);
        assert_eq!(detect_child("ÜberDateien", false, false, &sib(&n2, &f2)), 0);
    }
}
