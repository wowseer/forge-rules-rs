//! Zahlenausdrücke (`Count$…`, `Number$…`, SVars) und Bedingungen
//! (`IsPresent$`, `CheckSVar$`, `Condition$`, `ConditionPresent$` …).
//!
//! Java-Vorbild: `AbilityUtils.calculateAmount` / `xCount` / `doXMath`,
//! `forge.util.Expressions.compare`, `CardTraitBase.meetsCommonRequirements`,
//! `StaticAbility.checkConditions`, `SpellAbilityCondition`.
//!
//! Alles wird beim Laden kompiliert. Unbekannte Zähler/Bedingungen sind
//! Ladefehler (Karte „nicht unterstützt“).

use crate::effects::CompileError;
use crate::ids::{CardId, PlayerId};
use crate::state::GameState;
use crate::valid::ValidSpec;
use crate::zone::Zone;
use forge_script::{CardFace, ParamMap};
use std::cell::Cell;

/// Rechenoperation hinter `/` (Java `doXMath`, genau eine Operation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathOp {
    Plus,
    Minus,
    NMinus,
    Times,
    Twice,
    Thrice,
    HalfUp,
    HalfDown,
    Negative,
    LimitMax,
    LimitMin,
    DivideEvenlyDown,
    DivideEvenlyUp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Const(i32),
    /// Für X bezahlter Wert.
    XPaid,
    /// Anzahl Karten in Zonen (alle Spieler), die den Filter erfüllen (`Count$Valid…`).
    CountValid {
        zones: Vec<Zone>,
        spec: ValidSpec,
    },
    /// Stärke/Widerstandskraft/Marken der Quelle.
    CardPower,
    CardToughness,
    CardCounters(String),
    YourLifeTotal,
    YourStartingLife,
    CardsInYourHand,
    /// Anzahl Standardlandtypen unter deinen Ländern.
    Domain,
    /// `<Objekte>$<Eigenschaft>` (Java `calculateAmount` → `handlePaid` /
    /// `playerXProperty`): Summe der Eigenschaft über die Objekte.
    ObjProp {
        objs: ObjRef,
        prop: ObjPropKind,
    },
    Math(Box<Expr>, MathOp, Box<Expr>),
}

/// Objekte eines `<Objekte>$…`-Ausdrucks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjRef {
    /// `Targeted`, `ParentTargeted` (Ziele der Fähigkeit bzw. geerbt)
    Targeted,
    /// `Remembered` (wie gemerkt)
    Remembered,
    /// `TriggeredCard`
    TriggeredCard,
    /// `TargetedPlayer`
    TargetedPlayer,
    /// `TriggeredPlayer`
    TriggeredPlayer,
}

impl ObjRef {
    fn parse(s: &str) -> Option<ObjRef> {
        Some(match s {
            "Targeted" | "ParentTargeted" => ObjRef::Targeted,
            "Remembered" => ObjRef::Remembered,
            "TriggeredCard" => ObjRef::TriggeredCard,
            "TargetedPlayer" => ObjRef::TargetedPlayer,
            "TriggeredPlayer" => ObjRef::TriggeredPlayer,
            _ => return None,
        })
    }

    fn is_player(self) -> bool {
        matches!(self, ObjRef::TargetedPlayer | ObjRef::TriggeredPlayer)
    }
}

/// Eigenschaft eines `<Objekte>$…`-Ausdrucks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjPropKind {
    /// Anzahl der Objekte
    Amount,
    Power,
    Toughness,
    /// `CardManaCost` (Manabetrag)
    ManaValue,
    /// Spieler: Karten auf der Hand
    CardsInHand,
    /// Spieler: Lebenspunkte
    LifeTotal,
}

