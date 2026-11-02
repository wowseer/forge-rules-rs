//! Legale Aktionen, Länder spielen, Zauber wirken, Fähigkeiten aktivieren,
//! Kosten bezahlen (CR 116, 117, 305, 601, 602, 605).
//!
//! Java-Vorbild: `HumanPlaySpellAbility`, `CostPayment`, `ComputerUtilMana`
//! (nur für die Auto-Bezahlung), `Player.canPlayLand`.

use crate::cost::{Cost, CostPart};
use crate::decision::{Action, CardChoiceReason, Choice, Decision, PayAction, Target};
use crate::effects::resolve::ResolveCtx;
use crate::effects::{CompiledAbility, ManaProduce};
use crate::ids::{CardId, PlayerId};
use crate::keyword::Keyword;
use crate::mana_pool::{ManaPool, PhyrexianPay};
use crate::state::{
    Choices, Event, GameState, Interrupt, Paid, PaidKind, Res, StackItem, StackKind,
};
use crate::zone::Zone;
use forge_types::{Color, ManaCost};
use std::sync::Arc;

/// Eine nutzbare Spielerlaubnis (`MayPlay$`) für eine Karte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MayPlayGrant {
    /// Quelle der statischen Fähigkeit (oft ein Effektobjekt)
    pub host: CardId,
    pub static_index: usize,
    pub without_mana_cost: bool,
    pub with_flash: bool,
    pub ignore_type: bool,
    pub ignore_color: bool,
}

impl GameState {
    /// Darf `p` jetzt etwas mit Hexerei-Geschwindigkeit tun (CR 307.1)?
    pub fn sorcery_timing(&self, p: PlayerId) -> bool {
        self.active == p && self.step.is_main() && self.stack.is_empty()
    }

    /// CR 302.6: Kann die Kreatur angreifen bzw. {T}/{Q}-Kosten bezahlen?
    pub fn is_summoning_sick(&self, c: CardId) -> bool {
        self.is_creature(c)
            && !self.card(c).controlled_since_turn_start
            && !self.has_keyword(c, &Keyword::Haste)
    }

    pub(crate) fn can_pay_nonmana(&self, cost: &Cost, source: CardId, p: PlayerId) -> bool {
        let src = self.card(source);
        cost.parts.iter().all(|part| match part {
            CostPart::Tap => {
                src.zone == Some(Zone::Battlefield)
                    && !src.tapped
                    && !self.is_summoning_sick(source)
            }
            CostPart::Untap => {
                src.zone == Some(Zone::Battlefield) && src.tapped && !self.is_summoning_sick(source)
            }
            CostPart::SacrificeSelf => src.zone == Some(Zone::Battlefield),
            CostPart::Sacrifice { amount, valid } => {
                self.battlefield_of(p)
                    .filter(|c| self.card_matches(*c, valid, source, p))
                    .count() as u32
                    >= *amount
            }
            CostPart::PayLife(n) => self.player(p).life >= *n as i32,
            CostPart::AddCounter { .. } => src.zone == Some(Zone::Battlefield),
            CostPart::DiscardSelf => src.zone == Some(Zone::Hand) && src.owner == p,
            CostPart::ExileSelfFromGraveyard => src.zone == Some(Zone::Graveyard) && src.owner == p,
            CostPart::ExileSelf | CostPart::ReturnSelf => src.zone == Some(Zone::Battlefield),
            CostPart::ExileSelfFromHand => src.zone == Some(Zone::Hand) && src.owner == p,
            CostPart::ExileFromHand { amount, valid } => {
                self.player(p)
                    .hand
                    .iter()
                    .filter(|c| **c != source && self.card_matches(**c, valid, source, p))
                    .count() as u32
                    >= *amount
            }
            CostPart::ExileFromGraveyard { amount, valid } => {
                self.player(p)
                    .graveyard
                    .iter()
                    .filter(|c| self.card_matches(**c, valid, source, p))
                    .count() as u32
                    >= *amount
            }
            CostPart::TapType { amount, valid } => {
                self.tap_type_options(cost, valid, source, p).len() as u32 >= *amount
            }
            CostPart::Return { amount, valid } => {
                self.battlefield_of(p)
                    .filter(|c| self.card_matches(*c, valid, source, p))
                    .count() as u32
                    >= *amount
            }
            CostPart::RemoveCounter { amount, kind } => {
                src.zone == Some(Zone::Battlefield) && src.counter(kind) >= *amount
            }
            CostPart::Discard { amount, valid } => {
                self.player(p)
                    .hand
                    .iter()
                    .filter(|c| **c != source && self.card_matches(**c, valid, source, p))
                    .count() as u32
                    >= *amount
            }
        })
    }

    /// Mana-Fähigkeiten, die `p` jetzt aktivieren kann (CR 605.3a).
    pub fn available_mana_abilities(&self, p: PlayerId) -> Vec<(CardId, usize)> {
        let mut v = Vec::new();
        for c in self.battlefield_of(p).collect::<Vec<_>>() {
            for (i, ab) in self.card(c).compiled().activated.iter().enumerate() {
                if ab.is_mana_ability()
                    && !self.cant_activate(p, c, ab)
                    && ab
                        .cost
                        .as_ref()
                        .is_none_or(|cost| cost.mana.is_none() && self.can_pay_nonmana(cost, c, p))
                {
                    v.push((c, i));
                }
            }
        }
        v
    }

    /// Grobe Obergrenze für verfügbares Mana (Pool + Mana-Fähigkeiten).
    fn potential_mana(&self, p: PlayerId) -> u32 {
        let from_abilities: u32 = self
            .available_mana_abilities(p)
            .into_iter()
            .map(|(c, i)| {
                let ab = &self.card(c).compiled().activated[i];
                match &ab.effect {
                    crate::effects::Effect::AddMana { produce, amount } => {
                        let per = match produce {
                            ManaProduce::Fixed(v) => v.len().max(1) as i32,
                            _ => 1,
                        };
                        let n = match amount {
                            crate::effects::Amount::Fixed(n) => *n,
                            crate::effects::Amount::XPaid => 0,
                            crate::effects::Amount::Expr(e) => self.eval(
                                e,
                                &crate::expr::EvalCtx {
                                    source: c,
                                    controller: p,
                                    x: 0,
                                    targets: &[],
                                    trig: None,
                                    paid: &[],
                                },
                            ),
                        };
                        (per * n).max(0) as u32
                    }
                    _ => 1,
                }
            })
            .sum();
        self.player(p).mana_pool.len() as u32 + from_abilities
    }

    /// Sind für jede Fähigkeit der Kette mit Pflichtzielen Ziele vorhanden?
    /// Bei modalen Fähigkeiten: genug Modi mit legalen Zielen (CR 700.2a)?
    pub(crate) fn has_targets_for(
        &self,
        ab: &CompiledAbility,
        source: CardId,
        p: PlayerId,
    ) -> bool {
        if let crate::effects::Effect::Charm(spec) = &ab.effect {
            let feasible = spec
                .choices
                .iter()
                .filter(|m| self.has_targets_for(m, source, p))
                .count() as u32;
            return feasible >= spec.min.max(1).min(spec.choices.len() as u32)
                || (spec.can_repeat && feasible > 0);
        }
        ab.chain().iter().all(|a| match &a.target {
            Some(spec) => spec.min == 0 || !self.target_candidates(spec, source, p).is_empty(),
            None => true,
        })
    }

