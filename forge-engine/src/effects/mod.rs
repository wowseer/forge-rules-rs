//! Typisierte Effekte (Java: `ApiType` + `*Effect.java` in `ability/effects/`).
//!
//! [`compile`] übersetzt eine [`ScriptAbility`] (rohe Parameter) einmalig beim
//! Laden in eine [`CompiledAbility`]. Nicht unterstützte ApiTypes oder
//! Parameterwerte liefern [`CompileError`] – die Karte wird dann als „nicht
//! unterstützt“ markiert (nie stilles Ignorieren).
//!
//! Die Ausführung steht in `resolve.rs`.

pub mod resolve;

use crate::cost::{Cost, CostError};
use crate::keyword::Keyword;
use crate::valid::{ValidError, ValidSpec};
use forge_script::{AbilityKind, CardFace, ScriptAbility};
use forge_types::Color;
use std::fmt;
use std::sync::Arc;

/// Zahlenwert in Parametern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Amount {
    Fixed(i32),
    /// Der für X bezahlte Wert (`Count$xPaid`).
    XPaid,
}

/// Wen/was ein Effekt betrifft (`Defined$`, Java `AbilityUtils.getDefined…`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Defined {
    You,
    SelfCard,
    Targeted,
    TargetedController,
    EachPlayer,
    EachOpponent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetSpec {
    pub valid: ValidSpec,
    pub min: u32,
    pub max: u32,
    pub prompt: Option<String>,
}

/// Was eine Mana-Fähigkeit erzeugt (`Produced$`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManaProduce {
    /// Feste Symbole, z. B. `G` oder `R R` oder `C`. `None` = farblos.
    Fixed(Vec<Option<Color>>),
    /// Eine Farbe nach Wahl aus der Liste (`Any` = WUBRG, `Combo R G`).
    OneOf(Vec<Color>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    DealDamage {
        amount: Amount,
        to: Defined,
    },
    Draw {
        amount: Amount,
        who: Defined,
    },
    GainLife {
        amount: Amount,
        who: Defined,
    },
    LoseLife {
        amount: Amount,
        who: Defined,
    },
    Destroy {
        what: Defined,
        no_regen: bool,
    },
    Pump {
        what: Defined,
        power: Amount,
        toughness: Amount,
        keywords: Vec<Keyword>,
    },
    AddMana {
        produce: ManaProduce,
        amount: u32,
    },
    Tap {
        what: Defined,
    },
    Untap {
        what: Defined,
    },
    PutCounter {
        what: Defined,
        counter: String,
        amount: Amount,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledAbility {
    pub kind: AbilityKind,
    pub api: String,
    pub cost: Option<Cost>,
    pub target: Option<TargetSpec>,
    pub effect: Effect,
    pub sub: Option<Arc<CompiledAbility>>,
    pub description: String,
    /// `SorcerySpeed$ True` bei aktivierten Fähigkeiten.
    pub sorcery_speed: bool,
}

impl CompiledAbility {
    pub fn is_mana_ability(&self) -> bool {
        // CR 605.1a: kein Ziel, erzeugt Mana, keine Loyalitätsfähigkeit.
        self.kind == AbilityKind::Activated
            && matches!(self.effect, Effect::AddMana { .. })
            && self.target.is_none()
    }

    /// Alle Fähigkeiten der Kette (diese + SubAbilities), in Auflösungsreihenfolge.
    pub fn chain(&self) -> Vec<&CompiledAbility> {
        let mut v = vec![self];
        let mut cur = self;
        while let Some(s) = &cur.sub {
            v.push(s);
            cur = s;
        }
        v
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    UnsupportedApi(String),
    UnsupportedParam {
        api: String,
        key: String,
        value: String,
    },
    MissingParam {
        api: String,
        key: &'static str,
    },
    MissingSvar(String),
    Cost(CostError),
    Valid(ValidError),
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileError::UnsupportedApi(a) => write!(f, "Effekt nicht unterstützt: {a}"),
            CompileError::UnsupportedParam { api, key, value } => {
                write!(f, "{api}: {key}$ {value} nicht unterstützt")
            }
            CompileError::MissingParam { api, key } => {
                write!(f, "{api}: Pflichtparameter {key}$ fehlt")
            }
            CompileError::MissingSvar(s) => write!(f, "SVar {s} fehlt"),
            CompileError::Cost(e) => write!(f, "{e}"),
            CompileError::Valid(e) => write!(f, "{e}"),
        }
    }
}

impl From<CostError> for CompileError {
    fn from(e: CostError) -> Self {
        CompileError::Cost(e)
    }
}

impl From<ValidError> for CompileError {
    fn from(e: ValidError) -> Self {
        CompileError::Valid(e)
    }
}

/// Parameter, die das Verhalten nicht ändern (Texte, KI-Hinweise) und daher
/// ignoriert werden dürfen. Alle anderen unbekannten Parameter sind Fehler.
const NEUTRAL_PARAMS: &[&str] = &[
    "SP",
    "AB",
    "DB",
    "SpellDescription",
    "StackDescription",
    "TgtPrompt",
    "AILogic",
    "AITgts",
    "AIPreference",
    "PrecostDesc",
    "CostDesc",
    "PlayerTurn",
    "IsCurse",
    "AiAttackController",
    "AIManaPref",
    "IsPresent",
];

struct Ctx<'a> {
    face: &'a CardFace,
    ab: &'a ScriptAbility,
    used: Vec<&'static str>,
}

