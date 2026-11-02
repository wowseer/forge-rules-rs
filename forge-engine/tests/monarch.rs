//! Monarch (CR 724).

mod common;
use common::*;
use forge_engine::decision::{Choice, Decision};
use forge_engine::{Step, Zone};

#[test]
fn monarch_draws_at_end_step_and_static_condition() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 1);
    let regal = put(&db, &mut g, "Regal Lite", P1, Zone::Battlefield);
    let crown = put(&db, &mut g, "Crown Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    assert_eq!(d.g().chars(regal).power, Some(2));
    d.cast(crown, &[]);
    d.pass_all();
    assert_eq!(d.g().monarch, Some(P1));
    assert_eq!(d.g().chars(regal).power, Some(4), "Condition$ Monarch");
    let hand = d.g().player(P1).hand.len();
    d.run_until(|g, _| g.step == Step::End);
    assert_eq!(
        d.g().player(P1).hand.len(),
        hand + 1,
        "zieht im eigenen Endsegment"
    );
}

#[test]
fn combat_damage_to_monarch_steals_crown() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    g.monarch = Some(P2);
    let mut d = Driver::new(g);
    d.run_until(|_, dec| matches!(dec, Decision::DeclareAttackers { .. }));
    d.submit(Choice::Attackers(vec![(bear, T2)]));
    d.run_until(|g, _| g.step == Step::EndCombat);
    assert_eq!(d.g().monarch, Some(P1));
}
