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
    /// CR 702.51: Kreaturen tappen, um Mana zu bezahlen
    Convoke,
    /// CR 702.126: Artefakte tappen, um generisches Mana zu bezahlen
    Improvise,
    /// CR 702.66: Karten aus dem Friedhof exilieren, um generisches Mana zu bezahlen
    Delve,
    /// CR 702.41: kostet {1} weniger je bleibender Karte des Typs (`Affinity:Artifact`)
    Affinity(String),
    /// CR 702.33: `Kicker:<Kosten>[:<zweite Kosten>]` (Originaltext der Kosten)
    Kicker(Vec<String>),
    /// CR 702.37 / 702.168: Morph, Megamorph, Disguise – verdeckt wirkbar,
    /// für `cost` umdrehbar.
    Morph {
        cost: String,
        mega: bool,
        disguise: bool,
    },
    /// CR 702.14: Landwalk (Filter auf Länder des Verteidigers, Text des Filters)
    Landwalk(String),
    /// CR 702.36 Fear, 702.13 Intimidate, 702.28 Shadow, 702.31 Horsemanship, 702.118 Skulk
    Fear,
    Intimidate,
    Shadow,
    Horsemanship,
    Skulk,
    /// Deckbau-Keywords ohne Spielwirkung (Partner, Choose a Background,
    /// Doctor's companion; CR 702.124, 702.155, 702.163)
    DeckBuilding(String),
    /// CR 702.88: Rebound
    Rebound,
    /// CR 702.145: Daybound / Nightbound (Tag/Nacht, CR 726)
    Daybound,
    Nightbound,
    /// CR 702.179: „Start your engines!“ (Geschwindigkeit)
    StartYourEngines,
    /// „You may choose not to untap CARDNAME during your untap step.“ (CR 502.3)
    MayNotUntap,
    /// CR 702.93 Undying, 702.79 Persist, 702.45 Bushido n, 702.25 Flanking
    /// (als Trigger übersetzt, siehe `definition.rs`)
    Undying,
    Persist,
    Bushido(u32),
    Flanking,
    /// CR 702.83: Exalted (als Trigger übersetzt, siehe `definition.rs`)
    Exalted,
    /// CR 702.61: Split second – solange auf dem Stapel keine Zauber/Nicht-Mana-Fähigkeiten
    SplitSecond,
    /// CR 702.127: Aftermath (zweite Hälfte nur aus dem Friedhof, danach ins Exil)
    Aftermath,
    /// CR 702.90: Schaden als -1/-1-Marken bzw. Giftmarken
    Infect,
    /// CR 702.80: Schaden an Kreaturen als -1/-1-Marken
    Wither,
    /// CR 702.164: Kampfschaden an Spielern gibt zusätzlich n Giftmarken
    Toxic(u32),
    /// CR 702.16: Protection
    Protection(ProtectionFrom),
    /// `HIDDEN CARDNAME can't block.` (meist bis Zugende verliehen)
    CantBlock,
    /// `HIDDEN CARDNAME can't attack.`
    CantAttack,
    /// `HIDDEN CARDNAME can't attack or block.`
    CantAttackOrBlock,
    /// `HIDDEN This card doesn't untap during your next untap step.` – wird beim
    /// Verleihen als einmaliges Flag am Objekt umgesetzt (`CardObj::skip_next_untap`).
    NoUntapNext,
    /// Nicht (oder noch nicht) typisiertes Keyword, Originaltext.
    Other(String),
}

/// Wovor ein Objekt geschützt ist (CR 702.16a).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProtectionFrom {
    Color(forge_types::Color),
    /// „from each color“ / „from all colors“
    AllColors,
    /// „from everything“ (CR 702.16j)
    Everything,
    /// `Protection:<Filter>[:<Text>]` – Filter auf die Quelle (Text des Filters)
    Valid(String),
}

