//! Ausgelöste Fähigkeiten (CR 603, Forge_Trigger.xlsx).

mod common;
use common::*;
use forge_engine::decision::{Action, Choice, Decision, Target};
use forge_engine::state::StackKind;
use forge_engine::{Step, Zone};

fn is_trigger_on_top(d: &Driver) -> bool {
    matches!(
        d.g().stack.last().map(|s| &s.kind),
        Some(StackKind::Trigger { .. })
    )
}

#[test]
fn etb_trigger_uses_stack_and_resolves() {
    // CR 603.2, 603.3: ETB-Trigger geht auf den Stapel, bevor Priorität vergeben wird
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 2);
    let elf = put(&db, &mut g, "Elvish Visionary", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(elf, &[]);
    d.pass_all(); // Kreatur löst auf
    assert!(is_trigger_on_top(&d), "Trigger liegt auf dem Stapel");
    assert!(matches!(d.decision(), Decision::Priority { player, .. } if *player == P1));
    let hand = d.g().player(P1).hand.len();
    d.pass_all();
    assert_eq!(d.g().player(P1).hand.len(), hand + 1);
}

#[test]
fn dies_trigger_looks_back_in_time() {
    // CR 603.10a
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Mountain", P1, Zone::Battlefield);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let salve = put(&db, &mut g, "Salve Bearer Lite", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(bolt, &[Target::Card(salve)]);
    d.pass_all();
    assert_eq!(count(d.g(), "Salve Bearer Lite", Zone::Graveyard), 1);
    assert!(is_trigger_on_top(&d));
    assert_eq!(
        d.g().stack.last().unwrap().controller,
        P2,
        "Controller der Quelle (CR 603.3a)"
    );
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 22);
}

#[test]
fn simultaneous_deaths_see_each_other() {
    // CR 603.10a: Blood Artist sieht sich selbst und die gleichzeitig sterbende Kreatur
    let (db, mut g) = game_at(Step::Main1, 10);
    let artist = put(&db, &mut g, "Blood Artist Lite", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    g.card_mut(artist).damage = 1;
    g.card_mut(bear).damage = 2;
    let mut d = Driver::new(g);
    // zwei eigene Trigger → Reihenfolge wählen
    assert!(
        matches!(d.decision(), Decision::OrderTriggers { descriptions, .. } if descriptions.len() == 2)
    );
    d.submit(Choice::Order(vec![1, 0]));
    for _ in 0..2 {
        assert!(matches!(d.decision(), Decision::ChooseTargets { .. }));
        d.submit(Choice::Targets(vec![Target::Player(P2)]));
    }
    assert_eq!(d.g().stack.len(), 2);
    d.pass_all();
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 18);
}

#[test]
fn upkeep_trigger_with_sub_ability() {
    // CR 503.1a
    let (db, mut g) = game_at(Step::Untap, 10);
    put(&db, &mut g, "Phyrexian Arena Lite", P1, Zone::Battlefield);
    put(&db, &mut g, "Phyrexian Arena Lite", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.priority_in(Step::Upkeep, P1);
    assert_eq!(
        d.g().stack.len(),
        1,
        "nur der Trigger des aktiven Spielers (ValidPlayer$ You)"
    );
    d.pass_all();
    assert_eq!(d.g().player(P1).hand.len(), 1);
    assert_eq!(d.g().player(P1).life, 19);
    assert_eq!(d.g().player(P2).life, 20);
}

#[test]
fn apnap_order_active_player_triggers_resolve_last() {
    // CR 603.3b
    let (db, mut g) = game_at(Step::Untap, 10);
    put(&db, &mut g, "Optional Upkeep Lite", P1, Zone::Battlefield);
    put(&db, &mut g, "Optional Upkeep Lite", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.priority_in(Step::Upkeep, P1);
    let ctrls: Vec<_> = d.g().stack.iter().map(|s| s.controller).collect();
    assert_eq!(ctrls, vec![P1, P2], "AP zuerst auf den Stapel, NAP oben");
    // oberster (P2) löst zuerst auf; optional → Nachfrage
    d.act(Action::Pass);
    d.act(Action::Pass);
    assert!(matches!(d.decision(), Decision::OptionalTrigger { player, .. } if *player == P2));
    d.submit(Choice::No);
    d.pass_all();
    assert!(matches!(d.decision(), Decision::OptionalTrigger { player, .. } if *player == P1));
    d.submit(Choice::Yes);
    assert_eq!(d.g().player(P1).life, 21);
    assert_eq!(d.g().player(P2).life, 20);
}

#[test]
fn attack_trigger_with_triggered_player() {
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    let gob = put(&db, &mut g, "Attack Drummer Lite", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.run_until(|_, dec| matches!(dec, Decision::DeclareAttackers { .. }));
    d.submit(Choice::Attackers(vec![(gob, T2)]));
    assert!(is_trigger_on_top(&d));
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 19);
    d.run_until(|g, _| g.step == Step::Main2);
    assert_eq!(d.g().player(P2).life, 17);
}

#[test]
fn spell_cast_trigger_resolves_before_spell() {
    // CR 601.2i + 603.3: Trigger liegt über dem Zauber
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Firebrand Lite", P1, Zone::Battlefield);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 3);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let goblin = put(&db, &mut g, "Goblin Piker", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(bolt, &[Target::Player(P2)]);
    // Priorität: Trigger wird vorher auf den Stapel gelegt
    assert_eq!(d.g().stack.len(), 2);
    assert!(is_trigger_on_top(&d));
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 19);
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 16);
    d.cast(goblin, &[]);
    assert_eq!(d.g().stack.len(), 1, "Kreaturenzauber löst nicht aus");
}

