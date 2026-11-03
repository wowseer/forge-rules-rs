//! Ersatzeffekte (CR 614) – zunächst `Event$ Moved` (Zonenwechsel) und das
//! Keyword `etbCounter`.
//!
//! Java-Vorbild: `replacement/ReplaceMoved.java`, `ReplacementHandler`,
//! `CardFactoryUtil.makeEtbCounter`.
//!
//! Unterstützte Formen:
//! - **Umleitung**: `ReplaceWith$` zeigt auf `DB$ ChangeZone | Defined$ ReplacedCard |
//!   Destination$ <Zone>` → das Objekt geht stattdessen in diese Zone
//!   (z. B. Rest in Peace, Leyline of the Void).
//! - **Getappt ins Spiel** (CR 614.1c): `ReplaceWith$` zeigt auf `DB$ Tap | ETB$ True |
//!   Defined$ Self|ReplacedCard`, optional mit `ConditionPresent$` usw. (Check-Lands).
//! - **Mit Marken ins Spiel** (`K:etbCounter:<Typ>:<Anzahl>`).
//!
//! Jede Ersetzung wirkt höchstens einmal je Ereignis (CR 614.5). Wählt bei mehreren
//! Umleitungen eigentlich der betroffene Spieler (CR 616.1) – hier vereinfacht nach
//! Zeitstempel (dokumentiert im Arbeitsprotokoll).

