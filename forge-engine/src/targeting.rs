//! Auswertung von Filtern und Zielwahl (CR 115, CR 608.2b).

use crate::decision::Target;
use crate::effects::TargetSpec;
use crate::ids::{CardId, PlayerId};
use crate::keyword::Keyword;
use crate::state::{Characteristics, GameState};
use crate::valid::{Alternative, CardBase, PlayerBase, Prop, ValidSpec};
use crate::zone::Zone;

thread_local! {
    /// Angenommenes X für Filtervariablen während Wirkbarkeitsprüfungen.
    static ASSUMED_X: std::cell::Cell<Option<u32>> = const { std::cell::Cell::new(None) };
}

impl GameState {
    pub(crate) fn card_base_matches(
        &self,
        id: CardId,
        ch: &Characteristics,
        base: &CardBase,
    ) -> bool {
        match base {
            CardBase::Card => true,
            CardBase::Emblem => self.card(id).effect.is_some() && ch.name.starts_with("Emblem"),
            CardBase::Permanent => {
                self.card(id).zone == Some(Zone::Battlefield) && ch.types.is_permanent()
            }
            CardBase::Core(c) => ch.types.has_core(*c),
            CardBase::Word(w) => {
                ch.types.has_subtype(w)
                    || forge_types::Supertype::from_name(w).is_some_and(|s| ch.types.has_super(s))
                    // CR 702.73a: Changeling hat jeden Kreaturtyp
                    || (ch.has(&Keyword::Changeling)
                        && crate::valid::type_registry().is_none_or(|r| {
                            r.kind_of(w) == Some(forge_types::SubtypeKind::Creature)
                        }))
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
            Prop::InZone(z) => c.zone == Some(*z),
            Prop::ChosenColor => self
                .card(source)
                .chosen_color
                .is_some_and(|col| ch.colors.contains(col)),
            Prop::ChosenType => self.card(source).chosen_type.as_ref().is_some_and(|t| {
                forge_types::CoreType::from_name(t).map_or_else(
                    || self.card_base_matches(id, ch, &CardBase::Word(t.clone())),
                    |core| ch.types.has_core(core),
                )
            }),
            Prop::TopLibrary => {
                c.zone == Some(Zone::Library) && self.player(c.owner).library.last() == Some(&id)
            }
            Prop::ChosenCard { strict } => self
                .card(source)
                .chosen_cards
                .iter()
                .any(|c| *c == id || (!strict && self.current_object(*c) == id)),
            Prop::RememberedPlayerCtrl => {
                self.card(source).remembered_players.contains(&c.controller)
            }
            Prop::AttachedBySource => self.card(source).attached_to == Some(id),
            Prop::Monstrous => c.monstrous,
            Prop::IsCommander => c.commander.is_some(),
            Prop::ChosenCtrl => self.card(source).chosen_player == Some(c.controller),
            Prop::PlayerChosen => false,
            Prop::ExiledWithSource => c.zone == Some(Zone::Exile) && c.exiled_with == Some(source),
            Prop::Kicked(which) => match which {
                None => c.kicked != 0,
                Some(n) => c.kicked & (1 << (n - 1)) != 0,
            },
            Prop::WasCast { from } => {
                c.cast_from.is_some() && from.is_none_or(|z| c.cast_from == Some(z))
            }
            Prop::EnchantedController => false,
            Prop::Enchanted | Prop::Equipped => {
                let sub = if matches!(prop, Prop::Enchanted) {
                    "Aura"
                } else {
                    "Equipment"
                };
                self.battlefield.iter().any(|&a| {
                    self.card(a).attached_to == Some(id) && self.chars(a).types.has_subtype(sub)
                })
            }
            Prop::HasCounters => c.counters.values().any(|n| *n > 0),
            Prop::Counters { kind, op, value } => crate::expr::Compare {
                op: *op,
                rhs: crate::expr::Expr::Const(*value),
            }
            .holds(c.counter(kind) as i32, *value),
            Prop::StatVar { stat, op, var } => {
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
                let rhs = self.filter_var(source, var, you);
                v.is_some_and(|v| {
                    crate::expr::Compare {
                        op: *op,
                        rhs: crate::expr::Expr::Const(rhs),
                    }
                    .holds(v, rhs)
                })
            }
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

    /// Wert der SVar `var` der Quelle für Filtervergleiche (`cmcLEX`); X ist das
    /// zuletzt für die Quelle gewählte X. Unbekannt → 0.
    pub fn filter_var(&self, source: CardId, var: &str, you: PlayerId) -> i32 {
        type Key = (usize, usize, String);
        thread_local! {
            static CACHE: std::cell::RefCell<
                std::collections::HashMap<Key, Option<std::sync::Arc<crate::expr::Expr>>>,
            > = Default::default();
        }
        let card = self.card(source);
        let key = (
            std::sync::Arc::as_ptr(&card.def) as usize,
            card.face,
            var.to_string(),
        );
        let expr = CACHE.with(|c| {
            c.borrow_mut()
                .entry(key)
                .or_insert_with(|| {
                    let face = card.face();
                    face.svar(var)
                        .and_then(|sv| crate::expr::Expr::parse_svar(face, sv).ok())
                        .map(std::sync::Arc::new)
                })
                .clone()
        });
        let Some(e) = expr else { return 0 };
        let x = ASSUMED_X.with(|a| a.get()).unwrap_or(card.last_x);
        let _g = crate::expr::DepthGuard::enter();
        self.eval(
            &e,
            &crate::expr::EvalCtx {
                source,
                controller: you,
                x,
                targets: &[],
                trig: None,
                paid: &[],
            },
        )
    }

    /// Prüft `f` mit angenommenem X (Wirkbarkeit vor der Wahl von X, CR 601.2b).
    pub(crate) fn with_assumed_x<T>(&self, x: u32, f: impl FnOnce() -> T) -> T {
        let prev = ASSUMED_X.with(|a| a.replace(Some(x)));
        let r = f();
        ASSUMED_X.with(|a| a.set(prev));
        r
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
        self.player_matches_impl(p, spec, you, None)
    }

    /// Wie [`GameState::player_matches`], mit Quelle für quellenbezogene
    /// Eigenschaften (`Player.EnchantedController`).
    pub fn player_matches_src(
        &self,
        p: PlayerId,
        spec: &ValidSpec,
        you: PlayerId,
        source: CardId,
    ) -> bool {
        self.player_matches_impl(p, spec, you, Some(source))
    }

    fn player_matches_impl(
        &self,
        p: PlayerId,
        spec: &ValidSpec,
        you: PlayerId,
        source: Option<CardId>,
    ) -> bool {
        spec.alternatives.iter().any(|a| match a {
            Alternative::Player { base, props } => {
                let base_ok = match base {
                    PlayerBase::Player => true,
                    PlayerBase::Opponent => p != you,
                    PlayerBase::You => p == you,
                };
                base_ok
                    && props.iter().all(|pr| match pr {
                        Prop::PlayerOpponent => p != you,
                        Prop::PlayerChosen => {
                            source.is_some_and(|s| self.card(s).chosen_player == Some(p))
                        }
                        Prop::EnchantedController => source
                            .and_then(|s| self.card(s).attached_to)
                            .is_some_and(|a| {
                                self.card(a).zone.is_some() && self.card(a).controller == p
                            }),
                        _ => false,
                    })
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
                self.player(p).is_in_game()
                    && self.player_matches_src(p, &spec.valid, you, source)
                    && !self.player_has_shroud(p)
                    && !(p != you && self.player_has_hexproof(p))
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
