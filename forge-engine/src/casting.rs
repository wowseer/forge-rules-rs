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
use crate::state::{Choices, Event, GameState, Interrupt, Res, StackItem, StackKind};
use crate::zone::Zone;
use forge_types::{Color, ManaCost};
use std::sync::Arc;

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

    fn can_pay_nonmana(&self, cost: &Cost, source: CardId, p: PlayerId) -> bool {
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
        self.player(p).mana_pool.len() as u32 + self.available_mana_abilities(p).len() as u32
    }

    /// Sind für jede Fähigkeit der Kette mit Pflichtzielen Ziele vorhanden?
    fn has_targets_for(&self, ab: &CompiledAbility, source: CardId, p: PlayerId) -> bool {
        ab.chain().iter().all(|a| match &a.target {
            Some(spec) => spec.min == 0 || !self.target_candidates(spec, source, p).is_empty(),
            None => true,
        })
    }

    /// Alle legalen Prioritätsaktionen von `p` (CR 117.1).
    pub fn legal_actions(&self, p: PlayerId) -> Vec<Action> {
        let mut acts = vec![Action::Pass];
        let pl = self.player(p);
        for &c in &pl.hand {
            let card = self.card(c);
            if !card.def.is_supported() {
                continue;
            }
            let ch = self.chars(c);
            if ch.types.is_land() {
                // CR 305.2
                if self.sorcery_timing(p) && pl.lands_played_this_turn < pl.land_plays_per_turn {
                    acts.push(Action::PlayLand(c));
                }
                continue;
            }
            let instant_speed =
                ch.types.has_core(forge_types::CoreType::Instant) || ch.has(&Keyword::Flash);
            if !(instant_speed || self.sorcery_timing(p)) {
                continue;
            }
            if let Some(spell) = &card.compiled().spell
                && !self.has_targets_for(spell, c, p)
            {
                continue;
            }
            let mv = ch.mana_cost.as_ref().map(ManaCost::mana_value).unwrap_or(0);
            if ch.mana_cost.as_ref().is_some_and(|m| m.no_cost) {
                continue; // CR 118.6: ohne Manakosten nicht normal wirkbar
            }
            // Phyrexianische Symbole können mit Leben bezahlt werden (CR 119.4).
            let phy = ch
                .mana_cost
                .as_ref()
                .map(|m| m.shards.iter().filter(|s| s.phyrexian).count() as u32)
                .unwrap_or(0);
            let payable_with_life = phy.min((self.player(p).life.max(0) as u32) / 2);
            if mv.saturating_sub(payable_with_life) <= self.potential_mana(p) {
                acts.push(Action::Cast(c));
            }
        }
        for c in self.battlefield_of(p).collect::<Vec<_>>() {
            for (i, ab) in self.card(c).compiled().activated.iter().enumerate() {
                let cost_ok = ab.cost.as_ref().is_none_or(|cost| {
                    self.can_pay_nonmana(cost, c, p)
                        && cost
                            .mana
                            .as_ref()
                            .is_none_or(|m| m.mana_value() <= self.potential_mana(p))
                });
                let timing_ok = ab.is_mana_ability() || !ab.sorcery_speed || self.sorcery_timing(p);
                if cost_ok && timing_ok && self.has_targets_for(ab, c, p) {
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
                let new = self.move_card(c, Zone::Battlefield, true);
                self.card_mut(new).controller = p;
                self.player_mut(p).lands_played_this_turn += 1;
                self.push_event(Event::LandPlayed {
                    player: p,
                    card: new,
                });
                Ok(())
            }
            Action::Cast(c) => self.cast_spell(p, c, ch),
            Action::Activate(c, i) => {
                let ab = self.card(c).compiled().activated[i].clone();
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
        // CR 601.2a: Auf den Stapel legen.
        let spell_ab = self.card(card).compiled().spell.clone();
        let mana_cost = self.chars(card).mana_cost.clone().unwrap_or_default();
        let on_stack = self.move_card(card, Zone::Stack, true);
        self.card_mut(on_stack).controller = p;
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
        // 601.2c
        let targets = match &spell_ab {
            Some(ab) => self.choose_targets(p, on_stack, ab, ch)?,
            None => Vec::new(),
        };
        // 601.2f–h
        self.pay_mana_cost(p, &mana_cost, x, None, ch)?;

        self.stack.push(StackItem {
            id,
            kind: StackKind::Spell { card: on_stack },
            source: on_stack,
            controller: p,
            ability: spell_ab,
            targets,
            x,
        });
        self.push_event(Event::Cast {
            player: p,
            card: on_stack,
        });
        // CR 601.2i: Zauber ist gewirkt → „whenever … cast“-Trigger
        self.fire(crate::triggers::TriggerEvent::SpellCast {
            card: on_stack,
            player: p,
        });
        Ok(())
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
        let targets = self.choose_targets(p, source, &ab, ch)?;
        self.pay_nonmana_costs(p, source, &cost, ch)?;
        if cost.mana.is_some() {
            self.pay_mana_cost(p, &mana, x, Some(source), ch)?;
        }
        self.stack.push(StackItem {
            id,
            kind: StackKind::Ability,
            source,
            controller: p,
            ability: Some(ab),
            targets,
            x,
        });
        self.push_event(Event::Activated {
            player: p,
            card: source,
            index,
        });
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
        self.pay_nonmana_costs(p, source, &cost, ch)?;
        let ctx = ResolveCtx {
            source,
            controller: p,
            x: 0,
            trig: None,
        };
        self.apply_chain(ab, &[], &ctx, ch)
    }

    fn pay_nonmana_costs(
        &mut self,
        p: PlayerId,
        source: CardId,
        cost: &Cost,
        ch: &mut Choices,
    ) -> Res<()> {
        if !self.can_pay_nonmana(cost, source, p) {
            return Err(Interrupt::Cancelled);
        }
        for part in &cost.parts {
            match part {
                CostPart::Tap => self.card_mut(source).tapped = true,
                CostPart::Untap => self.card_mut(source).tapped = false,
                CostPart::SacrificeSelf => self.put_into_graveyard(source),
                CostPart::PayLife(n) => self.lose_life(p, *n as i32),
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
                        self.put_into_graveyard(c);
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
                        self.move_card(c, Zone::Graveyard, true);
                    }
                }
            }
        }
        Ok(())
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
                        amount: 1,
                    } if v.len() == 1 => Some((c, i, v.clone())),
                    crate::effects::Effect::AddMana {
                        produce: ManaProduce::OneOf(cols),
                        amount: 1,
                    } => Some((c, i, cols.iter().map(|c| Some(*c)).collect())),
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