    /// CR 601.2b / 603.3c: Modi wählen; liefert die zusammengesetzte Kette der
    /// gewählten Modi in Druckreihenfolge (CR 700.2). Nicht-modale Fähigkeiten
    /// werden unverändert zurückgegeben.
    pub(crate) fn choose_modes(
        &mut self,
        p: PlayerId,
        source: CardId,
        ab: &Arc<CompiledAbility>,
        ch: &mut Choices,
    ) -> Res<Arc<CompiledAbility>> {
        let crate::effects::Effect::Charm(spec) = &ab.effect else {
            return Ok(ab.clone());
        };
        let modes: Vec<(usize, String)> = spec
            .choices
            .iter()
            .enumerate()
            .filter(|(_, m)| self.has_targets_for(m, source, p))
            .map(|(i, m)| (i, m.description.clone()))
            .collect();
        let max = if spec.can_repeat {
            spec.num
        } else {
            spec.num.min(modes.len() as u32)
        };
        let min = spec.min.min(max);
        if max == 0 || (modes.len() as u32) < min.min(1) {
            return Err(Interrupt::Cancelled);
        }
        let mut chosen = match ch.ask(Decision::ChooseModes {
            player: p,
            modes,
            min,
            max,
            can_repeat: spec.can_repeat,
        })? {
            Choice::Modes(m) => m,
            _ => return Err(Interrupt::Cancelled),
        };
        chosen.sort_unstable();
        let mut iter = chosen.into_iter().map(|i| spec.choices[i].clone());
        let Some(first) = iter.next() else {
            // null Modi (MinCharmNum$ 0): nur die Unterfähigkeit des Charms selbst
            return Ok(Arc::new(CompiledAbility {
                effect: crate::effects::Effect::Charm(crate::effects::CharmSpec {
                    choices: vec![],
                    num: 0,
                    min: 0,
                    can_repeat: false,
                }),
                ..(**ab).clone()
            }));
        };
        let mut built: CompiledAbility = (*first).clone();
        for m in iter {
            built = built.append(m);
        }
        if let Some(sub) = &ab.sub {
            built = built.append(sub.clone());
        }
        Ok(Arc::new(built))
    }

    /// Spielerlaubnisse (`MayPlay$`) für `p`: (Karte, Erlaubnis) in fester Reihenfolge.
    /// Karten auf der Hand sind ohnehin spielbar und werden ausgelassen.
    pub fn may_play_grants(&self, p: PlayerId) -> Vec<(CardId, MayPlayGrant)> {
        use crate::statics::CompiledStatic;
        let mut out = Vec::new();
        let all = self.players_from(self.active);
        for host in self.live_cards() {
            let h = self.card(host);
            let Some(hz) = h.zone else { continue };
            for (si, cs) in h.compiled().statics.iter().enumerate() {
                let CompiledStatic::Continuous(st) = cs.as_ref() else {
                    continue;
                };
                let Some(mp) = &st.may_play else { continue };
                if !st.effect_zones.contains(&hz) {
                    continue;
                }
                if let Some(n) = mp.limit {
                    let used = h
                        .may_play_uses
                        .iter()
                        .filter(|(i, t)| *i == si && *t == self.turn)
                        .count() as u32;
                    if used >= n {
                        continue;
                    }
                }
                let ctx = crate::expr::EvalCtx {
                    source: host,
                    controller: h.controller,
                    x: 0,
                    targets: &[],
                    trig: None,
                    paid: &[],
                };
                if !self.check_conditions(&st.conditions, &ctx) {
                    continue;
                }
                let zones: Vec<Zone> = st
                    .affected_zones
                    .iter()
                    .copied()
                    .filter(|z| *z != Zone::Hand && *z != Zone::Battlefield && *z != Zone::Stack)
                    .collect();
                for c in self.cards_in_zones(&zones, &all) {
                    let grantee = if mp.card_owner {
                        self.card(c).owner
                    } else {
                        h.controller
                    };
                    if grantee != p
                        || (st.affects_self_only && c != host)
                        || !st
                            .affected
                            .as_ref()
                            .is_none_or(|a| self.card_matches(c, a, host, h.controller))
                    {
                        continue;
                    }
                    out.push((
                        c,
                        MayPlayGrant {
                            host,
                            static_index: si,
                            without_mana_cost: mp.without_mana_cost,
                            with_flash: mp.with_flash,
                            ignore_type: mp.ignore_type,
                            ignore_color: mp.ignore_color,
                        },
                    ));
                }
            }
        }
        out
    }

    /// Kann `p` die Karte `c` (mit ihrer aktuellen Seite) jetzt wirken?
    /// Zeitpunkt, Ziele und grob die Bezahlbarkeit.
    fn castable_now(&self, p: PlayerId, c: CardId) -> bool {
        self.castable_with(p, c, None)
    }

    /// Wie [`GameState::castable_now`], mit einer `MayPlay`-Erlaubnis (Flash, ohne Manakosten).
    /// Alternativkosten (`AlternativeCost`) von Handkarte `c`, die `p` jetzt nutzen kann.
    fn alt_cost_options(&self, p: PlayerId, c: CardId) -> Vec<usize> {
        use crate::statics::CompiledStatic;
        let card = self.card(c);
        let mut v = Vec::new();
        for (i, cs) in card.compiled().statics.iter().enumerate() {
            let CompiledStatic::AlternativeCost(alt) = cs.as_ref() else {
                continue;
            };
            let ctx = crate::expr::EvalCtx {
                source: c,
                controller: p,
                x: 0,
                targets: &[],
                trig: None,
                paid: &[],
            };
            if self.check_conditions(&alt.conditions, &ctx)
                && self.castable_base(p, c, false)
                && self.can_pay_nonmana(&alt.cost, c, p)
                && alt
                    .cost
                    .mana
                    .as_ref()
                    .is_none_or(|m| m.mana_value() <= self.potential_mana(p))
            {
                v.push(i);
            }
        }
        v
    }

    /// Zeitpunkt, Ziele und Zusatzkosten (ohne Manakosten).
    fn castable_base(&self, p: PlayerId, c: CardId, flash: bool) -> bool {
        if self.cant_cast(p, c) {
            return false;
        }
        let card = self.card(c);
        let ch = self.chars(c);
        let instant_speed =
            ch.types.has_core(forge_types::CoreType::Instant) || ch.has(&Keyword::Flash) || flash;
        if !(instant_speed || self.sorcery_timing(p)) {
            return false;
        }
        card.compiled().spell.as_ref().is_none_or(|spell| {
            self.with_assumed_x(self.potential_mana(p), || self.has_targets_for(spell, c, p))
                && spell
                    .cost
                    .as_ref()
                    .is_none_or(|cost| self.can_pay_nonmana(cost, c, p))
        })
    }

