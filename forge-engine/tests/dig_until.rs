//! `DigUntil` (Java `DigUntilEffect`): aufdecken, bis passende Karten gefunden sind.

mod common;
use common::*;
use forge_engine::{Step, Zone};

#[test]
fn reveal_until_creature_onto_battlefield_rest_bottom() {
    let (db, mut g) = game_at(Step::Main1, 5);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 1);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Library);
    let above: Vec<_> = (0..3)
        .map(|_| put(&db, &mut g, "Mountain", P1, Zone::Library))
        .collect();
    let dig = put(&db, &mut g, "Dig Creature Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(dig, &[]);
    d.pass_all();
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Battlefield), 1);
    assert_eq!(d.g().card(bear).zone, None);
    let lib = &d.g().player(P1).library;
    assert_eq!(lib.len(), 8);
    // die drei Berge liegen (neue Objekte) ganz unten, die Inseln oben
    let bottom: Vec<_> = lib[..3]
        .iter()
        .map(|c| d.g().card(*c).def.name().to_string())
        .collect();
    assert_eq!(bottom, vec!["Mountain"; 3]);
    assert!(above.iter().all(|c| d.g().card(*c).zone.is_none()));
}

#[test]
fn mill_until_land_sequential_to_graveyard() {
    let (db, mut g) = game_at(Step::Main1, 5);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 1);
    put(&db, &mut g, "Forest", P2, Zone::Library);
    put_n(&db, &mut g, "Grizzly Bears", P2, Zone::Library, 2);
    let mill = put(&db, &mut g, "Mill Until Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(mill, &[T2]);
    d.pass_all();
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Graveyard), 2);
    assert_eq!(count(d.g(), "Forest", Zone::Graveyard), 1);
    assert_eq!(d.g().player(P2).library.len(), 5);
}

#[test]
fn until_two_found_remembers_and_exiles_rest() {
    let (db, mut g) = game_at(Step::Main1, 0);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 1);
    put(&db, &mut g, "Forest", P1, Zone::Library);
    put(&db, &mut g, "Grizzly Bears", P1, Zone::Library);
    put(&db, &mut g, "Plains", P1, Zone::Library);
    put(&db, &mut g, "Hill Giant", P1, Zone::Library);
    let dig = put(&db, &mut g, "Two Lands Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(dig, &[]);
    d.pass_all();
    assert_eq!(count(d.g(), "Plains", Zone::Hand), 1);
    assert_eq!(count(d.g(), "Forest", Zone::Hand), 1);
    assert_eq!(count(d.g(), "Hill Giant", Zone::Exile), 1);
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Exile), 1);
    assert!(d.g().player(P1).library.is_empty());
}
