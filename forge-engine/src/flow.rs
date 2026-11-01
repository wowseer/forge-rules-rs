//! Spielablauf: Aufbau, Mulligan, Zugstruktur, Priorität (CR 103, 117, 500–514).
//!
//! Java-Vorbild: `PhaseHandler` (`onPhaseBegin`, `passPriority`,
//! `mainLoopStep`), `Match.startGame`, `MulliganService`.
//!
//! [`GameState::step_once`] führt genau einen atomaren Übergang aus. Danach
//! ist der Zustand ein gültiger Wiederaufsetzpunkt für `Engine`.

use crate::decision::{Action, CardChoiceReason, Choice, Decision};
use crate::ids::PlayerId;
use crate::state::{Choices, Combat, Duration, Event, Flow, GameState, Res};
use crate::turn::Step;
use crate::zone::Zone;

impl GameState {
    pub fn step_once(&mut self, ch: &mut Choices) -> Res<()> {
        if self.winner.is_some() {
            self.flow = Flow::GameOver;
            return Ok(());
        }
        match self.flow {
            Flow::Setup => self.setup(),
            Flow::Mulligan { player } => self.mulligan(player, ch)?,
            Flow::BeginStep => self.begin_step(ch)?,
            Flow::Priority { player, passes } => self.priority(player, passes, ch)?,
            Flow::EndStep => self.end_step(),
            Flow::GameOver => {}
        }
        if self.winner.is_some() {
            self.flow = Flow::GameOver;
        }
        Ok(())
    }

    /// CR 103.2–103.4: Mischen, Starthand ziehen.
    fn setup(&mut self) {
        for p in self.players_from(self.starting_player) {
            self.shuffle_library(p);
            let n = self.rules.starting_hand;
            self.draw_n(p, n);
        }
        self.flow = Flow::Mulligan {
            player: self.starting_player,
        };
    }

    /// CR 103.5: London-Mulligan.
    fn mulligan(&mut self, p: PlayerId, ch: &mut Choices) -> Res<()> {
        let taken = self.player(p).mulligans;
        match ch.ask(Decision::Mulligan {
            player: p,
            mulligans_taken: taken,
        })? {
            Choice::Mulligan if (taken as usize) < self.rules.starting_hand as usize => {
                let hand = std::mem::take(&mut self.player_mut(p).hand);
                for c in hand {
                    self.move_card(c, Zone::Library, true);
                }
                self.shuffle_library(p);
                let n = self.rules.starting_hand;
                self.draw_n(p, n);
                self.player_mut(p).mulligans += 1;
                return Ok(());
            }
            _ => {
                let count = taken.min(self.player(p).hand.len() as u32);
                if count > 0 {
                    let hand = self.player(p).hand.clone();
                    let chosen = match ch.ask(Decision::MulliganBottom {
                        player: p,
                        hand: hand.clone(),
                        count,
                    })? {
                        Choice::Cards(c) => c,
                        _ => hand[..count as usize].to_vec(),
                    };
                    for c in chosen {
                        self.move_card(c, Zone::Library, false);
                    }
                }
                self.player_mut(p).kept_hand = true;
            }
        }
        let next = self
            .players_from(p)
            .into_iter()
            .find(|x| !self.player(*x).kept_hand);
        match next {
            Some(n) => self.flow = Flow::Mulligan { player: n },
            None => self.start_turn(self.starting_player, 1),
        }
        Ok(())
    }

    fn start_turn(&mut self, p: PlayerId, number: u32) {
        self.turn = number;
        self.active = p;
        self.step = Step::Untap;
        for pl in &mut self.players {
            pl.lands_played_this_turn = 0;
        }
        self.push_event(Event::TurnBegan {
            turn: number,
            player: p,
        });
        self.flow = Flow::BeginStep;
    }

