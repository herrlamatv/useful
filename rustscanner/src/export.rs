// Markdown export of what is shown right now, in the same format as filescanner.bat
// (header table + results table, | and ` escaped the same way).

use crate::filter::{DateField, DateFilter, FilterParams, FilterResult, View, OLDER_MONTHS, UNKNOWN_YEAR};
use crate::i18n::{fill, T};
use crate::model::*;
use crate::rules::{INHERITED, RULES};
use crate::win;
use std::io::{BufWriter, Write};
use std::path::Path;

/// `name` -> code span, like Md-Code in filescanner.bat
pub fn md_code(s: &str) -> String {
    let s = s.replace('|', "\\|");
    if s.contains('`') {
        format!("`` {s} ``")
    } else {
        format!("`{s}`")
    }
}

/// one sentence about the filter: "name like '*.jpg', older than 2 years"
pub fn describe(t: &T, p: &FilterParams) -> String {
    let mut parts = Vec::new();
    let search = p.search.trim();
    if !search.is_empty() {
        let mut s = fill(t.desc_name, &[("p", search)]);
        if p.match_folders {
            s.push(' ');
            s.push_str(t.desc_folders);
        }
        parts.push(s);
    }
    if p.cats != 0 && p.cats != CAT_ALL {
        let names: Vec<&str> = (0..CAT_COUNT).filter(|i| p.cats & (1 << i) != 0).map(|i| t.cats[i]).collect();
        parts.push(fill(t.desc_types, &[("t", &names.join(", "))]));
    }
    if !p.exts.is_empty() {
        let e: Vec<String> = p.exts.iter().map(|e| format!(".{e}")).collect();
        parts.push(fill(t.desc_ext, &[("e", &e.join(" "))]));
    }
    let field = t.date_fields[(p.date_field == DateField::Created) as usize];
    match p.date {
        DateFilter::Any => {}
        DateFilter::OlderMonths(m) => {
            let i = OLDER_MONTHS.iter().position(|&x| x == m).unwrap_or(0);
            parts.push(fill(t.desc_older, &[("f", field), ("x", t.older[i])]));
        }
        DateFilter::Years(a, b) => {
            parts.push(fill(t.desc_years, &[("f", field), ("a", &a.min(b).to_string()), ("b", &a.max(b).to_string())]))
        }
    }
    if p.min_size > 0 {
        parts.push(fill(t.desc_size, &[("s", &t.size(p.min_size))]));
    }
    if p.git_only {
        parts.push(t.desc_git.to_string());
    }
    if parts.is_empty() {
        t.desc_all.to_string()
    } else {
        parts.join(", ")
    }
}

pub struct Info<'a> {
    pub t: &'static T,
    pub idx: &'a Index,
    pub res: &'a FilterResult,
    pub params: &'a FilterParams,
    pub view: View,
}

pub fn write(path: &Path, info: &Info) -> std::io::Result<()> {
    let mut w = BufWriter::new(std::fs::File::create(path)?);
    write_to(&mut w, info)?;
    w.flush()
}