#[test]
fn combat_damage_trigger() {
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    let ninja = put(&db, &mut g, "Ninja Lite", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.run_until(|_, dec| matches!(dec, Decision::DeclareAttackers { .. }));
    d.submit(Choice::Attackers(vec![(ninja, T2)]));
    d.priority_in(Step::CombatDamage, P1);
    assert!(is_trigger_on_top(&d));
    d.pass_all();
    assert_eq!(d.g().player(P1).hand.len(), 1);
}

#[test]
fn trigger_targets_chosen_on_stack_and_removed_without_targets() {
    // CR 603.3d
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 6);
    let e1 = put(&db, &mut g, "Shocking ETB Lite", P1, Zone::Hand);
    let e2 = put(&db, &mut g, "Shocking ETB Lite", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(e1, &[]);
    d.pass_all();
    match d.decision() {
        Decision::ChooseTargets { candidates, .. } => {
            assert_eq!(candidates, &vec![Target::Card(bear)])
        }
        o => panic!("{o:?}"),
    }
    d.submit(Choice::Targets(vec![Target::Card(bear)]));
    d.pass_all();
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Graveyard), 1);
    // zweites Exemplar: kein legales Ziel → Trigger wird entfernt
    d.cast(e2, &[]);
    d.pass_all();
    assert!(d.g().stack.is_empty());
    assert!(matches!(d.decision(), Decision::Priority { .. }));
}

#[test]
fn attackers_declared_once_with_minimum_amount() {
    // CR 508.1m: einmal je Angriffsdeklaration, nicht je Angreifer
    for (n, expect) in [(1usize, false), (2, true), (3, true)] {
        let (db, mut g) = game_at(Step::BeginCombat, 10);
        put(&db, &mut g, "War Horn Lite", P1, Zone::Battlefield);
        put(&db, &mut g, "War Horn Lite", P2, Zone::Battlefield);
        let atk: Vec<_> = (0..n)
            .map(|_| put(&db, &mut g, "Goblin Piker", P1, Zone::Battlefield))
            .collect();
        let mut d = Driver::new(g);
        d.run_until(|_, dec| matches!(dec, Decision::DeclareAttackers { .. }));
        d.submit(Choice::Attackers(atk.iter().map(|c| (*c, T2)).collect()));
        assert_eq!(
            d.g().stack.len(),
            usize::from(expect),
            "n={n}: nur P1s Horn"
        );
        d.pass_all();
        let (l1, l2) = (d.g().player(P1).life, d.g().player(P2).life);
        assert_eq!((l1, l2), if expect { (21, 19) } else { (20, 20) }, "n={n}");
    }
}

#[test]
fn attackers_declared_one_target_per_attacked_player() {
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    put(&db, &mut g, "Raid Banner Lite", P1, Zone::Battlefield);
    let a = put(&db, &mut g, "Goblin Piker", P1, Zone::Battlefield);
    let b = put(&db, &mut g, "Goblin Piker", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.run_until(|_, dec| matches!(dec, Decision::DeclareAttackers { .. }));
    d.submit(Choice::Attackers(vec![(a, T2), (b, T2)]));
    assert_eq!(d.g().stack.len(), 1, "einmal je angegriffenem Spieler");
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 19, "TriggeredAttackedTarget");
}