    fn castable_with(&self, p: PlayerId, c: CardId, grant: Option<&MayPlayGrant>) -> bool {
        if self.cant_cast(p, c) {
            return false;
        }
        let card = self.card(c);
        let ch = self.chars(c);
        let instant_speed = ch.types.has_core(forge_types::CoreType::Instant)
            || ch.has(&Keyword::Flash)
            || grant.is_some_and(|g| g.with_flash);
        if !(instant_speed || self.sorcery_timing(p)) {
            return false;
        }
        if let Some(spell) = &card.compiled().spell {
            let max_x = self.potential_mana(p);
            if !self.with_assumed_x(max_x, || self.has_targets_for(spell, c, p)) {
                return false;
            }
            // CR 601.2f/h: Zusatzkosten (`Cost$ … Sac<1/Creature>`) müssen zahlbar sein.
            if spell
                .cost
                .as_ref()
                .is_some_and(|cost| !self.can_pay_nonmana(cost, c, p))
            {
                return false;
            }
        }
        if grant.is_some_and(|g| g.without_mana_cost) {
            return true;
        }
        if ch.mana_cost.as_ref().is_some_and(|m| m.no_cost) {
            return false; // CR 118.6: ohne Manakosten nicht normal wirkbar
        }
        let mv = ch
            .mana_cost
            .as_ref()
            .map(|m| self.adjusted_spell_cost(p, c, m).mana_value())
            .unwrap_or(0);
        // Phyrexianische Symbole können mit Leben bezahlt werden (CR 119.4).
        let phy = ch
            .mana_cost
            .as_ref()
            .map(|m| m.shards.iter().filter(|s| s.phyrexian).count() as u32)
            .unwrap_or(0);
        let payable_with_life = phy.min((self.player(p).life.max(0) as u32) / 2);
        mv.saturating_sub(payable_with_life + self.keyword_helper_potential(p, c))
            <= self.potential_mana(p)
    }

    /// CR 707.10: Kopie eines Stapelobjekts unter Kontrolle von `p` auf den Stapel
    /// legen (nicht gewirkt, CR 707.10). Zauberkopien sind Objekte ohne Karte
    /// (hier Token, CR 707.10a/111.1): sie hören außerhalb des Stapels auf zu
    /// existieren bzw. werden als bleibende Karte zum Token (CR 608.3f).
    /// Neue Ziele optional (CR 707.10c).
    pub(crate) fn copy_stack_item(
        &mut self,
        orig: &StackItem,
        p: PlayerId,
        may_choose_new_targets: bool,
        ch: &mut Choices,
    ) -> Res<()> {
        let mut item = orig.clone();
        item.id = self.next_stack_id;
        self.next_stack_id += 1;
        item.controller = p;
        if let StackKind::Spell { card } = orig.kind {
            let o = self.card(card).clone();
            let new = self.create_card(o.def.clone(), p, Zone::Stack, true);
            {
                let n = self.card_mut(new);
                n.face = o.face;
                n.cast_x = o.cast_x;
                n.copy = o.copy.clone();
                n.chosen_color = o.chosen_color;
                n.chosen_type = o.chosen_type.clone();
            }
            item.kind = StackKind::Spell { card: new };
            item.source = new;
        }
        if may_choose_new_targets
            && let Some(ab) = item.ability.clone()
            && ab.chain().iter().any(|a| a.target.is_some())
            && matches!(
                ch.ask(Decision::Confirm {
                    player: p,
                    prompt: "Neue Ziele für die Kopie wählen?".into(),
                })?,
                Choice::Yes
            )
        {
            item.targets = self.choose_targets(p, item.source, &ab, ch)?;
        }
        self.push_event(Event::Copied {
            item: item.id,
            original: orig.id,
        });
        self.stack.push(item);
        self.fire_becomes_target();
        Ok(())
    }

    /// Kann `p` die Karte `c` durch einen `Play`-Effekt spielen? Zeitpunkt egal
    /// (CR 608.2g); Länder nur ohne `ValidSA$` und mit freiem Landspiel (CR 305.2).
    pub(crate) fn playable_by_effect(
        &self,
        p: PlayerId,
        c: CardId,
        valid_sa: Option<&crate::valid::ValidSpec>,
        source: CardId,
    ) -> bool {
        let ch = self.chars(c);
        if ch.types.is_land() {
            let pl = self.player(p);
            return valid_sa.is_none() && pl.lands_played_this_turn < pl.land_plays_per_turn;
        }
        if valid_sa.is_some_and(|v| !self.card_matches(c, v, source, p)) || self.cant_cast(p, c) {
            return false;
        }
        let card = self.card(c);
        card.compiled().spell.as_ref().is_none_or(|spell| {
            self.with_assumed_x(self.potential_mana(p), || self.has_targets_for(spell, c, p))
                && spell
                    .cost
                    .as_ref()
                    .is_none_or(|cost| self.can_pay_nonmana(cost, c, p))
        })
    }

    /// Spielt `c` durch einen Effekt: Land ins Spiel bzw. Zauber wirken (ohne
    /// Manakosten oder normal bezahlt). Bricht der Spieler die Bezahlung ab,
    /// bleibt alles unverändert (CR 728). Liefert das neue Objekt.
    pub(crate) fn play_by_effect(
        &mut self,
        p: PlayerId,
        c: CardId,
        without_mana_cost: bool,
        exile_instead_of_graveyard: bool,
        ch: &mut Choices,
    ) -> Res<Option<CardId>> {
        if self.chars(c).types.is_land() {
            let new = self.move_card_as(c, Zone::Battlefield, true, Some(p));
            self.player_mut(p).lands_played_this_turn += 1;
            self.push_event(Event::LandPlayed {
                player: p,
                card: new,
            });
            return Ok(Some(new));
        }
        let grant = MayPlayGrant {
            host: c,
            static_index: usize::MAX,
            without_mana_cost,
            with_flash: true,
            ignore_type: false,
            ignore_color: false,
        };
        let snapshot = self.clone();
        let before = self.stack.len();
        match self.cast_spell_free(p, c, Some(grant), None, ch) {
            Ok(()) => {}
            Err(Interrupt::Cancelled) => {
                *self = snapshot;
                return Ok(None);
            }
            Err(e) => return Err(e),
        }
        let new = self.stack[before..].iter().find_map(|i| match i.kind {
            StackKind::Spell { card } => Some(card),
            _ => None,
        });
        if let Some(new) = new
            && exile_instead_of_graveyard
        {
            self.card_mut(new).exile_from_stack = true;
        }
        Ok(new)
    }

    /// Obergrenze dessen, was Convoke/Improvise/Delve von `c` beitragen können.
    fn keyword_helper_potential(&self, p: PlayerId, c: CardId) -> u32 {
        let ch = self.chars(c);
        let mut n = 0;
        if ch.has(&Keyword::Convoke) || ch.has(&Keyword::Improvise) {
            n += self
                .battlefield_of(p)
                .filter(|&x| {
                    let xc = self.chars(x);
                    !self.card(x).tapped
                        && ((ch.has(&Keyword::Convoke)
                            && xc.types.has_core(forge_types::CoreType::Creature))
                            || (ch.has(&Keyword::Improvise)
                                && xc.types.has_core(forge_types::CoreType::Artifact)))
                })
                .count() as u32;
        }
        if ch.has(&Keyword::Delve) {
            n += self.player(p).graveyard.len() as u32;
        }
        n
    }

