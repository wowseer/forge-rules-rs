//! Statische Fähigkeiten (CR 604) – zunächst `Mode$ Continuous`.
//!
//! Java-Vorbild: `staticability/StaticAbility.java`,
//! `StaticAbilityContinuous.applyContinuousAbility`.
//!
//! Eine statische Fähigkeit erzeugt fortlaufende Effekte, solange ihre Quelle
//! in einer ihrer `EffectZone$` (Standard: Battlefield) ist. Die Anwendung
//! geschieht im Layer-System (`GameState::chars`, CR 613): jede Wirkung wird in
//! ihrem Layer in Zeitstempel-Reihenfolge angewendet; die betroffene Menge wird
//! im ersten Layer bestimmt, in dem der Effekt wirkt (CR 613.6).
//! Abhängigkeiten (CR 613.8) sind noch nicht umgesetzt.

use crate::effects::CompileError;
use crate::keyword::Keyword;
use crate::valid::ValidSpec;
use crate::zone::Zone;
use forge_script::{CardFace, ScriptStatic};
use forge_types::ColorSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinuousStatic {
    /// `Affected$`-Filter; `None` = kein Filter (nur `AffectedDefined$`).
    pub affected: Option<ValidSpec>,
    /// `AffectedDefined$ Self`: nur die Quelle selbst.
    pub affects_self_only: bool,
    pub affected_zones: Vec<Zone>,
    pub effect_zones: Vec<Zone>,
    /// Layer 4: hinzugefügte Typen (Kartentypen, Supertypen, Untertypen)
    pub add_types: Vec<String>,
    /// Layer 5
    pub set_color: Option<ColorSet>,
    /// Layer 6
    pub add_keywords: Vec<Keyword>,
    /// Layer 7b
    pub set_power: Option<i32>,
    pub set_toughness: Option<i32>,
    /// Layer 7c
    pub add_power: i32,
    pub add_toughness: i32,
    pub description: String,
}

