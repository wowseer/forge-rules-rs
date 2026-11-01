//! Planeswalker: Loyalität (CR 306.5b), Loyalitätsfähigkeiten (CR 606).

mod common;
use common::*;
use forge_engine::decision::{Action, Decision, Target};
use forge_engine::{Step, Zone};

fn actions(d: &Driver) -> Vec<Action> {
    match d.decision() {
        Decision::Priority { actions, .. } => actions.clone(),
        o => panic!("{o:?}"),
    }
}

fn walker(d: &Driver) -> forge_engine::CardId {
    find(d.g(), "Ajani Lite", Zone::Battlefield).expect("Ajani im Spiel")
}

#[test]
fn enters_with_loyalty_and_activates_once_per_turn_at_sorcery_speed() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 3);
    let a = put(&db, &mut g, "Ajani Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(a, &[]);
    d.pass_all();
    let w = walker(&d);
    assert_eq!(d.g().card(w).counter("LOYALTY"), 3, "CR 306.5b");
    // -3 nicht bezahlbar wäre 0 → erlaubt (3 ≥ 3); +1 zuerst
    d.act(Action::Activate(w, 0));
    // CR 606.3: Stapel nicht leer → keine weitere Loyalitätsfähigkeit
    assert!(!actions(&d).contains(&Action::Activate(w, 1)));
    d.pass_all();
    assert_eq!(d.g().card(w).counter("LOYALTY"), 4);
    assert_eq!(d.g().player(P1).life, 22);
    // CR 606.3: nur einmal pro Zug
    assert!(!actions(&d).contains(&Action::Activate(w, 0)));
    assert!(!actions(&d).contains(&Action::Activate(w, 1)));
}

#[test]
fn minus_ability_to_zero_loyalty_goes_to_graveyard() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 3);
    let a = put(&db, &mut g, "Ajani Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(a, &[]);
    d.pass_all();
    let w = walker(&d);
    d.act(Action::Activate(w, 1));
    d.finish_casting(&[Target::Player(P2)]);
    // CR 704.5i: 0 Loyalität → Friedhof, Fähigkeit löst trotzdem auf (CR 113.7a)
    d.pass_all();
    assert_eq!(count(d.g(), "Ajani Lite", Zone::Graveyard), 1);
    assert_eq!(d.g().player(P2).life, 17);
}

#[test]
fn damage_to_planeswalker_removes_loyalty() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 3);
    put(&db, &mut g, "Mountain", P2, Zone::Battlefield);
    let a = put(&db, &mut g, "Ajani Lite", P1, Zone::Hand);
    let bolt = put(&db, &mut g, "Lightning Bolt", P2, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(a, &[]);
    d.pass_all();
    let w = walker(&d);
    d.act(Action::Activate(w, 0));
    // P2 antwortet mit Blitz auf Ajani (4 Loyalität nach +1? Kosten sind schon bezahlt)
    d.act(Action::Pass);
    d.cast(bolt, &[Target::Card(w)]);
    d.pass_all();
    assert_eq!(d.g().card(w).counter("LOYALTY"), 1, "4 − 3 (CR 120.3c)");
}

/// Ajani unter P2s Kontrolle direkt ins Spiel (mit Loyalität), P1 im Kampf.
fn ajani_for_p2(lands_p1: usize) -> (Driver, forge_engine::CardId) {
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, lands_p1);
    let a = put(&db, &mut g, "Ajani Lite", P2, Zone::Battlefield);
    g.add_counters(a, "LOYALTY", 3);
    (Driver::new(g), a)
}

#[test]
fn creatures_can_attack_planeswalkers_and_damage_removes_loyalty() {
    let (mut d, _) = ajani_for_p2(0);
    let mut g = d.g().clone();
    let db = db();
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let w = find(&g, "Ajani Lite", Zone::Battlefield).unwrap();
    d = Driver::new(g);
    d.run_until(|_, dec| matches!(dec, Decision::DeclareAttackers { .. }));
    match d.decision() {
        Decision::DeclareAttackers { defenders, .. } => {
            assert!(defenders.contains(&Target::Card(w)), "CR 508.1b");
            assert!(defenders.contains(&T2));
        }
        o => panic!("{o:?}"),
    }
    d.submit(forge_engine::decision::Choice::Attackers(vec![(
        bear,
        Target::Card(w),
    )]));
    d.run_until(|g, _| g.step == Step::Main2);
    assert_eq!(
        d.g().card(w).counter("LOYALTY"),
        1,
        "2 Schaden → 2 Loyalität weniger"
    );
    assert_eq!(d.g().player(P2).life, 20, "Spieler unversehrt");
}

#[test]
fn attacked_planeswalker_removed_means_no_combat_damage() {
    // CR 506.4c: der Angreifer bleibt angreifend, verursacht aber keinen Schaden
    let (d, _) = ajani_for_p2(1);
    let mut g = d.g().clone();
    let db = db();
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let w = find(&g, "Ajani Lite", Zone::Battlefield).unwrap();
    let mut d = Driver::new(g);
    d.run_until(|_, dec| matches!(dec, Decision::DeclareAttackers { .. }));
    d.submit(forge_engine::decision::Choice::Attackers(vec![(
        bear,
        Target::Card(w),
    )]));
    d.run_until(|g, dec| {
        g.step == Step::DeclareAttackers && matches!(dec, Decision::Priority { .. })
    });
    d.cast(bolt, &[Target::Card(w)]);
    d.pass_all();
    assert_eq!(count(d.g(), "Ajani Lite", Zone::Graveyard), 1);
    d.run_until(|g, _| g.step == Step::Main2);
    assert_eq!(d.g().player(P2).life, 20);
}
