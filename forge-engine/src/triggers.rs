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
    /// `Mode$ Attacks` – eine Kreatur greift an (`Alone$`: als einziger Angreifer).
    Attacks {
        valid_card: ValidSpec,
        alone: Option<bool>,
    },
    /// `Mode$ SpellCast` – ein Zauber wurde gewirkt.
    SpellCast {
        valid_card: Option<ValidSpec>,
        valid_activator: Option<ValidSpec>,
        /// `TargetsValid$`: der Zauber zielt auf etwas Passendes (Heroic u. ä.)
        targets_valid: Option<ValidSpec>,
        /// `ActivatorThisTurnCast$`: Anzahl passender Zauber des Wirkenden in diesem Zug
        activator_this_turn: Option<crate::expr::Compare>,
    },
    /// `Mode$ AttackersDeclared` – Angreifer wurden deklariert (CR 508.1m).
    AttackersDeclared {
        attacking_player: Option<ValidSpec>,
        attacked_target: Option<ValidSpec>,
        /// `ValidAttackers$` + `ValidAttackersAmount$` (Standard `GE1`).
        valid_attackers: Option<(ValidSpec, crate::expr::Compare)>,
        /// `Mode$ AttackersDeclaredOneTarget`: je angegriffenem Ziel einmal.
        one_target: bool,
    },
    /// `Mode$ Cycled` – eine Karte wurde umgewandelt (Cycling, CR 702.29).
    Cycled {
        valid_card: Option<ValidSpec>,
        valid_player: Option<ValidSpec>,
    },
    /// `Mode$ Discarded` – eine Karte wurde abgeworfen (CR 701.9).
    Discarded { valid_card: Option<ValidSpec> },
    /// `Mode$ AttackerBlockedByCreature` – je Paar aus Angreifer und Blocker.
    AttackerBlockedByCreature {
        valid_card: Option<ValidSpec>,
        valid_blocker: Option<ValidSpec>,
    },
    /// `Mode$ BecomesTarget` – ein Objekt/Spieler wird Ziel eines Zaubers/einer Fähigkeit.
    BecomesTarget {
        valid_target: Option<ValidSpec>,
        valid_source: Option<SaFilter>,
    },
    /// `Mode$ ChangesZoneAll` – ein oder mehrere Objekte wechselten gleichzeitig die Zone.
    ChangesZoneAll {
        origin: Option<Vec<Zone>>,
        destination: Option<Vec<Zone>>,
        valid_cards: Option<ValidSpec>,
        valid_amount: Option<crate::expr::Compare>,
    },
    /// `Mode$ DamageDoneOnce` – ein Empfänger erhielt gleichzeitig Schaden.
    DamageDoneOnce {
        valid_target: Option<ValidSpec>,
        valid_source: Option<ValidSpec>,
        combat: Option<bool>,
        amount: Option<crate::expr::Compare>,
    },
    /// `Mode$ DamageDealtOnce` – eine Quelle fügte gleichzeitig Schaden zu.
    DamageDealtOnce {
        valid_source: Option<ValidSpec>,
        valid_target: Option<ValidSpec>,
        combat: Option<bool>,
    },
    /// `Mode$ Blocks` – eine Kreatur blockt (CR 509.4).
    Blocks {
        valid_card: Option<ValidSpec>,
        valid_blocked: Option<ValidSpec>,
    },
    /// `Mode$ AttackerBlocked` – ein Angreifer wird geblockt.
    AttackerBlocked {
        valid_card: Option<ValidSpec>,
        valid_blockers: Option<(ValidSpec, crate::expr::Compare)>,
    },
    /// `Mode$ AttackerUnblocked` – ein Angreifer bleibt ungeblockt.
    AttackerUnblocked {
        valid_card: Option<ValidSpec>,
        valid_defender: Option<ValidSpec>,
    },
    /// `Mode$ Drawn` – eine Karte wurde gezogen (CR 121).
    Drawn {
        valid_card: Option<ValidSpec>,
        valid_player: Option<ValidSpec>,
        /// `Number$`: die n-te gezogene Karte dieses Spielers in diesem Zug.
        number: Option<u32>,
    },
    /// Reflexiver Trigger (`ImmediateTrigger`, CR 603.12); kein Ereignis.
    Immediate,
    /// `Mode$ CounterAdded` – eine Marke wurde auf eine Karte gelegt (je Marke
    /// einzeln, Java `TriggerCounterAdded`).
    CounterAdded {
        valid_card: Option<ValidSpec>,
        counter_type: Option<String>,
        /// `CounterAmount$`: Markenzahl nach dieser Marke (Sagas)
        amount: Option<crate::expr::Compare>,
    },
    /// `Mode$ LifeGained` – ein Spieler erhielt Leben.
    LifeGained { valid_player: Option<ValidSpec> },
    /// `Mode$ Taps` – eine bleibende Karte wird getappt.
    Taps {
        valid_card: Option<ValidSpec>,
        valid_player: Option<ValidSpec>,
    },
    /// `Mode$ CounterAddedOnce` – Marken wurden (in einem Ereignis) daraufgelegt.
    CounterAddedOnce {
        valid_card: Option<ValidSpec>,
        counter_type: Option<String>,
        valid_source: Option<ValidSpec>,
    },
    /// `Mode$ AbilityCast` – eine aktivierte Fähigkeit wurde aktiviert (CR 602).
    AbilityCast {
        valid_card: Option<ValidSpec>,
        valid_activator: Option<ValidSpec>,
        /// `ValidSA$ Activated.Loyalty`
        loyalty_only: bool,
    },
    /// `Mode$ TapsForMana` – getriggerte Mana-Fähigkeit (CR 605.1b), wird sofort
    /// ohne Stapel ausgeführt (`GameState::tapped_for_mana`).
    TapsForMana {
        valid_card: Option<ValidSpec>,
        activator: Option<ValidSpec>,
    },
    /// `Mode$ TurnFaceUp` – eine bleibende Karte wird umgedreht (CR 708.8).
    TurnFaceUp { valid_card: Option<ValidSpec> },
    /// `Mode$ Sacrificed` – eine bleibende Karte wurde geopfert (CR 701.21).
    Sacrificed {
        valid_card: Option<ValidSpec>,
        valid_player: Option<ValidSpec>,
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
    /// Wer bei `optional` entscheidet, falls nicht der Controller
    /// (`OptionalDecider$ TriggeredCardController` u. a.).
    pub decider: Option<crate::effects::Defined>,
    /// Bedingungen (CR 603.4 „intervening if“), geprüft beim Auslösen und Auflösen.
    pub conditions: Vec<crate::expr::Condition>,
    /// `NoResolvingCheck$ True`: beim Auflösen nicht erneut prüfen.
    pub recheck_on_resolve: bool,
    pub description: String,
    /// Kapitelfähigkeit einer Saga (`K:Chapter`, CR 714.2).
    pub chapter: bool,
}