/// Vergleich wie `GE3`, `EQ0`, `LTX`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compare {
    pub op: CmpOp,
    pub rhs: Expr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Lt,
    Le,
    Eq,
    Ge,
    Gt,
    Ne,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerScope {
    Any,
    You,
    Opponent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    /// Anzahl passender Karten in Zonen vergleichen (`IsPresent$`).
    Present {
        spec: ValidSpec,
        zones: Vec<Zone>,
        player: PlayerScope,
        cmp: Compare,
    },
    /// Wert vergleichen (`CheckSVar$` + `SVarCompare$`).
    Value {
        expr: Expr,
        cmp: Compare,
    },
    /// Ist es der Zug des Controllers (true) bzw. nicht (false)?
    YourTurn(bool),
    Threshold(bool),
    Delirium(bool),
    Metalcraft(bool),
    Hellbent(bool),
    FatefulHour(bool),
    /// Gemerkte Objekte der Quelle filtern und zählen
    /// (`ConditionDefined$ Remembered` + `ConditionPresent$`).
    RememberedPresent {
        spec: ValidSpec,
        cmp: Compare,
        /// `RememberedLKI`: gemerkte Objekte wie gemerkt statt aktuell.
        lki: bool,
    },
}

/// Kontext einer Auswertung.
#[derive(Debug, Clone, Copy)]
pub struct EvalCtx<'a> {
    pub source: CardId,
    pub controller: PlayerId,
    pub x: u32,
    /// Ziele der auflösenden Fähigkeit (für `Targeted$…`-Ausdrücke).
    pub targets: &'a [crate::decision::Target],
    /// Trigger-Kontext (für `TriggeredCard$…`-Ausdrücke).
    pub trig: Option<&'a crate::triggers::TriggerInfo>,
}

// ---------------------------------------------------------------------------
// Parsen
// ---------------------------------------------------------------------------

fn bad(key: &str, v: &str) -> CompileError {
    CompileError::UnsupportedParam {
        api: "Ausdruck".into(),
        key: key.into(),
        value: v.into(),
    }
}

fn math_op(name: &str) -> Option<MathOp> {
    // Reihenfolge wie Java (`contains`-Prüfungen): NMinus vor Minus.
    Some(match name {
        n if n.contains("Plus") => MathOp::Plus,
        n if n.contains("NMinus") => MathOp::NMinus,
        n if n.contains("Minus") => MathOp::Minus,
        n if n.contains("Twice") => MathOp::Twice,
        n if n.contains("Thrice") => MathOp::Thrice,
        n if n.contains("HalfUp") => MathOp::HalfUp,
        n if n.contains("HalfDown") => MathOp::HalfDown,
        n if n.contains("Negative") => MathOp::Negative,
        n if n.contains("DivideEvenlyUp") => MathOp::DivideEvenlyUp,
        n if n.contains("DivideEvenlyDown") => MathOp::DivideEvenlyDown,
        n if n.contains("Times") => MathOp::Times,
        n if n.contains("LimitMax") => MathOp::LimitMax,
        n if n.contains("LimitMin") => MathOp::LimitMin,
        _ => return None,
    })
}

impl Expr {
    /// Wert eines Parameters: Zahl oder Name einer SVar.
    pub fn parse_value(face: &CardFace, v: &str) -> Result<Expr, CompileError> {
        let t = v.trim().trim_start_matches('+');
        if let Ok(n) = t.parse::<i32>() {
            return Ok(Expr::Const(n));
        }
        if let Some(name) = t.strip_prefix('-') {
            let inner = Expr::parse_value(face, name)?;
            return Ok(Expr::Math(
                Box::new(inner),
                MathOp::Negative,
                Box::new(Expr::Const(0)),
            ));
        }
        match face.svar(t) {
            Some(sv) => Expr::parse_svar(face, sv),
            None => Err(bad("Wert", v)),
        }
    }

