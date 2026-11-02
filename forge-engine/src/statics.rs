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
use crate::expr::{CondStyle, Condition, Expr, parse_conditions};
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
    /// `AffectedDefined$ Enchanted|Equipped`: nur das Objekt, an dem die Quelle hängt.
    pub affects_attached: bool,
    pub affected_zones: Vec<Zone>,
    pub effect_zones: Vec<Zone>,
    /// Layer 4: hinzugefügte Typen (Kartentypen, Supertypen, Untertypen)
    pub add_types: Vec<String>,
    /// Layer 5
    pub set_color: Option<ColorSet>,
    /// `SetColor$ ChosenColor`
    pub set_color_chosen: bool,
    /// Layer 6
    pub add_keywords: Vec<Keyword>,
    /// Layer 7b (bzw. 7a bei `CharacteristicDefining$ True`)
    pub set_power: Option<Expr>,
    pub set_toughness: Option<Expr>,
    /// Layer 7c
    pub add_power: Option<Expr>,
    pub add_toughness: Option<Expr>,
    /// `CharacteristicDefining$ True` (CR 604.3): wirkt nur auf die Quelle, in allen Zonen, Layer 7a.
    pub characteristic_defining: bool,
    /// Bedingungen (`IsPresent$`, `CheckSVar$`, `Condition$` …)
    pub conditions: Vec<Condition>,
    /// `MayPlay$ True`: betroffene Karten dürfen gespielt werden (CR 601.2a, 305.1).
    pub may_play: Option<MayPlay>,
    pub description: String,
}

/// Spielerlaubnis aus `MayPlay$ True` (Java `StaticAbilityContinuous`, `MayPlay…`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MayPlay {
    /// `MayPlayWithoutManaCost$`: ohne Manakosten (X = 0, CR 107.3)
    pub without_mana_cost: bool,
    /// `MayPlayWithFlash$`
    pub with_flash: bool,
    /// `MayPlayLimit$ N`: je Zug höchstens N Karten über diese Erlaubnis
    pub limit: Option<u32>,
    /// `MayPlayPlayer$ CardOwner`: der Besitzer der Karte statt des Controllers der Quelle
    pub card_owner: bool,
    /// `MayPlayIgnoreType$`/`MayPlayIgnoreColor$`: Mana wie beliebigen Typs bzw.
    /// beliebiger Farbe ausgeben (CR 609.4b)
    pub ignore_type: bool,
    pub ignore_color: bool,
}

impl ContinuousStatic {
    pub fn has_layer4(&self) -> bool {
        !self.add_types.is_empty()
    }
    pub fn has_layer5(&self) -> bool {
        self.set_color.is_some() || self.set_color_chosen
    }
    pub fn has_layer6(&self) -> bool {
        !self.add_keywords.is_empty()
    }
    pub fn has_layer7b(&self) -> bool {
        self.set_power.is_some() || self.set_toughness.is_some()
    }
    pub fn has_layer7c(&self) -> bool {
        self.add_power.is_some() || self.add_toughness.is_some()
    }