    /// Rundenbasierte Aktionen zu Beginn eines Schritts (CR 502–514).
    fn begin_step(&mut self, ch: &mut Choices) -> Res<()> {
        let ap = self.active;
        // CR 508.8 / 510.4: diese Schritte finden ohne Angreifer bzw.
        // Erstschlag gar nicht statt – sie werden auch nicht protokolliert.
        let skipped = match self.step {
            Step::DeclareBlockers | Step::CombatDamage => !self.has_attackers(),
            Step::FirstStrikeDamage => !self.has_attackers() || !self.needs_first_strike_step(),
            _ => false,
        };
        if skipped {
            self.flow = Flow::EndStep;
            return Ok(());
        }
        self.push_event(Event::StepBegan(self.step));
        self.fire(crate::triggers::TriggerEvent::StepBegan {
            step: self.step,
            active: ap,
        });
        let mut gets_priority = self.step.has_priority();
        match self.step {
            Step::Untap => {
                // „bis zu deinem nächsten Zug“ endet mit Zugbeginn
                self.end_effect_objects(|info, ap_is_owner, turn| {
                    info.duration == crate::state::EffectDuration::UntilYourNextTurn
                        && ap_is_owner
                        && turn > info.created_turn
                });
                // CR 302.6: Kontrolle seit Zugbeginn
                for c in self.battlefield_of(ap).collect::<Vec<_>>() {
                    let o = self.card_mut(c);
                    o.controlled_since_turn_start = true;
                    // CR 502.3
                    o.tapped = false;
                }
            }
            Step::Upkeep => {}
            Step::Draw => {
                // CR 103.8a
                let skip = self.turn == 1
                    && ap == self.starting_player
                    && self.rules.first_player_skips_draw;
                if !skip {
                    self.draw(ap);
                }
            }
            Step::Main1 | Step::Main2 | Step::End => {}
            Step::BeginCombat => {
                self.combat = Some(Combat::default());
            }
            Step::DeclareAttackers => {
                self.declare_attackers(ch)?;
                // CR 508.8: ohne Angreifer gibt es hier trotzdem Priorität,
                // aber Block- und Schadensschritte entfallen.
            }
            Step::DeclareBlockers => {
                if self.has_attackers() {
                    self.declare_blockers(ch)?;
                } else {
                    gets_priority = false;
                }
            }
            Step::FirstStrikeDamage => {
                if self.has_attackers() && self.needs_first_strike_step() {
                    self.combat_damage(true, ch)?;
                } else {
                    gets_priority = false;
                }
            }
            Step::CombatDamage => {
                if self.has_attackers() {
                    self.combat_damage(false, ch)?;
                } else {
                    gets_priority = false;
                }
            }
            Step::EndCombat => {}
            Step::Cleanup => {
                // CR 514.1: auf Handgröße abwerfen
                let hand = self.player(ap).hand.clone();
                let max = self.rules.max_hand_size as usize;
                if hand.len() > max {
                    let n = (hand.len() - max) as u32;
                    let chosen = match ch.ask(Decision::ChooseCards {
                        player: ap,
                        reason: CardChoiceReason::DiscardToHandSize,
                        options: hand.clone(),
                        min: n,
                        max: n,
                    })? {
                        Choice::Cards(c) => c,
                        _ => hand[..n as usize].to_vec(),
                    };
                    for c in chosen {
                        self.move_card(c, Zone::Graveyard, true);
                    }
                }
                // CR 514.2: Schaden entfernen, „bis Ende des Zuges“ endet
                for c in self.battlefield.clone() {
                    let o = self.card_mut(c);
                    o.damage = 0;
                    o.deathtouch_damage = false;
                    o.regen_shields = 0;
                }
                self.effects
                    .retain(|e| e.duration != Duration::UntilEndOfTurn);
                // „ThisTurn“-verzögerte Trigger enden (Java clearThisTurnDelayedTrigger).
                let turn = self.turn;
                self.delayed_triggers
                    .retain(|d| d.until_turn.is_none_or(|u| u > turn));
                self.end_effect_objects(|info, ap_is_owner, turn| match info.duration {
                    crate::state::EffectDuration::EndOfTurn => true,
                    crate::state::EffectDuration::UntilTheEndOfYourNextTurn => {
                        ap_is_owner && turn > info.created_turn
                    }
                    _ => false,
                });
                // CR 514.3a: SBA/Trigger → doch Priorität und neuer Cleanup
                let sba = self.check_state_based_actions(ch)?;
                if sba || !self.pending_triggers.is_empty() {
                    gets_priority = true;
                    self.repeat_cleanup = true;
                }
            }
        }
        self.flow = if gets_priority {
            Flow::Priority {
                player: ap,
                passes: 0,
            }
        } else {
            Flow::EndStep
        };
        Ok(())
    }

    /// CR 117: Priorität. Vor jeder Vergabe werden SBAs geprüft (CR 117.5).
    fn priority(&mut self, p: PlayerId, passes: u8, ch: &mut Choices) -> Res<()> {
        if self.check_state_based_actions(ch)? {
            return Ok(());
        }
        // CR 117.5: danach ausgelöste Trigger auf den Stapel, dann erneut prüfen.
        if self.put_triggers_on_stack(ch)? {
            return Ok(());
        }
        if !self.player(p).is_in_game() {
            self.flow = Flow::Priority {
                player: self.next_player(p),
                passes,
            };
            return Ok(());
        }
        let actions = self.legal_actions(p);
        let action = match ch.ask(Decision::Priority { player: p, actions })? {
            Choice::Action(a) => a,
            _ => Action::Pass,
        };
        if action == Action::Pass {
            let passes = passes + 1;
            if passes as usize >= self.players_in_game() {
                if self.stack.is_empty() {
                    // CR 500.2: Schritt endet
                    self.flow = Flow::EndStep;
                } else {
                    // CR 117.4 / 608.1
                    self.resolve_top(ch)?;
                    self.flow = Flow::Priority {
                        player: self.active,
                        passes: 0,
                    };
                }
            } else {
                self.flow = Flow::Priority {
                    player: self.next_player(p),
                    passes,
                };
            }
        } else {
            self.perform_action(p, action, ch)?;
            // CR 117.3c: wer handelt, erhält wieder Priorität
            self.flow = Flow::Priority {
                player: p,
                passes: 0,
            };
        }
        Ok(())
    }

    fn end_step(&mut self) {
        // CR 500.4
        for p in &mut self.players {
            p.mana_pool.clear();
        }
        if self.step == Step::Cleanup && self.repeat_cleanup {
            self.repeat_cleanup = false;
            self.flow = Flow::BeginStep;
            return;
        }
        if self.step == Step::EndCombat {
            self.combat = None; // CR 511.3
        }
        match self.step.next() {
            Some(s) => {
                self.step = s;
                self.flow = Flow::BeginStep;
            }
            None => {
                let next = self.next_player(self.active);
                let n = self.turn + 1;
                self.start_turn(next, n);
            }
        }
    }
}

impl GameState {
    /// Entfernt Effektobjekte, deren Dauer endet (`ends(info, gehört dem aktiven Spieler, Zug)`).
    pub(crate) fn end_effect_objects(
        &mut self,
        ends: impl Fn(&crate::state::EffectInfo, bool, u32) -> bool,
    ) {
        let ap = self.active;
        let turn = self.turn;
        let done: Vec<crate::ids::CardId> = self
            .command
            .iter()
            .copied()
            .filter(|c| {
                let o = self.card(*c);
                o.effect
                    .as_ref()
                    .is_some_and(|i| ends(i, o.owner == ap, turn))
            })
            .collect();
        for c in done {
            self.cease_to_exist(c);
        }
    }
}
