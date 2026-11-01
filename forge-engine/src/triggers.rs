//! Ausgelöste Fähigkeiten (CR 603).
//!
//! Java-Vorbild: `forge.game.trigger.TriggerHandler`, `Trigger`,
//! `TriggerChangesZone`, `TriggerPhase`, `TriggerAttacks`,
//! `TriggerSpellAbilityCastOrCopy`, `TriggerDamageDone`.
//!
//! Ablauf:
//! 1. Spielaktionen melden ein [`TriggerEvent`] über [`GameState::fire`].
//! 2. `fire` prüft sofort (mit dem Zustand zum Ereigniszeitpunkt) alle
//!    Trigger aller Objekte in ihren aktiven Zonen und merkt passende als
//!    [`PendingTrigger`] vor. Für „verlässt das Battlefield“ wird
//!    zurückgeschaut (CR 603.10a): das alte Objekt und alle Objekte, die im
//!    selben Schritt gleichzeitig das Battlefield verlassen, zählen mit.
//! 3. Bevor ein Spieler Priorität erhält, legt
//!    [`GameState::put_triggers_on_stack`] alle vorgemerkten Trigger in
//!    APNAP-Reihenfolge auf den Stapel (CR 603.3b); jeder Spieler ordnet
//!    seine eigenen. Ziele werden dabei gewählt (CR 603.3d); ohne legale
//!    Ziele wird der Trigger entfernt.
//! 4. Beim Auflösen fragen optionale Trigger („you may“) ihren Controller.

use crate::decision::{Choice, Decision, Target};
use crate::effects::{CompileError, CompiledAbility, compile};
use crate::ids::{CardId, PlayerId};
use crate::state::{Choices, Flow, GameState, Res, StackItem, StackKind};
use crate::turn::Step;
use crate::valid::ValidSpec;
use crate::zone::Zone;
use forge_script::{CardFace, ScriptAbility, ScriptTrigger};
use std::sync::Arc;

/// Auslösebedingung nach Modus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriggerKind {
    /// `Mode$ ChangesZone`. `None` bei Origin/Destination = `Any`.
    ChangesZone {
        origin: Option<Vec<Zone>>,
        destination: Option<Vec<Zone>>,
        valid_card: ValidSpec,
    },
    /// `Mode$ Phase` – zu Beginn eines Schritts.
    Phase {
        steps: Vec<Step>,
        valid_player: Option<ValidSpec>,
    },
    /// `Mode$ Attacks` – eine Kreatur greift an.
    Attacks { valid_card: ValidSpec },
    /// `Mode$ SpellCast` – ein Zauber wurde gewirkt.
    SpellCast {
        valid_card: Option<ValidSpec>,
        valid_activator: Option<ValidSpec>,
    },
    /// `Mode$ DamageDone` – Schaden wurde zugefügt.
    DamageDone {
        valid_source: Option<ValidSpec>,
        valid_target: Option<ValidSpec>,
        combat: Option<bool>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledTrigger {
    pub kind: TriggerKind,
    pub execute: Arc<CompiledAbility>,
    /// Zonen, in denen die Quelle sein muss, damit der Trigger aktiv ist.
    pub active_zones: Vec<Zone>,
    /// `OptionalDecider$ You`: „you may“.
    pub optional: bool,
    /// Bedingungen (CR 603.4 „intervening if“), geprüft beim Auslösen und Auflösen.
    pub conditions: Vec<crate::expr::Condition>,
    /// `NoResolvingCheck$ True`: beim Auflösen nicht erneut prüfen.
    pub recheck_on_resolve: bool,
    pub description: String,
}

/// Was beim Auslösen bekannt war (Java: `runParams`, „Triggered…“-Defined).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TriggerInfo {
    /// Betroffenes Objekt nach dem Ereignis (z. B. neues Objekt nach Zonenwechsel).
    pub card: Option<CardId>,
    /// Objekt vor dem Ereignis (last known information).
    pub card_lki: Option<CardId>,
    pub player: Option<PlayerId>,
    pub source: Option<CardId>,
    pub target: Option<Target>,
}