    /// Inhalt einer SVar: `Number$N`, `Count$…`.
    pub fn parse_svar(face: &CardFace, sv: &str) -> Result<Expr, CompileError> {
        let sv = sv.trim();
        if let Some(n) = sv.strip_prefix("Number$") {
            return n
                .trim()
                .parse()
                .map(Expr::Const)
                .map_err(|_| bad("Number", sv));
        }
        let (base, op) = if let Some(rest) = sv.strip_prefix("Count$") {
            let (main, op) = match rest.split_once('/') {
                Some((m, o)) => (m, Some(o)),
                None => (rest, None),
            };
            (Expr::parse_count(main).ok_or_else(|| bad("Count", sv))?, op)
        } else if let Some((head, rest)) = sv.split_once('$')
            && let Some(objs) = ObjRef::parse(head)
        {
            let (main, op) = match rest.split_once('/') {
                Some((m, o)) => (m, Some(o)),
                None => (rest, None),
            };
            let prop = match (objs.is_player(), main) {
                (_, "Amount") => ObjPropKind::Amount,
                (false, "CardPower") => ObjPropKind::Power,
                (false, "CardToughness") => ObjPropKind::Toughness,
                (false, "CardManaCost") => ObjPropKind::ManaValue,
                (true, "CardsInHand") => ObjPropKind::CardsInHand,
                (true, "LifeTotal") => ObjPropKind::LifeTotal,
                _ => return Err(bad("SVar", sv)),
            };
            (Expr::ObjProp { objs, prop }, op)
        } else {
            return Err(bad("SVar", sv));
        };
        let Some(op) = op else { return Ok(base) };
        let (name, arg) = match op.split_once('.') {
            Some((n, a)) => (n, Some(a)),
            None => (op, None),
        };
        let mop = math_op(name).ok_or_else(|| bad("Count-Operation", sv))?;
        let arg = match arg {
            Some(a) => Expr::parse_value(face, a)?,
            None => Expr::Const(0),
        };
        Ok(Expr::Math(Box::new(base), mop, Box::new(arg)))
    }

    fn parse_count(main: &str) -> Option<Expr> {
        let main = main.trim();
        if let Some(rest) = main.strip_prefix("Valid") {
            let (zone_part, spec) = rest.split_once(' ')?;
            if zone_part.contains("All") || zone_part.ends_with("Self") {
                return None;
            }
            let zones = if zone_part.is_empty() {
                vec![Zone::Battlefield]
            } else {
                zone_part
                    .split(',')
                    .map(Zone::parse)
                    .collect::<Option<Vec<_>>>()?
            };
            return Some(Expr::CountValid {
                zones,
                spec: ValidSpec::parse(spec).ok()?,
            });
        }
        if let Some(kind) = main.strip_prefix("CardCounters.") {
            return Some(Expr::CardCounters(kind.to_string()));
        }
        Some(match main {
            "xPaid" => Expr::XPaid,
            "CardPower" => Expr::CardPower,
            "CardToughness" => Expr::CardToughness,
            "YourLifeTotal" => Expr::YourLifeTotal,
            "YourStartingLife" => Expr::YourStartingLife,
            "CardsInYourHand" | "InYourHand" => Expr::CardsInYourHand,
            "Domain" => Expr::Domain,
            _ => return None,
        })
    }

    pub fn is_const(&self) -> bool {
        matches!(self, Expr::Const(_))
    }
}

impl Compare {
    /// `GE3`, `EQ0`, `LTX` …
    pub fn parse(face: &CardFace, s: &str) -> Result<Compare, CompileError> {
        let s = s.trim();
        if s.len() < 3 {
            return Err(bad("Vergleich", s));
        }
        let op = match &s[..2] {
            "LT" => CmpOp::Lt,
            "LE" => CmpOp::Le,
            "EQ" => CmpOp::Eq,
            "GE" => CmpOp::Ge,
            "GT" => CmpOp::Gt,
            "NE" => CmpOp::Ne,
            _ => return Err(bad("Vergleich", s)),
        };
        Ok(Compare {
            op,
            rhs: Expr::parse_value(face, &s[2..])?,
        })
    }