use crate::effects::CompileError;
use crate::expr::{CondStyle, Condition, EvalCtx, Expr, parse_conditions};
use crate::ids::CardId;
use crate::state::GameState;
use crate::valid::ValidSpec;
use crate::zone::Zone;
use forge_script::{CardFace, ScriptAbility, ScriptReplacement};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MovedAction {
    /// Stattdessen in diese Zone.
    Redirect(Zone),
    /// Kommt getappt ins Spiel, falls die Bedingungen der Ersatz-Fähigkeit gelten.
    EnterTapped(Vec<Condition>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledReplacement {
    pub origin: Option<Vec<Zone>>,
    pub destination: Vec<Zone>,
    pub valid_card: ValidSpec,
    /// `None` = Quelle in beliebiger Zone (Java-Standard ohne `ActiveZones$`).
    pub active_zones: Option<Vec<Zone>>,
    /// Bedingungen der Ersatzwirkung selbst (`IsPresent$` usw. an der `R:`-Zeile).
    pub conditions: Vec<Condition>,
    pub action: MovedAction,
    pub description: String,
}

fn zones(s: &str) -> Option<Vec<Zone>> {
    s.split(',').map(Zone::parse).collect()
}

const NEUTRAL: &[&str] = &[
    "Event",
    "Description",
    "ReplacementResult",
    "ReplaceWith",
    "Layer",
    "Secondary",
    "AILogic",
];

pub fn compile_replacement(
    face: &CardFace,
    r: &ScriptReplacement,
) -> Result<CompiledReplacement, CompileError> {
    let p = &r.params;
    let err = |k: &str, v: &str| CompileError::UnsupportedParam {
        api: format!("Replacement {}", r.mode),
        key: k.into(),
        value: v.into(),
    };
    if r.mode != "Moved" {
        return Err(CompileError::UnsupportedApi(format!(
            "Replacement {}",
            r.mode
        )));
    }
    let mut used: Vec<&str> = NEUTRAL.to_vec();
    used.extend(["Origin", "Destination", "ValidCard", "ActiveZones"]);
    let origin = match p.get("Origin") {
        Some("Any") => None,
        Some(o) => Some(zones(o).ok_or_else(|| err("Origin", o))?),
        None => None,
    };
    let destination = match p.get("Destination") {
        Some(d) => zones(d).ok_or_else(|| err("Destination", d))?,
        None => return Err(err("Destination", "fehlt")),
    };
    let valid_card =
        ValidSpec::parse(p.get("ValidCard").unwrap_or("Card")).map_err(CompileError::Valid)?;
    let active_zones = match p.get("ActiveZones") {
        Some(z) => Some(zones(z).ok_or_else(|| err("ActiveZones", z))?),
        None => None,
    };
    let (conditions, ck) = parse_conditions(face, p, CondStyle::Requirement)?;
    used.extend(ck);
    for (k, v) in p.iter() {
        if !used.iter().any(|u| u.eq_ignore_ascii_case(k)) {
            return Err(err(k, v));
        }
    }

    // Ersatzwirkung aus der SVar klassifizieren.
    let name = p
        .get("ReplaceWith")
        .ok_or_else(|| err("ReplaceWith", "fehlt"))?;
    let sv = face
        .svar(name)
        .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?;
    let ab = ScriptAbility::from_svar(sv)
        .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?
        .map_err(|_| CompileError::MissingSvar(name.to_string()))?;
    let ap = &ab.params;
    let self_ref = matches!(ap.get("Defined"), Some("Self") | Some("ReplacedCard"));
    let allowed = |extra: &[&str]| {
        ap.keys().all(|k| {
            [
                "DB",
                "Defined",
                "StackDescription",
                "SpellDescription",
                "AILogic",
            ]
            .iter()
            .chain(extra)
            .any(|x| x.eq_ignore_ascii_case(k))
        })
    };
    let action = match ab.api.as_str() {
        "Tap" if self_ref && ap.is_true("ETB") && destination == [Zone::Battlefield] => {
            let (conds, ck) = parse_conditions(face, ap, CondStyle::Effect)?;
            let mut extra = vec!["ETB"];
            extra.extend(ck);
            if !allowed(&extra) {
                return Err(err("ReplaceWith", sv));
            }
            MovedAction::EnterTapped(conds)
        }
        "ChangeZone" if ap.get("Defined") == Some("ReplacedCard") => {
            let to = ap
                .get("Destination")
                .and_then(Zone::parse)
                .ok_or_else(|| err("ReplaceWith", sv))?;
            if !allowed(&["Origin", "Destination", "Hidden"]) {
                return Err(err("ReplaceWith", sv));
            }
            MovedAction::Redirect(to)
        }
        _ => return Err(err("ReplaceWith", sv)),
    };
    Ok(CompiledReplacement {
        origin,
        destination,
        valid_card,
        active_zones,
        conditions,
        action,
        description: p.get("Description").unwrap_or("").to_string(),
    })
}

/// `R:Event$ Untap | ValidStepTurnToController$ You | Layer$ CantHappen`:
/// „… enttappt nicht während deines Enttappsegments“ (Java `ReplaceUntap`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UntapReplacement {
    pub valid_card: ValidSpec,
    pub active_zones: Option<Vec<Zone>>,
    pub conditions: Vec<Condition>,
}

pub fn compile_untap_replacement(
    face: &CardFace,
    r: &ScriptReplacement,
) -> Result<UntapReplacement, CompileError> {
    let p = &r.params;
    let err = |k: &str, v: &str| CompileError::UnsupportedParam {
        api: "Replacement Untap".into(),
        key: k.into(),
        value: v.into(),
    };
    if p.get("Layer") != Some("CantHappen") || p.has("ReplaceWith") {
        return Err(err("Layer", p.get("Layer").unwrap_or("fehlt")));
    }
    if p.get("ValidStepTurnToController") != Some("You") {
        return Err(err(
            "ValidStepTurnToController",
            p.get("ValidStepTurnToController").unwrap_or("fehlt"),
        ));
    }
    let mut used: Vec<&str> = NEUTRAL.to_vec();
    used.extend([
        "ValidCard",
        "ActiveZones",
        "ValidStepTurnToController",
        "EnduringStory",
    ]);
    let (conditions, ck) = parse_conditions(face, p, CondStyle::Requirement)?;
    used.extend(ck);
    for (k, v) in p.iter() {
        if !used.iter().any(|u| u.eq_ignore_ascii_case(k)) {
            return Err(err(k, v));
        }
    }
    Ok(UntapReplacement {
        valid_card: ValidSpec::parse(p.get("ValidCard").unwrap_or("Card"))
            .map_err(CompileError::Valid)?,
        active_zones: match p.get("ActiveZones") {
            Some(z) => Some(zones(z).ok_or_else(|| err("ActiveZones", z))?),
            None => None,
        },
        conditions,
    })
}

/// `R:Event$ AddCounter`: mehr Marken (Hardened Scales, Doubling Season …).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CounterReplacement {
    pub valid_card: Option<ValidSpec>,
    pub valid_player: Option<ValidSpec>,
    pub counter_type: Option<String>,
    /// nur bei Marken aus Effekten
    pub effect_only: bool,
    /// neue Anzahl; `Count$xPaid` steht für die ursprüngliche
    pub amount: Expr,
    pub active_zones: Option<Vec<Zone>>,
}

