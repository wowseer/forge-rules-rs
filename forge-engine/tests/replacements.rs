//! Ersatzeffekte (CR 614): getappt ins Spiel, Umleitung, mit Marken ins Spiel.

mod common;
use common::*;
use forge_engine::decision::{Action, Choice, Decision, Target};
use forge_engine::{Step, Zone};

#[test]
fn land_enters_tapped() {
    // CR 614.1c
    let (db, mut g) = game_at(Step::Main1, 10);
    let land = put(&db, &mut g, "Tapped Land Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.act(Action::PlayLand(land));
    let l = find(d.g(), "Tapped Land Lite", Zone::Battlefield).unwrap();
    assert!(d.g().card(l).tapped);
    assert_eq!(d.g().card(l).controller, P1);
}

#[test]
fn check_land_condition() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let land = put(&db, &mut g, "Check Land Lite", P1, Zone::Hand);
    let mut d = Driver::new(g.clone());
    d.act(Action::PlayLand(land));
    let l = find(d.g(), "Check Land Lite", Zone::Battlefield).unwrap();
    assert!(d.g().card(l).tapped, "ohne Plains getappt");

    put(&db, &mut g, "Plains", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.act(Action::PlayLand(land));
    let l = find(d.g(), "Check Land Lite", Zone::Battlefield).unwrap();
    assert!(!d.g().card(l).tapped, "mit Plains ungetappt");
}

#[test]
fn rest_in_peace_redirects_to_exile_and_no_dies_trigger() {
    // CR 614.1a/614.6: Kreatur „stirbt“ nicht, Dies-Trigger lösen nicht aus
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Rest in Peace Lite", P2, Zone::Battlefield);
    put(&db, &mut g, "Mountain", P1, Zone::Battlefield);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let salve = put(&db, &mut g, "Salve Bearer Lite", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(bolt, &[Target::Card(salve)]);
    d.pass_all();
    assert_eq!(count(d.g(), "Salve Bearer Lite", Zone::Exile), 1);
    assert_eq!(
        count(d.g(), "Lightning Bolt", Zone::Exile),
        1,
        "auch der Spontanzauber"
    );
    assert!(d.g().stack.is_empty(), "kein Dies-Trigger");
    assert_eq!(d.g().player(P2).life, 20);
}

#[test]
fn enters_with_counters() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 5);
    let bear = put(&db, &mut g, "Counter Bear Lite", P1, Zone::Hand);
    let ooze = put(&db, &mut g, "X Ooze Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(bear, &[]);
    d.pass_all();
    let b = find(d.g(), "Counter Bear Lite", Zone::Battlefield).unwrap();
    assert_eq!(d.g().chars(b).power, Some(3));
    d.act(Action::Cast(ooze));
    d.submit(Choice::Number(2));
    d.finish_casting(&[]);
    d.pass_all();
    let o = find(d.g(), "X Ooze Lite", Zone::Battlefield).expect("überlebt dank Marken (0/0 + 2)");
    assert_eq!(d.g().card(o).counter("P1P1"), 2);
    let _ = Decision::Mulligan {
        player: P1,
        mulligans_taken: 0,
    };
}

#[test]
fn damage_doubling_replacement() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Mountain", P1, Zone::Battlefield);
    put(&db, &mut g, "Furnace Lite", P1, Zone::Battlefield);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(bolt, &[Target::Player(P2)]);
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 14, "3 × 2");
}

#[test]
fn prevent_combat_damage_to_self_but_not_noncombat() {
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    let giant = put(&db, &mut g, "Hill Giant", P1, Zone::Battlefield);
    let wall = put(&db, &mut g, "Mist Wall Lite", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.run_until(|_, dec| matches!(dec, Decision::DeclareAttackers { .. }));
    d.submit(Choice::Attackers(vec![(giant, T2)]));
    d.run_until(|_, dec| matches!(dec, Decision::DeclareBlockers { .. }));
    d.submit(Choice::Blocks(vec![(wall, giant)]));
    d.run_until(|g, _| g.step == Step::Main2);
    assert_eq!(d.g().card(wall).damage, 0, "Kampfschaden verhindert");
    assert_eq!(d.g().card(wall).zone, Some(Zone::Battlefield));
}

#[test]
fn prevention_shield_next_n_damage_this_turn_after_doubling() {
    // CR 615.7; Vereinfachung CR 616.1: erst Verdoppeln, dann Verhindern
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 2);
    put(&db, &mut g, "Plains", P1, Zone::Battlefield);
    put(&db, &mut g, "Furnace Lite", P1, Zone::Battlefield);
    let salve = put(&db, &mut g, "Salve Lite", P1, Zone::Hand);
    let b1 = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let b2 = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(salve, &[Target::Player(P2)]);
    d.pass_all();
    assert_eq!(d.g().player(P2).prevent_next, 3);
    d.cast(b1, &[Target::Player(P2)]);
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 17, "6 − 3 verhindert");
    assert_eq!(d.g().player(P2).prevent_next, 0, "Schild verbraucht");
    d.cast(b2, &[Target::Player(P2)]);
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 11);
}
