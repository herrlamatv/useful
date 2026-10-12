// Settings in %APPDATA%\rustscanner\settings.txt, one "key=value" per line, lists = the key repeated.
// Windows paths can't contain line breaks, so nothing needs escaping.

use crate::filter::{DateField, View};
use crate::i18n::LangPref;
use crate::rules::{DEFAULT_RULES, RULES};
use crate::tree::Sort;
use std::path::PathBuf;

#[derive(Clone, PartialEq, Debug)]
pub struct Settings {
    pub lang: LangPref,
    pub view: View,
    pub sort: Sort,
    pub date_field: DateField,
    /// 0 = any, 1 = older than, 2 = years
    pub date_mode: usize,
    /// index into filter::OLDER_MONTHS
    pub older_idx: usize,
    pub year_from: i32,
    pub year_to: i32,
    /// 0 = any, else filter::SIZE_STEPS[i - 1]
    pub size_idx: usize,
    pub cats: u16,
    pub exts: String,
    pub git_only: bool,
    pub match_folders: bool,
    pub skip_system: bool,
    pub html_n: u16,
    pub rules: u16,
    pub locations: Vec<String>,
    pub whitelist: Vec<String>,
    pub manual: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            lang: LangPref::Auto,
            view: View::Tree,
            sort: Sort::Name,
            date_field: DateField::Modified,
            date_mode: 0,
            older_idx: 2,
            year_from: 2015,
            year_to: 2020,
            size_idx: 0,
            cats: 0,
            exts: String::new(),
            git_only: false,
            match_folders: true,
            skip_system: true,
            html_n: 2,
            rules: DEFAULT_RULES,
            locations: Vec::new(),
            whitelist: Vec::new(),
            manual: Vec::new(),
        }
    }
}

const VIEWS: [(View, &str); 5] =
    [(View::Tree, "tree"), (View::Folders, "folders"), (View::Types, "types"), (View::Years, "years"), (View::Hidden, "hidden")];
const SORTS: [(Sort, &str); 4] = [(Sort::Name, "name"), (Sort::Size, "size"), (Sort::Newest, "newest"), (Sort::Oldest, "oldest")];

pub fn dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("rustscanner"))
}

pub fn load() -> Settings {
    dir().and_then(|d| std::fs::read_to_string(d.join("settings.txt")).ok()).map(|t| parse(&t)).unwrap_or_default()
}

pub fn save(s: &Settings) {
    if let Some(d) = dir() {
        let _ = std::fs::create_dir_all(&d);
        let _ = std::fs::write(d.join("settings.txt"), to_text(s));
    }
}

pub fn to_text(s: &Settings) -> String {
    let mut o = String::from("# rustscanner settings\n");
    let mut kv = |k: &str, v: &str| {
        o.push_str(k);
        o.push('=');
        o.push_str(v);
        o.push('\n');
    };
    kv("lang", s.lang.key());
    kv("view", VIEWS.iter().find(|v| v.0 == s.view).map(|v| v.1).unwrap_or("tree"));
    kv("sort", SORTS.iter().find(|v| v.0 == s.sort).map(|v| v.1).unwrap_or("name"));
    kv("date_field", if s.date_field == DateField::Created { "created" } else { "modified" });
    kv("date_mode", &s.date_mode.to_string());
    kv("older", &s.older_idx.to_string());
    kv("year_from", &s.year_from.to_string());
    kv("year_to", &s.year_to.to_string());
    kv("size", &s.size_idx.to_string());
    kv("cats", &s.cats.to_string());
    kv("exts", &s.exts);
    kv("git_only", b(s.git_only));
    kv("folder_names", b(s.match_folders));
    kv("skip_system", b(s.skip_system));
    kv("html_n", &s.html_n.to_string());
    for r in RULES.iter() {
        kv(&format!("rule.{}", r.key), b(s.rules & r.bit != 0));
    }
    for l in &s.locations {
        kv("location", l);
    }
    for l in &s.whitelist {
        kv("whitelist", l);
    }
    for l in &s.manual {
        kv("manual_hide", l);
    }
    o
}

fn b(v: bool) -> &'static str {
    if v {
        "1"
    } else {
        "0"
    }
}

pub fn parse(text: &str) -> Settings {
    let mut s = Settings::default();
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else { continue };
        let num = |d: usize| v.trim().parse().unwrap_or(d);
        match k.trim() {
            "lang" => s.lang = LangPref::from_key(v.trim()),
            "view" => s.view = VIEWS.iter().find(|x| x.1 == v.trim()).map(|x| x.0).unwrap_or(View::Tree),
            "sort" => s.sort = SORTS.iter().find(|x| x.1 == v.trim()).map(|x| x.0).unwrap_or(Sort::Name),
            "date_field" => s.date_field = if v.trim() == "created" { DateField::Created } else { DateField::Modified },
            "date_mode" => s.date_mode = num(0).min(2),
            "older" => s.older_idx = num(2).min(4),
            "year_from" => s.year_from = v.trim().parse().unwrap_or(s.year_from),
            "year_to" => s.year_to = v.trim().parse().unwrap_or(s.year_to),
            "size" => s.size_idx = num(0).min(3),
            "cats" => s.cats = v.trim().parse().unwrap_or(0),
            "exts" => s.exts = v.to_string(),
            "git_only" => s.git_only = v.trim() == "1",
            "folder_names" => s.match_folders = v.trim() == "1",
            "skip_system" => s.skip_system = v.trim() == "1",
            "html_n" => s.html_n = v.trim().parse::<u16>().unwrap_or(2).clamp(1, 99),
            "location" if !v.trim().is_empty() => s.locations.push(v.trim().to_string()),
            "whitelist" if !v.trim().is_empty() => s.whitelist.push(v.trim().to_string()),
            "manual_hide" if !v.trim().is_empty() => s.manual.push(v.trim().to_string()),
            k if k.starts_with("rule.") => {
                if let Some(r) = RULES.iter().find(|r| r.key == &k[5..]) {
                    if v.trim() == "1" {
                        s.rules |= r.bit;
                    } else {
                        s.rules &= !r.bit;
                    }
                }
            }
            _ => {}
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let s = Settings {
            lang: LangPref::De,
            view: View::Years,
            sort: Sort::Oldest,
            date_field: DateField::Created,
            date_mode: 2,
            older_idx: 4,
            year_from: 2011,
            year_to: 2019,
            size_idx: 3,
            cats: 0b101,
            exts: "psd blend".into(),
            git_only: true,
            match_folders: false,
            skip_system: false,
            html_n: 7,
            rules: DEFAULT_RULES & !crate::rules::R_NODE,
            locations: vec!["C:\\".into(), "D:\\Fotos = alt".into()],
            whitelist: vec!["C:\\Users\\me\\Downloads".into()],
            manual: vec!["C:\\x".into()],
        };
        assert_eq!(parse(&to_text(&s)), s);
        assert_eq!(parse(""), Settings::default());
    }
}
