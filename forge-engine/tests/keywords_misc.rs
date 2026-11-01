//! Cycling/Typecycling (CR 702.29), Devoid (CR 702.114).

mod common;
use common::*;
use forge_engine::decision::{Action, Choice, Decision};
use forge_engine::{Step, Zone};

#[test]
fn cycling_from_hand_discards_and_draws() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 2);
    let c = put(&db, &mut g, "Cycler Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    match d.decision() {
        Decision::Priority { actions, .. } => assert!(actions.contains(&Action::Activate(c, 0))),
        o => panic!("{o:?}"),
    }
    d.act(Action::Activate(c, 0));
    d.finish_casting(&[]);
    assert_eq!(
        count(d.g(), "Cycler Lite", Zone::Graveyard),
        1,
        "Abwerfen ist Kosten"
    );
    d.pass_all();
    assert_eq!(d.g().player(P1).hand.len(), 1, "eine Karte gezogen");
}

#[test]
fn cycling_not_offered_from_battlefield_or_without_mana() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Island", P1, Zone::Battlefield);
    let h = put(&db, &mut g, "Cycler Lite", P1, Zone::Hand);
    let b = put(&db, &mut g, "Cycler Lite", P1, Zone::Battlefield);
    let d = Driver::new(g);
    match d.decision() {
        Decision::Priority { actions, .. } => {
            assert!(!actions.contains(&Action::Activate(h, 0)), "nur 1 Mana");
            assert!(
                !actions.contains(&Action::Activate(b, 0)),
                "nur aus der Hand"
            );
        }
        o => panic!("{o:?}"),
    }
}

#[test]
fn typecycling_searches_library_for_type() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Plains", P1, Zone::Battlefield);
    let plains = put(&db, &mut g, "Plains", P1, Zone::Library);
    let c = put(&db, &mut g, "Plains Cycler Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.act(Action::Activate(c, 0));
    d.finish_casting(&[]);
    d.pass_all();
    match d.decision() {
        Decision::ChooseCards { options, .. } => {
            assert_eq!(options, &vec![plains], "nur Ebenen");
            d.submit(Choice::Cards(vec![plains]));
        }
        o => panic!("{o:?}"),
    }
    assert_eq!(count(d.g(), "Plains", Zone::Hand), 1);
}

#[test]
fn devoid_is_colorless_in_all_zones() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let h = put(&db, &mut g, "Devoid Lite", P1, Zone::Hand);
    let b = put(&db, &mut g, "Devoid Lite", P1, Zone::Battlefield);
    assert!(g.chars(h).colors.is_colorless());
    assert!(g.chars(b).colors.is_colorless());
    assert_eq!(g.chars(b).mana_cost.as_ref().unwrap().mana_value(), 2);
}