    /// CR 702.66 / 702.51 / 702.126: Delve, Convoke und Improvise senken die zu
    /// zahlenden Manakosten, indem Karten exiliert bzw. getappt werden.
    fn pay_with_keyword_helpers(
        &mut self,
        p: PlayerId,
        spell: CardId,
        cost: &mut ManaCost,
        ch: &mut Choices,
    ) -> Res<()> {
        let chars = self.chars(spell);
        // Delve: nur generisches Mana (CR 702.66a)
        if chars.has(&Keyword::Delve) && cost.generic > 0 {
            let options: Vec<CardId> = self
                .player(p)
                .graveyard
                .iter()
                .copied()
                .filter(|&c| c != spell)
                .collect();
            if !options.is_empty() {
                let max = cost.generic.min(options.len() as u32);
                let chosen = match ch.ask(Decision::ChooseCards {
                    player: p,
                    reason: CardChoiceReason::Delve,
                    options,
                    min: 0,
                    max,
                })? {
                    Choice::Cards(c) => c,
                    _ => Vec::new(),
                };
                for c in chosen.into_iter().take(max as usize) {
                    if self.card(c).zone == Some(Zone::Graveyard) {
                        self.move_card(c, Zone::Exile, true);
                        cost.generic -= 1;
                    }
                }
            }
        }
        // Convoke: generisch oder ein Symbol in der Farbe der Kreatur (CR 702.51a)
        if chars.has(&Keyword::Convoke) {
            let options: Vec<CardId> = self
                .battlefield_of(p)
                .filter(|&c| {
                    !self.card(c).tapped
                        && self
                            .chars(c)
                            .types
                            .has_core(forge_types::CoreType::Creature)
                })
                .collect();
            let colored = cost
                .shards
                .iter()
                .filter(|s| !s.x && !s.colors.is_colorless())
                .count() as u32;
            let useful = cost.generic + colored;
            if !options.is_empty() && useful > 0 {
                let max = useful.min(options.len() as u32);
                let mut chosen = match ch.ask(Decision::ChooseCards {
                    player: p,
                    reason: CardChoiceReason::Convoke,
                    options,
                    min: 0,
                    max,
                })? {
                    Choice::Cards(c) => c,
                    _ => Vec::new(),
                };
                // Wenigfarbige zuerst, damit seltene Farben ihr Symbol finden.
                chosen.sort_by_key(|&c| self.chars(c).colors.iter().count());
                for c in chosen.into_iter().take(max as usize) {
                    let colors = self.chars(c).colors;
                    let shard = cost
                        .shards
                        .iter()
                        .enumerate()
                        .filter(|(_, s)| !s.x && colors.iter().any(|col| s.colors.contains(col)))
                        .min_by_key(|(_, s)| s.colors.iter().count())
                        .map(|(i, _)| i);
                    if let Some(i) = shard {
                        cost.shards.remove(i);
                    } else if cost.generic > 0 {
                        cost.generic -= 1;
                    } else {
                        continue;
                    }
                    self.tap(c);
                }
            }
        }
        // Improvise: nur generisches Mana (CR 702.126a)
        if chars.has(&Keyword::Improvise) && cost.generic > 0 {
            let options: Vec<CardId> = self
                .battlefield_of(p)
                .filter(|&c| {
                    !self.card(c).tapped
                        && self
                            .chars(c)
                            .types
                            .has_core(forge_types::CoreType::Artifact)
                })
                .collect();
            if !options.is_empty() {
                let max = cost.generic.min(options.len() as u32);
                let chosen = match ch.ask(Decision::ChooseCards {
                    player: p,
                    reason: CardChoiceReason::Improvise,
                    options,
                    min: 0,
                    max,
                })? {
                    Choice::Cards(c) => c,
                    _ => Vec::new(),
                };
                for c in chosen.into_iter().take(max as usize) {
                    if self.tap(c) {
                        cost.generic -= 1;
                    }
                }
            }
        }
        Ok(())
    }

    /// `ActivationLimit$`/`GameActivationLimit$` noch nicht erreicht?
    fn below_activation_limit(&self, c: CardId, i: usize, ab: &CompiledAbility) -> bool {
        let acts = &self.card(c).activations;
        let turn = self.turn;
        let this_turn = acts.iter().filter(|(j, t)| *j == i && *t == turn).count() as u32;
        let ever = acts.iter().filter(|(j, _)| *j == i).count() as u32;
        ab.activation_limit.is_none_or(|n| this_turn < n)
            && ab.game_activation_limit.is_none_or(|n| ever < n)
    }