pub fn compile_counter_replacement(
    face: &CardFace,
    r: &ScriptReplacement,
) -> Result<CounterReplacement, CompileError> {
    let p = &r.params;
    let err = |k: &str, v: &str| CompileError::UnsupportedParam {
        api: "Replacement AddCounter".into(),
        key: k.into(),
        value: v.into(),
    };
    let mut used: Vec<&str> = NEUTRAL.to_vec();
    used.extend([
        "ActiveZones",
        "ValidCard",
        "ValidPlayer",
        "ValidCounterType",
        "EffectOnly",
        "ReplaceWith",
    ]);
    for (k, v) in p.iter() {
        if !used.iter().any(|u| u.eq_ignore_ascii_case(k)) {
            return Err(err(k, v));
        }
    }
    let name = p
        .get("ReplaceWith")
        .ok_or_else(|| err("ReplaceWith", "fehlt"))?;
    let sv = face
        .svar(name)
        .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?;
    let ab = ScriptAbility::from_svar(sv)
        .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?
        .map_err(|_| CompileError::MissingSvar(name.to_string()))?;
    if ab.api != "ReplaceCounter" {
        return Err(err("ReplaceWith", sv));
    }
    let var = ab
        .params
        .get("Amount")
        .ok_or_else(|| err("Amount", "fehlt"))?;
    let x = face
        .svar(var)
        .ok_or_else(|| CompileError::MissingSvar(var.to_string()))?;
    if !x.trim_start().starts_with("ReplaceCount$CounterNum") {
        return Err(err("Amount", x));
    }
    let amount = Expr::parse_svar(
        face,
        &x.replacen("ReplaceCount$CounterNum", "Count$xPaid", 1),
    )?;
    let opt = |k: &str| {
        p.get(k)
            .map(ValidSpec::parse)
            .transpose()
            .map_err(CompileError::Valid)
    };
    Ok(CounterReplacement {
        valid_card: opt("ValidCard")?,
        valid_player: opt("ValidPlayer")?,
        counter_type: p
            .get("ValidCounterType")
            .or(ab.params.get("ValidCounterType"))
            .map(|t| t.to_ascii_uppercase()),
        effect_only: p.is_true("EffectOnly"),
        amount,
        active_zones: match p.get("ActiveZones") {
            Some(z) => Some(zones(z).ok_or_else(|| err("ActiveZones", z))?),
            None => None,
        },
    })
}

/// `R:Event$ CreateToken` mit `ReplaceToken | Type$ Amount`: doppelt so viele Spielsteine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenReplacement {
    pub valid_token: Option<ValidSpec>,
    pub valid_player: Option<ValidSpec>,
    pub effect_only: bool,
    pub active_zones: Option<Vec<Zone>>,
}

pub fn compile_token_replacement(
    face: &CardFace,
    r: &ScriptReplacement,
) -> Result<TokenReplacement, CompileError> {
    let p = &r.params;
    let err = |k: &str, v: &str| CompileError::UnsupportedParam {
        api: "Replacement CreateToken".into(),
        key: k.into(),
        value: v.into(),
    };
    let mut used: Vec<&str> = NEUTRAL.to_vec();
    used.extend([
        "ActiveZones",
        "ValidToken",
        "ValidPlayer",
        "EffectOnly",
        "ReplaceWith",
    ]);
    for (k, v) in p.iter() {
        if !used.iter().any(|u| u.eq_ignore_ascii_case(k)) {
            return Err(err(k, v));
        }
    }
    let name = p
        .get("ReplaceWith")
        .ok_or_else(|| err("ReplaceWith", "fehlt"))?;
    let sv = face
        .svar(name)
        .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?;
    let ab = ScriptAbility::from_svar(sv)
        .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?
        .map_err(|_| CompileError::MissingSvar(name.to_string()))?;
    let only_known = ab.params.keys().all(|k| {
        ["DB", "Type", "SpellDescription", "StackDescription"]
            .iter()
            .any(|x| x.eq_ignore_ascii_case(k))
    });
    if ab.api != "ReplaceToken" || ab.params.get("Type") != Some("Amount") || !only_known {
        return Err(err("ReplaceWith", sv));
    }
    let opt = |k: &str| {
        p.get(k)
            .map(ValidSpec::parse)
            .transpose()
            .map_err(CompileError::Valid)
    };
    Ok(TokenReplacement {
        valid_token: opt("ValidToken")?,
        valid_player: opt("ValidPlayer")?,
        effect_only: p.is_true("EffectOnly"),
        active_zones: match p.get("ActiveZones") {
            Some(z) => Some(zones(z).ok_or_else(|| err("ActiveZones", z))?),
            None => None,
        },
    })
}

