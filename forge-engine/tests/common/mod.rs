//! Testgerüst: kleine Kartenbasis, Spiel direkt in einem Schritt aufsetzen,
//! Entscheidungen beantworten.
#![allow(dead_code)]

use forge_engine::decision::{Action, Choice, Decision, PayAction, Target};
use forge_engine::state::Flow;
use forge_engine::{CardDb, CardId, Engine, GameRules, GameState, PlayerId, Status, Step, Zone};

pub const P1: PlayerId = PlayerId(0);
pub const P2: PlayerId = PlayerId(1);

pub const SCRIPTS: &[&str] = &[
    "Name:Plains\nManaCost:no cost\nTypes:Basic Land Plains\nOracle:\n",
    "Name:Island\nManaCost:no cost\nTypes:Basic Land Island\nOracle:\n",
    "Name:Swamp\nManaCost:no cost\nTypes:Basic Land Swamp\nOracle:\n",
    "Name:Mountain\nManaCost:no cost\nTypes:Basic Land Mountain\nOracle:\n",
    "Name:Forest\nManaCost:no cost\nTypes:Basic Land Forest\nOracle:\n",
    "Name:Grizzly Bears\nManaCost:1 G\nTypes:Creature Bear\nPT:2/2\nOracle:\n",
    "Name:Hill Giant\nManaCost:3 R\nTypes:Creature Giant\nPT:3/3\nOracle:\n",
    "Name:Serra Angel\nManaCost:3 W W\nTypes:Creature Angel\nPT:4/4\nK:Flying\nK:Vigilance\nOracle:\n",
    "Name:Giant Spider\nManaCost:3 G\nTypes:Creature Spider\nPT:2/4\nK:Reach\nOracle:\n",
    "Name:Goblin Piker\nManaCost:1 R\nTypes:Creature Goblin\nPT:2/1\nOracle:\n",
    "Name:Craw Wurm\nManaCost:4 G G\nTypes:Creature Wurm\nPT:6/4\nK:Trample\nOracle:\n",
    "Name:White Knight\nManaCost:W W\nTypes:Creature Human Knight\nPT:2/2\nK:First Strike\nOracle:\n",
    "Name:Fencing Ace\nManaCost:1 W\nTypes:Creature Human Soldier\nPT:1/1\nK:Double Strike\nOracle:\n",
    "Name:Typhoid Rats\nManaCost:B\nTypes:Creature Rat\nPT:1/1\nK:Deathtouch\nOracle:\n",
    "Name:Vampire Nighthawk\nManaCost:1 B B\nTypes:Creature Vampire Shaman\nPT:2/3\nK:Flying\nK:Deathtouch\nK:Lifelink\nOracle:\n",
    "Name:Raging Goblin\nManaCost:R\nTypes:Creature Goblin Berserker\nPT:1/1\nK:Haste\nOracle:\n",
    "Name:Wall of Stone\nManaCost:1 R R\nTypes:Creature Wall\nPT:0/8\nK:Defender\nOracle:\n",
    "Name:Goblin Heelcutter\nManaCost:3 R\nTypes:Creature Goblin Warrior\nPT:3/2\nK:Menace\nOracle:\n",
    "Name:Darksteel Myr\nManaCost:3\nTypes:Artifact Creature Myr\nPT:0/1\nK:Indestructible\nOracle:\n",
    "Name:Gladecover Scout\nManaCost:G\nTypes:Creature Elf Scout\nPT:1/1\nK:Hexproof\nOracle:\n",
    "Name:Isamaru, Hound of Konda\nManaCost:W\nTypes:Legendary Creature Dog\nPT:2/2\nOracle:\n",
    "Name:Llanowar Elves\nManaCost:G\nTypes:Creature Elf Druid\nPT:1/1\nA:AB$ Mana | Cost$ T | Produced$ G | SpellDescription$ Add {G}.\nOracle:\n",
    "Name:Lightning Bolt\nManaCost:R\nTypes:Instant\nA:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | SpellDescription$ CARDNAME deals 3 damage to any target.\nOracle:\n",
    "Name:Shock\nManaCost:R\nTypes:Instant\nA:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 2 | SpellDescription$ CARDNAME deals 2 damage to any target.\nOracle:\n",
    "Name:Murder\nManaCost:1 B B\nTypes:Instant\nA:SP$ Destroy | ValidTgts$ Creature | SpellDescription$ Destroy target creature.\nOracle:\n",
    "Name:Giant Growth\nManaCost:G\nTypes:Instant\nA:SP$ Pump | ValidTgts$ Creature | NumAtt$ +3 | NumDef$ +3 | SpellDescription$ Target creature gets +3/+3 until end of turn.\nOracle:\n",
    "Name:Weakness\nManaCost:B\nTypes:Instant\nA:SP$ Pump | ValidTgts$ Creature | NumAtt$ -2 | NumDef$ -1 | SpellDescription$ x\nOracle:\n",
    "Name:Divination\nManaCost:2 U\nTypes:Sorcery\nA:SP$ Draw | NumCards$ 2 | SpellDescription$ Draw two cards.\nOracle:\n",
    "Name:Healing Salve\nManaCost:W\nTypes:Instant\nA:SP$ GainLife | ValidTgts$ Player | LifeAmount$ 3 | SpellDescription$ x\nOracle:\n",
    "Name:Fireball Lite\nManaCost:X R\nTypes:Sorcery\nA:SP$ DealDamage | ValidTgts$ Any | NumDmg$ X | SpellDescription$ x\nSVar:X:Count$xPaid\nOracle:\n",
    "Name:Gitaxian Probe Lite\nManaCost:UP\nTypes:Sorcery\nA:SP$ Draw | NumCards$ 1 | SpellDescription$ x\nOracle:\n",
    "Name:Prodigal Pyromancer\nManaCost:2 R\nTypes:Creature Human Wizard\nPT:1/1\nA:AB$ DealDamage | Cost$ T | ValidTgts$ Any | NumDmg$ 1 | SpellDescription$ x\nOracle:\n",
    "Name:Nether Void Lite\nManaCost:3 B\nTypes:World Enchantment\nOracle:\n",
    "Name:Concordant Crossroads Lite\nManaCost:G\nTypes:World Enchantment\nOracle:\n",
    "Name:Liliana Lite\nManaCost:1 B B\nTypes:Legendary Planeswalker Liliana\nLoyalty:3\nOracle:\n",
    "Name:Invasion Lite\nManaCost:2 R\nTypes:Battle Siege\nDefense:3\nOracle:\n",
    "Name:Ambush Viper\nManaCost:1 G\nTypes:Creature Snake\nPT:2/1\nK:Flash\nK:Deathtouch\nOracle:\n",
    "Name:Silhana Lite\nManaCost:G\nTypes:Creature Elf\nPT:2/2\nK:Shroud\nOracle:\n",
    "Name:Mogg Fanatic Lite\nManaCost:R\nTypes:Creature Goblin\nPT:1/1\nA:AB$ DealDamage | Cost$ Sac<1/CARDNAME> | ValidTgts$ Any | NumDmg$ 1 | SpellDescription$ x\nOracle:\n",
    "Name:Untap Monk Lite\nManaCost:W\nTypes:Creature Human Monk\nPT:1/1\nA:AB$ GainLife | Cost$ Q | LifeAmount$ 2 | SpellDescription$ x\nOracle:\n",
    "Name:Mind Rot Lite\nManaCost:2 B\nTypes:Sorcery\nA:SP$ LoseLife | ValidTgts$ Player | LifeAmount$ 2 | Defined$ Targeted | SpellDescription$ x\nOracle:\n",
];

