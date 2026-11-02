//! „… bis [Quelle] das Spielfeld verlässt“ (CR 610.3), `ExiledWith`.

mod common;
use common::*;
use forge_engine::decision::Target;
use forge_engine::{Step, Zone};

#[test]
fn banisher_priest_exiles_until_it_leaves() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 3);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let priest = put(&db, &mut g, "Banisher Priest", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(priest, &[]);
    d.pass_all(); // Kreatur
    d.finish_casting(&[Target::Card(bear)]); // ETB-Ziel
    d.pass_all(); // ETB löst auf
    let exiled = find(d.g(), "Grizzly Bears", Zone::Exile).expect("im Exil");
    let p = find(d.g(), "Banisher Priest", Zone::Battlefield).unwrap();
    assert_eq!(d.g().card(exiled).exiled_with, Some(p));
    let spec = forge_engine::valid::ValidSpec::parse("Card.ExiledWithSource").unwrap();
    assert!(d.g().card_matches(exiled, &spec, p, P1));
    // Priester verlässt das Spielfeld → Bär kehrt unter Kontrolle des Besitzers zurück
    let mut g = d.g().clone();
    g.move_card(p, Zone::Graveyard, true);
    let back = find(&g, "Grizzly Bears", Zone::Battlefield).expect("zurück");
    assert_eq!(g.card(back).controller, P2);
    assert!(g.exile.is_empty());
}
