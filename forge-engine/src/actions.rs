//! Grundlegende Spielaktionen (Java: `GameAction`, Teile von `Player`/`Card`).

use crate::decision::Target;
use crate::ids::{CardId, PlayerId};
use crate::keyword::Keyword;
use crate::state::{CardObj, Event, GameState};
use crate::zone::Zone;

impl GameState {
    /// Erzeugt ein neues Kartenobjekt in einer Zone (Spielaufbau, Tokens).
    pub fn create_card(
        &mut self,
        def: std::sync::Arc<crate::definition::CardDefinition>,
        owner: PlayerId,
        zone: Zone,
        token: bool,
    ) -> CardId {
        let id = CardId(self.cards.len() as u32);
        let ts = self.timestamp();
        self.cards.push(CardObj {
            id,
            def,
            face: 0,
            owner,
            controller: owner,
            zone: Some(zone),
            token,
            tapped: false,
            controlled_since_turn_start: false,
            base_controller: owner,
            damage: 0,
            deathtouch_damage: false,
            counters: Default::default(),
            timestamp: ts,
            moved_to: None,
            base_pt: None,
            cast_x: 0,
            remembered: Vec::new(),
            remembered_players: Vec::new(),
            chosen_cards: Vec::new(),
            chosen_type: None,
            chosen_color: None,
            chosen_player: None,
            named_card: None,
            runtime_vars: Default::default(),
            goaded_by: Vec::new(),
            face_down: false,
            disguised: false,
            rebound: false,
            effect: None,
            attached_to: None,
            regen_shields: 0,
            on_adventure: false,
            loyalty_used_turn: None,
            activations: Vec::new(),
            may_play_uses: Vec::new(),
            monstrous: false,
            exile_from_stack: false,
            last_x: 0,
            cast_from: None,
            cast_by: None,
            kicked: 0,
            skip_next_untap: false,
            exiled_with: None,
            exiled_until: None,
            commander: None,
            commander_casts: 0,
            commander_stay: false,
            prevent_next: 0,
            copy: None,
        });
        if zone != Zone::Stack {
            self.zone_list_mut(zone, owner).push(id);
        }
        id
    }

    fn zone_list_mut(&mut self, zone: Zone, owner: PlayerId) -> &mut Vec<CardId> {
        match zone {
            Zone::Battlefield => &mut self.battlefield,
            Zone::Exile => &mut self.exile,
            Zone::Command => &mut self.command,
            Zone::Stack => panic!("Stack-Objekte werden über StackItem verwaltet"),
            z => self.player_mut(owner).zone_mut(z),
        }
    }

    fn remove_from_zone(&mut self, id: CardId) {
        let c = self.card(id);
        let (zone, owner) = (c.zone, c.owner);
        let Some(zone) = zone else { return };
        if zone == Zone::Stack {
            return;
        }
        self.zone_list_mut(zone, owner).retain(|x| *x != id);
    }

    /// Bewegt ein Objekt in eine andere Zone (CR 400.7: neues Objekt).
    /// `to_top`: bei Bibliothek oben (true) oder unten (false).
    /// Liefert die neue ID; Tokens außerhalb des Battlefields werden später
    /// per SBA entfernt (CR 704.5d).
    pub fn move_card(&mut self, id: CardId, to: Zone, to_top: bool) -> CardId {
        self.move_card_as(id, to, to_top, None)
    }

