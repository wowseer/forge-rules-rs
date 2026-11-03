//! Auflösen von Stapelobjekten (CR 608).
//!
//! Java-Vorbild: `MagicStack.resolveStack`, `AbilityUtils.resolve`,
//! die einzelnen `*Effect.resolve`-Methoden.

use super::DiscardMode;
use super::{Amount, CompiledAbility, Defined, Effect, ManaProduce};
use crate::decision::{Choice, Decision, Target};
use crate::ids::{CardId, PlayerId};
use crate::state::{
    Choices, ContinuousEffect, Duration, Event, GameState, Res, StackItem, StackKind,
};
use crate::zone::Zone;

/// Kontext einer Auflösung.
pub struct ResolveCtx {
    pub source: CardId,
    pub controller: PlayerId,
    pub x: u32,
    /// Bei ausgelösten Fähigkeiten: Wissen zum Auslösezeitpunkt.
    pub trig: Option<crate::triggers::TriggerInfo>,
    /// Als Kosten bezahlte Objekte.
    pub paid: Vec<crate::state::Paid>,
}

impl ResolveCtx {
    pub fn eval(&self) -> crate::expr::EvalCtx<'_> {
        self.eval_with(&[])
    }

    /// Wie [`ResolveCtx::eval`], mit den Zielen der auflösenden Fähigkeit.
    pub fn eval_with<'a>(&'a self, targets: &'a [Target]) -> crate::expr::EvalCtx<'a> {
        crate::expr::EvalCtx {
            source: self.source,
            controller: self.controller,
            x: self.x,
            targets,
            trig: self.trig.as_ref(),
            paid: &self.paid,
        }
    }
}

impl GameState {
    pub(crate) fn amount(&self, a: &Amount, ctx: &ResolveCtx, targets: &[Target]) -> i32 {
        match a {
            Amount::Fixed(n) => *n,
            Amount::XPaid => ctx.x as i32,
            Amount::Expr(e) => self.eval(e, &ctx.eval_with(targets)),
        }
    }

    /// Wen betrifft `Defined` – als Ziele (Karten/Spieler).
    fn resolve_defined(&self, d: Defined, targets: &[Target], ctx: &ResolveCtx) -> Vec<Target> {
        match d {
            Defined::You => vec![Target::Player(ctx.controller)],
            Defined::SelfCard => {
                // Zonenfilter macht der jeweilige Effekt (meist: nur Battlefield).
                if self.card(ctx.source).zone.is_some() {
                    vec![Target::Card(ctx.source)]
                } else {
                    vec![]
                }
            }
            Defined::Targeted => targets.to_vec(),
            Defined::TargetedController => targets
                .iter()
                .filter_map(|t| match t {
                    Target::Card(c) => Some(Target::Player(self.card(*c).controller)),
                    Target::Player(_) => None,
                })
                .collect(),
            Defined::EachPlayer => self.apnap().into_iter().map(Target::Player).collect(),
            Defined::EachOpponent => self
                .opponents(ctx.controller)
                .into_iter()
                .map(Target::Player)
                .collect(),
            Defined::TriggeredCard => ctx
                .trig
                .as_ref()
                .and_then(|t| t.card)
                .map(Target::Card)
                .into_iter()
                .collect(),
            Defined::TriggeredCardLki => ctx
                .trig
                .as_ref()
                .and_then(|t| t.card_lki.or(t.card))
                .map(Target::Card)
                .into_iter()
                .collect(),
            Defined::TriggeredCardController => ctx
                .trig
                .as_ref()
                .and_then(|t| t.card_lki.or(t.card))
                .map(|c| Target::Player(self.card(c).controller))
                .into_iter()
                .collect(),
            Defined::TriggeredPlayer => ctx
                .trig
                .as_ref()
                .and_then(|t| t.player)
                .map(Target::Player)
                .into_iter()
                .collect(),
            Defined::TriggeredTarget => ctx
                .trig
                .as_ref()
                .and_then(|t| t.target)
                .into_iter()
                .collect(),
            Defined::ChosenPlayer => self
                .card(ctx.source)
                .chosen_player
                .map(Target::Player)
                .into_iter()
                .collect(),
            Defined::ExiledWith => self
                .exile
                .iter()
                .filter(|c| self.card(**c).exiled_with == Some(ctx.source))
                .map(|c| Target::Card(*c))
                .collect(),
            Defined::Enchanted => self
                .card(ctx.source)
                .attached_to
                .filter(|c| self.card(*c).zone.is_some())
                .map(Target::Card)
                .into_iter()
                .collect(),
            Defined::EnchantedController => self
                .card(ctx.source)
                .attached_to
                .filter(|c| self.card(*c).zone.is_some())
                .map(|c| Target::Player(self.card(c).controller))
                .into_iter()
                .collect(),
            Defined::ChosenCard => self
                .card(ctx.source)
                .chosen_cards
                .iter()
                .map(|c| Target::Card(self.current_object(*c)))
                .collect(),
            Defined::Remembered => self
                .remembered_current(ctx.source)
                .into_iter()
                .map(Target::Card)
                .chain(
                    self.card(ctx.source)
                        .remembered_players
                        .iter()
                        .map(|p| Target::Player(*p)),
                )
                .collect(),
            Defined::DelayTriggerRememberedLki => ctx
                .trig
                .as_ref()
                .map(|t| t.delay_remembered.clone())
                .unwrap_or_default(),
            Defined::DelayTriggerRemembered => ctx
                .trig
                .as_ref()
                .map(|t| {
                    t.delay_remembered
                        .iter()
                        .map(|x| match x {
                            Target::Card(c) => Target::Card(self.current_object(*c)),
                            p => *p,
                        })
                        .collect()
                })
                .unwrap_or_default(),
            Defined::TriggeredBlocker => ctx
                .trig
                .as_ref()
                .and_then(|t| t.blocker.or(t.card))
                .map(Target::Card)
                .into_iter()
                .collect(),
            Defined::ParentTarget => {
                if self.parent_targets.is_empty() {
                    targets.to_vec()
                } else {
                    self.parent_targets.clone()
                }
            }
            Defined::TriggeredCards => ctx
                .trig
                .as_ref()
                .map(|t| t.cards.iter().map(|c| Target::Card(*c)).collect())
                .unwrap_or_default(),
            Defined::TriggeredAttackingPlayer => ctx
                .trig
                .as_ref()
                .and_then(|t| t.attacking_player)
                .map(Target::Player)
                .into_iter()
                .collect(),
            Defined::RememberedLki => self
                .card(ctx.source)
                .remembered
                .iter()
                .copied()
                .map(Target::Card)
                .chain(
                    self.card(ctx.source)
                        .remembered_players
                        .iter()
                        .map(|p| Target::Player(*p)),
                )
                .collect(),
            Defined::TriggeredSource => ctx
                .trig
                .as_ref()
                .and_then(|t| t.source)
                .map(Target::Card)
                .into_iter()
                .collect(),
        }
    }

    /// Karten aus Zielen, unabhängig von der Zone.
    fn cards_of_any(ts: &[Target]) -> Vec<CardId> {
        ts.iter()
            .filter_map(|t| match t {
                Target::Card(c) => Some(*c),
                Target::Player(_) => None,
            })
            .collect()
    }

    fn players_of(ts: &[Target]) -> Vec<PlayerId> {
        ts.iter()
            .filter_map(|t| {
                if let Target::Player(p) = t {
                    Some(*p)
                } else {
                    None
                }
            })
            .collect()
    }

    fn cards_of(&self, ts: &[Target]) -> Vec<CardId> {
        ts.iter()
            .filter_map(|t| {
                if let Target::Card(c) = t {
                    Some(*c)
                } else {
                    None
                }
            })
            .filter(|c| self.card(*c).zone == Some(Zone::Battlefield))
            .collect()
    }

    /// Führt einen einzelnen Effekt aus.
    pub fn apply_effect(
        &mut self,
        eff: &Effect,
        targets: &[Target],
        ctx: &ResolveCtx,
        ch: &mut Choices,
    ) -> Res<()> {
        // Zonenwechsel eines Effekts gelten als ein Ereignis (ChangesZoneAll).
        self.begin_zone_batch();
        let prev = self.effect_source.replace(ctx.source);
        let r = self.apply_effect_inner(eff, targets, ctx, ch);
        self.effect_source = prev;
        self.end_zone_batch();
        r
    }