    pub fn holds(&self, lhs: i32, rhs: i32) -> bool {
        match self.op {
            CmpOp::Lt => lhs < rhs,
            CmpOp::Le => lhs <= rhs,
            CmpOp::Eq => lhs == rhs,
            CmpOp::Ge => lhs >= rhs,
            CmpOp::Gt => lhs > rhs,
            CmpOp::Ne => lhs != rhs,
        }
    }
}

fn zones(face_v: &str) -> Result<Vec<Zone>, CompileError> {
    face_v
        .split(',')
        .map(|z| Zone::parse(z).ok_or_else(|| bad("Zone", face_v)))
        .collect()
}

fn flag(v: &str) -> bool {
    v.eq_ignore_ascii_case("true")
}

/// Welche Schlüsselnamen gelesen werden.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CondStyle {
    /// Statics und Trigger: `IsPresent$`, `CheckSVar$`, `Condition$`, `PlayerTurn$` …
    Requirement,
    /// Effekte: `ConditionPresent$`, `ConditionCheckSVar$` …
    Effect,
}

/// Liest alle Bedingungen aus `params`; liefert die gelesenen Schlüssel mit.
pub fn parse_conditions(
    face: &CardFace,
    params: &ParamMap,
    style: CondStyle,
) -> Result<(Vec<Condition>, Vec<&'static str>), CompileError> {
    let mut conds = Vec::new();
    let mut used: Vec<&'static str> = Vec::new();
    let (k_present, k_cmp, k_zone, k_player, k_svar, k_svar_cmp) = match style {
        CondStyle::Requirement => (
            "IsPresent",
            "PresentCompare",
            "PresentZone",
            "PresentPlayer",
            "CheckSVar",
            "SVarCompare",
        ),
        CondStyle::Effect => (
            "ConditionPresent",
            "ConditionCompare",
            "ConditionZone",
            "ConditionPresentPlayer",
            "ConditionCheckSVar",
            "ConditionSVarCompare",
        ),
    };
    used.extend([k_present, k_cmp, k_zone, k_player, k_svar, k_svar_cmp]);
    let defined = match style {
        CondStyle::Effect => {
            used.push("ConditionDefined");
            params.get("ConditionDefined")
        }
        CondStyle::Requirement => None,
    };
    if let Some(d) = defined {
        // Java: ohne ConditionPresent$ wird ConditionDefined$ nicht geprüft.
        if !matches!(d, "Remembered" | "RememberedLKI") {
            return Err(bad("ConditionDefined", d));
        }
        if params.has(k_zone) || params.has(k_player) {
            return Err(bad("ConditionDefined", "mit Zone/Spieler"));
        }
        if let Some(p) = params.get(k_present) {
            conds.push(Condition::RememberedPresent {
                spec: ValidSpec::parse(p).map_err(CompileError::Valid)?,
                cmp: Compare::parse(face, params.get(k_cmp).unwrap_or("GE1"))?,
                lki: d == "RememberedLKI",
            });
        }
    } else if let Some(p) = params.get(k_present) {
        let spec = ValidSpec::parse(p).map_err(CompileError::Valid)?;
        let cmp = Compare::parse(face, params.get(k_cmp).unwrap_or("GE1"))?;
        let zs = match params.get(k_zone) {
            Some(z) => zones(z)?,
            None => vec![Zone::Battlefield],
        };
        let player = match params.get(k_player).unwrap_or("Any") {
            "Any" => PlayerScope::Any,
            "You" => PlayerScope::You,
            "Opponent" => PlayerScope::Opponent,
            other => return Err(bad(k_player, other)),
        };
        conds.push(Condition::Present {
            spec,
            zones: zs,
            player,
            cmp,
        });
    } else if params.has(k_cmp) || params.has(k_zone) {
        return Err(bad(k_cmp, "ohne IsPresent"));
    }
    if let Some(sv) = params.get(k_svar) {
        let expr = Expr::parse_value(face, sv)?;
        let cmp = Compare::parse(face, params.get(k_svar_cmp).unwrap_or("GE1"))?;
        conds.push(Condition::Value { expr, cmp });
    }
    if style == CondStyle::Requirement {
        used.extend([
            "Condition",
            "PlayerTurn",
            "OpponentTurn",
            "Threshold",
            "Delirium",
            "Metalcraft",
            "Hellbent",
            "FatefulHour",
        ]);
        if let Some(c) = params.get("Condition") {
            conds.push(match c {
                "PlayerTurn" => Condition::YourTurn(true),
                "NotPlayerTurn" => Condition::YourTurn(false),
                "Threshold" => Condition::Threshold(true),
                "Delirium" => Condition::Delirium(true),
                "Metalcraft" => Condition::Metalcraft(true),
                "Hellbent" => Condition::Hellbent(true),
                "FatefulHour" => Condition::FatefulHour(true),
                other => return Err(bad("Condition", other)),
            });
        }
        if params.get("PlayerTurn").is_some_and(flag) {
            conds.push(Condition::YourTurn(true));
        }
        if params.get("OpponentTurn").is_some_and(flag) {
            conds.push(Condition::YourTurn(false));
        }
        for (k, mk) in [
            ("Threshold", Condition::Threshold as fn(bool) -> Condition),
            ("Delirium", Condition::Delirium),
            ("Metalcraft", Condition::Metalcraft),
            ("Hellbent", Condition::Hellbent),
            ("FatefulHour", Condition::FatefulHour),
        ] {
            if let Some(v) = params.get(k) {
                conds.push(mk(flag(v)));
            }
        }
    }
    Ok((conds, used))
}

