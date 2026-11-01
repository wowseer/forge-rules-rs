//! Auflösen von Stapelobjekten (CR 608).
//!
//! Java-Vorbild: `MagicStack.resolveStack`, `AbilityUtils.resolve`,
//! die einzelnen `*Effect.resolve`-Methoden.

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
}

impl GameState {
    fn amount(&self, a: &Amount, ctx: &ResolveCtx) -> i32 {
        match a {
            Amount::Fixed(n) => *n,
            Amount::XPaid => ctx.x as i32,
        }
    }

    /// Wen betrifft `Defined` – als Ziele (Karten/Spieler).
    fn resolve_defined(&self, d: Defined, targets: &[Target], ctx: &ResolveCtx) -> Vec<Target> {
        match d {
            Defined::You => vec![Target::Player(ctx.controller)],
            Defined::SelfCard => {
                let c = self.card(ctx.source);
                if c.zone == Some(Zone::Battlefield) {
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
            Defined::TriggeredSource => ctx
                .trig
                .as_ref()
                .and_then(|t| t.source)
                .map(Target::Card)
                .into_iter()
                .collect(),
        }
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
        match eff {
            Effect::DealDamage { amount, to } => {
                let n = self.amount(amount, ctx).max(0) as u32;
                for t in self.resolve_defined(*to, targets, ctx) {
                    self.deal_damage(ctx.source, t, n);
                }
            }
            Effect::Draw { amount, who } => {
                let n = self.amount(amount, ctx).max(0) as u32;
                for p in Self::players_of(&self.resolve_defined(*who, targets, ctx)) {
                    self.draw_n(p, n);
                }
            }
            Effect::GainLife { amount, who } => {
                let n = self.amount(amount, ctx);
                for p in Self::players_of(&self.resolve_defined(*who, targets, ctx)) {
                    self.gain_life(p, n);
                }
            }
            Effect::LoseLife { amount, who } => {
                let n = self.amount(amount, ctx);
                for p in Self::players_of(&self.resolve_defined(*who, targets, ctx)) {
                    self.lose_life(p, n);
                }
            }
            Effect::Destroy { what, .. } => {
                let ts = self.resolve_defined(*what, targets, ctx);
                // CR 701.8: gleichzeitig zerstören
                for c in self.cards_of(&ts) {
                    self.destroy(c);
                }
            }
            Effect::Pump {
                what,
                power,
                toughness,
                keywords,
            } => {
                let p = self.amount(power, ctx);
                let t = self.amount(toughness, ctx);
                let ts = self.resolve_defined(*what, targets, ctx);
                for c in self.cards_of(&ts) {
                    let timestamp = self.timestamp();
                    self.effects.push(ContinuousEffect {
                        affected: c,
                        power: p,
                        toughness: t,
                        keywords: keywords.clone(),
                        duration: Duration::UntilEndOfTurn,
                        timestamp,
                    });
                }
            }
            Effect::AddMana { produce, amount } => {
                let snow = self
                    .chars(ctx.source)
                    .types
                    .has_super(forge_types::Supertype::Snow);
                for _ in 0..*amount {
                    match produce {
                        ManaProduce::Fixed(syms) => {
                            for s in syms {
                                self.player_mut(ctx.controller).mana_pool.add(*s, snow);
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
                    self.card_mut(c).tapped = true;
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
            } => {
                let n = self.amount(amount, ctx).max(0) as u32;
                let ts = self.resolve_defined(*what, targets, ctx);
                for c in self.cards_of(&ts) {
                    self.add_counters(c, counter, n);
                }
            }
        }
        Ok(())
    }

    /// Führt eine Fähigkeitskette mit bereits geprüften Zielen aus.
    pub fn apply_chain(
        &mut self,
        ab: &CompiledAbility,
        targets: &[Vec<Target>],
        ctx: &ResolveCtx,
        ch: &mut Choices,
    ) -> Res<()> {
        for (i, a) in ab.chain().into_iter().enumerate() {
            let t = targets.get(i).map(Vec::as_slice).unwrap_or(&[]);
            self.apply_effect(&a.effect, t, ctx, ch)?;
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
        let fizzle = had_targets && !any_legal;

        self.stack.pop();
        if fizzle {
            self.push_event(Event::Fizzled { item: item.id });
        } else {
            // „You may“ wird beim Auflösen entschieden.
            let declined = optional
                && matches!(
                    ch.ask(Decision::OptionalTrigger {
                        player: item.controller,
                        description: match &item.kind {
                            StackKind::Trigger { description, .. } => description.clone(),
                            _ => String::new(),
                        },
                    })?,
                    Choice::No
                );
            if let (Some(ab), false) = (&item.ability, declined) {
                self.apply_chain(ab, &legal_targets, &ctx, ch)?;
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
        let is_permanent = self.chars(card).types.is_permanent();
        if is_permanent && !fizzled {
            let new = self.move_card(card, Zone::Battlefield, true);
            let ctrl = item.controller;
            self.card_mut(new).controller = ctrl;
        } else {
            self.move_card(card, Zone::Graveyard, true);
        }
    }
}
