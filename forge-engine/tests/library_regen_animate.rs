//! Scry, Mill, Dig, Regenerate, Animate.

mod common;
use common::*;
use forge_engine::decision::{Action, CardChoiceReason, Choice, Decision, Target};
use forge_engine::keyword::Keyword;
use forge_engine::{Step, Zone};

#[test]
fn scry_bottom_and_reorder_then_draw() {
    // CR 701.22
    let (db, mut g) = game_at(Step::Main1, 0);
    put(&db, &mut g, "Island", P1, Zone::Battlefield);
    let bottom = put(&db, &mut g, "Forest", P1, Zone::Library);
    let a = put(&db, &mut g, "Grizzly Bears", P1, Zone::Library);
    let b = put(&db, &mut g, "Hill Giant", P1, Zone::Library); // oberste
    let opt = put(&db, &mut g, "Opt Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(opt, &[]);
    d.pass_all();
    match d.decision().clone() {
        Decision::Scry { cards, .. } => assert_eq!(cards, vec![b, a], "oberste zuerst"),
        o => panic!("{o:?}"),
    }
    assert!(
        d.try_submit(Choice::Arrange {
            top: vec![b],
            bottom: vec![]
        })
        .is_err()
    );
    d.submit(Choice::Arrange {
        top: vec![],
        bottom: vec![b, a],
    });
    let lib = &d.g().player(P1).library;
    assert_eq!(lib.len(), 2);
    let hand_new = d.g().player(P1).hand.last().copied().unwrap();
    assert_eq!(
        d.g().card(hand_new).def.name(),
        "Forest",
        "Forest war darunter und wird gezogen"
    );
    assert_eq!(lib[0], a, "a ganz unten");
    assert_eq!(lib[1], b);
    let _ = bottom;
}

#[test]
fn mill_target_player() {
    let (db, mut g) = game_at(Step::Main1, 5);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 2);
    let s = put(&db, &mut g, "Mind Sculpt Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(s, &[Target::Player(P2)]);
    d.pass_all();
    assert_eq!(d.g().player(P2).library.len(), 2);
    assert_eq!(d.g().player(P2).graveyard.len(), 3);
}

#[test]
fn dig_one_to_hand_rest_bottom() {
    let (db, mut g) = game_at(Step::Main1, 0);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 2);
    let deep = put(&db, &mut g, "Plains", P1, Zone::Library);
    let top4 = put_n(&db, &mut g, "Grizzly Bears", P1, Zone::Library, 4);
    let s = put(&db, &mut g, "Impulse Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(s, &[]);
    d.pass_all();
    match d.decision() {
        Decision::ChooseCards {
            reason: CardChoiceReason::Dig,
            min: 1,
            max: 1,
            options,
            ..
        } => assert_eq!(options.len(), 4),
        o => panic!("{o:?}"),
    }
    d.submit(Choice::Cards(vec![top4[0]]));
    assert_eq!(d.g().player(P1).hand.len(), 1);
    let lib = &d.g().player(P1).library;
    assert_eq!(lib.len(), 4);
    assert_eq!(
        *lib.last().unwrap(),
        deep,
        "Rest unter die Bibliothek, Plains jetzt oben"
    );
}

#[test]
fn dig_all_matching_rest_to_graveyard() {
    let (db, mut g) = game_at(Step::Main1, 0);
    put(&db, &mut g, "Forest", P1, Zone::Battlefield);
    put(&db, &mut g, "Grizzly Bears", P1, Zone::Library);
    put(&db, &mut g, "Forest", P1, Zone::Library);
    put(&db, &mut g, "Island", P1, Zone::Library);
    let s = put(&db, &mut g, "Land Dig Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(s, &[]);
    d.pass_all();
    assert_eq!(d.g().player(P1).hand.len(), 2);
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Graveyard), 1);
}

#[test]
fn regenerate_replaces_destruction() {
    // CR 701.15
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Forest", P1, Zone::Battlefield);
    put_n(&db, &mut g, "Swamp", P2, Zone::Battlefield, 6);
    let boa = put(&db, &mut g, "River Boa Lite", P1, Zone::Battlefield);
    let m1 = put(&db, &mut g, "Murder", P2, Zone::Hand);
    let mut d = Driver::new(g);
    d.act(Action::Activate(boa, 0));
    d.finish_casting(&[]);
    d.pass_all();
    assert_eq!(d.g().card(boa).regen_shields, 1);
    let mut g = d.g().clone();
    g.active = P2;
    g.flow = forge_engine::state::Flow::Priority {
        player: P2,
        passes: 0,
    };
    let mut d = Driver::new(g);
    d.cast(m1, &[Target::Card(boa)]);
    d.pass_all();
    assert_eq!(d.g().card(boa).zone, Some(Zone::Battlefield), "regeneriert");
    assert!(d.g().card(boa).tapped);
    assert_eq!(d.g().card(boa).regen_shields, 0);
}

#[test]
fn regeneration_vs_lethal_damage_and_shield_ends() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let boa = put(&db, &mut g, "River Boa Lite", P1, Zone::Battlefield);
    g.card_mut(boa).regen_shields = 1;
    g.card_mut(boa).damage = 5;
    let mut d = Driver::new(g);
    assert_eq!(
        d.g().card(boa).zone,
        Some(Zone::Battlefield),
        "SBA-Zerstörung wird regeneriert"
    );
    assert_eq!(d.g().card(boa).damage, 0);
    d.run_until(|g, _| g.turn == 3);
    assert_eq!(d.g().card(boa).regen_shields, 0);
}

#[test]
fn animate_land_until_end_of_turn() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Forest", P1, Zone::Battlefield);
    let land = put(&db, &mut g, "Island", P1, Zone::Battlefield);
    let s = put(&db, &mut g, "Animate Land Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(s, &[Target::Card(land)]);
    d.pass_all();
    let ch = d.g().chars(land);
    assert!(ch.types.is_creature() && ch.types.is_land());
    assert!(ch.types.has_subtype("Elemental"));
    assert_eq!((ch.power, ch.toughness), (Some(3), Some(3)));
    assert!(
        ch.colors.contains(forge_types::Color::Red)
            && !ch.colors.contains(forge_types::Color::Blue)
    );
    assert!(d.g().has_keyword(land, &Keyword::Haste));
    d.run_until(|g, _| g.turn == 3);
    assert!(!d.g().chars(land).types.is_creature());
}