impl ContinuousStatic {
    pub fn has_layer4(&self) -> bool {
        !self.add_types.is_empty()
    }
    pub fn has_layer5(&self) -> bool {
        self.set_color.is_some()
    }
    pub fn has_layer6(&self) -> bool {
        !self.add_keywords.is_empty()
    }
    pub fn has_layer7b(&self) -> bool {
        self.set_power.is_some() || self.set_toughness.is_some()
    }
    pub fn has_layer7c(&self) -> bool {
        self.add_power != 0 || self.add_toughness != 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompiledStatic {
    Continuous(ContinuousStatic),
}

const NEUTRAL: &[&str] = &["Mode", "Description", "Secondary", "AILogic"];

fn zones(s: &str) -> Option<Vec<Zone>> {
    s.split(',').map(Zone::parse).collect()
}

/// Feste Zahl oder SVar mit `Number$N`; andere Ausdrücke sind (noch) nicht unterstützt.
fn number(face: &CardFace, v: &str) -> Option<i32> {
    let t = v.trim().trim_start_matches('+');
    if let Ok(n) = t.parse() {
        return Some(n);
    }
    let (neg, name) = match t.strip_prefix('-') {
        Some(n) => (true, n),
        None => (false, t),
    };
    let n: i32 = face
        .svar(name)?
        .trim()
        .strip_prefix("Number$")?
        .parse()
        .ok()?;
    Some(if neg { -n } else { n })
}

pub fn compile_static(face: &CardFace, s: &ScriptStatic) -> Result<CompiledStatic, CompileError> {
    let p = &s.params;
    let err = |k: &str, v: &str| CompileError::UnsupportedParam {
        api: format!("Static {}", s.mode),
        key: k.into(),
        value: v.into(),
    };
    if s.mode != "Continuous" {
        return Err(CompileError::UnsupportedApi(format!("Static {}", s.mode)));
    }
    let mut used: Vec<&str> = NEUTRAL.to_vec();
    let mut get = |k: &'static str| {
        used.push(k);
        p.get(k)
    };
    let affects_self_only = match get("AffectedDefined") {
        None => false,
        Some("Self") => true,
        Some(v) => return Err(err("AffectedDefined", v)),
    };
    let affected = match get("Affected") {
        Some(a) => Some(ValidSpec::parse(a).map_err(CompileError::Valid)?),
        None if affects_self_only => None,
        None => {
            return Err(CompileError::MissingParam {
                api: "Static Continuous".into(),
                key: "Affected",
            });
        }
    };
    if affected.as_ref().is_some_and(|a| a.allows_players()) {
        return Err(err("Affected", p.get("Affected").unwrap_or("")));
    }
    let affected_zones = match get("AffectedZone") {
        Some(z) => zones(z).ok_or_else(|| err("AffectedZone", z))?,
        None => vec![Zone::Battlefield],
    };
    let effect_zones = match get("EffectZone") {
        Some(z) => zones(z).ok_or_else(|| err("EffectZone", z))?,
        None => vec![Zone::Battlefield],
    };
    let add_types = match get("AddType") {
        Some(t) => {
            let words: Vec<String> = t
                .split('&')
                .map(|w| w.trim().to_string())
                .filter(|w| !w.is_empty())
                .collect();
            if words
                .iter()
                .any(|w| w.starts_with("All") || w.contains(' ') || w.contains('.'))
            {
                return Err(err("AddType", t));
            }
            words
        }
        None => Vec::new(),
    };
    let set_color = match get("SetColor") {
        Some(c) => {
            Some(ColorSet::from_names(&c.replace('&', ",")).ok_or_else(|| err("SetColor", c))?)
        }
        None => None,
    };
    let add_keywords = match get("AddKeyword") {
        Some(k) => {
            let kws = Keyword::parse_list(k);
            if let Some(bad) = kws.iter().find(|k| !k.is_supported()) {
                return Err(err("AddKeyword", &format!("{bad:?}")));
            }
            kws
        }
        None => Vec::new(),
    };
    let num =
        |k: &'static str, used: &mut Vec<&'static str>| -> Result<Option<i32>, CompileError> {
            used.push(k);
            match p.get(k) {
                Some(v) => number(face, v).map(Some).ok_or_else(|| err(k, v)),
                None => Ok(None),
            }
        };
    let set_power = num("SetPower", &mut used)?;
    let set_toughness = num("SetToughness", &mut used)?;
    let add_power = num("AddPower", &mut used)?.unwrap_or(0);
    let add_toughness = num("AddToughness", &mut used)?.unwrap_or(0);

    for (k, v) in p.iter() {
        if !used.iter().any(|u| u.eq_ignore_ascii_case(k)) {
            return Err(err(k, v));
        }
    }
    Ok(CompiledStatic::Continuous(ContinuousStatic {
        affected,
        affects_self_only,
        affected_zones,
        effect_zones,
        add_types,
        set_color,
        add_keywords,
        set_power,
        set_toughness,
        add_power,
        add_toughness,
        description: p.get("Description").unwrap_or("").to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_script::CardScript;
    use forge_types::TypeRegistry;

    fn st(line: &str) -> Result<CompiledStatic, CompileError> {
        let c = CardScript::parse(
            &format!("Name:A\nManaCost:1\nTypes:Enchantment\n{line}\nSVar:X:Number$2\n"),
            &TypeRegistry::default(),
        )
        .unwrap();
        compile_static(c.main(), &c.main().statics[0])
    }

    #[test]
    fn anthem_and_unsupported() {
        let CompiledStatic::Continuous(a) =
            st("S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ 1 | AddToughness$ 1 | Description$ x").unwrap();
        assert_eq!((a.add_power, a.add_toughness), (1, 1));
        assert!(st("S:Mode$ Continuous | Affected$ Creature | AddPower$ X").is_ok());
        assert!(
            st("S:Mode$ Continuous | Affected$ Creature | IsPresent$ Card.Self | AddPower$ 1")
                .is_err()
        );
        assert!(st("S:Mode$ Continuous | Affected$ You | AddKeyword$ Hexproof").is_err());
        assert!(st("S:Mode$ CantBlockBy | ValidAttacker$ Creature").is_err());
    }
}
