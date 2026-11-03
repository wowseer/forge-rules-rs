//! Einfache, deterministische Test-KI. Kein Ersatz für `forge-ai`, sondern
//! ein Werkzeug, um Partien vollständig durchzuspielen (Smoke-Tests,
//! Fuzzing der Engine). Entscheidet zufällig, aber seedbar.

use crate::decision::{Action, CardChoiceReason, Choice, Decision, PayAction, Target};
use crate::engine::Controller;
use crate::rng::Rng;
use crate::state::GameState;

pub struct SimpleAi {
    rng: Rng,
}

impl SimpleAi {
    pub fn new(seed: u64) -> SimpleAi {
        SimpleAi {
            rng: Rng::new(seed),
        }
    }

    fn pick<T: Clone>(&mut self, v: &[T]) -> T {
        v[self.rng.below(v.len() as u64) as usize].clone()
    }
}

impl Controller for SimpleAi {
    fn decide(&mut self, state: &GameState, d: &Decision) -> Choice {
        match d {
            Decision::Mulligan { .. } => Choice::Keep,
            Decision::MulliganBottom { hand, count, .. } => {
                Choice::Cards(hand[..*count as usize].to_vec())
            }
            Decision::Priority { actions, player } => {
                // Länder zuerst, dann Zauber/Fähigkeiten (außer reinen Mana-Fähigkeiten), sonst passen.
                if let Some(a) = actions.iter().find(|a| matches!(a, Action::PlayLand(_))) {
                    return Choice::Action(*a);
                }
                let useful: Vec<Action> = actions
                    .iter()
                    .copied()
                    .filter(|a| match a {
                        Action::Cast(_) | Action::CastFaceDown(_) | Action::TurnFaceUp(_) => true,
                        Action::Activate(c, i) => {
                            !state.card(*c).compiled().activated[*i].is_mana_ability()
                        }
                        _ => false,
                    })
                    .collect();
                // nicht in jeder Prioritätsrunde handeln → Spiel schreitet voran
                if !useful.is_empty()
                    && state.stack.is_empty()
                    && state.active == *player
                    && self.rng.below(4) != 0
                {
                    return Choice::Action(self.pick(&useful));
                }
                Choice::Action(Action::Pass)
            }
            Decision::ChooseTargets {
                candidates,
                min,
                max,
                player,
                ..
            } => {
                // Gegner bevorzugen
                let mut c: Vec<Target> = candidates
                    .iter()
                    .copied()
                    .filter(|t| match t {
                        Target::Player(p) => p != player,
                        Target::Card(id) => state.card(*id).controller != *player,
                    })
                    .collect();
                if (c.len() as u32) < *min {
                    c = candidates.clone();
                }
                let n = (*max).min(c.len() as u32).max(*min) as usize;
                Choice::Targets(c[..n].to_vec())
            }
            Decision::ChooseX { max, .. } => Choice::Number(*max),
            Decision::PayMana { options, .. } => {
                if options.contains(&PayAction::AutoPay) {
                    Choice::Pay(PayAction::AutoPay)
                } else {
                    Choice::Pay(PayAction::Cancel)
                }
            }
            Decision::PayLifeForPhyrexian { .. } => Choice::No,
            Decision::ChooseColor { options, .. } => Choice::Color(options[0]),
            Decision::ChooseType { .. } => Choice::Number(0),
            Decision::DeclareAttackers {
                candidates,
                defenders,
                ..
            } => {
                let must = state.must_attack(d.player());
                let att = candidates
                    .iter()
                    .filter(|c| must.contains(c) || self.rng.below(2) == 0)
                    .map(|c| (*c, state.goad_allowed_targets(*c, defenders)[0]))
                    .collect();
                Choice::Attackers(att)
            }
            Decision::DeclareBlockers {
                candidates,
                attackers,
                ..
            } => {
                let mut blocks = Vec::new();
                for b in candidates {
                    if self.rng.below(2) == 0 {
                        let legal: Vec<_> = attackers
                            .iter()
                            .filter(|a| state.can_block(*b, **a))
                            .collect();
                        if let Some(a) = legal.first() {
                            blocks.push((*b, **a));
                        }
                    }
                }
                if state.validate_blocks(&blocks).is_err() {
                    blocks.clear();
                }
                Choice::Blocks(blocks)
            }
            Decision::AssignCombatDamage {
                amount,
                blockers,
                lethal,
                trample_to,
                ..
            } => {
                let mut rest = *amount;
                let mut d: Vec<u32> = Vec::new();
                for l in lethal {
                    let x = (*l).min(rest);
                    d.push(x);
                    rest -= x;
                }
                if trample_to.is_some() {
                    d.push(rest);
                } else if let Some(last) = d.last_mut() {
                    *last += rest;
                }
                debug_assert_eq!(d.len(), blockers.len() + usize::from(trample_to.is_some()));
                Choice::Damage(d)
            }
            Decision::ChooseModes {
                modes,
                min,
                max,
                can_repeat,
                ..
            } => {
                let k = (*min).max(1).min(*max) as usize;
                // mit CanRepeatModes notfalls Modi wiederholen
                let pick: Vec<usize> = if *can_repeat {
                    modes.iter().cycle().take(k).map(|(i, _)| *i).collect()
                } else {
                    modes.iter().take(k).map(|(i, _)| *i).collect()
                };
                Choice::Modes(pick)
            }
            Decision::Scry { cards, .. } => Choice::Arrange {
                top: cards.clone(),
                bottom: vec![],
            },
            Decision::PayUnlessCost { .. } => Choice::Yes,
            Decision::OrderTriggers { descriptions, .. } => {
                Choice::Order((0..descriptions.len()).collect())
            }
            Decision::OptionalTrigger { .. } | Decision::Confirm { .. } => Choice::Yes,
            Decision::ChooseCards {
                options,
                max,
                reason: CardChoiceReason::Search,
                ..
            } => Choice::Cards(options[..*max as usize].to_vec()),
            // Crew u. ä. (beliebig viele tappen): alle wählen, das reicht immer.
            Decision::ChooseCards {
                options,
                max,
                min,
                reason: CardChoiceReason::TapCost,
                ..
            } if max > min => Choice::Cards(options[..(*max as usize).min(options.len())].to_vec()),
            // Kostenhilfen voll nutzen (die Engine begrenzt `max` auf den Nutzen).
            Decision::ChooseCards {
                options,
                max,
                reason:
                    CardChoiceReason::Convoke
                    | CardChoiceReason::Improvise
                    | CardChoiceReason::Delve
                    | CardChoiceReason::Play,
                ..
            } => Choice::Cards(options[..(*max as usize).min(options.len())].to_vec()),
            Decision::ChooseCards {
                options,
                min,
                reason,
                ..
            } => {
                let _ = reason == &CardChoiceReason::LegendRule;
                Choice::Cards(options[..*min as usize].to_vec())
            }
        }
    }
}