// ---------------------------------------------------------------------------
// Auswerten
// ---------------------------------------------------------------------------

thread_local! {
    /// Schachtelungstiefe von Auswertungen innerhalb von `chars()` – verhindert
    /// Endlosrekursion, wenn ein Ausdruck Eigenschaften anderer Objekte braucht
    /// (z. B. „+X/+X für jede Kreatur“). Siehe `GameState::chars`.
    pub(crate) static EVAL_DEPTH: Cell<u32> = const { Cell::new(0) };
}

pub(crate) struct DepthGuard;

impl DepthGuard {
    pub(crate) fn enter() -> DepthGuard {
        EVAL_DEPTH.with(|d| d.set(d.get() + 1));
        DepthGuard
    }
    pub(crate) fn active() -> bool {
        EVAL_DEPTH.with(|d| d.get() > 0)
    }
}

impl Drop for DepthGuard {
    fn drop(&mut self) {
        EVAL_DEPTH.with(|d| d.set(d.get() - 1));
    }
}

impl GameState {
    fn cards_in_zones(&self, zones: &[Zone], players: &[PlayerId]) -> Vec<CardId> {
        let mut v = Vec::new();
        for z in zones {
            match z {
                Zone::Battlefield => v.extend(
                    self.battlefield
                        .iter()
                        .copied()
                        .filter(|c| players.contains(&self.card(*c).controller)),
                ),
                Zone::Exile => v.extend(
                    self.exile
                        .iter()
                        .copied()
                        .filter(|c| players.contains(&self.card(*c).owner)),
                ),
                Zone::Command => v.extend(
                    self.command
                        .iter()
                        .copied()
                        .filter(|c| players.contains(&self.card(*c).owner)),
                ),
                Zone::Stack => v.extend(self.stack.iter().filter_map(|s| match s.kind {
                    crate::state::StackKind::Spell { card } if players.contains(&s.controller) => {
                        Some(card)
                    }
                    _ => None,
                })),
                pz => {
                    for p in players {
                        v.extend(self.player(*p).zone(*pz).iter().copied());
                    }
                }
            }
        }
        v
    }