pub fn db() -> CardDb {
    let mut db = CardDb::default();
    for s in SCRIPTS {
        let d = db.add_script(s).expect("Testskript parst");
        let pw_or_battle = d
            .face(0)
            .types
            .has_core(forge_types::CoreType::Planeswalker)
            || d.face(0).types.has_core(forge_types::CoreType::Battle);
        assert!(
            pw_or_battle || d.is_supported(),
            "{} nicht unterstützt: {:?}",
            d.name(),
            d.unsupported
        );
    }
    db
}

/// Spiel mit `lib_size` Inseln als Bibliothek je Spieler, fertig gemulliganed,
/// Zug 2, P1 aktiv, Beginn von `step`. Startspieler war P2.
pub fn game_at(step: Step, lib_size: usize) -> (CardDb, GameState) {
    let db = db();
    let island = db.get("Island").unwrap().clone();
    let decks = vec![
        ("P1".to_string(), vec![island.clone(); lib_size]),
        ("P2".to_string(), vec![island; lib_size]),
    ];
    let mut g = GameState::new(GameRules::default(), decks, 7, Some(P2));
    for p in &mut g.players {
        p.kept_hand = true;
    }
    g.turn = 2;
    g.active = P1;
    g.step = step;
    g.flow = Flow::BeginStep;
    if step.is_combat() {
        g.combat = Some(Default::default());
    }
    (db, g)
}

/// Legt eine Karte in eine Zone. Auf dem Battlefield gilt sie als seit Zugbeginn kontrolliert.
pub fn put(db: &CardDb, g: &mut GameState, name: &str, p: PlayerId, z: Zone) -> CardId {
    let def = db
        .get(name)
        .unwrap_or_else(|| panic!("Karte {name} fehlt"))
        .clone();
    let id = g.create_card(def, p, z, false);
    if z == Zone::Battlefield {
        g.card_mut(id).controlled_since_turn_start = true;
    }
    id
}

pub fn put_n(
    db: &CardDb,
    g: &mut GameState,
    name: &str,
    p: PlayerId,
    z: Zone,
    n: usize,
) -> Vec<CardId> {
    (0..n).map(|_| put(db, g, name, p, z)).collect()
}