    /// Wie [`GameState::move_card`], mit Controller beim Ankommen (Zauber des
    /// Wirkenden, „unter deiner Kontrolle“). Ersatzeffekte (CR 614) werden hier
    /// angewendet: Umleitung, getappt, mit Marken.
    pub fn move_card_as(
        &mut self,
        id: CardId,
        to: Zone,
        to_top: bool,
        controller: Option<PlayerId>,
    ) -> CardId {
        let from = self.card(id).zone;
        let to =
            if from == Some(Zone::Stack) && to == Zone::Graveyard && self.card(id).exile_from_stack
            {
                Zone::Exile
            } else {
                to
            };
        let (to, mods) = self.replace_move(id, from, to);
        self.remove_from_zone(id);
        // CR 707: Kopie-Werte enden mit dem Zonenwechsel (eigene Werte zurück).
        let old = {
            let mut o = self.card(id).clone();
            if let Some(cp) = o.copy.take() {
                o.def = cp.orig_def;
                o.face = cp.orig_face;
                o.base_pt = cp.orig_base_pt;
            }
            o
        };
        let new_id = CardId(self.cards.len() as u32);
        let ts = self.timestamp();
        // Standard: Besitzer (CR 108.4, 110.2); auf dem Battlefield ggf. abweichend.
        let controller = if to == Zone::Battlefield || to == Zone::Stack {
            controller.unwrap_or(old.owner)
        } else {
            old.owner
        };
        self.cards.push(CardObj {
            id: new_id,
            def: old.def.clone(),
            // Gewählte Seite bleibt auf dem Stapel und – bei modalen doppelseitigen
            // Karten – auf dem Battlefield (CR 712.8, 712.14); sonst Vorderseite.
            face: if to == Zone::Stack
                || (to == Zone::Battlefield
                    && old.def.script.split_mode == forge_script::SplitMode::Modal)
            {
                old.face
            } else {
                0
            },
            owner: old.owner,
            controller,
            zone: Some(to),
            token: old.token,
            tapped: false,
            controlled_since_turn_start: false,
            base_controller: controller,
            damage: 0,
            deathtouch_damage: false,
            counters: Default::default(),
            timestamp: ts,
            moved_to: None,
            base_pt: None,
            cast_x: if from == Some(Zone::Stack) {
                old.cast_x
            } else {
                0
            },
            remembered: Vec::new(),
            remembered_players: Vec::new(),
            chosen_cards: Vec::new(),
            chosen_type: None,
            chosen_color: None,
            chosen_player: None,
            named_card: None,
            runtime_vars: Default::default(),
            rebound: false,
            face_down: old.face_down
                && (to == Zone::Stack || (from == Some(Zone::Stack) && to == Zone::Battlefield)),
            disguised: old.disguised
                && (to == Zone::Stack || (from == Some(Zone::Stack) && to == Zone::Battlefield)),
            goaded_by: Vec::new(),
            effect: None,
            attached_to: None,
            regen_shields: 0,
            on_adventure: false,
            loyalty_used_turn: None,
            activations: Vec::new(),
            may_play_uses: Vec::new(),
            monstrous: false,
            exile_from_stack: false,
            last_x: 0,
            cast_by: if from == Some(Zone::Stack) && to == Zone::Battlefield {
                old.cast_by
            } else {
                None
            },
            cast_from: if from == Some(Zone::Stack) && to == Zone::Battlefield {
                old.cast_from
            } else {
                None
            },
            commander: old.commander,
            commander_casts: old.commander_casts,
            commander_stay: false,
            exiled_with: if to == Zone::Exile {
                self.effect_source
            } else {
                None
            },
            exiled_until: None,
            skip_next_untap: false,
            kicked: if from == Some(Zone::Stack) && to == Zone::Battlefield {
                old.kicked
            } else {
                0
            },
            prevent_next: 0,
            copy: None,
        });
        {
            let o = self.card_mut(id);
            o.zone = None;
            o.moved_to = Some(new_id);
        }
        // CR 614.1c/614.12: getappt bzw. mit Marken ins Spiel
        {
            let n = self.card_mut(new_id);
            n.tapped = mods.tapped;
            for (k, v) in &mods.counters {
                *n.counters.entry(k.clone()).or_insert(0) += v;
            }
        }
        // CR 306.5b: Planeswalker kommen mit ihrer aufgedruckten Loyalität ins Spiel.
        if to == Zone::Battlefield
            && let Some(forge_script::StatValue::Fixed(l)) = self.card(new_id).face().loyalty
            && self
                .card(new_id)
                .face()
                .types
                .has_core(forge_types::CoreType::Planeswalker)
        {
            self.add_counters(new_id, "LOYALTY", l.max(0) as u32);
        }
        if to != Zone::Stack {
            let owner = old.owner;
            let list = self.zone_list_mut(to, owner);
            if to == Zone::Library && !to_top {
                list.insert(0, new_id);
            } else {
                list.push(new_id);
            }
        }
        if let Some(combat) = self.combat.as_mut() {
            combat.attackers.retain(|a| a.card != id);
            combat.blocks.retain(|(b, _)| *b != id);
        }
        self.this_turn.moves.push((id, new_id, from, to));
        self.push_event(Event::ZoneChange {
            old: id,
            new: new_id,
            from,
            to,
        });
        self.fire(crate::triggers::TriggerEvent::ChangesZone {
            old: id,
            new: new_id,
            from,
            to,
        });
        match self.zone_batch.as_mut() {
            Some(b) => b.push((id, new_id, from, to)),
            None => self.fire(crate::triggers::TriggerEvent::ChangesZoneAll {
                moves: vec![(id, new_id, from, to)],
            }),
        }
        if to == Zone::Battlefield && !self.card(new_id).compiled().etb_other.is_empty() {
            self.pending_etb.push(new_id);
        }
        // CR 122.6: Marken, mit denen ein Objekt ins Spiel kommt, gelten als
        // daraufgelegt (z. B. Saga-Kapitel I).
        if to == Zone::Battlefield {
            let player = self.card(new_id).controller;
            for (k, v) in &mods.counters {
                self.fire_counters_added(new_id, k, 0, *v);
                self.fire(crate::triggers::TriggerEvent::CounterAddedOnce {
                    card: new_id,
                    kind: k.clone(),
                    amount: *v,
                    player,
                });
            }
        }
        if from == Some(Zone::Battlefield) {
            self.host_left_play(id);
        }
        new_id
    }

