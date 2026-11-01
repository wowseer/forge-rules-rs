//! Effektobjekte (`Effect`): Statics/Trigger in der Command Zone, gemerkte Objekte, Dauer.

mod common;
use common::*;
use forge_engine::decision::{Action, Decision, Target};
use forge_engine::keyword::Keyword;
use forge_engine::{Step, Zone};

#[test]
fn effect_grants_hexproof_to_remembered_until_end_of_turn() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Plains", P1, Zone::Battlefield);
    let bl = put(&db, &mut g, "Blessing Lite", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let other = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(bl, &[Target::Card(bear)]);
    d.pass_all();
    assert!(d.g().has_keyword(bear, &Keyword::Hexproof));
    assert!(
        !d.g().has_keyword(other, &Keyword::Hexproof),
        "nur das gemerkte Objekt"
    );
    assert_eq!(d.g().command.len(), 1);
    d.run_until(|g, _| g.turn == 3);
    assert!(!d.g().has_keyword(bear, &Keyword::Hexproof));
    assert!(d.g().command.is_empty(), "Effektobjekt endet im Cleanup");
}

#[test]
fn permanent_effect_like_emblem() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 3);
    let em = put(&db, &mut g, "Emblem Anthem Lite", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(em, &[]);
    d.pass_all();
    d.run_until(|g, _| g.turn == 4);
    assert_eq!(d.g().chars(bear).power, Some(3));
}

#[test]
fn until_your_next_turn_survives_opponents_turn() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 2);
    let fw = put(&db, &mut g, "Fog Wall Lite", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(fw, &[]);
    d.pass_all();
    d.run_until(|g, _| g.turn == 3 && g.step == Step::Upkeep);
    assert_eq!(
        d.g().chars(bear).toughness,
        Some(5),
        "im Zug des Gegners aktiv"
    );
    d.run_until(|g, _| g.turn == 4 && g.step == Step::Upkeep);
    assert_eq!(
        d.g().chars(bear).toughness,
        Some(2),
        "endet zu Beginn des eigenen Zuges"
    );
}

#[test]
fn effect_trigger_fires_from_command_zone() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Swamp", P1, Zone::Battlefield);
    put(&db, &mut g, "Swamp", P1, Zone::Battlefield);
    put(&db, &mut g, "Mountain", P1, Zone::Battlefield);
    let gh = put(&db, &mut g, "Grim Harvest Lite", P1, Zone::Hand);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(gh, &[]);
    d.pass_all();
    let hand = d.g().player(P1).hand.len();
    d.cast(bolt, &[Target::Card(bear)]);
    d.pass_all();
    assert!(matches!(d.decision(), Decision::Priority { .. }));
    d.pass_all();
    assert_eq!(
        d.g().player(P1).hand.len(),
        hand - 1 + 1,
        "Bolt gespielt, eine Karte gezogen"
    );
    let _ = Action::Pass;
}