fn t_full(s: &str) -> String {
    s.trim().to_string()
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
            "HIDDEN CARDNAME can't block." | "CARDNAME can't block." => Keyword::CantBlock,
            "HIDDEN CARDNAME can't attack." | "CARDNAME can't attack." => Keyword::CantAttack,
            "HIDDEN CARDNAME can't attack or block." | "CARDNAME can't attack or block." => {
                Keyword::CantAttackOrBlock
            }
            "HIDDEN This card doesn't untap during your next untap step." => Keyword::NoUntapNext,
            "Convoke" => Keyword::Convoke,
            "Infect" => Keyword::Infect,
            "Aftermath" => Keyword::Aftermath,
            "Fear" => Keyword::Fear,
            "Intimidate" => Keyword::Intimidate,
            "Shadow" => Keyword::Shadow,
            "Horsemanship" => Keyword::Horsemanship,
            "Skulk" => Keyword::Skulk,
            "Exalted" => Keyword::Exalted,
            "Rebound" => Keyword::Rebound,
            "Undying" => Keyword::Undying,
            "Daybound" => Keyword::Daybound,
            "Nightbound" => Keyword::Nightbound,
            "Start your engines" | "Start your engines!" => Keyword::StartYourEngines,
            "You may choose not to untap CARDNAME during your untap step." => Keyword::MayNotUntap,
            "Persist" => Keyword::Persist,
            "Flanking" => Keyword::Flanking,
            t if t.starts_with("Bushido:") && t["Bushido:".len()..].parse::<u32>().is_ok() => {
                Keyword::Bushido(t["Bushido:".len()..].parse().unwrap_or(1))
            }
            "Partner" | "Choose a Background" | "Doctor's companion" => {
                Keyword::DeckBuilding(t_full(s))
            }
            t if t.starts_with("Partner:") => Keyword::DeckBuilding(t.to_string()),
            t if t.starts_with("Landwalk:")
                && crate::valid::ValidSpec::parse(
                    t["Landwalk:".len()..].split(':').next().unwrap_or(""),
                )
                .is_ok() =>
            {
                Keyword::Landwalk(
                    t["Landwalk:".len()..]
                        .split(':')
                        .next()
                        .unwrap_or("")
                        .to_string(),
                )
            }
            "Split second" => Keyword::SplitSecond,
            "Wither" => Keyword::Wither,
            t if t.starts_with("Toxic:") && t["Toxic:".len()..].parse::<u32>().is_ok() => {
                Keyword::Toxic(t["Toxic:".len()..].parse().unwrap_or(1))
            }
            "Improvise" => Keyword::Improvise,
            "Delve" => Keyword::Delve,
            t if (t.starts_with("Morph:")
                || t.starts_with("Megamorph:")
                || t.starts_with("Disguise:"))
                && t.split_once(':')
                    .is_some_and(|(_, c)| crate::cost::Cost::parse(c).is_ok()) =>
            {
                let (k, c) = t.split_once(':').unwrap_or_default();
                Keyword::Morph {
                    cost: c.to_string(),
                    mega: k == "Megamorph",
                    disguise: k == "Disguise",
                }
            }
            t if t.starts_with("Protection from ") => {
                let what = &t["Protection from ".len()..];
                match what {
                    "everything" => Keyword::Protection(ProtectionFrom::Everything),
                    "each color" | "all colors" => Keyword::Protection(ProtectionFrom::AllColors),
                    c => match forge_types::Color::from_name(c) {
                        Some(col) => Keyword::Protection(ProtectionFrom::Color(col)),
                        None => Keyword::Other(t.to_string()),
                    },
                }
            }
            t if t.starts_with("Protection:") => {
                let spec = t["Protection:".len()..].split(':').next().unwrap_or("");
                if !spec.contains("Chosen")
                    && !spec.contains("UID")
                    && crate::valid::ValidSpec::parse(spec).is_ok_and(|v| !v.allows_players())
                {
                    Keyword::Protection(ProtectionFrom::Valid(spec.to_string()))
                } else {
                    Keyword::Other(t.to_string())
                }
            }
            t if t.starts_with("Kicker:")
                && t["Kicker:".len()..]
                    .split(':')
                    .all(|c| crate::cost::Cost::parse(c).is_ok()) =>
            {
                Keyword::Kicker(
                    t["Kicker:".len()..]
                        .split(':')
                        .map(str::to_string)
                        .collect(),
                )
            }
            t if t.starts_with("Affinity:")
                && crate::valid::ValidSpec::parse(&t["Affinity:".len()..]).is_ok() =>
            {
                Keyword::Affinity(t["Affinity:".len()..].to_string())
            }
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