    /// Hängt das Ergebnis vom Spielzustand ab (Bedingungen, Zähler)? Solche
    /// Statics werden bei verschachtelten Auswertungen übersprungen.
    pub fn is_dynamic(&self) -> bool {
        !self.conditions.is_empty()
            || [
                &self.set_power,
                &self.set_toughness,
                &self.add_power,
                &self.add_toughness,
            ]
            .iter()
            .any(|e| e.as_ref().is_some_and(|e| !e.is_const()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
// Statics liegen immer hinter `Arc` (CompiledFace), die Variantengröße ist daher unkritisch.
#[allow(clippy::large_enum_variant)]
pub enum CompiledStatic {
    Continuous(ContinuousStatic),
    /// Kampfbeschränkungen (CR 508.1c/d, 509.1b/c).
    Combat(CombatRestriction),
    /// Kostenänderung für Zaubersprüche (`ReduceCost`/`RaiseCost`, CR 601.2f).
    CostAdjust(CostAdjust),
    /// Alternativkosten für den Zauber selbst (`AlternativeCost | ValidSA$ Spell.Self`, CR 118.9).
    AlternativeCost(AltCost),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AltCost {
    pub cost: crate::cost::Cost,
    pub effect_zones: Vec<Zone>,
    pub conditions: Vec<Condition>,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CostAdjust {
    /// `true` = Senkung, `false` = Erhöhung (generisches Mana)
    pub reduce: bool,
    pub valid_card: ValidSpec,
    pub amount: Expr,
    /// Wer wirkt: `You`/`Opponent`/`Player` relativ zum Controller der Quelle (Standard: jeder)
    pub activator: Option<ValidSpec>,
    pub effect_zones: Vec<Zone>,
    pub conditions: Vec<Condition>,
}

fn compile_alt_cost(face: &CardFace, s: &ScriptStatic) -> Result<CompiledStatic, CompileError> {
    let p = &s.params;
    let err = |k: &str, v: &str| CompileError::UnsupportedParam {
        api: "Static AlternativeCost".into(),
        key: k.into(),
        value: v.into(),
    };
    let mut used: Vec<&str> = vec![
        "Mode",
        "Description",
        "Secondary",
        "EffectZone",
        "ValidSA",
        "ValidCard",
        "ValidPlayer",
        "Cost",
    ];
    let self_spell = matches!(
        (p.get("ValidSA"), p.get("ValidCard")),
        (Some("Spell.Self"), None | Some("Card.Self")) | (Some("Spell"), Some("Card.Self"))
    );
    if !self_spell {
        return Err(err("ValidSA", p.get("ValidSA").unwrap_or("fehlt")));
    }
    if !matches!(p.get("EffectZone"), Some("All") | Some("Hand")) {
        return Err(err("EffectZone", p.get("EffectZone").unwrap_or("fehlt")));
    }
    if !matches!(p.get("ValidPlayer"), None | Some("You")) {
        return Err(err("ValidPlayer", p.get("ValidPlayer").unwrap_or("")));
    }
    let cost_s = p.get("Cost").ok_or_else(|| err("Cost", "fehlt"))?;
    let cost = crate::cost::Cost::parse(cost_s)?;
    if cost.mana.as_ref().is_some_and(|m| m.x_count() > 0) {
        return Err(err("Cost", cost_s));
    }
    let (conditions, ck) = parse_conditions(face, p, CondStyle::Requirement)?;
    used.extend(ck);
    for (k, v) in p.iter() {
        if !used.iter().any(|u| u.eq_ignore_ascii_case(k)) {
            return Err(err(k, v));
        }
    }
    Ok(CompiledStatic::AlternativeCost(AltCost {
        cost,
        effect_zones: ALL.to_vec(),
        conditions,
        description: p.get("Description").unwrap_or("").to_string(),
    }))
}

fn compile_cost_adjust(
    face: &CardFace,
    s: &ScriptStatic,
    reduce: bool,
) -> Result<CompiledStatic, CompileError> {
    let p = &s.params;
    let err = |k: &str, v: &str| CompileError::UnsupportedParam {
        api: format!("Static {}", s.mode),
        key: k.into(),
        value: v.into(),
    };
    let mut used: Vec<&str> = vec![
        "Mode",
        "Description",
        "Secondary",
        "EffectZone",
        "ValidCard",
        "Type",
        "Amount",
        "Activator",
    ];
    match p.get("Type") {
        Some("Spell") => {}
        Some(v) => return Err(err("Type", v)),
        None => return Err(err("Type", "fehlt")),
    }
    let valid_card =
        ValidSpec::parse(p.get("ValidCard").unwrap_or("Card")).map_err(CompileError::Valid)?;
    let amount = Expr::parse_value(face, p.get("Amount").unwrap_or("1"))
        .map_err(|_| err("Amount", p.get("Amount").unwrap_or("")))?;
    let activator = p
        .get("Activator")
        .map(ValidSpec::parse)
        .transpose()
        .map_err(CompileError::Valid)?;
    let effect_zones = match p.get("EffectZone") {
        Some(z) => zones(z).ok_or_else(|| err("EffectZone", z))?,
        None => vec![Zone::Battlefield],
    };
    let (conditions, ck) = parse_conditions(face, p, CondStyle::Requirement)?;
    used.extend(ck);
    for (k, v) in p.iter() {
        if !used.iter().any(|u| u.eq_ignore_ascii_case(k)) {
            return Err(err(k, v));
        }
    }
    Ok(CompiledStatic::CostAdjust(CostAdjust {
        reduce,
        valid_card,
        amount,
        activator,
        effect_zones,
        conditions,
    }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestrictionKind {
    /// `CantBlockBy`: `valid` (Angreifer) kann nicht von `valid_blocker` geblockt werden
    /// (ohne `ValidBlocker$`: gar nicht).
    CantBlockBy,
    /// `CantBlock`: `valid` kann nicht blocken.
    CantBlock,
    /// `CantAttack`: `valid` kann nicht angreifen.
    CantAttack,
    /// `MustAttack`: `valid` greift jeden Kampf an, falls möglich.
    MustAttack,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatRestriction {
    pub kind: RestrictionKind,
    pub valid: ValidSpec,
    pub valid_blocker: Option<ValidSpec>,
    pub effect_zones: Vec<Zone>,
    pub conditions: Vec<Condition>,
    pub description: String,
}

impl CompiledStatic {
    pub fn effect_zones_mut(&mut self) -> &mut Vec<Zone> {
        match self {
            CompiledStatic::Continuous(c) => &mut c.effect_zones,
            CompiledStatic::Combat(c) => &mut c.effect_zones,
            CompiledStatic::CostAdjust(c) => &mut c.effect_zones,
            CompiledStatic::AlternativeCost(c) => &mut c.effect_zones,
        }
    }
}

fn compile_combat(
    face: &CardFace,
    s: &ScriptStatic,
    kind: RestrictionKind,
) -> Result<CompiledStatic, CompileError> {
    let p = &s.params;
    let err = |k: &str, v: &str| CompileError::UnsupportedParam {
        api: format!("Static {}", s.mode),
        key: k.into(),
        value: v.into(),
    };
    let key = match kind {
        RestrictionKind::CantBlockBy => "ValidAttacker",
        RestrictionKind::CantBlock | RestrictionKind::CantAttack => "ValidCard",
        RestrictionKind::MustAttack => "ValidCreature",
    };
    let mut used: Vec<&str> = vec![
        "Mode",
        "Description",
        "Secondary",
        "EffectZone",
        key,
        "ValidBlocker",
    ];
    let valid = ValidSpec::parse(p.get(key).ok_or_else(|| err(key, "fehlt"))?)
        .map_err(CompileError::Valid)?;
    let valid_blocker = match (kind, p.get("ValidBlocker")) {
        (RestrictionKind::CantBlockBy, Some(v)) => {
            Some(ValidSpec::parse(v).map_err(CompileError::Valid)?)
        }
        (_, Some(v)) => return Err(err("ValidBlocker", v)),
        _ => None,
    };
    let effect_zones = match p.get("EffectZone") {
        Some(z) => zones(z).ok_or_else(|| err("EffectZone", z))?,
        None => vec![Zone::Battlefield],
    };
    let (conditions, ck) = parse_conditions(face, p, CondStyle::Requirement)?;
    used.extend(ck);
    for (k, v) in p.iter() {
        if !used.iter().any(|u| u.eq_ignore_ascii_case(k)) {
            return Err(err(k, v));
        }
    }
    Ok(CompiledStatic::Combat(CombatRestriction {
        kind,
        valid,
        valid_blocker,
        effect_zones,
        conditions,
        description: p.get("Description").unwrap_or("").to_string(),
    }))
}

// `MayLookAt$`: nur Informationsfluss (wer verdeckte Karten sehen darf), ohne Regelwirkung.
const NEUTRAL: &[&str] = &["Mode", "Description", "Secondary", "AILogic", "MayLookAt"];

const ALL: [Zone; 7] = [
    Zone::Library,
    Zone::Hand,
    Zone::Battlefield,
    Zone::Graveyard,
    Zone::Stack,
    Zone::Exile,
    Zone::Command,
];

/// Zonenliste; `All` = alle Zonen.
fn zones(s: &str) -> Option<Vec<Zone>> {
    if s.trim() == "All" {
        return Some(ALL.to_vec());
    }
    s.split(',').map(Zone::parse).collect()
}

pub fn compile_static(face: &CardFace, s: &ScriptStatic) -> Result<CompiledStatic, CompileError> {
    let p = &s.params;
    let err = |k: &str, v: &str| CompileError::UnsupportedParam {
        api: format!("Static {}", s.mode),
        key: k.into(),
        value: v.into(),
    };
    match s.mode.as_str() {
        "Continuous" => {}
        "CantBlockBy" => return compile_combat(face, s, RestrictionKind::CantBlockBy),
        "CantBlock" => return compile_combat(face, s, RestrictionKind::CantBlock),
        "CantAttack" => return compile_combat(face, s, RestrictionKind::CantAttack),
        "MustAttack" => return compile_combat(face, s, RestrictionKind::MustAttack),
        "ReduceCost" => return compile_cost_adjust(face, s, true),
        "RaiseCost" => return compile_cost_adjust(face, s, false),
        "AlternativeCost" => return compile_alt_cost(face, s),
        _ => return Err(CompileError::UnsupportedApi(format!("Static {}", s.mode))),
    }
    let mut used: Vec<&str> = NEUTRAL.to_vec();
    let mut get = |k: &'static str| {
        used.push(k);
        p.get(k)
    };
    let (affects_self_only, affects_attached) = match get("AffectedDefined") {
        None => (false, false),
        Some("Self") => (true, false),
        Some("Enchanted") | Some("Equipped") => (false, true),
        Some(v) => return Err(err("AffectedDefined", v)),
    };
    // CDAs (CR 604.3) wirken immer nur auf die Quelle, in jeder Zone.
    let cda = p
        .get("CharacteristicDefining")
        .is_some_and(|v| v.eq_ignore_ascii_case("true"));
    let affects_self_only = affects_self_only || cda;
    let affected = match get("Affected") {
        Some(a) => Some(ValidSpec::parse(a).map_err(CompileError::Valid)?),
        None if affects_self_only || affects_attached => None,
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
        None if cda => ALL.to_vec(),
        None => vec![Zone::Battlefield],
    };
    let effect_zones = match get("EffectZone") {
        Some(z) => zones(z).ok_or_else(|| err("EffectZone", z))?,
        None if cda => ALL.to_vec(),
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
    let set_color_chosen = p.get("SetColor") == Some("ChosenColor");
    let set_color = match get("SetColor") {
        Some("ChosenColor") => None,
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
        |k: &'static str, used: &mut Vec<&'static str>| -> Result<Option<Expr>, CompileError> {
            used.push(k);
            match p.get(k) {
                Some(v) => Expr::parse_value(face, v).map(Some).map_err(|_| err(k, v)),
                None => Ok(None),
            }
        };
    let set_power = num("SetPower", &mut used)?;
    let set_toughness = num("SetToughness", &mut used)?;
    let add_power = num("AddPower", &mut used)?;
    let add_toughness = num("AddToughness", &mut used)?;
    used.push("CharacteristicDefining");
    let characteristic_defining = p
        .get("CharacteristicDefining")
        .is_some_and(|v| v.eq_ignore_ascii_case("true"));
    let (conditions, cond_keys) = parse_conditions(face, p, CondStyle::Requirement)?;
    used.extend(cond_keys);
    let flag = |k: &str| p.get(k).is_some_and(|v| v.eq_ignore_ascii_case("true"));
    let may_play = if flag("MayPlay") {
        used.extend([
            "MayPlay",
            "MayPlayWithoutManaCost",
            "MayPlayWithFlash",
            "MayPlayLimit",
            "MayPlayPlayer",
            "MayPlayIgnoreType",
            "MayPlayIgnoreColor",
        ]);
        let without_mana_cost = flag("MayPlayWithoutManaCost");
        if without_mana_cost && affected_zones.contains(&Zone::Hand) {
            return Err(err("AffectedZone", "Hand mit MayPlayWithoutManaCost"));
        }
        Some(MayPlay {
            without_mana_cost,
            with_flash: flag("MayPlayWithFlash"),
            limit: match p.get("MayPlayLimit") {
                None => None,
                Some(v) => Some(v.trim().parse().map_err(|_| err("MayPlayLimit", v))?),
            },
            card_owner: match p.get("MayPlayPlayer") {
                None => false,
                Some("CardOwner") => true,
                Some(v) => return Err(err("MayPlayPlayer", v)),
            },
            ignore_type: flag("MayPlayIgnoreType"),
            ignore_color: flag("MayPlayIgnoreColor"),
        })
    } else {
        None
    };

    for (k, v) in p.iter() {
        if !used.iter().any(|u| u.eq_ignore_ascii_case(k)) {
            return Err(err(k, v));
        }
    }
    Ok(CompiledStatic::Continuous(ContinuousStatic {
        affected,
        affects_self_only,
        affects_attached,
        affected_zones,
        effect_zones,
        add_types,
        set_color,
        set_color_chosen,
        add_keywords,
        set_power,
        set_toughness,
        add_power,
        add_toughness,
        characteristic_defining,
        conditions,
        may_play,
        description: p.get("Description").unwrap_or("").to_string(),
    }))
}

/// `Mode$ CantAttack,CantBlock` → je Modus eine statische Fähigkeit.
pub fn compile_static_multi(
    face: &CardFace,
    s: &ScriptStatic,
) -> Result<Vec<CompiledStatic>, CompileError> {
    if !s.mode.contains(',') {
        return Ok(vec![compile_static(face, s)?]);
    }
    s.mode
        .split(',')
        .map(|m| {
            let mut one = s.clone();
            one.mode = m.trim().to_string();
            one.params.insert("Mode", m.trim());
            compile_static(face, &one)
        })
        .collect()
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
        let Ok(CompiledStatic::Continuous(a)) = st(
            "S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ 1 | AddToughness$ 1 | Description$ x",
        ) else {
            panic!("Continuous erwartet")
        };
        assert_eq!(
            (a.add_power, a.add_toughness),
            (Some(Expr::Const(1)), Some(Expr::Const(1)))
        );
        assert!(st("S:Mode$ Continuous | Affected$ Creature | AddPower$ X").is_ok());
        assert!(
            st("S:Mode$ Continuous | Affected$ Creature | IsPresent$ Card.Self | AddPower$ 1")
                .is_ok()
        );
        assert!(
            st("S:Mode$ Continuous | Affected$ Creature | IsPresent2$ Card.Self | AddPower$ 1")
                .is_err()
        );
        assert!(st("S:Mode$ Continuous | Affected$ You | AddKeyword$ Hexproof").is_err());
        assert!(st("S:Mode$ CantBlockBy | ValidAttacker$ Creature").is_ok());
        assert!(st("S:Mode$ CantRegenerate | ValidCard$ Creature").is_err());
    }
}