    /// Legt einen Commander von `p` in die Befehlszone (CR 903.6).
    pub fn add_commander(
        &mut self,
        def: std::sync::Arc<crate::definition::CardDefinition>,
        p: PlayerId,
    ) -> CardId {
        let ident = self
            .cards
            .iter()
            .filter_map(|c| c.commander)
            .max()
            .map_or(0, |m| m + 1);
        let id = self.create_card(def, p, Zone::Command, false);
        self.card_mut(id).commander = Some(ident);
        id
    }

    /// CR 610.3: Die Quelle `host` hat das Battlefield verlassen – „bis … das
    /// Spielfeld verlässt“ endet: exilierte Karten kehren zurück, Effektobjekte enden.
    fn host_left_play(&mut self, host: CardId) {
        let returning: Vec<(CardId, Zone)> = self
            .exile
            .iter()
            .filter_map(|c| {
                let o = self.card(*c);
                o.exiled_until
                    .filter(|(h, _)| *h == host)
                    .map(|(_, z)| (*c, z))
            })
            .collect();
        let ending: Vec<CardId> = self
            .command
            .iter()
            .copied()
            .filter(|c| {
                self.card(*c).effect.is_some_and(|e| {
                    e.duration == crate::state::EffectDuration::UntilHostLeavesPlay
                        && e.host == Some(host)
                })
            })
            .collect();
        for c in ending {
            self.cease_to_exist(c);
        }
        let prev = self.effect_source.take();
        for (c, zone) in returning {
            // unter Kontrolle des Besitzers (CR 610.3c, 110.2)
            self.move_card(c, zone, true);
        }
        self.effect_source = prev;
    }

    /// CR 121: Karte ziehen. Leere Bibliothek → Markierung für CR 704.5b.
    pub fn draw(&mut self, p: PlayerId) -> Option<CardId> {
        let Some(top) = self.player(p).library.last().copied() else {
            self.player_mut(p).drew_from_empty_library = true;
            return None;
        };
        let new = self.move_card(top, Zone::Hand, true);
        self.push_event(Event::Drew {
            player: p,
            card: new,
        });
        self.player_mut(p).cards_drawn_this_turn += 1;
        let number = self.player(p).cards_drawn_this_turn;
        self.fire(crate::triggers::TriggerEvent::Drawn {
            card: new,
            player: p,
            number,
        });
        Some(new)
    }

    pub fn draw_n(&mut self, p: PlayerId, n: u32) {
        for _ in 0..n {
            self.draw(p);
        }
    }

    pub fn shuffle_library(&mut self, p: PlayerId) {
        let mut lib = std::mem::take(&mut self.player_mut(p).library);
        self.rng.shuffle(&mut lib);
        self.player_mut(p).library = lib;
    }