/// Filter für Zaubersprüche/Fähigkeiten (`ValidSource$ SpellAbility.OppCtrl`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaFilter {
    /// `Some(true)` = nur Zaubersprüche, `Some(false)` = nur Fähigkeiten
    pub spell: Option<bool>,
    /// `Some(true)` = du kontrollierst, `Some(false)` = ein Gegner
    pub yours: Option<bool>,
}

impl SaFilter {
    fn parse(s: &str) -> Option<SaFilter> {
        let (base, props) = match s.split_once('.') {
            Some((b, p)) => (b, Some(p)),
            None => (s, None),
        };
        let spell = match base {
            "SpellAbility" | "Card" => None,
            "Spell" => Some(true),
            "Ability" | "Activated" => Some(false),
            _ => return None,
        };
        let yours = match props {
            None => None,
            Some("YouCtrl") => Some(true),
            Some("OppCtrl") => Some(false),
            Some(_) => return None,
        };
        Some(SaFilter { spell, yours })
    }
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
    /// Angreifender Spieler (`TriggeredAttackingPlayer`).
    pub attacking_player: Option<PlayerId>,
    /// Vom verzögerten Trigger gemerkte Objekte (`DelayTriggerRemembered…`).
    pub delay_remembered: Vec<Target>,
    /// Betroffene Objekte bei Sammel-Triggern (`TriggeredCards`).
    pub cards: Vec<CardId>,
    /// Auslösendes Stapelobjekt (`TriggeredSourceSA`).
    pub stack_item: Option<u32>,
    /// Blocker (`TriggeredBlocker`), wenn `card` der Angreifer ist.
    pub blocker: Option<CardId>,
    /// Erzeugtes Mana (`TapsForMana`, für `ManaReflected | ReflectProperty$ Produced`).
    pub produced: Vec<Option<forge_types::Color>>,
    /// Menge des Ereignisses (`TriggerCount$DamageAmount`, `…$LifeAmount`).
    pub amount: Option<i32>,
}

