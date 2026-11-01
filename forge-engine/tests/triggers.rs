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
    d.submit(Choice::Attackers(vec![(gob, P2)]));
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
    d.submit(Choice::Attackers(vec![(ninja, P2)]));
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
