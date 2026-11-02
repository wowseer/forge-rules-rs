//! Marken-Schlüsselwortaktionen (Adapt CR 701.46, Monstrosity CR 701.37,
//! Bolster CR 701.39, Support CR 701.41), Marken-Filter und `RemoveCounter`.

mod common;
use common::*;
use forge_engine::decision::{Action, Choice, Decision, Target};
use forge_engine::keyword::Keyword;
use forge_engine::{Step, Zone};

fn activate(d: &mut Driver, c: forge_engine::CardId) {
    d.act(Action::Activate(c, 0));
    d.finish_casting(&[]);
    d.pass_all();
}

#[test]
fn adapt_only_without_p1p1_counters() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 2);
    let elf = put(&db, &mut g, "Adapt Lite", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    activate(&mut d, elf);
    assert_eq!(d.g().card(elf).counter("P1P1"), 2);
    activate(&mut d, elf);
    assert_eq!(
        d.g().card(elf).counter("P1P1"),
        2,
        "zweites Adapt ohne Wirkung"
    );
}

#[test]
fn monstrosity_once_and_filter() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 2);
    let beast = put(&db, &mut g, "Monster Lite", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    assert!(!d.g().card(beast).monstrous);
    activate(&mut d, beast);
    assert!(d.g().card(beast).monstrous);
    assert_eq!(d.g().chars(beast).power, Some(5));
    activate(&mut d, beast);
    assert_eq!(d.g().card(beast).counter("P1P1"), 3, "nur einmal monströs");
    let spec = forge_engine::valid::ValidSpec::parse("Creature.IsMonstrous").unwrap();
    assert!(d.g().card_matches(beast, &spec, beast, P1));
}

#[test]
fn bolster_picks_least_toughness_with_choice_on_tie() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 1);
    let giant = put(&db, &mut g, "Hill Giant", P1, Zone::Battlefield);
    let b1 = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let b2 = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let _opp = put(&db, &mut g, "Typhoid Rats", P2, Zone::Battlefield);
    let bolster = put(&db, &mut g, "Bolster Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(bolster, &[]);
    d.pass_all();
    match d.decision().clone() {
        Decision::ChooseCards { options, .. } => {
            assert_eq!(
                options,
                vec![b1, b2],
                "nur die Bären (Widerstand 2), nicht der Gegner"
            );
            d.submit(Choice::Cards(vec![b2]));
        }
        o => panic!("{o:?}"),
    }
    assert_eq!(d.g().card(b2).counter("P1P1"), 2);
    assert_eq!(d.g().card(b1).counter("P1P1"), 0);
    assert_eq!(d.g().card(giant).counter("P1P1"), 0);
}

#[test]
fn support_up_to_n_targets_one_counter_each() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 1);
    let a = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let b = put(&db, &mut g, "Hill Giant", P1, Zone::Battlefield);
    let sup = put(&db, &mut g, "Support Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(sup, &[Target::Card(a), Target::Card(b)]);
    d.pass_all();
    assert_eq!(d.g().card(a).counter("P1P1"), 1);
    assert_eq!(d.g().card(b).counter("P1P1"), 1);
}

#[test]
fn counters_filter_grants_flying_and_remove_counter_all() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Swamp", P1, Zone::Battlefield, 1);
    put(&db, &mut g, "Falconer Lite", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    g.card_mut(bear).counters.insert("P1P1".into(), 2);
    let scrub = put(&db, &mut g, "Scrub Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    assert!(
        d.g().chars(bear).has(&Keyword::Flying),
        "Marke → Flugfähigkeit"
    );
    d.cast(scrub, &[Target::Card(bear)]);
    d.pass_all();
    assert_eq!(d.g().card(bear).counter("P1P1"), 0);
    assert!(!d.g().chars(bear).has(&Keyword::Flying));
}

#[test]
fn remove_counter_any_chooses_kind_per_counter() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Swamp", P1, Zone::Battlefield, 1);
    let bear = put(&db, &mut g, "Hill Giant", P2, Zone::Battlefield);
    g.card_mut(bear).counters.insert("P1P1".into(), 1);
    g.card_mut(bear).counters.insert("CHARGE".into(), 3);
    let thief = put(&db, &mut g, "Thief Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(thief, &[Target::Card(bear)]);
    d.pass_all();
    for _ in 0..2 {
        match d.decision().clone() {
            Decision::ChooseType { options, .. } => {
                let i = options.iter().position(|o| o == "CHARGE").unwrap();
                d.submit(Choice::Number(i as u32));
            }
            o => panic!("{o:?}"),
        }
    }
    assert_eq!(d.g().card(bear).counter("CHARGE"), 1);
    assert_eq!(d.g().card(bear).counter("P1P1"), 1);
}

#[test]
fn remove_all_counters_needs_has_counters_target() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Swamp", P1, Zone::Battlefield, 1);
    let plain = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let marked = put(&db, &mut g, "Hill Giant", P2, Zone::Battlefield);
    g.card_mut(marked).counters.insert("P1P1".into(), 2);
    g.card_mut(marked).counters.insert("STUN".into(), 1);
    let clear = put(&db, &mut g, "Clear Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.act(Action::Cast(clear));
    match d.decision().clone() {
        Decision::ChooseTargets { candidates, .. } => {
            assert!(candidates.contains(&Target::Card(marked)));
            assert!(!candidates.contains(&Target::Card(plain)), "HasCounters");
        }
        o => panic!("{o:?}"),
    }
    d.finish_casting(&[Target::Card(marked)]);
    d.pass_all();
    assert!(d.g().card(marked).counters.is_empty());
}
