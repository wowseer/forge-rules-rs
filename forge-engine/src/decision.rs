//! Spielerentscheidungen.
//!
//! Die Engine ruft **keine** Callbacks auf (Java: `PlayerController` mit 113
//! Methoden). Stattdessen hält sie an und liefert eine [`Decision`]; der
//! Aufrufer (GUI, KI, Test, Replay) antwortet mit einer [`Choice`].
//! [`Decision::validate`] prüft jede Antwort, ungültige werden abgelehnt.

use crate::ids::{CardId, PlayerId};
use forge_types::Color;
use std::fmt;

/// Ziel einer Fähigkeit (CR 115).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Target {
    Card(CardId),
    Player(PlayerId),
}

/// Aktion, die ein Spieler mit Priorität ausführen kann (CR 117.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Pass,
    PlayLand(CardId),
    Cast(CardId),
    /// Andere Seite wirken (Adventure, modale doppelseitige Karte; CR 715.3, 712.11).
    CastAlt(CardId, usize),
    /// Andere Seite als Land spielen (modale doppelseitige Karte, CR 712.12).
    PlayLandAlt(CardId, usize),
    /// Aktivierte Fähigkeit: Karte + Index in `CompiledFace::activated`.
    Activate(CardId, usize),
    /// Zauber mit Alternativkosten wirken: Karte + Index in `CompiledFace::statics`
    /// (`AlternativeCost`, CR 118.9).
    CastWithAltCost(CardId, usize),
}