    fn eval_obj_prop(&self, objs: ObjRef, prop: ObjPropKind, ctx: &EvalCtx) -> i32 {
        use crate::decision::Target;
        if objs.is_player() {
            let players: Vec<PlayerId> = match objs {
                ObjRef::TargetedPlayer => ctx
                    .targets
                    .iter()
                    .filter_map(|t| match t {
                        Target::Player(p) => Some(*p),
                        Target::Card(_) => None,
                    })
                    .collect(),
                _ => ctx.trig.and_then(|t| t.player).into_iter().collect(),
            };
            return players
                .iter()
                .map(|p| match prop {
                    ObjPropKind::CardsInHand => self.player(*p).hand.len() as i32,
                    ObjPropKind::LifeTotal => self.player(*p).life,
                    _ => 1,
                })
                .sum();
        }
        let cards: Vec<CardId> = match objs {
            ObjRef::Targeted => ctx
                .targets
                .iter()
                .filter_map(|t| match t {
                    Target::Card(c) => Some(*c),
                    Target::Player(_) => None,
                })
                .collect(),
            ObjRef::Remembered => self.card(ctx.source).remembered.clone(),
            // Nach Verlassen des Battlefields zählt das alte Objekt (LKI).
            _ => ctx
                .trig
                .and_then(|t| match t.card {
                    Some(c) if self.card(c).zone == Some(Zone::Battlefield) => Some(c),
                    c => t.card_lki.or(c),
                })
                .into_iter()
                .collect(),
        };
        cards
            .iter()
            .map(|c| match prop {
                ObjPropKind::Power => self.chars(*c).power.unwrap_or(0),
                ObjPropKind::Toughness => self.chars(*c).toughness.unwrap_or(0),
                ObjPropKind::ManaValue => self
                    .chars(*c)
                    .mana_cost
                    .as_ref()
                    .map_or(0, |m| m.mana_value() as i32),
                _ => 1,
            })
            .sum()
    }

