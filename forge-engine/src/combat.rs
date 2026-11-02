//! Kampf (CR 506–511).
//!
//! Java-Vorbild: `forge.game.combat.Combat`, `CombatUtil`, `AttackConstraints`,
//! `CombatUtil.canBlock`, `Combat.assignCombatDamage`.
//!
//! Schadensverteilung folgt den Regeln ab *Foundations* (2024): keine
//! Schadenszuweisungsreihenfolge mehr, der Angreifer verteilt frei auf seine
//! Blocker; Trample verlangt tödlichen Schaden an alle Blocker, bevor Rest
//! an den Spieler geht (CR 702.19c).

use crate::decision::{Choice, Decision, InvalidChoice, Target};
use crate::ids::{CardId, PlayerId};
use crate::keyword::Keyword;
use crate::state::{AttackerInfo, Choices, Combat, Event, GameState, Interrupt, Res};
use crate::zone::Zone;

impl GameState {
    /// Wirkende Kampfbeschränkungen einer Art: (Quelle, Controller, Beschränkung).
    pub(crate) fn restrictions(
        &self,
        kind: crate::statics::RestrictionKind,
    ) -> Vec<(CardId, PlayerId, &crate::statics::CombatRestriction)> {
        let mut v = Vec::new();
        for c in self.battlefield.iter().chain(self.command.iter()) {
            let o = self.card(*c);
            let Some(zone) = o.zone else { continue };
            for s in &o.compiled().statics {
                if let crate::statics::CompiledStatic::Combat(r) = s.as_ref() {
                    let ctx = crate::expr::EvalCtx {
                        source: *c,
                        controller: o.controller,
                        x: 0,
                        targets: &[],
                        trig: None,
                        paid: &[],
                    };
                    if r.kind == kind
                        && r.effect_zones.contains(&zone)
                        && self.check_conditions(&r.conditions, &ctx)
                    {
                        v.push((*c, o.controller, r));
                    }
                }
            }
        }
        v
    }

    /// Wirkende Spieler-Statics, die `p` betreffen: (Quelle, Controller, Static).
    pub(crate) fn player_statics(
        &self,
        p: PlayerId,
    ) -> Vec<(CardId, PlayerId, &crate::statics::PlayerStatic)> {
        let mut v = Vec::new();
        for c in self.battlefield.iter().chain(self.command.iter()) {
            let o = self.card(*c);
            let Some(zone) = o.zone else { continue };
            for s in &o.compiled().statics {
                if let crate::statics::CompiledStatic::Player(r) = s.as_ref() {
                    let ctx = crate::expr::EvalCtx {
                        source: *c,
                        controller: o.controller,
                        x: 0,
                        targets: &[],
                        trig: None,
                        paid: &[],
                    };
                    if r.effect_zones.contains(&zone)
                        && self.player_matches(p, &r.affected, o.controller)
                        && self.check_conditions(&r.conditions, &ctx)
                    {
                        v.push((*c, o.controller, r));
                    }
                }
            }
        }
        v
    }

    /// Maximale Handgröße von `p` (`None` = keine, CR 402.2); Java: erst
    /// `SetMaxHandSize`, dann `RaiseMaxHandSize`.
    pub fn max_hand_size(&self, p: PlayerId) -> Option<u32> {
        use crate::statics::HandSize;
        let statics = self.player_statics(p);
        let ctx = |src: CardId, ctrl: PlayerId| crate::expr::EvalCtx {
            source: src,
            controller: ctrl,
            x: 0,
            targets: &[],
            trig: None,
            paid: &[],
        };
        let mut size = self.rules.max_hand_size as i32;
        for (src, ctrl, s) in &statics {
            match &s.set_max_hand {
                Some(HandSize::Unlimited) => return None,
                Some(HandSize::Set(e)) => size = self.eval(e, &ctx(*src, *ctrl)),
                None => {}
            }
        }
        for (src, ctrl, s) in &statics {
            if let Some(e) = &s.raise_max_hand {
                size += self.eval(e, &ctx(*src, *ctrl));
            }
        }
        Some(size.max(0) as u32)
    }

    /// Spieler hat Fluchsicherheit/Verhüllung (CR 702.11d, 702.18d).
    pub fn player_has_hexproof(&self, p: PlayerId) -> bool {
        self.player_statics(p).iter().any(|(_, _, s)| s.hexproof)
    }
    pub fn player_has_shroud(&self, p: PlayerId) -> bool {
        self.player_statics(p).iter().any(|(_, _, s)| s.shroud)
    }

    fn restricted(&self, kind: crate::statics::RestrictionKind, card: CardId) -> bool {
        self.restrictions(kind)
            .iter()
            .any(|(src, ctrl, r)| self.card_matches(card, &r.valid, *src, *ctrl))
    }

