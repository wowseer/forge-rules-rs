//! Zustandsbasierte Aktionen (Forge_State_Based_Action.xlsx, CR 704).

mod common;
use common::*;
use forge_engine::decision::{Choice, Decision, Target};
use forge_engine::state::LossReason;
use forge_engine::{Step, Zone};

#[test]
fn cr_704_5a_zero_life_loses() {
    let (_db, mut g) = game_at(Step::Main1, 10);
    g.player_mut(P2).life = 0;
    let d = Driver::new(g);
    assert_eq!(d.is_over(), Some(Some(P1)));
    assert_eq!(d.g().player(P2).lost, Some(LossReason::LifeZero));
}

#[test]
fn cr_704_5b_draw_from_empty_library_loses() {
    let (_db, g) = game_at(Step::Upkeep, 0);
    let mut d = Driver::new(g);
    d.run_until(|_, _| false);
    assert_eq!(d.is_over(), Some(Some(P2)));
    assert_eq!(
        d.g().player(P1).lost,
        Some(LossReason::DrewFromEmptyLibrary)
    );
}

#[test]
fn empty_library_without_drawing_does_not_lose() {
    let (_db, g) = game_at(Step::Main1, 0);
    let d = Driver::new(g);
    assert!(d.is_over().is_none());
}

#[test]
fn cr_704_5c_ten_poison_loses() {
    let (_db, mut g) = game_at(Step::Main1, 10);
    g.player_mut(P1).poison = 10;
    let d = Driver::new(g);
    assert_eq!(d.is_over(), Some(Some(P2)));
    assert_eq!(d.g().player(P1).lost, Some(LossReason::Poison));
}

#[test]
fn cr_704_5d_token_outside_battlefield_ceases_to_exist() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let def = db.get("Grizzly Bears").unwrap().clone();
    let t = g.create_card(def, P1, Zone::Graveyard, true);
    let d = Driver::new(g);
    assert_eq!(d.g().card(t).zone, None);
    assert!(d.g().player(P1).graveyard.is_empty());
}

#[test]
fn cr_704_5f_zero_toughness_dies_even_if_indestructible() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Swamp", P1, Zone::Battlefield);
    let w = put(&db, &mut g, "Weakness", P1, Zone::Hand);
    let myr = put(&db, &mut g, "Darksteel Myr", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(w, &[Target::Card(myr)]);
    d.pass_all();
    assert_eq!(count(d.g(), "Darksteel Myr", Zone::Graveyard), 1);
}

#[test]
fn cr_704_5g_lethal_damage_destroys_but_not_indestructible() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let myr = put(&db, &mut g, "Darksteel Myr", P2, Zone::Battlefield);
    g.card_mut(bear).damage = 2;
    g.card_mut(myr).damage = 5;
    let d = Driver::new(g);
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Graveyard), 1);
    assert_eq!(d.g().card(myr).zone, Some(Zone::Battlefield));
}

#[test]
fn cr_704_5h_deathtouch_damage_destroys() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let rats = put(&db, &mut g, "Typhoid Rats", P1, Zone::Battlefield);
    let wurm = put(&db, &mut g, "Craw Wurm", P2, Zone::Battlefield);
    g.deal_damage(rats, Target::Card(wurm), 1);
    let d = Driver::new(g);
    assert_eq!(count(d.g(), "Craw Wurm", Zone::Graveyard), 1);
}

#[test]
fn cr_704_5j_legend_rule_controller_chooses() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let a = put(
        &db,
        &mut g,
        "Isamaru, Hound of Konda",
        P1,
        Zone::Battlefield,
    );
    let b = put(
        &db,
        &mut g,
        "Isamaru, Hound of Konda",
        P1,
        Zone::Battlefield,
    );
    // gleiche Legende beim Gegner ist erlaubt
    let c = put(
        &db,
        &mut g,
        "Isamaru, Hound of Konda",
        P2,
        Zone::Battlefield,
    );
    let mut d = Driver::new(g);
    match d.decision() {
        Decision::ChooseCards {
            player,
            options,
            min: 1,
            max: 1,
            ..
        } => {
            assert_eq!(*player, P1);
            assert_eq!(options, &vec![a, b]);
        }
        other => panic!("{other:?}"),
    }
    d.submit(Choice::Cards(vec![b]));
    assert_eq!(d.g().card(b).zone, Some(Zone::Battlefield));
    assert_eq!(d.g().card(a).zone, None);
    assert_eq!(d.g().card(c).zone, Some(Zone::Battlefield));
}

#[test]
fn cr_704_5q_plus_and_minus_counters_annihilate() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    g.add_counters(bear, "P1P1", 3);
    g.add_counters(bear, "M1M1", 1);
    let d = Driver::new(g);
    assert_eq!(d.g().card(bear).counter("P1P1"), 2);
    assert_eq!(d.g().card(bear).counter("M1M1"), 0);
    assert_eq!(d.g().chars(bear).power, Some(4));
}

#[test]
fn sbas_are_simultaneous_both_players_lose_is_draw() {
    // CR 704.3 + 104.4a
    let (_db, mut g) = game_at(Step::Main1, 10);
    g.player_mut(P1).life = 0;
    g.player_mut(P2).life = -2;
    let d = Driver::new(g);
    assert_eq!(d.is_over(), Some(None));
}

#[test]
fn cr_704_5i_planeswalker_without_loyalty() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let pw = put(&db, &mut g, "Liliana Lite", P2, Zone::Battlefield);
    g.add_counters(pw, "LOYALTY", 3);
    let gob = put(&db, &mut g, "Goblin Piker", P1, Zone::Battlefield);
    g.deal_damage(gob, Target::Card(pw), 2);
    assert_eq!(
        g.card(pw).counter("LOYALTY"),
        1,
        "Schaden entfernt Loyalität (CR 120.3c)"
    );
    g.deal_damage(gob, Target::Card(pw), 2);
    let d = Driver::new(g);
    assert_eq!(count(d.g(), "Liliana Lite", Zone::Graveyard), 1);
}

#[test]
fn cr_704_5k_world_rule_newest_stays() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let old = put(&db, &mut g, "Nether Void Lite", P1, Zone::Battlefield);
    let new = put(
        &db,
        &mut g,
        "Concordant Crossroads Lite",
        P2,
        Zone::Battlefield,
    );
    let d = Driver::new(g);
    assert_eq!(d.g().card(old).zone, None);
    assert_eq!(d.g().card(new).zone, Some(Zone::Battlefield));
}

#[test]
fn cr_704_5v_battle_without_defense() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let b = put(&db, &mut g, "Invasion Lite", P2, Zone::Battlefield);
    g.add_counters(b, "DEFENSE", 3);
    let gob = put(&db, &mut g, "Hill Giant", P1, Zone::Battlefield);
    g.deal_damage(gob, Target::Card(b), 3);
    let d = Driver::new(g);
    assert_eq!(count(d.g(), "Invasion Lite", Zone::Graveyard), 1);
}
