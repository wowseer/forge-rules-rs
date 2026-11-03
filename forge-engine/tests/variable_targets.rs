//! Variable Zielanzahl (`TargetMax$ X`).

mod common;
use common::*;
use forge_engine::decision::{Action, Choice, Decision, Target};
use forge_engine::{Step, Zone};

#[test]
fn up_to_x_targets() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 3);
    let bears: Vec<_> = (0..3)
        .map(|_| put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield))
        .collect();
    let s = put(&db, &mut g, "X Tap Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.act(Action::Cast(s));
    d.submit(Choice::Number(2));
    match d.decision() {
        Decision::ChooseTargets { min, max, .. } => assert_eq!((*min, *max), (0, 2)),
        o => panic!("{o:?}"),
    }
    d.finish_casting(&[Target::Card(bears[0]), Target::Card(bears[1])]);
    d.pass_all();
    assert!(d.g().card(bears[0]).tapped && d.g().card(bears[1]).tapped);
    assert!(!d.g().card(bears[2]).tapped);
}
