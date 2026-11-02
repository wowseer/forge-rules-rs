//! Crew (CR 702.122).

mod common;
use common::*;
use forge_engine::decision::{Action, Choice, Decision};
use forge_engine::{Step, Zone};

#[test]
fn crew_with_enough_power_animates_until_end_of_turn() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let v = put(&db, &mut g, "Vehicle Lite", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    assert!(!d.g().chars(v).types.is_creature());
    d.act(Action::Activate(v, 0));
    match d.decision().clone() {
        Decision::ChooseCards { options, min, .. } => {
            assert_eq!(options, vec![bear], "nicht das Fahrzeug selbst");
            assert_eq!(min, 1);
            d.submit(Choice::Cards(vec![bear]));
        }
        o => panic!("{o:?}"),
    }
    assert!(d.g().card(bear).tapped);
    d.pass_all();
    assert!(d.g().chars(v).types.is_creature());
    assert_eq!(d.g().chars(v).power, Some(3));
    d.run_until(|g, _| g.active == P2);
    assert!(!d.g().chars(v).types.is_creature(), "bis Zugende");
}

#[test]
fn crew_needs_total_power() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let v = put(&db, &mut g, "Vehicle Lite", P1, Zone::Battlefield);
    put(&db, &mut g, "Raging Goblin", P1, Zone::Battlefield);
    assert!(
        !g.legal_actions(P1).contains(&Action::Activate(v, 0)),
        "Stärke 1 < 2"
    );
    put(&db, &mut g, "Raging Goblin", P1, Zone::Battlefield);
    assert!(
        g.legal_actions(P1).contains(&Action::Activate(v, 0)),
        "1 + 1 reicht"
    );
}