    /// Kreaturen von `p`, die angreifen müssen, sofern sie können (CR 508.1d).
    pub fn must_attack(&self, p: PlayerId) -> Vec<CardId> {
        self.attack_candidates(p)
            .into_iter()
            .filter(|c| self.restricted(crate::statics::RestrictionKind::MustAttack, *c))
            .collect()
    }

    /// CR 508.1a: Kreaturen, die angreifen können.
    pub fn attack_candidates(&self, p: PlayerId) -> Vec<CardId> {
        let _cache = crate::state::CharsCache::enter();
        self.battlefield_of(p)
            .filter(|c| {
                self.is_creature(*c)
                    && !self.card(*c).tapped
                    && !self.is_summoning_sick(*c)
                    && !self.has_keyword(*c, &Keyword::Defender)
                    && !self.restricted(crate::statics::RestrictionKind::CantAttack, *c)
            })
            .collect()
    }

    /// CR 509.1a: Kreaturen von `p`, die blocken können.
    pub fn block_candidates(&self, p: PlayerId) -> Vec<CardId> {
        let _cache = crate::state::CharsCache::enter();
        self.battlefield_of(p)
            .filter(|c| {
                self.is_creature(*c)
                    && !self.card(*c).tapped
                    && !self.restricted(crate::statics::RestrictionKind::CantBlock, *c)
            })
            .collect()
    }

    /// Kann `blocker` `attacker` blocken (Einzelprüfung)? CR 702.9b Flying.
    pub fn can_block(&self, blocker: CardId, attacker: CardId) -> bool {
        if self.has_keyword(attacker, &Keyword::Flying)
            && !self.has_keyword(blocker, &Keyword::Flying)
            && !self.has_keyword(blocker, &Keyword::Reach)
        {
            return false;
        }
        // CantBlockBy: „kann nicht (von …) geblockt werden“
        for (src, ctrl, r) in self.restrictions(crate::statics::RestrictionKind::CantBlockBy) {
            if self.card_matches(attacker, &r.valid, src, ctrl)
                && r.valid_blocker
                    .as_ref()
                    .is_none_or(|v| self.card_matches(blocker, v, src, ctrl))
            {
                return false;
            }
        }
        true
    }

    /// Zustandsabhängige Prüfung einer Block-Deklaration (CR 509.1b–c).
    pub fn validate_blocks(&self, blocks: &[(CardId, CardId)]) -> Result<(), InvalidChoice> {
        for (b, a) in blocks {
            if !self.can_block(*b, *a) {
                return Err(InvalidChoice(format!(
                    "{b} kann {a} nicht blocken (Flugfähigkeit)"
                )));
            }
        }
        // Menace (CR 702.111b): von zwei oder mehr Kreaturen geblockt oder gar nicht.
        let attackers: std::collections::BTreeSet<CardId> = blocks.iter().map(|x| x.1).collect();
        for a in attackers {
            let n = blocks.iter().filter(|x| x.1 == a).count();
            if n == 1 && self.has_keyword(a, &Keyword::Menace) {
                return Err(InvalidChoice(format!(
                    "{a} hat Bedrohlich und braucht mindestens zwei Blocker"
                )));
            }
        }
        Ok(())
    }

    /// Verteidigender Spieler für ein Angriffsziel (Planeswalker → Controller).
    fn defending_player_of(&self, t: Target) -> PlayerId {
        match t {
            Target::Player(p) => p,
            Target::Card(c) => self.card(c).controller,
        }
    }