/// `R:Event$ Counter | Layer$ CantHappen`: „kann nicht neutralisiert werden“
/// (CR 101.2, Java `ReplaceCounter`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CantCounterReplacement {
    pub valid_card: Option<ValidSpec>,
    /// `ValidSA$ Spell[.…]` als Kartenfilter auf den Zauber
    pub valid_spell: ValidSpec,
    pub active_zones: Option<Vec<Zone>>,
    pub conditions: Vec<Condition>,
}

pub fn compile_cant_counter_replacement(
    face: &CardFace,
    r: &ScriptReplacement,
) -> Result<CantCounterReplacement, CompileError> {
    let p = &r.params;
    let err = |k: &str, v: &str| CompileError::UnsupportedParam {
        api: "Replacement Counter".into(),
        key: k.into(),
        value: v.into(),
    };
    if p.get("Layer") != Some("CantHappen") || p.has("ReplaceWith") {
        return Err(err("Layer", p.get("Layer").unwrap_or("fehlt")));
    }
    let mut used: Vec<&str> = NEUTRAL.to_vec();
    used.extend(["ValidCard", "ValidSA", "ActiveZones"]);
    let (conditions, ck) = parse_conditions(face, p, CondStyle::Requirement)?;
    used.extend(ck);
    for (k, v) in p.iter() {
        if !used.iter().any(|u| u.eq_ignore_ascii_case(k)) {
            return Err(err(k, v));
        }
    }
    let sa = p.get("ValidSA").unwrap_or("Spell");
    let spec: Vec<String> = sa
        .split(',')
        .map(|a| {
            let a = a.trim();
            match a.split_once('.') {
                Some(("Spell", rest)) => Ok(format!("Card.{rest}")),
                None if a == "Spell" => Ok("Card".to_string()),
                _ => Err(err("ValidSA", sa)),
            }
        })
        .collect::<Result<_, _>>()?;
    Ok(CantCounterReplacement {
        valid_card: p
            .get("ValidCard")
            .map(ValidSpec::parse)
            .transpose()
            .map_err(CompileError::Valid)?,
        valid_spell: ValidSpec::parse(&spec.join(",")).map_err(CompileError::Valid)?,
        active_zones: match p.get("ActiveZones") {
            Some(z) => Some(zones(z).ok_or_else(|| err("ActiveZones", z))?),
            None => None,
        },
        conditions,
    })
}

impl GameState {
    /// CR 614.1a: Ersatzeffekte für das Legen von Marken (je Effekt einmal,
    /// nach Zeitstempel; Verlauf der Wahl des Betroffenen vereinfacht).
    pub(crate) fn replace_counter_amount(
        &self,
        target: crate::decision::Target,
        kind: &str,
        n: u32,
    ) -> u32 {
        use crate::decision::Target;
        if n == 0 {
            return 0;
        }
        let mut hosts: Vec<(u64, CardId)> = self
            .live_cards()
            .map(|id| self.card(id))
            .filter(|c| c.zone.is_some() && !c.compiled().counter_replacements.is_empty())
            .map(|c| (c.timestamp, c.id))
            .collect();
        hosts.sort();
        let mut n = n as i32;
        for (_, host) in hosts {
            let h = self.card(host);
            let hz = h.zone.expect("gefiltert");
            for r in &h.compiled().counter_replacements {
                if r.active_zones.as_ref().is_some_and(|z| !z.contains(&hz))
                    || r.counter_type.as_ref().is_some_and(|t| t != kind)
                    || (r.effect_only && self.effect_source.is_none())
                {
                    continue;
                }
                let ok = match target {
                    Target::Card(c) => {
                        r.valid_player.is_none()
                            && r.valid_card
                                .as_ref()
                                .is_none_or(|v| self.card_matches(c, v, host, h.controller))
                    }
                    Target::Player(p) => {
                        r.valid_card.is_none()
                            && r.valid_player
                                .as_ref()
                                .is_some_and(|v| self.player_matches_src(p, v, h.controller, host))
                    }
                };
                if !ok {
                    continue;
                }
                let ctx = EvalCtx {
                    source: host,
                    controller: h.controller,
                    x: n.max(0) as u32,
                    targets: &[],
                    trig: None,
                    paid: &[],
                };
                n = self.eval(&r.amount, &ctx);
            }
        }
        n.max(0) as u32
    }

