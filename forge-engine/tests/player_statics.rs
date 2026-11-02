//! Statische Fähigkeiten, die Spieler betreffen (`Affected$ You|Opponent`):
//! maximale Handgröße (CR 402.2, 514.1), Fluchsicherheit für Spieler (CR 702.11d).

mod common;
use common::*;
use forge_engine::decision::{Action, Decision, Target};
use forge_engine::{Step, Zone};

fn hand_after_cleanup(
    setup: impl FnOnce(&forge_engine::CardDb, &mut forge_engine::GameState),
) -> usize {
    let (db, mut g) = game_at(Step::Cleanup, 10);
    put_n(&db, &mut g, "Island", P1, Zone::Hand, 10);
    setup(&db, &mut g);
    let mut d = Driver::new(g);
    d.run_until(|g, _| g.active == P2);
    d.g().player(P1).hand.len()
}

#[test]
fn no_maximum_hand_size() {
    assert_eq!(hand_after_cleanup(|_, _| {}), 7);
    let n = hand_after_cleanup(|db, g| {
        put(db, g, "Tower Lite", P1, Zone::Battlefield);
    });
    assert_eq!(n, 10);
}

#[test]
fn opponent_hand_size_reduced_but_not_own() {
    let n = hand_after_cleanup(|db, g| {
        put(db, g, "Squeeze Lite", P2, Zone::Battlefield);
    });
    assert_eq!(n, 5, "7 − 2 für den Gegner des Controllers");
    let n = hand_after_cleanup(|db, g| {
        put(db, g, "Squeeze Lite", P1, Zone::Battlefield);
    });
    assert_eq!(n, 7, "eigene Handgröße unverändert");
}

#[test]
fn player_hexproof_only_against_opponents() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Swamp", P1, Zone::Battlefield, 3);
    put(&db, &mut g, "Sanctity Lite", P2, Zone::Battlefield);
    let rot = put(&db, &mut g, "Mind Rot Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.act(Action::Cast(rot));
    match d.decision().clone() {
        Decision::ChooseTargets { candidates, .. } => {
            assert!(
                !candidates.contains(&Target::Player(P2)),
                "Gegner hat Fluchsicherheit"
            );
            assert!(
                candidates.contains(&Target::Player(P1)),
                "sich selbst darf man anvisieren"
            );
        }
        o => panic!("{o:?}"),
    }
}
