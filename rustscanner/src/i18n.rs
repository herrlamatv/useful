// English + German texts, switchable at runtime (Auto · DE · EN like in chroofer).
// Every text is a struct field, so a missing translation is a compile error.
// {placeholders} are filled with fill().

use crate::rules::RULE_COUNT;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En,
    De,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LangPref {
    Auto,
    De,
    En,
}

impl LangPref {
    pub const ALL: [LangPref; 3] = [LangPref::Auto, LangPref::De, LangPref::En];

    pub fn resolve(self) -> Lang {
        match self {
            LangPref::De => Lang::De,
            LangPref::En => Lang::En,
            LangPref::Auto if crate::win::ui_is_german() => Lang::De,
            LangPref::Auto => Lang::En,
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            LangPref::Auto => "auto",
            LangPref::De => "de",
            LangPref::En => "en",
        }
    }
    pub fn from_key(s: &str) -> Self {
        match s {
            "de" => LangPref::De,
            "en" => LangPref::En,
            _ => LangPref::Auto,
        }
    }
}

pub fn texts(lang: Lang) -> &'static T {
    match lang {
        Lang::En => &EN,
        Lang::De => &DE,
    }
}

/// "{n} files" + [("n", "3")] -> "3 files"
pub fn fill(tpl: &str, args: &[(&str, &str)]) -> String {
    let mut s = tpl.to_string();
    for (k, v) in args {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s
}

pub struct T {
    pub lang: Lang,
    // header
    pub app_by: &'static str,
    pub lang_auto: &'static str,
    pub lang_auto_tip: &'static str,
    pub chip_files: &'static str,
    pub chip_scanning: &'static str,
    pub chip_cancelled: &'static str,
    pub chip_updating: &'static str,
    // locations
    pub loc_title: &'static str,
    pub loc_sub: &'static str,
    pub drives: &'static str,
    pub drive_free: &'static str,
    pub drive_kinds: [&'static str; 5],
    pub quick: &'static str,
    pub known: [&'static str; 8],
    pub folders: &'static str,
    pub add_folder: &'static str,
    pub drop_hint: &'static str,
    pub no_location: &'static str,
    pub nested: &'static str,
    pub skip_system: &'static str,
    pub skip_system_sub: &'static str,
    pub scan: &'static str,
    pub scan_again: &'static str,
    pub cancel: &'static str,
    pub loc_changed: &'static str,
    pub drop_overlay: &'static str,
    pub remove: &'static str,
    pub refresh_drives: &'static str,
    // filter
    pub filter_title: &'static str,
    pub filter_sub: &'static str,
    pub types: &'static str,
    pub cats: [&'static str; 9],
    pub git_repos: &'static str,
    pub ext: &'static str,
    pub ext_hint: &'static str,
    pub date: &'static str,
    pub date_fields: [&'static str; 2],
    pub date_modes: [&'static str; 3],
    pub older: [&'static str; 5],
    pub older_short: [&'static str; 5],
    pub year_from: &'static str,
    pub year_to: &'static str,
    pub created_hint: &'static str,
    pub size: &'static str,
    pub size_opts: [&'static str; 4],
    pub match_folders: &'static str,
    pub match_folders_sub: &'static str,
    pub reset: &'static str,
    // hide rules
    pub hide_title: &'static str,
    pub hide_sub: &'static str,
    /// by rule bit index
    pub rule_names: [&'static str; RULE_COUNT],
    pub html_examples: &'static str,
    pub manual_examples: &'static str,
    pub rule_stat: &'static str,
    pub rule_none: &'static str,
    pub show_list: &'static str,
    // toolbar
    pub search_hint: &'static str,
    pub views: [&'static str; 5],
    pub sorts: [&'static str; 4],
    pub expand_all: &'static str,
    pub collapse_all: &'static str,
    pub export: &'static str,
    pub summary: &'static str,
    pub summary_hidden: &'static str,
    pub col_name: &'static str,
    pub col_files: &'static str,
    // rows
    pub more: &'static str,
    pub too_many: &'static str,
    pub nothing: &'static str,
    pub sec_hidden: &'static str,
    pub sec_shown: &'static str,
    pub show_anyway: &'static str,
    pub hide_again: &'static str,
    pub unknown_year: &'static str,
    pub no_hidden: &'static str,
    // welcome + scan
    pub welcome_title: &'static str,
    pub welcome_sub: &'static str,
    pub welcome_tips: [&'static str; 3],
    pub scanning_title: &'static str,
    pub scan_hint: &'static str,
    pub denied: &'static str,
    // details / context menu
    pub open: &'static str,
    pub show_explorer: &'static str,
    pub copy_path: &'static str,
    pub copied: &'static str,
    pub hide_folder: &'static str,
    pub expand_below: &'static str,
    pub d_accessed: &'static str,
    pub cloud: &'static str,
    pub open_failed: &'static str,
    pub d_files: &'static str,
    pub hidden_because: &'static str,
    // export
    pub ex_title: &'static str,
    pub ex_date: &'static str,
    pub ex_locations: &'static str,
    pub ex_view: &'static str,
    pub ex_filter: &'static str,
    pub ex_hidden: &'static str,
    pub ex_scanned: &'static str,
    pub ex_duration: &'static str,
    pub ex_results: &'static str,
    pub ex_cancelled: &'static str,
    pub ex_hidden_h: &'static str,
    pub ex_nothing: &'static str,
    pub ex_folder: &'static str,
    pub ex_saved: &'static str,
    pub ex_failed: &'static str,
    pub ex_show: &'static str,
    pub ex_none: &'static str,
    pub ex_writing: &'static str,
    pub ex_reason: &'static str,
    pub desc_all: &'static str,
    pub desc_name: &'static str,
    pub desc_types: &'static str,
    pub desc_ext: &'static str,
    pub desc_older: &'static str,
    pub desc_years: &'static str,
    pub desc_size: &'static str,
    pub desc_git: &'static str,
    pub desc_folders: &'static str,
    // age
    pub age_today: &'static str,
    pub age_day: &'static str,
    pub age_days: &'static str,
    pub age_month: &'static str,
    pub age_months: &'static str,
    pub age_year: &'static str,
    pub age_years: &'static str,
}

pub static EN: T = T {
    lang: Lang::En,
    app_by: "by herrlamatv",
    lang_auto: "Auto",
    lang_auto_tip: "Follow the Windows language",
    chip_files: "{n} files",
    chip_scanning: "Scanning …",
    chip_cancelled: "Cancelled – incomplete",
    chip_updating: "updating …",
    loc_title: "Locations",
    loc_sub: "Where should I look?",
    drives: "Drives",
    drive_free: "{free} free of {total}",
    drive_kinds: ["Local disk", "USB / removable", "Network", "CD / DVD", "RAM disk"],
    quick: "Quick",
    known: ["Desktop", "Documents", "Downloads", "Pictures", "Videos", "Music", "User folder", "OneDrive"],
    folders: "Folders",
    add_folder: "+  Add folder",
    drop_hint: "or drop folders into the window",
    no_location: "Pick a drive or a folder first.",
    nested: "inside {p} – skipped",
    skip_system: "Skip Windows & program folders",
    skip_system_sub: "Faster. Windows, Program Files, ProgramData and AppData are not entered.",
    scan: "Scan",
    scan_again: "Scan again",
    cancel: "Cancel",
    loc_changed: "Locations changed – scan again to see them.",
    drop_overlay: "Drop folders to add them",
    remove: "Remove",
    refresh_drives: "Refresh drives",
    filter_title: "Filter",
    filter_sub: "Everything updates live – no new scan needed.",
    types: "Types",
    cats: ["Web", "Images", "Videos", "Audio", "Documents", "Code / scripts", "Archives", "Programs", "Other"],
    git_repos: "Only in Git repos",
    ext: "Extensions",
    ext_hint: "e.g. psd blend",
    date: "Date",
    date_fields: ["Modified", "Created"],
    date_modes: ["Any", "Older than", "Years"],
    older: ["1 month", "6 months", "1 year", "2 years", "5 years"],
    older_short: ["1 M", "6 M", "1 Y", "2 Y", "5 Y"],
    year_from: "from",
    year_to: "to",
    created_hint: "Created is often the day a file was copied here, not when it was made.",
    size: "Size",
    size_opts: ["Any", "> 1 MB", "> 100 MB", "> 1 GB"],
    match_folders: "Search folder names too",
    match_folders_sub: "A matching folder shows everything inside it.",
    reset: "Reset filters",
    hide_title: "Hide",
    hide_sub: "Noise like tool folders. Everything hidden is listed and can be shown anyway.",
    rule_names: [
        "Windows & program folders",
        "Hidden by you",
        "Git / SVN internals",
        "CLion / JetBrains / CMake",
        "Visual Studio / VS Code",
        "Node / web builds",
        "Python (venv, caches)",
        "Build output (Rust, Gradle …)",
        "Folders with more than {n} HTML files",
        "Saved web pages",
        "Hidden / system files",
        "Package caches (Cargo, Maven, npm …)",
    ],
    html_examples: "the whole folder incl. subfolders",
    manual_examples: "right-click a folder → Hide this folder",
    rule_stat: "{f} folders · {n} files · {s}",
    rule_none: "nothing found",
    show_list: "Show list ({n})",
    search_hint: "Search name …   * ? = wildcard",
    views: ["Tree", "Folders", "Types", "Years", "Hidden"],
    sorts: ["Name", "Size", "Newest", "Oldest"],
    expand_all: "Expand all",
    collapse_all: "Collapse all",
    export: "Export .md",
    summary: "{n} files · {s} · in {f} folders",
    summary_hidden: "{n} files hidden",
    col_name: "Name",
    col_files: "Files",
    more: "+ {n} more – click to show",
    too_many: "Too many rows – search more precisely or collapse folders.",
    nothing: "Nothing found with these filters.",
    sec_hidden: "Hidden folders – with everything inside",
    sec_shown: "Shown anyway",
    show_anyway: "Show anyway",
    hide_again: "Hide again",
    unknown_year: "Unknown date",
    no_hidden: "Nothing is hidden.",
    welcome_title: "Find your old files again",
    welcome_sub: "Pick drives or folders on the left and press Scan. Then filter live by name, type, date and size – the tree updates instantly.",
    welcome_tips: [
        "Years view: “I made it around 2021” → open 2021.",
        "Tool folders (.idea, node_modules …) are hidden – the Hidden view lists them.",
        "Right-click any row: open, show in Explorer, copy path.",
    ],
    scanning_title: "Scanning …",
    scan_hint: "Cancel keeps what was found so far.",
    denied: "{n} not accessible",
    open: "Open",
    show_explorer: "Show in Explorer",
    copy_path: "Copy path",
    copied: "Copied",
    hide_folder: "Hide this folder",
    expand_below: "Expand everything below",
    d_accessed: "Accessed",
    cloud: "Only in the cloud – opening downloads it.",
    open_failed: "Could not open it.",
    d_files: "{n} files · {s}",
    hidden_because: "Hidden: {r}",
    ex_title: "Filescan",
    ex_date: "Date",
    ex_locations: "Locations",
    ex_view: "View",
    ex_filter: "Filter",
    ex_hidden: "Hidden",
    ex_scanned: "Folders scanned",
    ex_duration: "Duration",
    ex_results: "Results",
    ex_cancelled: "**Cancelled** – the results are incomplete.",
    ex_hidden_h: "Hidden folders",
    ex_nothing: "Nothing found.",
    ex_folder: "Folder",
    ex_saved: "Saved",
    ex_failed: "Export failed: {e}",
    ex_show: "Show",
    ex_none: "none",
    ex_writing: "Exporting …",
    ex_reason: "Reason",
    desc_all: "all files",
    desc_name: "name like '{p}'",
    desc_types: "types: {t}",
    desc_ext: "extension {e}",
    desc_older: "{f} older than {x}",
    desc_years: "{f} {a}–{b}",
    desc_size: "size > {s}",
    desc_git: "only in Git repos",
    desc_folders: "(folder names too)",
    age_today: "today",
    age_day: "1 day ago",
    age_days: "{n} days ago",
    age_month: "1 month ago",
    age_months: "{n} months ago",
    age_year: "1 year ago",
    age_years: "{n} years ago",
};

pub static DE: T = T {
    lang: Lang::De,
    app_by: "by herrlamatv",
    lang_auto: "Auto",
    lang_auto_tip: "Wie die Windows-Sprache",
    chip_files: "{n} Dateien",
    chip_scanning: "Scanne …",
    chip_cancelled: "Abgebrochen – unvollständig",
    chip_updating: "aktualisiere …",
    loc_title: "Orte",
    loc_sub: "Wo soll ich suchen?",
    drives: "Laufwerke",
    drive_free: "{free} frei von {total}",
    drive_kinds: ["Lokaler Datenträger", "USB / Wechseldatenträger", "Netzwerk", "CD / DVD", "RAM-Disk"],
    quick: "Schnellwahl",
    known: ["Desktop", "Dokumente", "Downloads", "Bilder", "Videos", "Musik", "Benutzerordner", "OneDrive"],
    folders: "Ordner",
    add_folder: "+  Ordner hinzufügen",
    drop_hint: "oder Ordner ins Fenster ziehen",
    no_location: "Wähle zuerst ein Laufwerk oder einen Ordner.",
    nested: "liegt in {p} – übersprungen",
    skip_system: "Windows- & Programmordner überspringen",
    skip_system_sub: "Schneller. Windows, Program Files, ProgramData und AppData werden nicht betreten.",
    scan: "Scannen",
    scan_again: "Neu scannen",
    cancel: "Abbrechen",
    loc_changed: "Orte geändert – neu scannen, um sie zu sehen.",
    drop_overlay: "Ordner loslassen, um sie hinzuzufügen",
    remove: "Entfernen",
    refresh_drives: "Laufwerke neu laden",
    filter_title: "Filter",
    filter_sub: "Alles aktualisiert sich live – kein neuer Scan nötig.",
    types: "Typen",
    cats: ["Web", "Bilder", "Videos", "Audio", "Dokumente", "Code / Skripte", "Archive", "Programme", "Andere"],
    git_repos: "Nur in Git-Repos",
    ext: "Endungen",
    ext_hint: "z. B. psd blend",
    date: "Datum",
    date_fields: ["Geändert", "Erstellt"],
    date_modes: ["Egal", "Älter als", "Jahre"],
    older: ["1 Monat", "6 Monate", "1 Jahr", "2 Jahre", "5 Jahre"],
    older_short: ["1 M", "6 M", "1 J", "2 J", "5 J"],
    year_from: "von",
    year_to: "bis",
    created_hint: "Erstellt ist oft der Tag, an dem die Datei hierher kopiert wurde – nicht, wann sie entstand.",
    size: "Größe",
    size_opts: ["Egal", "> 1 MB", "> 100 MB", "> 1 GB"],
    match_folders: "Auch Ordnernamen durchsuchen",
    match_folders_sub: "Ein passender Ordner zeigt alles, was darin liegt.",
    reset: "Filter zurücksetzen",
    hide_title: "Ausblenden",
    hide_sub: "Lärm wie Tool-Ordner. Alles Ausgeblendete wird aufgelistet und lässt sich trotzdem anzeigen.",
    rule_names: [
        "Windows- & Programmordner",
        "Von dir ausgeblendet",
        "Git- / SVN-Interna",
        "CLion / JetBrains / CMake",
        "Visual Studio / VS Code",
        "Node / Web-Builds",
        "Python (venv, Caches)",
        "Build-Ausgabe (Rust, Gradle …)",
        "Ordner mit mehr als {n} HTML-Dateien",
        "Gespeicherte Webseiten",
        "Versteckte / Systemdateien",
        "Paket-Caches (Cargo, Maven, npm …)",
    ],
    html_examples: "der ganze Ordner inkl. Unterordner",
    manual_examples: "Rechtsklick auf einen Ordner → Diesen Ordner ausblenden",
    rule_stat: "{f} Ordner · {n} Dateien · {s}",
    rule_none: "nichts gefunden",
    show_list: "Liste zeigen ({n})",
    search_hint: "Name suchen …   * ? = Platzhalter",
    views: ["Baum", "Ordner", "Typen", "Jahre", "Ausgeblendet"],
    sorts: ["Name", "Größe", "Neueste", "Älteste"],
    expand_all: "Alle aufklappen",
    collapse_all: "Alle zuklappen",
    export: "Als .md exportieren",
    summary: "{n} Dateien · {s} · in {f} Ordnern",
    summary_hidden: "{n} Dateien ausgeblendet",
    col_name: "Name",
    col_files: "Dateien",
    more: "+ {n} weitere – klicken zum Anzeigen",
    too_many: "Zu viele Zeilen – genauer suchen oder Ordner zuklappen.",
    nothing: "Mit diesen Filtern nichts gefunden.",
    sec_hidden: "Ausgeblendete Ordner – samt Inhalt",
    sec_shown: "Trotzdem angezeigt",
    show_anyway: "Trotzdem zeigen",
    hide_again: "Wieder ausblenden",
    unknown_year: "Unbekanntes Datum",
    no_hidden: "Nichts ausgeblendet.",
    welcome_title: "Finde deine alten Dateien wieder",
    welcome_sub: "Wähle links Laufwerke oder Ordner und drücke Scannen. Danach live nach Name, Typ, Datum und Größe filtern – der Baum passt sich sofort an.",
    welcome_tips: [
        "Jahre-Ansicht: „Das war so um 2021“ → 2021 aufklappen.",
        "Tool-Ordner (.idea, node_modules …) sind ausgeblendet – die Ansicht „Ausgeblendet“ listet sie auf.",
        "Rechtsklick auf eine Zeile: öffnen, im Explorer zeigen, Pfad kopieren.",
    ],
    scanning_title: "Scanne …",
    scan_hint: "Abbrechen behält, was bisher gefunden wurde.",
    denied: "{n} nicht lesbar",
    open: "Öffnen",
    show_explorer: "Im Explorer zeigen",
    copy_path: "Pfad kopieren",
    copied: "Kopiert",
    hide_folder: "Diesen Ordner ausblenden",
    expand_below: "Alles darunter aufklappen",
    d_accessed: "Letzter Zugriff",
    cloud: "Nur in der Cloud – Öffnen lädt sie herunter.",
    open_failed: "Konnte nicht geöffnet werden.",
    d_files: "{n} Dateien · {s}",
    hidden_because: "Ausgeblendet: {r}",
    ex_title: "Filescan",
    ex_date: "Datum",
    ex_locations: "Orte",
    ex_view: "Ansicht",
    ex_filter: "Filter",
    ex_hidden: "Ausgeblendet",
    ex_scanned: "Ordner gescannt",
    ex_duration: "Dauer",
    ex_results: "Ergebnisse",
    ex_cancelled: "**Abgebrochen** – die Ergebnisse sind unvollständig.",
    ex_hidden_h: "Ausgeblendete Ordner",
    ex_nothing: "Nichts gefunden.",
    ex_folder: "Ordner",
    ex_saved: "Gespeichert",
    ex_failed: "Export fehlgeschlagen: {e}",
    ex_show: "Zeigen",
    ex_none: "keine",
    ex_writing: "Exportiere …",
    ex_reason: "Grund",
    desc_all: "alle Dateien",
    desc_name: "Name wie '{p}'",
    desc_types: "Typen: {t}",
    desc_ext: "Endung {e}",
    desc_older: "{f} älter als {x}",
    desc_years: "{f} {a}–{b}",
    desc_size: "Größe > {s}",
    desc_git: "nur in Git-Repos",
    desc_folders: "(auch Ordnernamen)",
    age_today: "heute",
    age_day: "vor 1 Tag",
    age_days: "vor {n} Tagen",
    age_month: "vor 1 Monat",
    age_months: "vor {n} Monaten",
    age_year: "vor 1 Jahr",
    age_years: "vor {n} Jahren",
};

impl T {
    /// 1,234,567 / 1.234.567
    pub fn int(&self, n: u64) -> String {
        let sep = if self.lang == Lang::De { '.' } else { ',' };
        let s = n.to_string();
        let mut out = String::with_capacity(s.len() + s.len() / 3);
        for (i, c) in s.chars().enumerate() {
            if i > 0 && (s.len() - i).is_multiple_of(3) {
                out.push(sep);
            }
            out.push(c);
        }
        out
    }

    /// same steps as filescanner.bat (TB 2 decimals, GB/MB 1, KB 0), decimal comma in German
    pub fn size(&self, b: u64) -> String {
        const K: f64 = 1024.0;
        let f = b as f64;
        let s = if f >= K * K * K * K {
            format!("{:.2} TB", f / (K * K * K * K))
        } else if f >= K * K * K {
            format!("{:.1} GB", f / (K * K * K))
        } else if f >= K * K {
            format!("{:.1} MB", f / (K * K))
        } else if f >= K {
            format!("{:.0} KB", f / K)
        } else {
            return format!("{b} B");
        };
        if self.lang == Lang::De {
            s.replace('.', ",")
        } else {
            s
        }
    }

    /// "3 years ago" / "vor 3 Jahren"
    pub fn age(&self, secs_ago: i64) -> String {
        let days = secs_ago / 86_400;
        let n = |tpl: &str, v: i64| fill(tpl, &[("n", &v.to_string())]);
        if days < 1 {
            self.age_today.into()
        } else if days < 2 {
            self.age_day.into()
        } else if days < 31 {
            n(self.age_days, days)
        } else if days < 61 {
            self.age_month.into()
        } else if days < 365 {
            n(self.age_months, days / 30)
        } else if days < 730 {
            self.age_year.into()
        } else {
            n(self.age_years, days / 365)
        }
    }

    /// 00:42 / 1:02:03 like filescanner
    pub fn duration(&self, d: Duration) -> String {
        let s = d.as_secs();
        if s >= 3600 {
            format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
        } else {
            format!("{:02}:{:02}", s / 60, s % 60)
        }
    }

    pub fn rule_name(&self, bit: u16, html_n: u16) -> String {
        let name = self.rule_names[bit.trailing_zeros() as usize];
        fill(name, &[("n", &html_n.to_string())])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatting() {
        assert_eq!(EN.int(1234567), "1,234,567");
        assert_eq!(DE.int(1234567), "1.234.567");
        assert_eq!(EN.int(999), "999");
        assert_eq!(EN.size(1536 * 1024), "1.5 MB");
        assert_eq!(DE.size(1536 * 1024), "1,5 MB");
        assert_eq!(EN.size(500), "500 B");
        assert_eq!(DE.age(3 * 365 * 86_400 + 10), "vor 3 Jahren");
        assert_eq!(EN.age(5 * 86_400), "5 days ago");
        assert_eq!(EN.duration(Duration::from_secs(62)), "01:02");
        assert_eq!(EN.duration(Duration::from_secs(3723)), "1:02:03");
        assert_eq!(DE.rule_name(crate::rules::R_HTML, 2), "Ordner mit mehr als 2 HTML-Dateien");
    }
}
