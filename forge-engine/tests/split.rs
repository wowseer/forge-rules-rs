//! Split-Karten (CR 709), Aftermath (CR 702.127), Split second (CR 702.61).

mod common;
use common::*;
use forge_engine::decision::{Action, Target};
use forge_engine::{Step, Zone};

#[test]
fn split_card_combined_outside_stack_and_halves_castable() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 1);
    let card = put(&db, &mut g, "Wax Lite", P1, Zone::Hand);
    put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    put(&db, &mut g, "Prowler Lite", P2, Zone::Battlefield); // Ziel für Wane
    let ch = g.chars(card);
    assert_eq!(ch.mana_cost.unwrap().mana_value(), 2, "CR 709.4b: Summe");
    assert!(
        ch.colors.contains(forge_types::Color::Green)
            && ch.colors.contains(forge_types::Color::White)
    );
    let acts = g.legal_actions(P1);
    // je Hälfte wirkbar (Bezahlbarkeit grob über den Manawert der Hälfte: 1 ≤ 1)
    assert!(acts.contains(&Action::Cast(card)));
    assert!(acts.contains(&Action::CastAlt(card, 1)));
}

#[test]
fn cast_left_half_uses_its_own_cost() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 1);
    let card = put(&db, &mut g, "Wax Lite", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(card, &[Target::Card(bear)]);
    d.pass_all();
    assert_eq!(d.g().chars(bear).power, Some(6));
}

#[test]
fn aftermath_half_only_from_graveyard_then_exiled() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 5);
    let hand = put(&db, &mut g, "Dusk Lite", P1, Zone::Hand);
    let gy = put(&db, &mut g, "Dusk Lite", P1, Zone::Graveyard);
    let acts = g.legal_actions(P1);
    assert!(acts.contains(&Action::Cast(hand)));
    assert!(
        !acts.contains(&Action::CastAlt(hand, 1)),
        "Aftermath nicht aus der Hand"
    );
    assert!(acts.contains(&Action::CastAlt(gy, 1)));
    assert!(!acts.contains(&Action::Cast(gy)));
    let mut d = Driver::new(g);
    d.act(Action::CastAlt(gy, 1));
    d.finish_casting(&[]);
    d.pass_all();
    assert_eq!(d.g().player(P1).life, 23);
    assert_eq!(count(d.g(), "Dusk Lite", Zone::Exile), 1);
}

#[test]
fn split_second_locks_out_responses() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 3);
    let s = put(&db, &mut g, "Sudden Lite", P1, Zone::Hand);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(s, &[T2]);
    let acts = d.g().legal_actions(P1);
    assert!(!acts.contains(&Action::Cast(bolt)));
    assert!(acts.contains(&Action::Pass));
}
