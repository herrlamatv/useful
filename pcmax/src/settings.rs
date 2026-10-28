// Settings in %APPDATA%\pcmax\settings.txt, one "key=value" per line, lists = the key repeated.
// Exe paths and names can't contain line breaks, so nothing needs escaping.

use crate::boost::{self, Measure};
use crate::rules::{self, Rule};
use std::path::PathBuf;

pub fn dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("pcmax"))
}

fn file() -> Option<PathBuf> {
    dir().map(|d| d.join("settings.txt"))
}

#[derive(Clone, PartialEq, Debug)]
pub struct Settings {
    /// "auto" | "de" | "en"
    pub lang: String,
    pub boost_on: bool,
    pub measures: Vec<Measure>,
    /// exe names, comma separated
    pub background: String,
    pub work: String,
    pub games: String,
    /// everything under \steamapps\common\ and \Epic Games\ counts as a game (touches anti-cheat games!)
    pub auto_games: bool,
    /// full exe paths that should run on the dedicated GPU while the boost is on
    pub gpu_apps: Vec<String>,
    pub rules: Vec<Rule>,
    pub close_to_tray: bool,
    pub hotkeys: bool,
    /// 0 = all threads
    pub extract_threads: usize,
    pub copy_threads: usize,
    pub overwrite: bool,
    pub defender_skip: bool,
    pub scan_after: bool,
    pub verify_copy: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            lang: "auto".into(),
            boost_on: false,
            measures: boost::default_measures(),
            background: boost::DEFAULT_BACKGROUND.into(),
            work: boost::DEFAULT_WORK.into(),
            games: String::new(),
            auto_games: false,
            gpu_apps: vec![],
            rules: rules::defaults(),
            close_to_tray: true,
            hotkeys: true,
            extract_threads: 0,
            copy_threads: 0,
            overwrite: false,
            defender_skip: false,
            scan_after: true,
            verify_copy: false,
        }
    }
}

impl Settings {
    pub fn has(&self, m: Measure) -> bool {
        self.measures.contains(&m)
    }

    pub fn set_measure(&mut self, m: Measure, on: bool) {
        self.measures.retain(|x| *x != m);
        if on {
            self.measures.push(m);
        }
    }

    pub fn encode(&self) -> String {
        let b = |v: bool| if v { "1" } else { "0" };
        let mut s = String::new();
        let mut kv = |k: &str, v: &str| {
            s.push_str(k);
            s.push('=');
            s.push_str(v);
            s.push('\n');
        };
        kv("lang", &self.lang);
        kv("boost_on", b(self.boost_on));
        // marks that the list was saved, so "all measures off" survives
        kv("measures_saved", "1");
        for m in &self.measures {
            kv("measure", m.key());
        }
        kv("background", &self.background);
        kv("work", &self.work);
        kv("games", &self.games);
        kv("auto_games", b(self.auto_games));
        for g in &self.gpu_apps {
            kv("gpu_app", g);
        }
        // marks that rules were saved, so "no rules" survives
        kv("rules_saved", "1");
        for r in &self.rules {
            kv("rule", &r.encode());
        }
        kv("close_to_tray", b(self.close_to_tray));
        kv("hotkeys", b(self.hotkeys));
        kv("extract_threads", &self.extract_threads.to_string());
        kv("copy_threads", &self.copy_threads.to_string());
        kv("overwrite", b(self.overwrite));
        kv("defender_skip", b(self.defender_skip));
        kv("scan_after", b(self.scan_after));
        kv("verify_copy", b(self.verify_copy));
        s
    }

    pub fn decode(text: &str) -> Settings {
        let mut s = Settings::default();
        let (mut measures, mut rules) = (vec![], vec![]);
        let (mut measures_saved, mut rules_saved) = (false, false);
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            let on = v.trim() == "1";
            match k.trim() {
                "lang" => s.lang = v.trim().to_string(),
                "boost_on" => s.boost_on = on,
                "measures_saved" => measures_saved = true,
                "measure" => measures.extend(Measure::parse(v)),
                "background" => s.background = v.trim().to_string(),
                "work" => s.work = v.trim().to_string(),
                "games" => s.games = v.trim().to_string(),
                "auto_games" => s.auto_games = on,
                "gpu_app" => s.gpu_apps.push(v.trim().to_string()),
                "rules_saved" => rules_saved = true,
                "rule" => rules.extend(Rule::decode(v)),
                "close_to_tray" => s.close_to_tray = on,
                "hotkeys" => s.hotkeys = on,
                "extract_threads" => s.extract_threads = v.trim().parse().unwrap_or(0),
                "copy_threads" => s.copy_threads = v.trim().parse().unwrap_or(0),
                "overwrite" => s.overwrite = on,
                "defender_skip" => s.defender_skip = on,
                "scan_after" => s.scan_after = on,
                "verify_copy" => s.verify_copy = on,
                _ => {}
            }
        }
        if measures_saved {
            s.measures = measures;
        }
        if rules_saved {
            s.rules = rules;
        }
        s
    }

    pub fn load() -> Settings {
        file().and_then(|f| std::fs::read_to_string(f).ok()).map(|t| Settings::decode(&t)).unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let f = file().ok_or("no APPDATA")?;
        if let Some(d) = f.parent() {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        std::fs::write(&f, self.encode()).map_err(|e| format!("{}: {e}", f.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let mut s = Settings::default();
        assert_eq!(Settings::decode(&s.encode()), s);
        s.measures.clear();
        s.rules.clear();
        s.gpu_apps = vec![r"C:\Games\x.exe".into()];
        s.boost_on = true;
        s.extract_threads = 8;
        assert_eq!(Settings::decode(&s.encode()), s);
        assert_eq!(Settings::decode(""), Settings::default());
    }
}
