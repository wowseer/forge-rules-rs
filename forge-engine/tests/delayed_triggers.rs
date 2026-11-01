//! Verzögerte Trigger (`DelayedTrigger`, CR 603.7).

mod common;
use common::*;
use forge_engine::decision::Target;
use forge_engine::{Step, Zone};

#[test]
fn flicker_returns_at_next_end_step_following_the_card() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Plains", P1, Zone::Battlefield);
    let f = put(&db, &mut g, "Flicker Lite", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(f, &[Target::Card(bear)]);
    d.pass_all();
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Exile), 1);
    assert_eq!(d.g().delayed_triggers.len(), 1);
    d.run_until(|g, _| g.step == Step::End && !g.stack.is_empty());
    d.pass_all();
    assert_eq!(
        count(d.g(), "Grizzly Bears", Zone::Battlefield),
        1,
        "zurück aus dem Exil"
    );
    assert!(d.g().delayed_triggers.is_empty(), "CR 603.7c: nur einmal");
}

#[test]
fn delayed_trigger_fires_once_at_your_next_upkeep() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Island", P1, Zone::Battlefield);
    let s = put(&db, &mut g, "Next Upkeep Draw Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(s, &[]);
    d.pass_all();
    // P2s Zug (ValidPlayer$ You): nichts; P1s nächster Versorgungsschritt: ziehen
    d.run_until(|g, _| g.turn == 4 && g.step == Step::Main1);
    // 1 (Versorgung) + 1 (Ziehschritt)
    assert_eq!(d.g().player(P1).hand.len(), 2);
    d.run_until(|g, _| g.turn == 6 && g.step == Step::Main1);
    assert_eq!(
        d.g().player(P1).hand.len(),
        3,
        "nicht erneut (nur der Ziehschritt)"
    );
}

#[test]
fn this_turn_delayed_trigger_expires_at_cleanup() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Island", P1, Zone::Battlefield);
    let s = put(&db, &mut g, "This Turn Upkeep Draw Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(s, &[]);
    d.pass_all();
    assert_eq!(d.g().delayed_triggers.len(), 1);
    d.run_until(|g, _| g.turn == 3 && g.step == Step::Main1);
    assert!(d.g().delayed_triggers.is_empty());
    assert_eq!(d.g().player(P2).hand.len(), 1, "nur der eigene Ziehschritt");
}