    /// Wie oft ein Spielstein für `owner` erzeugt wird (Verdopplungen, CR 614.1a).
    pub(crate) fn token_multiplier(&self, token: CardId, owner: crate::ids::PlayerId) -> u32 {
        let mut mult = 1;
        for host in self.live_cards().collect::<Vec<_>>() {
            let h = self.card(host);
            let Some(hz) = h.zone else { continue };
            for r in &h.compiled().token_replacements {
                if r.active_zones.as_ref().is_some_and(|z| !z.contains(&hz))
                    || (r.effect_only && self.effect_source.is_none())
                    || r.valid_token
                        .as_ref()
                        .is_some_and(|v| !self.card_matches(token, v, host, h.controller))
                    || r.valid_player
                        .as_ref()
                        .is_some_and(|v| !self.player_matches_src(owner, v, h.controller, host))
                {
                    continue;
                }
                mult *= 2;
            }
        }
        mult
    }

    /// CR 101.2: Kann der Zauber `spell` (auf dem Stapel) nicht neutralisiert werden?
    pub fn cant_be_countered(&self, spell: CardId) -> bool {
        if self.card(spell).zone != Some(Zone::Stack) {
            return false;
        }
        self.live_cards().chain(std::iter::once(spell)).any(|hid| {
            let h = self.card(hid);
            let Some(hz) = h.zone else { return false };
            h.compiled().cant_counter.iter().any(|r| {
                r.active_zones.as_ref().is_none_or(|z| z.contains(&hz))
                    && r.valid_card
                        .as_ref()
                        .is_none_or(|v| self.card_matches(spell, v, hid, h.controller))
                    && self.card_matches(spell, &r.valid_spell, hid, h.controller)
                    && self.check_conditions(
                        &r.conditions,
                        &EvalCtx {
                            source: hid,
                            controller: h.controller,
                            x: 0,
                            targets: &[],
                            trig: None,
                            paid: &[],
                        },
                    )
            })
        })
    }

    /// CR 502.3 / 614: Wird `c` im Enttappsegment von `ap` am Enttappen gehindert?
    pub fn untap_prevented(&self, c: CardId, ap: crate::ids::PlayerId) -> bool {
        self.live_cards().any(|hid| {
            let h = self.card(hid);
            let Some(hz) = h.zone else { return false };
            // ValidStepTurnToController$ You: nur im Zug des Controllers der Quelle
            h.controller == ap
                && h.compiled().untap_replacements.iter().any(|r| {
                    r.active_zones.as_ref().is_none_or(|z| z.contains(&hz))
                        && self.card_matches(c, &r.valid_card, hid, h.controller)
                        && self.check_conditions(
                            &r.conditions,
                            &EvalCtx {
                                source: hid,
                                controller: h.controller,
                                x: 0,
                                targets: &[],
                                trig: None,
                                paid: &[],
                            },
                        )
                })
        })
    }
}

/// Was ein Schadens-Ersatzeffekt tut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DamageAction {
    /// `Prevent$ True`: der Schaden wird verhindert (CR 615).
    Prevent,
    /// `ReplaceEffect | VarName$ DamageAmount | VarValue$ X`: neue Menge; im
    /// Ausdruck steht `Count$xPaid` für die ursprüngliche Menge
    /// (`ReplaceCount$DamageAmount`).
    Modify(Expr),
    /// Verhindern, danach verlässt das Effektobjekt selbst die Befehlszone
    /// (`ReplaceWith$` → `ChangeZone | Defined$ Self | Origin$ Command | Destination$ Exile`):
    /// „das nächste Mal“.
    PreventOnce,
}