    /// Alle legalen Prioritätsaktionen von `p` (CR 117.1).
    pub fn legal_actions(&self, p: PlayerId) -> Vec<Action> {
        let _cache = crate::state::CharsCache::enter();
        let mut acts = vec![Action::Pass];
        let pl = self.player(p);
        let land_ok = self.sorcery_timing(p) && pl.lands_played_this_turn < pl.land_plays_per_turn;
        for &c in &pl.hand {
            let card = self.card(c);
            if !card.def.is_supported() {
                continue;
            }
            if self.chars(c).types.is_land() {
                // CR 305.2
                if land_ok {
                    acts.push(Action::PlayLand(c));
                }
            } else {
                if self.castable_now(p, c) {
                    acts.push(Action::Cast(c));
                }
                for i in self.alt_cost_options(p, c) {
                    acts.push(Action::CastWithAltCost(c, i));
                }
            }
            // Andere Seite (Adventure, modale doppelseitige Karte)
            if let Some(face) = card.def.alt_cast_face() {
                let mut g = self.clone();
                g.card_mut(c).face = face;
                if g.chars(c).types.is_land() {
                    if land_ok {
                        acts.push(Action::PlayLandAlt(c, face));
                    }
                } else if g.castable_now(p, c) {
                    acts.push(Action::CastAlt(c, face));
                }
            }
        }
        // Spielerlaubnisse (`MayPlay$`): Karten außerhalb der Hand
        for (c, grant) in self.may_play_grants(p) {
            let card = self.card(c);
            if !card.def.is_supported() {
                continue;
            }
            let act = if self.chars(c).types.is_land() {
                (land_ok && !grant.without_mana_cost).then_some(Action::PlayLand(c))
            } else {
                self.castable_with(p, c, Some(&grant))
                    .then_some(Action::Cast(c))
            };
            if let Some(a) = act
                && !acts.contains(&a)
            {
                acts.push(a);
            }
        }
        // CR 715.4: Karten „auf Abenteuer“ dürfen aus dem Exil als Kreatur gewirkt werden.
        for &c in &self.exile {
            let card = self.card(c);
            if card.on_adventure && card.owner == p && self.castable_now(p, c) {
                acts.push(Action::Cast(c));
            }
        }
        // Fähigkeiten aus Hand/Friedhof (`ActivationZone$`, z. B. Cycling)
        let pl = self.player(p);
        let off_battlefield: Vec<CardId> =
            pl.hand.iter().chain(pl.graveyard.iter()).copied().collect();
        for c in off_battlefield {
            let zone = self.card(c).zone;
            for (i, ab) in self.card(c).compiled().activated.iter().enumerate() {
                if Some(ab.activation_zone) != zone {
                    continue;
                }
                let cost_ok = ab.cost.as_ref().is_none_or(|cost| {
                    self.can_pay_nonmana(cost, c, p)
                        && cost
                            .mana
                            .as_ref()
                            .is_none_or(|m| m.mana_value() <= self.potential_mana(p))
                });
                let timing_ok = (!ab.sorcery_speed || self.sorcery_timing(p))
                    && self.below_activation_limit(c, i, ab);
                if cost_ok
                    && timing_ok
                    && self
                        .with_assumed_x(self.potential_mana(p), || self.has_targets_for(ab, c, p))
                    && !self.cant_activate(p, c, ab)
                {
                    acts.push(Action::Activate(c, i));
                }
            }
        }
        for c in self.battlefield_of(p).collect::<Vec<_>>() {
            for (i, ab) in self.card(c).compiled().activated.iter().enumerate() {
                if ab.activation_zone != Zone::Battlefield {
                    continue;
                }
                let cost_ok = ab.cost.as_ref().is_none_or(|cost| {
                    self.can_pay_nonmana(cost, c, p)
                        && cost
                            .mana
                            .as_ref()
                            .is_none_or(|m| m.mana_value() <= self.potential_mana(p))
                });
                // CR 606.3: Loyalitätsfähigkeiten nur als Hexerei und einmal pro Zug je Permanent.
                let timing_ok = if ab.planeswalker {
                    self.sorcery_timing(p) && self.card(c).loyalty_used_turn != Some(self.turn)
                } else {
                    ab.is_mana_ability() || !ab.sorcery_speed || self.sorcery_timing(p)
                } && self.below_activation_limit(c, i, ab);
                if cost_ok
                    && timing_ok
                    && self
                        .with_assumed_x(self.potential_mana(p), || self.has_targets_for(ab, c, p))
                    && !self.cant_activate(p, c, ab)
                {
                    acts.push(Action::Activate(c, i));
                }
            }
        }
        acts
    }

    /// Führt eine Prioritätsaktion aus (außer Pass).
    pub fn perform_action(&mut self, p: PlayerId, a: Action, ch: &mut Choices) -> Res<()> {
        match a {
            Action::Pass => Ok(()),
            Action::PlayLand(c) => {
                if self.card(c).zone != Some(Zone::Hand) {
                    self.use_may_play(p, c, false);
                }
                let new = self.move_card_as(c, Zone::Battlefield, true, Some(p));
                self.player_mut(p).lands_played_this_turn += 1;
                self.push_event(Event::LandPlayed {
                    player: p,
                    card: new,
                });
                Ok(())
            }
            Action::Cast(c) => {
                let grant = match self.card(c).zone {
                    Some(Zone::Hand) | None => None,
                    _ if self.card(c).on_adventure => None,
                    _ => self.use_may_play(p, c, true),
                };
                self.cast_spell_free(p, c, grant, None, ch)
            }
            Action::CastWithAltCost(c, i) => {
                let crate::statics::CompiledStatic::AlternativeCost(alt) =
                    self.card(c).compiled().statics[i].as_ref()
                else {
                    return Err(Interrupt::Cancelled);
                };
                let cost = alt.cost.clone();
                self.cast_spell_free(p, c, None, Some(cost), ch)
            }
            Action::CastAlt(c, face) => {
                // Die Seite bleibt beim Wechsel auf den Stapel erhalten.
                self.card_mut(c).face = face;
                self.cast_spell(p, c, ch)
            }
            Action::PlayLandAlt(c, face) => {
                self.card_mut(c).face = face;
                self.perform_action(p, Action::PlayLand(c), ch)
            }
            Action::Activate(c, i) => {
                let ab = self.card(c).compiled().activated[i].clone();
                let turn = self.turn;
                self.card_mut(c).activations.push((i, turn));
                if ab.is_mana_ability() {
                    self.activate_mana_ability(p, c, &ab, ch)
                } else {
                    self.activate_ability(p, c, i, ab, ch)
                }
            }
        }
    }

    /// CR 601.2c: Ziele für jede Fähigkeit der Kette wählen.
    pub(crate) fn choose_targets(
        &mut self,
        p: PlayerId,
        source: CardId,
        ab: &CompiledAbility,
        ch: &mut Choices,
    ) -> Res<Vec<Vec<Target>>> {
        let mut all = Vec::new();
        for a in ab.chain() {
            let Some(spec) = &a.target else {
                all.push(Vec::new());
                continue;
            };
            let candidates = self.target_candidates(spec, source, p);
            let max = spec.max.min(candidates.len() as u32);
            if (candidates.len() as u32) < spec.min {
                return Err(Interrupt::Cancelled);
            }
            let choice = ch.ask(Decision::ChooseTargets {
                player: p,
                source,
                description: spec.prompt.clone().unwrap_or_else(|| a.description.clone()),
                candidates,
                min: spec.min,
                max,
            })?;
            match choice {
                Choice::Targets(ts) => all.push(ts),
                _ => return Err(Interrupt::Cancelled),
            }
        }
        Ok(all)
    }

    fn cast_spell(&mut self, p: PlayerId, card: CardId, ch: &mut Choices) -> Res<()> {
        self.cast_spell_free(p, card, None, None, ch)
    }

    /// Nutzt die erste passende `MayPlay`-Erlaubnis (bevorzugt „ohne Manakosten“, wenn
    /// `prefer_free`); zählt `MayPlayLimit$`.
    fn use_may_play(&mut self, p: PlayerId, c: CardId, prefer_free: bool) -> Option<MayPlayGrant> {
        let mut grants: Vec<MayPlayGrant> = self
            .may_play_grants(p)
            .into_iter()
            .filter(|(x, _)| *x == c)
            .map(|(_, g)| g)
            .collect();
        if prefer_free {
            grants.sort_by_key(|g| !g.without_mana_cost);
        }
        let g = grants.first().copied()?;
        let turn = self.turn;
        self.card_mut(g.host)
            .may_play_uses
            .push((g.static_index, turn));
        Some(g)
    }

