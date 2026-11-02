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
    /// Menge des auslösenden Ereignisses (`TriggerCount$DamageAmount`,
    /// `TriggerCount$LifeAmount`); ohne Trigger-Kontext 0.
    TriggerAmount,
    /// `Count$LifeYouGainedThisTurn` / `LifeYouLostThisTurn` / `LifeOppsLostThisTurn`
    LifeGainedThisTurn,
    LifeLostThisTurn,
    OppsLifeLostThisTurn,
    /// `Count$YouDrewThisTurn`
    DrewThisTurn,
    /// `Count$AttackersDeclared`: verschiedene Kreaturen, die du in diesem Zug
    /// angreifen ließt.
    AttackersDeclared,
    /// `Count$ThisTurnCast_<Valid>`
    ThisTurnCast(ValidSpec),
    /// `Count$ThisTurnEntered_<Ziel>[_from_<Herkunft>]_<Valid>` (Filter auf das
    /// Objekt vor dem Zonenwechsel)
    ThisTurnEntered {
        to: Zone,
        from: Option<Zone>,
        spec: ValidSpec,
    },
    /// `Count$Morbid.<ja>.<nein>`: starb in diesem Zug eine Kreatur?
    Morbid(Box<Expr>, Box<Expr>),
    /// `Count$Valid <Filter>$<Aggregat>`: Wert über die passenden Karten.
    ValidAgg {
        zones: Vec<Zone>,
        spec: ValidSpec,
        agg: Agg,
    },
    /// `PlayerCount<Gruppe>$…` (Java `playerXCount`)
    PlayerCount {
        group: PlayerGroup,
        prop: PlayerAgg,
    },
    /// `Count$Party` (CR 700.8): Kleriker, Schurke, Krieger, Zauberer – je höchstens einer.
    Party,
    /// `Count$RememberedSize`: gemerkte Objekte (Karten und Spieler) der Quelle.
    RememberedSize,
    /// `Count$Monarch.<ja>.<nein>`: bist du der Monarch?
    YouAreMonarch(Box<Expr>, Box<Expr>),
    /// `Count$Kicked.<ja>.<nein>`: wurde die Quelle gekickt?
    Kicked(Box<Expr>, Box<Expr>),
    /// Wert nur, wenn der Vergleich gilt (sonst 0) – `etbCounter … CheckSVar$`.
    IfCmp(Box<Expr>, Box<Compare>, Box<Expr>),
    Math(Box<Expr>, MathOp, Box<Expr>),
}

/// Aggregat über Karten (`$GreatestCardPower`, `$Colors` …).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Agg {
    SumPower,
    SumToughness,
    SumManaValue,
    GreatestPower,
    GreatestToughness,
    GreatestManaValue,
    /// Anzahl verschiedener Farben insgesamt
    Colors,
    DifferentPower,
    DifferentManaValue,
    DifferentNames,
    /// Anzahl verschiedener Kartentypen insgesamt
    CardTypes,
}