    pub(crate) fn declare_attackers(&mut self, ch: &mut Choices) -> Res<()> {
        let p = self.active;
        let candidates = self.attack_candidates(p);
        // CR 508.1b: Gegner und von Gegnern kontrollierte Planeswalker
        let opps = self.opponents(p);
        let mut defenders: Vec<Target> = opps.iter().map(|o| Target::Player(*o)).collect();
        defenders.extend(
            opps.iter()
                .flat_map(|o| self.battlefield_of(*o).collect::<Vec<_>>())
                .filter(|c| {
                    self.chars(*c)
                        .types
                        .has_core(forge_types::CoreType::Planeswalker)
                })
                .map(Target::Card),
        );
        let chosen = if candidates.is_empty() || opps.is_empty() {
            Vec::new()
        } else {
            match ch.ask(Decision::DeclareAttackers {
                player: p,
                candidates,
                defenders,
            })? {
                Choice::Attackers(v) => v,
                _ => Vec::new(),
            }
        };
        // CR 508.1d: Angriffspflichten erfüllen, soweit möglich
        let must = self.must_attack(p);
        if let Some(m) = must.iter().find(|m| !chosen.iter().any(|(c, _)| c == *m)) {
            return Err(Interrupt::Invalid(InvalidChoice(format!(
                "{m} muss angreifen"
            ))));
        }
        let infos: Vec<AttackerInfo> = chosen
            .iter()
            .map(|(c, t)| AttackerInfo {
                card: *c,
                defender: self.defending_player_of(*t),
                target: *t,
                blocked: false,
            })
            .collect();
        let combat = self.combat.get_or_insert_with(Combat::default);
        combat.attackers = infos.clone();
        for (c, _) in &chosen {
            // CR 508.1f
            if !self.has_keyword(*c, &Keyword::Vigilance) {
                self.tap(*c);
            }
        }
        for i in &infos {
            let ctrl = self.card(i.card).controller;
            self.this_turn.attackers.push((ctrl, i.card));
        }
        for i in &infos {
            self.fire(crate::triggers::TriggerEvent::Attacks {
                attacker: i.card,
                defender: i.defender,
                target: i.target,
            });
        }
        if !chosen.is_empty() {
            // Java PhaseHandler: erst je angegriffenem Ziel, dann insgesamt.
            let mut targets: Vec<Target> = Vec::new();
            for (_, t) in &chosen {
                if !targets.contains(t) {
                    targets.push(*t);
                }
            }
            for tgt in targets {
                let of: Vec<(CardId, Target)> =
                    chosen.iter().copied().filter(|(_, t)| *t == tgt).collect();
                if !of.is_empty() {
                    self.fire(crate::triggers::TriggerEvent::AttackersDeclared {
                        player: self.active,
                        attackers: of,
                        one_target: true,
                    });
                }
            }
            self.fire(crate::triggers::TriggerEvent::AttackersDeclared {
                player: self.active,
                attackers: chosen.clone(),
                one_target: false,
            });
        }
        self.push_event(Event::AttackersDeclared(chosen));
        Ok(())
    }

    pub fn has_attackers(&self) -> bool {
        self.combat
            .as_ref()
            .is_some_and(|c| !c.attackers.is_empty())
    }

    pub(crate) fn declare_blockers(&mut self, ch: &mut Choices) -> Res<()> {
        let defenders: Vec<PlayerId> = {
            let c = self.combat.as_ref().expect("Kampf aktiv");
            let mut d: Vec<PlayerId> = c.attackers.iter().map(|a| a.defender).collect();
            d.sort();
            d.dedup();
            // APNAP-Reihenfolge
            self.apnap().into_iter().filter(|p| d.contains(p)).collect()
        };
        let mut all_blocks = Vec::new();
        for d in defenders {
            let attackers: Vec<CardId> = self
                .combat
                .as_ref()
                .expect("Kampf")
                .attackers
                .iter()
                .filter(|a| a.defender == d)
                .map(|a| a.card)
                .collect();
            let candidates = self.block_candidates(d);
            if candidates.is_empty() || attackers.is_empty() {
                continue;
            }
            let blocks = match ch.ask(Decision::DeclareBlockers {
                player: d,
                candidates,
                attackers,
            })? {
                Choice::Blocks(b) => b,
                _ => Vec::new(),
            };
            if let Err(e) = self.validate_blocks(&blocks) {
                return Err(Interrupt::Invalid(e));
            }
            all_blocks.extend(blocks);
        }
        let combat = self.combat.as_mut().expect("Kampf");
        for (_, a) in &all_blocks {
            if let Some(info) = combat.attackers.iter_mut().find(|x| x.card == *a) {
                info.blocked = true;
            }
        }
        combat.blocks = all_blocks.clone();
        let infos = combat.attackers.clone();
        // CR 509.2–509.5: Blocks-, AttackerBlocked- und AttackerUnblocked-Trigger
        let mut blockers: Vec<CardId> = Vec::new();
        for (b, _) in &all_blocks {
            if !blockers.contains(b) {
                blockers.push(*b);
            }
        }
        for b in blockers {
            let attackers = all_blocks
                .iter()
                .filter(|(x, _)| *x == b)
                .map(|(_, a)| *a)
                .collect();
            self.fire(crate::triggers::TriggerEvent::Blocks {
                blocker: b,
                attackers,
            });
        }
        for i in &infos {
            let blockers: Vec<CardId> = all_blocks
                .iter()
                .filter(|(_, a)| *a == i.card)
                .map(|(b, _)| *b)
                .collect();
            for b in &blockers {
                self.fire(crate::triggers::TriggerEvent::AttackerBlockedByCreature {
                    attacker: i.card,
                    blocker: *b,
                    defender: i.defender,
                });
            }
            if i.blocked {
                self.fire(crate::triggers::TriggerEvent::AttackerBlocked {
                    attacker: i.card,
                    blockers,
                    defender: i.defender,
                });
            } else {
                self.fire(crate::triggers::TriggerEvent::AttackerUnblocked {
                    attacker: i.card,
                    defender: i.defender,
                    target: i.target,
                });
            }
        }
        self.push_event(Event::BlockersDeclared(all_blocks));
        Ok(())
    }

