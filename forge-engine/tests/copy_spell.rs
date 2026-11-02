//! Zauber kopieren (CR 707.10, Java `CopySpellAbilityEffect`).

mod common;
use common::*;
use forge_engine::decision::{Choice, Decision, Target};
use forge_engine::state::StackKind;
use forge_engine::{Step, Zone};

#[test]
fn copy_target_spell_with_new_target() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 1);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 1);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let twin = put(&db, &mut g, "Twin Lite", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(bolt, &[T2]);
    let bolt_on_stack = match d.g().stack[0].kind {
        StackKind::Spell { card } => card,
        _ => unreachable!(),
    };
    d.cast(twin, &[Target::Card(bolt_on_stack)]);
    d.pass_all(); // Twin löst auf → Frage nach neuen Zielen
    match d.decision() {
        Decision::Confirm { .. } => d.submit(Choice::Yes),
        o => panic!("{o:?}"),
    }
    d.finish_casting(&[Target::Card(bear)]);
    assert_eq!(d.g().stack.len(), 2, "Kopie über dem Original");
    let copy = match d.g().stack[1].kind {
        StackKind::Spell { card } => card,
        _ => unreachable!(),
    };
    assert!(d.g().card(copy).token, "Kopie ist keine Karte");
    d.pass_all();
    assert_eq!(d.g().card(bear).zone, None, "Kopie trifft den Bären");
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 17, "Original trifft den Spieler");
    assert_eq!(
        count(d.g(), "Lightning Bolt", Zone::Graveyard),
        1,
        "Kopie hört auf zu existieren"
    );
    assert_eq!(
        d.g().this_turn.cast.len(),
        2,
        "Kopie zählt nicht als gewirkt"
    );
}

#[test]
fn triggered_copy_of_cast_spell() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 1);
    put(&db, &mut g, "Echo Lite", P1, Zone::Battlefield);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(bolt, &[T2]);
    d.pass_all(); // Trigger löst auf → Kopie
    d.pass_all(); // Kopie
    d.pass_all(); // Original
    assert_eq!(d.g().player(P2).life, 14);
    assert!(d.g().stack.is_empty());
}