/// Option beim Bezahlen von Manakosten.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PayAction {
    /// Mana-Fähigkeit aktivieren (Karte, Fähigkeitsindex).
    ActivateMana(CardId, usize),
    /// Engine wählt Quellen automatisch.
    AutoPay,
    /// Abbrechen – das Wirken wird vollständig zurückgenommen (CR 728).
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// CR 601.2b / 700.2: Modi eines modalen Zaubers wählen. `modes` = (Index, Text)
    /// der wählbaren Modi (Modi ohne legale Ziele fehlen).
    ChooseModes {
        player: PlayerId,
        modes: Vec<(usize, String)>,
        min: u32,
        max: u32,
        can_repeat: bool,
    },
    /// London-Mulligan (CR 103.5): behalten oder neu ziehen.
    Mulligan {
        player: PlayerId,
        mulligans_taken: u32,
    },
    /// Nach dem Behalten `count` Karten unter die Bibliothek legen.
    MulliganBottom {
        player: PlayerId,
        hand: Vec<CardId>,
        count: u32,
    },
    Priority {
        player: PlayerId,
        actions: Vec<Action>,
    },
    ChooseTargets {
        player: PlayerId,
        source: CardId,
        description: String,
        candidates: Vec<Target>,
        min: u32,
        max: u32,
    },
    ChooseX {
        player: PlayerId,
        max: u32,
    },
    PayMana {
        player: PlayerId,
        remaining: String,
        options: Vec<PayAction>,
    },
    /// Phyrexianisches Symbol mit 2 Leben bezahlen?
    PayLifeForPhyrexian {
        player: PlayerId,
        symbol: String,
    },
    ChooseColor {
        player: PlayerId,
        options: Vec<Color>,
    },
    /// Typ wählen (`ChooseType`); Antwort `Choice::Number(index)`.
    ChooseType {
        player: PlayerId,
        options: Vec<String>,
    },
    DeclareAttackers {
        player: PlayerId,
        candidates: Vec<CardId>,
        /// Angreifbare Ziele: Gegner und ihre Planeswalker (CR 508.1b).
        defenders: Vec<Target>,
    },
    DeclareBlockers {
        player: PlayerId,
        candidates: Vec<CardId>,
        attackers: Vec<CardId>,
    },
    /// Kampfschaden eines Angreifers verteilen (CR 510.1c–d). `lethal[i]` ist
    /// der tödliche Schaden für `blockers[i]` (für Trample relevant).
    AssignCombatDamage {
        player: PlayerId,
        attacker: CardId,
        amount: u32,
        blockers: Vec<CardId>,
        lethal: Vec<u32>,
        /// Angegriffener Spieler bzw. Planeswalker (CR 702.19c).
        trample_to: Option<Target>,
    },
    /// Karten wählen (Abwerfen auf Handgröße, Opfern als Kosten, …).
    ChooseCards {
        player: PlayerId,
        reason: CardChoiceReason,
        options: Vec<CardId>,
        min: u32,
        max: u32,
    },
    /// CR 603.3b: eigene gleichzeitig ausgelöste Trigger ordnen.
    OrderTriggers {
        player: PlayerId,
        descriptions: Vec<String>,
    },
    /// CR 701.22: Scry – Karten oben/unten anordnen. Bei Surveil (CR 701.25,
    /// `graveyard = true`) gehen die „unteren“ Karten in den Friedhof.
    Scry {
        player: PlayerId,
        cards: Vec<CardId>,
        graveyard: bool,
    },
    /// „… es sei denn, [Spieler] zahlt [Kosten]“ (CR 118.12). Wird nur
    /// gestellt, wenn die Kosten bezahlbar sind.
    PayUnlessCost {
        player: PlayerId,
        cost: String,
    },
    /// „You may“-Trigger beim Auflösen (OptionalDecider$ You).
    OptionalTrigger {
        player: PlayerId,
        description: String,
    },
    /// Allgemeine Ja/Nein-Frage eines Effekts (z. B. „du darfst aufdecken“).
    Confirm {
        player: PlayerId,
        prompt: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardChoiceReason {
    DiscardToHandSize,
    SacrificeCost,
    DiscardCost,
    /// CR 704.5j: welche legendäre bleibt
    LegendRule,
    /// Dig: Karten aus den obersten wählen
    Dig,
    /// CR 701.19: Bibliothek/Hand durchsuchen
    Search,
    /// CR 701.9: Abwerfen durch einen Effekt
    Discard,
    /// CR 701.21: Opfern durch einen Effekt (auch `Destroy$`-Variante)
    Sacrifice,
    /// CR 701.28: optionales Transformieren
    Transform,
    /// Kosten: Karten exilieren (`ExileFromGrave<n/Typ>`)
    ExileCost,
    /// Kosten: Permanents tappen (`tapXType<n/Typ>`)
    TapCost,
    /// Kosten: Permanents auf die Hand (`Return<n/Typ>`)
    ReturnCost,
    /// `ChooseCard`: Karten für einen Effekt wählen
    Choose,
    /// CR 702.51: Kreaturen für Convoke tappen
    Convoke,
    /// CR 702.126: Artefakte für Improvise tappen
    Improvise,
    /// CR 702.66: Karten aus dem Friedhof für Delve exilieren
    Delve,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Choice {
    Keep,
    Mulligan,
    Action(Action),
    Targets(Vec<Target>),
    Number(u32),
    Pay(PayAction),
    Yes,
    No,
    Color(Color),
    /// (Angreifer, angegriffener Spieler oder Planeswalker)
    Attackers(Vec<(CardId, Target)>),
    /// (Blocker, Angreifer)
    Blocks(Vec<(CardId, CardId)>),
    /// Schaden je Blocker (gleiche Reihenfolge wie in der Entscheidung),
    /// optional gefolgt vom Schaden an den Spieler (Trample).
    Damage(Vec<u32>),
    Cards(Vec<CardId>),
    /// Permutation der Indizes (z. B. Trigger-Reihenfolge; erstes Element kommt zuerst auf den Stapel).
    Order(Vec<usize>),
    /// Gewählte Modi (Indizes aus `ChooseModes::modes`).
    Modes(Vec<usize>),
    /// Scry: `top` (von oben nach unten) und `bottom` (von oben nach unten).
    Arrange {
        top: Vec<CardId>,
        bottom: Vec<CardId>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidChoice(pub String);

impl fmt::Display for InvalidChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ungültige Wahl: {}", self.0)
    }
}

impl std::error::Error for InvalidChoice {}

fn err<T>(s: impl Into<String>) -> Result<T, InvalidChoice> {
    Err(InvalidChoice(s.into()))
}

fn distinct<T: PartialEq>(v: &[T]) -> bool {
    v.iter().enumerate().all(|(i, a)| !v[..i].contains(a))
}

impl Decision {
    pub fn player(&self) -> PlayerId {
        match self {
            Decision::Mulligan { player, .. }
            | Decision::MulliganBottom { player, .. }
            | Decision::Priority { player, .. }
            | Decision::ChooseTargets { player, .. }
            | Decision::ChooseX { player, .. }
            | Decision::PayMana { player, .. }
            | Decision::PayLifeForPhyrexian { player, .. }
            | Decision::ChooseColor { player, .. }
            | Decision::ChooseType { player, .. }
            | Decision::DeclareAttackers { player, .. }
            | Decision::DeclareBlockers { player, .. }
            | Decision::AssignCombatDamage { player, .. }
            | Decision::ChooseCards { player, .. }
            | Decision::OrderTriggers { player, .. }
            | Decision::OptionalTrigger { player, .. }
            | Decision::Confirm { player, .. }
            | Decision::ChooseModes { player, .. }
            | Decision::PayUnlessCost { player, .. }
            | Decision::Scry { player, .. } => *player,
        }
    }

    /// Prüft eine Antwort formal (Typ, Mengen, Kandidaten). Regelprüfungen,
    /// die Spielzustand brauchen (z. B. Blockbeschränkungen), macht die Engine.
    pub fn validate(&self, c: &Choice) -> Result<(), InvalidChoice> {
        match (self, c) {
            (Decision::Mulligan { .. }, Choice::Keep | Choice::Mulligan) => Ok(()),
            (Decision::MulliganBottom { hand, count, .. }, Choice::Cards(cs)) => {
                if cs.len() as u32 != *count
                    || !distinct(cs)
                    || cs.iter().any(|c| !hand.contains(c))
                {
                    return err(format!("genau {count} verschiedene Handkarten wählen"));
                }
                Ok(())
            }
            (Decision::Priority { actions, .. }, Choice::Action(a)) => {
                if actions.contains(a) {
                    Ok(())
                } else {
                    err(format!("{a:?} ist keine legale Aktion"))
                }
            }
            (
                Decision::ChooseTargets {
                    candidates,
                    min,
                    max,
                    ..
                },
                Choice::Targets(ts),
            ) => {
                let n = ts.len() as u32;
                if n < *min
                    || n > *max
                    || !distinct(ts)
                    || ts.iter().any(|t| !candidates.contains(t))
                {
                    return err(format!("{min}..={max} verschiedene legale Ziele"));
                }
                Ok(())
            }
            (Decision::ChooseX { max, .. }, Choice::Number(n)) => {
                if n <= max {
                    Ok(())
                } else {
                    err(format!("X darf höchstens {max} sein"))
                }
            }
            (Decision::PayMana { options, .. }, Choice::Pay(p)) => {
                if options.contains(p) {
                    Ok(())
                } else {
                    err(format!("{p:?} nicht verfügbar"))
                }
            }
            (Decision::PayLifeForPhyrexian { .. }, Choice::Yes | Choice::No) => Ok(()),
            (Decision::ChooseType { options, .. }, Choice::Number(i)) => {
                if (*i as usize) < options.len() {
                    Ok(())
                } else {
                    err("Typ nicht wählbar")
                }
            }
            (Decision::ChooseColor { options, .. }, Choice::Color(col)) => {
                if options.contains(col) {
                    Ok(())
                } else {
                    err("Farbe nicht wählbar")
                }
            }
            (
                Decision::DeclareAttackers {
                    candidates,
                    defenders,
                    ..
                },
                Choice::Attackers(at),
            ) => {
                let cards: Vec<CardId> = at.iter().map(|a| a.0).collect();
                if !distinct(&cards)
                    || at
                        .iter()
                        .any(|(c, d)| !candidates.contains(c) || !defenders.contains(d))
                {
                    return err("ungültige Angreifer");
                }
                Ok(())
            }
            (
                Decision::DeclareBlockers {
                    candidates,
                    attackers,
                    ..
                },
                Choice::Blocks(bs),
            ) => {
                let blockers: Vec<CardId> = bs.iter().map(|b| b.0).collect();
                if !distinct(&blockers)
                    || bs
                        .iter()
                        .any(|(b, a)| !candidates.contains(b) || !attackers.contains(a))
                {
                    return err("ungültige Blocker");
                }
                Ok(())
            }
            (
                Decision::AssignCombatDamage {
                    amount,
                    blockers,
                    lethal,
                    trample_to,
                    ..
                },
                Choice::Damage(d),
            ) => {
                let expected = blockers.len() + usize::from(trample_to.is_some());
                if d.len() != expected || d.iter().sum::<u32>() != *amount {
                    return err(format!(
                        "genau {amount} Schaden auf {expected} Empfänger verteilen"
                    ));
                }
                if trample_to.is_some()
                    && d[blockers.len()] > 0
                    && d.iter().zip(lethal).any(|(x, l)| x < l)
                {
                    return err("Trample: erst allen Blockern tödlichen Schaden zuweisen");
                }
                Ok(())
            }
            (
                Decision::ChooseCards {
                    options, min, max, ..
                },
                Choice::Cards(cs),
            ) => {
                let n = cs.len() as u32;
                if n < *min || n > *max || !distinct(cs) || cs.iter().any(|c| !options.contains(c))
                {
                    return err(format!("{min}..={max} Karten aus den Optionen"));
                }
                Ok(())
            }
            (Decision::OrderTriggers { descriptions, .. }, Choice::Order(o)) => {
                let mut sorted = o.clone();
                sorted.sort_unstable();
                if sorted != (0..descriptions.len()).collect::<Vec<_>>() {
                    return err("Reihenfolge muss jede Position genau einmal enthalten");
                }
                Ok(())
            }
            (
                Decision::OptionalTrigger { .. }
                | Decision::PayUnlessCost { .. }
                | Decision::Confirm { .. },
                Choice::Yes | Choice::No,
            ) => Ok(()),
            (Decision::Scry { cards, .. }, Choice::Arrange { top, bottom }) => {
                let mut all: Vec<CardId> = top.iter().chain(bottom).copied().collect();
                all.sort();
                let mut expected = cards.clone();
                expected.sort();
                if all != expected {
                    return err("jede Karte genau einmal oben oder unten");
                }
                Ok(())
            }
            (
                Decision::ChooseModes {
                    modes,
                    min,
                    max,
                    can_repeat,
                    ..
                },
                Choice::Modes(m),
            ) => {
                let n = m.len() as u32;
                if n < *min || n > *max || m.iter().any(|i| !modes.iter().any(|(x, _)| x == i)) {
                    return err(format!("{min}..={max} wählbare Modi"));
                }
                if !can_repeat && !distinct(m) {
                    return err("Modi dürfen sich nicht wiederholen");
                }
                Ok(())
            }
            (d, c) => err(format!("{c:?} passt nicht zu {d:?}")),
        }
    }
}