    fn apply_effect_inner(
        &mut self,
        eff: &Effect,
        targets: &[Target],
        ctx: &ResolveCtx,
        ch: &mut Choices,
    ) -> Res<()> {
        match eff {
            Effect::DealDamage { amount, to, source } => {
                let n = self.amount(amount, ctx, targets).max(0) as u32;
                // `DamageSource$`: erstes passende Objekt (auch LKI, Java)
                let src = match source {
                    None => ctx.source,
                    Some(d) => {
                        match Self::cards_of_any(&self.resolve_defined(*d, targets, ctx)).first() {
                            Some(c) => *c,
                            None => return Ok(()),
                        }
                    }
                };
                self.begin_damage_batch();
                for t in self.resolve_defined(*to, targets, ctx) {
                    // CR 601.2d: aufgeteilte Menge je Ziel
                    let n = self
                        .current_division
                        .iter()
                        .find(|(x, _)| *x == t)
                        .map_or(n, |(_, v)| *v);
                    self.deal_damage(src, t, n);
                }
                self.end_damage_batch();
            }
            Effect::Draw { amount, who } => {
                let n = self.amount(amount, ctx, targets).max(0) as u32;
                for p in Self::players_of(&self.resolve_defined(*who, targets, ctx)) {
                    self.draw_n(p, n);
                }
            }
            Effect::GainLife { amount, who } => {
                let n = self.amount(amount, ctx, targets);
                for p in Self::players_of(&self.resolve_defined(*who, targets, ctx)) {
                    self.gain_life(p, n);
                }
            }
            Effect::LoseLife { amount, who } => {
                let n = self.amount(amount, ctx, targets);
                for p in Self::players_of(&self.resolve_defined(*who, targets, ctx)) {
                    self.lose_life(p, n);
                }
            }
            Effect::Destroy { what, no_regen } => {
                let ts = self.resolve_defined(*what, targets, ctx);
                // CR 701.8: gleichzeitig zerstören
                for c in self.cards_of(&ts) {
                    self.destroy_ex(c, !no_regen);
                }
            }
            Effect::Pump {
                what,
                power,
                toughness,
                keywords,
                double,
            } => {
                let p = self.amount(power, ctx, targets);
                let t = self.amount(toughness, ctx, targets);
                let ts = self.resolve_defined(*what, targets, ctx);
                let no_untap = keywords.contains(&crate::keyword::Keyword::NoUntapNext);
                let keywords: Vec<crate::keyword::Keyword> = keywords
                    .iter()
                    .filter(|k| **k != crate::keyword::Keyword::NoUntapNext)
                    .cloned()
                    .collect();
                for c in self.cards_of(&ts) {
                    if no_untap {
                        self.card_mut(c).skip_next_untap = true;
                    }
                    let ch = self.chars(c);
                    let dp = if double.0 { ch.power.unwrap_or(0) } else { 0 };
                    let dt = if double.1 {
                        ch.toughness.unwrap_or(0)
                    } else {
                        0
                    };
                    let timestamp = self.timestamp();
                    self.effects.push(ContinuousEffect {
                        affected: c,
                        power: p + dp,
                        toughness: t + dt,
                        keywords: keywords.clone(),
                        duration: Duration::UntilEndOfTurn,
                        timestamp,
                        ..Default::default()
                    });
                }
            }
            Effect::AddMana { produce, amount } => {
                let snow = self
                    .chars(ctx.source)
                    .types
                    .has_super(forge_types::Supertype::Snow);
                let n = self.amount(amount, ctx, targets).max(0);
                for _ in 0..n {
                    match produce {
                        ManaProduce::Fixed(syms) => {
                            for s in syms {
                                self.player_mut(ctx.controller).mana_pool.add(*s, snow);
                            }
                        }
                        ManaProduce::ChosenColor => {
                            if let Some(col) = self.card(ctx.source).chosen_color {
                                self.player_mut(ctx.controller)
                                    .mana_pool
                                    .add(Some(col), snow);
                            }
                        }
                        ManaProduce::OneOf(cols) => {
                            let col = if cols.len() == 1 {
                                cols[0]
                            } else {
                                match ch.ask(Decision::ChooseColor {
                                    player: ctx.controller,
                                    options: cols.clone(),
                                })? {
                                    Choice::Color(c) => c,
                                    _ => cols[0],
                                }
                            };
                            self.player_mut(ctx.controller)
                                .mana_pool
                                .add(Some(col), snow);
                        }
                    }
                }
            }
            Effect::Tap { what } => {
                let ts = self.resolve_defined(*what, targets, ctx);
                for c in self.cards_of(&ts) {
                    self.tap(c);
                }
            }
            Effect::Untap { what } => {
                let ts = self.resolve_defined(*what, targets, ctx);
                for c in self.cards_of(&ts) {
                    self.card_mut(c).tapped = false;
                }
            }
            Effect::PutCounter {
                what,
                counter,
                amount,
                mode,
            } => {
                use crate::effects::CounterMode;
                let n = self.amount(amount, ctx, targets).max(0) as u32;
                let cards = if *mode == CounterMode::Bolster {
                    // CR 701.39a: bei Gleichstand wählt der Spieler
                    let mine: Vec<CardId> = self
                        .battlefield_of(ctx.controller)
                        .filter(|&c| {
                            self.chars(c)
                                .types
                                .has_core(forge_types::CoreType::Creature)
                        })
                        .collect();
                    let least = mine.iter().filter_map(|&c| self.chars(c).toughness).min();
                    let options: Vec<CardId> = mine
                        .into_iter()
                        .filter(|&c| self.chars(c).toughness == least)
                        .collect();
                    if options.len() > 1 {
                        match ch.ask(Decision::ChooseCards {
                            player: ctx.controller,
                            reason: crate::decision::CardChoiceReason::Choose,
                            options: options.clone(),
                            min: 1,
                            max: 1,
                        })? {
                            Choice::Cards(v) if v.len() == 1 && options.contains(&v[0]) => v,
                            _ => options[..1].to_vec(),
                        }
                    } else {
                        options
                    }
                } else {
                    let ts = self.resolve_defined(*what, targets, ctx);
                    self.cards_of(&ts)
                };
                for c in cards {
                    match mode {
                        CounterMode::Adapt if self.card(c).counter("P1P1") > 0 => continue,
                        CounterMode::Monstrosity if self.card(c).monstrous => continue,
                        _ => {}
                    }
                    let n = self
                        .current_division
                        .iter()
                        .find(|(x, _)| *x == Target::Card(c))
                        .map_or(n, |(_, v)| *v);
                    self.add_counters(c, counter, n);
                    if *mode == CounterMode::Monstrosity {
                        self.card_mut(c).monstrous = true;
                    }
                }
            }
            Effect::BecomeMonarch { who } => {
                if let Some(p) = Self::players_of(&self.resolve_defined(*who, targets, ctx))
                    .into_iter()
                    .find(|p| self.player(*p).is_in_game())
                {
                    self.monarch = Some(p);
                }
            }
            Effect::ChooseSource { choices, remember } => {
                let mut options: Vec<CardId> = self
                    .battlefield
                    .iter()
                    .chain(self.command.iter())
                    .copied()
                    .collect();
                for item in &self.stack {
                    if !options.contains(&item.source) {
                        options.push(item.source);
                    }
                }
                options.retain(|c| {
                    self.card(*c).zone.is_some()
                        && self.card_matches(*c, choices, ctx.source, ctx.controller)
                });
                if options.is_empty() {
                    return Ok(());
                }
                let chosen = match ch.ask(Decision::ChooseCards {
                    player: ctx.controller,
                    reason: crate::decision::CardChoiceReason::Choose,
                    options: options.clone(),
                    min: 1,
                    max: 1,
                })? {
                    Choice::Cards(v) => v.into_iter().find(|c| options.contains(c)),
                    _ => None,
                }
                .unwrap_or(options[0]);
                self.card_mut(ctx.source).chosen_cards = vec![chosen];
                if *remember {
                    self.card_mut(ctx.source).remembered.push(chosen);
                }
            }
            Effect::GenericChoice {
                who,
                choices,
                at_random,
                remember_chooser,
            } => {
                for p in Self::players_of(&self.resolve_defined(*who, targets, ctx)) {
                    if !self.player(p).is_in_game() || choices.is_empty() {
                        continue;
                    }
                    let i = if *at_random {
                        self.rng.below(choices.len() as u64) as usize
                    } else {
                        let modes: Vec<(usize, String)> = choices
                            .iter()
                            .enumerate()
                            .map(|(i, a)| (i, a.description.clone()))
                            .collect();
                        match ch.ask(Decision::ChooseModes {
                            player: p,
                            modes,
                            min: 1,
                            max: 1,
                            can_repeat: false,
                        })? {
                            Choice::Modes(v) if v.len() == 1 && v[0] < choices.len() => v[0],
                            _ => 0,
                        }
                    };
                    if *remember_chooser {
                        self.card_mut(ctx.source).remembered_players.push(p);
                    }
                    self.apply_chain_inherit(&choices[i], targets, ctx, ch)?;
                    if *remember_chooser {
                        let rp = &mut self.card_mut(ctx.source).remembered_players;
                        if let Some(pos) = rp.iter().rposition(|x| *x == p) {
                            rp.remove(pos);
                        }
                    }
                }
            }
            Effect::DigUntil {
                players,
                valid,
                amount,
                max_revealed,
                found_dest,
                revealed_dest,
                random_order,
                shuffle,
                optional,
                remember_found,
                remember_revealed,
                tapped,
                gain_control,
            } => {
                let until = self.amount(amount, ctx, targets).max(0) as usize;
                if until == 0 {
                    return Ok(());
                }
                let max = max_revealed
                    .as_ref()
                    .map(|a| self.amount(a, ctx, targets).max(0) as usize);
                for p in Self::players_of(&self.resolve_defined(*players, targets, ctx)) {
                    if !self.player(p).is_in_game() {
                        continue;
                    }
                    if *optional
                        && !matches!(
                            ch.ask(Decision::Confirm {
                                player: p,
                                prompt: "Bibliothek aufdecken?".into(),
                            })?,
                            Choice::Yes
                        )
                    {
                        continue;
                    }
                    // oberste Karte = letztes Element
                    let lib: Vec<CardId> = self.player(p).library.iter().rev().copied().collect();
                    let limit = max.map_or(lib.len(), |m| m.min(lib.len()));
                    let mut revealed = Vec::new();
                    let mut found = Vec::new();
                    for &c in &lib[..limit] {
                        revealed.push(c);
                        if self.card_matches(c, valid, ctx.source, ctx.controller) {
                            found.push(c);
                            if *remember_found {
                                self.card_mut(ctx.source).remembered.push(c);
                            }
                            if found.len() == until {
                                break;
                            }
                        }
                    }
                    self.push_event(Event::Revealed {
                        player: p,
                        cards: revealed.clone(),
                    });
                    // Java: gleiche Zielzone → alle aufgedeckten Karten der Reihe nach bewegen
                    let sequential = found_dest.is_some()
                        && found_dest.map(|d| d.0) == revealed_dest.map(|d| d.0);
                    if let Some((zone, top)) = found_dest {
                        let movers = if sequential {
                            revealed.clone()
                        } else {
                            found.clone()
                        };
                        for c in movers {
                            if *zone == Zone::Battlefield {
                                let ctrl = gain_control.then_some(ctx.controller);
                                let new = self.move_card_as(c, Zone::Battlefield, true, ctrl);
                                if *tapped {
                                    self.card_mut(new).tapped = true;
                                }
                            } else {
                                self.move_card(c, *zone, *top);
                            }
                        }
                        revealed.retain(|c| !found.contains(c));
                    }
                    if *remember_revealed {
                        self.card_mut(ctx.source)
                            .remembered
                            .extend(revealed.iter().copied());
                    }
                    if *random_order {
                        self.rng.shuffle(&mut revealed);
                    }
                    if !sequential && let Some((zone, top)) = revealed_dest {
                        for c in revealed {
                            if self.card(c).zone == Some(Zone::Library) || *zone != Zone::Library {
                                self.move_card(c, *zone, *top);
                            }
                        }
                    }
                    if *shuffle {
                        self.shuffle_library(p);
                    }
                }
            }
            Effect::CopySpellAbility {
                triggered_sa,
                amount,
                may_choose_new_targets,
                controller,
            } => {
                let Some(p) = Self::players_of(&self.resolve_defined(*controller, targets, ctx))
                    .into_iter()
                    .next()
                else {
                    return Ok(());
                };
                let originals: Vec<StackItem> = if *triggered_sa {
                    let id = ctx.trig.as_ref().and_then(|t| t.stack_item);
                    self.stack
                        .iter()
                        .filter(|s| Some(s.id) == id)
                        .cloned()
                        .collect()
                } else {
                    Self::cards_of_any(targets)
                        .into_iter()
                        .filter_map(|card| {
                            self.stack
                                .iter()
                                .find(
                                    |s| matches!(s.kind, StackKind::Spell { card: c } if c == card),
                                )
                                .cloned()
                        })
                        .collect()
                };
                let n = self.amount(amount, ctx, targets).max(0);
                for orig in originals {
                    for _ in 0..n {
                        self.copy_stack_item(&orig, p, *may_choose_new_targets, ch)?;
                    }
                }
            }
            Effect::Play {
                defined,
                valid,
                zones,
                valid_sa,
                amount,
                optional,
                without_mana_cost,
                controller,
                exile_instead_of_graveyard,
                remember_played,
            } => {
                let Some(p) = Self::players_of(&self.resolve_defined(*controller, targets, ctx))
                    .into_iter()
                    .next()
                else {
                    return Ok(());
                };
                let mut pool: Vec<CardId> = match defined {
                    Some(d) => Self::cards_of_any(&self.resolve_defined(*d, targets, ctx)),
                    None => {
                        let all = self.players_from(self.active);
                        self.cards_in_zones(zones, &all)
                            .into_iter()
                            .filter(|c| {
                                valid.as_ref().is_none_or(|v| {
                                    self.card_matches(*c, v, ctx.source, ctx.controller)
                                })
                            })
                            .collect()
                    }
                };
                pool.retain(|c| self.card(*c).zone.is_some());
                let mut n = match amount {
                    Some(a) => self.amount(a, ctx, targets).max(0) as u32,
                    None => pool.len() as u32,
                };
                while n > 0 {
                    let options: Vec<CardId> = pool
                        .iter()
                        .copied()
                        .filter(|&c| self.playable_by_effect(p, c, valid_sa.as_ref(), ctx.source))
                        .collect();
                    if options.is_empty() {
                        break;
                    }
                    let chosen = if !*optional && options.len() == 1 {
                        Some(options[0])
                    } else {
                        match ch.ask(Decision::ChooseCards {
                            player: p,
                            reason: crate::decision::CardChoiceReason::Play,
                            options: options.clone(),
                            min: u32::from(!*optional),
                            max: 1,
                        })? {
                            Choice::Cards(v) => v.into_iter().find(|c| options.contains(c)),
                            _ => None,
                        }
                    };
                    let Some(card) = chosen.or_else(|| (!*optional).then(|| options[0])) else {
                        break;
                    };
                    pool.retain(|c| *c != card);
                    n -= 1;
                    if let Some(new) = self.play_by_effect(
                        p,
                        card,
                        *without_mana_cost,
                        *exile_instead_of_graveyard,
                        ch,
                    )? && *remember_played
                    {
                        self.card_mut(ctx.source).remembered.push(new);
                    }
                }
            }
            Effect::RemoveCounter {
                what,
                kind,
                amount,
                up_to,
                optional,
            } => {
                use crate::effects::CounterKind;
                if *optional
                    && !matches!(
                        ch.ask(Decision::Confirm {
                            player: ctx.controller,
                            prompt: "Marken entfernen?".into(),
                        })?,
                        Choice::Yes
                    )
                {
                    return Ok(());
                }
                let n = amount
                    .as_ref()
                    .map(|a| self.amount(a, ctx, targets).max(0) as u32);
                let ts = self.resolve_defined(*what, targets, ctx);
                for c in self.cards_of(&ts) {
                    if self.card(c).zone.is_none() {
                        continue;
                    }
                    match kind {
                        CounterKind::All => {
                            let kinds: Vec<String> =
                                self.card(c).counters.keys().cloned().collect();
                            for k in kinds {
                                self.remove_counters(c, &k, u32::MAX);
                            }
                        }
                        CounterKind::Named(k) => {
                            let mut m = n.unwrap_or(u32::MAX).min(self.card(c).counter(k));
                            if *up_to && m > 0 {
                                m = match ch.ask(Decision::ChooseX {
                                    player: ctx.controller,
                                    max: m,
                                })? {
                                    Choice::Number(x) => x.min(m),
                                    _ => m,
                                };
                            }
                            self.remove_counters(c, k, m);
                        }
                        CounterKind::Any => {
                            // Java `removeAnyType`: je Marke die Art wählen
                            for _ in 0..n.unwrap_or(u32::MAX) {
                                let options: Vec<String> = self
                                    .card(c)
                                    .counters
                                    .iter()
                                    .filter(|(_, v)| **v > 0)
                                    .map(|(k, _)| k.clone())
                                    .collect();
                                let k = match options.len() {
                                    0 => break,
                                    1 => options[0].clone(),
                                    _ => match ch.ask(Decision::ChooseType {
                                        player: ctx.controller,
                                        options: options.clone(),
                                    })? {
                                        Choice::Number(i) if (i as usize) < options.len() => {
                                            options[i as usize].clone()
                                        }
                                        _ => options[0].clone(),
                                    },
                                };
                                self.remove_counters(c, &k, 1);
                            }
                        }
                    }
                }
            }
            Effect::Token {
                scripts,
                amount,
                owner,
                tapped,
                power,
                toughness,
            } => {
                let n = self.amount(amount, ctx, targets).max(0) as u32;
                let p_override = power.as_ref().map(|a| self.amount(a, ctx, targets));
                let t_override = toughness.as_ref().map(|a| self.amount(a, ctx, targets));
                let owners = Self::players_of(&self.resolve_defined(*owner, targets, ctx));
                for p in owners {
                    for _ in 0..n {
                        for s in scripts {
                            let Some(def) = self.token_defs.get(s).cloned() else {
                                continue;
                            };
                            let id = self.create_card(def, p, Zone::Battlefield, true);
                            let face = self.card(id).face().clone();
                            let o = self.card_mut(id);
                            o.tapped = *tapped;
                            if p_override.is_some() || t_override.is_some() {
                                o.base_pt = Some((
                                    p_override.unwrap_or(
                                        face.power.as_ref().map(|x| x.base()).unwrap_or(0),
                                    ),
                                    t_override.unwrap_or(
                                        face.toughness.as_ref().map(|x| x.base()).unwrap_or(0),
                                    ),
                                ));
                            }
                            // CR 111.1: Token kommt ins Spiel → ETB-Trigger
                            self.push_event(crate::state::Event::ZoneChange {
                                old: id,
                                new: id,
                                from: None,
                                to: Zone::Battlefield,
                            });
                            self.fire(crate::triggers::TriggerEvent::ChangesZone {
                                old: id,
                                new: id,
                                from: None,
                                to: Zone::Battlefield,
                            });
                        }
                    }
                }
            }
            Effect::PumpAll {
                valid,
                power,
                toughness,
                keywords,
            } => {
                let p = self.amount(power, ctx, targets);
                let t = self.amount(toughness, ctx, targets);
                let affected: Vec<CardId> = self
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|c| self.card_matches(*c, valid, ctx.source, ctx.controller))
                    .collect();
                for c in affected {
                    let timestamp = self.timestamp();
                    self.effects.push(ContinuousEffect {
                        affected: c,
                        power: p,
                        toughness: t,
                        keywords: keywords.clone(),
                        duration: Duration::UntilEndOfTurn,
                        timestamp,
                        ..Default::default()
                    });
                }
            }
            Effect::ForAll {
                valid,
                players,
                inner,
            } => {
                let owners =
                    players.map(|d| Self::players_of(&self.resolve_defined(d, targets, ctx)));
                let cards: Vec<Target> = self
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|c| {
                        owners
                            .as_ref()
                            .is_none_or(|ps| ps.contains(&self.card(*c).controller))
                    })
                    .filter(|c| self.card_matches(*c, valid, ctx.source, ctx.controller))
                    .map(Target::Card)
                    .collect();
                self.apply_effect_inner(inner, &cards, ctx, ch)?;
            }
            Effect::SacrificeAll {
                what,
                valid,
                controller,
                remember,
            } => {
                let mut list: Vec<CardId> = match (what, valid) {
                    (Some(d), _) => self.cards_of(&self.resolve_defined(*d, targets, ctx)),
                    (None, Some(v)) => self
                        .battlefield
                        .iter()
                        .copied()
                        .filter(|c| self.card_matches(*c, v, ctx.source, ctx.controller))
                        .collect(),
                    (None, None) => Vec::new(),
                };
                if let Some(d) = controller {
                    let ps = Self::players_of(&self.resolve_defined(*d, targets, ctx));
                    list.retain(|c| ps.contains(&self.card(*c).controller));
                }
                if *remember {
                    self.card_mut(ctx.source).remembered.clear();
                }
                let outer_lki = self.lki_battlefield.is_some();
                if !outer_lki {
                    self.lki_battlefield = Some(self.battlefield.clone());
                }
                for c in list {
                    if self.sacrifice(c) && *remember {
                        self.card_mut(ctx.source).remembered.push(c);
                    }
                }
                if !outer_lki {
                    self.lki_battlefield = None;
                }
            }
            Effect::Fog => self.this_turn.prevent_combat_damage = true,
            Effect::Proliferate { times } => {
                for _ in 0..self.amount(times, ctx, targets).max(0) {
                    let options: Vec<CardId> = self
                        .battlefield
                        .iter()
                        .copied()
                        .filter(|c| self.card(*c).counters.values().any(|n| *n > 0))
                        .collect();
                    let cards = if options.is_empty() {
                        Vec::new()
                    } else {
                        let max = options.len() as u32;
                        match ch.ask(Decision::ChooseCards {
                            player: ctx.controller,
                            reason: crate::decision::CardChoiceReason::Choose,
                            options,
                            min: 0,
                            max,
                        })? {
                            Choice::Cards(v) => v,
                            _ => Vec::new(),
                        }
                    };
                    let mut players = Vec::new();
                    for p in self.apnap() {
                        if self.player(p).poison > 0
                            && matches!(
                                ch.ask(Decision::Confirm {
                                    player: ctx.controller,
                                    prompt: format!("Giftmarke für Spieler {}?", p.0 + 1),
                                })?,
                                Choice::Yes
                            )
                        {
                            players.push(p);
                        }
                    }
                    // CR 701.27a: gleichzeitig, je Markenart eine
                    for c in cards {
                        let kinds: Vec<String> = self
                            .card(c)
                            .counters
                            .iter()
                            .filter(|(_, n)| **n > 0)
                            .map(|(k, _)| k.clone())
                            .collect();
                        for k in kinds {
                            self.add_counters(c, &k, 1);
                        }
                    }
                    for p in players {
                        self.player_mut(p).poison += 1;
                    }
                }
            }
            Effect::DestroyAll { valid, no_regen } => {
                let affected: Vec<CardId> = self
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|c| self.card_matches(*c, valid, ctx.source, ctx.controller))
                    .collect();
                // CR 701.8 + 603.10a: gleichzeitig, Rückblick für „stirbt“-Trigger
                self.lki_battlefield = Some(self.battlefield.clone());
                for c in affected {
                    self.destroy_ex(c, !no_regen);
                }
                self.lki_battlefield = None;
            }
            Effect::DamageAll {
                amount,
                valid_cards,
                valid_players,
            } => {
                let n = self.amount(amount, ctx, targets).max(0) as u32;
                let mut recipients: Vec<Target> = Vec::new();
                if let Some(v) = valid_cards {
                    recipients.extend(
                        self.battlefield
                            .iter()
                            .copied()
                            .filter(|c| self.card_matches(*c, v, ctx.source, ctx.controller))
                            .map(Target::Card),
                    );
                }
                if let Some(v) = valid_players {
                    recipients.extend(
                        self.apnap()
                            .into_iter()
                            .filter(|p| self.player_matches(*p, v, ctx.controller))
                            .map(Target::Player),
                    );
                }
                self.begin_damage_batch();
                for r in recipients {
                    self.deal_damage(ctx.source, r, n);
                }
                self.end_damage_batch();
            }
            Effect::Discard {
                who,
                mode,
                num,
                valid,
                remember,
            } => {
                let n = self.amount(num, ctx, targets).max(0) as usize;
                for p in Self::players_of(&self.resolve_defined(*who, targets, ctx)) {
                    let hand: Vec<CardId> = self
                        .player(p)
                        .hand
                        .iter()
                        .copied()
                        .filter(|c| {
                            valid.as_ref().is_none_or(|v| {
                                self.card_matches(*c, v, ctx.source, ctx.controller)
                            })
                        })
                        .collect();
                    let chosen: Vec<CardId> = match mode {
                        DiscardMode::Hand => self.player(p).hand.clone(),
                        DiscardMode::Random => {
                            let mut h = hand.clone();
                            self.rng.shuffle(&mut h);
                            h.into_iter().take(n).collect()
                        }
                        DiscardMode::PlayerChooses | DiscardMode::YouChoose => {
                            let k = n.min(hand.len()) as u32;
                            if k == 0 {
                                Vec::new()
                            } else {
                                let chooser = if *mode == DiscardMode::YouChoose {
                                    ctx.controller
                                } else {
                                    p
                                };
                                match ch.ask(Decision::ChooseCards {
                                    player: chooser,
                                    reason: crate::decision::CardChoiceReason::Discard,
                                    options: hand,
                                    min: k,
                                    max: k,
                                })? {
                                    Choice::Cards(c) => c,
                                    _ => Vec::new(),
                                }
                            }
                        }
                    };
                    for c in chosen {
                        let new = self.discard(c);
                        if *remember {
                            self.card_mut(ctx.source).remembered.push(new);
                        }
                    }
                }
            }
            Effect::Counter { triggered_sa: true } => {
                // CR 701.5a: Zauber → Friedhof, Fähigkeit → entfernt
                let id = ctx.trig.as_ref().and_then(|t| t.stack_item);
                if let Some(pos) = self.stack.iter().position(|s| Some(s.id) == id)
                    && !matches!(self.stack[pos].kind, StackKind::Spell { card } if self.cant_be_countered(card))
                {
                    let item = self.stack.remove(pos);
                    if let StackKind::Spell { card } = item.kind {
                        self.move_card(card, Zone::Graveyard, true);
                    }
                }
            }
            Effect::Counter {
                triggered_sa: false,
            } => {
                for t in targets {
                    let Target::Card(card) = *t else { continue };
                    let Some(pos) = self
                        .stack
                        .iter()
                        .position(|s| matches!(s.kind, StackKind::Spell { card: c } if c == card))
                    else {
                        continue;
                    };
                    if self.cant_be_countered(card) {
                        continue;
                    }
                    // CR 701.5a: vom Stapel in den Friedhof des Besitzers
                    self.stack.remove(pos);
                    self.move_card(card, Zone::Graveyard, true);
                }
            }
            Effect::RepeatEach {
                over,
                sub,
                starting_with_you,
                clear_remembered_before,
                damage_map,
            } => {
                let cards: Option<Vec<CardId>> = match over {
                    super::RepeatOver::Players(_) => None,
                    super::RepeatOver::Cards(v, zones) => {
                        let all = self.players_from(self.active);
                        Some(
                            self.cards_in_zones(zones, &all)
                                .into_iter()
                                .filter(|c| self.card_matches(*c, v, ctx.source, ctx.controller))
                                .collect(),
                        )
                    }
                    super::RepeatOver::DefinedCards(d) => {
                        Some(Self::cards_of_any(&self.resolve_defined(*d, targets, ctx)))
                    }
                };
                if let Some(cards) = cards {
                    if *clear_remembered_before {
                        let o = self.card_mut(ctx.source);
                        o.remembered.clear();
                        o.remembered_players.clear();
                    }
                    if *damage_map {
                        self.begin_damage_batch();
                    }
                    // Java: Karte merken, Unterfähigkeit ausführen, wieder vergessen.
                    for c in cards {
                        self.card_mut(ctx.source).remembered.push(c);
                        let r = self.apply_chain_inherit(sub, targets, ctx, ch);
                        let o = self.card_mut(ctx.source);
                        if let Some(i) = o.remembered.iter().rposition(|x| *x == c) {
                            o.remembered.remove(i);
                        }
                        r?;
                    }
                    if *damage_map {
                        self.end_damage_batch();
                    }
                    return Ok(());
                }
                let super::RepeatOver::Players(players) = over else {
                    unreachable!()
                };
                let mut ps = Self::players_of(&self.resolve_defined(*players, targets, ctx));
                if *starting_with_you {
                    let order = self.players_from(ctx.controller);
                    ps.sort_by_key(|p| order.iter().position(|o| o == p));
                }
                if *clear_remembered_before {
                    let o = self.card_mut(ctx.source);
                    o.remembered.clear();
                    o.remembered_players.clear();
                }
                if *damage_map {
                    self.begin_damage_batch();
                }
                for p in ps {
                    // Java RepeatEachEffect: andere gemerkte Spieler vorübergehend entfernen.
                    let saved = std::mem::take(&mut self.card_mut(ctx.source).remembered_players);
                    self.card_mut(ctx.source).remembered_players = vec![p];
                    let r = self.apply_chain_inherit(sub, targets, ctx, ch);
                    let o = self.card_mut(ctx.source);
                    o.remembered_players.retain(|x| *x != p);
                    o.remembered_players.extend(saved);
                    r?;
                }
                if *damage_map {
                    self.end_damage_batch();
                }
            }
            Effect::Fight { defined } => {
                // Java: erster Kämpfer = erstes Ziel; Defined$ liefert den ersten
                // (bzw. beide, wenn es keine Ziele gibt).
                let tgts = Self::cards_of_any(targets);
                let mut f1 = tgts.first().copied();
                let mut f2 = None;
                if let Some(d) = defined {
                    let def: Vec<CardId> =
                        Self::cards_of_any(&self.resolve_defined(*d, targets, ctx))
                            .into_iter()
                            .filter(|c| {
                                self.card(*c).zone == Some(Zone::Battlefield)
                                    && self.is_creature(*c)
                            })
                            .collect();
                    if def.len() > 1 && f1.is_none() {
                        f1 = Some(def[0]);
                        f2 = Some(def[1]);
                    } else if let Some(first) = def.first() {
                        f2 = f1;
                        f1 = Some(*first);
                    }
                } else if tgts.len() > 1 {
                    f2 = Some(tgts[1]);
                }
                // CR 701.14b: beide müssen noch Kreaturen im Spiel sein
                if let (Some(a), Some(b)) = (f1, f2)
                    && a != b
                    && [a, b].iter().all(|c| {
                        self.card(*c).zone == Some(Zone::Battlefield) && self.is_creature(*c)
                    })
                {
                    let pa = self.chars(a).power.unwrap_or(0).max(0) as u32;
                    let pb = self.chars(b).power.unwrap_or(0).max(0) as u32;
                    self.begin_damage_batch();
                    self.deal_damage(a, Target::Card(b), pa);
                    self.deal_damage(b, Target::Card(a), pb);
                    self.end_damage_batch();
                }
            }
            Effect::PutCounterAll {
                valid,
                counter,
                amount,
            } => {
                let n = self.amount(amount, ctx, targets).max(0) as u32;
                let cards: Vec<CardId> = self
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|c| self.card_matches(*c, valid, ctx.source, ctx.controller))
                    .collect();
                for c in cards {
                    self.add_counters(c, counter, n);
                }
            }
            Effect::GainControl {
                what,
                new_controller,
                until_eot,
                until_end_of_next_turn,
                leaves_play,
                lose_control,
                untap,
                keywords,
            } => {
                let Some(new) =
                    Self::players_of(&self.resolve_defined(*new_controller, targets, ctx))
                        .into_iter()
                        .next()
                else {
                    return Ok(());
                };
                let source_controller = self.card(ctx.source).controller;
                for c in self.cards_of(&self.resolve_defined(*what, targets, ctx)) {
                    let timestamp = self.timestamp();
                    self.control_effects.push(crate::state::ControlEffect {
                        timestamp,
                        card: c,
                        controller: new,
                        source: ctx.source,
                        source_controller,
                        created_turn: self.turn,
                        until_eot: *until_eot,
                        until_end_of_next_turn: *until_end_of_next_turn,
                        leaves_play: *leaves_play,
                        lose_control: *lose_control,
                    });
                    self.apply_control(c);
                    if *untap {
                        self.card_mut(c).tapped = false;
                    }
                    if !keywords.is_empty() {
                        let timestamp = self.timestamp();
                        self.effects.push(ContinuousEffect {
                            affected: c,
                            keywords: keywords.clone(),
                            duration: Duration::UntilEndOfTurn,
                            timestamp,
                            ..Default::default()
                        });
                    }
                }
            }
            Effect::PreventDamage { what, amount } => {
                let n = self.amount(amount, ctx, targets).max(0) as u32;
                for t in self.resolve_defined(*what, targets, ctx) {
                    match t {
                        Target::Player(p) => self.player_mut(p).prevent_next += n,
                        Target::Card(c) if self.card(c).zone == Some(Zone::Battlefield) => {
                            self.card_mut(c).prevent_next += n;
                        }
                        Target::Card(_) => {}
                    }
                }
            }
            Effect::DelayedTrigger {
                trigger,
                remember,
                scope,
            } => {
                let remembered = match remember {
                    Some(d) => self.resolve_defined(*d, targets, ctx),
                    None => Vec::new(),
                };
                let (from_turn, until_turn) = match scope {
                    super::DelayScope::Until => (self.turn, None),
                    super::DelayScope::ThisTurn => (self.turn, Some(self.turn)),
                    super::DelayScope::NextTurn => (self.turn + 1, Some(self.turn + 1)),
                };
                self.delayed_triggers.push(crate::triggers::DelayedTrigger {
                    trigger: trigger.clone(),
                    source: ctx.source,
                    controller: ctx.controller,
                    remembered,
                    from_turn,
                    until_turn,
                });
            }
            Effect::ImmediateTrigger {
                trigger,
                remember,
                amount,
            } => {
                let remembered = match remember {
                    Some(d) => self.resolve_defined(*d, targets, ctx),
                    None => Vec::new(),
                };
                let n = self.amount(amount, ctx, targets).max(0);
                for _ in 0..n {
                    let mut info = ctx.trig.clone().unwrap_or_default();
                    info.delay_remembered = remembered.clone();
                    self.pending_triggers.push(crate::triggers::PendingTrigger {
                        trigger: trigger.clone(),
                        source: ctx.source,
                        controller: ctx.controller,
                        info,
                    });
                }
            }
            Effect::Transform { what, optional } => {
                for c in self.cards_of(&self.resolve_defined(*what, targets, ctx)) {
                    if *optional
                        && !matches!(
                            ch.ask(Decision::ChooseCards {
                                player: ctx.controller,
                                reason: crate::decision::CardChoiceReason::Transform,
                                options: vec![c],
                                min: 0,
                                max: 1,
                            })?,
                            Choice::Cards(v) if !v.is_empty()
                        )
                    {
                        continue;
                    }
                    self.transform(c);
                }
            }
            Effect::Cleanup {
                what,
                clear_remembered,
                clear_chosen,
            } => {
                for t in self.resolve_defined(*what, targets, ctx) {
                    let Target::Card(c) = t else { continue };
                    let o = self.card_mut(c);
                    if *clear_remembered {
                        o.remembered.clear();
                        o.remembered_players.clear();
                    }
                    if clear_chosen[0] {
                        o.chosen_cards.clear();
                    }
                    if clear_chosen[1] {
                        o.chosen_player = None;
                    }
                    if clear_chosen[2] {
                        o.chosen_color = None;
                    }
                    if clear_chosen[3] {
                        o.chosen_type = None;
                    }
                    if clear_chosen[4] {
                        o.named_card = None;
                    }
                }
            }
            Effect::ChoosePlayer {
                who,
                choices,
                random,
                remember,
            } => {
                for chooser in Self::players_of(&self.resolve_defined(*who, targets, ctx)) {
                    let options: Vec<PlayerId> = self
                        .players_from(self.active)
                        .into_iter()
                        .filter(|p| {
                            self.player(*p).is_in_game()
                                && self.player_matches_src(*p, choices, chooser, ctx.source)
                        })
                        .collect();
                    if options.is_empty() {
                        continue;
                    }
                    let chosen = if *random || options.len() == 1 {
                        options[self.rng.below(options.len() as u64) as usize]
                    } else {
                        match ch.ask(Decision::ChooseTargets {
                            player: chooser,
                            source: ctx.source,
                            description: "Spieler wählen".into(),
                            candidates: options.iter().map(|p| Target::Player(*p)).collect(),
                            min: 1,
                            max: 1,
                        })? {
                            Choice::Targets(v) => v
                                .iter()
                                .find_map(|t| match t {
                                    Target::Player(p) if options.contains(p) => Some(*p),
                                    _ => None,
                                })
                                .unwrap_or(options[0]),
                            _ => options[0],
                        }
                    };
                    let o = self.card_mut(ctx.source);
                    o.chosen_player = Some(chosen);
                    if *remember {
                        o.remembered_players.push(chosen);
                    }
                }
            }
            Effect::ManaReflectProduced { who } => {
                let mut kinds: Vec<Option<forge_types::Color>> = ctx
                    .trig
                    .as_ref()
                    .map(|t| t.produced.clone())
                    .unwrap_or_default();
                kinds.sort_by_key(|k| k.map(|c| c as u8));
                kinds.dedup();
                let Some(&first) = kinds.first() else {
                    return Ok(());
                };
                let colors: Vec<forge_types::Color> = kinds.iter().filter_map(|k| *k).collect();
                let pick = if kinds.len() > 1 && !colors.is_empty() {
                    match ch.ask(Decision::ChooseColor {
                        player: ctx.controller,
                        options: colors.clone(),
                    })? {
                        Choice::Color(c) if colors.contains(&c) => Some(c),
                        _ => first,
                    }
                } else {
                    first
                };
                for p in Self::players_of(&self.resolve_defined(*who, targets, ctx)) {
                    self.player_mut(p).mana_pool.add(pick, false);
                }
            }
            Effect::NameCard { who, valid, random } => {
                for p in Self::players_of(&self.resolve_defined(*who, targets, ctx)) {
                    // Namen aller Karten im Spiel (Abweichung: Java bietet den ganzen
                    // Kartenpool an), sortiert und ohne Dopplungen
                    let mut names: Vec<String> = self
                        .live_cards()
                        .filter(|c| {
                            let o = self.card(*c);
                            o.zone.is_some()
                                && !o.token
                                && o.effect.is_none()
                                && valid
                                    .as_ref()
                                    .is_none_or(|v| self.card_matches(*c, v, ctx.source, p))
                        })
                        .map(|c| self.card(c).def.face(0).name.clone())
                        .filter(|n| !n.is_empty())
                        .collect();
                    names.sort();
                    names.dedup();
                    if names.is_empty() {
                        continue;
                    }
                    let i = if *random {
                        self.rng.below(names.len() as u64) as usize
                    } else {
                        match ch.ask(Decision::NameCard {
                            player: p,
                            options: names.clone(),
                        })? {
                            Choice::Number(i) if (i as usize) < names.len() => i as usize,
                            _ => 0,
                        }
                    };
                    self.card_mut(ctx.source).named_card = Some(names[i].clone());
                }
            }
            Effect::Cascade => {
                let p = ctx.controller;
                let mv = self
                    .chars(ctx.source)
                    .mana_cost
                    .as_ref()
                    .map_or(0, |m| m.mana_value());
                let mut exiled: Vec<CardId> = Vec::new();
                let mut hit = None;
                while let Some(&top) = self.player(p).library.last() {
                    let ch_top = self.chars(top);
                    let new = self.move_card(top, Zone::Exile, true);
                    exiled.push(new);
                    let top_mv = ch_top.mana_cost.as_ref().map_or(0, |m| m.mana_value());
                    if !ch_top.types.is_land() && top_mv < mv {
                        hit = Some(new);
                        break;
                    }
                }
                self.push_event(Event::Revealed {
                    player: p,
                    cards: exiled.clone(),
                });
                if let Some(h) = hit
                    && self.playable_by_effect(p, h, None, ctx.source)
                    && matches!(
                        ch.ask(Decision::Confirm {
                            player: p,
                            prompt: "Kaskade: Karte gratis wirken?".into(),
                        })?,
                        Choice::Yes
                    )
                {
                    self.play_by_effect(p, h, true, false, ch)?;
                }
                // CR 702.85a: übrige (nicht gewirkte) in zufälliger Reihenfolge unter die Bibliothek
                let mut rest: Vec<CardId> = exiled
                    .into_iter()
                    .filter(|c| self.card(*c).zone == Some(Zone::Exile))
                    .collect();
                self.rng.shuffle(&mut rest);
                for c in rest {
                    self.move_card(c, Zone::Library, false);
                }
            }
            Effect::Amass { num, kind } => {
                let p = ctx.controller;
                let is_army = |g: &GameState, c: CardId| g.chars(c).types.has_subtype("Army");
                // CR 701.47a: ohne Army zuerst einen 0/0-Army-Spielstein erschaffen
                if !self.battlefield_of(p).any(|c| is_army(self, c)) {
                    let specific = format!("b_0_0_{}_army", kind.to_ascii_lowercase());
                    let script = if self.token_defs.contains_key(&specific) {
                        specific
                    } else {
                        "b_0_0_army".to_string()
                    };
                    self.apply_effect_inner(
                        &super::Effect::Token {
                            scripts: vec![script],
                            amount: super::Amount::Fixed(1),
                            owner: super::Defined::You,
                            tapped: false,
                            power: None,
                            toughness: None,
                        },
                        targets,
                        ctx,
                        ch,
                    )?;
                }
                let armies: Vec<CardId> = self
                    .battlefield_of(p)
                    .filter(|c| is_army(self, *c))
                    .collect();
                let Some(&first) = armies.first() else {
                    return Ok(());
                };
                let army = if armies.len() > 1 {
                    match ch.ask(Decision::ChooseCards {
                        player: p,
                        reason: crate::decision::CardChoiceReason::Choose,
                        options: armies.clone(),
                        min: 1,
                        max: 1,
                    })? {
                        Choice::Cards(v) => {
                            v.into_iter().find(|c| armies.contains(c)).unwrap_or(first)
                        }
                        _ => first,
                    }
                } else {
                    first
                };
                let n = self.amount(num, ctx, targets).max(0) as u32;
                self.add_counters(army, "P1P1", n);
                // CR 701.47a: wird zusätzlich vom Typ
                if !self.chars(army).types.has_subtype(kind) {
                    let timestamp = self.timestamp();
                    self.effects.push(ContinuousEffect {
                        affected: army,
                        add_types: vec![kind.clone()],
                        duration: Duration::Permanent,
                        timestamp,
                        ..Default::default()
                    });
                }
            }
            Effect::FlipCoin { win, lose } => {
                let won = self.rng.below(2) == 0;
                self.push_event(Event::CoinFlipped {
                    player: ctx.controller,
                    won,
                });
                if let Some(ab) = if won { win } else { lose } {
                    self.apply_chain_inherit(ab, targets, ctx, ch)?;
                }
            }
            Effect::ChooseNumber {
                who,
                min,
                max,
                random,
            } => {
                let lo = self.amount(min, ctx, targets).max(0) as u32;
                let hi = (self.amount(max, ctx, targets).max(0) as u32).max(lo);
                for p in Self::players_of(&self.resolve_defined(*who, targets, ctx)) {
                    let n = if *random {
                        lo + self.rng.below(u64::from(hi - lo + 1)) as u32
                    } else {
                        match ch.ask(Decision::ChooseX { player: p, max: hi })? {
                            Choice::Number(n) => n.clamp(lo, hi),
                            _ => lo,
                        }
                    };
                    self.card_mut(ctx.source)
                        .runtime_vars
                        .insert("ChosenNumber".into(), n as i32);
                }
            }
            Effect::Goad { what, permanent } => {
                let goader = ctx.controller;
                for c in self.cards_of(&self.resolve_defined(*what, targets, ctx)) {
                    let o = self.card_mut(c);
                    if !o.goaded_by.iter().any(|(g, _)| *g == goader) {
                        o.goaded_by.push((goader, *permanent));
                    }
                }
            }
            Effect::Reveal {
                who,
                defined,
                valid,
                all_valid,
                num,
                any_number,
                random,
                optional,
                remember,
            } => {
                for p in Self::players_of(&self.resolve_defined(*who, targets, ctx)) {
                    let cards: Vec<CardId> = if let Some(d) = defined {
                        Self::cards_of_any(&self.resolve_defined(*d, targets, ctx))
                    } else {
                        let hand: Vec<CardId> = self
                            .player(p)
                            .hand
                            .iter()
                            .copied()
                            .filter(|c| {
                                valid
                                    .as_ref()
                                    .is_none_or(|v| self.card_matches(*c, v, ctx.source, p))
                            })
                            .collect();
                        if *all_valid {
                            hand
                        } else if hand.is_empty() {
                            Vec::new()
                        } else if *random {
                            vec![hand[self.rng.below(hand.len() as u64) as usize]]
                        } else {
                            let n = (self.amount(num, ctx, targets).max(0) as u32)
                                .min(hand.len() as u32);
                            let (min, max) = if *any_number {
                                (0, hand.len() as u32)
                            } else if *optional {
                                (0, n)
                            } else {
                                (n, n)
                            };
                            match ch.ask(Decision::ChooseCards {
                                player: p,
                                reason: crate::decision::CardChoiceReason::Choose,
                                options: hand.clone(),
                                min,
                                max,
                            })? {
                                Choice::Cards(v) => {
                                    v.into_iter().filter(|c| hand.contains(c)).collect()
                                }
                                _ => Vec::new(),
                            }
                        }
                    };
                    if cards.is_empty() {
                        continue;
                    }
                    if *remember {
                        self.card_mut(ctx.source)
                            .remembered
                            .extend(cards.iter().copied());
                    }
                    self.push_event(Event::Revealed { player: p, cards });
                }
            }
            Effect::Debuff { what, keywords } => {
                for c in self.cards_of(&self.resolve_defined(*what, targets, ctx)) {
                    let timestamp = self.timestamp();
                    self.effects.push(ContinuousEffect {
                        affected: c,
                        remove_keywords: keywords.clone(),
                        duration: Duration::UntilEndOfTurn,
                        timestamp,
                        ..Default::default()
                    });
                }
            }
            Effect::Repeat {
                sub,
                max,
                present,
                check,
                optional,
            } => {
                let max = max
                    .as_ref()
                    .map(|a| self.amount(a, ctx, targets).max(0) as usize);
                if max == Some(0) {
                    return Ok(());
                }
                // Schutz vor Endlosschleifen ohne Bedingung
                let cap = max.unwrap_or(100).min(100);
                for count in 1..=cap {
                    self.apply_chain_inherit(sub, targets, ctx, ch)?;
                    if count >= cap || self.winner.is_some() {
                        break;
                    }
                    let ectx = ctx.eval_with(targets);
                    if let Some((spec, def, cmp)) = present {
                        let list: Vec<CardId> = match def {
                            Some(d) => Self::cards_of_any(&self.resolve_defined(*d, targets, ctx)),
                            None => self.battlefield.clone(),
                        };
                        let n = list
                            .iter()
                            .filter(|c| self.card_matches(**c, spec, ctx.source, ctx.controller))
                            .count() as i32;
                        if !cmp.holds(n, self.eval(&cmp.rhs, &ectx)) {
                            break;
                        }
                    }
                    if let Some((e, cmp)) = check
                        && !cmp.holds(self.eval(e, &ectx), self.eval(&cmp.rhs, &ectx))
                    {
                        break;
                    }
                    if *optional
                        && !matches!(
                            ch.ask(Decision::Confirm {
                                player: ctx.controller,
                                prompt: "Wiederholen?".into(),
                            })?,
                            Choice::Yes
                        )
                    {
                        break;
                    }
                }
            }
            Effect::RevealHand { who, remember } => {
                for p in Self::players_of(&self.resolve_defined(*who, targets, ctx)) {
                    let cards = self.player(p).hand.clone();
                    if *remember {
                        self.card_mut(ctx.source)
                            .remembered
                            .extend(cards.iter().copied());
                    }
                    self.push_event(Event::Revealed { player: p, cards });
                }
            }
            Effect::Protection {
                what,
                fixed,
                choose,
            } => {
                let mut kws = fixed.clone();
                if *choose {
                    let options: Vec<forge_types::Color> = forge_types::Color::ALL.to_vec();
                    let col = match ch.ask(Decision::ChooseColor {
                        player: ctx.controller,
                        options: options.clone(),
                    })? {
                        Choice::Color(c) if options.contains(&c) => c,
                        _ => options[0],
                    };
                    kws.push(crate::keyword::Keyword::Protection(
                        crate::keyword::ProtectionFrom::Color(col),
                    ));
                }
                for c in self.cards_of(&self.resolve_defined(*what, targets, ctx)) {
                    let timestamp = self.timestamp();
                    self.effects.push(ContinuousEffect {
                        affected: c,
                        keywords: kws.clone(),
                        duration: Duration::UntilEndOfTurn,
                        timestamp,
                        ..Default::default()
                    });
                }
            }
            Effect::RollDice {
                sides,
                amount,
                modifier,
                result_var,
                results,
            } => {
                let n = self.amount(amount, ctx, targets).max(0);
                let mut rolls = Vec::new();
                for _ in 0..n {
                    rolls.push(self.rng.below(u64::from((*sides).max(1))) as i32 + 1);
                }
                let m = modifier
                    .as_ref()
                    .map_or(0, |a| self.amount(a, ctx, targets));
                let total = (rolls.iter().sum::<i32>() + m).max(0);
                self.push_event(Event::DiceRolled {
                    player: ctx.controller,
                    sides: *sides,
                    rolls,
                    total,
                });
                if let Some(v) = result_var {
                    self.card_mut(ctx.source)
                        .runtime_vars
                        .insert(v.clone(), total);
                }
                let t = total as u32;
                if let Some((_, _, ab)) = results.iter().find(|(lo, hi, _)| *lo <= t && t <= *hi) {
                    self.apply_chain_inherit(ab, targets, ctx, ch)?;
                }
            }
            Effect::Branch {
                check,
                cmp,
                then,
                otherwise,
            } => {
                let ectx = ctx.eval_with(targets);
                let v = self.eval(check, &ectx);
                let rhs = self.eval(&cmp.rhs, &ectx);
                let branch = if cmp.holds(v, rhs) { then } else { otherwise };
                if let Some(ab) = branch {
                    self.apply_chain_inherit(ab, targets, ctx, ch)?;
                }
            }
            Effect::Sacrifice {
                who,
                valid,
                amount,
                optional,
                strict,
                destroy,
                remember,
            } => {
                let n = self.amount(amount, ctx, targets).max(0) as usize;
                // CR 101.4: erst wählen alle (APNAP-Reihenfolge der Defined-Liste),
                // dann wird gleichzeitig geopfert.
                let mut chosen: Vec<CardId> = Vec::new();
                // Java SacrificeEffect: `Defined$` mit Karten → genau diese opfern
                // (nur eigene bleibende Karten, CR 701.21a).
                let defined_cards: Vec<CardId> =
                    self.cards_of(&self.resolve_defined(*who, targets, ctx));
                match valid {
                    None if !defined_cards.is_empty() && !*optional => {
                        chosen.extend(defined_cards.into_iter().filter(|c| {
                            let o = self.card(*c);
                            o.zone == Some(Zone::Battlefield) && o.controller == ctx.controller
                        }));
                    }
                    None => {
                        // „Opfere ~“: nur wenn die Quelle noch unter deiner Kontrolle im Spiel ist.
                        let src = self.card(ctx.source);
                        if src.zone == Some(Zone::Battlefield) && src.controller == ctx.controller {
                            let take = !*optional
                                || matches!(
                                    ch.ask(Decision::ChooseCards {
                                        player: ctx.controller,
                                        reason: crate::decision::CardChoiceReason::Sacrifice,
                                        options: vec![ctx.source],
                                        min: 0,
                                        max: 1,
                                    })?,
                                    Choice::Cards(v) if !v.is_empty()
                                );
                            if take {
                                chosen.push(ctx.source);
                            }
                        }
                    }
                    Some(v) => {
                        for p in Self::players_of(&self.resolve_defined(*who, targets, ctx)) {
                            if self.player(p).lost.is_some() {
                                continue; // CR 800.4a
                            }
                            let options: Vec<CardId> = self
                                .battlefield_of(p)
                                .filter(|c| self.card_matches(*c, v, ctx.source, ctx.controller))
                                .collect();
                            if *strict && options.len() < n {
                                continue;
                            }
                            let max = n.min(options.len()) as u32;
                            let min = if *optional && !*strict { 0 } else { max };
                            if max == 0 {
                                continue;
                            }
                            if let Choice::Cards(cs) = ch.ask(Decision::ChooseCards {
                                player: p,
                                reason: crate::decision::CardChoiceReason::Sacrifice,
                                options,
                                min,
                                max,
                            })? {
                                chosen.extend(cs);
                            }
                        }
                    }
                }
                // Gleichzeitig, mit Rückblick für „stirbt“-Trigger (CR 603.10a).
                let outer_lki = self.lki_battlefield.is_some();
                if !outer_lki {
                    self.lki_battlefield = Some(self.battlefield.clone());
                }
                for c in chosen {
                    let done = if *destroy {
                        self.destroy_ex(c, true)
                    } else {
                        self.sacrifice(c)
                    };
                    if done && *remember {
                        self.card_mut(ctx.source).remembered.push(c);
                    }
                }
                if !outer_lki {
                    self.lki_battlefield = None;
                }
            }
            Effect::CreateEffect {
                def,
                duration,
                remember,
            } => {
                let id = self.create_card(def.0.clone(), ctx.controller, Zone::Command, false);
                let objs = match remember {
                    Some(d) => self.resolve_defined(*d, targets, ctx),
                    None => Vec::new(),
                };
                let turn = self.turn;
                // Java EffectEffect: Wahl der Quelle (Karte/Farbe/Typ) übernehmen
                let host = self.card(ctx.source).clone();
                let o = self.card_mut(id);
                o.chosen_cards = host.chosen_cards;
                o.chosen_color = host.chosen_color;
                o.chosen_type = host.chosen_type;
                o.remembered = Self::cards_of_any(&objs);
                o.remembered_players = Self::players_of(&objs);
                o.effect = Some(crate::state::EffectInfo {
                    duration: *duration,
                    created_turn: turn,
                    host: Some(ctx.source),
                });
            }
            Effect::AuraSpell => {}
            Effect::Surveil { who, num } => {
                let n = self.amount(num, ctx, targets).max(0) as usize;
                for p in Self::players_of(&self.resolve_defined(*who, targets, ctx)) {
                    let lib = &self.player(p).library;
                    let k = n.min(lib.len());
                    if k == 0 {
                        continue;
                    }
                    let cards: Vec<CardId> = lib.iter().rev().take(k).copied().collect();
                    let (top, grave) = match ch.ask(Decision::Scry {
                        player: p,
                        cards: cards.clone(),
                        graveyard: true,
                    })? {
                        Choice::Arrange { top, bottom } => (top, bottom),
                        _ => (cards.clone(), Vec::new()),
                    };
                    let lib = &mut self.player_mut(p).library;
                    lib.retain(|c| !cards.contains(c));
                    for c in top.iter().rev() {
                        lib.push(*c);
                    }
                    for c in grave {
                        self.move_card(c, Zone::Graveyard, true);
                    }
                }
            }
            Effect::Clone {
                source,
                cloner,
                optional,
                mods,
            } => {
                let Some(of) = self
                    .pick_clone_source(source, targets, ctx, ch)?
                    .first()
                    .copied()
                else {
                    return Ok(());
                };
                if *optional
                    && !matches!(
                        ch.ask(Decision::Confirm {
                            player: ctx.controller,
                            prompt: "Kopie werden?".into(),
                        })?,
                        Choice::Yes
                    )
                {
                    return Ok(());
                }
                for c in self.cards_of(&self.resolve_defined(*cloner, targets, ctx)) {
                    if self.card(c).zone == Some(Zone::Battlefield) && c != of {
                        self.become_copy(c, of, mods);
                    }
                }
            }
            Effect::CopyPermanent {
                source,
                num,
                controller,
                tapped,
                mods,
                remember,
                at_eot,
            } => {
                let originals = self.pick_clone_source(source, targets, ctx, ch)?;
                let n = self.amount(num, ctx, targets).max(0);
                let owners = Self::players_of(&self.resolve_defined(*controller, targets, ctx));
                let mut made: Vec<CardId> = Vec::new();
                for p in owners {
                    for &of in &originals {
                        for _ in 0..n {
                            let (def, face, base_pt, prev) = self.copiable_values(of);
                            let id = self.create_card(def.clone(), p, Zone::Battlefield, true);
                            {
                                let o = self.card_mut(id);
                                o.face = face;
                                o.base_pt = base_pt;
                                o.tapped = *tapped;
                                o.copy = Some(Box::new(crate::state::CopyInfo {
                                    orig_def: def,
                                    orig_face: face,
                                    orig_base_pt: base_pt,
                                    mods: prev.then(mods),
                                }));
                            }
                            made.push(id);
                            self.push_event(crate::state::Event::ZoneChange {
                                old: id,
                                new: id,
                                from: None,
                                to: Zone::Battlefield,
                            });
                            self.fire(crate::triggers::TriggerEvent::ChangesZone {
                                old: id,
                                new: id,
                                from: None,
                                to: Zone::Battlefield,
                            });
                        }
                    }
                }
                if *remember {
                    self.card_mut(ctx.source)
                        .remembered
                        .extend(made.iter().copied());
                }
                if let Some(t) = at_eot
                    && !made.is_empty()
                {
                    let turn = self.turn;
                    self.delayed_triggers.push(crate::triggers::DelayedTrigger {
                        trigger: t.clone(),
                        source: ctx.source,
                        controller: ctx.controller,
                        remembered: made.into_iter().map(Target::Card).collect(),
                        from_turn: turn,
                        until_turn: None,
                    });
                }
            }
            Effect::ChooseColor { who } => {
                for p in Self::players_of(&self.resolve_defined(*who, targets, ctx)) {
                    let col = match ch.ask(Decision::ChooseColor {
                        player: p,
                        options: forge_types::Color::ALL.to_vec(),
                    })? {
                        Choice::Color(c) => c,
                        _ => forge_types::Color::White,
                    };
                    self.card_mut(ctx.source).chosen_color = Some(col);
                }
            }
            Effect::ChooseType {
                who,
                kind,
                invalid,
                valid,
            } => {
                let mut options = self.type_choice_options(*kind);
                options.retain(|t| !invalid.contains(t));
                if let Some(v) = valid {
                    options.retain(|t| v.contains(t));
                }
                for p in Self::players_of(&self.resolve_defined(*who, targets, ctx)) {
                    if options.is_empty() {
                        break;
                    }
                    let i = match ch.ask(Decision::ChooseType {
                        player: p,
                        options: options.clone(),
                    })? {
                        Choice::Number(i) => (i as usize).min(options.len() - 1),
                        _ => 0,
                    };
                    self.card_mut(ctx.source).chosen_type = Some(options[i].clone());
                }
            }
            Effect::Shuffle { who } => {
                for p in Self::players_of(&self.resolve_defined(*who, targets, ctx)) {
                    self.shuffle_library(p);
                }
            }
            Effect::ChooseCard {
                who,
                zones,
                choices,
                defined_cards,
                amount,
                min,
                mandatory,
                controlled_by_chooser,
                target_controls,
                at_random,
                remember,
                forget,
            } => {
                // Java ChooseCardEffect
                let choosers = Self::players_of(&self.resolve_defined(*who, targets, ctx));
                let mut pool: Vec<CardId> = match defined_cards {
                    Some(d) => self.cards_of(&self.resolve_defined(*d, targets, ctx)),
                    None => {
                        let all = self.players_from(self.active);
                        self.cards_in_zones(zones, &all)
                            .into_iter()
                            .filter(|c| {
                                choices.as_ref().is_none_or(|v| {
                                    self.card_matches(*c, v, ctx.source, ctx.controller)
                                })
                            })
                            .collect()
                    }
                };
                if *target_controls {
                    pool.retain(|c| choosers.contains(&self.card(*c).controller));
                }
                let n = self.amount(amount, ctx, targets);
                let mut all_chosen: Vec<CardId> = Vec::new();
                if n > 0 {
                    for p in choosers {
                        let mut opts = pool.clone();
                        if *controlled_by_chooser {
                            opts.retain(|c| self.card(*c).controller == p);
                        }
                        let max = (n as usize).min(opts.len());
                        let chosen = if *at_random {
                            self.rng.shuffle(&mut opts);
                            opts.into_iter().take(max).collect()
                        } else if max == 0 {
                            Vec::new()
                        } else {
                            let want_min = min.unwrap_or(n as u32).min(max as u32);
                            match ch.ask(Decision::ChooseCards {
                                player: p,
                                reason: crate::decision::CardChoiceReason::Choose,
                                options: opts,
                                min: if *mandatory { want_min } else { 0 },
                                max: max as u32,
                            })? {
                                Choice::Cards(c) => c,
                                _ => Vec::new(),
                            }
                        };
                        all_chosen.extend(chosen);
                    }
                }
                let src = self.card_mut(ctx.source);
                src.chosen_cards = all_chosen.clone();
                if *remember {
                    src.remembered.extend(all_chosen.iter().copied());
                }
                if *forget {
                    src.remembered.retain(|c| !all_chosen.contains(c));
                }
            }
            Effect::PeekAndReveal {
                who,
                num,
                reveal_valid,
                no_reveal,
                reveal_optional,
                remember_revealed,
                remember_peeked,
            } => {
                // Java PeekAndRevealEffect: je Spieler die obersten N Karten.
                let n = self.amount(num, ctx, targets).max(0) as usize;
                for p in Self::players_of(&self.resolve_defined(*who, targets, ctx)) {
                    let top: Vec<CardId> = self
                        .player(p)
                        .library
                        .iter()
                        .rev()
                        .take(n)
                        .copied()
                        .collect();
                    let revealable: Vec<CardId> = top
                        .into_iter()
                        .filter(|c| {
                            reveal_valid.as_ref().is_none_or(|v| {
                                self.card_matches(*c, v, ctx.source, ctx.controller)
                            })
                        })
                        .collect();
                    let mut reveal = !*no_reveal && !revealable.is_empty();
                    if reveal && *reveal_optional {
                        reveal = matches!(
                            ch.ask(Decision::Confirm {
                                player: ctx.controller,
                                prompt: "Karten aufdecken?".into(),
                            })?,
                            Choice::Yes
                        );
                    }
                    if (reveal && *remember_revealed) || (!reveal && *remember_peeked) {
                        self.card_mut(ctx.source).remembered.extend(revealable);
                    }
                }
            }
            Effect::Scry { who, num } => {
                let n = self.amount(num, ctx, targets).max(0) as usize;
                for p in Self::players_of(&self.resolve_defined(*who, targets, ctx)) {
                    let lib = &self.player(p).library;
                    let k = n.min(lib.len());
                    if k == 0 {
                        continue;
                    }
                    // oberste zuerst
                    let cards: Vec<CardId> = lib.iter().rev().take(k).copied().collect();
                    let (top, bottom) = match ch.ask(Decision::Scry {
                        player: p,
                        cards: cards.clone(),
                        graveyard: false,
                    })? {
                        Choice::Arrange { top, bottom } => (top, bottom),
                        _ => (cards.clone(), Vec::new()),
                    };
                    let lib = &mut self.player_mut(p).library;
                    lib.retain(|c| !cards.contains(c));
                    // unten: die zuerst genannte liegt am weitesten oben (Index 0 = ganz unten)
                    for c in bottom.iter() {
                        lib.insert(0, *c);
                    }
                    for c in top.iter().rev() {
                        lib.push(*c);
                    }
                }
            }
            Effect::Mill { who, num } => {
                let n = self.amount(num, ctx, targets).max(0) as usize;
                for p in Self::players_of(&self.resolve_defined(*who, targets, ctx)) {
                    let top: Vec<CardId> = self
                        .player(p)
                        .library
                        .iter()
                        .rev()
                        .take(n)
                        .copied()
                        .collect();
                    for c in top {
                        self.move_card(c, Zone::Graveyard, true);
                    }
                }
            }
            Effect::Dig {
                who,
                dig,
                change,
                valid,
                dest1,
                dest1_bottom,
                dest2,
                dest2_bottom,
                remember,
                optional,
            } => {
                let n = self.amount(dig, ctx, targets).max(0) as usize;
                for p in Self::players_of(&self.resolve_defined(*who, targets, ctx)) {
                    let top: Vec<CardId> = self
                        .player(p)
                        .library
                        .iter()
                        .rev()
                        .take(n)
                        .copied()
                        .collect();
                    let options: Vec<CardId> = top
                        .iter()
                        .copied()
                        .filter(|c| {
                            valid.as_ref().is_none_or(|v| {
                                self.card_matches(*c, v, ctx.source, ctx.controller)
                            })
                        })
                        .collect();
                    let chosen: Vec<CardId> = match change {
                        // ChangeNum$ All: alle passenden
                        None => options.clone(),
                        Some(a) => {
                            let max = (self.amount(a, ctx, targets).max(0) as usize)
                                .min(options.len()) as u32;
                            let min = if *optional { 0 } else { max };
                            if max == 0 {
                                Vec::new()
                            } else {
                                match ch.ask(Decision::ChooseCards {
                                    player: ctx.controller,
                                    reason: crate::decision::CardChoiceReason::Dig,
                                    options,
                                    min,
                                    max,
                                })? {
                                    Choice::Cards(c) => c,
                                    _ => Vec::new(),
                                }
                            }
                        }
                    };
                    for c in &chosen {
                        let ctrl = (*dest1 == Zone::Battlefield).then_some(ctx.controller);
                        let new = self.move_card_as(*c, *dest1, !dest1_bottom, ctrl);
                        if *remember {
                            self.card_mut(ctx.source).remembered.push(new);
                        }
                    }
                    for c in top.into_iter().filter(|c| !chosen.contains(c)) {
                        self.move_card(c, *dest2, !dest2_bottom);
                    }
                }
            }
            Effect::Regenerate { what } => {
                let ts = self.resolve_defined(*what, targets, ctx);
                for c in self.cards_of(&ts) {
                    self.card_mut(c).regen_shields += 1;
                }
            }
            Effect::Animate {
                what,
                power,
                toughness,
                types,
                remove_creature_types,
                keywords,
                colors,
                colors_chosen,
                permanent,
            } => {
                let colors = match colors_chosen {
                    Some(o) => self
                        .card(ctx.source)
                        .chosen_color
                        .map(|c| (std::iter::once(c).collect(), *o)),
                    None => *colors,
                };
                let sp = power.as_ref().map(|a| self.amount(a, ctx, targets));
                let st = toughness.as_ref().map(|a| self.amount(a, ctx, targets));
                let ts = self.resolve_defined(*what, targets, ctx);
                for c in self.cards_of(&ts) {
                    let timestamp = self.timestamp();
                    self.effects.push(ContinuousEffect {
                        affected: c,
                        keywords: keywords.clone(),
                        add_types: types
                            .iter()
                            .filter_map(|t| {
                                if t == "ChosenType" {
                                    self.card(ctx.source).chosen_type.clone()
                                } else {
                                    Some(t.clone())
                                }
                            })
                            .collect(),
                        remove_creature_types: *remove_creature_types,
                        colors,
                        set_power: sp,
                        set_toughness: st,
                        duration: if *permanent {
                            Duration::Permanent
                        } else {
                            Duration::UntilEndOfTurn
                        },
                        timestamp,
                        ..Default::default()
                    });
                }
            }
            Effect::Attach { object, to } => {
                // aktuelles Objekt (LKI-Verweise wie `TriggeredCardLKICopy` folgen dem Zonenwechsel)
                let to = Self::cards_of_any(&self.resolve_defined(*to, targets, ctx))
                    .into_iter()
                    .map(|c| self.current_object(c))
                    .find(|c| self.card(*c).zone == Some(Zone::Battlefield));
                let obj = self
                    .resolve_defined(*object, &[], ctx)
                    .into_iter()
                    .find_map(|t| {
                        if let Target::Card(c) = t {
                            Some(c)
                        } else {
                            None
                        }
                    });
                if let (Some(to), Some(obj)) = (to, obj)
                    && self.card(obj).zone == Some(Zone::Battlefield)
                    && obj != to
                {
                    self.card_mut(obj).attached_to = Some(to);
                }
            }
            // Modale Fähigkeiten werden vor dem Stapeln aufgelöst (choose_modes);
            // ein Charm ohne gewählte Modi tut nichts.
            Effect::Charm(_) => {}
            Effect::ChangeZone {
                what,
                origin,
                destination,
                place,
                shuffle,
                optional,
            } => {
                let mut owners: Vec<PlayerId> = Vec::new();
                let ts = self.resolve_defined(*what, targets, ctx);
                let cards: Vec<CardId> = ts
                    .iter()
                    .filter_map(|t| {
                        if let Target::Card(c) = t {
                            Some(*c)
                        } else {
                            None
                        }
                    })
                    .filter(|c| self.card(*c).zone.is_some_and(|z| origin.contains(&z)))
                    .collect();
                for c in cards {
                    if *optional
                        && !matches!(
                            ch.ask(Decision::Confirm {
                                player: ctx.controller,
                                prompt: "Karte bewegen?".into(),
                            })?,
                            Choice::Yes
                        )
                    {
                        continue;
                    }
                    let owner = self.card(c).owner;
                    let new = self.put_into_zone(c, *destination, place, ctx.controller);
                    if !owners.contains(&owner) {
                        owners.push(owner);
                    }
                    if place.remember {
                        self.card_mut(ctx.source).remembered.push(new);
                    }
                }
                if *shuffle {
                    for p in owners {
                        self.shuffle_library(p);
                    }
                }
            }
            Effect::ChangeZoneSearch {
                origin,
                optional,
                zone_owner,
                mandatory,
                destination,
                change_type,
                num,
                place,
                shuffle,
            } => {
                let chooser = ctx.controller;
                let p = match zone_owner {
                    Some(d) => {
                        match Self::players_of(&self.resolve_defined(*d, targets, ctx)).first() {
                            Some(o) => *o,
                            None => return Ok(()),
                        }
                    }
                    None => chooser,
                };
                let n = self.amount(num, ctx, targets).max(0) as u32;
                // Kandidaten: Karten des Spielers in den Ursprungszonen (Battlefield: kontrolliert).
                let mut pool: Vec<CardId> = Vec::new();
                for z in origin {
                    match z {
                        Zone::Battlefield => pool.extend(self.battlefield_of(p)),
                        Zone::Exile => pool.extend(
                            self.exile
                                .iter()
                                .copied()
                                .filter(|c| self.card(*c).owner == p),
                        ),
                        Zone::Command => pool.extend(
                            self.command
                                .iter()
                                .copied()
                                .filter(|c| self.card(*c).owner == p),
                        ),
                        Zone::Stack => {}
                        pz => pool.extend(self.player(p).zone(*pz).iter().copied()),
                    }
                }
                let options: Vec<CardId> = pool
                    .into_iter()
                    .filter(|c| self.card_matches(*c, change_type, ctx.source, chooser))
                    .collect();
                let max = n.min(options.len() as u32);
                // CR 701.19b: nur in verdeckten Zonen darf man „nicht finden“.
                let min = if *mandatory {
                    max
                } else if *optional || origin.iter().any(|z| z.is_hidden()) {
                    0
                } else {
                    max
                };
                let chosen = if max == 0 {
                    Vec::new()
                } else {
                    match ch.ask(Decision::ChooseCards {
                        player: chooser,
                        reason: crate::decision::CardChoiceReason::Search,
                        options,
                        min,
                        max,
                    })? {
                        Choice::Cards(c) => c,
                        _ => Vec::new(),
                    }
                };
                for c in chosen {
                    let new = self.put_into_zone(c, *destination, place, chooser);
                    if place.remember {
                        self.card_mut(ctx.source).remembered.push(new);
                    }
                }
                if *shuffle && origin.contains(&Zone::Library) {
                    self.shuffle_library(p);
                }
            }
            Effect::ChangeZoneAll {
                origin,
                destination,
                change_type,
                place,
                shuffle,
            } => {
                let mut cards = Vec::new();
                for z in origin {
                    match z {
                        Zone::Battlefield => cards.extend(self.battlefield.iter().copied()),
                        Zone::Exile => cards.extend(self.exile.iter().copied()),
                        Zone::Command | Zone::Stack => {}
                        pz => {
                            for pl in self.apnap() {
                                cards.extend(self.player(pl).zone(*pz).iter().copied());
                            }
                        }
                    }
                }
                cards.retain(|c| self.card_matches(*c, change_type, ctx.source, ctx.controller));
                // gleichzeitig: Rückblick für Trigger (CR 603.10a)
                self.lki_battlefield = Some(self.battlefield.clone());
                for c in cards {
                    let new = self.put_into_zone(c, *destination, place, ctx.controller);
                    if place.remember {
                        self.card_mut(ctx.source).remembered.push(new);
                    }
                }
                self.lki_battlefield = None;
                if *shuffle {
                    for pl in self.apnap() {
                        self.shuffle_library(pl);
                    }
                }
            }
        }
        Ok(())
    }

    /// Was kopiert wird: Auswahl aus `Choices$` (eine Karte) oder `Defined$`/Ziele.
    fn pick_clone_source(
        &mut self,
        source: &super::CloneSource,
        targets: &[Target],
        ctx: &ResolveCtx,
        ch: &mut Choices,
    ) -> Res<Vec<CardId>> {
        Ok(match source {
            super::CloneSource::Defined(d) => {
                self.cards_of(&self.resolve_defined(*d, targets, ctx))
            }
            super::CloneSource::Choices {
                zones,
                spec,
                optional,
            } => {
                let all = self.players_from(self.active);
                let options: Vec<CardId> = self
                    .cards_in_zones(zones, &all)
                    .into_iter()
                    .filter(|c| self.card_matches(*c, spec, ctx.source, ctx.controller))
                    .collect();
                if options.is_empty() {
                    return Ok(Vec::new());
                }
                match ch.ask(Decision::ChooseCards {
                    player: ctx.controller,
                    reason: crate::decision::CardChoiceReason::Choose,
                    options,
                    min: u32::from(!*optional),
                    max: 1,
                })? {
                    Choice::Cards(c) => c,
                    _ => Vec::new(),
                }
            }
        })
    }

    /// Bewegt eine Karte gemäß `place` (Tapped, Kontrolle, Bibliotheksposition).
    pub fn put_into_zone(
        &mut self,
        c: CardId,
        to: Zone,
        place: &super::ZonePlacement,
        controller: PlayerId,
    ) -> CardId {
        let ctrl = place.gain_control.then_some(controller);
        let origin = self.card(c).zone;
        let host = self.effect_source;
        if place.until_host_leaves
            && host.is_none_or(|h| self.card(h).zone != Some(Zone::Battlefield))
        {
            // CR 610.3c: Quelle schon weg → nichts geschieht
            return c;
        }
        let new = self.move_card_as(c, to, !place.library_bottom, ctrl);
        if place.until_host_leaves
            && let (Some(h), Some(o)) = (host, origin)
        {
            self.card_mut(new).exiled_until = Some((h, o));
        }
        if self.card(new).zone == Some(Zone::Battlefield) && place.tapped {
            self.card_mut(new).tapped = true;
        }
        if self.card(new).zone == Some(Zone::Battlefield) && place.transformed {
            self.transform(new);
        }
        new
    }

    /// Führt eine Kette ohne eigene Ziele aus, die die Ziele `inherited` der
    /// Elternfähigkeit erbt (z. B. `RepeatSubAbility$`).
    pub fn apply_chain_inherit(
        &mut self,
        ab: &CompiledAbility,
        inherited: &[Target],
        ctx: &ResolveCtx,
        ch: &mut Choices,
    ) -> Res<()> {
        self.apply_chain_from(ab, &[], inherited, ctx, ch)
    }

    /// Führt eine Fähigkeitskette mit bereits geprüften Zielen aus.
    pub fn apply_chain(
        &mut self,
        ab: &CompiledAbility,
        targets: &[Vec<Target>],
        ctx: &ResolveCtx,
        ch: &mut Choices,
    ) -> Res<()> {
        self.apply_chain_from(ab, targets, &[], ctx, ch)
    }

    fn apply_chain_from(
        &mut self,
        ab: &CompiledAbility,
        targets: &[Vec<Target>],
        inherited: &[Target],
        ctx: &ResolveCtx,
        ch: &mut Choices,
    ) -> Res<()> {
        // Unterfähigkeiten ohne eigene Ziele beziehen sich auf die Ziele der
        // nächsten Elternfähigkeit mit Zielen (Java: SpellAbility.getParentTargetingCard).
        let mut inherited: &[Target] = inherited;
        for (i, a) in ab.chain().into_iter().enumerate() {
            let own = targets.get(i).map(Vec::as_slice).unwrap_or(&[]);
            // Ziele der Elternfähigkeit (für `Defined$ ParentTarget`)
            self.parent_targets = inherited.to_vec();
            self.current_division = if a.target.as_ref().is_some_and(|t| t.divided) {
                self.stack_division.get(i).cloned().unwrap_or_default()
            } else {
                Vec::new()
            };
            let t = if a.target.is_some() {
                inherited = own;
                own
            } else {
                inherited
            };
            // CR 608.2c: bedingte Teile nur, wenn die Bedingung erfüllt ist.
            if !self.check_conditions(&a.conditions, &ctx.eval_with(t)) {
                continue;
            }
            let Some(u) = &a.unless else {
                self.apply_effect(&a.effect, t, ctx, ch)?;
                continue;
            };
            // CR 118.12: „… es sei denn, [Spieler] zahlt …“
            let paid = self.offer_unless_cost(u, t, ctx, ch)?;
            if paid == u.switched {
                self.apply_effect(&a.effect, t, ctx, ch)?;
            }
            let go_on = match u.subs {
                super::UnlessSubs::Always => true,
                super::UnlessSubs::WhenPaid => paid,
                super::UnlessSubs::WhenNotPaid => !paid,
            };
            if !go_on {
                break;
            }
        }
        Ok(())
    }

    /// Löst das oberste Stapelobjekt auf.
    pub fn resolve_top(&mut self, ch: &mut Choices) -> Res<()> {
        let Some(item) = self.stack.last().cloned() else {
            return Ok(());
        };
        let (trig, optional) = match &item.kind {
            StackKind::Trigger { info, optional, .. } => (Some(info.clone()), *optional),
            _ => (None, false),
        };
        let ctx = ResolveCtx {
            source: item.source,
            controller: item.controller,
            x: item.x,
            trig,
            paid: item.paid.clone(),
        };

        // CR 608.2b: Ziele erneut prüfen.
        let mut legal_targets: Vec<Vec<Target>> = Vec::new();
        let mut had_targets = false;
        let mut any_legal = false;
        if let Some(ab) = &item.ability {
            for (i, a) in ab.chain().into_iter().enumerate() {
                let chosen = item.targets.get(i).cloned().unwrap_or_default();
                let legal: Vec<Target> = match &a.target {
                    Some(spec) => {
                        had_targets = true;
                        chosen
                            .into_iter()
                            .filter(|t| {
                                self.is_legal_target(spec, *t, item.source, item.controller)
                            })
                            .collect()
                    }
                    None => chosen,
                };
                if a.target.is_some() && !legal.is_empty() {
                    any_legal = true;
                }
                legal_targets.push(legal);
            }
        }
        // CR 603.4: „intervening if“ beim Auflösen erneut prüfen – sonst passiert nichts.
        let condition_failed = match &item.kind {
            StackKind::Trigger { conditions, .. } => {
                !self.check_conditions(conditions, &ctx.eval())
            }
            _ => false,
        };
        let fizzle = (had_targets && !any_legal) || condition_failed;

        self.stack.pop();
        if fizzle {
            self.push_event(Event::Fizzled { item: item.id });
        } else {
            // „You may“ wird beim Auflösen entschieden.
            let decider = match &item.kind {
                StackKind::Trigger {
                    decider: Some(d), ..
                } => {
                    let flat: Vec<Target> = legal_targets.iter().flatten().copied().collect();
                    Self::players_of(&self.resolve_defined(*d, &flat, &ctx))
                        .into_iter()
                        .find(|p| self.player(*p).is_in_game())
                        .unwrap_or(item.controller)
                }
                _ => item.controller,
            };
            let declined = optional
                && matches!(
                    ch.ask(Decision::OptionalTrigger {
                        player: decider,
                        description: match &item.kind {
                            StackKind::Trigger { description, .. } => description.clone(),
                            _ => String::new(),
                        },
                    })?,
                    Choice::No
                );
            if let (Some(ab), false) = (&item.ability, declined) {
                self.stack_division = item.division.clone();
                let r = self.apply_chain(ab, &legal_targets, &ctx, ch);
                self.stack_division.clear();
                self.current_division.clear();
                r?;
            }
            self.push_event(Event::Resolved { item: item.id });
        }
        self.finish_spell(&item, fizzle);
        Ok(())
    }

    /// Nach dem Auflösen: Spontanzauber/Hexereien auf den Friedhof (CR 608.2n),
    /// bleibende Karten aufs Battlefield (CR 608.3).
    fn finish_spell(&mut self, item: &StackItem, fizzled: bool) {
        let StackKind::Spell { card } = item.kind else {
            return;
        };
        // CR 715.3d: aufgelöster Abenteuer-Zauber geht ins Exil, „auf Abenteuer“.
        if !fizzled
            && self.card(card).face == 1
            && self.card(card).def.script.split_mode == forge_script::SplitMode::Adventure
        {
            let new = self.move_card(card, Zone::Exile, true);
            self.card_mut(new).on_adventure = true;
            return;
        }
        let is_permanent = self.chars(card).types.is_permanent();
        if is_permanent && !fizzled {
            // CR 608.3a: unter Kontrolle des Zauberspruch-Controllers
            let new = self.move_card_as(card, Zone::Battlefield, true, Some(item.controller));
            // CR 303.4f: Aura kommt an das (noch legale) Ziel angelegt ins Spiel
            if let Some(ab) = &item.ability
                && matches!(ab.effect, super::Effect::AuraSpell)
            {
                let target = item
                    .targets
                    .first()
                    .and_then(|t| t.first())
                    .and_then(|t| match t {
                        Target::Card(c) => Some(*c),
                        Target::Player(_) => None,
                    });
                if let Some(t) = target.filter(|t| self.card(*t).zone == Some(Zone::Battlefield)) {
                    self.card_mut(new).attached_to = Some(t);
                }
            }
        } else if !fizzled
            && self.card(card).cast_from == Some(Zone::Hand)
            && self.chars(card).has(&crate::keyword::Keyword::Rebound)
        {
            // CR 702.88a: aus der Hand gewirkt → ins Exil statt in den Friedhof
            let new = self.move_card(card, Zone::Exile, true);
            self.card_mut(new).rebound = true;
        } else {
            self.move_card(card, Zone::Graveyard, true);
        }
    }
}