impl Agg {
    fn parse(s: &str) -> Option<Agg> {
        Some(match s {
            "CardPower" => Agg::SumPower,
            "CardToughness" => Agg::SumToughness,
            "CardManaCost" => Agg::SumManaValue,
            "GreatestCardPower" => Agg::GreatestPower,
            "GreatestCardToughness" => Agg::GreatestToughness,
            "GreatestCardManaCost" => Agg::GreatestManaValue,
            "Colors" => Agg::Colors,
            "DifferentCardPower" => Agg::DifferentPower,
            "DifferentCardManaCost" => Agg::DifferentManaValue,
            "DifferentCardNames" => Agg::DifferentNames,
            "CardTypes" => Agg::CardTypes,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerGroup {
    /// alle Spieler im Spiel
    Players,
    Opponents,
}

/// Was über die Spieler einer Gruppe berechnet wird.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerAgg {
    Amount,
    /// `HasPropertyisMonarch`: Anzahl Spieler, die Monarch sind
    MonarchCount,
    Sum(PlayerQty),
    Highest(PlayerQty),
    Lowest(PlayerQty),
}

/// Spielerbezogene Größe (Java `playerXProperty`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerQty {
    /// Passende Karten in Zonen aus Sicht des Spielers (`YouCtrl` = er).
    Valid {
        zones: Vec<Zone>,
        spec: ValidSpec,
    },
    LifeTotal,
    CardsInHand,
    CardsInGraveyard,
}

impl PlayerQty {
    fn parse(s: &str) -> Option<PlayerQty> {
        Some(match s {
            "LifeTotal" => PlayerQty::LifeTotal,
            "CardsInHand" => PlayerQty::CardsInHand,
            "CardsInGraveyard" => PlayerQty::CardsInGraveyard,
            _ => {
                let rest = s.strip_prefix("Valid")?;
                let (zone_part, spec) = rest.split_once(' ')?;
                let zones = if zone_part.is_empty() {
                    vec![Zone::Battlefield]
                } else {
                    zone_part
                        .split(',')
                        .map(Zone::parse)
                        .collect::<Option<Vec<_>>>()?
                };
                PlayerQty::Valid {
                    zones,
                    spec: ValidSpec::parse(spec).ok()?,
                }
            }
        })
    }
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
    /// `TriggeredCards` (Sammel-Trigger)
    TriggeredCards,
    /// `TargetedPlayer`
    TargetedPlayer,
    /// `TriggeredPlayer`
    TriggeredPlayer,
    /// `Sacrificed`, `Discarded`, `Exiled`: als Kosten bezahlte Objekte
    Paid(crate::state::PaidKind),
}

impl ObjRef {
    fn parse(s: &str) -> Option<ObjRef> {
        Some(match s {
            "Targeted" | "ParentTargeted" => ObjRef::Targeted,
            "Remembered" => ObjRef::Remembered,
            "TriggeredCard" => ObjRef::TriggeredCard,
            "TriggeredCards" => ObjRef::TriggeredCards,
            "TargetedPlayer" => ObjRef::TargetedPlayer,
            "TriggeredPlayer" => ObjRef::TriggeredPlayer,
            "Sacrificed" => ObjRef::Paid(crate::state::PaidKind::Sacrificed),
            "Discarded" => ObjRef::Paid(crate::state::PaidKind::Discarded),
            "Exiled" => ObjRef::Paid(crate::state::PaidKind::Exiled),
            "Tapped" => ObjRef::Paid(crate::state::PaidKind::Tapped),
            "Returned" => ObjRef::Paid(crate::state::PaidKind::Returned),
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
    /// `Condition$ Kicked`: die Quelle wurde gekickt.
    Kicked,
    /// Du bist der Monarch (`Condition$ Monarch`, `CheckDefinedPlayer$ You.isMonarch`).
    Monarch,
    /// Gemerkte Objekte der Quelle filtern und zählen
    /// (`ConditionDefined$ Remembered` + `ConditionPresent$`).
    RememberedPresent {
        spec: ValidSpec,
        cmp: Compare,
        /// `RememberedLKI`: gemerkte Objekte wie gemerkt statt aktuell.
        lki: bool,
        /// `ConditionDefined$ Targeted`: die Ziele der Fähigkeit statt der gemerkten.
        targeted: bool,
        /// `ConditionDefined$ ChosenCard`: die gewählten Karten der Quelle.
        chosen: bool,
        /// `ConditionDefined$ Self`: die Quelle selbst.
        self_card: bool,
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
    /// Als Kosten bezahlte Objekte (für `Sacrificed$…`-Ausdrücke).
    pub paid: &'a [crate::state::Paid],
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
        let (base, op) = if let Some(rest) = sv.strip_prefix("TriggerCount$") {
            let (main, op) = match rest.split_once('/') {
                Some((m, o)) => (m, Some(o)),
                None => (rest, None),
            };
            match main {
                "DamageAmount" | "LifeAmount" => (Expr::TriggerAmount, op),
                _ => return Err(bad("TriggerCount", sv)),
            }
        } else if let Some(rest) = sv.strip_prefix("Count$") {
            let (main, op) = match rest.split_once('/') {
                Some((m, o)) => (m, Some(o)),
                None => (rest, None),
            };
            if let Some(args) = main.strip_prefix("Morbid.") {
                let (a, b) = args.split_once('.').ok_or_else(|| bad("Count", sv))?;
                let base = Expr::Morbid(
                    Box::new(Expr::parse_value(face, a)?),
                    Box::new(Expr::parse_value(face, b)?),
                );
                (base, op)
            } else if let Some(args) = main.strip_prefix("Monarch.") {
                let (a, b) = args.split_once('.').ok_or_else(|| bad("Count", sv))?;
                let base = Expr::YouAreMonarch(
                    Box::new(Expr::parse_value(face, a)?),
                    Box::new(Expr::parse_value(face, b)?),
                );
                (base, op)
            } else if let Some(args) = main.strip_prefix("Kicked.") {
                let (a, b) = args.split_once('.').ok_or_else(|| bad("Count", sv))?;
                let base = Expr::Kicked(
                    Box::new(Expr::parse_value(face, a)?),
                    Box::new(Expr::parse_value(face, b)?),
                );
                (base, op)
            } else {
                (Expr::parse_count(main).ok_or_else(|| bad("Count", sv))?, op)
            }
        } else if let Some((head, rest)) = sv.split_once('$')
            && let Some(group) = head.strip_prefix("PlayerCount")
        {
            let group = match group {
                "Players" | "" => PlayerGroup::Players,
                "Opponents" | "Other" => PlayerGroup::Opponents,
                _ => return Err(bad("SVar", sv)),
            };
            let (main, op) = match rest.split_once('/') {
                Some((m, o)) => (m, Some(o)),
                None => (rest, None),
            };
            let prop = if main == "Amount" {
                PlayerAgg::Amount
            } else if main == "HasPropertyisMonarch" {
                PlayerAgg::MonarchCount
            } else if let Some(q) = main.strip_prefix("Highest") {
                PlayerAgg::Highest(PlayerQty::parse(q).ok_or_else(|| bad("SVar", sv))?)
            } else if let Some(q) = main.strip_prefix("Lowest") {
                PlayerAgg::Lowest(PlayerQty::parse(q).ok_or_else(|| bad("SVar", sv))?)
            } else {
                PlayerAgg::Sum(PlayerQty::parse(main).ok_or_else(|| bad("SVar", sv))?)
            };
            (Expr::PlayerCount { group, prop }, op)
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
            if let Some((spec, agg)) = spec.split_once('$') {
                return Some(Expr::ValidAgg {
                    zones,
                    spec: ValidSpec::parse(spec).ok()?,
                    agg: Agg::parse(agg)?,
                });
            }
            return Some(Expr::CountValid {
                zones,
                spec: ValidSpec::parse(spec).ok()?,
            });
        }
        if let Some(v) = main.strip_prefix("ThisTurnCast_") {
            return Some(Expr::ThisTurnCast(ValidSpec::parse(v).ok()?));
        }
        if let Some(rest) = main.strip_prefix("ThisTurnEntered_") {
            let parts: Vec<&str> = rest.splitn(4, '_').collect();
            let (to, from, v) = match parts.as_slice() {
                [to, "from", from, v] => (*to, Some(Zone::parse(from)?), *v),
                [to, v] => (*to, None, *v),
                _ => return None,
            };
            return Some(Expr::ThisTurnEntered {
                to: Zone::parse(to)?,
                from,
                spec: ValidSpec::parse(v).ok()?,
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
            "LifeYouGainedThisTurn" => Expr::LifeGainedThisTurn,
            "LifeYouLostThisTurn" => Expr::LifeLostThisTurn,
            "LifeOppsLostThisTurn" => Expr::OppsLifeLostThisTurn,
            "YouDrewThisTurn" => Expr::DrewThisTurn,
            "AttackersDeclared" => Expr::AttackersDeclared,
            "Party" => Expr::Party,
            "RememberedSize" => Expr::RememberedSize,
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
        if !matches!(
            d,
            "Remembered" | "RememberedLKI" | "Targeted" | "ChosenCard" | "Self"
        ) {
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
                targeted: d == "Targeted",
                chosen: d == "ChosenCard",
                self_card: d == "Self",
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
    used.push("CheckDefinedPlayer");
    if let Some(v) = params.get("CheckDefinedPlayer") {
        conds.push(match v.trim() {
            "You.isMonarch" => Condition::Monarch,
            other => return Err(bad("CheckDefinedPlayer", other)),
        });
    }
    // `Condition$` gilt für Statics/Trigger und für Effekte (Java SpellAbilityCondition).
    used.push("Condition");
    if let Some(c) = params.get("Condition") {
        conds.push(match c {
            "PlayerTurn" => Condition::YourTurn(true),
            "NotPlayerTurn" => Condition::YourTurn(false),
            "Threshold" => Condition::Threshold(true),
            "Delirium" => Condition::Delirium(true),
            "Metalcraft" => Condition::Metalcraft(true),
            "Hellbent" => Condition::Hellbent(true),
            "FatefulHour" => Condition::FatefulHour(true),
            "Kicked" => Condition::Kicked,
            "Monarch" => Condition::Monarch,
            other => return Err(bad("Condition", other)),
        });
    }
    if style == CondStyle::Requirement {
        used.extend([
            "PlayerTurn",
            "OpponentTurn",
            "Threshold",
            "Delirium",
            "Metalcraft",
            "Hellbent",
            "FatefulHour",
        ]);
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
    static MORBID_SPEC: ValidSpec = ValidSpec::parse("Creature").expect("Filter Creature");
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
    /// Aggregat über Karten (`Count$Valid …$<Agg>`, Java `CardLists`-Hilfen).
    pub(crate) fn aggregate(&self, cards: &[CardId], agg: Agg) -> i32 {
        let chs: Vec<_> = cards.iter().map(|c| self.chars(*c)).collect();
        let mv = |c: &crate::state::Characteristics| {
            c.mana_cost.as_ref().map_or(0, |m| m.mana_value() as i32)
        };
        let distinct = |mut v: Vec<i64>| {
            v.sort();
            v.dedup();
            v.len() as i32
        };
        match agg {
            Agg::SumPower => chs.iter().map(|c| c.power.unwrap_or(0)).sum(),
            Agg::SumToughness => chs.iter().map(|c| c.toughness.unwrap_or(0)).sum(),
            Agg::SumManaValue => chs.iter().map(mv).sum(),
            Agg::GreatestPower => chs.iter().filter_map(|c| c.power).max().unwrap_or(0),
            Agg::GreatestToughness => chs.iter().filter_map(|c| c.toughness).max().unwrap_or(0),
            Agg::GreatestManaValue => chs.iter().map(mv).max().unwrap_or(0),
            Agg::Colors => chs
                .iter()
                .fold(forge_types::ColorSet::COLORLESS, |a, c| a.union(c.colors))
                .iter()
                .count() as i32,
            Agg::DifferentPower => {
                distinct(chs.iter().filter_map(|c| c.power.map(i64::from)).collect())
            }
            Agg::DifferentManaValue => distinct(chs.iter().map(|c| i64::from(mv(c))).collect()),
            Agg::DifferentNames => {
                let mut v: Vec<&str> = chs.iter().map(|c| c.name.as_str()).collect();
                v.sort();
                v.dedup();
                v.len() as i32
            }
            Agg::CardTypes => {
                use forge_types::CoreType::*;
                [
                    Artifact,
                    Battle,
                    Creature,
                    Enchantment,
                    Instant,
                    Kindred,
                    Land,
                    Planeswalker,
                    Sorcery,
                ]
                .iter()
                .filter(|t| chs.iter().any(|c| c.types.has_core(**t)))
                .count() as i32
            }
        }
    }

    pub(crate) fn cards_in_zones(&self, zones: &[Zone], players: &[PlayerId]) -> Vec<CardId> {
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
            ObjRef::Remembered => {
                if prop == ObjPropKind::Amount {
                    let s = self.card(ctx.source);
                    return (s.remembered.len() + s.remembered_players.len()) as i32;
                }
                self.card(ctx.source).remembered.clone()
            }
            ObjRef::TriggeredCards => ctx.trig.map(|t| t.cards.clone()).unwrap_or_default(),
            ObjRef::Paid(kind) => ctx
                .paid
                .iter()
                .filter(|p| p.kind == kind)
                .map(|p| p.card)
                .collect(),
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
            Expr::ValidAgg { zones, spec, agg } => {
                let all = self.players_from(self.active);
                let cards: Vec<CardId> = self
                    .cards_in_zones(zones, &all)
                    .into_iter()
                    .filter(|c| self.card_matches(*c, spec, ctx.source, ctx.controller))
                    .collect();
                self.aggregate(&cards, *agg)
            }
            Expr::PlayerCount { group, prop } => {
                let players: Vec<PlayerId> = match group {
                    PlayerGroup::Players => self
                        .players_from(self.active)
                        .into_iter()
                        .filter(|p| self.player(*p).is_in_game())
                        .collect(),
                    PlayerGroup::Opponents => self.opponents(ctx.controller),
                };
                let qty = |p: PlayerId, q: &PlayerQty| -> i32 {
                    match q {
                        PlayerQty::LifeTotal => self.player(p).life,
                        PlayerQty::CardsInHand => self.player(p).hand.len() as i32,
                        PlayerQty::CardsInGraveyard => self.player(p).graveyard.len() as i32,
                        PlayerQty::Valid { zones, spec } => {
                            let all = self.players_from(self.active);
                            self.cards_in_zones(zones, &all)
                                .into_iter()
                                .filter(|c| self.card_matches(*c, spec, ctx.source, p))
                                .count() as i32
                        }
                    }
                };
                match prop {
                    PlayerAgg::Amount => players.len() as i32,
                    PlayerAgg::MonarchCount => {
                        players.iter().filter(|p| self.monarch == Some(**p)).count() as i32
                    }
                    PlayerAgg::Sum(q) => players.iter().map(|p| qty(*p, q)).sum(),
                    PlayerAgg::Highest(q) => {
                        players.iter().map(|p| qty(*p, q)).max().unwrap_or(0).max(0)
                    }
                    PlayerAgg::Lowest(q) => players.iter().map(|p| qty(*p, q)).min().unwrap_or(0),
                }
            }
            Expr::Party => {
                // Bipartite Zuordnung: höchstens 4 Rollen, Rücksetzsuche genügt.
                let roles = ["Cleric", "Rogue", "Warrior", "Wizard"];
                let fits: Vec<Vec<usize>> = self
                    .battlefield_of(ctx.controller)
                    .filter(|c| {
                        self.chars(*c)
                            .types
                            .has_core(forge_types::CoreType::Creature)
                    })
                    .map(|c| {
                        (0..4)
                            .filter(|i| {
                                self.card_base_matches(
                                    c,
                                    &self.chars(c),
                                    &crate::valid::CardBase::Word(roles[*i].to_string()),
                                )
                            })
                            .collect::<Vec<_>>()
                    })
                    .filter(|v| !v.is_empty())
                    .collect();
                fn best(i: usize, fits: &[Vec<usize>], used: &mut [bool; 4]) -> i32 {
                    if i == fits.len() {
                        return 0;
                    }
                    let mut b = best(i + 1, fits, used);
                    for &r in &fits[i] {
                        if !used[r] {
                            used[r] = true;
                            b = b.max(1 + best(i + 1, fits, used));
                            used[r] = false;
                        }
                        if b == 4 {
                            break;
                        }
                    }
                    b
                }
                best(0, &fits, &mut [false; 4])
            }
            Expr::YouAreMonarch(a, b) => {
                if self.monarch == Some(ctx.controller) {
                    self.eval(a, ctx)
                } else {
                    self.eval(b, ctx)
                }
            }
            Expr::Kicked(a, b) => {
                if self.card(ctx.source).kicked != 0 {
                    self.eval(a, ctx)
                } else {
                    self.eval(b, ctx)
                }
            }
            Expr::IfCmp(check, cmp, v) => {
                if cmp.holds(self.eval(check, ctx), self.eval(&cmp.rhs, ctx)) {
                    self.eval(v, ctx)
                } else {
                    0
                }
            }
            Expr::RememberedSize => {
                let s = self.card(ctx.source);
                (s.remembered.len() + s.remembered_players.len()) as i32
            }
            Expr::CardPower => self.chars(ctx.source).power.unwrap_or(0),
            Expr::CardToughness => self.chars(ctx.source).toughness.unwrap_or(0),
            Expr::CardCounters(k) => self.card(ctx.source).counter(k) as i32,
            Expr::YourLifeTotal => self.player(ctx.controller).life,
            Expr::YourStartingLife => self.rules.starting_life,
            Expr::CardsInYourHand => self.player(ctx.controller).hand.len() as i32,
            Expr::ObjProp { objs, prop } => self.eval_obj_prop(*objs, *prop, ctx),
            Expr::TriggerAmount => ctx.trig.and_then(|t| t.amount).unwrap_or(0),
            Expr::LifeGainedThisTurn => self.player(ctx.controller).life_gained_this_turn,
            Expr::LifeLostThisTurn => self.player(ctx.controller).life_lost_this_turn,
            Expr::OppsLifeLostThisTurn => self
                .opponents(ctx.controller)
                .iter()
                .map(|p| self.player(*p).life_lost_this_turn)
                .sum(),
            Expr::DrewThisTurn => self.player(ctx.controller).cards_drawn_this_turn as i32,
            Expr::AttackersDeclared => {
                let mut seen: Vec<CardId> = Vec::new();
                for (p, c) in &self.this_turn.attackers {
                    if *p == ctx.controller && !seen.contains(c) {
                        seen.push(*c);
                    }
                }
                seen.len() as i32
            }
            Expr::ThisTurnCast(spec) => self
                .this_turn
                .cast
                .iter()
                .filter(|c| self.card_matches(**c, spec, ctx.source, ctx.controller))
                .count() as i32,
            Expr::ThisTurnEntered { to, from, spec } => {
                self.entered_this_turn(*to, *from, spec, ctx.source, ctx.controller)
            }
            Expr::Morbid(yes, no) => {
                let died = self.entered_this_turn(
                    Zone::Graveyard,
                    Some(Zone::Battlefield),
                    &MORBID_SPEC.with(Clone::clone),
                    ctx.source,
                    ctx.controller,
                ) > 0;
                self.eval(if died { yes } else { no }, ctx)
            }
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

    /// Java `CardUtil.getThisTurnEntered`: Zonenwechsel dieses Zuges nach `to`
    /// (optional aus `from`), gefiltert auf das Objekt vor dem Wechsel.
    pub(crate) fn entered_this_turn(
        &self,
        to: Zone,
        from: Option<Zone>,
        spec: &ValidSpec,
        source: CardId,
        controller: PlayerId,
    ) -> i32 {
        self.this_turn
            .moves
            .iter()
            .filter(|(old, _, f, t)| {
                *t == to
                    && from.is_none_or(|z| *f == Some(z))
                    && self.card_matches(*old, spec, source, controller)
            })
            .count() as i32
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
            Condition::Kicked => self.card(ctx.source).kicked != 0,
            Condition::Monarch => self.monarch == Some(you),
            Condition::RememberedPresent {
                spec,
                cmp,
                lki,
                targeted,
                chosen,
                self_card,
            } => {
                if *targeted {
                    let n = ctx
                        .targets
                        .iter()
                        .filter(|t| match t {
                            crate::decision::Target::Card(c) => {
                                self.card_matches(*c, spec, ctx.source, you)
                            }
                            crate::decision::Target::Player(p) => {
                                self.player_matches(*p, spec, you)
                            }
                        })
                        .count() as i32;
                    return cmp.holds(n, self.eval(&cmp.rhs, ctx));
                }
                let objs = if *self_card {
                    vec![ctx.source]
                } else if *chosen {
                    self.card(ctx.source)
                        .chosen_cards
                        .iter()
                        .map(|c| self.current_object(*c))
                        .collect()
                } else if *lki {
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
