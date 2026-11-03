//! Getriggerte Mana-Fähigkeiten (`TapsForMana`, CR 605.1b) und `ManaReflected`.

mod common;
use common::*;
use forge_engine::decision::Action;
use forge_engine::{Step, Zone};

#[test]
fn flare_doubles_land_mana_immediately() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let m = put(&db, &mut g, "Mountain", P1, Zone::Battlefield);
    put(&db, &mut g, "Flare Lite", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.act(Action::Activate(m, 0));
    let pool = &d.g().player(P1).mana_pool;
    assert_eq!(pool.len(), 2, "rotes Mana doppelt");
    assert!(
        pool.units
            .iter()
            .all(|u| u.color == Some(forge_types::Color::Red))
    );
    assert!(d.g().stack.is_empty(), "ohne Stapel");
}

#[test]
fn flare_mana_used_when_auto_paying() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Mountain", P1, Zone::Battlefield);
    put(&db, &mut g, "Flare Lite", P1, Zone::Battlefield);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(bolt, &[T2]);
    assert_eq!(d.g().player(P1).mana_pool.len(), 1, "ein R übrig");
}
