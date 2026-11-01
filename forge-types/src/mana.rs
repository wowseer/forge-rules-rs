//! Manakosten und Manasymbole (CR 107.4, CR 202).
//!
//! Java-Vorbild: `forge.card.mana.ManaCost`, `ManaCostShard`, `ManaAtom`,
//! `ManaCostParser` (forge-core).
//!
//! Skriptformat (`ManaCost:`-Zeile): durch Leerzeichen getrennte Token.
//! Ganzzahlen sind generisches Mana (`2`, `10`), alle anderen Token sind
//! Symbole. Hybrid wird ohne Schrägstrich geschrieben (`WU`, `2R`, `BP`,
//! `RWP`, `CG`), in Fähigkeitskosten auch mit (`W/U`, `2/R`). `no cost`
//! bedeutet „hat keine Manakosten“ (z. B. Länder, Ancestral Vision) und ist
//! etwas anderes als `{0}`.

use crate::color::{Color, ColorSet};
use std::fmt;

/// Ein einzelnes nicht-generisches Manasymbol.
///
/// Statt der 40 Java-Enum-Konstanten wird das Symbol aus seinen
/// Bestandteilen („Atomen“) zusammengesetzt, genau wie `ManaAtom` es tut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ManaShard {
    /// Farben, mit denen das Symbol bezahlt werden kann.
    pub colors: ColorSet,
    /// `{C}` bzw. farbloser Anteil eines `{C/W}`-Hybrids.
    pub colorless: bool,
    /// `{2/W}`: alternativ mit zwei generischem Mana bezahlbar (CR 107.4e).
    pub or_two_generic: bool,
    /// Phyrexianisch: alternativ mit 2 Leben bezahlbar (CR 107.4f).
    pub phyrexian: bool,
    /// `{S}`: mit Mana aus einer Snow-Quelle zu bezahlen (CR 107.4h).
    pub snow: bool,
    /// `{X}` (CR 107.3).
    pub x: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManaParseError {
    UnknownSymbol(String),
}

impl fmt::Display for ManaParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManaParseError::UnknownSymbol(s) => write!(f, "unbekanntes Manasymbol '{s}'"),
        }
    }
}

impl std::error::Error for ManaParseError {}

impl ManaShard {
    const EMPTY: ManaShard = ManaShard {
        colors: ColorSet::COLORLESS,
        colorless: false,
        or_two_generic: false,
        phyrexian: false,
        snow: false,
        x: false,
    };

    pub const fn colored(c: Color) -> ManaShard {
        ManaShard {
            colors: ColorSet::of(c),
            ..ManaShard::EMPTY
        }
    }

    pub const COLORLESS: ManaShard = ManaShard {
        colorless: true,
        ..ManaShard::EMPTY
    };
    pub const SNOW: ManaShard = ManaShard {
        snow: true,
        ..ManaShard::EMPTY
    };
    pub const X: ManaShard = ManaShard {
        x: true,
        ..ManaShard::EMPTY
    };

    /// Parst ein Symbol ohne geschweifte Klammern, z. B. `W`, `WU`, `W/U`,
    /// `2/R`, `BP`, `B/G/P`, `CW`, `S`, `X`.
    pub fn parse(token: &str) -> Result<ManaShard, ManaParseError> {
        let t = token.trim().trim_start_matches('{').trim_end_matches('}');
        let mut s = ManaShard::EMPTY;
        let mut any = false;
        for ch in t.chars() {
            any = true;
            match ch.to_ascii_uppercase() {
                '/' => {}
                'P' => s.phyrexian = true,
                'S' => s.snow = true,
                'X' => s.x = true,
                'C' => s.colorless = true,
                '2' => s.or_two_generic = true,
                c => match Color::from_symbol(c) {
                    Some(col) => s.colors = s.colors.with(col),
                    None => return Err(ManaParseError::UnknownSymbol(token.to_string())),
                },
            }
        }
        let valid = any
            && match () {
                _ if s.x => s.colors.is_colorless() && !s.snow && !s.phyrexian && !s.colorless,
                _ if s.snow => {
                    s.colors.is_colorless() && !s.phyrexian && !s.colorless && !s.or_two_generic
                }
                _ if s.or_two_generic => s.colors.is_mono() && !s.phyrexian && !s.colorless,
                _ if s.colorless => s.colors.count() <= 1 && !s.phyrexian,
                _ if s.phyrexian => (1..=2).contains(&s.colors.count()),
                _ => (1..=2).contains(&s.colors.count()),
            };
        if valid {
            Ok(s)
        } else {
            Err(ManaParseError::UnknownSymbol(token.to_string()))
        }
    }

    /// Hybrid-Symbol (zwei Bezahloptionen, CR 107.4e). Phyrexianisch allein
    /// zählt nicht als Hybrid.
    pub fn is_hybrid(&self) -> bool {
        self.colors.count() == 2
            || self.or_two_generic
            || (self.colorless && !self.colors.is_colorless())
    }

    /// Beitrag zum Manawert (CR 202.3): `{2/W}` zählt 2, `{X}` 0, sonst 1.
    pub fn mana_value(&self) -> u32 {
        if self.x {
            0
        } else if self.or_two_generic {
            2
        } else {
            1
        }
    }
}

