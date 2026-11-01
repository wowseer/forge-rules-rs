//! Statische Fähigkeiten (Continuous) und Layer-System (CR 604, 613).

mod common;
use common::*;
use forge_engine::decision::{Action, Choice, Decision, Target};
use forge_engine::keyword::Keyword;
use forge_engine::{Step, Zone};

#[test]
fn anthem_affects_only_own_creatures_and_new_ones() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Glorious Anthem", P1, Zone::Battlefield);
    let mine = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let theirs = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 2);
    let later = put(&db, &mut g, "Grizzly Bears", P1, Zone::Hand);
    assert_eq!(g.chars(mine).power, Some(3));
    assert_eq!(g.chars(theirs).power, Some(2));
    let mut d = Driver::new(g);
    d.cast(later, &[]);
    d.pass_all();
    let new = d.g().battlefield.last().copied().unwrap();
    assert_eq!(
        d.g().chars(new).toughness,
        Some(3),
        "gilt auch für später hinzukommende (CR 611.3a)"
    );
}

#[test]
fn effect_ends_when_source_leaves() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let anthem = put(&db, &mut g, "Glorious Anthem", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    g.move_card(anthem, Zone::Graveyard, true);
    assert_eq!(g.chars(bear).power, Some(2));
}

#[test]
fn lord_other_and_granted_haste() {
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    let lord = put(&db, &mut g, "Goblin Lord Lite", P1, Zone::Battlefield);
    let gob = put(&db, &mut g, "Goblin Piker", P1, Zone::Battlefield);
    g.card_mut(gob).controlled_since_turn_start = false;
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    assert_eq!(g.chars(lord).power, Some(2), "Other: nicht sich selbst");
    assert_eq!(g.chars(gob).power, Some(3));
    assert_eq!(g.chars(bear).power, Some(2), "nur Goblins");
    let d = {
        let mut d = Driver::new(g);
        d.run_until(|_, dec| matches!(dec, Decision::DeclareAttackers { .. }));
        d
    };
    match d.decision() {
        Decision::DeclareAttackers { candidates, .. } => {
            assert!(candidates.contains(&gob), "Haste aus statischer Fähigkeit")
        }
        o => panic!("{o:?}"),
    }
}

#[test]
fn layer_7b_before_7c_regardless_of_timestamp() {
    // CR 613.4b/c: Pump (früher) und Anthem wirken nach dem Setzen auf 1/1
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Forest", P1, Zone::Battlefield);
    let growth = put(&db, &mut g, "Giant Growth", P1, Zone::Hand);
    let wurm = put(&db, &mut g, "Craw Wurm", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(growth, &[Target::Card(wurm)]);
    d.pass_all();
    let mut g = d.g().clone();
    put(&db, &mut g, "Humility Lite", P2, Zone::Battlefield);
    put(&db, &mut g, "Glorious Anthem", P1, Zone::Battlefield);
    g.add_counters(wurm, "P1P1", 1);
    let ch = g.chars(wurm);
    assert_eq!(
        (ch.power, ch.toughness),
        (Some(1 + 3 + 1 + 1), Some(1 + 3 + 1 + 1))
    );
}

#[test]
fn affected_set_determined_after_type_change() {
    // CR 613.6: Living Lands macht Wälder in Layer 4 zu Kreaturen → Anthem (7c) erfasst sie
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Glorious Anthem", P1, Zone::Battlefield);
    let forest = put(&db, &mut g, "Forest", P1, Zone::Battlefield);
    let island = put(&db, &mut g, "Island", P1, Zone::Battlefield);
    put(&db, &mut g, "Living Lands Lite", P2, Zone::Battlefield);
    let ch = g.chars(forest);
    assert!(ch.types.is_creature() && ch.types.is_land());
    assert_eq!((ch.power, ch.toughness), (Some(2), Some(2)));
    assert!(!g.chars(island).types.is_creature());
}

#[test]
fn layer_5_color_changes_targeting() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Darkest Hour", P2, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    assert!(g.chars(bear).colors.contains(forge_types::Color::Black));
    assert!(
        !g.chars(bear).colors.contains(forge_types::Color::Green),
        "Farbe wird gesetzt, nicht hinzugefügt"
    );
}

#[test]
fn static_keywords_used_by_combat() {
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    put(&db, &mut g, "Akroma's Memorial Lite", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let blocker = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    assert!(g.has_keyword(bear, &Keyword::Flying) && g.has_keyword(bear, &Keyword::FirstStrike));
    let mut d = Driver::new(g);
    d.run_until(|_, dec| matches!(dec, Decision::DeclareAttackers { .. }));
    d.submit(Choice::Attackers(vec![(bear, T2)]));
    d.run_until(|_, dec| matches!(dec, Decision::DeclareBlockers { .. }));
    assert!(
        d.try_submit(Choice::Blocks(vec![(blocker, bear)])).is_err(),
        "Flying aus Static"
    );
    d.submit(Choice::Blocks(vec![]));
    d.run_until(|g, _| g.step == Step::Main2);
    assert_eq!(d.g().player(P2).life, 18);
    let _ = Action::Pass;
}

#[test]
fn static_zero_toughness_kills_via_sba() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Zero Field Lite", P2, Zone::Battlefield);
    put(&db, &mut g, "Goblin Piker", P1, Zone::Battlefield);
    let d = Driver::new(g);
    assert_eq!(count(d.g(), "Goblin Piker", Zone::Graveyard), 1);
}

#[test]
fn affected_defined_self() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let mut db = db;
    db.add_script("Name:Self Pumper Lite\nManaCost:2\nTypes:Creature Golem\nPT:1/1\nS:Mode$ Continuous | AffectedDefined$ Self | AddPower$ 2 | AddKeyword$ Trample | Description$ CARDNAME gets +2/+0 and has trample.\nOracle:\n").unwrap();
    let golem = put(&db, &mut g, "Self Pumper Lite", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    assert_eq!(g.chars(golem).power, Some(3));
    assert!(g.has_keyword(golem, &Keyword::Trample));
    assert_eq!(g.chars(bear).power, Some(2));
}
