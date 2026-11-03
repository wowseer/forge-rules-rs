//! Aufteilen beim Wirken (`DividedAsYouChoose$`, CR 601.2d).

mod common;
use common::*;
use forge_engine::decision::{Action, Choice, Decision, Target};
use forge_engine::{Step, Zone};

#[test]
fn divided_damage_between_two_targets() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 1);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let s = put(&db, &mut g, "Forked Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.act(Action::Cast(s));
    d.submit(Choice::Targets(vec![T2, Target::Card(bear)]));
    // 2 auf 2 Ziele: zwingend je 1, keine Frage
    assert!(!matches!(d.decision(), Decision::ChooseX { .. }));
    d.finish_casting(&[]);
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 19);
    assert_eq!(d.g().card(bear).damage, 1);
}

#[test]
fn single_target_gets_everything_and_counters_split() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 1);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 1);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mine = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let other = put(&db, &mut g, "Hill Giant", P1, Zone::Battlefield);
    let s = put(&db, &mut g, "Forked Lite", P1, Zone::Hand);
    let c = put(&db, &mut g, "Spread Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(s, &[Target::Card(bear)]);
    d.pass_all();
    assert_eq!(d.g().card(bear).zone, None, "2 Schaden auf ein Ziel");
    d.act(Action::Cast(c));
    d.submit(Choice::Targets(vec![
        Target::Card(mine),
        Target::Card(other),
    ]));
    d.submit(Choice::Number(2));
    d.finish_casting(&[]);
    d.pass_all();
    assert_eq!(d.g().card(mine).counter("P1P1"), 2);
    assert_eq!(d.g().card(other).counter("P1P1"), 1);
}