impl<'a> Ctx<'a> {
    fn get(&mut self, key: &'static str) -> Option<&'a str> {
        self.used.push(key);
        self.ab.params.get(key)
    }

    fn unsupported(&self, key: &str, value: &str) -> CompileError {
        CompileError::UnsupportedParam {
            api: self.ab.api.clone(),
            key: key.into(),
            value: value.into(),
        }
    }

    fn amount(&mut self, key: &'static str, default: Option<i32>) -> Result<Amount, CompileError> {
        match self.get(key) {
            None => default
                .map(Amount::Fixed)
                .ok_or(CompileError::MissingParam {
                    api: self.ab.api.clone(),
                    key,
                }),
            Some(v) => self.amount_value(key, v),
        }
    }

    fn amount_value(&self, key: &str, v: &str) -> Result<Amount, CompileError> {
        let t = v.trim().trim_start_matches('+');
        if let Ok(n) = t.parse::<i32>() {
            return Ok(Amount::Fixed(n));
        }
        // Verweis auf eine SVar (z. B. X → Count$xPaid)
        let (neg, name) = match t.strip_prefix('-') {
            Some(n) => (true, n),
            None => (false, t),
        };
        match self.face.svar(name) {
            Some(sv) if sv.trim() == "Count$xPaid" && !neg => Ok(Amount::XPaid),
            Some(sv) if sv.trim().starts_with("Number$") => {
                let n: i32 = sv.trim()["Number$".len()..]
                    .parse()
                    .map_err(|_| self.unsupported(key, sv))?;
                Ok(Amount::Fixed(if neg { -n } else { n }))
            }
            Some(sv) => Err(self.unsupported(key, sv)),
            None => Err(self.unsupported(key, v)),
        }
    }

    fn defined(&mut self, default: Defined) -> Result<Defined, CompileError> {
        let has_tgt = self.ab.params.has("ValidTgts");
        match self.get("Defined") {
            None if has_tgt => Ok(Defined::Targeted),
            None => Ok(default),
            Some(v) => Ok(match v {
                "You" => Defined::You,
                "Self" => Defined::SelfCard,
                "Targeted" | "TargetedPlayer" | "TargetedCard" => Defined::Targeted,
                "TargetedController" => Defined::TargetedController,
                "Player" => Defined::EachPlayer,
                "Player.Opponent" | "Opponent" => Defined::EachOpponent,
                other => return Err(self.unsupported("Defined", other)),
            }),
        }
    }
}

fn parse_produced(s: &str) -> Option<ManaProduce> {
    let s = s.trim();
    if s == "Any" {
        return Some(ManaProduce::OneOf(Color::ALL.to_vec()));
    }
    if let Some(rest) = s.strip_prefix("Combo ") {
        if rest.trim() == "Any" {
            return None; // Kombinationen mehrerer Farben: später
        }
        let cols: Option<Vec<Color>> = rest
            .split_whitespace()
            .map(|t| t.chars().next().and_then(Color::from_symbol))
            .collect();
        return cols.map(ManaProduce::OneOf);
    }
    let syms: Option<Vec<Option<Color>>> = s
        .split_whitespace()
        .map(|t| match t {
            "C" => Some(None),
            t if t.len() == 1 => t.chars().next().and_then(Color::from_symbol).map(Some),
            _ => None,
        })
        .collect();
    syms.map(ManaProduce::Fixed)
}

/// Übersetzt eine Fähigkeit samt `SubAbility$`-Kette.
pub fn compile(face: &CardFace, ab: &ScriptAbility) -> Result<CompiledAbility, CompileError> {
    compile_depth(face, ab, 0)
}

fn compile_depth(
    face: &CardFace,
    ab: &ScriptAbility,
    depth: usize,
) -> Result<CompiledAbility, CompileError> {
    let mut c = Ctx {
        face,
        ab,
        used: Vec::new(),
    };
    let api = ab.api.as_str();

    let cost = match c.get("Cost") {
        Some(s) => Some(Cost::parse(s)?),
        None => None,
    };
    let target = match c.get("ValidTgts") {
        Some(v) => {
            let min = match c.get("TargetMin") {
                Some(s) => s.parse().map_err(|_| c.unsupported("TargetMin", s))?,
                None => 1,
            };
            let max = match c.get("TargetMax") {
                Some(s) => s.parse().map_err(|_| c.unsupported("TargetMax", s))?,
                None => 1,
            };
            Some(TargetSpec {
                valid: ValidSpec::parse(v)?,
                min,
                max,
                prompt: c.get("TgtPrompt").map(str::to_string),
            })
        }
        None => None,
    };
    let sorcery_speed = c
        .get("SorcerySpeed")
        .is_some_and(|v| v.eq_ignore_ascii_case("true"));
    let description = c
        .get("SpellDescription")
        .or(c.ab.params.get("StackDescription"))
        .unwrap_or("")
        .to_string();

    let effect = match api {
        "DealDamage" => {
            let amount = c.amount("NumDmg", None)?;
            let to = c.defined(Defined::Targeted)?;
            Effect::DealDamage { amount, to }
        }
        "Draw" => {
            let amount = c.amount("NumCards", Some(1))?;
            let who = c.defined(Defined::You)?;
            Effect::Draw { amount, who }
        }
        "GainLife" => {
            let amount = c.amount("LifeAmount", None)?;
            let who = c.defined(Defined::You)?;
            Effect::GainLife { amount, who }
        }
        "LoseLife" => {
            let amount = c.amount("LifeAmount", None)?;
            let who = c.defined(Defined::You)?;
            Effect::LoseLife { amount, who }
        }
        "Destroy" => {
            let what = c.defined(Defined::Targeted)?;
            let no_regen = c
                .get("NoRegen")
                .is_some_and(|v| v.eq_ignore_ascii_case("true"));
            Effect::Destroy { what, no_regen }
        }
        "Pump" => {
            let what = c.defined(Defined::SelfCard)?;
            let power = c.amount("NumAtt", Some(0))?;
            let toughness = c.amount("NumDef", Some(0))?;
            let keywords = c.get("KW").map(Keyword::parse_list).unwrap_or_default();
            if let Some(k) = keywords.iter().find(|k| !k.is_supported()) {
                return Err(c.unsupported("KW", &format!("{k:?}")));
            }
            Effect::Pump {
                what,
                power,
                toughness,
                keywords,
            }
        }
        "Mana" => {
            let produced = c.get("Produced").ok_or(CompileError::MissingParam {
                api: api.into(),
                key: "Produced",
            })?;
            let produce =
                parse_produced(produced).ok_or_else(|| c.unsupported("Produced", produced))?;
            let amount = match c.get("Amount") {
                Some(s) => s.parse().map_err(|_| c.unsupported("Amount", s))?,
                None => 1,
            };
            Effect::AddMana { produce, amount }
        }
        "Tap" => Effect::Tap {
            what: c.defined(Defined::Targeted)?,
        },
        "Untap" => Effect::Untap {
            what: c.defined(Defined::Targeted)?,
        },
        "PutCounter" => {
            let what = c.defined(Defined::SelfCard)?;
            let counter = c.get("CounterType").ok_or(CompileError::MissingParam {
                api: api.into(),
                key: "CounterType",
            })?;
            let amount = c.amount("CounterNum", Some(1))?;
            Effect::PutCounter {
                what,
                counter: counter.to_string(),
                amount,
            }
        }
        other => return Err(CompileError::UnsupportedApi(other.to_string())),
    };

    let sub = match c.get("SubAbility") {
        Some(name) if depth < 32 => {
            let sv = face
                .svar(name)
                .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?;
            let sub_ab = ScriptAbility::from_svar(sv)
                .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?
                .map_err(|_| CompileError::MissingSvar(name.to_string()))?;
            Some(Arc::new(compile_depth(face, &sub_ab, depth + 1)?))
        }
        Some(name) => return Err(c.unsupported("SubAbility", name)),
        None => None,
    };

    // Alle übrigen Parameter müssen neutral sein.
    for (k, v) in ab.params.iter() {
        let known = c.used.iter().any(|u| u.eq_ignore_ascii_case(k))
            || NEUTRAL_PARAMS.iter().any(|n| n.eq_ignore_ascii_case(k));
        if !known {
            return Err(c.unsupported(k, v));
        }
    }

    Ok(CompiledAbility {
        kind: ab.kind,
        api: ab.api.clone(),
        cost,
        target,
        effect,
        sub,
        description,
        sorcery_speed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_script::CardScript;
    use forge_types::TypeRegistry;

    fn first(txt: &str) -> Result<CompiledAbility, CompileError> {
        let c = CardScript::parse(txt, &TypeRegistry::default()).unwrap();
        compile(c.main(), &c.main().abilities[0])
    }

    #[test]
    fn bolt_and_x() {
        let a = first("Name:B\nManaCost:R\nTypes:Instant\nA:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | SpellDescription$ x\n").unwrap();
        assert_eq!(
            a.effect,
            Effect::DealDamage {
                amount: Amount::Fixed(3),
                to: Defined::Targeted
            }
        );
        let x = first("Name:F\nManaCost:X R\nTypes:Sorcery\nA:SP$ DealDamage | ValidTgts$ Any | NumDmg$ X\nSVar:X:Count$xPaid\n").unwrap();
        assert_eq!(
            x.effect,
            Effect::DealDamage {
                amount: Amount::XPaid,
                to: Defined::Targeted
            }
        );
    }

    #[test]
    fn subability_chain() {
        let a = first("Name:C\nManaCost:1 U\nTypes:Instant\nA:SP$ Draw | NumCards$ 1 | SubAbility$ DBGain\nSVar:DBGain:DB$ GainLife | LifeAmount$ 2\n").unwrap();
        assert_eq!(a.chain().len(), 2);
    }

    #[test]
    fn unknown_param_rejected() {
        let e = first("Name:C\nManaCost:1 U\nTypes:Instant\nA:SP$ Draw | NumCards$ 1 | Weird$ 3\n")
            .unwrap_err();
        assert!(matches!(e, CompileError::UnsupportedParam { .. }));
    }

    #[test]
    fn mana_ability() {
        let a = first("Name:E\nManaCost:G\nTypes:Creature Elf\nPT:1/1\nA:AB$ Mana | Cost$ T | Produced$ G | SpellDescription$ Add {G}.\n").unwrap();
        assert!(a.is_mana_ability());
    }
}