/// `R:Event$ DamageDone` (Java `ReplaceDamage`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DamageReplacement {
    pub valid_target: Option<ValidSpec>,
    pub valid_source: Option<ValidSpec>,
    /// `IsCombat$`
    pub combat: Option<bool>,
    pub active_zones: Option<Vec<Zone>>,
    pub conditions: Vec<Condition>,
    pub action: DamageAction,
    /// Verhinderungseffekt (`Prevent$`/`PreventionEffect$`): nach den übrigen
    /// Ersetzungen angewendet (Vereinfachung von CR 616.1).
    pub prevention: bool,
}

pub fn compile_damage_replacement(
    face: &CardFace,
    r: &ScriptReplacement,
) -> Result<DamageReplacement, CompileError> {
    let p = &r.params;
    let err = |k: &str, v: &str| CompileError::UnsupportedParam {
        api: "Replacement DamageDone".into(),
        key: k.into(),
        value: v.into(),
    };
    let mut used: Vec<&str> = NEUTRAL.to_vec();
    used.extend([
        "ValidTarget",
        "ValidSource",
        "IsCombat",
        "ActiveZones",
        "Prevent",
        "PreventionEffect",
        "AlwaysReplace",
    ]);
    let valid = |k: &str| {
        p.get(k)
            .map(ValidSpec::parse)
            .transpose()
            .map_err(CompileError::Valid)
    };
    let valid_target = valid("ValidTarget")?;
    let valid_source = valid("ValidSource")?;
    let combat = p.get("IsCombat").map(|v| v.eq_ignore_ascii_case("true"));
    let active_zones = match p.get("ActiveZones") {
        Some(z) => Some(zones(z).ok_or_else(|| err("ActiveZones", z))?),
        None => None,
    };
    let (conditions, ck) = parse_conditions(face, p, CondStyle::Requirement)?;
    used.extend(ck);
    for (k, v) in p.iter() {
        if !used.iter().any(|u| u.eq_ignore_ascii_case(k)) {
            return Err(err(k, v));
        }
    }
    let prevent = p.is_true("Prevent");
    let prevention = prevent || p.has("PreventionEffect");
    let action = if prevent {
        if p.has("ReplaceWith") {
            return Err(err("ReplaceWith", "mit Prevent"));
        }
        DamageAction::Prevent
    } else {
        let name = p
            .get("ReplaceWith")
            .ok_or_else(|| err("ReplaceWith", "fehlt"))?;
        let sv = face
            .svar(name)
            .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?;
        let ab = ScriptAbility::from_svar(sv)
            .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?
            .map_err(|_| CompileError::MissingSvar(name.to_string()))?;
        let ap = &ab.params;
        let only_known = ap.keys().all(|k| {
            [
                "DB",
                "VarName",
                "VarValue",
                "StackDescription",
                "SpellDescription",
            ]
            .iter()
            .any(|x| x.eq_ignore_ascii_case(k))
        });
        let exile_self = ab.api == "ChangeZone"
            && ap.get("Defined") == Some("Self")
            && ap.get("Origin") == Some("Command")
            && ap.get("Destination") == Some("Exile")
            && ap.keys().all(|k| {
                [
                    "DB",
                    "Defined",
                    "Origin",
                    "Destination",
                    "StackDescription",
                    "SpellDescription",
                ]
                .iter()
                .any(|x| x.eq_ignore_ascii_case(k))
            });
        if exile_self && prevention {
            return Ok(DamageReplacement {
                valid_target,
                valid_source,
                combat,
                active_zones,
                conditions,
                action: DamageAction::PreventOnce,
                prevention,
            });
        }
        if ab.api != "ReplaceEffect" || ap.get("VarName") != Some("DamageAmount") || !only_known {
            return Err(err("ReplaceWith", sv));
        }
        let var = ap.get("VarValue").ok_or_else(|| err("ReplaceWith", sv))?;
        let x = face
            .svar(var)
            .ok_or_else(|| CompileError::MissingSvar(var.to_string()))?;
        if !x.trim_start().starts_with("ReplaceCount$DamageAmount") {
            return Err(err("VarValue", x));
        }
        let expr = Expr::parse_svar(
            face,
            &x.replacen("ReplaceCount$DamageAmount", "Count$xPaid", 1),
        )?;
        DamageAction::Modify(expr)
    };
    Ok(DamageReplacement {
        valid_target,
        valid_source,
        combat,
        active_zones,
        conditions,
        action,
        prevention,
    })
}