    fn cast_spell_free(
        &mut self,
        p: PlayerId,
        card: CardId,
        grant: Option<MayPlayGrant>,
        alt: Option<Cost>,
        ch: &mut Choices,
    ) -> Res<()> {
        // CR 601.2a: Auf den Stapel legen.
        let spell_ab = self.card(card).compiled().spell.clone();
        // CR 118.9 / 107.3: ohne Manakosten gewirkt → keine Manakosten, X = 0;
        // Alternativkosten ersetzen die Manakosten.
        let mut mana_cost = if grant.is_some_and(|g| g.without_mana_cost) {
            ManaCost::default()
        } else if let Some(a) = &alt {
            a.mana.clone().unwrap_or_default()
        } else {
            self.chars(card).mana_cost.clone().unwrap_or_default()
        };
        // CR 609.4b: Mana wie beliebigen Typs/beliebiger Farbe ausgeben → farbige
        // (bzw. alle nicht-generischen) Symbole wie generisches Mana behandeln.
        if let Some(g) = grant
            && (g.ignore_type || g.ignore_color)
        {
            mana_cost.shards.retain(|s| {
                let convertible =
                    !s.x && !s.phyrexian && (g.ignore_type || !s.colors.is_colorless());
                if convertible {
                    mana_cost.generic += 1;
                }
                !convertible
            });
        }
        let on_stack = self.move_card(card, Zone::Stack, true);
        self.card_mut(on_stack).controller = p;
        // X merken: für Werte, die sich beim Ankommen auf X beziehen (etbCounter)
        let remember_x_on = on_stack;
        let id = self.next_stack_id;
        self.next_stack_id += 1;

        // 601.2b: X wählen
        let x = if mana_cost.x_count() > 0 {
            let max = self.potential_mana(p);
            match ch.ask(Decision::ChooseX { player: p, max })? {
                Choice::Number(n) => n,
                _ => 0,
            }
        } else {
            0
        };
        self.card_mut(on_stack).last_x = x;
        // 601.2b: Modi
        let spell_ab = match spell_ab {
            Some(ab) => Some(self.choose_modes(p, on_stack, &ab, ch)?),
            None => None,
        };
        // 601.2c
        let targets = match &spell_ab {
            Some(ab) => self.choose_targets(p, on_stack, ab, ch)?,
            None => Vec::new(),
        };
        // 601.2f: Kostenänderungen (zuerst Erhöhungen, dann Senkungen)
        let mut mana_cost = self.adjusted_spell_cost(p, on_stack, &mana_cost);
        // 601.2g–h (Convoke/Improvise/Delve vor dem Mana, Java `CostAdjustment.adjust`)
        self.pay_with_keyword_helpers(p, on_stack, &mut mana_cost, ch)?;
        self.pay_mana_cost(p, &mana_cost, x, None, ch)?;
        // Zusatzkosten nach dem Mana (Java `Cost.sort`: Mana vor Zonenwechseln).
        let mut nonmana = spell_ab
            .as_ref()
            .and_then(|ab| ab.cost.clone())
            .unwrap_or_default();
        if let Some(a) = &alt {
            nonmana.parts.extend(a.parts.iter().cloned());
        }
        let paid = if nonmana.parts.is_empty() {
            Vec::new()
        } else {
            self.pay_nonmana_costs(p, on_stack, &nonmana, ch)?
        };

        self.card_mut(remember_x_on).cast_x = x;
        self.stack.push(StackItem {
            id,
            kind: StackKind::Spell { card: on_stack },
            source: on_stack,
            controller: p,
            ability: spell_ab,
            targets,
            x,
            paid,
        });
        self.this_turn.cast.push(on_stack);
        self.push_event(Event::Cast {
            player: p,
            card: on_stack,
        });
        // CR 601.2i: Zauber ist gewirkt → „whenever … cast“-Trigger
        self.fire(crate::triggers::TriggerEvent::SpellCast {
            card: on_stack,
            player: p,
        });
        self.fire_becomes_target();
        Ok(())
    }

    /// „Wird zum Ziel“ für das oberste Stapelobjekt (CR 115.1, Trigger `BecomesTarget`).
    pub(crate) fn fire_becomes_target(&mut self) {
        let Some(item) = self.stack.last() else {
            return;
        };
        let mut seen: Vec<Target> = Vec::new();
        for t in item.targets.iter().flatten() {
            if !seen.contains(t) {
                seen.push(*t);
            }
        }
        let (id, source, controller) = (item.id, item.source, item.controller);
        let is_spell = matches!(item.kind, StackKind::Spell { .. });
        for target in seen {
            self.fire(crate::triggers::TriggerEvent::BecomesTarget {
                target,
                source,
                stack_item: id,
                is_spell,
                controller,
            });
        }
    }

    fn activate_ability(
        &mut self,
        p: PlayerId,
        source: CardId,
        index: usize,
        ab: Arc<CompiledAbility>,
        ch: &mut Choices,
    ) -> Res<()> {
        let id = self.next_stack_id;
        self.next_stack_id += 1;
        let cost = ab.cost.clone().unwrap_or_default();
        let mana = cost.mana.clone().unwrap_or_default();
        let x = if mana.x_count() > 0 {
            let max = self.potential_mana(p);
            match ch.ask(Decision::ChooseX { player: p, max })? {
                Choice::Number(n) => n,
                _ => 0,
            }
        } else {
            0
        };
        self.card_mut(source).last_x = x;
        let ab = self.choose_modes(p, source, &ab, ch)?;
        let targets = self.choose_targets(p, source, &ab, ch)?;
        let paid = self.pay_nonmana_costs(p, source, &cost, ch)?;
        if cost.mana.is_some() {
            self.pay_mana_cost(p, &mana, x, Some(source), ch)?;
        }
        if ab.planeswalker {
            let turn = self.turn;
            self.card_mut(source).loyalty_used_turn = Some(turn);
        }
        if ab.cycling {
            self.fire(crate::triggers::TriggerEvent::Cycled {
                card: source,
                player: p,
            });
        }
        self.stack.push(StackItem {
            id,
            kind: StackKind::Ability,
            source,
            controller: p,
            ability: Some(ab),
            targets,
            x,
            paid,
        });
        self.push_event(Event::Activated {
            player: p,
            card: source,
            index,
        });
        self.fire_becomes_target();
        Ok(())
    }

    /// CR 605.3b: Mana-Fähigkeiten lösen sofort auf, ohne Stapel.
    pub fn activate_mana_ability(
        &mut self,
        p: PlayerId,
        source: CardId,
        ab: &CompiledAbility,
        ch: &mut Choices,
    ) -> Res<()> {
        let cost = ab.cost.clone().unwrap_or_default();
        let paid = self.pay_nonmana_costs(p, source, &cost, ch)?;
        let ctx = ResolveCtx {
            source,
            controller: p,
            x: 0,
            trig: None,
            paid,
        };
        self.apply_chain(ab, &[], &ctx, ch)
    }

    pub(crate) fn pay_nonmana_costs(
        &mut self,
        p: PlayerId,
        source: CardId,
        cost: &Cost,
        ch: &mut Choices,
    ) -> Res<Vec<Paid>> {
        if !self.can_pay_nonmana(cost, source, p) {
            return Err(Interrupt::Cancelled);
        }
        self.begin_zone_batch();
        let mut paid = Vec::new();
        let r = self.pay_nonmana_parts(p, source, cost, ch, &mut paid);
        self.end_zone_batch();
        r.map(|()| paid)
    }

