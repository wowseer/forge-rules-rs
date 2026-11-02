//! „Kann nicht neutralisiert werden“ (`R:Event$ Counter | Layer$ CantHappen`, CR 101.2).

mod common;
use common::*;
use forge_engine::decision::Target;
use forge_engine::state::StackKind;
use forge_engine::{Step, Zone};

fn top_spell(d: &Driver) -> forge_engine::CardId {
    match d.g().stack.last().unwrap().kind {
        StackKind::Spell { card } => card,
        _ => unreachable!(),
    }
}

#[test]
fn self_uncounterable_spell_resolves() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 2);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 2);
    let bear = put(&db, &mut g, "Uncounterable Lite", P1, Zone::Hand);
    let cs = put(&db, &mut g, "Counterspell", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(bear, &[]);
    let spell = top_spell(&d);
    d.cast(cs, &[Target::Card(spell)]);
    d.pass_all(); // Counterspell löst auf, ohne Wirkung
    d.pass_all(); // Kreatur
    assert_eq!(count(d.g(), "Uncounterable Lite", Zone::Battlefield), 1);
}

#[test]
fn static_protects_only_own_creature_spells() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Prowler Lite", P1, Zone::Battlefield);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 2);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 4);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 1);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Hand);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let cs1 = put(&db, &mut g, "Counterspell", P1, Zone::Hand);
    let cs = put(&db, &mut g, "Counterspell", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(bear, &[]);
    let spell = top_spell(&d);
    d.cast(cs1, &[Target::Card(spell)]);
    d.pass_all();
    d.pass_all();
    assert_eq!(
        count(d.g(), "Grizzly Bears", Zone::Battlefield),
        1,
        "geschützt"
    );
    d.cast(bolt, &[T2]);
    let spell = top_spell(&d);
    d.cast(cs, &[Target::Card(spell)]);
    d.pass_all();
    assert_eq!(
        count(d.g(), "Lightning Bolt", Zone::Graveyard),
        1,
        "Nicht-Kreatur neutralisiert"
    );
    assert_eq!(d.g().player(P2).life, 20);
}
