//! Zustandsbasierte Aktionen (CR 704).
//!
//! Java-Vorbild: `GameAction.checkStateEffects`.
//! Alle zutreffenden SBAs werden gleichzeitig ausgeführt (CR 704.3), dann
//! wird erneut geprüft, bis keine mehr zutreffen.
//!
//! Umgesetzt: 704.5a, b, c, d, f, g, h, i, j, k, m, n, p, q, v. Siehe `docs/03-regelstand.md`.

use crate::decision::{CardChoiceReason, Choice, Decision};
use crate::ids::{CardId, PlayerId};
use crate::state::{Choices, Event, GameState, LossReason, Res};
use crate::zone::Zone;
use forge_types::CoreType;

impl GameState {
    /// Führt SBAs aus, bis keine mehr zutreffen. Liefert `true`, wenn
    /// mindestens eine ausgeführt wurde.
    pub fn check_state_based_actions(&mut self, ch: &mut Choices) -> Res<bool> {
        let mut any = false;
        loop {
            if self.winner.is_some() {
                return Ok(any);
            }
            let performed = self.sba_pass(ch)?;
            if !performed {
                return Ok(any);
            }
            any = true;
        }
    }

    fn sba_pass(&mut self, ch: &mut Choices) -> Res<bool> {
        let mut performed = false;

        // --- Spieler (704.5a–c) ---
        let mut losers: Vec<(PlayerId, LossReason)> = Vec::new();
        for p in &self.players {
            if !p.is_in_game() {
                continue;
            }
            if p.life <= 0 {
                losers.push((p.id, LossReason::LifeZero));
            } else if p.drew_from_empty_library {
                losers.push((p.id, LossReason::DrewFromEmptyLibrary));
            } else if p.poison >= self.rules.poison_to_lose {
                losers.push((p.id, LossReason::Poison));
            }
        }

        // --- Objekte (gleichzeitig ermitteln) ---
        let mut to_graveyard: Vec<CardId> = Vec::new();
        let mut to_destroy: Vec<CardId> = Vec::new();
        let mut cease: Vec<CardId> = Vec::new();

        // 704.5d: Tokens außerhalb des Battlefields hören auf zu existieren.
        for c in &self.cards {
            if c.token
                && c.zone.is_some()
                && c.zone != Some(Zone::Battlefield)
                && c.zone != Some(Zone::Stack)
            {
                cease.push(c.id);
            }
        }

        for &id in &self.battlefield {
            let chars = self.chars(id);
            let card = self.card(id);
            if chars.types.is_creature() {
                let t = chars.toughness.unwrap_or(0);
                if t <= 0 {
                    // 704.5f
                    to_graveyard.push(id);
                } else if card.damage as i32 >= t || (card.damage > 0 && card.deathtouch_damage) {
                    // 704.5g / 704.5h – Zerstören (Unzerstörbar verhindert)
                    to_destroy.push(id);
                }
            }
            // 704.5i
            if chars.types.has_core(CoreType::Planeswalker) && card.counter("LOYALTY") == 0 {
                to_graveyard.push(id);
            }
            // 704.5v
            if chars.types.has_core(CoreType::Battle) && card.counter("DEFENSE") == 0 {
                to_graveyard.push(id);
            }
        }

        // 704.5q: +1/+1- und -1/-1-Marken heben sich auf.
        let mut annihilate: Vec<(CardId, u32)> = Vec::new();
        for &id in &self.battlefield {
            let c = self.card(id);
            let n = c.counter("P1P1").min(c.counter("M1M1"));
            if n > 0 {
                annihilate.push((id, n));
            }
        }

        // 704.5j: Legendenregel – Spieler wählt, welche bleibt.
        let mut legend_choices: Vec<(PlayerId, Vec<CardId>)> = Vec::new();
        for p in self.apnap() {
            let mut by_name: std::collections::BTreeMap<String, Vec<CardId>> = Default::default();
            for id in self.battlefield_of(p) {
                let ch = self.chars(id);
                if ch.types.is_legendary() {
                    by_name.entry(ch.name).or_default().push(id);
                }
            }
            for (_, ids) in by_name {
                if ids.len() > 1 {
                    legend_choices.push((p, ids));
                }
            }
        }
        for (p, ids) in legend_choices {
            let keep = match ch.ask(Decision::ChooseCards {
                player: p,
                reason: CardChoiceReason::LegendRule,
                options: ids.clone(),
                min: 1,
                max: 1,
            })? {
                Choice::Cards(c) if c.len() == 1 => c[0],
                _ => ids[0],
            };
            to_graveyard.extend(ids.into_iter().filter(|c| *c != keep));
        }

        // 704.5k: Weltregel – alle außer der jüngsten World-Permanent.
        let worlds: Vec<CardId> = self
            .battlefield
            .iter()
            .copied()
            .filter(|c| {
                self.chars(*c)
                    .types
                    .has_super(forge_types::Supertype::World)
            })
            .collect();
        if worlds.len() > 1 {
            let newest = worlds
                .iter()
                .map(|c| self.card(*c).timestamp)
                .max()
                .unwrap_or(0);
            let newest_ids: Vec<CardId> = worlds
                .iter()
                .copied()
                .filter(|c| self.card(*c).timestamp == newest)
                .collect();
            // Gleich alt (gleichzeitig ins Spiel gekommen) → alle gehen.
            if newest_ids.len() == 1 {
                to_graveyard.extend(worlds.into_iter().filter(|c| *c != newest_ids[0]));
            } else {
                to_graveyard.extend(worlds);
            }
        }

        // 704.5m/n/p: Angelegte Objekte prüfen.
        let mut unattach: Vec<CardId> = Vec::new();
        for &id in &self.battlefield {
            let c = self.card(id);
            let ch = self.chars(id);
            let target_ok = |t: CardId| self.card(t).zone == Some(Zone::Battlefield) && t != id;
            if ch.types.has_subtype("Aura") {
                // 704.5m: Aura an nichts oder an illegalem Objekt → Friedhof
                let legal = c.attached_to.is_some_and(|t| {
                    target_ok(t)
                        && c.compiled()
                            .enchant
                            .as_ref()
                            .is_none_or(|v| self.card_matches(t, v, id, c.controller))
                });
                if !legal {
                    to_graveyard.push(id);
                }
            } else if let Some(t) = c.attached_to {
                // 704.5n: Ausrüstung an Nicht-Kreatur/weg → lösen; 704.5p: andere → lösen
                let is_equipment = ch.types.has_subtype("Equipment");
                if !is_equipment || !target_ok(t) || !self.is_creature(t) {
                    unattach.push(id);
                }
            }
        }

        // --- Ausführen ---
        for (p, reason) in losers {
            self.player_mut(p).lost = Some(reason);
            self.push_event(Event::PlayerLost { player: p, reason });
            self.remove_player_objects(p);
            performed = true;
        }
        for id in cease {
            self.cease_to_exist(id);
            performed = true;
        }
        for id in unattach {
            self.card_mut(id).attached_to = None;
            performed = true;
        }
        for (id, n) in annihilate {
            let c = self.card_mut(id);
            *c.counters.get_mut("P1P1").expect("geprüft") -= n;
            *c.counters.get_mut("M1M1").expect("geprüft") -= n;
            c.counters.retain(|_, v| *v > 0);
            performed = true;
        }
        to_graveyard.sort();
        to_graveyard.dedup();
        // CR 603.10a: alle gleichzeitig Sterbenden „sehen“ einander.
        self.lki_battlefield = Some(self.battlefield.clone());
        for id in to_graveyard {
            if self.card(id).zone == Some(Zone::Battlefield) {
                self.put_into_graveyard(id);
                performed = true;
            }
        }
        for id in to_destroy {
            if self.card(id).zone == Some(Zone::Battlefield) && self.destroy(id) {
                performed = true;
            }
        }
        self.lki_battlefield = None;

        // Spielende (CR 104.2a): nur noch ein Spieler übrig.
        let remaining: Vec<PlayerId> = self
            .players
            .iter()
            .filter(|p| p.is_in_game())
            .map(|p| p.id)
            .collect();
        if remaining.len() <= 1 {
            let w = remaining.first().copied();
            self.winner = Some(w);
            self.push_event(Event::GameWon(w));
            performed = true;
        }
        Ok(performed)
    }

