//! Schlüsselwörter (CR 702) aus `K:`-Zeilen.
//!
//! Java-Vorbild: `forge.game.keyword.Keyword` + `CardFactoryUtil` (dort werden
//! viele Keywords in Trigger/Statics/Replacements übersetzt).
//!
//! Hier sind zunächst die **statischen Kampf-/Schutz-Keywords** typisiert, die
//! die Engine direkt abfragt. Alle anderen bleiben als [`Keyword::Other`]
//! erhalten und gelten als „noch nicht unterstützt“ (siehe
//! `CardDefinition::unsupported`).

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Keyword {
    Flying,
    Reach,
    Vigilance,
    Trample,
    Haste,
    Flash,
    Menace,
    FirstStrike,
    DoubleStrike,
    Lifelink,
    Deathtouch,
    Defender,
    Indestructible,
    Hexproof,
    Shroud,
    /// CR 702.114: farblos (Farb-CDA, wirkt in allen Zonen; in `chars()` umgesetzt)
    Devoid,
    /// CR 702.73: jeder Kreaturtyp (CDA, alle Zonen; in der Filterprüfung umgesetzt)
    Changeling,
    /// Nicht (oder noch nicht) typisiertes Keyword, Originaltext.
    Other(String),
}

impl Keyword {
    pub fn parse(s: &str) -> Keyword {
        match s.trim() {
            "Flying" => Keyword::Flying,
            "Reach" => Keyword::Reach,
            "Vigilance" => Keyword::Vigilance,
            "Trample" => Keyword::Trample,
            "Haste" => Keyword::Haste,
            "Flash" => Keyword::Flash,
            "Menace" => Keyword::Menace,
            "First Strike" => Keyword::FirstStrike,
            "Double Strike" => Keyword::DoubleStrike,
            "Lifelink" => Keyword::Lifelink,
            "Deathtouch" => Keyword::Deathtouch,
            "Defender" => Keyword::Defender,
            "Indestructible" => Keyword::Indestructible,
            "Hexproof" => Keyword::Hexproof,
            "Shroud" => Keyword::Shroud,
            "Devoid" => Keyword::Devoid,
            "Changeling" => Keyword::Changeling,
            other => Keyword::Other(other.to_string()),
        }
    }

    /// Wird von der Engine vollständig umgesetzt.
    pub fn is_supported(&self) -> bool {
        !matches!(self, Keyword::Other(_))
    }

    /// Parst die `KW$`-Liste eines Pump-Effekts (`Flying & Trample`).
    pub fn parse_list(s: &str) -> Vec<Keyword> {
        s.split('&')
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .map(Keyword::parse)
            .collect()
    }
}
