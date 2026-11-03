//! Filter-Ausdrücke wie `Creature.nonBlack+YouCtrl,Planeswalker` (`ValidTgts$`,
//! `ValidCards$`, `Valid…$`).
//!
//! Java-Vorbild: `CardTraitBase.isValid` → `CardType`/`CardProperty.cardHasProperty`
//! bzw. `PlayerProperty.playerHasProperty`.
//!
//! Syntax: Komma = ODER zwischen Alternativen. Eine Alternative ist
//! `Basis[.Eigenschaft[+Eigenschaft…]]`. Basis ist ein Typ (`Creature`,
//! `Card`, `Permanent`, Untertyp …) oder eine Spielerangabe (`Player`,
//! `Opponent`, `You`). `Any` steht für Kreatur, Spieler, Planeswalker, Battle.
//!
//! Nicht unterstützte Eigenschaften führen zu [`ValidError::Unsupported`] –
//! die Karte gilt dann als noch nicht spielbar, statt still falsch zu laufen.

use crate::keyword::Keyword;
use forge_types::{Color, CoreType};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardBase {
    /// `Card` / `Permanent` (letzteres nur auf dem Battlefield).
    Card,
    Permanent,
    Core(CoreType),
    /// Untertyp oder Supertyp-Wort wie `Legendary`, `Basic`.
    Word(String),
    /// Emblem (CR 114): Effektobjekt, dessen Name mit „Emblem“ beginnt (Java `isEmblem`).
    Emblem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerBase {
    Player,
    Opponent,
    You,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prop {
    YouCtrl,
    YouDontCtrl,
    OppCtrl,
    YouOwn,
    Other,
    SelfCard,
    Tapped,
    Untapped,
    Attacking,
    Blocking,
    Token,
    NonToken,
    Is(CardBase),
    Non(CardBase),
    Color(Color),
    NonColor(Color),
    Multicolor,
    Colorless,
    NonColorless,
    With(Keyword),
    Without(Keyword),
    /// Spieler-Eigenschaft `Opponent` in `Player.Opponent`.
    PlayerOpponent,
    /// `!<Eigenschaft>`
    Not(Box<Prop>),
    /// Gegner besitzt die Karte
    OppOwn,
    /// Von der Quelle gemerkt (`IsRemembered`)
    IsRemembered,
    /// `inZone<Zone>`: liegt in dieser Zone
    InZone(crate::zone::Zone),
    /// `TopLibrary`: oberste Karte der Bibliothek ihres Besitzers
    TopLibrary,
    /// `ChosenType`: hat den von der Quelle gewählten Typ
    ChosenType,
    /// `ChosenColor`: hat die von der Quelle gewählte Farbe
    ChosenColor,
    /// Von der Quelle gewählt (`ChosenCard`; `strict`: genau dieses Objekt)
    ChosenCard {
        strict: bool,
    },
    /// Controller ist ein von der Quelle gemerkter Spieler (`RememberedPlayerCtrl`)
    RememberedPlayerCtrl,
    /// Die Quelle (Aura/Ausrüstung) ist an dieses Objekt angelegt.
    AttachedBySource,
    /// `counters_GE1_P1P1`: Anzahl Marken einer Art (Java `CardProperty` „counters“)
    Counters {
        kind: String,
        op: crate::expr::CmpOp,
        value: i32,
    },
    /// `HasCounters`: mindestens eine Marke irgendeiner Art
    HasCounters,
    /// `IsMonstrous`/`monstrous` (CR 701.37b)
    Monstrous,
    /// Spieler: der von der Quelle gewählte Spieler (`Player.Chosen`)
    PlayerChosen,
    /// `ChosenCtrl`: vom gewählten Spieler der Quelle kontrolliert
    ChosenCtrl,
    /// `Historic` (CR 700.6): Artefakt, legendär oder Saga
    Historic,
    /// `IsCommander` (CR 903.3)
    IsCommander,
    /// `ExiledWithSource`: von der Quelle ins Exil geschickt (Java `exiledWith`)
    ExiledWithSource,
    /// `kicked` / `kicked 1` / `kicked 2` (CR 702.33d)
    Kicked(Option<u8>),
    /// `wasCast[From<Zone>|FromYourHand]` (Java `CardProperty` „wasCast“)
    WasCast {
        from: Option<crate::zone::Zone>,
    },
    /// Spieler: Controller des Objekts, an dem die Quelle hängt (`EnchantedController`)
    EnchantedController,
    /// Karte: eine Aura (`enchanted`) bzw. Ausrüstung (`equipped`) hängt daran
    Enchanted,
    Equipped,
    /// `cmcLEX`, `powerLTY`: Vergleich mit einer SVar der Quelle (Java
    /// `AbilityUtils.calculateAmount(source, …)`), beim Prüfen ausgewertet.
    StatVar {
        stat: Stat,
        op: crate::expr::CmpOp,
        var: String,
    },
    /// `powerGE4`, `toughnessLT2`, `cmcLE3` (Manawert)
    Stat {
        stat: Stat,
        op: crate::expr::CmpOp,
        value: i32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stat {
    Power,
    Toughness,
    ManaValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Alternative {
    Card { base: CardBase, props: Vec<Prop> },
    Player { base: PlayerBase, props: Vec<Prop> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidSpec {
    pub alternatives: Vec<Alternative>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidError {
    Unsupported(String),
}

impl fmt::Display for ValidError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidError::Unsupported(s) => write!(f, "Filter nicht unterstützt: {s}"),
        }
    }
}

fn card_base(s: &str) -> CardBase {
    match s {
        "Card" => CardBase::Card,
        "Permanent" => CardBase::Permanent,
        "Emblem" => CardBase::Emblem,
        other => match CoreType::from_name(other) {
            Some(c) => CardBase::Core(c),
            None => CardBase::Word(other.to_string()),
        },
    }
}

fn is_word_base(s: &str) -> bool {
    !s.is_empty()
        && s.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        && s.chars()
            .all(|c| c.is_alphanumeric() || c == '\'' || c == '-')
}

/// Bekannte Untertypen (aus `TypeLists.txt`), gesetzt von `CardDb::load`.
/// Ohne Registry (Unit-Tests) gelten alle großgeschriebenen Wörter als Typ.
static TYPE_REGISTRY: std::sync::OnceLock<forge_types::TypeRegistry> = std::sync::OnceLock::new();

pub fn set_type_registry(reg: forge_types::TypeRegistry) {
    let _ = TYPE_REGISTRY.set(reg);
}

/// Ist `w` ein Kartentyp, Supertyp oder bekannter Untertyp?
fn is_type_word(w: &str) -> bool {
    if !is_word_base(w) {
        return false;
    }
    if w == "Card"
        || w == "Permanent"
        || CoreType::from_name(w).is_some()
        || forge_types::Supertype::from_name(w).is_some()
    {
        return true;
    }
    match TYPE_REGISTRY.get() {
        Some(reg) => reg.kind_of(w).is_some(),
        None => true,
    }
}

/// SVar-Name als Vergleichswert in Filtern (`X`, `Y`, `Z`, `MaxCMC` …):
/// beginnt mit einem Großbuchstaben, nur Buchstaben/Ziffern.
pub fn is_filter_var(v: &str) -> bool {
    v.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        && v.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Alle Variablennamen in Filter-Eigenschaften eines Textes (`cmcLEX` → `X`).
pub fn filter_vars_in(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for word in text.split(|c: char| !c.is_ascii_alphanumeric()) {
        for prefix in ["cmc", "power", "toughness"] {
            if let Some(rest) = word.strip_prefix(prefix)
                && rest.len() > 2
                && cmp_op(&rest[..2]).is_some()
                && rest[2..].parse::<i32>().is_err()
                && is_filter_var(&rest[2..])
            {
                out.push(rest[2..].to_string());
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Vergleichsoperator in Filtern (`GE`, `LT` …).
fn cmp_op(s: &str) -> Option<crate::expr::CmpOp> {
    use crate::expr::CmpOp;
    Some(match s {
        "LT" => CmpOp::Lt,
        "LE" => CmpOp::Le,
        "EQ" => CmpOp::Eq,
        "GE" => CmpOp::Ge,
        "GT" => CmpOp::Gt,
        "NE" => CmpOp::Ne,
        _ => return None,
    })
}

fn parse_prop(p: &str) -> Result<Prop, ValidError> {
    if let Some(rest) = p.strip_prefix('!') {
        return Ok(Prop::Not(Box::new(parse_prop(rest)?)));
    }
    if matches!(p, "EnchantedBy" | "EquippedBy" | "AttachedBy") {
        return Ok(Prop::AttachedBySource);
    }
    if p == "IsRemembered" {
        return Ok(Prop::IsRemembered);
    }
    if p == "TopLibrary" {
        return Ok(Prop::TopLibrary);
    }
    match p {
        "ChosenType" => return Ok(Prop::ChosenType),
        "ChosenColor" => return Ok(Prop::ChosenColor),
        "nonChosenColor" => return Ok(Prop::Not(Box::new(Prop::ChosenColor))),
        "IsNotChosenType" | "nonChosenType" => return Ok(Prop::Not(Box::new(Prop::ChosenType))),
        _ => {}
    }
    if let Some(z) = p.strip_prefix("inZone")
        && let Some(z) = crate::zone::Zone::parse(z)
    {
        return Ok(Prop::InZone(z));
    }
    match p {
        "ChosenCard" => return Ok(Prop::ChosenCard { strict: false }),
        "ChosenCardStrict" => return Ok(Prop::ChosenCard { strict: true }),
        "nonChosenCard" => return Ok(Prop::Not(Box::new(Prop::ChosenCard { strict: false }))),
        _ => {}
    }
    if p == "RememberedPlayerCtrl" {
        return Ok(Prop::RememberedPlayerCtrl);
    }
    if p == "OppOwn" {
        return Ok(Prop::OppOwn);
    }
    if p == "HasCounters" {
        return Ok(Prop::HasCounters);
    }
    // `RedSource`, `ChosenColorSource`: Quelle der Farbe (Objekte sind ihre eigene Quelle)
    if let Some(col) = p.strip_suffix("Source") {
        if col == "ChosenColor" {
            return Ok(Prop::ChosenColor);
        }
        if let Some(c) = Color::from_name(col) {
            return Ok(Prop::Color(c));
        }
    }
    match p {
        "IsCommander" => return Ok(Prop::IsCommander),
        "Historic" => return Ok(Prop::Historic),
        "nonHistoric" => return Ok(Prop::Not(Box::new(Prop::Historic))),
        "ChosenCtrl" => return Ok(Prop::ChosenCtrl),
        "Chosen" => return Ok(Prop::PlayerChosen),
        "IsNotCommander" | "nonCommander" => return Ok(Prop::Not(Box::new(Prop::IsCommander))),
        _ => {}
    }
    if p == "ExiledWithSource" {
        return Ok(Prop::ExiledWithSource);
    }
    match p {
        "kicked" => return Ok(Prop::Kicked(None)),
        "kicked 1" => return Ok(Prop::Kicked(Some(1))),
        "kicked 2" => return Ok(Prop::Kicked(Some(2))),
        "notkicked" | "unkicked" => return Ok(Prop::Not(Box::new(Prop::Kicked(None)))),
        _ => {}
    }
    if let Some(rest) = p.strip_prefix("wasCast") {
        let from = match rest {
            "" => None,
            "FromYourHand" | "FromHand" | "FromTheirHand" => Some(crate::zone::Zone::Hand),
            "FromYourGraveyard" | "FromGraveyard" => Some(crate::zone::Zone::Graveyard),
            "FromExile" => Some(crate::zone::Zone::Exile),
            "FromLibrary" => Some(crate::zone::Zone::Library),
            _ => return Err(ValidError::Unsupported(p.to_string())),
        };
        return Ok(Prop::WasCast { from });
    }
    if p == "wasNotCast" {
        return Ok(Prop::Not(Box::new(Prop::WasCast { from: None })));
    }
    match p {
        "EnchantedController" | "EquippedController" => return Ok(Prop::EnchantedController),
        "enchanted" => return Ok(Prop::Enchanted),
        "equipped" => return Ok(Prop::Equipped),
        "unenchanted" => return Ok(Prop::Not(Box::new(Prop::Enchanted))),
        "unequipped" => return Ok(Prop::Not(Box::new(Prop::Equipped))),
        "IsMonstrous" | "monstrous" => return Ok(Prop::Monstrous),
        "nonMonstrous" | "IsNotMonstrous" => return Ok(Prop::Not(Box::new(Prop::Monstrous))),
        _ => {}
    }
    if let Some(rest) = p.strip_prefix("counters_") {
        let mut it = rest.splitn(2, '_');
        let (cmp, kind) = (it.next().unwrap_or(""), it.next().unwrap_or(""));
        if cmp.len() > 2
            && !kind.is_empty()
            && !kind.contains('_')
            && let Some(op) = cmp_op(&cmp[..2])
            && let Ok(value) = cmp[2..].parse::<i32>()
        {
            return Ok(Prop::Counters {
                kind: kind.to_ascii_uppercase(),
                op,
                value,
            });
        }
        return Err(ValidError::Unsupported(p.to_string()));
    }
    let colors = |s: &str| Color::from_name(s);
    Ok(match p {
        "YouCtrl" => Prop::YouCtrl,
        "YouDontCtrl" => Prop::YouDontCtrl,
        "OppCtrl" => Prop::OppCtrl,
        "YouOwn" => Prop::YouOwn,
        "Other" | "StrictlyOther" => Prop::Other,
        "Self" | "StrictlySelf" => Prop::SelfCard,
        "tapped" => Prop::Tapped,
        "untapped" => Prop::Untapped,
        "attacking" => Prop::Attacking,
        "blocking" => Prop::Blocking,
        "token" => Prop::Token,
        "nonToken" => Prop::NonToken,
        "Opponent" => Prop::PlayerOpponent,
        "multicolor" | "MultiColor" => Prop::Multicolor,
        "Colorless" | "colorless" => Prop::Colorless,
        "nonColorless" => Prop::NonColorless,
        _ => {
            for (prefix, stat) in [
                ("power", Stat::Power),
                ("toughness", Stat::Toughness),
                ("cmc", Stat::ManaValue),
            ] {
                if let Some(rest) = p.strip_prefix(prefix)
                    && rest.len() > 2
                {
                    let op = cmp_op(&rest[..2]);
                    if let (Some(op), Ok(value)) = (op, rest[2..].parse::<i32>()) {
                        return Ok(Prop::Stat { stat, op, value });
                    }
                    let var = &rest[2..];
                    if let Some(op) = op
                        && is_filter_var(var)
                    {
                        return Ok(Prop::StatVar {
                            stat,
                            op,
                            var: var.to_string(),
                        });
                    }
                }
            }
            if let Some(rest) = p.strip_prefix("non") {
                if let Some(c) = colors(rest) {
                    return Ok(Prop::NonColor(c));
                }
                if is_type_word(rest) {
                    return Ok(Prop::Non(card_base(rest)));
                }
            }
            if let Some(k) = p.strip_prefix("without") {
                let kw = Keyword::parse(k);
                if kw.is_supported() {
                    return Ok(Prop::Without(kw));
                }
            }
            if let Some(k) = p.strip_prefix("with") {
                let kw = Keyword::parse(k);
                if kw.is_supported() {
                    return Ok(Prop::With(kw));
                }
            }
            if let Some(c) = colors(p) {
                return Ok(Prop::Color(c));
            }
            if is_type_word(p) {
                return Ok(Prop::Is(card_base(p)));
            }
            return Err(ValidError::Unsupported(p.to_string()));
        }
    })
}

impl ValidSpec {
    pub fn parse(s: &str) -> Result<ValidSpec, ValidError> {
        let mut alternatives = Vec::new();
        for alt in s.split(',') {
            let alt = alt.trim();
            if alt == "Any" {
                alternatives.push(Alternative::Card {
                    base: CardBase::Core(CoreType::Creature),
                    props: vec![],
                });
                alternatives.push(Alternative::Card {
                    base: CardBase::Core(CoreType::Planeswalker),
                    props: vec![],
                });
                alternatives.push(Alternative::Card {
                    base: CardBase::Core(CoreType::Battle),
                    props: vec![],
                });
                alternatives.push(Alternative::Player {
                    base: PlayerBase::Player,
                    props: vec![],
                });
                continue;
            }
            let (base, props) = match alt.split_once('.') {
                Some((b, p)) => (b, Some(p)),
                None => (alt, None),
            };
            let props = match props {
                Some(p) => p
                    .split('+')
                    .map(parse_prop)
                    .collect::<Result<Vec<_>, _>>()?,
                None => Vec::new(),
            };
            let a = match base {
                "Player" => Alternative::Player {
                    base: PlayerBase::Player,
                    props,
                },
                "Opponent" => Alternative::Player {
                    base: PlayerBase::Opponent,
                    props,
                },
                "You" => Alternative::Player {
                    base: PlayerBase::You,
                    props,
                },
                "Emblem" => Alternative::Card {
                    base: CardBase::Emblem,
                    props,
                },
                b if is_type_word(b) => Alternative::Card {
                    base: card_base(b),
                    props,
                },
                b => return Err(ValidError::Unsupported(b.to_string())),
            };
            if let Alternative::Player { props, .. } = &a
                && props.iter().any(|p| {
                    !matches!(
                        p,
                        Prop::PlayerOpponent | Prop::EnchantedController | Prop::PlayerChosen
                    )
                })
            {
                return Err(ValidError::Unsupported(alt.to_string()));
            }
            alternatives.push(a);
        }
        Ok(ValidSpec { alternatives })
    }

    pub fn allows_players(&self) -> bool {
        self.alternatives
            .iter()
            .any(|a| matches!(a, Alternative::Player { .. }))
    }

    pub fn allows_cards(&self) -> bool {
        self.alternatives
            .iter()
            .any(|a| matches!(a, Alternative::Card { .. }))
    }
}

/// Geladene Typliste (falls `CardDb::load` lief).
pub fn type_registry() -> Option<&'static forge_types::TypeRegistry> {
    TYPE_REGISTRY.get()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_filters_parse() {
        for s in [
            "Creature",
            "Creature.YouCtrl",
            "Player",
            "Any",
            "Opponent",
            "Card",
            "Creature.OppCtrl",
            "Creature,Planeswalker",
            "Permanent.nonLand",
            "Creature.Other+YouCtrl",
            "Creature.attacking,Creature.blocking",
            "Creature.withFlying",
            "Creature.nonBlack",
            "Player.Opponent",
        ] {
            assert!(ValidSpec::parse(s).is_ok(), "{s}");
        }
        assert_eq!(ValidSpec::parse("Any").unwrap().alternatives.len(), 4);
        assert!(ValidSpec::parse("Creature.powerGE3").is_ok());
        assert!(ValidSpec::parse("Creature.enchanted").is_ok());
        assert!(ValidSpec::parse("Creature.modified").is_err());
    }
}
