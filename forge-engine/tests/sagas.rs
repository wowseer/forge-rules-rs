//! Sagas (CR 714): Lore-Marken, Kapitelfähigkeiten, Opfern nach dem letzten Kapitel.

mod common;
use common::*;
use forge_engine::decision::Decision;
use forge_engine::{Step, Zone};

#[test]
fn saga_chapters_trigger_and_saga_is_sacrificed_after_last() {
    let (db, mut g) = game_at(Step::Main1, 20);
    put(&db, &mut g, "Plains", P1, Zone::Battlefield);
    let saga = put(&db, &mut g, "Saga Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(saga, &[]);
    d.pass_all(); // Saga löst auf, Kapitel I wird ausgelöst (CR 714.3a)
    let on_bf = find(d.g(), "Saga Lite", Zone::Battlefield).expect("Saga im Spiel");
    assert_eq!(d.g().card(on_bf).counter("LORE"), 1);
    assert!(!d.g().stack.is_empty(), "Kapitel I auf dem Stapel");
    d.pass_all();
    assert_eq!(d.g().player(P1).life, 22);

    // Nächster eigener Zug: Lore-Marke zu Beginn der ersten Hauptphase (CR 714.3b)
    d.run_until(|g, dec| {
        g.turn == 4 && g.step == Step::Main1 && matches!(dec, Decision::Priority { .. })
    });
    assert_eq!(d.g().card(on_bf).counter("LORE"), 2);
    d.pass_all();
    assert_eq!(d.g().player(P1).life, 24, "Kapitel II");

    d.run_until(|g, dec| {
        g.turn == 6 && g.step == Step::Main1 && matches!(dec, Decision::Priority { .. })
    });
    // CR 714.4: solange Kapitel III auf dem Stapel liegt, bleibt die Saga
    assert_eq!(d.g().card(on_bf).zone, Some(Zone::Battlefield));
    let hand = d.g().player(P1).hand.len();
    d.pass_all();
    assert_eq!(d.g().player(P1).hand.len(), hand + 1, "Kapitel III");
    assert_eq!(
        count(d.g(), "Saga Lite", Zone::Graveyard),
        1,
        "danach geopfert"
    );
}

#[test]
fn adding_several_lore_counters_triggers_each_chapter() {
    // CR 714.2b: jedes überschrittene Kapitel löst aus
    let (db, mut g) = game_at(Step::Main1, 20);
    let saga = put(&db, &mut g, "Saga Lite", P1, Zone::Battlefield);
    g.add_counters(saga, "LORE", 2);
    assert_eq!(g.pending_triggers.len(), 2);
}
