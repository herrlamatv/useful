// Rules: "when this program starts, give it this profile". Every rule can be switched off.

use crate::procs;
use crate::profile::Profile;

#[derive(Clone, PartialEq, Debug)]
pub struct Rule {
    pub enabled: bool,
    /// exe names, comma separated, `*` as wildcard ("7z.exe, WinRAR.exe") - or a path part ("\steamapps\common\")
    pub pattern: String,
    pub profile: Profile,
}

impl Rule {
    pub fn encode(&self) -> String {
        format!("{}|{}|{}", if self.enabled { 1 } else { 0 }, self.pattern.replace('|', ""), self.profile.encode())
    }

    pub fn decode(s: &str) -> Option<Rule> {
        let mut p = s.splitn(3, '|');
        let enabled = p.next()?.trim() == "1";
        let pattern = p.next()?.trim().to_string();
        let profile = Profile::decode(p.next().unwrap_or("")).ok()?;
        Some(Rule { enabled, pattern, profile })
    }

    /// name = exe name, path = full exe path (if readable)
    pub fn matches(&self, name: &str, path: Option<&str>) -> bool {
        matches_list(&self.pattern, name, path)
    }
}

/// "a.exe, b*, \steamapps\common\" against one process
pub fn matches_list(list: &str, name: &str, path: Option<&str>) -> bool {
    list.split([',', ';', '\n']).map(str::trim).filter(|p| !p.is_empty()).any(|p| {
        if p.contains('\\') || p.contains('/') {
            let needle = p.replace('/', "\\").to_ascii_lowercase();
            path.is_some_and(|x| x.to_ascii_lowercase().contains(&needle))
        } else {
            procs::name_matches(p, name)
        }
    })
}

pub fn defaults() -> Vec<Rule> {
    vec![
        Rule {
            enabled: true,
            pattern: "7z.exe, 7zG.exe, 7zFM.exe, WinRAR.exe, Rar.exe, UnRAR.exe, tar.exe".into(),
            profile: Profile::decode("prio=high;io=high;eco=off").unwrap(),
        },
        Rule {
            enabled: false,
            pattern: "chrome.exe, msedge.exe".into(),
            profile: Profile::decode("cpu=0-20;eco=on").unwrap(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codec_and_match() {
        for r in defaults() {
            assert_eq!(Rule::decode(&r.encode()), Some(r));
        }
        let r = &defaults()[0];
        assert!(r.matches("WinRAR.exe", None));
        assert!(r.matches("7zg.EXE", None));
        assert!(!r.matches("7zip.exe", None));
        assert!(matches_list(r"\steamapps\common\", "game.exe", Some(r"D:\Steam\steamapps\common\X\game.exe")));
        assert!(!matches_list(r"\steamapps\common\", "game.exe", None));
    }
}