    pub(crate) fn cease_to_exist(&mut self, id: CardId) {
        let c = self.card(id);
        let (zone, owner) = (c.zone, c.owner);
        if let Some(z) = zone {
            match z {
                Zone::Exile => self.exile.retain(|x| *x != id),
                Zone::Command => self.command.retain(|x| *x != id),
                Zone::Battlefield => self.battlefield.retain(|x| *x != id),
                Zone::Stack => {}
                z => self.player_mut(owner).zone_mut(z).retain(|x| *x != id),
            }
        }
        self.card_mut(id).zone = None;
    }

    /// CR 800.4a: Verlässt ein Spieler das Spiel, verlassen alle Objekte, die
    /// er besitzt, das Spiel; Stapelobjekte, die er kontrolliert, hören auf zu
    /// existieren.
    pub fn remove_player_objects(&mut self, p: PlayerId) {
        let owned: Vec<CardId> = self
            .cards
            .iter()
            .filter(|c| c.owner == p && c.zone.is_some())
            .map(|c| c.id)
            .collect();
        for id in owned {
            self.cease_to_exist(id);
        }
        self.stack.retain(|s| s.controller != p);
        if let Some(c) = self.combat.as_mut() {
            c.attackers.retain(|a| a.defender != p);
        }
    }
}
