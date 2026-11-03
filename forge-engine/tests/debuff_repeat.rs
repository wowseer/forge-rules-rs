//! Debuff, Repeat, MinMaxBlocker.

mod common;
use common::*;
use forge_engine::decision::Target;
use forge_engine::keyword::Keyword;
use forge_engine::{Step, Zone};

#[test]
fn debuff_removes_flying_until_end_of_turn() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 1);
    let angel = put(&db, &mut g, "Serra Angel", P2, Zone::Battlefield);
    let s = put(&db, &mut g, "Ground Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(s, &[Target::Card(angel)]);
    d.pass_all();
    assert!(!d.g().chars(angel).has(&Keyword::Flying));
    assert!(d.g().chars(angel).has(&Keyword::Vigilance));
    d.run_until(|g, _| g.active == P2);
    assert!(d.g().chars(angel).has(&Keyword::Flying));
}

#[test]
fn repeat_until_condition_fails() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Swamp", P1, Zone::Battlefield, 1);
    let s = put(&db, &mut g, "Repeat Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(s, &[]);
    d.pass_all();
    assert_eq!(
        d.g().player(P1).cards_drawn_this_turn,
        3,
        "ziehen, solange < 3"
    );
}

#[test]
fn max_one_blocker() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let beast = put(&db, &mut g, "Lone Blocker Lite", P1, Zone::Battlefield);
    let b1 = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let b2 = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    assert!(g.validate_blocks(&[(b1, beast)]).is_ok());
    assert!(g.validate_blocks(&[(b1, beast), (b2, beast)]).is_err());
}
