//! Kampf und Kampf-Keywords (CR 506–511, 702).

mod common;
use common::*;
use forge_engine::decision::{Choice, Decision};
use forge_engine::{CardId, GameState, Step, Zone};

fn to_attackers(g: GameState) -> Driver {
    let mut d = Driver::new(g);
    d.run_until(|_, dec| matches!(dec, Decision::DeclareAttackers { .. }));
    d
}

fn attack(d: &mut Driver, cards: &[CardId]) {
    d.submit(Choice::Attackers(cards.iter().map(|c| (*c, P2)).collect()));
}

fn block(d: &mut Driver, blocks: &[(CardId, CardId)]) {
    d.run_until(|_, dec| matches!(dec, Decision::DeclareBlockers { .. }));
    d.submit(Choice::Blocks(blocks.to_vec()));
}

fn to_main2(d: &mut Driver) {
    d.run_until(|g, _| g.step == Step::Main2);
}

#[test]
fn unblocked_attacker_damages_player_and_taps() {
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let mut d = to_attackers(g);
    attack(&mut d, &[bear]);
    assert!(d.g().card(bear).tapped, "CR 508.1f");
    to_main2(&mut d);
    assert_eq!(d.g().player(P2).life, 18);
}

#[test]
fn vigilance_does_not_tap() {
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    let angel = put(&db, &mut g, "Serra Angel", P1, Zone::Battlefield);
    let mut d = to_attackers(g);
    attack(&mut d, &[angel]);
    assert!(!d.g().card(angel).tapped);
}

#[test]
fn summoning_sickness_haste_defender() {
    // CR 302.6, 702.10, 702.3b
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    g.card_mut(bear).controlled_since_turn_start = false;
    let gob = put(&db, &mut g, "Raging Goblin", P1, Zone::Battlefield);
    g.card_mut(gob).controlled_since_turn_start = false;
    put(&db, &mut g, "Wall of Stone", P1, Zone::Battlefield);
    let d = to_attackers(g);
    match d.decision() {
        Decision::DeclareAttackers { candidates, .. } => assert_eq!(candidates, &vec![gob]),
        o => panic!("{o:?}"),
    }
}

#[test]
fn blocked_creatures_trade() {
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    let a = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let b = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = to_attackers(g);
    attack(&mut d, &[a]);
    block(&mut d, &[(b, a)]);
    to_main2(&mut d);
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Graveyard), 2);
    assert_eq!(d.g().player(P2).life, 20);
}

#[test]
fn flying_needs_flying_or_reach_to_block() {
    // CR 702.9b, 702.17b
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    let angel = put(&db, &mut g, "Serra Angel", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let spider = put(&db, &mut g, "Giant Spider", P2, Zone::Battlefield);
    let mut d = to_attackers(g);
    attack(&mut d, &[angel]);
    d.run_until(|_, dec| matches!(dec, Decision::DeclareBlockers { .. }));
    assert!(d.try_submit(Choice::Blocks(vec![(bear, angel)])).is_err());
    d.submit(Choice::Blocks(vec![(spider, angel)]));
    to_main2(&mut d);
    assert_eq!(d.g().card(spider).damage, 4);
    assert_eq!(d.g().player(P2).life, 20);
}

#[test]
fn menace_needs_two_blockers() {
    // CR 702.111b
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    let gob = put(&db, &mut g, "Goblin Heelcutter", P1, Zone::Battlefield);
    let b1 = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let b2 = put(&db, &mut g, "Giant Spider", P2, Zone::Battlefield);
    let mut d = to_attackers(g);
    attack(&mut d, &[gob]);
    d.run_until(|_, dec| matches!(dec, Decision::DeclareBlockers { .. }));
    assert!(d.try_submit(Choice::Blocks(vec![(b1, gob)])).is_err());
    d.submit(Choice::Blocks(vec![(b1, gob), (b2, gob)]));
    // Angreifer verteilt 3 Schaden auf zwei Blocker
    d.run_until(|_, dec| matches!(dec, Decision::AssignCombatDamage { .. }));
    d.submit(Choice::Damage(vec![2, 1]));
    to_main2(&mut d);
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Graveyard), 1);
    assert_eq!(count(d.g(), "Goblin Heelcutter", Zone::Graveyard), 1);
}

