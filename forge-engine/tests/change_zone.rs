//! ChangeZone / ChangeZoneAll (Bounce, Exil, Reanimation, Suche).

mod common;
use common::*;
use forge_engine::decision::{CardChoiceReason, Choice, Decision, Target};
use forge_engine::{Step, Zone};

#[test]
fn bounce_returns_to_owners_hand() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Island", P1, Zone::Battlefield);
    let uns = put(&db, &mut g, "Unsummon", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    g.card_mut(bear).controller = P1; // P1 kontrolliert, P2 besitzt
    let mut d = Driver::new(g);
    d.cast(uns, &[Target::Card(bear)]);
    d.pass_all();
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Hand), 1);
    assert!(
        d.g()
            .player(P2)
            .hand
            .iter()
            .any(|c| d.g().card(*c).def.name() == "Grizzly Bears"),
        "Hand des Besitzers"
    );
}

#[test]
fn target_card_in_graveyard_and_hexproof_irrelevant_there() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Swamp", P1, Zone::Battlefield);
    let rd = put(&db, &mut g, "Raise Dead", P1, Zone::Hand);
    let mine = put(&db, &mut g, "Gladecover Scout", P1, Zone::Graveyard);
    put(&db, &mut g, "Grizzly Bears", P2, Zone::Graveyard);
    put(&db, &mut g, "Lightning Bolt", P1, Zone::Graveyard);
    let mut d = Driver::new(g);
    d.act(forge_engine::Action::Cast(rd));
    match d.decision() {
        Decision::ChooseTargets { candidates, .. } => assert_eq!(
            candidates,
            &vec![Target::Card(mine)],
            "nur eigene Kreaturenkarten im Friedhof"
        ),
        o => panic!("{o:?}"),
    }
    d.finish_casting(&[Target::Card(mine)]);
    d.pass_all();
    assert_eq!(count(d.g(), "Gladecover Scout", Zone::Hand), 1);
}

#[test]
fn reanimate_under_your_control() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Swamp", P1, Zone::Battlefield);
    let re = put(&db, &mut g, "Reanimate Lite", P1, Zone::Hand);
    let wurm = put(&db, &mut g, "Craw Wurm", P2, Zone::Graveyard);
    let mut d = Driver::new(g);
    d.cast(re, &[Target::Card(wurm)]);
    d.pass_all();
    let w = find(d.g(), "Craw Wurm", Zone::Battlefield).unwrap();
    assert_eq!(d.g().card(w).controller, P1);
    assert_eq!(d.g().card(w).owner, P2);
}

#[test]
fn search_library_tapped_then_shuffle() {
    // CR 701.19
    let (db, mut g) = game_at(Step::Main1, 0);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 2);
    put_n(&db, &mut g, "Grizzly Bears", P1, Zone::Library, 3);
    let forest = put(&db, &mut g, "Forest", P1, Zone::Library);
    let rg = put(&db, &mut g, "Rampant Growth", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(rg, &[]);
    d.pass_all();
    match d.decision().clone() {
        Decision::ChooseCards {
            reason: CardChoiceReason::Search,
            options,
            min: 0,
            max: 1,
            ..
        } => {
            assert_eq!(options, vec![forest], "nur Standardländer");
            d.submit(Choice::Cards(options));
        }
        o => panic!("{o:?}"),
    }
    assert_eq!(count(d.g(), "Forest", Zone::Battlefield), 3);
    let new = find(d.g(), "Forest", Zone::Battlefield)
        .map(|_| d.g().battlefield.last().copied().unwrap())
        .unwrap();
    assert!(d.g().card(new).tapped);
    assert_eq!(d.g().player(P1).library.len(), 3);
}

#[test]
fn search_may_fail_to_find() {
    let (db, mut g) = game_at(Step::Main1, 0);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 2);
    put(&db, &mut g, "Forest", P1, Zone::Library);
    let rg = put(&db, &mut g, "Rampant Growth", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(rg, &[]);
    d.pass_all();
    d.submit(Choice::Cards(vec![]));
    assert_eq!(d.g().player(P1).library.len(), 1);
}

#[test]
fn exile_with_sub_ability_targeted_controller() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Plains", P1, Zone::Battlefield);
    let stp = put(&db, &mut g, "Swords to Plowshares Lite", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(stp, &[Target::Card(bear)]);
    d.pass_all();
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Exile), 1);
    assert_eq!(d.g().player(P2).life, 23);
}

#[test]
fn change_zone_all_bounces_everything() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 5);
    let ev = put(&db, &mut g, "Evacuation", P1, Zone::Hand);
    put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    put_n(&db, &mut g, "Hill Giant", P2, Zone::Battlefield, 2);
    let mut d = Driver::new(g);
    d.cast(ev, &[]);
    d.pass_all();
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Hand), 1);
    assert_eq!(count(d.g(), "Hill Giant", Zone::Hand), 2);
    assert_eq!(
        count(d.g(), "Island", Zone::Battlefield),
        5,
        "Länder bleiben"
    );
}

#[test]
fn library_bottom() {
    let (db, mut g) = game_at(Step::Main1, 5);
    put(&db, &mut g, "Plains", P1, Zone::Battlefield);
    let b = put(&db, &mut g, "Bottom Lite", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(b, &[Target::Card(bear)]);
    d.pass_all();
    let lib = &d.g().player(P2).library;
    assert_eq!(d.g().card(lib[0]).def.name(), "Grizzly Bears");
}

#[test]
fn dies_trigger_returns_new_object_to_hand() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Mountain", P1, Zone::Battlefield);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let z = put(&db, &mut g, "Undying Lite", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(bolt, &[Target::Card(z)]);
    d.pass_all();
    d.pass_all();
    assert_eq!(count(d.g(), "Undying Lite", Zone::Hand), 1);
}
