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

impl ResolveCtx {
    pub fn eval(&self) -> crate::expr::EvalCtx {
        crate::expr::EvalCtx {
            source: self.source,
            controller: self.controller,
            x: self.x,
        }
    }
}

impl GameState {
    fn amount(&self, a: &Amount, ctx: &ResolveCtx) -> i32 {
        match a {
            Amount::Fixed(n) => *n,
            Amount::XPaid => ctx.x as i32,
            Amount::Expr(e) => self.eval(e, &ctx.eval()),
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
            Effect::Token {
                scripts,
                amount,
                owner,
                tapped,
                power,
                toughness,
            } => {
                let n = self.amount(amount, ctx).max(0) as u32;
                let p_override = power.as_ref().map(|a| self.amount(a, ctx));
                let t_override = toughness.as_ref().map(|a| self.amount(a, ctx));
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
            // Modale Fähigkeiten werden vor dem Stapeln aufgelöst (choose_modes);
            // ein Charm ohne gewählte Modi tut nichts.
            Effect::Charm(_) => {}
            Effect::ChangeZone {
                what,
                origin,
                destination,
                place,
            } => {
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
                    self.put_into_zone(c, *destination, place, ctx.controller);
                }
            }
            Effect::ChangeZoneSearch {
                origin,
                destination,
                change_type,
                num,
                place,
                shuffle,
            } => {
                let p = ctx.controller;
                let n = self.amount(num, ctx).max(0) as u32;
                // Kandidaten: eigene Karten in der Ursprungszone (Battlefield: kontrolliert).
                let pool: Vec<CardId> = match origin {
                    Zone::Battlefield => self.battlefield_of(p).collect(),
                    Zone::Exile => self
                        .exile
                        .iter()
                        .copied()
                        .filter(|c| self.card(*c).owner == p)
                        .collect(),
                    Zone::Command => self
                        .command
                        .iter()
                        .copied()
                        .filter(|c| self.card(*c).owner == p)
                        .collect(),
                    Zone::Stack => Vec::new(),
                    pz => self.player(p).zone(*pz).clone(),
                };
                let options: Vec<CardId> = pool
                    .into_iter()
                    .filter(|c| self.card_matches(*c, change_type, ctx.source, p))
                    .collect();
                let max = n.min(options.len() as u32);
                // CR 701.19b: nur in verdeckten Zonen darf man „nicht finden“.
                let min = if origin.is_hidden() { 0 } else { max };
                let chosen = if max == 0 {
                    Vec::new()
                } else {
                    match ch.ask(Decision::ChooseCards {
                        player: p,
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
                    self.put_into_zone(c, *destination, place, p);
                }
                if *shuffle && *origin == Zone::Library {
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
                    self.put_into_zone(c, *destination, place, ctx.controller);
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

    /// Bewegt eine Karte gemäß `place` (Tapped, Kontrolle, Bibliotheksposition).
    pub fn put_into_zone(
        &mut self,
        c: CardId,
        to: Zone,
        place: &super::ZonePlacement,
        controller: PlayerId,
    ) -> CardId {
        let new = self.move_card(c, to, !place.library_bottom);
        if to == Zone::Battlefield {
            if place.tapped {
                self.card_mut(new).tapped = true;
            }
            if place.gain_control {
                self.card_mut(new).controller = controller;
            }
        }
        new
    }

    /// Führt eine Fähigkeitskette mit bereits geprüften Zielen aus.
    pub fn apply_chain(
        &mut self,
        ab: &CompiledAbility,
        targets: &[Vec<Target>],
        ctx: &ResolveCtx,
        ch: &mut Choices,
    ) -> Res<()> {
        // Unterfähigkeiten ohne eigene Ziele beziehen sich auf die Ziele der
        // nächsten Elternfähigkeit mit Zielen (Java: SpellAbility.getParentTargetingCard).
        let mut inherited: &[Target] = &[];
        for (i, a) in ab.chain().into_iter().enumerate() {
            let own = targets.get(i).map(Vec::as_slice).unwrap_or(&[]);
            let t = if a.target.is_some() {
                inherited = own;
                own
            } else {
                inherited
            };
            // CR 608.2c: bedingte Teile nur, wenn die Bedingung erfüllt ist.
            if !self.check_conditions(&a.conditions, &ctx.eval()) {
                continue;
            }
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