/// Aktuelles Objekt mit Namen in einer Zone (IDs ändern sich bei Zonenwechseln).
pub fn find(g: &GameState, name: &str, z: Zone) -> Option<CardId> {
    g.cards
        .iter()
        .find(|c| c.zone == Some(z) && c.def.name() == name)
        .map(|c| c.id)
}

pub fn count(g: &GameState, name: &str, z: Zone) -> usize {
    g.cards
        .iter()
        .filter(|c| c.zone == Some(z) && c.def.name() == name)
        .count()
}

/// Treiber mit Standardantworten: Passen, keine Angriffe/Blocks, AutoPay.
pub struct Driver {
    pub e: Engine,
    pub status: Status,
}

impl Driver {
    pub fn new(g: GameState) -> Driver {
        let mut e = Engine::new(g);
        let status = e.advance();
        Driver { e, status }
    }

    pub fn g(&self) -> &GameState {
        self.e.state()
    }

    pub fn decision(&self) -> &Decision {
        match &self.status {
            Status::NeedsDecision(d) => d,
            s => panic!("keine Entscheidung offen: {s:?}"),
        }
    }

    pub fn submit(&mut self, c: Choice) {
        self.status = self
            .e
            .submit(c.clone())
            .unwrap_or_else(|e| panic!("{c:?} abgelehnt bei {:?}: {e}", self.status));
    }

    pub fn try_submit(&mut self, c: Choice) -> Result<(), forge_engine::decision::InvalidChoice> {
        self.status = self.e.submit(c)?;
        Ok(())
    }

    /// Beantwortet Standardentscheidungen, bis `stop` zutrifft.
    pub fn run_until(&mut self, mut stop: impl FnMut(&GameState, &Decision) -> bool) {
        for _ in 0..10_000 {
            let d = match &self.status {
                Status::NeedsDecision(d) => d.clone(),
                Status::GameOver(_) => return,
            };
            if stop(self.e.state(), &d) {
                return;
            }
            let c = default_choice(&d);
            self.submit(c);
        }
        panic!("run_until: kein Ende");
    }

    /// Passt, bis `player` in `step` Priorität hat.
    pub fn priority_in(&mut self, step: Step, player: PlayerId) {
        self.run_until(|g, d| {
            matches!(d, Decision::Priority { player: p, .. } if *p == player) && g.step == step
        });
    }

    pub fn act(&mut self, a: Action) {
        self.submit(Choice::Action(a));
    }

    /// Wirkt eine Handkarte mit Zielen; bezahlt automatisch.
    pub fn cast(&mut self, card: CardId, targets: &[Target]) {
        self.act(Action::Cast(card));
        self.finish_casting(targets);
    }

    pub fn finish_casting(&mut self, targets: &[Target]) {
        loop {
            match self.decision().clone() {
                Decision::ChooseTargets { .. } => self.submit(Choice::Targets(targets.to_vec())),
                Decision::PayMana { .. } => self.submit(Choice::Pay(PayAction::AutoPay)),
                Decision::PayLifeForPhyrexian { .. } => self.submit(Choice::No),
                _ => return,
            }
        }
    }

    /// Alle Spieler passen einmal (Stapel löst einmal auf bzw. Schritt endet).
    pub fn pass_all(&mut self) {
        let n = self.g().players_in_game();
        for _ in 0..n {
            if matches!(self.decision(), Decision::Priority { .. }) {
                self.act(Action::Pass);
            }
        }
    }

    pub fn is_over(&self) -> Option<Option<PlayerId>> {
        match self.status {
            Status::GameOver(w) => Some(w),
            _ => None,
        }
    }
}

pub fn default_choice(d: &Decision) -> Choice {
    match d {
        Decision::Mulligan { .. } => Choice::Keep,
        Decision::MulliganBottom { hand, count, .. } => {
            Choice::Cards(hand[..*count as usize].to_vec())
        }
        Decision::Priority { .. } => Choice::Action(Action::Pass),
        Decision::DeclareAttackers { .. } => Choice::Attackers(vec![]),
        Decision::DeclareBlockers { .. } => Choice::Blocks(vec![]),
        Decision::PayMana { .. } => Choice::Pay(PayAction::AutoPay),
        Decision::PayLifeForPhyrexian { .. } => Choice::No,
        Decision::ChooseCards { options, min, .. } => {
            Choice::Cards(options[..*min as usize].to_vec())
        }
        Decision::ChooseColor { options, .. } => Choice::Color(options[0]),
        Decision::ChooseX { .. } => Choice::Number(0),
        Decision::ChooseTargets {
            candidates, min, ..
        } => Choice::Targets(candidates[..*min as usize].to_vec()),
        Decision::AssignCombatDamage {
            amount,
            blockers,
            trample_to,
            ..
        } => {
            let mut v = vec![0; blockers.len() + usize::from(trample_to.is_some())];
            v[0] = *amount;
            Choice::Damage(v)
        }
    }
}