    fn has_first_strike(&self, c: CardId) -> bool {
        self.has_keyword(c, &Keyword::FirstStrike) || self.has_keyword(c, &Keyword::DoubleStrike)
    }

    /// CR 510.4: Gibt es einen Erstschlag-Schadensschritt?
    pub fn needs_first_strike_step(&self) -> bool {
        let Some(c) = &self.combat else { return false };
        c.attackers
            .iter()
            .map(|a| a.card)
            .chain(c.blocks.iter().map(|b| b.0))
            .any(|x| self.has_first_strike(x))
    }

    fn deals_damage_this_step(&self, c: CardId, first_strike_step: bool) -> bool {
        if self.card(c).zone != Some(Zone::Battlefield) {
            return false;
        }
        if first_strike_step {
            self.has_first_strike(c)
        } else {
            let dealt = self
                .combat
                .as_ref()
                .is_some_and(|cb| cb.dealt_first_strike.contains(&c));
            !dealt || self.has_keyword(c, &Keyword::DoubleStrike)
        }
    }

    fn lethal_for(&self, blocker: CardId, attacker: CardId) -> u32 {
        let t = self.chars(blocker).toughness.unwrap_or(0);
        let remaining = (t - self.card(blocker).damage as i32).max(0) as u32;
        if self.has_keyword(attacker, &Keyword::Deathtouch) {
            remaining.min(1)
        } else {
            remaining
        }
    }

    /// CR 510.1–2: Kampfschaden zuweisen und gleichzeitig zufügen.
    pub(crate) fn combat_damage(&mut self, first_strike_step: bool, ch: &mut Choices) -> Res<()> {
        let Some(combat) = self.combat.clone() else {
            return Ok(());
        };
        let mut assignments: Vec<(CardId, Target, u32)> = Vec::new();

        for info in &combat.attackers {
            let a = info.card;
            if !self.deals_damage_this_step(a, first_strike_step) {
                continue;
            }
            let power = self.chars(a).power.unwrap_or(0).max(0) as u32;
            if power == 0 {
                continue;
            }
            let blockers: Vec<CardId> = combat
                .blockers_of(a)
                .into_iter()
                .filter(|b| self.card(*b).zone == Some(Zone::Battlefield))
                .collect();
            let trample = self.has_keyword(a, &Keyword::Trample);
            let attacked = match info.target {
                Target::Card(pw) if self.card(pw).zone != Some(Zone::Battlefield) => None,
                t => Some(t),
            };
            if !info.blocked {
                // CR 506.4c: angegriffener Planeswalker weg → kein Kampfschaden
                if let Some(t) = attacked {
                    assignments.push((a, t, power));
                }
            } else if blockers.is_empty() {
                // CR 509.1h / 702.19e
                if trample && let Some(t) = attacked {
                    assignments.push((a, t, power));
                }
            } else if blockers.len() == 1 && !trample {
                assignments.push((a, Target::Card(blockers[0]), power));
            } else {
                let lethal: Vec<u32> = blockers.iter().map(|b| self.lethal_for(*b, a)).collect();
                let trample_to = attacked.filter(|_| trample);
                let dist = match ch.ask(Decision::AssignCombatDamage {
                    player: self.card(a).controller,
                    attacker: a,
                    amount: power,
                    blockers: blockers.clone(),
                    lethal,
                    trample_to,
                })? {
                    Choice::Damage(d) => d,
                    _ => return Err(Interrupt::Cancelled),
                };
                for (b, n) in blockers.iter().zip(&dist) {
                    assignments.push((a, Target::Card(*b), *n));
                }
                if let Some(t) = trample_to {
                    assignments.push((a, t, dist[blockers.len()]));
                }
            }
        }
        for (b, a) in &combat.blocks {
            if !self.deals_damage_this_step(*b, first_strike_step)
                || self.card(*a).zone != Some(Zone::Battlefield)
            {
                continue;
            }
            let power = self.chars(*b).power.unwrap_or(0).max(0) as u32;
            if power > 0 {
                assignments.push((*b, Target::Card(*a), power));
            }
        }

        if first_strike_step {
            let dealers: Vec<CardId> = assignments.iter().map(|x| x.0).collect();
            if let Some(c) = self.combat.as_mut() {
                c.dealt_first_strike.extend(dealers);
            }
        }
        // CR 510.2: gleichzeitig
        self.begin_damage_batch();
        for (src, tgt, n) in assignments {
            self.deal_damage_ex(src, tgt, n, true);
        }
        self.end_damage_batch();
        Ok(())
    }
}
