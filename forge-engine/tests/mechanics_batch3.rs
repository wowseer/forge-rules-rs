//! Energie, Marken-/Spielstein-Verdopplung, MultiplyCounter, Connive, LandPlayed,
//! AdjustLandPlays, Filter modified.

mod common;
use common::*;
use forge_engine::decision::{Action, Choice, Decision, Target};
use forge_engine::{Step, Zone};

#[test]
fn energy_gained_and_paid() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 1);
    let e = put(&db, &mut g, "Energy Kw", P1, Zone::Hand);
    let u = put(&db, &mut g, "Energy User Kw", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(e, &[]);
    d.pass_all();
    assert_eq!(d.g().player(P1).counters.get("ENERGY"), Some(&3));
    d.act(Action::Activate(u, 0));
    d.finish_casting(&[T2]);
    d.pass_all();
    assert_eq!(d.g().player(P1).counters.get("ENERGY"), Some(&1));
    assert_eq!(d.g().player(P2).life, 19, "X = verbleibende Energie (1)");
}

#[test]
fn hardened_scales_and_doubling_season() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Hardened Scales", P1, Zone::Battlefield);
    put(&db, &mut g, "Doubling Season", P1, Zone::Battlefield);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 1);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 1);
    let elf = put(&db, &mut g, "Adapt Lite", P1, Zone::Battlefield);
    let tok = put(&db, &mut g, "Token Memory Kw", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.act(Action::Activate(elf, 0));
    d.finish_casting(&[]);
    d.pass_all();
    // Adapt 2: Scales +1 und Season ×2 (Reihenfolge nach Zeitstempel: (2+1)×2)
    assert_eq!(d.g().card(elf).counter("P1P1"), 6);
    d.cast(tok, &[]);
    d.pass_all();
    let tokens = d
        .g()
        .battlefield
        .iter()
        .filter(|c| d.g().card(**c).token)
        .count();
    assert_eq!(tokens, 4, "2 Soldaten verdoppelt");
}

#[test]
fn multiply_and_connive() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 1);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 1);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    g.card_mut(bear).counters.insert("P1P1".into(), 2);
    let dbl = put(&db, &mut g, "Doubler Kw", P1, Zone::Hand);
    let con = put(&db, &mut g, "Conniver Kw", P1, Zone::Hand);
    let giant = put(&db, &mut g, "Hill Giant", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(dbl, &[Target::Card(bear)]);
    d.pass_all();
    assert_eq!(d.g().card(bear).counter("P1P1"), 4);
    d.cast(con, &[]);
    d.pass_all();
    d.pass_all();
    match d.decision().clone() {
        Decision::ChooseCards { options, .. } => {
            assert!(options.contains(&giant));
            d.submit(Choice::Cards(vec![giant]));
        }
        o => panic!("{o:?}"),
    }
    let c = find(d.g(), "Conniver Kw", Zone::Battlefield).unwrap();
    assert_eq!(d.g().card(c).counter("P1P1"), 1, "Nichtland abgeworfen");
}

#[test]
fn landfall_trigger_and_extra_land_play() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Landfall Kw", P1, Zone::Battlefield);
    let f1 = put(&db, &mut g, "Forest", P1, Zone::Hand);
    let f2 = put(&db, &mut g, "Forest", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.act(Action::PlayLand(f1));
    d.pass_all();
    assert_eq!(d.g().player(P1).life, 22);
    assert!(
        d.g().legal_actions(P1).contains(&Action::PlayLand(f2)),
        "zweites Landspiel"
    );
    let spec = forge_engine::valid::ValidSpec::parse("Creature.modified").unwrap();
    let mut g = d.g().clone();
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    assert!(!g.card_matches(bear, &spec, bear, P1));
    g.card_mut(bear).counters.insert("P1P1".into(), 1);
    assert!(g.card_matches(bear, &spec, bear, P1));
}