/// Ein Spielereignis, das Trigger auslösen kann.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriggerEvent {
    ChangesZone {
        old: CardId,
        new: CardId,
        from: Option<Zone>,
        to: Zone,
    },
    StepBegan {
        step: Step,
        active: PlayerId,
    },
    Attacks {
        attacker: CardId,
        defender: PlayerId,
    },
    SpellCast {
        card: CardId,
        player: PlayerId,
    },
    DamageDone {
        source: CardId,
        target: Target,
        amount: u32,
        combat: bool,
    },
}

/// Ausgelöster, noch nicht auf den Stapel gelegter Trigger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingTrigger {
    pub trigger: Arc<CompiledTrigger>,
    pub source: CardId,
    pub controller: PlayerId,
    pub info: TriggerInfo,
}

// ---------------------------------------------------------------------------
// Kompilieren
// ---------------------------------------------------------------------------

fn zones(s: &str) -> Result<Option<Vec<Zone>>, String> {
    if s.trim() == "Any" {
        return Ok(None);
    }
    s.split(',')
        .map(|z| Zone::parse(z).ok_or_else(|| z.to_string()))
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn steps(s: &str) -> Option<Vec<Step>> {
    s.split(',')
        .map(|p| {
            Step::ALL
                .into_iter()
                .find(|st| st.script_name() == p.trim())
        })
        .collect()
}

const TRIGGER_NEUTRAL: &[&str] = &[
    "Mode",
    "Execute",
    "TriggerZones",
    "TriggerDescription",
    "Secondary",
    "NoResolvingCheck",
    "PrecostDesc",
    "OptionalDecider",
];

pub fn compile_trigger(
    face: &CardFace,
    t: &ScriptTrigger,
) -> Result<CompiledTrigger, CompileError> {
    let p = &t.params;
    let unsupported = |k: &str, v: &str| CompileError::UnsupportedParam {
        api: format!("Trigger {}", t.mode),
        key: k.into(),
        value: v.into(),
    };
    let mut used: Vec<&str> = TRIGGER_NEUTRAL.to_vec();
    let valid = |k: &'static str,
                 used: &mut Vec<&'static str>|
     -> Result<Option<ValidSpec>, CompileError> {
        used.push(k);
        p.get(k)
            .map(ValidSpec::parse)
            .transpose()
            .map_err(CompileError::Valid)
    };

    let kind = match t.mode.as_str() {
        "ChangesZone" => {
            used.extend(["Origin", "Destination"]);
            let origin = match p.get("Origin") {
                Some(o) => zones(o).map_err(|z| unsupported("Origin", &z))?,
                None => None,
            };
            let destination = match p.get("Destination") {
                Some(d) => zones(d).map_err(|z| unsupported("Destination", &z))?,
                None => None,
            };
            let valid_card = valid("ValidCard", &mut used)?.ok_or(CompileError::MissingParam {
                api: "Trigger ChangesZone".into(),
                key: "ValidCard",
            })?;
            TriggerKind::ChangesZone {
                origin,
                destination,
                valid_card,
            }
        }
        "Phase" => {
            used.push("Phase");
            let ph = p.get("Phase").ok_or(CompileError::MissingParam {
                api: "Trigger Phase".into(),
                key: "Phase",
            })?;
            let steps = steps(ph).ok_or_else(|| unsupported("Phase", ph))?;
            TriggerKind::Phase {
                steps,
                valid_player: valid("ValidPlayer", &mut used)?,
            }
        }
        "Attacks" => {
            let valid_card = valid("ValidCard", &mut used)?.ok_or(CompileError::MissingParam {
                api: "Trigger Attacks".into(),
                key: "ValidCard",
            })?;
            TriggerKind::Attacks { valid_card }
        }
        "SpellCast" => TriggerKind::SpellCast {
            valid_card: valid("ValidCard", &mut used)?,
            valid_activator: valid("ValidActivatingPlayer", &mut used)?,
        },
        "DamageDone" => {
            used.push("CombatDamage");
            let combat = p
                .get("CombatDamage")
                .map(|v| v.eq_ignore_ascii_case("true"));
            TriggerKind::DamageDone {
                valid_source: valid("ValidSource", &mut used)?,
                valid_target: valid("ValidTarget", &mut used)?,
                combat,
            }
        }
        other => return Err(CompileError::UnsupportedApi(format!("Trigger {other}"))),
    };

    let (conditions, cond_keys) =
        crate::expr::parse_conditions(face, p, crate::expr::CondStyle::Requirement)?;
    used.extend(cond_keys);

    for (k, v) in p.iter() {
        if !used.iter().any(|u| u.eq_ignore_ascii_case(k)) {
            return Err(unsupported(k, v));
        }
    }

    let optional = match p.get("OptionalDecider") {
        None => false,
        Some("You") => true,
        Some(o) => return Err(unsupported("OptionalDecider", o)),
    };
    let exec_name = p.get("Execute").ok_or(CompileError::MissingParam {
        api: format!("Trigger {}", t.mode),
        key: "Execute",
    })?;
    let sv = face
        .svar(exec_name)
        .ok_or_else(|| CompileError::MissingSvar(exec_name.to_string()))?;
    let ab: ScriptAbility = ScriptAbility::from_svar(sv)
        .ok_or_else(|| CompileError::MissingSvar(exec_name.to_string()))?
        .map_err(|_| CompileError::MissingSvar(exec_name.to_string()))?;
    let execute = Arc::new(compile(face, &ab)?);

    // Aktive Zonen wie Trigger.java / TriggerChangesZone.setTriggerZones
    let mut active_zones = match p.get("TriggerZones") {
        Some(z) => zones(z)
            .map_err(|z| unsupported("TriggerZones", &z))?
            .unwrap_or_default(),
        None => vec![Zone::Battlefield],
    };
    if let TriggerKind::ChangesZone {
        origin,
        destination,
        valid_card,
    } = &kind
    {
        let self_ref = p.get("ValidCard").is_some_and(|v| v.contains("Self"));
        if origin
            .as_ref()
            .is_some_and(|o| o.contains(&Zone::Battlefield))
        {
            active_zones = vec![Zone::Battlefield];
        }
        if self_ref
            && origin.is_none()
            && let Some(d) = destination
        {
            active_zones = d.clone();
        }
        let _ = valid_card;
    }

    Ok(CompiledTrigger {
        kind,
        execute,
        active_zones,
        optional,
        conditions,
        recheck_on_resolve: !p
            .get("NoResolvingCheck")
            .is_some_and(|v| v.eq_ignore_ascii_case("true")),
        description: p.get("TriggerDescription").unwrap_or("").to_string(),
    })
}

// ---------------------------------------------------------------------------
// Auslösen
// ---------------------------------------------------------------------------

impl GameState {
    fn trigger_matches(
        &self,
        t: &CompiledTrigger,
        source: CardId,
        controller: PlayerId,
        ev: &TriggerEvent,
    ) -> Option<TriggerInfo> {
        match (&t.kind, ev) {
            (
                TriggerKind::ChangesZone {
                    origin,
                    destination,
                    valid_card,
                },
                TriggerEvent::ChangesZone { old, new, from, to },
            ) => {
                if let Some(o) = origin
                    && !from.is_some_and(|f| o.contains(&f))
                {
                    return None;
                }
                if let Some(d) = destination
                    && !d.contains(to)
                {
                    return None;
                }
                // Verlassen des Battlefields: Eigenschaften des alten Objekts (CR 603.10a).
                let subject = if *from == Some(Zone::Battlefield) {
                    *old
                } else {
                    *new
                };
                let self_ok = source == *old || source == *new;
                let matches = self.card_matches(subject, valid_card, source, controller)
                    || (self_ok && self.card_matches(subject, valid_card, subject, controller));
                matches.then(|| TriggerInfo {
                    card: Some(*new),
                    card_lki: Some(*old),
                    ..Default::default()
                })
            }
            (
                TriggerKind::Phase {
                    steps,
                    valid_player,
                },
                TriggerEvent::StepBegan { step, active },
            ) => {
                if !steps.contains(step) {
                    return None;
                }
                if let Some(vp) = valid_player
                    && !self.player_matches(*active, vp, controller)
                {
                    return None;
                }
                Some(TriggerInfo {
                    player: Some(*active),
                    ..Default::default()
                })
            }
            (TriggerKind::Attacks { valid_card }, TriggerEvent::Attacks { attacker, defender }) => {
                self.card_matches(*attacker, valid_card, source, controller)
                    .then(|| TriggerInfo {
                        card: Some(*attacker),
                        card_lki: Some(*attacker),
                        player: Some(*defender),
                        ..Default::default()
                    })
            }
            (
                TriggerKind::SpellCast {
                    valid_card,
                    valid_activator,
                },
                TriggerEvent::SpellCast { card, player },
            ) => {
                if valid_card
                    .as_ref()
                    .is_some_and(|v| !self.card_matches(*card, v, source, controller))
                {
                    return None;
                }
                if valid_activator
                    .as_ref()
                    .is_some_and(|v| !self.player_matches(*player, v, controller))
                {
                    return None;
                }
                Some(TriggerInfo {
                    card: Some(*card),
                    card_lki: Some(*card),
                    player: Some(*player),
                    ..Default::default()
                })
            }
            (
                TriggerKind::DamageDone {
                    valid_source,
                    valid_target,
                    combat,
                },
                TriggerEvent::DamageDone {
                    source: dsrc,
                    target,
                    amount,
                    combat: is_combat,
                },
            ) => {
                if *amount == 0 || combat.is_some_and(|c| c != *is_combat) {
                    return None;
                }
                if valid_source
                    .as_ref()
                    .is_some_and(|v| !self.card_matches(*dsrc, v, source, controller))
                {
                    return None;
                }
                if let Some(v) = valid_target {
                    let ok = match target {
                        Target::Player(p) => self.player_matches(*p, v, controller),
                        Target::Card(c) => self.card_matches(*c, v, source, controller),
                    };
                    if !ok {
                        return None;
                    }
                }
                let player = match target {
                    Target::Player(p) => Some(*p),
                    Target::Card(c) => Some(self.card(*c).controller),
                };
                Some(TriggerInfo {
                    card: Some(*dsrc),
                    card_lki: Some(*dsrc),
                    source: Some(*dsrc),
                    target: Some(*target),
                    player,
                })
            }
            _ => None,
        }
    }

    /// Meldet ein Ereignis; passende Trigger werden vorgemerkt.
    pub fn fire(&mut self, ev: TriggerEvent) {
        // Kandidaten: Objekte in Zonen + Rückblick bei Verlassen des Battlefields.
        let mut sources: Vec<(CardId, Zone)> = self
            .live_cards()
            .map(|id| self.card(id))
            .filter(|c| !c.compiled().triggers.is_empty())
            .filter_map(|c| c.zone.map(|z| (c.id, z)))
            .collect();
        if let TriggerEvent::ChangesZone {
            old,
            from: Some(Zone::Battlefield),
            ..
        } = &ev
        {
            let mut lki: Vec<CardId> = self.lki_battlefield.clone().unwrap_or_default();
            lki.push(*old);
            for id in lki {
                let c = self.card(id);
                if c.zone.is_none()
                    && !c.compiled().triggers.is_empty()
                    && !sources.iter().any(|(s, _)| *s == id)
                {
                    sources.push((id, Zone::Battlefield));
                }
            }
        }
        let mut found = Vec::new();
        for (src, zone) in sources {
            let card = self.card(src);
            let controller = card.controller;
            for t in &card.compiled().triggers {
                if !t.active_zones.contains(&zone) {
                    continue;
                }
                let ectx = crate::expr::EvalCtx {
                    source: src,
                    controller,
                    x: 0,
                };
                if let Some(info) = self
                    .trigger_matches(t, src, controller, &ev)
                    .filter(|_| self.check_conditions(&t.conditions, &ectx))
                {
                    found.push(PendingTrigger {
                        trigger: t.clone(),
                        source: src,
                        controller,
                        info,
                    });
                }
            }
        }
        self.pending_triggers.extend(found);
    }

    /// CR 603.3b: vorgemerkte Trigger in APNAP-Reihenfolge auf den Stapel.
    /// Liefert `true`, wenn mindestens einer gelegt wurde.
    pub fn put_triggers_on_stack(&mut self, ch: &mut Choices) -> Res<bool> {
        if self.pending_triggers.is_empty() {
            return Ok(false);
        }
        let pending = std::mem::take(&mut self.pending_triggers);
        let mut any = false;
        for p in self.apnap() {
            let mut mine: Vec<PendingTrigger> = pending
                .iter()
                .filter(|t| t.controller == p)
                .cloned()
                .collect();
            if mine.len() > 1 {
                let descriptions = mine.iter().map(|t| t.trigger.description.clone()).collect();
                if let Choice::Order(order) = ch.ask(Decision::OrderTriggers {
                    player: p,
                    descriptions,
                })? {
                    mine = order.into_iter().map(|i| mine[i].clone()).collect();
                }
            }
            for t in mine {
                let ab = t.trigger.execute.clone();
                // CR 603.3d: ohne legale Ziele (bzw. wählbare Modi) wird der Trigger entfernt.
                if !self.has_targets_for(&ab, t.source, p) {
                    continue;
                }
                // CR 603.3c: Modi beim Stapeln wählen
                let ab = self.choose_modes(p, t.source, &ab, ch)?;
                let targets = self.choose_targets(p, t.source, &ab, ch)?;
                let id = self.next_stack_id;
                self.next_stack_id += 1;
                self.stack.push(StackItem {
                    id,
                    kind: StackKind::Trigger {
                        info: t.info.clone(),
                        optional: t.trigger.optional,
                        description: t.trigger.description.clone(),
                        conditions: if t.trigger.recheck_on_resolve {
                            t.trigger.conditions.clone()
                        } else {
                            Vec::new()
                        },
                    },
                    source: t.source,
                    controller: p,
                    ability: Some(ab),
                    targets,
                    x: 0,
                });
                any = true;
            }
        }
        if any && let Flow::Priority { player, .. } = self.flow {
            self.flow = Flow::Priority { player, passes: 0 };
        }
        Ok(any)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_script::CardScript;
    use forge_types::TypeRegistry;

    fn trig(txt: &str) -> Result<CompiledTrigger, CompileError> {
        let c = CardScript::parse(txt, &TypeRegistry::default()).unwrap();
        compile_trigger(c.main(), &c.main().triggers[0])
    }

    #[test]
    fn etb_and_dies_zones() {
        let etb = trig("Name:E\nManaCost:1\nTypes:Creature\nPT:1/1\nT:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDraw | TriggerDescription$ x\nSVar:TrigDraw:DB$ Draw | NumCards$ 1\n").unwrap();
        assert_eq!(etb.active_zones, vec![Zone::Battlefield]);
        let dies = trig("Name:D\nManaCost:1\nTypes:Creature\nPT:1/1\nT:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigDraw\nSVar:TrigDraw:DB$ Draw | NumCards$ 1\n").unwrap();
        assert_eq!(dies.active_zones, vec![Zone::Battlefield]);
        let up = trig("Name:U\nManaCost:1\nTypes:Enchantment\nT:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigDraw | OptionalDecider$ You\nSVar:TrigDraw:DB$ Draw | NumCards$ 1\n").unwrap();
        assert!(up.optional);
        assert!(trig("Name:X\nManaCost:1\nTypes:Enchantment\nT:Mode$ Phase | Phase$ Upkeep | CheckSVar$ X | Execute$ TrigDraw\nSVar:TrigDraw:DB$ Draw\n").is_err());
    }
}