    fn pay_nonmana_parts(
        &mut self,
        p: PlayerId,
        source: CardId,
        cost: &Cost,
        ch: &mut Choices,
        paid: &mut Vec<Paid>,
    ) -> Res<()> {
        let mut note = |kind, card| paid.push(Paid { kind, card });
        for part in &cost.parts {
            match part {
                CostPart::Tap => {
                    self.tap(source);
                }
                CostPart::Untap => self.card_mut(source).tapped = false,
                CostPart::SacrificeSelf => {
                    if self.sacrifice(source) {
                        note(PaidKind::Sacrificed, source);
                    }
                }
                CostPart::PayLife(n) => self.lose_life(p, *n as i32),
                CostPart::AddCounter { amount, kind } => self.add_counters(source, kind, *amount),
                CostPart::DiscardSelf => {
                    self.discard(source);
                    note(PaidKind::Discarded, source);
                }
                CostPart::RemoveCounter { amount, kind } => {
                    self.remove_counters(source, kind, *amount);
                }
                CostPart::Sacrifice { amount, valid } => {
                    let options: Vec<CardId> = self
                        .battlefield_of(p)
                        .filter(|c| self.card_matches(*c, valid, source, p))
                        .collect();
                    let chosen = match ch.ask(Decision::ChooseCards {
                        player: p,
                        reason: CardChoiceReason::SacrificeCost,
                        options,
                        min: *amount,
                        max: *amount,
                    })? {
                        Choice::Cards(c) => c,
                        _ => return Err(Interrupt::Cancelled),
                    };
                    for c in chosen {
                        if self.sacrifice(c) {
                            note(PaidKind::Sacrificed, c);
                        }
                    }
                }
                CostPart::Discard { amount, valid } => {
                    let options: Vec<CardId> = self
                        .player(p)
                        .hand
                        .iter()
                        .copied()
                        .filter(|c| *c != source && self.card_matches(*c, valid, source, p))
                        .collect();
                    let chosen = match ch.ask(Decision::ChooseCards {
                        player: p,
                        reason: CardChoiceReason::DiscardCost,
                        options,
                        min: *amount,
                        max: *amount,
                    })? {
                        Choice::Cards(c) => c,
                        _ => return Err(Interrupt::Cancelled),
                    };
                    for c in chosen {
                        self.discard(c);
                        note(PaidKind::Discarded, c);
                    }
                }
                CostPart::ExileSelfFromGraveyard
                | CostPart::ExileSelf
                | CostPart::ExileSelfFromHand => {
                    self.move_card(source, Zone::Exile, true);
                    note(PaidKind::Exiled, source);
                }
                CostPart::ExileFromHand { amount, valid } => {
                    let options: Vec<CardId> = self
                        .player(p)
                        .hand
                        .iter()
                        .copied()
                        .filter(|c| *c != source && self.card_matches(*c, valid, source, p))
                        .collect();
                    for c in self.choose_cost_cards(
                        p,
                        CardChoiceReason::ExileCost,
                        options,
                        *amount,
                        ch,
                    )? {
                        self.move_card(c, Zone::Exile, true);
                        note(PaidKind::Exiled, c);
                    }
                }
                CostPart::ReturnSelf => {
                    self.move_card(source, Zone::Hand, true);
                    note(PaidKind::Returned, source);
                }
                CostPart::ExileFromGraveyard { amount, valid } => {
                    let options: Vec<CardId> = self
                        .player(p)
                        .graveyard
                        .iter()
                        .copied()
                        .filter(|c| self.card_matches(*c, valid, source, p))
                        .collect();
                    for c in self.choose_cost_cards(
                        p,
                        CardChoiceReason::ExileCost,
                        options,
                        *amount,
                        ch,
                    )? {
                        self.move_card(c, Zone::Exile, true);
                        note(PaidKind::Exiled, c);
                    }
                }
                CostPart::TapType { amount, valid } => {
                    let options = self.tap_type_options(cost, valid, source, p);
                    for c in
                        self.choose_cost_cards(p, CardChoiceReason::TapCost, options, *amount, ch)?
                    {
                        self.tap(c);
                        note(PaidKind::Tapped, c);
                    }
                }
                CostPart::Return { amount, valid } => {
                    let options: Vec<CardId> = self
                        .battlefield_of(p)
                        .filter(|c| self.card_matches(*c, valid, source, p))
                        .collect();
                    for c in self.choose_cost_cards(
                        p,
                        CardChoiceReason::ReturnCost,
                        options,
                        *amount,
                        ch,
                    )? {
                        self.move_card(c, Zone::Hand, true);
                        note(PaidKind::Returned, c);
                    }
                }
            }
        }
        Ok(())
    }

    /// Wählbare Permanents für `tapXType`: ungetappt, eigener Filter; mit `{T}` in
    /// denselben Kosten nie die Quelle selbst (Java `CostTapType.canTapSource`).
    fn tap_type_options(
        &self,
        cost: &Cost,
        valid: &crate::valid::ValidSpec,
        source: CardId,
        p: PlayerId,
    ) -> Vec<CardId> {
        let taps_source = cost.parts.contains(&CostPart::Tap);
        self.battlefield_of(p)
            .filter(|c| {
                !(taps_source && *c == source)
                    && !self.card(*c).tapped
                    && self.card_matches(*c, valid, source, p)
            })
            .collect()
    }

    /// Genau `amount` Karten für einen Kostenteil wählen lassen.
    fn choose_cost_cards(
        &mut self,
        p: PlayerId,
        reason: CardChoiceReason,
        options: Vec<CardId>,
        amount: u32,
        ch: &mut Choices,
    ) -> Res<Vec<CardId>> {
        match ch.ask(Decision::ChooseCards {
            player: p,
            reason,
            options,
            min: amount,
            max: amount,
        })? {
            Choice::Cards(c) => Ok(c),
            _ => Err(Interrupt::Cancelled),
        }
    }

