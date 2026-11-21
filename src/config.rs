//! 設定ファイル。`key = value` の行と、`[種類 名前]` のセクション。`#` 以降はコメント。

use crate::util::{Result, ZError, proto};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Default, Clone)]
pub struct Section {
    pub kind: String,
    pub name: String,
    pub kv: BTreeMap<String, String>,
}

impl Section {
    pub fn get(&self, k: &str) -> Option<&str> {
        self.kv.get(k).map(|s| s.as_str())
    }
    pub fn req(&self, k: &str) -> Result<&str> {
        match self.get(k) {
            Some(v) => Ok(v),
            None => proto(format!("設定 [{} {}] に {k} がない", self.kind, self.name)),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Conf {
    /// セクションより前の設定
    pub global: Section,
    pub sections: Vec<Section>,
}

impl Conf {
    pub fn parse(text: &str) -> Result<Conf> {
        let mut conf = Conf::default();
        conf.global.kind = "global".into();
        let mut cur: Option<Section> = None;
        for (i, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap().trim();
            if line.is_empty() {
                continue;
            }
            if let Some(h) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                if let Some(s) = cur.take() {
                    conf.sections.push(s);
                }
                let mut it = h.split_whitespace();
                let (Some(kind), Some(name), None) = (it.next(), it.next(), it.next()) else {
                    return proto(format!("{}行目: セクションは [種類 名前]", i + 1));
                };
                cur = Some(Section {
                    kind: kind.into(),
                    name: name.into(),
                    kv: BTreeMap::new(),
                });
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                return proto(format!("{}行目: key = value の形式でない", i + 1));
            };
            let target = cur.as_mut().unwrap_or(&mut conf.global);
            target.kv.insert(k.trim().into(), v.trim().into());
        }
        if let Some(s) = cur.take() {
            conf.sections.push(s);
        }
        Ok(conf)
    }

    pub fn load(path: &Path) -> Result<Conf> {
        let text = std::fs::read_to_string(path).map_err(ZError::Io)?;
        Conf::parse(&text)
    }

    pub fn of_kind<'a>(&'a self, kind: &'a str) -> impl Iterator<Item = &'a Section> {
        self.sections.iter().filter(move |s| s.kind == kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sections_and_comments() {
        let c = Conf::parse("a = 1 # c\n\n[bank 0002]\ncenter=x\n[company ACME]\nkey = k\n").unwrap();
        assert_eq!(c.global.get("a"), Some("1"));
        assert_eq!(c.sections.len(), 2);
        assert_eq!(c.of_kind("bank").next().unwrap().name, "0002");
        assert_eq!(c.of_kind("company").next().unwrap().req("key").unwrap(), "k");
        assert!(c.of_kind("bank").next().unwrap().req("nope").is_err());
    }

    #[test]
    fn rejects_malformed() {
        assert!(Conf::parse("novalue").is_err());
        assert!(Conf::parse("[bank]").is_err());
    }
}