/// `K:etbCounter:<Typ>:<Anzahl>[:no Condition[:Text]]` → (Typ, Anzahl).
pub fn parse_etb_counter(face: &CardFace, kw: &str) -> Result<(String, Expr), CompileError> {
    let parts: Vec<&str> = kw.split(':').collect();
    let err = || CompileError::UnsupportedParam {
        api: "etbCounter".into(),
        key: "K".into(),
        value: kw.into(),
    };
    if parts.len() < 3 || parts[1].starts_with("EACH") {
        return Err(err());
    }
    let amount = Expr::parse_value(face, parts[2]).map_err(|_| err())?;
    match parts.get(3).map(|c| c.trim()) {
        None | Some("no Condition") | Some("no condition") | Some("") => {
            Ok((parts[1].to_string(), amount))
        }
        // `CheckSVar$ X[ | SVarCompare$ LT2]` (Java etbCounter-Bedingung, Standard GE1)
        Some(c) if c.starts_with("CheckSVar$") => {
            let pm = forge_script::ParamMap::parse(c);
            let check = Expr::parse_value(face, pm.get("CheckSVar").unwrap_or(""))?;
            let cmp = crate::expr::Compare::parse(face, pm.get("SVarCompare").unwrap_or("GE1"))?;
            Ok((
                parts[1].to_string(),
                Expr::IfCmp(Box::new(check), Box::new(cmp), Box::new(amount)),
            ))
        }
        Some(_) => Err(err()),
    }
}

/// Was beim Zonenwechsel geändert wird.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MoveMods {
    pub tapped: bool,
    pub counters: Vec<(String, u32)>,
}

impl GameState {
    /// CR 614/615: Schadens-Ersatz- und Verhinderungseffekte. Liefert die
    /// tatsächlich zuzufügende Menge. Jeder Effekt wirkt höchstens einmal
    /// (CR 614.5); erst Mengenänderungen, dann Verhinderung, jeweils nach
    /// Zeitstempel (statt Wahl des Betroffenen, CR 616.1).
    pub(crate) fn replace_damage(
        &mut self,
        source: CardId,
        target: crate::decision::Target,
        amount: u32,
        combat: bool,
    ) -> u32 {
        use crate::decision::Target;
        if combat && self.this_turn.prevent_combat_damage {
            return 0;
        }
        // CR 702.16c: Schaden von Quellen mit der Eigenschaft wird verhindert
        if let Target::Card(c) = target
            && self.protected_from(c, source)
        {
            return 0;
        }
        let mut hosts: Vec<(u64, CardId)> = self
            .live_cards()
            .map(|id| self.card(id))
            .filter(|c| c.zone.is_some() && !c.compiled().damage_replacements.is_empty())
            .map(|c| (c.timestamp, c.id))
            .collect();
        hosts.sort();
        let mut amount = amount as i32;
        let mut used_up: Vec<CardId> = Vec::new();
        for prevention_pass in [false, true] {
            for (_, host) in &hosts {
                let h = self.card(*host);
                let hzone = h.zone.expect("gefiltert");
                let ctrl = h.controller;
                for r in &h.compiled().damage_replacements {
                    if amount <= 0 {
                        break;
                    }
                    if r.prevention != prevention_pass
                        || r.active_zones.as_ref().is_some_and(|z| !z.contains(&hzone))
                        || r.combat.is_some_and(|c| c != combat)
                    {
                        continue;
                    }
                    let tgt_ok = r.valid_target.as_ref().is_none_or(|v| match target {
                        Target::Player(p) => self.player_matches(p, v, ctrl),
                        Target::Card(c) => self.card_matches(c, v, *host, ctrl),
                    });
                    let src_ok = r
                        .valid_source
                        .as_ref()
                        .is_none_or(|v| self.card_matches(source, v, *host, ctrl));
                    if !tgt_ok || !src_ok {
                        continue;
                    }
                    let ctx = EvalCtx {
                        source: *host,
                        controller: ctrl,
                        x: amount.max(0) as u32,
                        targets: &[],
                        trig: None,
                        paid: &[],
                    };
                    if !self.check_conditions(&r.conditions, &ctx) {
                        continue;
                    }
                    if used_up.contains(host) {
                        continue;
                    }
                    amount = match &r.action {
                        DamageAction::Prevent => 0,
                        DamageAction::PreventOnce => {
                            used_up.push(*host);
                            0
                        }
                        DamageAction::Modify(e) => self.eval(e, &ctx),
                    };
                }
            }
            if !prevention_pass {
                // CR 615.7: Schilde „die nächsten N Schaden verhindern“
                continue;
            }
            let shield = match target {
                Target::Player(p) => &mut self.player_mut(p).prevent_next,
                Target::Card(c) => &mut self.card_mut(c).prevent_next,
            };
            let used = (*shield).min(amount.max(0) as u32);
            *shield -= used;
            amount -= used as i32;
        }
        // „das nächste Mal“: verbrauchte Effektobjekte verlassen die Befehlszone
        for h in used_up {
            if self.card(h).zone == Some(Zone::Command) {
                self.move_card(h, Zone::Exile, true);
            }
        }
        amount.max(0) as u32
    }

