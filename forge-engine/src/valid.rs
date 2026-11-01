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

fn parse_prop(p: &str) -> Result<Prop, ValidError> {
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
                    let op = match &rest[..2] {
                        "LT" => Some(crate::expr::CmpOp::Lt),
                        "LE" => Some(crate::expr::CmpOp::Le),
                        "EQ" => Some(crate::expr::CmpOp::Eq),
                        "GE" => Some(crate::expr::CmpOp::Ge),
                        "GT" => Some(crate::expr::CmpOp::Gt),
                        "NE" => Some(crate::expr::CmpOp::Ne),
                        _ => None,
                    };
                    if let (Some(op), Ok(value)) = (op, rest[2..].parse::<i32>()) {
                        return Ok(Prop::Stat { stat, op, value });
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
                b if is_type_word(b) => Alternative::Card {
                    base: card_base(b),
                    props,
                },
                b => return Err(ValidError::Unsupported(b.to_string())),
            };
            if let Alternative::Player { props, .. } = &a
                && props.iter().any(|p| !matches!(p, Prop::PlayerOpponent))
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
        assert!(ValidSpec::parse("Creature.enchanted").is_err());
    }
}
