//! Imprint, Suspect, Tag/Nacht, Speed, verliehene Fähigkeiten/Trigger.

mod common;
use common::*;
use forge_engine::decision::{Action, Choice, Decision, Target};
use forge_engine::keyword::Keyword;
use forge_engine::{Step, Zone};

#[test]
fn imprint_and_use_imprinted_value() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 1);
    let art = put(&db, &mut g, "Imprint Kw", P1, Zone::Hand);
    let giant = put(&db, &mut g, "Hill Giant", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(art, &[]);
    d.pass_all();
    d.pass_all();
    match d.decision().clone() {
        Decision::ChooseCards { options, .. } => {
            assert!(options.contains(&giant));
            d.submit(Choice::Cards(vec![giant]));
        }
        o => panic!("{o:?}"),
    }
    let a = find(d.g(), "Imprint Kw", Zone::Battlefield).unwrap();
    assert_eq!(d.g().card(a).imprinted.len(), 1);
    d.act(Action::Activate(a, 0));
    d.finish_casting(&[]);
    d.pass_all();
    assert_eq!(
        d.g().player(P1).life,
        24,
        "Manawert des eingeprägten Riesen"
    );
}

#[test]
fn suspected_has_menace_and_cant_block() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Swamp", P1, Zone::Battlefield, 1);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let s = put(&db, &mut g, "Suspect Kw", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(s, &[Target::Card(bear)]);
    d.pass_all();
    assert!(d.g().card(bear).suspected);
    assert!(d.g().chars(bear).has(&Keyword::Menace));
    assert!(d.g().chars(bear).has(&Keyword::CantBlock));
}

#[test]
fn day_night_transforms_werewolf() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let w = put(&db, &mut g, "Wolf Kw", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    assert_eq!(d.g().day, Some(true), "Daybound macht es Tag");
    // P1 wirkt keinen Zauber → nächster Zug wird Nacht
    d.run_until(|g, _| g.active == P2 && g.step == Step::Upkeep);
    assert_eq!(d.g().day, Some(false));
    assert_eq!(d.g().chars(w).power, Some(4), "Nachtseite");
}

#[test]
fn speed_rises_when_opponent_loses_life_on_your_turn() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 1);
    let racer = put(&db, &mut g, "Racer Kw", P1, Zone::Battlefield);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let mut d = Driver::new(g);
    assert_eq!(d.g().player(P1).speed, Some(1));
    d.cast(bolt, &[T2]);
    d.pass_all();
    assert_eq!(d.g().player(P1).speed, Some(2));
    let mut g = d.g().clone();
    g.player_mut(P1).speed = Some(4);
    assert_eq!(g.chars(racer).power, Some(4), "Max speed");
}

#[test]
fn granted_mana_ability_and_trigger() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Mana Giver Kw", P1, Zone::Battlefield);
    put(&db, &mut g, "Trigger Giver Kw", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    assert_eq!(g.activated(bear).len(), 1);
    assert!(
        g.legal_actions(P1).contains(&Action::Cast(bolt)),
        "Bär liefert Mana"
    );
    let mut d = Driver::new(g);
    d.run_until(|_, dec| matches!(dec, Decision::DeclareAttackers { .. }));
    d.submit(Choice::Attackers(vec![(bear, T2)]));
    d.run_until(|g, _| g.step == Step::EndCombat);
    assert_eq!(d.g().player(P1).life, 21, "verliehener Angriffstrigger");
}
