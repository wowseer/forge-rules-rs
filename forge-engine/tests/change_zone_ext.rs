//! ChangeZone: `Shuffle$`, `Optional$`, Suche in mehreren Zonen.

mod common;
use common::*;
use forge_engine::decision::{CardChoiceReason, Choice, Decision, Target};
use forge_engine::{Step, Zone};

#[test]
fn shuffle_into_library() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 1);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Graveyard);
    let spell = put(&db, &mut g, "Tuck Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(spell, &[Target::Card(bear)]);
    d.pass_all();
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Library), 1);
    assert_eq!(d.g().player(P2).library.len(), 11);
}

#[test]
fn optional_move_can_be_declined() {
    for (yes, in_hand) in [(true, 1), (false, 0)] {
        let (db, mut g) = game_at(Step::Main1, 10);
        put_n(&db, &mut g, "Swamp", P1, Zone::Battlefield, 1);
        let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Graveyard);
        let spell = put(&db, &mut g, "Maybe Return Lite", P1, Zone::Hand);
        let mut d = Driver::new(g);
        d.cast(spell, &[Target::Card(bear)]);
        d.pass_all();
        match d.decision() {
            Decision::Confirm { .. } => d.submit(if yes { Choice::Yes } else { Choice::No }),
            o => panic!("{o:?}"),
        }
        assert_eq!(count(d.g(), "Grizzly Bears", Zone::Hand), in_hand);
    }
}

#[test]
fn search_graveyard_and_exile() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 1);
    let gy = put(&db, &mut g, "Grizzly Bears", P1, Zone::Graveyard);
    let ex = put(&db, &mut g, "Hill Giant", P1, Zone::Exile);
    let spell = put(&db, &mut g, "Recall Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(spell, &[]);
    d.pass_all();
    match d.decision().clone() {
        Decision::ChooseCards {
            reason: CardChoiceReason::Search,
            options,
            ..
        } => {
            assert!(options.contains(&gy) && options.contains(&ex));
            d.submit(Choice::Cards(vec![ex]));
        }
        o => panic!("{o:?}"),
    }
    assert_eq!(count(d.g(), "Hill Giant", Zone::Hand), 1);
}
