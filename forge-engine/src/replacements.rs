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
    if parts.len() > 3 && parts[3] != "no Condition" {
        return Err(err());
    }
    let amount = Expr::parse_value(face, parts[2]).map_err(|_| err())?;
    Ok((parts[1].to_string(), amount))
}

/// Was beim Zonenwechsel geändert wird.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MoveMods {
    pub tapped: bool,
    pub counters: Vec<(String, u32)>,
}

impl GameState {
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