    pub fn eval(&self, e: &Expr, ctx: &EvalCtx) -> i32 {
        match e {
            Expr::Const(n) => *n,
            Expr::XPaid => ctx.x as i32,
            Expr::CountValid { zones, spec } => {
                let all = self.players_from(self.active);
                self.cards_in_zones(zones, &all)
                    .into_iter()
                    .filter(|c| self.card_matches(*c, spec, ctx.source, ctx.controller))
                    .count() as i32
            }
            Expr::CardPower => self.chars(ctx.source).power.unwrap_or(0),
            Expr::CardToughness => self.chars(ctx.source).toughness.unwrap_or(0),
            Expr::CardCounters(k) => self.card(ctx.source).counter(k) as i32,
            Expr::YourLifeTotal => self.player(ctx.controller).life,
            Expr::YourStartingLife => self.rules.starting_life,
            Expr::CardsInYourHand => self.player(ctx.controller).hand.len() as i32,
            Expr::ObjProp { objs, prop } => self.eval_obj_prop(*objs, *prop, ctx),
            Expr::Domain => {
                let mut n = 0;
                for t in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
                    if self.battlefield_of(ctx.controller).any(|c| {
                        let ch = self.chars(c);
                        ch.types.is_land() && ch.types.has_subtype(t)
                    }) {
                        n += 1;
                    }
                }
                n
            }
            Expr::Math(a, op, b) => {
                let x = self.eval(a, ctx);
                let y = self.eval(b, ctx);
                match op {
                    MathOp::Plus => x + y,
                    MathOp::Minus => x - y,
                    MathOp::NMinus => y - x,
                    MathOp::Times => x * y,
                    MathOp::Twice => x * 2,
                    MathOp::Thrice => x * 3,
                    MathOp::HalfUp => (x as f64 / 2.0).ceil() as i32,
                    MathOp::HalfDown => (x as f64 / 2.0).floor() as i32,
                    MathOp::Negative => -x,
                    MathOp::LimitMax => x.min(y),
                    MathOp::LimitMin => x.max(y),
                    MathOp::DivideEvenlyDown => {
                        if y == 0 {
                            0
                        } else {
                            x / y
                        }
                    }
                    MathOp::DivideEvenlyUp => {
                        if y == 0 {
                            0
                        } else {
                            x / y + i32::from(x % y != 0)
                        }
                    }
                }
            }
        }
    }

    fn card_types_in_graveyard(&self, p: PlayerId) -> usize {
        let mut types = std::collections::BTreeSet::new();
        for c in &self.player(p).graveyard {
            for t in &self.chars(*c).types.core_types {
                types.insert(*t);
            }
        }
        types.len()
    }

    pub fn check_condition(&self, c: &Condition, ctx: &EvalCtx) -> bool {
        let you = ctx.controller;
        match c {
            Condition::Present {
                spec,
                zones,
                player,
                cmp,
            } => {
                let players: Vec<PlayerId> = match player {
                    PlayerScope::Any => self.players_from(self.active),
                    PlayerScope::You => vec![you],
                    PlayerScope::Opponent => self.opponents(you),
                };
                let n = self
                    .cards_in_zones(zones, &players)
                    .into_iter()
                    .filter(|x| self.card_matches(*x, spec, ctx.source, you))
                    .count() as i32;
                cmp.holds(n, self.eval(&cmp.rhs, ctx))
            }
            Condition::Value { expr, cmp } => {
                cmp.holds(self.eval(expr, ctx), self.eval(&cmp.rhs, ctx))
            }
            Condition::YourTurn(b) => (self.active == you) == *b,
            Condition::Threshold(b) => (self.player(you).graveyard.len() >= 7) == *b,
            Condition::Delirium(b) => (self.card_types_in_graveyard(you) >= 4) == *b,
            Condition::Metalcraft(b) => {
                (self
                    .battlefield_of(you)
                    .filter(|x| {
                        self.chars(*x)
                            .types
                            .has_core(forge_types::CoreType::Artifact)
                    })
                    .count()
                    >= 3)
                    == *b
            }
            Condition::Hellbent(b) => self.player(you).hand.is_empty() == *b,
            Condition::FatefulHour(b) => (self.player(you).life <= 5) == *b,
            Condition::RememberedPresent { spec, cmp, lki } => {
                let objs = if *lki {
                    self.card(ctx.source).remembered.clone()
                } else {
                    self.remembered_current(ctx.source)
                };
                let n = objs
                    .iter()
                    .filter(|x| self.card_matches(**x, spec, ctx.source, you))
                    .count() as i32;
                cmp.holds(n, self.eval(&cmp.rhs, ctx))
            }
        }
    }

    pub fn check_conditions(&self, cs: &[Condition], ctx: &EvalCtx) -> bool {
        cs.iter().all(|c| self.check_condition(c, ctx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_script::CardScript;
    use forge_types::TypeRegistry;

    fn face(svars: &str) -> forge_script::CardScript {
        CardScript::parse(
            &format!("Name:A\nManaCost:1\nTypes:Enchantment\n{svars}"),
            &TypeRegistry::default(),
        )
        .unwrap()
    }

    #[test]
    fn parse_counts() {
        let c = face(
            "SVar:X:Count$Valid Creature.YouCtrl/Plus.1\nSVar:Y:Count$ValidGraveyard Card.YouOwn\nSVar:Z:Count$xPaid/Twice\n",
        );
        let f = c.main();
        assert!(matches!(
            Expr::parse_value(f, "X").unwrap(),
            Expr::Math(_, MathOp::Plus, _)
        ));
        assert!(matches!(
            Expr::parse_value(f, "Y").unwrap(),
            Expr::CountValid { .. }
        ));
        assert!(matches!(
            Expr::parse_value(f, "-Z").unwrap(),
            Expr::Math(_, MathOp::Negative, _)
        ));
        assert!(Expr::parse_value(f, "Q").is_err());
        assert!(Compare::parse(f, "GEX").is_ok());
        assert!(Compare::parse(f, "XX3").is_err());
    }
}