    pub fn gain_life(&mut self, p: PlayerId, n: i32) {
        if n <= 0 {
            return;
        }
        self.player_mut(p).life += n;
        self.player_mut(p).life_gained_this_turn += n;
        self.push_event(Event::LifeChanged {
            player: p,
            delta: n,
        });
        self.fire(crate::triggers::TriggerEvent::LifeGained {
            player: p,
            amount: n as u32,
        });
    }

    /// Tappt eine bleibende Karte; „wird getappt“-Trigger (CR 701.26a). Ins
    /// Spiel kommen getappt zählt nicht und läuft nicht hierüber.
    /// Enttappt eine bleibende Karte; Trigger `Untaps`.
    pub fn untap(&mut self, c: CardId) -> bool {
        if !self.card(c).tapped {
            return false;
        }
        self.card_mut(c).tapped = false;
        self.fire(crate::triggers::TriggerEvent::Untaps { card: c });
        true
    }

    pub fn tap(&mut self, c: CardId) -> bool {
        if self.card(c).tapped {
            return false;
        }
        self.card_mut(c).tapped = true;
        self.fire(crate::triggers::TriggerEvent::Taps { card: c });
        true
    }

    pub fn lose_life(&mut self, p: PlayerId, n: i32) {
        if n <= 0 {
            return;
        }
        self.player_mut(p).life -= n;
        self.player_mut(p).life_lost_this_turn += n;
        self.push_event(Event::LifeChanged {
            player: p,
            delta: -n,
        });
    }

    /// Schaden zufügen (CR 120). Behandelt Lifelink (CR 702.15) und
    /// Deathtouch-Markierung (CR 702.2) sowie Schaden an Planeswalkern/Battles.
    pub fn deal_damage(&mut self, source: CardId, target: Target, amount: u32) {
        self.deal_damage_ex(source, target, amount, false);
    }

    /// Wie [`GameState::deal_damage`], `combat` = Kampfschaden (für Trigger).
    pub fn deal_damage_ex(&mut self, source: CardId, target: Target, amount: u32, combat: bool) {
        if amount == 0 {
            return;
        }
        // CR 614/615: Ersatz- und Verhinderungseffekte
        let amount = self.replace_damage(source, target, amount, combat);
        if amount == 0 {
            return;
        }
        let src = self.chars(source);
        match target {
            Target::Player(p) => {
                if !self.player(p).is_in_game() {
                    return;
                }
                // CR 724.2: Kampfschaden einer Kreatur an den Monarchen → deren
                // Controller wird Monarch (vereinfacht ohne Stapel)
                if combat && self.monarch == Some(p) && src.types.is_creature() {
                    self.monarch = Some(self.card(source).controller);
                }
                // CR 903.10a: Kampfschaden von Commandern wird je Commander gezählt
                if combat
                    && self.rules.commander
                    && let Some(ident) = self.card(source).commander
                {
                    *self
                        .player_mut(p)
                        .commander_damage
                        .entry(ident)
                        .or_insert(0) += amount;
                }
                // CR 702.90b: Infect → Giftmarken statt Lebensverlust
                if src.has(&Keyword::Infect) {
                    self.player_mut(p).poison += amount;
                } else {
                    self.lose_life(p, amount as i32);
                }
                // CR 702.164c: Toxic bei Kampfschaden an Spielern
                if combat {
                    let toxic: u32 = src
                        .keywords
                        .iter()
                        .map(|k| if let Keyword::Toxic(n) = k { *n } else { 0 })
                        .sum();
                    self.player_mut(p).poison += toxic;
                }
            }
            Target::Card(c) => {
                if self.card(c).zone != Some(Zone::Battlefield) {
                    return;
                }
                let ch = self.chars(c);
                if ch.types.is_creature() {
                    self.this_turn.damaged.push((source, c));
                    // CR 702.80a/702.90c: Wither/Infect → -1/-1-Marken statt Schaden
                    if src.has(&Keyword::Infect) || src.has(&Keyword::Wither) {
                        self.add_counters(c, "M1M1", amount);
                    } else {
                        self.card_mut(c).damage += amount;
                    }
                    if src.has(&Keyword::Deathtouch) {
                        self.card_mut(c).deathtouch_damage = true;
                    }
                }
                if ch.types.has_core(forge_types::CoreType::Planeswalker) {
                    let o = self.card_mut(c);
                    let l = o.counter("LOYALTY");
                    o.counters
                        .insert("LOYALTY".into(), l.saturating_sub(amount));
                }
                if ch.types.has_core(forge_types::CoreType::Battle) {
                    let o = self.card_mut(c);
                    let d = o.counter("DEFENSE");
                    o.counters
                        .insert("DEFENSE".into(), d.saturating_sub(amount));
                }
            }
        }
        self.push_event(Event::Damage {
            source,
            target,
            amount,
        });
        match self.damage_batch.as_mut() {
            Some(b) => b.push((source, target, amount, combat)),
            None => {
                self.fire(crate::triggers::TriggerEvent::DamageDoneOnce {
                    target,
                    sources: vec![(source, amount)],
                    combat,
                });
                self.fire(crate::triggers::TriggerEvent::DamageDealtOnce {
                    source,
                    targets: vec![(target, amount)],
                    combat,
                });
            }
        }
        self.fire(crate::triggers::TriggerEvent::DamageDone {
            source,
            target,
            amount,
            combat,
        });
        if src.has(&Keyword::Lifelink) {
            let ctrl = self.card(source).controller;
            self.gain_life(ctrl, amount as i32);
        }
    }

