//! Editionen (`res/editions/*.txt`) und Formate (`res/formats/**.txt`).
//!
//! Java-Vorbild: `forge.card.CardEdition`, `forge.game.GameFormat.buildFilter`.
//!
//! Legalität einer Karte in einem Format (wie Java):
//! - nicht auf der `Banned`-Liste,
//! - keine Alchemy-Rebalanced-Karte (`A-…`),
//! - falls `Sets` gesetzt: in mindestens einer dieser Editionen gedruckt,
//! - falls `Rarities` gesetzt: in mindestens einer dieser Seltenheiten gedruckt,
//! - oder auf der `Additional`-Liste.
//!
//! Zusätzlich (Projekt-Scope, siehe CLAUDE.md): Formate ohne `Sets`
//! (Commander, Pauper) berücksichtigen nur Drucke aus Papier-Editionen –
//! `Funny` (Un-Sets) und `Online` (Alchemy/digital) zählen nicht.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edition {
    pub code: String,
    pub name: String,
    pub kind: String,
    pub date: String,
    /// (Seltenheit, Kartenname)
    pub cards: Vec<(char, String)>,
}

impl Edition {
    pub fn parse(text: &str) -> Option<Edition> {
        let mut e = Edition {
            code: String::new(),
            name: String::new(),
            kind: String::new(),
            date: String::new(),
            cards: Vec::new(),
        };
        let mut section = String::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(s) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                section = s.to_ascii_lowercase();
                continue;
            }
            if section == "metadata" {
                if let Some((k, v)) = line.split_once('=') {
                    match k.trim() {
                        "Code" => e.code = v.trim().into(),
                        "Name" => e.name = v.trim().into(),
                        "Type" => e.kind = v.trim().into(),
                        "Date" => e.date = v.trim().into(),
                        _ => {}
                    }
                }
                continue;
            }
            if matches!(
                section.as_str(),
                "tokens" | "other" | "rebalanced" | "conjured"
            ) {
                continue;
            }
            // `<Nummer> <Seltenheit> <Name> [@Künstler] [$…]`
            let mut parts = line.splitn(3, ' ');
            let (Some(_num), Some(r), Some(rest)) = (parts.next(), parts.next(), parts.next())
            else {
                continue;
            };
            if r.len() != 1 || !"CURMSLTX".contains(r) {
                continue;
            }
            let end = rest
                .find(" @")
                .or_else(|| rest.find(" $"))
                .unwrap_or(rest.len());
            let name = rest[..end].trim();
            if !name.is_empty() {
                e.cards
                    .push((r.chars().next().unwrap_or('C'), name.to_string()));
            }
        }
        (!e.code.is_empty()).then_some(e)
    }

    pub fn is_paper(&self) -> bool {
        !matches!(self.kind.as_str(), "Funny" | "Online")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Format {
    pub name: String,
    pub sets: Vec<String>,
    pub banned: BTreeSet<String>,
    pub restricted: BTreeSet<String>,
    pub rarities: Vec<char>,
    pub additional: BTreeSet<String>,
}

fn name_list(v: &str) -> BTreeSet<String> {
    v.split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

impl Format {
    pub fn parse(text: &str) -> Format {
        let mut f = Format::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once(':') else {
                continue;
            };
            match k.trim() {
                "Name" => f.name = v.trim().into(),
                "Sets" => {
                    f.sets = v
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                }
                "Banned" => f.banned = name_list(v),
                "Restricted" => f.restricted = name_list(v),
                "Additional" => f.additional = name_list(v),
                "Rarities" => {
                    f.rarities = v
                        .split(',')
                        .filter_map(|s| s.trim().chars().next())
                        .collect()
                }
                _ => {}
            }
        }
        f
    }
}

/// Die Zielformate des Projekts (siehe CLAUDE.md) als Dateipfade relativ zu `res/formats`.
pub const TARGET_FORMATS: &[&str] = &[
    "Sanctioned/Standard.txt",
    "Sanctioned/Pioneer.txt",
    "Sanctioned/Modern.txt",
    "Sanctioned/Legacy.txt",
    "Sanctioned/Vintage.txt",
    "Sanctioned/Pauper.txt",
    "Casual/Commander.txt",
];

/// Druckinformationen aller Karten.
#[derive(Debug, Clone, Default)]
pub struct Printings {
    /// Kartenname → (Editionscode, Seltenheit, Papier?)
    pub by_card: HashMap<String, Vec<(String, char, bool)>>,
}

impl Printings {
    pub fn load(res: &Path) -> std::io::Result<Printings> {
        let mut p = Printings::default();
        let mut files: Vec<_> = std::fs::read_dir(res.join("editions"))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .collect();
        files.sort();
        for f in files {
            let text = String::from_utf8_lossy(&std::fs::read(&f)?).into_owned();
            if let Some(e) = Edition::parse(&text) {
                let paper = e.is_paper();
                for (r, name) in e.cards {
                    p.by_card
                        .entry(name)
                        .or_default()
                        .push((e.code.clone(), r, paper));
                }
            }
        }
        Ok(p)
    }

    pub fn is_legal(&self, card: &str, f: &Format) -> bool {
        if f.additional.contains(card) {
            return true;
        }
        if f.banned.contains(card) || card.starts_with("A-") {
            return false;
        }
        let Some(prints) = self.by_card.get(card) else {
            return false;
        };
        let in_sets = f.sets.is_empty() && prints.iter().any(|x| x.2)
            || prints.iter().any(|x| f.sets.contains(&x.0));
        let rarity_ok =
            f.rarities.is_empty() || prints.iter().any(|x| x.2 && f.rarities.contains(&x.1));
        in_sets && rarity_ok
    }
}

/// Kartenname → Liste der Zielformate, in denen er legal ist.
pub fn target_pool(res: &Path) -> std::io::Result<BTreeMap<String, Vec<String>>> {
    let printings = Printings::load(res)?;
    let formats: Vec<Format> = TARGET_FORMATS
        .iter()
        .map(|p| std::fs::read_to_string(res.join("formats").join(p)).map(|t| Format::parse(&t)))
        .collect::<std::io::Result<_>>()?;
    let mut pool: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for name in printings.by_card.keys() {
        for f in &formats {
            if printings.is_legal(name, f) {
                pool.entry(name.clone()).or_default().push(f.name.clone());
            }
        }
    }
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edition_and_format() {
        let e = Edition::parse("[metadata]\nCode=TST\nType=Core\n[cards]\n1 C Grizzly Bears @A\n2 R Black Lotus $x\n[tokens]\n3 C bear_token\n").unwrap();
        assert_eq!(
            e.cards,
            vec![('C', "Grizzly Bears".into()), ('R', "Black Lotus".into())]
        );
        let f = Format::parse("Name:Test\nSets:TST\nBanned:Black Lotus; Foo\nRarities:C\n");
        let mut p = Printings::default();
        for (r, n) in &e.cards {
            p.by_card
                .entry(n.clone())
                .or_default()
                .push(("TST".into(), *r, true));
        }
        assert!(p.is_legal("Grizzly Bears", &f));
        assert!(!p.is_legal("Black Lotus", &f));
    }
}
