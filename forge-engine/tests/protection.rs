//! Protection (CR 702.16): nicht Ziel, kein Schaden, nicht blockbar, nicht verzaubert.

mod common;
use common::*;
use forge_engine::decision::{Action, Choice, Decision, Target};
use forge_engine::{Step, Zone};

#[test]
fn cannot_be_targeted_by_black_spell() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Swamp", P1, Zone::Battlefield, 1);
    let pal = put(&db, &mut g, "Paladin Lite", P2, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let bolt = put(&db, &mut g, "Black Bolt Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.act(Action::Cast(bolt));
    match d.decision().clone() {
        Decision::ChooseTargets { candidates, .. } => {
            assert!(!candidates.contains(&Target::Card(pal)));
            assert!(candidates.contains(&Target::Card(bear)));
        }
        o => panic!("{o:?}"),
    }
}

#[test]
fn cannot_be_blocked_by_black_creatures_and_prevents_their_damage() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let pal = put(&db, &mut g, "Paladin Lite", P1, Zone::Battlefield);
    let rats = put(&db, &mut g, "Typhoid Rats", P2, Zone::Battlefield);
    assert!(
        !g.can_block(rats, pal),
        "schwarze Kreatur kann nicht blocken"
    );
    // Schaden einer schwarzen Quelle wird verhindert
    g.deal_damage_ex(rats, Target::Card(pal), 1, false);
    assert_eq!(g.card(pal).damage, 0);
    assert!(!g.card(pal).deathtouch_damage);
}

#[test]
fn black_aura_falls_off() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let pal = put(&db, &mut g, "Paladin Lite", P1, Zone::Battlefield);
    let aura = put(&db, &mut g, "Black Aura Lite", P1, Zone::Battlefield);
    g.card_mut(aura).attached_to = Some(pal);
    let d = Driver::new(g);
    assert_eq!(
        count(d.g(), "Black Aura Lite", Zone::Graveyard),
        1,
        "CR 702.16d"
    );
}

#[test]
fn protection_effect_with_chosen_color() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 1);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let rats = put(&db, &mut g, "Typhoid Rats", P2, Zone::Battlefield);
    let spell = put(&db, &mut g, "Gods Willing Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(spell, &[Target::Card(bear)]);
    d.pass_all();
    match d.decision() {
        Decision::ChooseColor { .. } => d.submit(Choice::Color(forge_types::Color::Black)),
        o => panic!("{o:?}"),
    }
    assert!(!d.g().can_block(rats, bear));
}