    /// Wendet Ersatzeffekte auf den Zonenwechsel von `id` an (CR 614).
    /// Liefert die tatsächliche Zielzone und Änderungen für das neue Objekt.
    pub(crate) fn replace_move(
        &self,
        id: CardId,
        from: Option<Zone>,
        to: Zone,
    ) -> (Zone, MoveMods) {
        let mut dest = to;
        let mut mods = MoveMods::default();
        let mut redirected = false;
        let moving = self.card(id);
        let mover_ctrl = moving.controller;

        let mut hosts: Vec<(u64, CardId)> = self
            .live_cards()
            .map(|id| self.card(id))
            .filter(|c| c.zone.is_some() && !c.compiled().replacements.is_empty())
            .map(|c| (c.timestamp, c.id))
            .collect();
        hosts.sort();
        for (_, host) in hosts {
            let h = self.card(host);
            let hzone = h.zone.expect("gefiltert");
            for r in &h.compiled().replacements {
                if r.active_zones.as_ref().is_some_and(|z| !z.contains(&hzone)) {
                    continue;
                }
                if !r.destination.contains(&dest) {
                    continue;
                }
                if r.origin
                    .as_ref()
                    .is_some_and(|o| !from.is_some_and(|f| o.contains(&f)))
                {
                    continue;
                }
                if !self.card_matches(id, &r.valid_card, host, h.controller) {
                    continue;
                }
                let ctx = EvalCtx {
                    source: host,
                    controller: h.controller,
                    x: 0,
                    targets: &[],
                    trig: None,
                    paid: &[],
                };
                if !self.check_conditions(&r.conditions, &ctx) {
                    continue;
                }
                match &r.action {
                    MovedAction::Redirect(z) if !redirected => {
                        dest = *z;
                        redirected = true; // CR 614.5: einmal je Ereignis
                    }
                    MovedAction::EnterTapped(conds) if dest == Zone::Battlefield => {
                        let ctx = EvalCtx {
                            source: id,
                            controller: mover_ctrl,
                            x: 0,
                            targets: &[],
                            trig: None,
                            paid: &[],
                        };
                        if self.check_conditions(conds, &ctx) {
                            mods.tapped = true;
                        }
                    }
                    _ => {}
                }
            }
        }
        // etbCounter des eintretenden Objekts selbst
        if dest == Zone::Battlefield {
            let ctx = EvalCtx {
                source: id,
                controller: mover_ctrl,
                x: moving.cast_x,
                targets: &[],
                trig: None,
                paid: &[],
            };
            for (kind, amount) in &moving.compiled().etb_counters {
                let n = self.eval(amount, &ctx).max(0) as u32;
                if n > 0 {
                    mods.counters.push((kind.clone(), n));
                }
            }
        }
        (dest, mods)
    }
}
