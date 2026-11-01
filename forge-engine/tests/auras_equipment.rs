//! Auren und Ausrüstung (CR 301.5, 303.4, 702.5, 702.6, 704.5m/n).

mod common;
use common::*;
use forge_engine::decision::{Action, Choice, Decision, Target};
use forge_engine::keyword::Keyword;
use forge_engine::{Step, Zone};

#[test]
fn aura_attaches_on_resolution_and_buffs() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Forest", P1, Zone::Battlefield);
    let r = put(&db, &mut g, "Rancor Lite", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(r, &[Target::Card(bear)]);
    d.pass_all();
    let aura = find(d.g(), "Rancor Lite", Zone::Battlefield).unwrap();
    assert_eq!(d.g().card(aura).attached_to, Some(bear));
    assert_eq!(d.g().chars(bear).power, Some(4));
    assert!(d.g().has_keyword(bear, &Keyword::Trample));
}

#[test]
fn aura_goes_to_graveyard_when_creature_dies_and_trigger_returns_it() {
    // CR 704.5m
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Forest", P1, Zone::Battlefield);
    put(&db, &mut g, "Mountain", P1, Zone::Battlefield);
    let r = put(&db, &mut g, "Rancor Lite", P1, Zone::Hand);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(r, &[Target::Card(bear)]);
    d.pass_all();
    d.cast(bolt, &[Target::Card(bear)]);
    d.pass_all();
    assert!(
        find(d.g(), "Rancor Lite", Zone::Battlefield).is_none(),
        "SBA: Aura ohne Objekt"
    );
    d.pass_all(); // Rancor-Trigger
    assert_eq!(count(d.g(), "Rancor Lite", Zone::Hand), 1);
}

#[test]
fn aura_fizzles_if_target_gone() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Forest", P1, Zone::Battlefield);
    put(&db, &mut g, "Mountain", P1, Zone::Battlefield);
    let r = put(&db, &mut g, "Rancor Lite", P1, Zone::Hand);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(r, &[Target::Card(bear)]);
    d.cast(bolt, &[Target::Card(bear)]);
    d.pass_all();
    d.pass_all();
    assert_eq!(
        count(d.g(), "Rancor Lite", Zone::Graveyard) + count(d.g(), "Rancor Lite", Zone::Hand),
        1
    );
    assert!(find(d.g(), "Rancor Lite", Zone::Battlefield).is_none());
}

#[test]
fn pacifism_multi_mode_static() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 2);
    let pac = put(&db, &mut g, "Pacifism", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(pac, &[Target::Card(bear)]);
    d.pass_all();
    assert!(d.g().block_candidates(P2).is_empty(), "kann nicht blocken");
    let mut g = d.g().clone();
    g.active = P2;
    assert!(g.attack_candidates(P2).is_empty(), "kann nicht angreifen");
}

#[test]
fn equip_sorcery_speed_own_creature_and_moves() {
    // CR 702.6
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 3);
    let eq = put(&db, &mut g, "Bonesplitter", P1, Zone::Battlefield);
    let a = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let b = put(&db, &mut g, "Hill Giant", P1, Zone::Battlefield);
    let theirs = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    let equip_idx = d
        .g()
        .card(eq)
        .compiled()
        .activated
        .iter()
        .position(|x| x.api == "Attach")
        .unwrap();
    d.act(Action::Activate(eq, equip_idx));
    match d.decision() {
        Decision::ChooseTargets { candidates, .. } => assert!(
            !candidates.contains(&Target::Card(theirs)),
            "nur eigene Kreaturen"
        ),
        o => panic!("{o:?}"),
    }
    d.finish_casting(&[Target::Card(a)]);
    // Stapel nicht leer → keine Hexerei-Geschwindigkeit
    match d.decision() {
        Decision::Priority { actions, .. } => {
            assert!(!actions.contains(&Action::Activate(eq, equip_idx)))
        }
        o => panic!("{o:?}"),
    }
    d.pass_all();
    assert_eq!(d.g().chars(a).power, Some(4));
    d.act(Action::Activate(eq, equip_idx));
    d.finish_casting(&[Target::Card(b)]);
    d.pass_all();
    assert_eq!(d.g().chars(a).power, Some(2));
    assert_eq!(d.g().chars(b).power, Some(5));
}

#[test]
fn equipment_stays_when_creature_dies() {
    // CR 704.5n
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Mountain", P1, Zone::Battlefield);
    let eq = put(&db, &mut g, "Bonesplitter", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    g.card_mut(eq).attached_to = Some(bear);
    g.card_mut(bear).damage = 5;
    let d = Driver::new(g);
    assert_eq!(d.g().card(eq).zone, Some(Zone::Battlefield));
    assert_eq!(d.g().card(eq).attached_to, None);
    let _ = Choice::Yes;
}