#[test]
fn blocks_attacker_blocked_and_unblocked_triggers() {
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    let r1 = put(&db, &mut g, "Rampager Lite", P1, Zone::Battlefield);
    let r2 = put(&db, &mut g, "Rampager Lite", P1, Zone::Battlefield);
    let wall = put(&db, &mut g, "Wall Watcher Lite", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.run_until(|_, dec| matches!(dec, Decision::DeclareAttackers { .. }));
    d.submit(Choice::Attackers(vec![(r1, T2), (r2, T2)]));
    d.run_until(|_, dec| matches!(dec, Decision::DeclareBlockers { .. }));
    d.submit(Choice::Blocks(vec![(wall, r1)]));
    // Blocks (P2), AttackerBlocked + AttackerUnblocked (P1)
    d.run_until(|g, _| g.step == Step::Main2);
    assert_eq!(
        d.g().player(P2).life,
        20 + 2 - 3,
        "Wall-Trigger, r2 ungeblockt"
    );
    assert_eq!(d.g().chars(r1).power, Some(5), "+2/+2 bis Zugende");
    assert_eq!(d.g().player(P1).hand.len(), 1, "Unblocked → Karte");
}

#[test]
fn drawn_number_life_gained_and_taps_triggers() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Second Draw Lite", P1, Zone::Battlefield);
    put(&db, &mut g, "Life Echo Lite", P1, Zone::Battlefield);
    put(&db, &mut g, "Tap Watcher Lite", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    // Zug 3 (P2): P2 greift an → Taps-Trigger; Zug 4: P1 zieht im Ziehschritt (1. Karte)
    let mut g = d.g().clone();
    g.draw(P1); // 1. Karte dieses Zuges
    g.draw(P1); // 2. → Trigger
    d = Driver::new(g);
    d.pass_all();
    // Second Draw: +3 Leben → Life Echo: Gegner −1
    d.pass_all();
    assert_eq!(d.g().player(P1).life, 23);
    assert_eq!(d.g().player(P2).life, 19);
    let mut g = d.g().clone();
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    g.tap(bear);
    let mut d = Driver::new(g);
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 18, "Taps-Trigger");
}

#[test]
fn damage_done_once_triggers_once_per_recipient() {
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    put(&db, &mut g, "Combat Watcher Lite", P1, Zone::Battlefield);
    let a = put(&db, &mut g, "Goblin Piker", P1, Zone::Battlefield);
    let b = put(&db, &mut g, "Goblin Piker", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.run_until(|_, dec| matches!(dec, Decision::DeclareAttackers { .. }));
    d.submit(Choice::Attackers(vec![(a, T2), (b, T2)]));
    d.run_until(|g, _| g.step == Step::Main2);
    assert_eq!(d.g().player(P2).life, 16);
    assert_eq!(d.g().player(P1).hand.len(), 1, "einmal, nicht je Angreifer");
}

#[test]
fn changes_zone_all_triggers_once_per_simultaneous_event() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 4);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 1);
    put(&db, &mut g, "Mass Grief Lite", P1, Zone::Battlefield);
    let w = put(&db, &mut g, "Wrath of God", P1, Zone::Hand);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let piker = put(&db, &mut g, "Goblin Piker", P2, Zone::Battlefield);
    put_n(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield, 3);
    let mut d = Driver::new(g);
    d.cast(bolt, &[Target::Card(piker)]);
    d.pass_all();
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 19, "ein Tod (SBA)");
    d.cast(w, &[]);
    d.pass_all();
    assert_eq!(
        d.g().stack.len(),
        1,
        "ein Trigger für drei gleichzeitige Tode"
    );
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 16);
}

#[test]
fn ward_counters_unless_paid() {
    for (lands, pay, countered) in [(3usize, true, false), (3, false, true), (1, true, true)] {
        let (db, mut g) = game_at(Step::Main1, 10);
        put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, lands);
        let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
        let bear = put(&db, &mut g, "Warded Bear Lite", P2, Zone::Battlefield);
        let mut d = Driver::new(g);
        d.cast(bolt, &[Target::Card(bear)]);
        assert_eq!(d.g().stack.len(), 2, "Ward-Trigger über dem Blitz");
        d.pass_all();
        if lands == 3 {
            match d.decision() {
                Decision::PayUnlessCost { player, .. } => assert_eq!(*player, P1),
                o => panic!("{o:?}"),
            }
            d.submit(if pay { Choice::Yes } else { Choice::No });
        }
        d.pass_all();
        assert_eq!(
            count(d.g(), "Warded Bear Lite", Zone::Battlefield) == 1,
            countered,
            "lands={lands} pay={pay}"
        );
        assert_eq!(count(d.g(), "Lightning Bolt", Zone::Graveyard), 1);
    }
}

#[test]
fn becomes_target_only_by_own_spells() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 2);
    let hero = put(&db, &mut g, "Heroic Lite", P1, Zone::Battlefield);
    let bite = put(&db, &mut g, "Bite Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(bite, &[Target::Card(hero)]);
    assert_eq!(d.g().stack.len(), 2);
    d.pass_all();
    d.pass_all();
    assert_eq!(d.g().card(hero).counter("P1P1"), 1);
}

#[test]
fn attacker_blocked_by_creature_names_the_blocker() {
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    let thorn = put(&db, &mut g, "Thorn Lite", P1, Zone::Battlefield);
    let wall = put(&db, &mut g, "Wall of Stone", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.run_until(|_, dec| matches!(dec, Decision::DeclareAttackers { .. }));
    d.submit(Choice::Attackers(vec![(thorn, T2)]));
    d.run_until(|_, dec| matches!(dec, Decision::DeclareBlockers { .. }));
    d.submit(Choice::Blocks(vec![(wall, thorn)]));
    d.run_until(|g, _| g.step == Step::Main2);
    assert_eq!(count(d.g(), "Wall of Stone", Zone::Graveyard), 1);
}
