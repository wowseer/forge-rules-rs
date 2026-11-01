//! Auswertung von Filtern und Zielwahl (CR 115, CR 608.2b).

use crate::decision::Target;
use crate::effects::TargetSpec;
use crate::ids::{CardId, PlayerId};
use crate::keyword::Keyword;
use crate::state::{Characteristics, GameState};
use crate::valid::{Alternative, CardBase, PlayerBase, Prop, ValidSpec};
use crate::zone::Zone;

impl GameState {
    fn card_base_matches(&self, id: CardId, ch: &Characteristics, base: &CardBase) -> bool {
        match base {
            CardBase::Card => true,
            CardBase::Permanent => {
                self.card(id).zone == Some(Zone::Battlefield) && ch.types.is_permanent()
            }
            CardBase::Core(c) => ch.types.has_core(*c),
            CardBase::Word(w) => {
                ch.types.has_subtype(w)
                    || forge_types::Supertype::from_name(w).is_some_and(|s| ch.types.has_super(s))
            }
        }
    }

    fn prop_matches(
        &self,
        id: CardId,
        ch: &Characteristics,
        prop: &Prop,
        source: CardId,
        you: PlayerId,
    ) -> bool {
        let c = self.card(id);
        let combat = self.combat.as_ref();
        match prop {
            Prop::YouCtrl => c.controller == you,
            Prop::YouDontCtrl => c.controller != you,
            Prop::OppCtrl => c.controller != you,
            Prop::YouOwn => c.owner == you,
            Prop::Other => id != source,
            Prop::SelfCard => id == source,
            Prop::Tapped => c.tapped,
            Prop::Untapped => !c.tapped,
            Prop::Attacking => combat.is_some_and(|cb| cb.is_attacking(id)),
            Prop::Blocking => combat.is_some_and(|cb| cb.is_blocking(id)),
            Prop::Token => c.token,
            Prop::NonToken => !c.token,
            Prop::Is(b) => self.card_base_matches(id, ch, b),
            Prop::Non(b) => !self.card_base_matches(id, ch, b),
            Prop::Color(col) => ch.colors.contains(*col),
            Prop::NonColor(col) => !ch.colors.contains(*col),
            Prop::Multicolor => ch.colors.is_multicolor(),
            Prop::Colorless => ch.colors.is_colorless(),
            Prop::NonColorless => !ch.colors.is_colorless(),
            Prop::With(k) => ch.has(k),
            Prop::Without(k) => !ch.has(k),
            Prop::PlayerOpponent => false,
            Prop::Not(inner) => !self.prop_matches(id, ch, inner, source, you),
            Prop::OppOwn => c.owner != you,
            Prop::IsRemembered => self.card(source).remembered.contains(&id),
            Prop::AttachedBySource => self.card(source).attached_to == Some(id),
            Prop::Stat { stat, op, value } => {
                let v = match stat {
                    crate::valid::Stat::Power => ch.power,
                    crate::valid::Stat::Toughness => ch.toughness,
                    crate::valid::Stat::ManaValue => Some(
                        ch.mana_cost
                            .as_ref()
                            .map(|m| m.mana_value() as i32)
                            .unwrap_or(0),
                    ),
                };
                v.is_some_and(|v| {
                    crate::expr::Compare {
                        op: *op,
                        rhs: crate::expr::Expr::Const(*value),
                    }
                    .holds(v, *value)
                })
            }
        }
    }

    /// Erfüllt das Objekt den Filter (aus Sicht von `you`, Quelle `source`)?
    pub fn card_matches(
        &self,
        id: CardId,
        spec: &ValidSpec,
        source: CardId,
        you: PlayerId,
    ) -> bool {
        let ch = self.chars(id);
        self.card_matches_with(id, &ch, spec, source, you)
    }

    /// Wie [`GameState::card_matches`], aber gegen gegebene (Zwischen-)Eigenschaften –
    /// nötig im Layer-System, um Rekursion zu vermeiden (CR 613.6).
    pub fn card_matches_with(
        &self,
        id: CardId,
        ch: &Characteristics,
        spec: &ValidSpec,
        source: CardId,
        you: PlayerId,
    ) -> bool {
        spec.alternatives.iter().any(|a| match a {
            Alternative::Card { base, props } => {
                self.card_base_matches(id, ch, base)
                    && props
                        .iter()
                        .all(|p| self.prop_matches(id, ch, p, source, you))
            }
            Alternative::Player { .. } => false,
        })
    }

    pub fn player_matches(&self, p: PlayerId, spec: &ValidSpec, you: PlayerId) -> bool {
        spec.alternatives.iter().any(|a| match a {
            Alternative::Player { base, props } => {
                let base_ok = match base {
                    PlayerBase::Player => true,
                    PlayerBase::Opponent => p != you,
                    PlayerBase::You => p == you,
                };
                base_ok
                    && props
                        .iter()
                        .all(|pr| matches!(pr, Prop::PlayerOpponent) && p != you)
            }
            Alternative::Card { .. } => false,
        })
    }

    /// Kann `target` von einer Fähigkeit von `source` (kontrolliert von `you`)
    /// anvisiert werden? Prüft Filter, Zone (Standard: Battlefield) sowie
    /// Fluchsicherheit/Verhüllung (CR 702.11, 702.18).
    pub fn is_legal_target(
        &self,
        spec: &TargetSpec,
        target: Target,
        source: CardId,
        you: PlayerId,
    ) -> bool {
        match target {
            Target::Player(p) => {
                self.player(p).is_in_game() && self.player_matches(p, &spec.valid, you)
            }
            Target::Card(c) => {
                let card = self.card(c);
                let Some(zone) = card.zone else { return false };
                if !spec.zones.contains(&zone) {
                    return false;
                }
                // Fluchsicherheit/Verhüllung wirken nur auf bleibende Karten (CR 702.11a, 702.18a).
                if zone == Zone::Battlefield {
                    if self.has_keyword(c, &Keyword::Shroud) {
                        return false;
                    }
                    if self.has_keyword(c, &Keyword::Hexproof) && card.controller != you {
                        return false;
                    }
                }
                self.card_matches(c, &spec.valid, source, you)
            }
        }
    }

    pub fn target_candidates(
        &self,
        spec: &TargetSpec,
        source: CardId,
        you: PlayerId,
    ) -> Vec<Target> {
        let _cache = crate::state::CharsCache::enter();
        let mut v: Vec<Target> = Vec::new();
        if spec.valid.allows_players() {
            v.extend(
                self.players_from(self.active)
                    .into_iter()
                    .map(Target::Player),
            );
        }
        if spec.valid.allows_cards() {
            for z in &spec.zones {
                match z {
                    Zone::Battlefield => {
                        v.extend(self.battlefield.iter().map(|c| Target::Card(*c)))
                    }
                    Zone::Exile => v.extend(self.exile.iter().map(|c| Target::Card(*c))),
                    Zone::Command => v.extend(self.command.iter().map(|c| Target::Card(*c))),
                    Zone::Stack => v.extend(self.stack.iter().filter_map(|s| match s.kind {
                        crate::state::StackKind::Spell { card } => Some(Target::Card(card)),
                        _ => None,
                    })),
                    pz => {
                        for p in self.players_from(self.active) {
                            v.extend(self.player(p).zone(*pz).iter().map(|c| Target::Card(*c)));
                        }
                    }
                }
            }
        }
        v.retain(|t| self.is_legal_target(spec, *t, source, you));
        v
    }
}
