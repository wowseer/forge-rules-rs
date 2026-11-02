//! `Play`-Effekt (Java `PlayEffect`): Karten während der Auflösung spielen
//! oder wirken (CR 608.2g, 601.2), ohne Manakosten oder bezahlt.

mod common;
use common::*;
use forge_engine::decision::{CardChoiceReason, Choice, Decision, PayAction, Target};
use forge_engine::{Step, Zone};

fn choose_play(d: &mut Driver, pick: Option<forge_engine::CardId>) -> Vec<forge_engine::CardId> {
    match d.decision().clone() {
        Decision::ChooseCards {
            reason: CardChoiceReason::Play,
            options,
            ..
        } => {
            d.submit(Choice::Cards(pick.into_iter().collect()));
            options
        }
        o => panic!("{o:?}"),
    }
}

#[test]
fn cast_spell_from_hand_without_mana_cost() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 2);
    let free = put(&db, &mut g, "Free Cast Lite", P1, Zone::Hand);
    let rot = put(&db, &mut g, "Mind Rot Lite", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(free, &[]);
    d.pass_all();
    let options = choose_play(&mut d, Some(rot));
    assert_eq!(
        options,
        vec![rot],
        "nur Spontanzauber/Hexereien, keine Kreatur"
    );
    d.finish_casting(&[T2]);
    assert_eq!(d.g().stack.len(), 1, "Mind Rot liegt auf dem Stapel");
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 18);
    assert_eq!(d.g().card(bear).zone, Some(Zone::Hand));
    assert!(d.g().player(P1).mana_pool.is_empty());
}

#[test]
fn recast_from_graveyard_is_exiled_afterwards() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 1);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Graveyard);
    let recast = put(&db, &mut g, "Recast Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(recast, &[Target::Card(bolt)]);
    d.pass_all();
    // Bolt wird gewirkt: Ziel wählen
    d.finish_casting(&[T2]);
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 17);
    assert_eq!(
        count(d.g(), "Lightning Bolt", Zone::Exile),
        1,
        "statt Friedhof ins Exil"
    );
    assert_eq!(count(d.g(), "Lightning Bolt", Zone::Graveyard), 0);
    assert_eq!(count(d.g(), "Recast Lite", Zone::Graveyard), 1);
}

#[test]
fn paid_play_can_be_cancelled_and_lands_use_land_drop() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 1);
    let paid = put(&db, &mut g, "Paid Cast Lite", P1, Zone::Hand);
    let giant = put(&db, &mut g, "Hill Giant", P1, Zone::Hand);
    let forest = put(&db, &mut g, "Forest", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(paid, &[]);
    d.pass_all();
    let options = choose_play(&mut d, Some(giant));
    assert!(
        options.contains(&forest),
        "Land, solange ein Landspiel frei ist"
    );
    // Kein Mana mehr übrig → abbrechen: der Riese bleibt auf der Hand
    match d.decision().clone() {
        Decision::PayMana { .. } => d.submit(Choice::Pay(PayAction::Cancel)),
        o => panic!("{o:?}"),
    }
    assert_eq!(d.g().card(giant).zone, Some(Zone::Hand));
    assert_eq!(count(d.g(), "Paid Cast Lite", Zone::Graveyard), 1);
    assert!(d.g().stack.is_empty());
}

#[test]
fn play_land_by_effect_counts_as_land_drop() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 1);
    let paid = put(&db, &mut g, "Paid Cast Lite", P1, Zone::Hand);
    let forest = put(&db, &mut g, "Forest", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(paid, &[]);
    d.pass_all();
    choose_play(&mut d, Some(forest));
    assert_eq!(count(d.g(), "Forest", Zone::Battlefield), 1);
    assert_eq!(d.g().player(P1).lands_played_this_turn, 1);
}
