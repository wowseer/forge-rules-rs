//! `OptionalDecider$`: wer bei „may“-Triggern entscheidet.

mod common;
use common::*;
use forge_engine::decision::{Choice, Decision};
use forge_engine::{Step, Zone};

#[test]
fn triggered_card_controller_decides() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Gift Lite", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Hand);
    g.move_card(bear, Zone::Battlefield, true);
    let mut d = Driver::new(g);
    d.pass_all();
    match d.decision() {
        Decision::OptionalTrigger { player, .. } => {
            assert_eq!(*player, P2, "Controller der eintretenden Kreatur");
            d.submit(Choice::Yes);
        }
        o => panic!("{o:?}"),
    }
    assert_eq!(d.g().player(P2).hand.len(), 1);
}