    /// Setzt den Controller von `c` aus Kontrolleffekten neu (CR 613.1b, jüngster
    /// gewinnt). Ein Kontrollwechsel macht „krank“ (CR 302.6) und entfernt aus
    /// dem Kampf (CR 506.4).
    pub fn apply_control(&mut self, c: CardId) -> bool {
        let effects = self
            .control_effects
            .iter()
            .filter(|e| e.card == c)
            .map(|e| (e.timestamp, e.controller));
        let new = effects
            .chain(self.static_control_grants(c))
            .max_by_key(|(ts, _)| *ts)
            .map(|(_, p)| p)
            .unwrap_or(self.card(c).base_controller);
        if self.card(c).controller == new {
            return false;
        }
        let o = self.card_mut(c);
        o.controller = new;
        o.controlled_since_turn_start = false;
        if let Some(cb) = self.combat.as_mut() {
            cb.attackers.retain(|a| a.card != c);
            cb.blocks.retain(|(b, a)| *b != c && *a != c);
        }
        true
    }

    /// Statische Kontrolle (`GainControl$ You`) über `c`: (Zeitstempel der Quelle, Controller).
    fn static_control_grants(&self, c: CardId) -> Vec<(u64, PlayerId)> {
        let mut v = Vec::new();
        for &sid in self.battlefield.iter().chain(self.command.iter()) {
            let s = self.card(sid);
            let Some(zone) = s.zone else { continue };
            for cs in &s.compiled().statics {
                let crate::statics::CompiledStatic::Continuous(st) = cs.as_ref() else {
                    continue;
                };
                if !st.gain_control
                    || !st.effect_zones.contains(&zone)
                    || (st.affects_self_only && c != sid)
                    || (st.affects_attached && s.attached_to != Some(c))
                    || st
                        .affected
                        .as_ref()
                        .is_some_and(|a| !self.card_matches(c, a, sid, s.controller))
                {
                    continue;
                }
                let ctx = crate::expr::EvalCtx {
                    source: sid,
                    controller: s.controller,
                    x: 0,
                    targets: &[],
                    trig: None,
                    paid: &[],
                };
                if self.check_conditions(&st.conditions, &ctx) {
                    v.push((s.timestamp, s.controller));
                }
            }
        }
        v
    }

    /// Statische Kontrolle neu auswerten (in den SBA-Durchläufen): betroffene
    /// Objekte und solche mit abweichendem Controller.
    pub fn apply_static_control(&mut self) -> bool {
        let mut changed = false;
        let any_static = self.battlefield.iter().chain(self.command.iter()).any(|&sid| {
            self.card(sid).compiled().statics.iter().any(|cs| {
                matches!(cs.as_ref(), crate::statics::CompiledStatic::Continuous(st) if st.gain_control)
            })
        });
        let cards: Vec<CardId> = self
            .battlefield
            .iter()
            .copied()
            .filter(|&c| any_static || self.card(c).controller != self.card(c).base_controller)
            .collect();
        for c in cards {
            changed |= self.apply_control(c);
        }
        changed
    }