    /// CR 601.2g–h: Mana-Fähigkeiten aktivieren und Manakosten bezahlen.
    fn pay_mana_cost(
        &mut self,
        p: PlayerId,
        cost: &ManaCost,
        x: u32,
        _source: Option<CardId>,
        ch: &mut Choices,
    ) -> Res<()> {
        // Phyrexianische Symbole: Leben oder Mana (CR 107.4f, 119.4)
        let mut phy = Vec::new();
        let mut life_to_pay = 0;
        for s in cost.shards.iter().filter(|s| s.phyrexian) {
            let can_life = self.player(p).life >= life_to_pay + 2;
            let pay_life = can_life
                && matches!(
                    ch.ask(Decision::PayLifeForPhyrexian {
                        player: p,
                        symbol: s.to_string()
                    })?,
                    Choice::Yes
                );
            if pay_life {
                life_to_pay += 2;
                phy.push(PhyrexianPay::Life);
            } else {
                phy.push(PhyrexianPay::Mana);
            }
        }
        loop {
            let mut pool = self.player(p).mana_pool.clone();
            if pool.try_pay(cost, x, &phy) {
                self.player_mut(p).mana_pool = pool;
                break;
            }
            let mut options: Vec<PayAction> = self
                .available_mana_abilities(p)
                .into_iter()
                .map(|(c, i)| PayAction::ActivateMana(c, i))
                .collect();
            let plan = self.auto_pay_plan(p, cost, x, &phy);
            if plan.is_some() {
                options.push(PayAction::AutoPay);
            }
            options.push(PayAction::Cancel);
            let choice = ch.ask(Decision::PayMana {
                player: p,
                remaining: format!("{cost} (X={x})"),
                options,
            })?;
            match choice {
                Choice::Pay(PayAction::ActivateMana(c, i)) => {
                    let ab = self.card(c).compiled().activated[i].clone();
                    self.activate_mana_ability(p, c, &ab, ch)?;
                }
                Choice::Pay(PayAction::AutoPay) => {
                    let Some(plan) = plan else {
                        return Err(Interrupt::Cancelled);
                    };
                    for (c, i, color) in plan {
                        let ab = self.card(c).compiled().activated[i].clone();
                        let cost = ab.cost.clone().unwrap_or_default();
                        self.pay_nonmana_costs(p, c, &cost, ch)?;
                        let snow = self.chars(c).types.has_super(forge_types::Supertype::Snow);
                        self.player_mut(p).mana_pool.add(color, snow);
                    }
                }
                _ => return Err(Interrupt::Cancelled),
            }
        }
        self.lose_life(p, life_to_pay);
        Ok(())
    }

    /// Sucht eine Menge einfacher Mana-Quellen ({T}: ein Mana), mit der die
    /// Kosten bezahlt werden können. (Java: `ComputerUtilMana.payManaCost`)
    pub fn auto_pay_plan(
        &self,
        p: PlayerId,
        cost: &ManaCost,
        x: u32,
        phy: &[PhyrexianPay],
    ) -> Option<Vec<(CardId, usize, Option<Color>)>> {
        let sources: Vec<(CardId, usize, Vec<Option<Color>>)> = self
            .available_mana_abilities(p)
            .into_iter()
            .filter_map(|(c, i)| {
                let ab = &self.card(c).compiled().activated[i];
                if ab.sub.is_some() {
                    return None;
                }
                match &ab.effect {
                    crate::effects::Effect::AddMana {
                        produce: ManaProduce::Fixed(v),
                        amount: crate::effects::Amount::Fixed(1),
                    } if v.len() == 1 => Some((c, i, v.clone())),
                    crate::effects::Effect::AddMana {
                        produce: ManaProduce::OneOf(cols),
                        amount: crate::effects::Amount::Fixed(1),
                    } => Some((c, i, cols.iter().map(|c| Some(*c)).collect())),
                    crate::effects::Effect::AddMana {
                        produce: ManaProduce::ChosenColor,
                        amount: crate::effects::Amount::Fixed(1),
                    } => self.card(c).chosen_color.map(|col| (c, i, vec![Some(col)])),
                    _ => None,
                }
            })
            .collect();
        let base = self.player(p).mana_pool.clone();
        let mut chosen: Vec<(CardId, usize, Option<Color>)> = Vec::new();
        let mut budget = 50_000u32;

        #[allow(clippy::too_many_arguments)]
        fn dfs(
            idx: usize,
            sources: &[(CardId, usize, Vec<Option<Color>>)],
            pool: &ManaPool,
            chosen: &mut Vec<(CardId, usize, Option<Color>)>,
            cost: &ManaCost,
            x: u32,
            phy: &[PhyrexianPay],
            budget: &mut u32,
        ) -> bool {
            if *budget == 0 {
                return false;
            }
            *budget -= 1;
            let mut test = pool.clone();
            for (_, _, col) in chosen.iter() {
                test.add(*col, false);
            }
            if test.try_pay(cost, x, phy) {
                return true;
            }
            if idx >= sources.len() {
                return false;
            }
            let (c, i, cols) = &sources[idx];
            // Eine Karte kann nur einmal getappt werden (z. B. Duallands mit zwei Fähigkeiten).
            if chosen.iter().any(|(cc, _, _)| cc == c) {
                return dfs(idx + 1, sources, pool, chosen, cost, x, phy, budget);
            }
            for col in cols {
                chosen.push((*c, *i, *col));
                if dfs(idx + 1, sources, pool, chosen, cost, x, phy, budget) {
                    return true;
                }
                chosen.pop();
            }
            dfs(idx + 1, sources, pool, chosen, cost, x, phy, budget)
        }

        if dfs(0, &sources, &base, &mut chosen, cost, x, phy, &mut budget) {
            Some(chosen)
        } else {
            None
        }
    }
}

impl GameState {
    /// CR 601.2f: Gesamtkosten eines Zauberspruchs nach `RaiseCost`/`ReduceCost`
    /// (nur generischer Anteil; Senkung nicht unter 0).
    pub fn adjusted_spell_cost(
        &self,
        caster: PlayerId,
        spell: CardId,
        base: &ManaCost,
    ) -> ManaCost {
        let mut raise = 0i32;
        let mut reduce = 0i32;
        // Der Zauber selbst zählt immer (beim Wirken liegt er schon in der Stack-Zone,
        // ist aber noch kein StackItem).
        let sources = self
            .live_cards()
            .chain(std::iter::once(spell))
            .collect::<std::collections::BTreeSet<_>>();
        let sources = sources.into_iter().map(|id| self.card(id)).filter(|c| {
            c.zone.is_some()
                && c.compiled()
                    .statics
                    .iter()
                    .any(|s| matches!(s.as_ref(), crate::statics::CompiledStatic::CostAdjust(_)))
        });
        for src in sources {
            let zone = src.zone.expect("gefiltert");
            for s in &src.compiled().statics {
                let crate::statics::CompiledStatic::CostAdjust(a) = s.as_ref() else {
                    continue;
                };
                if !a.effect_zones.contains(&zone) {
                    continue;
                }
                let ctx = crate::expr::EvalCtx {
                    source: src.id,
                    controller: src.controller,
                    x: 0,
                    targets: &[],
                    trig: None,
                    paid: &[],
                };
                if !self.check_conditions(&a.conditions, &ctx) {
                    continue;
                }
                if a.activator
                    .as_ref()
                    .is_some_and(|v| !self.player_matches(caster, v, src.controller))
                {
                    continue;
                }
                if !self.card_matches(spell, &a.valid_card, src.id, src.controller) {
                    continue;
                }
                let n = self.eval(&a.amount, &ctx).max(0);
                if a.reduce {
                    reduce += n;
                } else {
                    raise += n;
                }
            }
        }
        // CR 702.41: Affinity for … (Java: ReduceCost-Static auf dem Zauber selbst)
        for k in &self.chars(spell).keywords {
            if let Keyword::Affinity(spec) = k
                && let Ok(v) = crate::valid::ValidSpec::parse(spec)
            {
                reduce += self
                    .battlefield_of(caster)
                    .filter(|&c| self.card_matches(c, &v, spell, caster))
                    .count() as i32;
            }
        }
        let mut cost = base.clone();
        let g = cost.generic as i32 + raise - reduce;
        cost.generic = g.max(0) as u32;
        cost
    }
}
