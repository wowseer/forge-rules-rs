//! Magic-Farben (CR 105) als Bitmaske.
//!
//! Java-Vorbild: `forge.card.MagicColor` und `forge.card.ColorSet` (forge-core).

use std::fmt;

/// Eine der fünf Farben. Farblos ist **keine** Farbe (CR 105.2c) und wird
/// durch ein leeres [`ColorSet`] dargestellt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Color {
    White,
    Blue,
    Black,
    Red,
    Green,
}

impl Color {
    /// WUBRG-Reihenfolge.
    pub const ALL: [Color; 5] = [
        Color::White,
        Color::Blue,
        Color::Black,
        Color::Red,
        Color::Green,
    ];

    /// Bit dieser Farbe im [`ColorSet`] (identisch zu `MagicColor` in Java).
    pub const fn bit(self) -> u8 {
        match self {
            Color::White => 1,
            Color::Blue => 2,
            Color::Black => 4,
            Color::Red => 8,
            Color::Green => 16,
        }
    }

    /// Manasymbol (`W`, `U`, `B`, `R`, `G`).
    pub const fn symbol(self) -> char {
        match self {
            Color::White => 'W',
            Color::Blue => 'U',
            Color::Black => 'B',
            Color::Red => 'R',
            Color::Green => 'G',
        }
    }

    pub fn from_symbol(c: char) -> Option<Color> {
        match c.to_ascii_uppercase() {
            'W' => Some(Color::White),
            'U' => Some(Color::Blue),
            'B' => Some(Color::Black),
            'R' => Some(Color::Red),
            'G' => Some(Color::Green),
            _ => None,
        }
    }

    /// Englischer Name, wie er in Skripten vorkommt (`Colors:red,green`).
    pub fn from_name(s: &str) -> Option<Color> {
        match s.trim().to_ascii_lowercase().as_str() {
            "white" | "w" => Some(Color::White),
            "blue" | "u" => Some(Color::Blue),
            "black" | "b" => Some(Color::Black),
            "red" | "r" => Some(Color::Red),
            "green" | "g" => Some(Color::Green),
            _ => None,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Color::White => "white",
            Color::Blue => "blue",
            Color::Black => "black",
            Color::Red => "red",
            Color::Green => "green",
        }
    }
}

/// Menge von Farben. Leere Menge = farblos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ColorSet(u8);

impl ColorSet {
    pub const COLORLESS: ColorSet = ColorSet(0);
    pub const ALL: ColorSet = ColorSet(0b11111);

    pub const fn from_bits(bits: u8) -> ColorSet {
        ColorSet(bits & 0b11111)
    }

    pub const fn bits(self) -> u8 {
        self.0
    }

    pub const fn of(c: Color) -> ColorSet {
        ColorSet(c.bit())
    }

    /// Parst eine komma-separierte Liste von Farbnamen (`Colors:`-Zeile).
    /// `colorless` ergibt die leere Menge; `multicolor`/`all` alle Farben.
    pub fn from_names(s: &str) -> Option<ColorSet> {
        let mut set = ColorSet::COLORLESS;
        for part in s.split(',') {
            let p = part.trim();
            if p.eq_ignore_ascii_case("colorless") {
                continue;
            }
            if p.eq_ignore_ascii_case("multicolor") || p.eq_ignore_ascii_case("all") {
                set = ColorSet::ALL;
                continue;
            }
            set = set.with(Color::from_name(p)?);
        }
        Some(set)
    }

    pub const fn with(self, c: Color) -> ColorSet {
        ColorSet(self.0 | c.bit())
    }

    pub const fn union(self, other: ColorSet) -> ColorSet {
        ColorSet(self.0 | other.0)
    }

    pub const fn intersection(self, other: ColorSet) -> ColorSet {
        ColorSet(self.0 & other.0)
    }

    pub const fn contains(self, c: Color) -> bool {
        self.0 & c.bit() != 0
    }

    pub const fn is_subset_of(self, other: ColorSet) -> bool {
        self.0 & !other.0 == 0
    }

    pub const fn is_colorless(self) -> bool {
        self.0 == 0
    }

    pub const fn count(self) -> u32 {
        self.0.count_ones()
    }

    /// CR 105.4: mehrfarbig = zwei oder mehr Farben.
    pub const fn is_multicolor(self) -> bool {
        self.count() > 1
    }

    pub const fn is_mono(self) -> bool {
        self.count() == 1
    }

    pub fn iter(self) -> impl Iterator<Item = Color> {
        Color::ALL.into_iter().filter(move |c| self.contains(*c))
    }
}

impl FromIterator<Color> for ColorSet {
    fn from_iter<I: IntoIterator<Item = Color>>(iter: I) -> Self {
        iter.into_iter().fold(ColorSet::COLORLESS, ColorSet::with)
    }
}

impl fmt::Display for ColorSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_colorless() {
            return f.write_str("C");
        }
        for c in self.iter() {
            write!(f, "{}", c.symbol())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_roundtrip() {
        let s = ColorSet::from_names("red,green").unwrap();
        assert!(s.contains(Color::Red) && s.contains(Color::Green));
        assert_eq!(s.count(), 2);
        assert!(s.is_multicolor());
        assert_eq!(s.to_string(), "RG");
        assert_eq!(ColorSet::from_names("colorless"), Some(ColorSet::COLORLESS));
        assert_eq!(ColorSet::from_names("purple"), None);
    }

    #[test]
    fn subset() {
        let rg = ColorSet::from_names("red,green").unwrap();
        assert!(ColorSet::of(Color::Red).is_subset_of(rg));
        assert!(!ColorSet::of(Color::Blue).is_subset_of(rg));
        assert!(ColorSet::COLORLESS.is_subset_of(rg));
    }
}
