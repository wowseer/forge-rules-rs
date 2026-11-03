//! Ausweichfähigkeiten (Landwalk, Fear, Shadow, Skulk) und Exalted.

mod common;
use common::*;
use forge_engine::decision::{Choice, Decision};
use forge_engine::{Step, Zone};

#[test]
fn evasion_rules() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let walker = put(&db, &mut g, "Swampwalker Lite", P1, Zone::Battlefield);
    let fear = put(&db, &mut g, "Fear Lite", P1, Zone::Battlefield);
    let shadow = put(&db, &mut g, "Shadow Lite", P1, Zone::Battlefield);
    let skulk = put(&db, &mut g, "Skulker Lite", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let rats = put(&db, &mut g, "Typhoid Rats", P2, Zone::Battlefield);
    let orni = put(&db, &mut g, "Ornithopter", P2, Zone::Battlefield);
    let shadow2 = put(&db, &mut g, "Shadow Lite", P2, Zone::Battlefield);
    assert!(g.can_block(bear, walker), "ohne Sumpf blockbar");
    put(&db, &mut g, "Swamp", P2, Zone::Battlefield);
    assert!(!g.can_block(bear, walker), "Sumpfwandeln");
    assert!(!g.can_block(bear, fear));
    assert!(g.can_block(rats, fear), "schwarz");
    assert!(g.can_block(orni, fear), "Artefakt");
    assert!(!g.can_block(bear, shadow));
    assert!(g.can_block(shadow2, shadow));
    assert!(!g.can_block(shadow2, bear), "Shadow blockt nur Shadow");
    assert!(!g.can_block(bear, skulk), "größere Stärke");
    assert!(g.can_block(orni, skulk), "Stärke 0");
}

#[test]
fn exalted_pumps_lone_attacker() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Exalted Lite", P1, Zone::Battlefield);
    put(&db, &mut g, "Exalted Lite", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.run_until(|_, dec| matches!(dec, Decision::DeclareAttackers { .. }));
    d.submit(Choice::Attackers(vec![(bear, T2)]));
    d.run_until(|g, _| g.step == Step::EndCombat);
    assert_eq!(d.g().player(P2).life, 16, "2 + 2×Exalted");
}