pub fn write_to(w: &mut impl Write, info: &Info) -> std::io::Result<()> {
    let Info { t, idx, res, params, view } = *info;
    let date = |s: i64| win::local(s).map(|d| d.ymd_hm()).unwrap_or_else(|| "-".into());
    let view_name = t.views[View::ALL.iter().position(|v| *v == view).unwrap_or(0)];
    let locations: Vec<String> = idx.roots.iter().map(|r| md_code(&r.path)).collect();
    let rules_on: Vec<String> =
        RULES.iter().filter(|r| params.rules & r.bit != 0).map(|r| t.rule_name(r.bit, params.html_n)).collect();
    let hidden = if res.hidden_top.is_empty() && res.hidden_files == 0 {
        t.ex_none.to_string()
    } else {
        format!(
            "{} · {}",
            fill(t.rule_stat, &[("f", &t.int(res.hidden_top.len() as u64)), ("n", &t.int(res.hidden_files)), ("s", &t.size(res.hidden_bytes))]),
            rules_on.join(", ")
        )
    };

    writeln!(w, "# {}", t.ex_title)?;
    writeln!(w)?;
    writeln!(w, "| | |")?;
    writeln!(w, "|---|---|")?;
    writeln!(w, "| {} | {} |", t.ex_date, win::local(idx.started).map(|d| d.ymd_hms()).unwrap_or_default())?;
    writeln!(w, "| {} | {} |", t.ex_locations, locations.join(", "))?;
    writeln!(w, "| {} | {} |", t.ex_view, view_name)?;
    writeln!(w, "| {} | {} |", t.ex_filter, md_code(&describe(t, params)))?;
    writeln!(w, "| {} | {} |", t.ex_hidden, hidden.replace('|', "\\|"))?;
    writeln!(w, "| {} | {} ({}) |", t.ex_scanned, t.int(idx.folders.len() as u64), fill(t.denied, &[("n", &t.int(idx.denied as u64))]))?;
    writeln!(w, "| {} | {} |", t.ex_duration, t.duration(idx.duration))?;
    writeln!(w, "| {} | {} |", t.ex_results, t.int(res.total.count as u64))?;
    if idx.cancelled {
        writeln!(w)?;
        writeln!(w, "> {}", t.ex_cancelled)?;
    }
    writeln!(w)?;
    writeln!(w, "## {}", t.ex_results)?;
    writeln!(w)?;

    let field = t.date_fields[(params.date_field == DateField::Created) as usize];
    let header = |w: &mut dyn Write| -> std::io::Result<()> {
        writeln!(w, "| # | {} | {} | {} | {} |", t.col_name, t.size, field, t.ex_folder)?;
        writeln!(w, "|--:|---|--:|---|---|")
    };
    let row = |w: &mut dyn Write, n: usize, i: u32| -> std::io::Result<()> {
        let r = &idx.files[i as usize];
        let d = if params.date_field == DateField::Created { r.ctime } else { r.mtime };
        writeln!(
            w,
            "| {} | {} | {} | {} | {} |",
            n,
            md_code(idx.file_name(i)),
            t.size(r.size),
            date(d),
            md_code(&idx.folder_path(r.folder))
        )
    };

    if res.total.count == 0 {
        writeln!(w, "{}", t.ex_nothing)?;
    } else if matches!(view, View::Types | View::Years) && !res.groups.is_empty() {
        for g in &res.groups {
            let title = match view {
                View::Types => t.cats[g.key as usize].to_string(),
                _ if g.key == UNKNOWN_YEAR => t.unknown_year.to_string(),
                _ => g.key.to_string(),
            };
            writeln!(w, "### {} ({})", title, t.int(g.files.len() as u64))?;
            writeln!(w)?;
            header(w)?;
            for (n, &i) in g.files.iter().enumerate() {
                row(w, n + 1, i)?;
            }
            writeln!(w)?;
        }
    } else {
        header(w)?;
        let mut n = 0;
        for i in 0..idx.files.len() as u32 {
            if res.is_match(i) {
                n += 1;
                row(w, n, i)?;
            }
        }
    }

    if !res.hidden_top.is_empty() {
        writeln!(w)?;
        writeln!(w, "## {}", t.ex_hidden_h)?;
        writeln!(w)?;
        writeln!(w, "| # | {} | {} | {} | {} |", t.ex_folder, t.ex_reason, t.col_files, t.size)?;
        writeln!(w, "|--:|---|---|--:|--:|")?;
        for (n, &f) in res.hidden_top.iter().enumerate() {
            let d = &idx.folders[f as usize];
            let reason = t.rule_name(res.hide[f as usize] & !INHERITED, params.html_n);
            writeln!(
                w,
                "| {} | {} | {} | {} | {} |",
                n + 1,
                md_code(&idx.folder_path(f)),
                reason,
                t.int(d.sub_files as u64),
                t.size(d.sub_bytes)
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::tests::filt;
    use crate::i18n::{DE, EN};
    use crate::scan::tests::{fixture, scan_dir};

    #[test]
    fn escaping() {
        assert_eq!(md_code("a|b"), "`a\\|b`");
        assert_eq!(md_code("x`y"), "`` x`y ``");
    }

    #[test]
    fn export_md() {
        let fx = fixture("export");
        let idx = scan_dir(&fx.dir, 2);
        let p = FilterParams { search: "urlaub".into(), ..Default::default() };
        let res = filt(&idx, &p);
        let mut out = Vec::new();
        write_to(&mut out, &Info { t: &EN, idx: &idx, res: &res, params: &p, view: View::Tree }).unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(s.starts_with("# Filescan\n"));
        assert!(s.contains("| Results | 1 |"));
        assert!(s.contains("`Urlaub.JPG`"));
        assert!(s.contains("| 1 | `Urlaub.JPG` | 1.9 MB | 2019-"));
        assert!(s.contains("## Hidden folders"));
        assert!(s.contains("name like 'urlaub' (folder names too)"));

        let p = FilterParams { view: View::Years, ..Default::default() };
        let res = filt(&idx, &p);
        let mut out = Vec::new();
        write_to(&mut out, &Info { t: &DE, idx: &idx, res: &res, params: &p, view: View::Years }).unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains("### 2019 (1)"));
        assert!(s.contains("| Ergebnisse |"));
    }
}
