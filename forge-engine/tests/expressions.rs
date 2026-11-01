//! Ausdrücke (Count$) und Bedingungen in Effekten, Statics, Triggern.

mod common;
use common::*;
use forge_engine::decision::{Action, Decision, Target};
use forge_engine::keyword::Keyword;
use forge_engine::{Step, Zone};

#[test]
fn effect_amount_counts_creatures() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 3);
    put_n(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield, 4);
    put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let s = put(&db, &mut g, "Overrun Count Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(s, &[Target::Player(P2)]);
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 16);
}

#[test]
fn dynamic_static_without_infinite_recursion() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Crusade Count Lite", P1, Zone::Battlefield);
    let bears = put_n(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield, 3);
    assert_eq!(g.chars(bears[0]).power, Some(5));
    g.move_card(bears[2], Zone::Graveyard, true);
    assert_eq!(g.chars(bears[0]).power, Some(4));
}

#[test]
fn characteristic_defining_ability_in_all_zones() {
    // CR 604.3
    let (db, mut g) = game_at(Step::Main1, 10);
    let goyf = put(&db, &mut g, "Goyf Lite", P1, Zone::Hand);
    assert!(
        g.card(goyf).def.is_supported(),
        "{:?}",
        g.card(goyf).def.unsupported
    );
    assert_eq!(
        (g.chars(goyf).power, g.chars(goyf).toughness),
        (Some(0), Some(1))
    );
    put_n(&db, &mut g, "Lightning Bolt", P2, Zone::Graveyard, 2);
    put(&db, &mut g, "Forest", P1, Zone::Graveyard);
    assert_eq!(
        (g.chars(goyf).power, g.chars(goyf).toughness),
        (Some(3), Some(4)),
        "auch in der Hand"
    );
}

#[test]
fn static_condition_player_turn() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let f = put(&db, &mut g, "Turn Fighter Lite", P1, Zone::Battlefield);
    assert_eq!(g.chars(f).power, Some(3));
    g.active = P2;
    assert_eq!(g.chars(f).power, Some(1));
}

#[test]
fn static_is_present() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let fish = put(&db, &mut g, "Island Flyer Lite", P1, Zone::Battlefield);
    put(&db, &mut g, "Island", P2, Zone::Battlefield);
    assert!(
        !g.has_keyword(fish, &Keyword::Flying),
        "Insel des Gegners zählt nicht"
    );
    put(&db, &mut g, "Island", P1, Zone::Battlefield);
    assert!(g.has_keyword(fish, &Keyword::Flying));
}

#[test]
fn intervening_if_checked_on_trigger_and_resolution() {
    // CR 603.4
    let (db, mut g) = game_at(Step::Untap, 10);
    put(&db, &mut g, "Low Life Healer Lite", P1, Zone::Battlefield);
    g.player_mut(P1).life = 15;
    let mut d = Driver::new(g.clone());
    d.priority_in(Step::Upkeep, P1);
    assert!(d.g().stack.is_empty(), "löst bei 15 Leben nicht aus");

    g.player_mut(P1).life = 10;
    let mut d = Driver::new(g.clone());
    d.priority_in(Step::Upkeep, P1);
    assert_eq!(d.g().stack.len(), 1);
    d.pass_all();
    assert_eq!(d.g().player(P1).life, 12);

    // Bedingung beim Auflösen nicht mehr erfüllt → nichts passiert
    put(&db, &mut g, "Plains", P1, Zone::Battlefield);
    put(&db, &mut g, "Healing Salve", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.priority_in(Step::Upkeep, P1);
    let salve = d
        .g()
        .player(P1)
        .hand
        .iter()
        .copied()
        .find(|c| d.g().card(*c).def.name() == "Healing Salve")
        .unwrap();
    d.cast(salve, &[Target::Player(P1)]);
    d.pass_all(); // Salve: 13 Leben
    d.pass_all(); // Trigger: Bedingung LE10 verletzt
    assert_eq!(d.g().player(P1).life, 13);
}

#[test]
fn effect_condition_present() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Forest", P1, Zone::Battlefield);
    put(&db, &mut g, "Forest", P1, Zone::Battlefield);
    let s1 = put(&db, &mut g, "Forest Insight Lite", P1, Zone::Hand);
    let mut d = Driver::new(g.clone());
    d.cast(s1, &[]);
    d.pass_all();
    assert_eq!(d.g().player(P1).life, 21);
    assert_eq!(d.g().player(P1).hand.len(), 1, "Forest vorhanden → gezogen");

    let (db2, mut g2) = game_at(Step::Main1, 10);
    put_n(&db2, &mut g2, "Island", P1, Zone::Battlefield, 2);
    g2.player_mut(P1)
        .mana_pool
        .add(Some(forge_types::Color::Green), false);
    let s2 = put(&db2, &mut g2, "Forest Insight Lite", P1, Zone::Hand);
    let mut d2 = Driver::new(g2);
    d2.cast(s2, &[]);
    d2.pass_all();
    assert_eq!(d2.g().player(P1).life, 21);
    assert_eq!(
        d2.g().player(P1).hand.len(),
        0,
        "kein Forest → nicht gezogen"
    );
}

#[test]
fn filter_power_comparison() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Swamp", P1, Zone::Battlefield);
    let bgh = put(&db, &mut g, "Big Game Hunter Lite", P1, Zone::Hand);
    let wurm = put(&db, &mut g, "Craw Wurm", P2, Zone::Battlefield);
    put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.act(Action::Cast(bgh));
    match d.decision() {
        Decision::ChooseTargets { candidates, .. } => {
            assert_eq!(candidates, &vec![Target::Card(wurm)])
        }
        o => panic!("{o:?}"),
    }
}
