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
            Defined::Remembered => self
                .remembered_current(ctx.source)
                .into_iter()
                .map(Target::Card)
                .collect(),
            Defined::RememberedLki => self
                .card(ctx.source)
                .remembered
                .iter()
                .copied()
                .map(Target::Card)
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
                        ..Default::default()
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
            Effect::PumpAll {
                valid,
                power,
                toughness,
                keywords,
            } => {
                let p = self.amount(power, ctx);
                let t = self.amount(toughness, ctx);
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
                let n = self.amount(amount, ctx).max(0) as u32;
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
                for r in recipients {
                    self.deal_damage(ctx.source, r, n);
                }
            }
            Effect::Discard {
                who,
                mode,
                num,
                valid,
            } => {
                let n = self.amount(num, ctx).max(0) as usize;
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
                        self.move_card(c, Zone::Graveyard, true);
                    }
                }
            }
            Effect::Counter => {
                for t in targets {
                    let Target::Card(card) = *t else { continue };
                    let Some(pos) = self
                        .stack
                        .iter()
                        .position(|s| matches!(s.kind, StackKind::Spell { card: c } if c == card))
                    else {
                        continue;
                    };
                    // CR 701.5a: vom Stapel in den Friedhof des Besitzers
                    self.stack.remove(pos);
                    self.move_card(card, Zone::Graveyard, true);
                }
            }
            Effect::Cleanup {
                what,
                clear_remembered,
            } => {
                for t in self.resolve_defined(*what, targets, ctx) {
                    if let (Target::Card(c), true) = (t, *clear_remembered) {
                        self.card_mut(c).remembered.clear();
                    }
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
                let n = self.amount(amount, ctx).max(0) as usize;
                // CR 101.4: erst wählen alle (APNAP-Reihenfolge der Defined-Liste),
                // dann wird gleichzeitig geopfert.
                let mut chosen: Vec<CardId> = Vec::new();
                match valid {
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
                let remembered: Vec<CardId> = match remember {
                    Some(d) => self
                        .resolve_defined(*d, targets, ctx)
                        .into_iter()
                        .filter_map(|t| {
                            if let Target::Card(c) = t {
                                Some(c)
                            } else {
                                None
                            }
                        })
                        .collect(),
                    None => Vec::new(),
                };
                let turn = self.turn;
                let o = self.card_mut(id);
                o.remembered = remembered;
                o.effect = Some(crate::state::EffectInfo {
                    duration: *duration,
                    created_turn: turn,
                });
            }
            Effect::AuraSpell => {}
            Effect::Scry { who, num } => {
                let n = self.amount(num, ctx).max(0) as usize;
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
                let n = self.amount(num, ctx).max(0) as usize;
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
                optional,
            } => {
                let n = self.amount(dig, ctx).max(0) as usize;
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
                            let max =
                                (self.amount(a, ctx).max(0) as usize).min(options.len()) as u32;
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
                        self.move_card_as(*c, *dest1, !dest1_bottom, ctrl);
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
                permanent,
            } => {
                let sp = power.as_ref().map(|a| self.amount(a, ctx));
                let st = toughness.as_ref().map(|a| self.amount(a, ctx));
                let ts = self.resolve_defined(*what, targets, ctx);
                for c in self.cards_of(&ts) {
                    let timestamp = self.timestamp();
                    self.effects.push(ContinuousEffect {
                        affected: c,
                        keywords: keywords.clone(),
                        add_types: types.clone(),
                        remove_creature_types: *remove_creature_types,
                        colors: *colors,
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
            Effect::Attach { object } => {
                let to = self.cards_of(targets).into_iter().next();
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
        let ctrl = place.gain_control.then_some(controller);
        let new = self.move_card_as(c, to, !place.library_bottom, ctrl);
        if self.card(new).zone == Some(Zone::Battlefield) && place.tapped {
            self.card_mut(new).tapped = true;
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
            self.card_mut(c).tapped = true;
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
