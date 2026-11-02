//! Auren: statische Kontrolle (Layer 2, CR 613.1b), `Defined$ Enchanted`,
//! `Enchant:<Filter>:<Beschreibung>`.

mod common;
use common::*;
use forge_engine::decision::Target;
use forge_engine::{Step, Zone};

#[test]
fn control_aura_steals_until_it_leaves() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 1);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let aura = put(&db, &mut g, "Control Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(aura, &[Target::Card(bear)]);
    d.pass_all();
    assert_eq!(d.g().card(bear).controller, P1, "Kontrolle übernommen");
    // Aura zerstören (direkt), danach SBA → Kontrolle zurück
    let on_bf = find(d.g(), "Control Lite", Zone::Battlefield).unwrap();
    let mut g = d.g().clone();
    g.move_card(on_bf, Zone::Graveyard, true);
    let d = Driver::new(g);
    assert_eq!(d.g().card(bear).controller, P2, "zurück beim Besitzer");
}

#[test]
fn enchant_with_description_and_defined_enchanted() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 1);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let aura = put(&db, &mut g, "Tap Aura Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(aura, &[Target::Card(bear)]);
    d.pass_all(); // Aura
    d.pass_all(); // ETB-Trigger
    assert!(d.g().card(bear).tapped);
}
