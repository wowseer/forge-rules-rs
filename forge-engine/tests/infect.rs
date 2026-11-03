//! Infect (CR 702.90), Wither (CR 702.80), Toxic (CR 702.164), Historic (CR 700.6).

mod common;
use common::*;
use forge_engine::decision::Target;
use forge_engine::{Step, Zone};

#[test]
fn infect_and_wither_damage() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let inf = put(&db, &mut g, "Infect Lite", P1, Zone::Battlefield);
    let wit = put(&db, &mut g, "Wither Lite", P1, Zone::Battlefield);
    let giant = put(&db, &mut g, "Hill Giant", P2, Zone::Battlefield);
    g.deal_damage_ex(inf, Target::Player(P2), 2, true);
    assert_eq!(g.player(P2).poison, 2);
    assert_eq!(g.player(P2).life, 20, "kein Lebensverlust");
    g.deal_damage_ex(wit, Target::Card(giant), 1, false);
    g.deal_damage_ex(inf, Target::Card(giant), 1, false);
    assert_eq!(g.card(giant).counter("M1M1"), 2);
    assert_eq!(g.card(giant).damage, 0);
    assert_eq!(g.chars(giant).power, Some(1));
    // Wither an Spieler: normaler Lebensverlust
    g.deal_damage_ex(wit, Target::Player(P2), 2, false);
    assert_eq!(g.player(P2).life, 18);
}

#[test]
fn toxic_adds_poison_on_combat_damage_only() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let tox = put(&db, &mut g, "Toxic Lite", P1, Zone::Battlefield);
    g.deal_damage_ex(tox, Target::Player(P2), 1, true);
    assert_eq!((g.player(P2).life, g.player(P2).poison), (19, 2));
    g.deal_damage_ex(tox, Target::Player(P2), 1, false);
    assert_eq!((g.player(P2).life, g.player(P2).poison), (18, 2));
}

#[test]
fn historic_filter() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let thalia = put(&db, &mut g, "Thalia Lite", P1, Zone::Battlefield);
    let orni = put(&db, &mut g, "Ornithopter", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let spec = forge_engine::valid::ValidSpec::parse("Permanent.Historic").unwrap();
    assert!(g.card_matches(thalia, &spec, bear, P1), "legendär");
    assert!(g.card_matches(orni, &spec, bear, P1), "Artefakt");
    assert!(!g.card_matches(bear, &spec, bear, P1));
}
