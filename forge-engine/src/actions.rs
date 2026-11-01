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
            damage: 0,
            deathtouch_damage: false,
            counters: Default::default(),
            timestamp: ts,
            moved_to: None,
            base_pt: None,
            cast_x: 0,
            remembered: Vec::new(),
            effect: None,
            attached_to: None,
            regen_shields: 0,
        });
        self.zone_list_mut(zone, owner).push(id);
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
        let (to, mods) = self.replace_move(id, from, to);
        self.remove_from_zone(id);
        let old = self.card(id).clone();
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
            face: 0,
            owner: old.owner,
            controller,
            zone: Some(to),
            token: old.token,
            tapped: false,
            controlled_since_turn_start: false,
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
            effect: None,
            attached_to: None,
            regen_shields: 0,
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
        new_id
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
        self.push_event(Event::LifeChanged {
            player: p,
            delta: n,
        });
    }

    pub fn lose_life(&mut self, p: PlayerId, n: i32) {
        if n <= 0 {
            return;
        }
        self.player_mut(p).life -= n;
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
        let src = self.chars(source);
        match target {
            Target::Player(p) => {
                if !self.player(p).is_in_game() {
                    return;
                }
                self.lose_life(p, amount as i32);
            }
            Target::Card(c) => {
                if self.card(c).zone != Some(Zone::Battlefield) {
                    return;
                }
                let ch = self.chars(c);
                if ch.types.is_creature() {
                    let o = self.card_mut(c);
                    o.damage += amount;
                    if src.has(&Keyword::Deathtouch) {
                        o.deathtouch_damage = true;
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
            o.tapped = true;
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

    /// Auf den Friedhof legen ohne „zerstören“ (SBA 704.5f).
    pub fn put_into_graveyard(&mut self, c: CardId) {
        self.move_card(c, Zone::Graveyard, true);
    }

    pub fn add_counters(&mut self, c: CardId, kind: &str, n: u32) {
        if n == 0 {
            return;
        }
        *self
            .card_mut(c)
            .counters
            .entry(kind.to_string())
            .or_insert(0) += n;
    }

    /// Kann `p` eine Karte ziehen? (nur informativ für KI/GUI)
    pub fn library_size(&self, p: PlayerId) -> usize {
        self.player(p).library.len()
    }
}
