//! Statics `CantBeCast`/`CantBeActivated` (CR 101.2: „kann nicht“ gewinnt).

mod common;
use common::*;
use forge_engine::decision::Action;
use forge_engine::{Step, Zone};

#[test]
fn opponents_cant_cast_spells() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Silence Lite", P2, Zone::Battlefield);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 1);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    assert!(!g.legal_actions(P1).contains(&Action::Cast(bolt)));
    let (db2, mut g2) = game_at(Step::Main1, 10);
    put(&db2, &mut g2, "Silence Lite", P1, Zone::Battlefield);
    put_n(&db2, &mut g2, "Mountain", P1, Zone::Battlefield, 1);
    let bolt2 = put(&db2, &mut g2, "Lightning Bolt", P1, Zone::Hand);
    assert!(
        g2.legal_actions(P1).contains(&Action::Cast(bolt2)),
        "eigene Zauber erlaubt"
    );
}

#[test]
fn only_one_spell_each_turn() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Rule Lite", P2, Zone::Battlefield);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 2);
    let b1 = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let b2 = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let mut d = Driver::new(g);
    assert!(d.g().legal_actions(P1).contains(&Action::Cast(b1)));
    d.cast(b1, &[T2]);
    assert!(
        !d.g().legal_actions(P1).contains(&Action::Cast(b2)),
        "zweiter Zauber verboten"
    );
}

#[test]
fn opponents_cant_activate_non_mana_abilities() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Pithing Lite", P2, Zone::Battlefield);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 1);
    let elf = put(&db, &mut g, "Adapt Lite", P1, Zone::Battlefield);
    let acts = g.legal_actions(P1);
    assert!(
        !acts.contains(&Action::Activate(elf, 0)),
        "Fähigkeit verboten"
    );
    assert!(
        !g.available_mana_abilities(P1).is_empty(),
        "Mana-Fähigkeiten erlaubt"
    );
}
