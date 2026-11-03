//! Storm (CR 702.40), Cascade (CR 702.85), Rebound (CR 702.88), Partner (Deckbau).

mod common;
use common::*;
use forge_engine::decision::{Choice, Decision};
use forge_engine::{Step, Zone};

#[test]
fn storm_copies_for_each_earlier_spell() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 3);
    let b1 = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let b2 = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let s = put(&db, &mut g, "Storm Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(b1, &[T2]);
    d.pass_all();
    d.cast(b2, &[T2]);
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 14);
    d.cast(s, &[T2]);
    d.pass_all(); // Storm-Trigger: 2 Kopien, je „neue Ziele?“ → nein
    for _ in 0..2 {
        match d.decision() {
            Decision::Confirm { .. } => d.submit(Choice::No),
            o => panic!("{o:?}"),
        }
    }
    d.run_until(|g, dec| g.stack.is_empty() && matches!(dec, Decision::Priority { .. }));
    assert_eq!(d.g().player(P2).life, 11, "Original + 2 Kopien");
}

#[test]
fn cascade_casts_cheaper_nonland_for_free() {
    let (db, mut g) = game_at(Step::Main1, 0);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 4);
    put(&db, &mut g, "Hill Giant", P1, Zone::Library); // MV 4, nicht < 4
    let bears = put(&db, &mut g, "Grizzly Bears", P1, Zone::Library);
    put(&db, &mut g, "Forest", P1, Zone::Library); // oben: Land
    let c = put(&db, &mut g, "Cascade Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(c, &[]);
    d.pass_all(); // Kaskade-Trigger
    match d.decision() {
        Decision::Confirm { .. } => d.submit(Choice::Yes),
        o => panic!("{o:?}"),
    }
    d.run_until(|g, dec| g.stack.is_empty() && matches!(dec, Decision::Priority { .. }));
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Battlefield), 1);
    assert_eq!(d.g().card(bears).zone, None);
    assert_eq!(d.g().player(P1).life, 21);
    // Forest (aufgedeckt) unten in der Bibliothek, Riese blieb oben liegen
    assert_eq!(d.g().player(P1).library.len(), 2);
    assert!(d.g().exile.is_empty());
}

#[test]
fn rebound_recasts_next_upkeep() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 1);
    let r = put(&db, &mut g, "Rebound Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(r, &[]);
    d.pass_all();
    assert_eq!(d.g().player(P1).life, 23);
    assert_eq!(count(d.g(), "Rebound Lite", Zone::Exile), 1);
    d.run_until(|g, dec| g.active == P1 && g.turn == 4 && matches!(dec, Decision::Confirm { .. }));
    d.submit(Choice::Yes);
    d.run_until(|g, dec| g.stack.is_empty() && matches!(dec, Decision::Priority { .. }));
    assert_eq!(d.g().player(P1).life, 26);
    assert_eq!(count(d.g(), "Rebound Lite", Zone::Graveyard), 1);
}

#[test]
fn partner_is_supported() {
    let db = db();
    assert!(db.get("Partner Lite").unwrap().is_supported());
}