impl fmt::Display for ManaShard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts: Vec<String> = Vec::new();
        if self.x {
            parts.push("X".into());
        }
        if self.snow {
            parts.push("S".into());
        }
        if self.or_two_generic {
            parts.push("2".into());
        }
        if self.colorless {
            parts.push("C".into());
        }
        for c in self.colors.iter() {
            parts.push(c.symbol().to_string());
        }
        if self.phyrexian {
            parts.push("P".into());
        }
        write!(f, "{{{}}}", parts.join("/"))
    }
}

/// Vollständige Manakosten eines Objekts.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ManaCost {
    /// `true` für „no cost“ (CR 202.1b: Objekt hat keine Manakosten).
    pub no_cost: bool,
    pub generic: u32,
    pub shards: Vec<ManaShard>,
}

impl ManaCost {
    pub fn no_cost() -> ManaCost {
        ManaCost {
            no_cost: true,
            ..ManaCost::default()
        }
    }

    pub fn generic(n: u32) -> ManaCost {
        ManaCost {
            generic: n,
            ..ManaCost::default()
        }
    }

    /// Parst eine `ManaCost:`-Zeile oder eine Kostenangabe in Fähigkeiten.
    /// Akzeptiert sowohl `2 W U` als auch `{2}{W}{U}`.
    pub fn parse(s: &str) -> Result<ManaCost, ManaParseError> {
        let s = s.trim();
        if s.eq_ignore_ascii_case("no cost") {
            return Ok(ManaCost::no_cost());
        }
        let normalized = s.replace(['}', '{'], " ");
        let mut cost = ManaCost::default();
        for tok in normalized.split_whitespace() {
            if let Ok(n) = tok.parse::<u32>() {
                cost.generic += n;
            } else {
                cost.shards.push(ManaShard::parse(tok)?);
            }
        }
        Ok(cost)
    }

    /// Manawert (CR 202.3). X zählt außerhalb des Stapels 0.
    pub fn mana_value(&self) -> u32 {
        if self.no_cost {
            return 0;
        }
        self.generic + self.shards.iter().map(ManaShard::mana_value).sum::<u32>()
    }

    /// Farben aus den Manasymbolen (CR 202.2).
    pub fn colors(&self) -> ColorSet {
        self.shards
            .iter()
            .fold(ColorSet::COLORLESS, |acc, s| acc.union(s.colors))
    }

    pub fn x_count(&self) -> usize {
        self.shards.iter().filter(|s| s.x).count()
    }

    /// Anzahl Symbole, die (auch) mit der Farbe bezahlt werden können
    /// (Devotion, CR 700.5).
    pub fn devotion_to(&self, colors: ColorSet) -> u32 {
        self.shards
            .iter()
            .filter(|s| !s.colors.intersection(colors).is_colorless())
            .count() as u32
    }

    /// `{0}` – hat Manakosten, aber Manawert 0.
    pub fn is_zero(&self) -> bool {
        !self.no_cost && self.generic == 0 && self.shards.is_empty()
    }
}

impl fmt::Display for ManaCost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.no_cost {
            return f.write_str("no cost");
        }
        for s in self.shards.iter().filter(|s| s.x) {
            write!(f, "{s}")?;
        }
        if self.generic > 0 || self.shards.is_empty() {
            write!(f, "{{{}}}", self.generic)?;
        }
        for s in self.shards.iter().filter(|s| !s.x) {
            write!(f, "{s}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple() {
        let c = ManaCost::parse("3 W W").unwrap();
        assert_eq!(c.generic, 3);
        assert_eq!(c.mana_value(), 5);
        assert_eq!(c.to_string(), "{3}{W}{W}");
        assert_eq!(c.colors(), ColorSet::of(Color::White));
    }

    #[test]
    fn hybrid_and_phyrexian() {
        let c = ManaCost::parse("2R 2R 2R").unwrap();
        assert_eq!(c.mana_value(), 6);
        assert!(c.shards[0].is_hybrid());
        let p = ManaCost::parse("1 BP").unwrap();
        assert!(p.shards[0].phyrexian);
        assert_eq!(p.mana_value(), 2);
        let h = ManaCost::parse("RWP").unwrap();
        assert_eq!(h.shards[0].colors.count(), 2);
        assert!(h.shards[0].phyrexian);
        assert_eq!(
            ManaCost::parse("W/U").unwrap(),
            ManaCost::parse("WU").unwrap()
        );
        assert_eq!(
            ManaCost::parse("{2}{G}").unwrap(),
            ManaCost::parse("2 G").unwrap()
        );
    }

    #[test]
    fn x_and_specials() {
        let c = ManaCost::parse("X X G").unwrap();
        assert_eq!(c.x_count(), 2);
        assert_eq!(c.mana_value(), 1);
        assert_eq!(c.to_string(), "{X}{X}{G}");
        assert!(ManaCost::parse("no cost").unwrap().no_cost);
        assert!(ManaCost::parse("0").unwrap().is_zero());
        assert!(ManaCost::parse("S").unwrap().shards[0].snow);
        assert!(ManaCost::parse("CW").unwrap().shards[0].is_hybrid());
        assert!(ManaCost::parse("Q").is_err());
        assert!(ManaCost::parse("WUB").is_err());
    }

    #[test]
    fn devotion() {
        let c = ManaCost::parse("1 B G BG").unwrap();
        assert_eq!(c.devotion_to(ColorSet::of(Color::Black)), 2);
        assert_eq!(
            c.devotion_to(ColorSet::from_names("black,green").unwrap()),
            3
        );
    }
}