    /// Beendet abgelaufene Kontrolleffekte (CR 611.2b) und setzt Controller neu.
    pub fn expire_control_effects(&mut self, at_cleanup: bool) -> bool {
        if self.control_effects.is_empty() {
            return false;
        }
        let (turn, active) = (self.turn, self.active);
        let before = self.control_effects.clone();
        let keep: Vec<bool> = before
            .iter()
            .map(|e| {
                let src = self.card(e.source);
                !(self.card(e.card).zone != Some(Zone::Battlefield)
                    || (at_cleanup && e.until_eot)
                    || (at_cleanup
                        && e.until_end_of_next_turn
                        && active == e.source_controller
                        && turn > e.created_turn)
                    || (e.leaves_play && src.zone != Some(Zone::Battlefield))
                    || (e.lose_control
                        && (src.zone != Some(Zone::Battlefield)
                            || src.controller != e.source_controller)))
            })
            .collect();
        if keep.iter().all(|k| *k) {
            return false;
        }
        let mut i = 0;
        self.control_effects.retain(|_| {
            i += 1;
            keep[i - 1]
        });
        let mut changed = false;
        for (e, k) in before.iter().zip(&keep) {
            if !k && self.card(e.card).zone == Some(Zone::Battlefield) {
                changed |= self.apply_control(e.card);
            }
        }
        changed
    }

    /// Bündelt Zonenwechsel (Java `CardZoneTable`) bis zum passenden
    /// [`GameState::end_zone_batch`]; dann löst `ChangesZoneAll` einmal aus.
    pub fn begin_zone_batch(&mut self) {
        self.zone_batch_depth += 1;
        if self.zone_batch.is_none() {
            self.zone_batch = Some(Vec::new());
        }
    }

    pub fn end_zone_batch(&mut self) {
        self.zone_batch_depth = self.zone_batch_depth.saturating_sub(1);
        if self.zone_batch_depth > 0 {
            return;
        }
        if let Some(moves) = self.zone_batch.take()
            && !moves.is_empty()
        {
            self.fire(crate::triggers::TriggerEvent::ChangesZoneAll { moves });
        }
    }

    /// Beginnt gleichzeitigen Schaden (CR 120.2, 510.2) – `DamageDoneOnce`
    /// löst danach einmal je Empfänger aus.
    pub fn begin_damage_batch(&mut self) {
        if self.damage_batch.is_none() {
            self.damage_batch = Some(Vec::new());
        }
    }

    pub fn end_damage_batch(&mut self) {
        let Some(batch) = self.damage_batch.take() else {
            return;
        };
        let mut targets: Vec<Target> = Vec::new();
        for (_, t, _, _) in &batch {
            if !targets.contains(t) {
                targets.push(*t);
            }
        }
        for t in targets {
            let entries: Vec<_> = batch.iter().filter(|e| e.1 == t).collect();
            let combat = entries.iter().any(|e| e.3);
            let sources = entries.iter().map(|e| (e.0, e.2)).collect();
            self.fire(crate::triggers::TriggerEvent::DamageDoneOnce {
                target: t,
                sources,
                combat,
            });
        }
        let mut sources: Vec<CardId> = Vec::new();
        for (s, _, _, _) in &batch {
            if !sources.contains(s) {
                sources.push(*s);
            }
        }
        for s in sources {
            let entries: Vec<_> = batch.iter().filter(|e| e.0 == s).collect();
            self.fire(crate::triggers::TriggerEvent::DamageDealtOnce {
                source: s,
                targets: entries.iter().map(|e| (e.1, e.2)).collect(),
                combat: entries.iter().any(|e| e.3),
            });
        }
    }

    /// CR 701.8: Zerstören. Unzerstörbare bleiben (CR 702.12b).
    pub fn destroy(&mut self, c: CardId) -> bool {
        self.destroy_ex(c, true)
    }

