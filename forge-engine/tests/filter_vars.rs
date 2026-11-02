//! Filter mit Variablen (`cmcLEX`, Java `CardProperty` mit `calculateAmount`).

mod common;
use common::*;
use forge_engine::decision::{Action, CardChoiceReason, Choice, Decision, Target};
use forge_engine::{Step, Zone};

#[test]
fn target_filter_uses_chosen_x() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Swamp", P1, Zone::Battlefield, 3);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let giant = put(&db, &mut g, "Hill Giant", P2, Zone::Battlefield);
    let decay = put(&db, &mut g, "Decay Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.act(Action::Cast(decay));
    match d.decision() {
        Decision::ChooseX { .. } => d.submit(Choice::Number(2)),
        o => panic!("{o:?}"),
    }
    match d.decision().clone() {
        Decision::ChooseTargets { candidates, .. } => {
            assert!(candidates.contains(&Target::Card(bear)), "Manawert 2 ≤ X");
            assert!(!candidates.contains(&Target::Card(giant)), "Manawert 4 > X");
        }
        o => panic!("{o:?}"),
    }
    d.finish_casting(&[Target::Card(bear)]);
    d.pass_all();
    assert_eq!(d.g().card(bear).zone, None);
}

#[test]
fn library_search_filter_uses_x_at_resolution() {
    let (db, mut g) = game_at(Step::Main1, 3);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 3);
    put(&db, &mut g, "Hill Giant", P1, Zone::Library);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Library);
    let zenith = put(&db, &mut g, "Zenith Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.act(Action::Cast(zenith));
    match d.decision() {
        Decision::ChooseX { .. } => d.submit(Choice::Number(2)),
        o => panic!("{o:?}"),
    }
    d.finish_casting(&[]);
    d.pass_all();
    match d.decision().clone() {
        Decision::ChooseCards {
            reason: CardChoiceReason::Search,
            options,
            ..
        } => {
            assert_eq!(options, vec![bear], "nur Manawert ≤ 2");
            d.submit(Choice::Cards(vec![bear]));
        }
        o => panic!("{o:?}"),
    }
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Battlefield), 1);
}