/// Verzögerter Trigger (CR 603.7), angelegt vom Effekt `DelayedTrigger`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelayedTrigger {
    pub trigger: Arc<CompiledTrigger>,
    pub source: CardId,
    pub controller: PlayerId,
    pub remembered: Vec<Target>,
    /// aktiv ab diesem Zug …
    pub from_turn: u32,
    /// … und bis einschließlich diesem Zug (`None` = bis zum Auslösen).
    pub until_turn: Option<u32>,
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
        /// verteidigender Spieler
        defender: PlayerId,
        /// angegriffener Spieler oder Planeswalker
        target: Target,
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
    /// Alle Angreifer eines Spielers mit ihren Verteidigern (einmal je Kampf).
    AttackersDeclared {
        player: PlayerId,
        attackers: Vec<(CardId, Target)>,
        /// Variante je Ziel (alle `attackers` greifen dasselbe Ziel an).
        one_target: bool,
    },
    /// `card` ist das Objekt in der Hand vor dem Abwerfen (LKI).
    Cycled {
        card: CardId,
        player: PlayerId,
    },
    Discarded {
        old: CardId,
        new: CardId,
        player: PlayerId,
    },
    AttackerBlockedByCreature {
        attacker: CardId,
        blocker: CardId,
        defender: PlayerId,
    },
    BecomesTarget {
        target: Target,
        source: CardId,
        stack_item: u32,
        is_spell: bool,
        controller: PlayerId,
    },
    /// Gleichzeitige Zonenwechsel (alt, neu, von, nach).
    ChangesZoneAll {
        moves: Vec<crate::state::ZoneMove>,
    },
    /// Gleichzeitiger Schaden einer Quelle (Empfänger mit Mengen).
    DamageDealtOnce {
        source: CardId,
        targets: Vec<(Target, u32)>,
        combat: bool,
    },
    /// Gleichzeitiger Schaden an einen Empfänger (Quellen mit Mengen).
    DamageDoneOnce {
        target: Target,
        sources: Vec<(CardId, u32)>,
        combat: bool,
    },
    Blocks {
        blocker: CardId,
        attackers: Vec<CardId>,
    },
    AttackerBlocked {
        attacker: CardId,
        blockers: Vec<CardId>,
        defender: PlayerId,
    },
    AttackerUnblocked {
        attacker: CardId,
        defender: PlayerId,
        target: Target,
    },
    Drawn {
        card: CardId,
        player: PlayerId,
        number: u32,
    },
    LifeGained {
        player: PlayerId,
        amount: u32,
    },
    /// Eine Marke vom Typ `kind` kam hinzu; `total` = Anzahl danach.
    CounterAdded {
        card: CardId,
        kind: String,
        total: u32,
    },
    Taps {
        card: CardId,
    },
    TurnFaceUp {
        card: CardId,
    },
    CounterAddedOnce {
        card: CardId,
        kind: String,
        amount: u32,
        /// Spieler, der die Marken legt
        player: PlayerId,
    },
    AbilityCast {
        source: CardId,
        player: PlayerId,
        loyalty: bool,
        stack_item: u32,
    },
    /// `old` ist das geopferte Objekt (LKI), `player` wer geopfert hat.
    Sacrificed {
        old: CardId,
        player: PlayerId,
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
            used.push("Alone");
            let alone = p.get("Alone").map(|v| v.eq_ignore_ascii_case("true"));
            TriggerKind::Attacks { valid_card, alone }
        }
        "SpellCast" => TriggerKind::SpellCast {
            valid_card: valid("ValidCard", &mut used)?,
            valid_activator: valid("ValidActivatingPlayer", &mut used)?,
            targets_valid: valid("TargetsValid", &mut used)?,
            activator_this_turn: {
                used.push("ActivatorThisTurnCast");
                p.get("ActivatorThisTurnCast")
                    .map(|v| crate::expr::Compare::parse(face, v))
                    .transpose()?
            },
        },
        "AttackersDeclared" | "AttackersDeclaredOneTarget" => {
            used.push("ValidAttackersAmount");
            let valid_attackers = match valid("ValidAttackers", &mut used)? {
                Some(v) => Some((
                    v,
                    crate::expr::Compare::parse(
                        face,
                        p.get("ValidAttackersAmount").unwrap_or("GE1"),
                    )?,
                )),
                None if p.has("ValidAttackersAmount") => {
                    return Err(unsupported("ValidAttackersAmount", "ohne ValidAttackers"));
                }
                None => None,
            };
            TriggerKind::AttackersDeclared {
                attacking_player: valid("AttackingPlayer", &mut used)?,
                attacked_target: valid("AttackedTarget", &mut used)?,
                valid_attackers,
                one_target: t.mode == "AttackersDeclaredOneTarget",
            }
        }
        "Cycled" => TriggerKind::Cycled {
            valid_card: valid("ValidCard", &mut used)?,
            valid_player: valid("ValidPlayer", &mut used)?,
        },
        "Discarded" => TriggerKind::Discarded {
            valid_card: valid("ValidCard", &mut used)?,
        },
        "AttackerBlockedByCreature" => TriggerKind::AttackerBlockedByCreature {
            valid_card: valid("ValidCard", &mut used)?,
            valid_blocker: valid("ValidBlocker", &mut used)?,
        },
        "BecomesTarget" => {
            used.push("ValidSource");
            let valid_source = match p.get("ValidSource") {
                Some(v) => Some(SaFilter::parse(v).ok_or_else(|| unsupported("ValidSource", v))?),
                None => None,
            };
            TriggerKind::BecomesTarget {
                valid_target: valid("ValidTarget", &mut used)?,
                valid_source,
            }
        }
        "ChangesZoneAll" => {
            used.extend(["Origin", "Destination", "ValidAmount"]);
            let origin = match p.get("Origin") {
                Some(o) => zones(o).map_err(|z| unsupported("Origin", &z))?,
                None => None,
            };
            let destination = match p.get("Destination") {
                Some(d) => zones(d).map_err(|z| unsupported("Destination", &z))?,
                None => None,
            };
            let valid_amount = match p.get("ValidAmount") {
                Some(a) => Some(crate::expr::Compare::parse(face, a)?),
                None => None,
            };
            TriggerKind::ChangesZoneAll {
                origin,
                destination,
                valid_cards: valid("ValidCards", &mut used)?,
                valid_amount,
            }
        }
        "DamageDoneOnce" => {
            used.extend(["CombatDamage", "DamageAmount"]);
            let combat = p
                .get("CombatDamage")
                .map(|v| v.eq_ignore_ascii_case("true"));
            let amount = match p.get("DamageAmount") {
                Some(a) => Some(crate::expr::Compare::parse(face, a)?),
                None => None,
            };
            TriggerKind::DamageDoneOnce {
                valid_target: valid("ValidTarget", &mut used)?,
                valid_source: valid("ValidSource", &mut used)?,
                combat,
                amount,
            }
        }
        "DamageDealtOnce" => {
            used.push("CombatDamage");
            TriggerKind::DamageDealtOnce {
                valid_source: valid("ValidSource", &mut used)?,
                valid_target: valid("ValidTarget", &mut used)?,
                combat: p
                    .get("CombatDamage")
                    .map(|v| v.eq_ignore_ascii_case("true")),
            }
        }
        "Blocks" => TriggerKind::Blocks {
            valid_card: valid("ValidCard", &mut used)?,
            valid_blocked: valid("ValidBlocked", &mut used)?,
        },
        "AttackerBlocked" => {
            used.push("ValidBlockerAmount");
            let valid_blockers = match valid("ValidBlocker", &mut used)? {
                Some(v) => Some((
                    v,
                    crate::expr::Compare::parse(
                        face,
                        p.get("ValidBlockerAmount").unwrap_or("GE1"),
                    )?,
                )),
                None => None,
            };
            TriggerKind::AttackerBlocked {
                valid_card: valid("ValidCard", &mut used)?,
                valid_blockers,
            }
        }
        "AttackerUnblocked" => TriggerKind::AttackerUnblocked {
            valid_card: valid("ValidCard", &mut used)?,
            valid_defender: valid("ValidDefender", &mut used)?,
        },
        "Drawn" => {
            used.push("Number");
            let number = match p.get("Number") {
                Some(n) => Some(n.trim().parse().map_err(|_| unsupported("Number", n))?),
                None => None,
            };
            TriggerKind::Drawn {
                valid_card: valid("ValidCard", &mut used)?,
                valid_player: valid("ValidPlayer", &mut used)?,
                number,
            }
        }
        "CounterAdded" => {
            used.extend(["CounterType", "CounterAmount", "Chapter"]);
            TriggerKind::CounterAdded {
                valid_card: valid("ValidCard", &mut used)?,
                counter_type: p.get("CounterType").map(str::to_string),
                amount: p
                    .get("CounterAmount")
                    .map(|v| crate::expr::Compare::parse(face, v))
                    .transpose()?,
            }
        }
        "LifeGained" => TriggerKind::LifeGained {
            valid_player: valid("ValidPlayer", &mut used)?,
        },
        "CounterAddedOnce" => {
            used.push("CounterType");
            TriggerKind::CounterAddedOnce {
                valid_card: valid("ValidCard", &mut used)?,
                counter_type: p.get("CounterType").map(str::to_string),
                valid_source: valid("ValidSource", &mut used)?,
            }
        }
        "AbilityCast" => {
            used.push("ValidSA");
            let loyalty_only = match p.get("ValidSA") {
                None | Some("SpellAbility.!ManaAbility") | Some("Activated") => false,
                Some("Activated.Loyalty") => true,
                Some(v) => return Err(unsupported("ValidSA", v)),
            };
            TriggerKind::AbilityCast {
                valid_card: valid("ValidCard", &mut used)?,
                valid_activator: valid("ValidActivatingPlayer", &mut used)?,
                loyalty_only,
            }
        }
        "TapsForMana" => {
            used.push("Static");
            TriggerKind::TapsForMana {
                valid_card: valid("ValidCard", &mut used)?,
                activator: valid("Activator", &mut used)?,
            }
        }
        "TurnFaceUp" => TriggerKind::TurnFaceUp {
            valid_card: valid("ValidCard", &mut used)?,
        },
        "Taps" => TriggerKind::Taps {
            valid_card: valid("ValidCard", &mut used)?,
            valid_player: valid("ValidPlayer", &mut used)?,
        },
        // Reflexiver Trigger: wird nur vom Effekt `ImmediateTrigger` ausgelöst.
        "Immediate" => TriggerKind::Immediate,
        "Sacrificed" => TriggerKind::Sacrificed {
            valid_card: valid("ValidCard", &mut used)?,
            valid_player: valid("ValidPlayer", &mut used)?,
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

    use crate::effects::Defined as D;
    let (optional, decider) = match p.get("OptionalDecider") {
        None => (false, None),
        Some("You") | Some("True") => (true, None),
        Some("TriggeredCardController") => (true, Some(D::TriggeredCardController)),
        Some("TriggeredPlayer") | Some("TriggeredActivator") => (true, Some(D::TriggeredPlayer)),
        Some("TriggeredAttackingPlayer") => (true, Some(D::TriggeredAttackingPlayer)),
        Some("TargetedController") => (true, Some(D::TargetedController)),
        Some("EnchantedController") => (true, Some(D::EnchantedController)),
        Some("Opponent") => (true, Some(D::EachOpponent)),
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
    // „Wenn du ~ umwandelst/abwirfst“: die Karte ist dabei in der Hand.
    if matches!(
        kind,
        TriggerKind::Cycled { .. } | TriggerKind::Discarded { .. }
    ) && p.get("ValidCard").is_some_and(|v| v.contains("Self"))
        && !p.has("TriggerZones")
    {
        active_zones = vec![Zone::Hand];
    }

    Ok(CompiledTrigger {
        kind,
        execute,
        active_zones,
        optional,
        decider,
        conditions,
        recheck_on_resolve: !p
            .get("NoResolvingCheck")
            .is_some_and(|v| v.eq_ignore_ascii_case("true")),
        description: p.get("TriggerDescription").unwrap_or("").to_string(),
        chapter: false,
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
                    && !self.player_matches_src(*active, vp, controller, source)
                {
                    return None;
                }
                Some(TriggerInfo {
                    player: Some(*active),
                    ..Default::default()
                })
            }
            (
                TriggerKind::Attacks { valid_card, alone },
                TriggerEvent::Attacks {
                    attacker,
                    defender,
                    target,
                },
            ) => (self.card_matches(*attacker, valid_card, source, controller)
                && alone.is_none_or(|a| {
                    (self.combat.as_ref().map_or(0, |c| c.attackers.len()) == 1) == a
                }))
            .then(|| TriggerInfo {
                card: Some(*attacker),
                card_lki: Some(*attacker),
                player: Some(*defender),
                target: Some(*target),
                attacking_player: Some(self.card(*attacker).controller),
                ..Default::default()
            }),
            (
                TriggerKind::SpellCast {
                    valid_card,
                    valid_activator,
                    targets_valid,
                    activator_this_turn,
                },
                TriggerEvent::SpellCast { card, player },
            ) => {
                // Java: ThisTurnCast mit ValidCard, gezählt für den Wirkenden (inkl. diesem).
                if let Some(cmp) = activator_this_turn {
                    let n = self
                        .this_turn
                        .cast
                        .iter()
                        .filter(|c| {
                            self.card(**c).controller == *player
                                && valid_card
                                    .as_ref()
                                    .is_none_or(|v| self.card_matches(**c, v, source, controller))
                        })
                        .count() as i32;
                    if !cmp.holds(
                        n,
                        self.eval(
                            &cmp.rhs,
                            &crate::expr::EvalCtx {
                                source,
                                controller,
                                x: 0,
                                targets: &[],
                                trig: None,
                                paid: &[],
                            },
                        ),
                    ) {
                        return None;
                    }
                }
                // Java TriggerSpellAbilityCastOrCopy: irgendein Ziel der Kette passt.
                if let Some(v) = targets_valid {
                    let item = self.stack.iter().rev().find(
                        |s| matches!(s.kind, crate::state::StackKind::Spell { card: c } if c == *card),
                    );
                    let hit = item.is_some_and(|s| {
                        s.targets.iter().flatten().any(|t| match t {
                            Target::Card(c) => {
                                self.opt_card(&Some(v.clone()), *c, source, controller)
                            }
                            Target::Player(p) => self.player_matches_src(*p, v, controller, source),
                        })
                    });
                    if !hit {
                        return None;
                    }
                }
                if valid_card
                    .as_ref()
                    .is_some_and(|v| !self.card_matches(*card, v, source, controller))
                {
                    return None;
                }
                if valid_activator
                    .as_ref()
                    .is_some_and(|v| !self.player_matches_src(*player, v, controller, source))
                {
                    return None;
                }
                let stack_item = self
                    .stack
                    .iter()
                    .rev()
                    .find(|s| matches!(s.kind, crate::state::StackKind::Spell { card: c } if c == *card))
                    .map(|s| s.id);
                Some(TriggerInfo {
                    card: Some(*card),
                    card_lki: Some(*card),
                    player: Some(*player),
                    stack_item,
                    ..Default::default()
                })
            }
            (
                TriggerKind::AttackersDeclared {
                    attacking_player,
                    attacked_target,
                    valid_attackers,
                    one_target,
                },
                TriggerEvent::AttackersDeclared {
                    player,
                    attackers,
                    one_target: ev_one,
                },
            ) => {
                if one_target != ev_one {
                    return None;
                }
                if attacking_player
                    .as_ref()
                    .is_some_and(|v| !self.player_matches_src(*player, v, controller, source))
                {
                    return None;
                }
                // Java matchesValidParam auf einer Sammlung: einer genügt.
                if let Some(v) = attacked_target
                    && !attackers.iter().any(|(_, t)| match t {
                        Target::Player(p) => self.player_matches_src(*p, v, controller, source),
                        Target::Card(c) => self.card_matches(*c, v, source, controller),
                    })
                {
                    return None;
                }
                if let Some((v, cmp)) = valid_attackers {
                    let n = attackers
                        .iter()
                        .filter(|(c, _)| self.card_matches(*c, v, source, controller))
                        .count() as i32;
                    let ectx = crate::expr::EvalCtx {
                        source,
                        controller,
                        x: 0,
                        targets: &[],
                        trig: None,
                        paid: &[],
                    };
                    if !cmp.holds(n, self.eval(&cmp.rhs, &ectx)) {
                        return None;
                    }
                }
                Some(TriggerInfo {
                    attacking_player: Some(*player),
                    target: one_target
                        .then(|| attackers.first().map(|(_, t)| *t))
                        .flatten(),
                    ..Default::default()
                })
            }
            (
                TriggerKind::Cycled {
                    valid_card,
                    valid_player,
                },
                TriggerEvent::Cycled { card, player },
            ) => {
                if !self.opt_card(valid_card, *card, source, controller)
                    || valid_player
                        .as_ref()
                        .is_some_and(|v| !self.player_matches_src(*player, v, controller, source))
                {
                    return None;
                }
                Some(TriggerInfo {
                    card: Some(self.current_object(*card)),
                    card_lki: Some(*card),
                    player: Some(*player),
                    ..Default::default()
                })
            }
            (
                TriggerKind::Discarded { valid_card },
                TriggerEvent::Discarded { old, new, player },
            ) => {
                if !self.opt_card(valid_card, *old, source, controller) {
                    return None;
                }
                Some(TriggerInfo {
                    card: Some(*new),
                    card_lki: Some(*old),
                    player: Some(*player),
                    ..Default::default()
                })
            }
            (
                TriggerKind::AttackerBlockedByCreature {
                    valid_card,
                    valid_blocker,
                },
                TriggerEvent::AttackerBlockedByCreature {
                    attacker,
                    blocker,
                    defender,
                },
            ) => {
                if !self.opt_card(valid_card, *attacker, source, controller)
                    || !self.opt_card(valid_blocker, *blocker, source, controller)
                {
                    return None;
                }
                Some(TriggerInfo {
                    card: Some(*attacker),
                    card_lki: Some(*attacker),
                    blocker: Some(*blocker),
                    player: Some(*defender),
                    attacking_player: Some(self.card(*attacker).controller),
                    ..Default::default()
                })
            }
            (
                TriggerKind::BecomesTarget {
                    valid_target,
                    valid_source,
                },
                TriggerEvent::BecomesTarget {
                    target,
                    source: sa_source,
                    stack_item,
                    is_spell,
                    controller: sa_ctrl,
                },
            ) => {
                if let Some(f) = valid_source
                    && (f.spell.is_some_and(|s| s != *is_spell)
                        || f.yours.is_some_and(|y| y != (*sa_ctrl == controller)))
                {
                    return None;
                }
                if let Some(v) = valid_target {
                    let ok = match target {
                        Target::Player(p) => self.player_matches_src(*p, v, controller, source),
                        Target::Card(c) => self.opt_card(&Some(v.clone()), *c, source, controller),
                    };
                    if !ok {
                        return None;
                    }
                }
                Some(TriggerInfo {
                    target: Some(*target),
                    source: Some(*sa_source),
                    player: Some(*sa_ctrl),
                    stack_item: Some(*stack_item),
                    ..Default::default()
                })
            }
            (
                TriggerKind::ChangesZoneAll {
                    origin,
                    destination,
                    valid_cards,
                    valid_amount,
                },
                TriggerEvent::ChangesZoneAll { moves },
            ) => {
                let cards: Vec<CardId> = moves
                    .iter()
                    .filter(|(_, _, from, to)| {
                        origin
                            .as_ref()
                            .is_none_or(|o| from.is_some_and(|f| o.contains(&f)))
                            && destination.as_ref().is_none_or(|d| d.contains(to))
                    })
                    // Verlassen des Battlefields: Eigenschaften des alten Objekts (CR 603.10a)
                    .map(|(old, new, from, _)| {
                        if *from == Some(Zone::Battlefield) {
                            *old
                        } else {
                            *new
                        }
                    })
                    .filter(|c| self.opt_card(valid_cards, *c, source, controller))
                    .collect();
                if cards.is_empty() {
                    return None;
                }
                if let Some(cmp) = valid_amount {
                    let ectx = crate::expr::EvalCtx {
                        source,
                        controller,
                        x: 0,
                        targets: &[],
                        trig: None,
                        paid: &[],
                    };
                    if !cmp.holds(cards.len() as i32, self.eval(&cmp.rhs, &ectx)) {
                        return None;
                    }
                }
                Some(TriggerInfo {
                    cards,
                    ..Default::default()
                })
            }
            (
                TriggerKind::DamageDealtOnce {
                    valid_source,
                    valid_target,
                    combat,
                },
                TriggerEvent::DamageDealtOnce {
                    source: dsrc,
                    targets,
                    combat: is_combat,
                },
            ) => {
                if combat.is_some_and(|c| c != *is_combat)
                    || !self.opt_card(valid_source, *dsrc, source, controller)
                {
                    return None;
                }
                let total: u32 = targets
                    .iter()
                    .filter(|(t, _)| {
                        valid_target.as_ref().is_none_or(|v| match t {
                            Target::Player(p) => self.player_matches_src(*p, v, controller, source),
                            Target::Card(c) => self.card_matches(*c, v, source, controller),
                        })
                    })
                    .map(|(_, n)| *n)
                    .sum();
                if total == 0 {
                    return None;
                }
                Some(TriggerInfo {
                    card: Some(*dsrc),
                    card_lki: Some(*dsrc),
                    source: Some(*dsrc),
                    player: Some(self.card(*dsrc).controller),
                    amount: Some(total as i32),
                    ..Default::default()
                })
            }
            (
                TriggerKind::DamageDoneOnce {
                    valid_target,
                    valid_source,
                    combat,
                    amount,
                },
                TriggerEvent::DamageDoneOnce {
                    target,
                    sources,
                    combat: is_combat,
                },
            ) => {
                if combat.is_some_and(|c| c != *is_combat) {
                    return None;
                }
                if let Some(v) = valid_target {
                    let ok = match target {
                        Target::Player(p) => self.player_matches_src(*p, v, controller, source),
                        Target::Card(c) => self.opt_card(&Some(v.clone()), *c, source, controller),
                    };
                    if !ok {
                        return None;
                    }
                }
                let total: u32 = sources
                    .iter()
                    .filter(|(s, _)| self.opt_card(valid_source, *s, source, controller))
                    .map(|(_, n)| *n)
                    .sum();
                if valid_source.is_some() && total == 0 {
                    return None;
                }
                if let Some(cmp) = amount {
                    let ectx = crate::expr::EvalCtx {
                        source,
                        controller,
                        x: 0,
                        targets: &[],
                        trig: None,
                        paid: &[],
                    };
                    if !cmp.holds(total as i32, self.eval(&cmp.rhs, &ectx)) {
                        return None;
                    }
                }
                let first = sources.first().map(|(s, _)| *s);
                Some(TriggerInfo {
                    target: Some(*target),
                    player: match target {
                        Target::Player(p) => Some(*p),
                        Target::Card(c) => Some(self.card(*c).controller),
                    },
                    source: first,
                    attacking_player: first.map(|s| self.card(s).controller),
                    amount: Some(total as i32),
                    ..Default::default()
                })
            }
            (
                TriggerKind::Blocks {
                    valid_card,
                    valid_blocked,
                },
                TriggerEvent::Blocks { blocker, attackers },
            ) => {
                if !self.opt_card(valid_card, *blocker, source, controller) {
                    return None;
                }
                if let Some(v) = valid_blocked
                    && !attackers
                        .iter()
                        .any(|a| self.card_matches(*a, v, source, controller))
                {
                    return None;
                }
                Some(TriggerInfo {
                    card: Some(*blocker),
                    card_lki: Some(*blocker),
                    ..Default::default()
                })
            }
            (
                TriggerKind::AttackerBlocked {
                    valid_card,
                    valid_blockers,
                },
                TriggerEvent::AttackerBlocked {
                    attacker,
                    blockers,
                    defender,
                },
            ) => {
                if !self.opt_card(valid_card, *attacker, source, controller) {
                    return None;
                }
                if let Some((v, cmp)) = valid_blockers {
                    let n = blockers
                        .iter()
                        .filter(|b| self.card_matches(**b, v, source, controller))
                        .count() as i32;
                    let ectx = crate::expr::EvalCtx {
                        source,
                        controller,
                        x: 0,
                        targets: &[],
                        trig: None,
                        paid: &[],
                    };
                    if !cmp.holds(n, self.eval(&cmp.rhs, &ectx)) {
                        return None;
                    }
                }
                Some(TriggerInfo {
                    card: Some(*attacker),
                    card_lki: Some(*attacker),
                    player: Some(*defender),
                    attacking_player: Some(self.card(*attacker).controller),
                    ..Default::default()
                })
            }
            (
                TriggerKind::AttackerUnblocked {
                    valid_card,
                    valid_defender,
                },
                TriggerEvent::AttackerUnblocked {
                    attacker,
                    defender,
                    target,
                },
            ) => {
                if !self.opt_card(valid_card, *attacker, source, controller) {
                    return None;
                }
                if let Some(v) = valid_defender {
                    let ok = match target {
                        Target::Player(p) => self.player_matches_src(*p, v, controller, source),
                        Target::Card(c) => self.card_matches(*c, v, source, controller),
                    };
                    if !ok {
                        return None;
                    }
                }
                Some(TriggerInfo {
                    card: Some(*attacker),
                    card_lki: Some(*attacker),
                    player: Some(*defender),
                    target: Some(*target),
                    attacking_player: Some(self.card(*attacker).controller),
                    ..Default::default()
                })
            }
            (
                TriggerKind::Drawn {
                    valid_card,
                    valid_player,
                    number: want,
                },
                TriggerEvent::Drawn {
                    card,
                    player,
                    number,
                },
            ) => {
                if want.is_some_and(|w| w != *number)
                    || !self.opt_card(valid_card, *card, source, controller)
                    || valid_player
                        .as_ref()
                        .is_some_and(|v| !self.player_matches_src(*player, v, controller, source))
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
                TriggerKind::CounterAdded {
                    valid_card,
                    counter_type,
                    amount,
                },
                TriggerEvent::CounterAdded { card, kind, total },
            ) => {
                if counter_type.as_ref().is_some_and(|t| t != kind)
                    || !self.opt_card(valid_card, *card, source, controller)
                {
                    return None;
                }
                if let Some(cmp) = amount {
                    let ectx = crate::expr::EvalCtx {
                        source,
                        controller,
                        x: 0,
                        targets: &[],
                        trig: None,
                        paid: &[],
                    };
                    if !cmp.holds(*total as i32, self.eval(&cmp.rhs, &ectx)) {
                        return None;
                    }
                }
                Some(TriggerInfo {
                    card: Some(*card),
                    card_lki: Some(*card),
                    player: Some(self.card(*card).controller),
                    ..Default::default()
                })
            }
            (
                TriggerKind::LifeGained { valid_player },
                TriggerEvent::LifeGained { player, amount },
            ) => {
                if valid_player
                    .as_ref()
                    .is_some_and(|v| !self.player_matches_src(*player, v, controller, source))
                {
                    return None;
                }
                Some(TriggerInfo {
                    player: Some(*player),
                    amount: Some(*amount as i32),
                    ..Default::default()
                })
            }
            (
                TriggerKind::CounterAddedOnce {
                    valid_card,
                    counter_type,
                    valid_source,
                },
                TriggerEvent::CounterAddedOnce {
                    card,
                    kind,
                    amount,
                    player,
                },
            ) => {
                if counter_type.as_ref().is_some_and(|t| t != kind)
                    || !self.opt_card(valid_card, *card, source, controller)
                    || valid_source
                        .as_ref()
                        .is_some_and(|v| !self.player_matches_src(*player, v, controller, source))
                {
                    return None;
                }
                Some(TriggerInfo {
                    card: Some(*card),
                    card_lki: Some(*card),
                    player: Some(*player),
                    amount: Some(*amount as i32),
                    ..Default::default()
                })
            }
            (
                TriggerKind::AbilityCast {
                    valid_card,
                    valid_activator,
                    loyalty_only,
                },
                TriggerEvent::AbilityCast {
                    source: sa_source,
                    player,
                    loyalty,
                    stack_item,
                },
            ) => {
                if (*loyalty_only && !*loyalty)
                    || !self.opt_card(valid_card, *sa_source, source, controller)
                    || valid_activator
                        .as_ref()
                        .is_some_and(|v| !self.player_matches_src(*player, v, controller, source))
                {
                    return None;
                }
                Some(TriggerInfo {
                    card: Some(*sa_source),
                    card_lki: Some(*sa_source),
                    player: Some(*player),
                    stack_item: Some(*stack_item),
                    ..Default::default()
                })
            }
            (TriggerKind::TurnFaceUp { valid_card }, TriggerEvent::TurnFaceUp { card }) => {
                if !self.opt_card(valid_card, *card, source, controller) {
                    return None;
                }
                Some(TriggerInfo {
                    card: Some(*card),
                    card_lki: Some(*card),
                    player: Some(self.card(*card).controller),
                    ..Default::default()
                })
            }
            (
                TriggerKind::Taps {
                    valid_card,
                    valid_player,
                },
                TriggerEvent::Taps { card },
            ) => {
                let ctrl = self.card(*card).controller;
                if !self.opt_card(valid_card, *card, source, controller)
                    || valid_player
                        .as_ref()
                        .is_some_and(|v| !self.player_matches_src(ctrl, v, controller, source))
                {
                    return None;
                }
                Some(TriggerInfo {
                    card: Some(*card),
                    card_lki: Some(*card),
                    player: Some(ctrl),
                    ..Default::default()
                })
            }
            (
                TriggerKind::Sacrificed {
                    valid_card,
                    valid_player,
                },
                TriggerEvent::Sacrificed { old, player },
            ) => {
                if let Some(v) = valid_card {
                    // Eigenschaften des geopferten Objekts (LKI); `Self` auch,
                    // wenn die Quelle sich selbst opfert.
                    let ok = self.card_matches(*old, v, source, controller)
                        || (source == *old && self.card_matches(*old, v, *old, controller));
                    if !ok {
                        return None;
                    }
                }
                if valid_player
                    .as_ref()
                    .is_some_and(|v| !self.player_matches_src(*player, v, controller, source))
                {
                    return None;
                }
                Some(TriggerInfo {
                    card: Some(*old),
                    card_lki: Some(*old),
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
                        Target::Player(p) => self.player_matches_src(*p, v, controller, source),
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
                    amount: Some(*amount as i32),
                    ..Default::default()
                })
            }
            _ => None,
        }
    }

    /// Optionaler Kartenfilter; `Self` passt auch, wenn die Quelle selbst betroffen ist.
    fn opt_card(
        &self,
        v: &Option<ValidSpec>,
        card: CardId,
        source: CardId,
        controller: PlayerId,
    ) -> bool {
        v.as_ref().is_none_or(|v| {
            self.card_matches(card, v, source, controller)
                || (card == source && self.card_matches(card, v, card, controller))
        })
    }

    /// Meldet ein Ereignis; passende Trigger werden vorgemerkt.
    /// Wie [`GameState::fire`], aber nur für die Trigger von `only` (z. B. ETB eines
    /// Objekts, das beim Ankommen zur Kopie wurde).
    pub fn fire_only_for(&mut self, ev: TriggerEvent, only: CardId) {
        self.fire_only = Some(only);
        self.fire(ev);
        self.fire_only = None;
    }

    pub fn fire(&mut self, ev: TriggerEvent) {
        // Die Prüfung verändert keine Eigenschaften → `chars()` cachen.
        let _cache = crate::state::CharsCache::enter();
        // Kandidaten: Objekte in Zonen + Rückblick bei Verlassen des Battlefields.
        let mut sources: Vec<(CardId, Zone)> = self
            .live_cards()
            .map(|id| self.card(id))
            .filter_map(|c| c.zone.map(|z| (c, z)))
            .filter(|(c, z)| {
                c.compiled()
                    .triggers
                    .iter()
                    .any(|t| t.active_zones.contains(z))
            })
            .map(|(c, z)| (c.id, z))
            .filter(|(id, _)| self.fire_only.is_none_or(|o| o == *id))
            .collect();
        let left_battlefield = match &ev {
            TriggerEvent::ChangesZone {
                old,
                from: Some(Zone::Battlefield),
                ..
            }
            | TriggerEvent::Sacrificed { old, .. } => Some(vec![*old]),
            TriggerEvent::ChangesZoneAll { moves } => {
                let v: Vec<CardId> = moves
                    .iter()
                    .filter(|m| m.2 == Some(Zone::Battlefield))
                    .map(|m| m.0)
                    .collect();
                (!v.is_empty()).then_some(v)
            }
            _ => None,
        };
        // „Wenn du ~ umwandelst/abwirfst“: Rückblick auf das Objekt in der Hand.
        if let TriggerEvent::Cycled { card: old, .. } | TriggerEvent::Discarded { old, .. } = &ev
            && self.card(*old).zone.is_none()
            && self
                .card(*old)
                .compiled()
                .triggers
                .iter()
                .any(|t| t.active_zones.contains(&Zone::Hand))
        {
            sources.push((*old, Zone::Hand));
        }
        if let Some(olds) = left_battlefield {
            let mut lki: Vec<CardId> = self.lki_battlefield.clone().unwrap_or_default();
            lki.extend(olds);
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
                if let Some(info) = self
                    .trigger_matches(t, src, controller, &ev)
                    .filter(|info| {
                        let ectx = crate::expr::EvalCtx {
                            source: src,
                            controller,
                            x: 0,
                            targets: &[],
                            trig: Some(info),
                            paid: &[],
                        };
                        self.check_conditions(&t.conditions, &ectx)
                    })
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
        // CR 603.7c: verzögerte Trigger lösen nur einmal aus.
        let mut i = if self.fire_only.is_some() {
            self.delayed_triggers.len()
        } else {
            0
        };
        while i < self.delayed_triggers.len() {
            let d = &self.delayed_triggers[i];
            let active = self.turn >= d.from_turn && d.until_turn.is_none_or(|u| self.turn <= u);
            let hit = active
                .then(|| self.trigger_matches(&d.trigger, d.source, d.controller, &ev))
                .flatten()
                .filter(|info| {
                    let ectx = crate::expr::EvalCtx {
                        source: d.source,
                        controller: d.controller,
                        x: 0,
                        targets: &[],
                        trig: Some(info),
                        paid: &[],
                    };
                    self.check_conditions(&d.trigger.conditions, &ectx)
                });
            if let Some(mut info) = hit {
                let d = self.delayed_triggers.remove(i);
                info.delay_remembered = d.remembered;
                found.push(PendingTrigger {
                    trigger: d.trigger,
                    source: d.source,
                    controller: d.controller,
                    info,
                });
            } else {
                i += 1;
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
                        decider: t.trigger.decider,
                        description: t.trigger.description.clone(),
                        conditions: if t.trigger.recheck_on_resolve {
                            t.trigger.conditions.clone()
                        } else {
                            Vec::new()
                        },
                        chapter: t.trigger.chapter,
                    },
                    source: t.source,
                    controller: p,
                    ability: Some(ab),
                    targets,
                    x: 0,
                    paid: Vec::new(),
                });
                self.fire_becomes_target();
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