impl GameState {
    /// Bietet den `UnlessPayer$`-Spielern nacheinander an, die Kosten zu
    /// zahlen. Gefragt wird nur, wer sie bezahlen kann; sobald einer zahlt, ist
    /// Schluss (Java fragt weitere Spieler mit `alreadyPaid`; die KI zahlt dann nie).
    fn offer_unless_cost(
        &mut self,
        u: &super::UnlessSpec,
        targets: &[Target],
        ctx: &ResolveCtx,
        ch: &mut Choices,
    ) -> Res<bool> {
        for p in Self::players_of(&self.resolve_defined(u.payer, targets, ctx)) {
            if self.player(p).lost.is_some() || !self.can_pay_unless(p, &u.cost, ctx.source) {
                continue; // CR 800.4f
            }
            let wants = matches!(
                ch.ask(Decision::PayUnlessCost {
                    player: p,
                    cost: u.cost.to_string(),
                })?,
                Choice::Yes
            );
            if !wants {
                continue;
            }
            if let Some(m) = &u.cost.mana
                && !self.pay_unless_cost(p, m)
            {
                continue;
            }
            self.pay_nonmana_costs(p, ctx.source, &u.cost, ch)?;
            return Ok(true);
        }
        Ok(false)
    }

    /// Kann `p` die Unless-Kosten jetzt bezahlen?
    fn can_pay_unless(&self, p: PlayerId, cost: &crate::cost::Cost, source: CardId) -> bool {
        use crate::cost::CostPart;
        // „Opfere ~“ als Kosten kann nur der Controller der Quelle zahlen.
        if cost.parts.contains(&CostPart::SacrificeSelf) && self.card(source).controller != p {
            return false;
        }
        if !self.can_pay_nonmana(cost, source, p) {
            return false;
        }
        match &cost.mana {
            None => true,
            Some(m) => {
                self.player(p).mana_pool.clone().try_pay(m, 0, &[])
                    || self.auto_pay_plan(p, m, 0, &[]).is_some()
            }
        }
    }

    /// Bezahlt Manakosten außerhalb des Wirkens (UnlessCost) aus Pool und
    /// einfachen Quellen. `false`, wenn nicht bezahlbar (nichts wird verändert).
    pub(crate) fn pay_unless_cost(&mut self, p: PlayerId, cost: &forge_types::ManaCost) -> bool {
        let mut pool = self.player(p).mana_pool.clone();
        if pool.try_pay(cost, 0, &[]) {
            self.player_mut(p).mana_pool = pool;
            return true;
        }
        let Some(plan) = self.auto_pay_plan(p, cost, 0, &[]) else {
            return false;
        };
        for (c, _i, color) in plan {
            self.tap(c);
            self.player_mut(p).mana_pool.add(color, false);
        }
        let mut pool = self.player(p).mana_pool.clone();
        let ok = pool.try_pay(cost, 0, &[]);
        if ok {
            self.player_mut(p).mana_pool = pool;
        }
        ok
    }
}
