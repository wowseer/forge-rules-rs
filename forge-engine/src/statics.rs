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
    /// Layer 6: verliehene aktivierte Fähigkeiten (`AddAbility$`)
    pub add_abilities: Vec<std::sync::Arc<crate::effects::CompiledAbility>>,
    /// Layer 6: verliehene Trigger (`AddTrigger$`)
    pub add_triggers: Vec<std::sync::Arc<crate::triggers::CompiledTrigger>>,
    /// Layer 6: `RemoveAllAbilities$ True`
    pub remove_all_abilities: bool,
    /// Layer 4: `RemoveLandTypes$ True`
    pub remove_land_types: bool,
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
    /// Layer 2: `GainControl$ You` – der Controller der Quelle kontrolliert die
    /// betroffenen Objekte (CR 613.1b).
    pub gain_control: bool,
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
        !self.add_types.is_empty() || self.remove_land_types
    }
    pub fn has_layer5(&self) -> bool {
        self.set_color.is_some() || self.set_color_chosen
    }
    pub fn has_layer6(&self) -> bool {
        !self.add_keywords.is_empty()
            || !self.add_abilities.is_empty()
            || !self.add_triggers.is_empty()
            || self.remove_all_abilities
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
    /// `Continuous` mit `Affected$ You|Player|Opponent`: wirkt auf Spieler.
    Player(PlayerStatic),
    /// `CantBeCast`/`CantBeActivated` (CR 101.2: „kann nicht“ gewinnt).
    CantPlay(CantPlay),
    /// `CastWithFlash`: Zauber dürfen wie mit Flash gewirkt werden (CR 702.8, 601.3).
    CastWithFlash(CastWithFlash),
    /// `MinMaxBlocker`: Angreifer nur von mindestens/höchstens so vielen Kreaturen blockbar.
    MinMaxBlocker(MinMaxBlocker),
    /// `CanAttackDefender`: kann trotz Defender angreifen (CR 702.3b).
    CanAttackDefender(CanAttackDefender),
    /// `CantTarget`: kann nicht Ziel sein (CR 115.4 u. a.).
    CantTarget(CantTarget),
    /// `Panharmonicon`: Trigger lösen ein zusätzliches Mal aus (CR 603.2d).
    Panharmonicon(Panharmonicon),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CantTarget {
    pub valid_target: Option<ValidSpec>,
    /// `ValidSA$`: nur Zauber (`Some(true)`) bzw. nur Fähigkeiten (`Some(false)`)
    pub spell: Option<bool>,
    pub activator: Option<ValidSpec>,
    pub valid_source: Option<ValidSpec>,
    /// `AffectedZone$`: Zone des Ziels (Standard Battlefield)
    pub target_zones: Vec<Zone>,
    pub effect_zones: Vec<Zone>,
    pub conditions: Vec<Condition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Panharmonicon {
    /// Quelle des Triggers
    pub valid_card: ValidSpec,
    /// `ValidMode$` (leer = alle)
    pub modes: Vec<String>,
    /// `ValidCause$`: das auslösende Objekt
    pub valid_cause: Option<ValidSpec>,
    /// `Destination$`/`Origin$` bei Zonenwechsel-Triggern
    pub destination: Option<Zone>,
    pub effect_zones: Vec<Zone>,
    pub conditions: Vec<Condition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanAttackDefender {
    pub valid: ValidSpec,
    pub effect_zones: Vec<Zone>,
    pub conditions: Vec<Condition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinMaxBlocker {
    pub valid: ValidSpec,
    pub min: Option<u32>,
    pub max: Option<u32>,
    pub effect_zones: Vec<Zone>,
    pub conditions: Vec<Condition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CastWithFlash {
    pub valid_card: ValidSpec,
    pub caster: Option<ValidSpec>,
    pub effect_zones: Vec<Zone>,
    pub conditions: Vec<Condition>,
}

/// Verbot, Zauber zu wirken oder Fähigkeiten zu aktivieren
/// (Java `StaticAbilityCantBeCast`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CantPlay {
    /// `false` = `CantBeCast`, `true` = `CantBeActivated`
    pub activate: bool,
    /// Karte (Zauber bzw. Quelle der Fähigkeit)
    pub valid_card: ValidSpec,
    /// `Caster$`/`Activator$`
    pub player: Option<ValidSpec>,
    /// `Origin$` (Wirken) bzw. `AffectedZone$` (Aktivieren): Zone der Karte
    pub card_zones: Option<Vec<Zone>>,
    /// `NumLimitEachTurn$`: erst ab so vielen gewirkten passenden Zaubern im Zug
    pub limit_each_turn: Option<u32>,
    /// `OnlySorcerySpeed$`: nur außerhalb der Hexerei-Zeitpunkte verboten
    pub only_sorcery_speed: bool,
    /// `ValidSA$ Activated[.!ManaAbility|.ManaAbility|.Loyalty|.!Loyalty]`
    pub mana_ability: Option<bool>,
    pub loyalty: Option<bool>,
    pub effect_zones: Vec<Zone>,
    pub conditions: Vec<Condition>,
}

/// Maximale Handgröße (CR 402.2, 514.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandSize {
    Unlimited,
    Set(Expr),
}

/// Fortlaufender Effekt auf Spieler (Java `StaticAbilityContinuous`, `affectedPlayers`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerStatic {
    pub affected: ValidSpec,
    pub effect_zones: Vec<Zone>,
    pub conditions: Vec<Condition>,
    /// `SetMaxHandSize$`
    pub set_max_hand: Option<HandSize>,
    /// `RaiseMaxHandSize$` (auch negativ)
    pub raise_max_hand: Option<Expr>,
    /// `AdjustLandPlays$ n`: zusätzliche Landspiele je Zug (CR 305.2)
    pub adjust_land_plays: Option<Expr>,
    /// `AddKeyword$ Hexproof|Shroud` (CR 702.11d, 702.18d)
    pub hexproof: bool,
    pub shroud: bool,
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

fn compile_player_static(
    face: &CardFace,
    s: &ScriptStatic,
    affected: ValidSpec,
) -> Result<CompiledStatic, CompileError> {
    let p = &s.params;
    let err = |k: &str, v: &str| CompileError::UnsupportedParam {
        api: format!("Static {}", s.mode),
        key: k.into(),
        value: v.into(),
    };
    let mut used: Vec<&str> = NEUTRAL.to_vec();
    used.extend([
        "Affected",
        "EffectZone",
        "SetMaxHandSize",
        "RaiseMaxHandSize",
        "AddKeyword",
        "AdjustLandPlays",
    ]);
    let effect_zones = match p.get("EffectZone") {
        Some(z) => zones(z).ok_or_else(|| err("EffectZone", z))?,
        None => vec![Zone::Battlefield],
    };
    let set_max_hand = match p.get("SetMaxHandSize") {
        None => None,
        Some("Unlimited") => Some(HandSize::Unlimited),
        Some(v) => Some(HandSize::Set(Expr::parse_value(face, v)?)),
    };
    let raise_max_hand = p
        .get("RaiseMaxHandSize")
        .map(|v| Expr::parse_value(face, v))
        .transpose()?;
    let (mut hexproof, mut shroud) = (false, false);
    if let Some(k) = p.get("AddKeyword") {
        for kw in Keyword::parse_list(k) {
            match kw {
                Keyword::Hexproof => hexproof = true,
                Keyword::Shroud => shroud = true,
                other => return Err(err("AddKeyword", &format!("{other:?}"))),
            }
        }
    }
    let adjust_land_plays = p
        .get("AdjustLandPlays")
        .map(|v| Expr::parse_value(face, v))
        .transpose()?;
    let (conditions, ck) = parse_conditions(face, p, CondStyle::Requirement)?;
    used.extend(ck);
    for (k, v) in p.iter() {
        if !used.iter().any(|u| u.eq_ignore_ascii_case(k)) {
            return Err(err(k, v));
        }
    }
    if set_max_hand.is_none()
        && raise_max_hand.is_none()
        && adjust_land_plays.is_none()
        && !hexproof
        && !shroud
    {
        return Err(err("Affected", p.get("Affected").unwrap_or("")));
    }
    Ok(CompiledStatic::Player(PlayerStatic {
        affected,
        effect_zones,
        conditions,
        set_max_hand,
        raise_max_hand,
        adjust_land_plays,
        hexproof,
        shroud,
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
    let amount = Expr::parse_value(face, p.get("Amount").unwrap_or("1"))?;
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
            CompiledStatic::Player(c) => &mut c.effect_zones,
            CompiledStatic::CantPlay(c) => &mut c.effect_zones,
            CompiledStatic::CastWithFlash(c) => &mut c.effect_zones,
            CompiledStatic::MinMaxBlocker(c) => &mut c.effect_zones,
            CompiledStatic::CanAttackDefender(c) => &mut c.effect_zones,
            CompiledStatic::CantTarget(c) => &mut c.effect_zones,
            CompiledStatic::Panharmonicon(c) => &mut c.effect_zones,
        }
    }
}

fn compile_min_max_blocker(
    face: &CardFace,
    s: &ScriptStatic,
) -> Result<CompiledStatic, CompileError> {
    let p = &s.params;
    let err = |k: &str, v: &str| CompileError::UnsupportedParam {
        api: format!("Static {}", s.mode),
        key: k.into(),
        value: v.into(),
    };
    let mut used: Vec<&str> = NEUTRAL.to_vec();
    used.extend(["EffectZone", "ValidCard", "Min", "Max"]);
    let num = |k: &str| -> Result<Option<u32>, CompileError> {
        match p.get(k) {
            None => Ok(None),
            Some(v) => v.trim().parse().map(Some).map_err(|_| err(k, v)),
        }
    };
    let (min, max) = (num("Min")?, num("Max")?);
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
    Ok(CompiledStatic::MinMaxBlocker(MinMaxBlocker {
        valid: ValidSpec::parse(p.get("ValidCard").unwrap_or("Creature"))?,
        min,
        max,
        effect_zones,
        conditions,
    }))
}

fn compile_cast_with_flash(
    face: &CardFace,
    s: &ScriptStatic,
) -> Result<CompiledStatic, CompileError> {
    let p = &s.params;
    let err = |k: &str, v: &str| CompileError::UnsupportedParam {
        api: format!("Static {}", s.mode),
        key: k.into(),
        value: v.into(),
    };
    let mut used: Vec<&str> = NEUTRAL.to_vec();
    used.extend(["EffectZone", "ValidCard", "Caster", "ValidSA"]);
    match p.get("ValidSA") {
        None | Some("Spell") => {}
        Some(v) => return Err(err("ValidSA", v)),
    }
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
    Ok(CompiledStatic::CastWithFlash(CastWithFlash {
        valid_card: ValidSpec::parse(p.get("ValidCard").unwrap_or("Card"))?,
        caster: p.get("Caster").map(ValidSpec::parse).transpose()?,
        effect_zones,
        conditions,
    }))
}

fn compile_cant_play(
    face: &CardFace,
    s: &ScriptStatic,
    activate: bool,
) -> Result<CompiledStatic, CompileError> {
    let p = &s.params;
    let err = |k: &str, v: &str| CompileError::UnsupportedParam {
        api: format!("Static {}", s.mode),
        key: k.into(),
        value: v.into(),
    };
    let player_key = if activate { "Activator" } else { "Caster" };
    let zone_key = if activate { "AffectedZone" } else { "Origin" };
    let mut used: Vec<&str> = NEUTRAL.to_vec();
    used.extend([
        "EffectZone",
        "ValidCard",
        player_key,
        zone_key,
        "NumLimitEachTurn",
        "OnlySorcerySpeed",
        "ValidSA",
    ]);
    let valid_card = ValidSpec::parse(p.get("ValidCard").unwrap_or("Card"))?;
    let player = p.get(player_key).map(ValidSpec::parse).transpose()?;
    let card_zones = match p.get(zone_key) {
        Some(z) => Some(zones(z).ok_or_else(|| err(zone_key, z))?),
        None => None,
    };
    let limit_each_turn = match p.get("NumLimitEachTurn") {
        Some(v) if !activate => Some(v.trim().parse().map_err(|_| err("NumLimitEachTurn", v))?),
        Some(v) => return Err(err("NumLimitEachTurn", v)),
        None => None,
    };
    let only_sorcery_speed = p
        .get("OnlySorcerySpeed")
        .is_some_and(|v| v.eq_ignore_ascii_case("true"));
    let (mut mana_ability, mut loyalty) = (None, None);
    if let Some(v) = p.get("ValidSA") {
        let props = match v.split_once('.') {
            Some(("Activated", rest)) if activate => rest,
            None if v == "Activated" && activate => "",
            _ => return Err(err("ValidSA", v)),
        };
        for prop in props.split('+').filter(|x| !x.is_empty()) {
            match prop {
                "ManaAbility" => mana_ability = Some(true),
                "!ManaAbility" | "nonManaAbility" => mana_ability = Some(false),
                "Loyalty" => loyalty = Some(true),
                "!Loyalty" => loyalty = Some(false),
                _ => return Err(err("ValidSA", v)),
            }
        }
    }
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
    Ok(CompiledStatic::CantPlay(CantPlay {
        activate,
        valid_card,
        player,
        card_zones,
        limit_each_turn,
        only_sorcery_speed,
        mana_ability,
        loyalty,
        effect_zones,
        conditions,
    }))
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
        "CantBeCast" => return compile_cant_play(face, s, false),
        "CastWithFlash" => return compile_cast_with_flash(face, s),
        "MinMaxBlocker" => return compile_min_max_blocker(face, s),
        "CantTarget" => {
            let mut used: Vec<&str> = NEUTRAL.to_vec();
            used.extend([
                "EffectZone",
                "ValidTarget",
                "ValidSA",
                "Activator",
                "ValidSource",
                "AffectedZone",
            ]);
            let opt = |k: &str| p.get(k).map(ValidSpec::parse).transpose();
            let spell = match p.get("ValidSA") {
                None | Some("Spell,Activated") | Some("SpellAbility") => None,
                Some("Spell") => Some(true),
                Some("Activated") | Some("Ability") => Some(false),
                Some(v) => return Err(err("ValidSA", v)),
            };
            let target_zones = match p.get("AffectedZone") {
                Some(z) => zones(z).ok_or_else(|| err("AffectedZone", z))?,
                None => vec![Zone::Battlefield],
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
            return Ok(CompiledStatic::CantTarget(CantTarget {
                valid_target: opt("ValidTarget")?,
                spell,
                activator: opt("Activator")?,
                valid_source: opt("ValidSource")?,
                target_zones,
                effect_zones,
                conditions,
            }));
        }
        "Panharmonicon" => {
            let mut used: Vec<&str> = NEUTRAL.to_vec();
            used.extend([
                "EffectZone",
                "ValidCard",
                "ValidMode",
                "ValidCause",
                "Destination",
                "Origin",
            ]);
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
            let destination = match p.get("Destination") {
                Some(z) => Some(Zone::parse(z).ok_or_else(|| err("Destination", z))?),
                None => None,
            };
            return Ok(CompiledStatic::Panharmonicon(Panharmonicon {
                valid_card: ValidSpec::parse(p.get("ValidCard").unwrap_or("Card"))?,
                modes: p
                    .get("ValidMode")
                    .map(|m| m.split(',').map(|x| x.trim().to_string()).collect())
                    .unwrap_or_default(),
                valid_cause: p.get("ValidCause").map(ValidSpec::parse).transpose()?,
                destination,
                effect_zones,
                conditions,
            }));
        }
        "CanAttackDefender" => {
            let mut used: Vec<&str> = NEUTRAL.to_vec();
            used.extend(["EffectZone", "ValidCard"]);
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
            return Ok(CompiledStatic::CanAttackDefender(CanAttackDefender {
                valid: ValidSpec::parse(p.get("ValidCard").unwrap_or("Creature"))?,
                effect_zones,
                conditions,
            }));
        }
        "CantBeActivated" => return compile_cant_play(face, s, true),
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
        Some("Enchanted") | Some("Equipped") | Some("AttachedBy Self") => (false, true),
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
    if let Some(a) = &affected
        && a.allows_players()
    {
        if a.allows_cards() {
            return Err(err("Affected", p.get("Affected").unwrap_or("")));
        }
        return compile_player_static(face, s, a.clone());
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
    let svar_list = |key: &str| -> Vec<String> {
        p.get(key)
            .map(|v| {
                v.split('&')
                    .map(|x| x.trim().to_string())
                    .filter(|x| !x.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    };
    let mut add_abilities = Vec::new();
    for name in svar_list("AddAbility") {
        let sv = face
            .svar(&name)
            .ok_or_else(|| CompileError::MissingSvar(name.clone()))?;
        let ab = forge_script::ScriptAbility::from_svar(sv)
            .ok_or_else(|| CompileError::MissingSvar(name.clone()))?
            .map_err(|_| CompileError::MissingSvar(name.clone()))?;
        let compiled = crate::effects::compile(face, &ab)?;
        if compiled.kind != forge_script::AbilityKind::Activated {
            return Err(err("AddAbility", &name));
        }
        add_abilities.push(std::sync::Arc::new(compiled));
    }
    let mut add_triggers = Vec::new();
    for name in svar_list("AddTrigger") {
        let sv = face
            .svar(&name)
            .ok_or_else(|| CompileError::MissingSvar(name.clone()))?;
        let st = forge_script::ScriptTrigger::parse(forge_script::ParamMap::parse(sv))
            .map_err(|_| err("AddTrigger", &name))?;
        add_triggers.push(std::sync::Arc::new(crate::triggers::compile_trigger(
            face, &st,
        )?));
    }
    get("AddAbility");
    get("AddTrigger");
    // KI-Hinweise bzw. SVars verliehener Fähigkeiten (kompiliert aus der Quelle)
    get("AddSVar");
    let remove_all_abilities =
        get("RemoveAllAbilities").is_some_and(|v| v.eq_ignore_ascii_case("true"));
    let remove_land_types = get("RemoveLandTypes").is_some_and(|v| v.eq_ignore_ascii_case("true"));
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
                Some(v) => Expr::parse_value(face, v).map(Some),
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

    used.push("GainControl");
    let gain_control = match p.get("GainControl") {
        None => false,
        Some("You") => true,
        Some(v) => return Err(err("GainControl", v)),
    };
    for (k, v) in p.iter() {
        if !used.iter().any(|u| u.eq_ignore_ascii_case(k)) {
            return Err(err(k, v));
        }
    }
    Ok(CompiledStatic::Continuous(ContinuousStatic {
        gain_control,
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
        add_abilities,
        add_triggers,
        remove_all_abilities,
        remove_land_types,
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
        assert!(st("S:Mode$ Continuous | Affected$ You | AddKeyword$ Hexproof").is_ok());
        assert!(st("S:Mode$ Continuous | Affected$ You | AddKeyword$ Flying").is_err());
        assert!(st("S:Mode$ CantBlockBy | ValidAttacker$ Creature").is_ok());
        assert!(st("S:Mode$ CantRegenerate | ValidCard$ Creature").is_err());
    }
}
