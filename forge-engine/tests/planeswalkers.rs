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
