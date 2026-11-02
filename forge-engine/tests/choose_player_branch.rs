//! `ChoosePlayer` (Defined ChosenPlayer, Filter ChosenCtrl, Cleanup) und `Branch`.

mod common;
use common::*;
use forge_engine::{Step, Zone};

#[test]
fn chosen_opponent_loses_life_and_creatures() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Swamp", P1, Zone::Battlefield, 1);
    let mine = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let theirs = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let spell = put(&db, &mut g, "Pick Foe Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(spell, &[]);
    d.pass_all(); // einziger Gegner → keine Frage
    assert_eq!(d.g().player(P2).life, 18);
    assert_eq!(d.g().card(theirs).zone, None);
    assert_eq!(d.g().card(mine).zone, Some(Zone::Battlefield));
    let gy = find(d.g(), "Pick Foe Lite", Zone::Graveyard).unwrap();
    assert_eq!(d.g().card(gy).chosen_player, None);
}

#[test]
fn branch_picks_sub_ability_by_condition() {
    for (bears, drawn, life) in [(2, 2, 20), (1, 0, 25)] {
        let (db, mut g) = game_at(Step::Main1, 10);
        put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 1);
        put_n(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield, bears);
        let spell = put(&db, &mut g, "Branch Lite", P1, Zone::Hand);
        let mut d = Driver::new(g);
        let hand = d.g().player(P1).hand.len();
        d.cast(spell, &[]);
        d.pass_all();
        assert_eq!(d.g().player(P1).hand.len() + 1 - hand, drawn);
        assert_eq!(d.g().player(P1).life, life);
    }
}
