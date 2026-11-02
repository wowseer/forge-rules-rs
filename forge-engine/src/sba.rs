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
        let mut any = self.run_pending_etb(ch)?;
        // Kontrolleffekte „solange …“ enden sofort (CR 611.2b), vor den SBAs geprüft.
        any |= self.expire_control_effects(false);
        // Statische Kontrolle (Layer 2); eine Änderung beendet den Schritt wie eine SBA.
        any |= self.apply_static_control();
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

    /// `ETBReplacement:Other` (z. B. „wähle beim Ankommen einen Kreaturtyp“).
    /// Vereinfachung: läuft direkt nach dem Zonenwechsel vor den nächsten SBAs
    /// statt als echter Ersatzeffekt während des Wechsels (CR 614.12).
    fn run_pending_etb(&mut self, ch: &mut Choices) -> Res<bool> {
        let mut any = false;
        while !self.pending_etb.is_empty() {
            let c = self.pending_etb.remove(0);
            if self.card(c).zone != Some(Zone::Battlefield) {
                continue;
            }
            let controller = self.card(c).controller;
            for (ab, optional) in self.card(c).compiled().etb_other.clone() {
                if optional
                    && !matches!(
                        ch.ask(crate::decision::Decision::Confirm {
                            player: controller,
                            prompt: ab.description.clone(),
                        })?,
                        Choice::Yes
                    )
                {
                    continue;
                }
                let ctx = crate::effects::resolve::ResolveCtx {
                    source: c,
                    controller,
                    x: self.card(c).cast_x,
                    trig: None,
                    paid: Vec::new(),
                };
                let was_copy = self.card(c).copy.is_some();
                self.apply_chain(&ab, &[], &ctx, ch)?;
                any = true;
                // Als Kopie angekommen (CR 707, 614.1c): eigene ETB-Trigger der Kopie
                if !was_copy && self.card(c).copy.is_some() {
                    self.fire_only_for(
                        crate::triggers::TriggerEvent::ChangesZone {
                            old: c,
                            new: c,
                            from: None,
                            to: Zone::Battlefield,
                        },
                        c,
                    );
                }
            }
        }
        Ok(any)
    }

    fn sba_pass(&mut self, ch: &mut Choices) -> Res<bool> {
        // Sammelphase ist rein lesend → Eigenschaften cachen (vor dem Ausführen freigeben).
        let cache = crate::state::CharsCache::enter();
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

        // 704.5s / 714.4: Saga mit Lore-Marken ≥ letztem Kapitel, deren
        // Kapitelfähigkeit weder ausgelöst wartet noch auf dem Stapel liegt.
        let mut to_sacrifice: Vec<CardId> = Vec::new();
        for &id in &self.battlefield {
            let Some(last) = self.card(id).compiled().final_chapter else {
                continue;
            };
            if !self.is_saga(id) || self.card(id).counter("LORE") < last {
                continue;
            }
            let pending = self
                .pending_triggers
                .iter()
                .any(|t| t.source == id && t.trigger.chapter);
            let on_stack = self.stack.iter().any(|s| {
                s.source == id
                    && matches!(
                        s.kind,
                        crate::state::StackKind::Trigger { chapter: true, .. }
                    )
            });
            if !pending && !on_stack {
                to_sacrifice.push(id);
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

        drop(cache);
        // --- Ausführen --- (gleichzeitig: ein Zonenwechsel-Ereignis)
        self.begin_zone_batch();
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
        for id in to_sacrifice {
            if self.sacrifice(id) {
                performed = true;
            }
        }
        for id in to_destroy {
            // Auch eine Regeneration (CR 701.15) verändert den Zustand.
            if self.card(id).zone == Some(Zone::Battlefield) {
                let shields = self.card(id).regen_shields;
                let destroyed = self.destroy(id);
                if destroyed || self.card(id).regen_shields < shields {
                    performed = true;
                }
            }
        }
        self.lki_battlefield = None;
        self.end_zone_batch();

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
            .live_cards()
            .map(|id| self.card(id))
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