#[test]
fn trample_assigns_excess_to_player() {
    // CR 702.19b/c
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    let wurm = put(&db, &mut g, "Craw Wurm", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = to_attackers(g);
    attack(&mut d, &[wurm]);
    block(&mut d, &[(bear, wurm)]);
    d.run_until(|_, dec| matches!(dec, Decision::AssignCombatDamage { .. }));
    match d.decision() {
        Decision::AssignCombatDamage {
            amount: 6,
            lethal,
            trample_to: Some(p),
            ..
        } => {
            assert_eq!(lethal, &vec![2]);
            assert_eq!(*p, P2);
        }
        o => panic!("{o:?}"),
    }
    assert!(
        d.try_submit(Choice::Damage(vec![1, 5])).is_err(),
        "erst tödlicher Schaden an Blocker"
    );
    d.submit(Choice::Damage(vec![2, 4]));
    to_main2(&mut d);
    assert_eq!(d.g().player(P2).life, 16);
}

#[test]
fn trample_with_removed_blocker_hits_player() {
    // CR 702.19e
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    let wurm = put(&db, &mut g, "Craw Wurm", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    put(&db, &mut g, "Mountain", P1, Zone::Battlefield);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let mut d = to_attackers(g);
    attack(&mut d, &[wurm]);
    block(&mut d, &[(bear, wurm)]);
    d.priority_in(Step::DeclareBlockers, P1);
    d.cast(bolt, &[forge_engine::Target::Card(bear)]);
    d.pass_all();
    to_main2(&mut d);
    assert_eq!(d.g().player(P2).life, 14);
}

#[test]
fn blocked_attacker_without_trample_deals_no_damage_if_blocker_removed() {
    // CR 509.1h
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    let giant = put(&db, &mut g, "Hill Giant", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    put(&db, &mut g, "Mountain", P1, Zone::Battlefield);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let mut d = to_attackers(g);
    attack(&mut d, &[giant]);
    block(&mut d, &[(bear, giant)]);
    d.priority_in(Step::DeclareBlockers, P1);
    d.cast(bolt, &[forge_engine::Target::Card(bear)]);
    d.pass_all();
    to_main2(&mut d);
    assert_eq!(d.g().player(P2).life, 20);
}

#[test]
fn first_strike_kills_before_regular_damage() {
    // CR 510.4, 702.7
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    let knight = put(&db, &mut g, "White Knight", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = to_attackers(g);
    attack(&mut d, &[knight]);
    block(&mut d, &[(bear, knight)]);
    to_main2(&mut d);
    assert_eq!(d.g().card(knight).zone, Some(Zone::Battlefield));
    assert_eq!(d.g().card(knight).damage, 0);
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Graveyard), 1);
}

#[test]
fn double_strike_deals_damage_twice() {
    // CR 702.4
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    let ace = put(&db, &mut g, "Fencing Ace", P1, Zone::Battlefield);
    let mut d = to_attackers(g);
    attack(&mut d, &[ace]);
    to_main2(&mut d);
    assert_eq!(d.g().player(P2).life, 18);
}

#[test]
fn deathtouch_and_lifelink_in_combat() {
    // CR 702.2, 702.15
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    let hawk = put(&db, &mut g, "Vampire Nighthawk", P1, Zone::Battlefield);
    let wurm = put(&db, &mut g, "Craw Wurm", P2, Zone::Battlefield);
    g.card_mut(wurm).tapped = false;
    // Wurm hat keine Reichweite – blocken nicht möglich; Angriff unblockt
    let mut d = to_attackers(g);
    attack(&mut d, &[hawk]);
    to_main2(&mut d);
    assert_eq!(d.g().player(P2).life, 18);
    assert_eq!(d.g().player(P1).life, 22);
}

#[test]
fn deathtouch_lethal_is_one_for_damage_assignment() {
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    let hawk = put(&db, &mut g, "Vampire Nighthawk", P1, Zone::Battlefield);
    let s1 = put(&db, &mut g, "Giant Spider", P2, Zone::Battlefield);
    let s2 = put(&db, &mut g, "Giant Spider", P2, Zone::Battlefield);
    let mut d = to_attackers(g);
    attack(&mut d, &[hawk]);
    block(&mut d, &[(s1, hawk), (s2, hawk)]);
    d.run_until(|_, dec| matches!(dec, Decision::AssignCombatDamage { .. }));
    match d.decision() {
        Decision::AssignCombatDamage { lethal, .. } => assert_eq!(lethal, &vec![1, 1]),
        o => panic!("{o:?}"),
    }
    d.submit(Choice::Damage(vec![1, 1]));
    to_main2(&mut d);
    assert_eq!(count(d.g(), "Giant Spider", Zone::Graveyard), 2);
    assert_eq!(count(d.g(), "Vampire Nighthawk", Zone::Graveyard), 1);
    assert_eq!(
        d.g().player(P1).life,
        22,
        "Lifelink auch bei Schaden an Kreaturen"
    );
}