    /// Zerstören; `can_regenerate = false` bei „kann nicht regeneriert werden“.
    pub fn destroy_ex(&mut self, c: CardId, can_regenerate: bool) -> bool {
        if self.card(c).zone != Some(Zone::Battlefield)
            || self.has_keyword(c, &Keyword::Indestructible)
        {
            return false;
        }
        // CR 701.15: Regeneration ersetzt das Zerstören
        if can_regenerate && self.card(c).regen_shields > 0 {
            let o = self.card_mut(c);
            o.regen_shields -= 1;
            o.damage = 0;
            o.deathtouch_damage = false;
            self.tap(c);
            if let Some(cb) = self.combat.as_mut() {
                cb.attackers.retain(|a| a.card != c);
                cb.blocks.retain(|(b, a)| *b != c && *a != c);
            }
            return false;
        }
        self.push_event(crate::state::Event::Destroyed(c));
        self.move_card(c, Zone::Graveyard, true);
        true
    }

    /// CR 701.28/712: Transformieren – dasselbe Objekt zeigt die andere Seite
    /// und erhält einen neuen Zeitstempel. Nur echte doppelseitige Karten.
    pub fn transform(&mut self, c: CardId) -> bool {
        let o = self.card(c);
        if o.def.script.split_mode != forge_script::SplitMode::DoubleFaced
            || o.def.faces.get(1).is_none_or(Option::is_none)
        {
            return false;
        }
        let ts = self.timestamp();
        let o = self.card_mut(c);
        o.face = 1 - o.face.min(1);
        o.timestamp = ts;
        true
    }

    /// CR 701.21: Opfern – vom Battlefield in den Friedhof des Besitzers, ohne
    /// „zerstören“ (Unzerstörbarkeit und Regeneration greifen nicht).
    pub fn sacrifice(&mut self, c: CardId) -> bool {
        if self.card(c).zone != Some(Zone::Battlefield) {
            return false;
        }
        let player = self.card(c).controller;
        self.push_event(crate::state::Event::Sacrificed(c));
        self.move_card(c, Zone::Graveyard, true);
        self.fire(crate::triggers::TriggerEvent::Sacrificed { old: c, player });
        true
    }

    /// CR 701.9: Abwerfen (aus der Hand in den Friedhof); Trigger `Discarded`.
    pub fn discard(&mut self, c: CardId) -> CardId {
        let player = self.card(c).owner;
        let new = self.move_card(c, Zone::Graveyard, true);
        self.fire(crate::triggers::TriggerEvent::Discarded {
            old: c,
            new,
            player,
        });
        new
    }

    /// Auf den Friedhof legen ohne „zerstören“ (SBA 704.5f).
    pub fn put_into_graveyard(&mut self, c: CardId) {
        self.move_card(c, Zone::Graveyard, true);
    }

    pub fn add_counters(&mut self, c: CardId, kind: &str, n: u32) {
        if n == 0 {
            return;
        }
        let before = self.card(c).counter(kind);
        *self
            .card_mut(c)
            .counters
            .entry(kind.to_string())
            .or_insert(0) += n;
        self.fire_counters_added(c, kind, before, n);
        // Wer legt die Marken? Controller der Effektquelle, sonst der des Objekts.
        let player = self
            .effect_source
            .map(|s| self.card(s).controller)
            .unwrap_or(self.card(c).controller);
        self.fire(crate::triggers::TriggerEvent::CounterAddedOnce {
            card: c,
            kind: kind.to_string(),
            amount: n,
            player,
        });
    }

    /// Entfernt bis zu `n` Marken der Art; liefert die Zahl entfernter Marken.
    pub fn remove_counters(&mut self, c: CardId, kind: &str, n: u32) -> u32 {
        let o = self.card_mut(c);
        let have = o.counter(kind);
        let k = have.min(n);
        if have - k == 0 {
            o.counters.remove(kind);
        } else {
            o.counters.insert(kind.to_string(), have - k);
        }
        k
    }

    /// Trigger `CounterAdded` je einzelner Marke (Java `Card.addCounter`).
    pub(crate) fn fire_counters_added(&mut self, c: CardId, kind: &str, before: u32, n: u32) {
        for i in 1..=n {
            self.fire(crate::triggers::TriggerEvent::CounterAdded {
                card: c,
                kind: kind.to_string(),
                total: before + i,
            });
        }
    }

    /// Kann `p` eine Karte ziehen? (nur informativ für KI/GUI)
    pub fn library_size(&self, p: PlayerId) -> usize {
        self.player(p).library.len()
    }
}
