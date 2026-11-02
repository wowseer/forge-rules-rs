//! Kostenhilfen beim Wirken: Convoke (CR 702.51), Delve (CR 702.66),
//! Improvise (CR 702.126), Affinity (CR 702.41).

mod common;
use common::*;
use forge_engine::decision::{Action, CardChoiceReason, Choice, Decision};
use forge_engine::{Step, Zone};

/// Beantwortet die nächste Kostenhilfe-Auswahl mit `pick`.
fn answer_helper(d: &mut Driver, reason: CardChoiceReason, pick: &[forge_engine::CardId]) {
    match d.decision().clone() {
        Decision::ChooseCards {
            reason: r, options, ..
        } if r == reason => {
            assert!(
                pick.iter().all(|c| options.contains(c)),
                "{pick:?} in {options:?}"
            );
            d.submit(Choice::Cards(pick.to_vec()));
        }
        other => panic!("erwartet {reason:?}, war {other:?}"),
    }
}

#[test]
fn convoke_taps_creatures_for_colored_and_generic_mana() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 1);
    let goblin = put(&db, &mut g, "Goblin Piker", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let giant = put(&db, &mut g, "Hill Giant", P1, Zone::Battlefield);
    let stoke = put(&db, &mut g, "Stoke the Flames", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.act(Action::Cast(stoke));
    d.finish_casting(&[T2]);
    // {2}{R}{R}: Goblin + Riese zahlen {R}{R}, Bär {1}, Berg {1}
    answer_helper(&mut d, CardChoiceReason::Convoke, &[goblin, bear, giant]);
    d.finish_casting(&[T2]);
    for c in [goblin, bear, giant] {
        assert!(d.g().card(c).tapped, "Kreatur getappt");
    }
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 16);
    assert_eq!(count(d.g(), "Mountain", Zone::Battlefield), 1);
}

#[test]
fn convoke_green_creature_cannot_pay_red() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let bears: Vec<_> = (0..4)
        .map(|_| put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield))
        .collect();
    let stoke = put(&db, &mut g, "Stoke the Flames", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.act(Action::Cast(stoke));
    d.finish_casting(&[T2]);
    answer_helper(&mut d, CardChoiceReason::Convoke, &bears);
    // Nur zwei Bären zahlen {2}; {R}{R} bleibt offen → nicht bezahlbar
    match d.decision().clone() {
        Decision::PayMana {
            remaining, options, ..
        } => {
            assert!(remaining.starts_with("{R}{R}"), "{remaining}");
            assert!(!options.contains(&forge_engine::PayAction::AutoPay));
        }
        o => panic!("{o:?}"),
    }
    d.submit(Choice::Pay(forge_engine::PayAction::Cancel));
    // CR 728: Abbruch nimmt alles zurück
    assert!(bears.iter().all(|&b| !d.g().card(b).tapped));
    assert_eq!(d.g().card(stoke).zone, Some(Zone::Hand));
}

#[test]
fn delve_exiles_graveyard_cards_for_generic() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 2);
    let gy: Vec<_> = (0..6)
        .map(|_| put(&db, &mut g, "Grizzly Bears", P1, Zone::Graveyard))
        .collect();
    let cruise = put(&db, &mut g, "Treasure Cruise", P1, Zone::Hand);
    let mut d = Driver::new(g);
    let hand = d.g().player(P1).hand.len();
    d.act(Action::Cast(cruise));
    d.finish_casting(&[]);
    answer_helper(&mut d, CardChoiceReason::Delve, &gy);
    d.finish_casting(&[]);
    d.pass_all();
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Exile), 6);
    assert_eq!(d.g().player(P1).hand.len(), hand - 1 + 3);
}

#[test]
fn improvise_taps_artifacts_for_generic_only() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 2);
    let thopters: Vec<_> = (0..4)
        .map(|_| put(&db, &mut g, "Ornithopter", P1, Zone::Battlefield))
        .collect();
    let rev = put(&db, &mut g, "Reverse Engineer", P1, Zone::Hand);
    let mut d = Driver::new(g);
    let hand = d.g().player(P1).hand.len();
    d.act(Action::Cast(rev));
    d.finish_casting(&[]);
    match d.decision().clone() {
        Decision::ChooseCards { max, .. } => assert_eq!(max, 3, "nur generisches Mana"),
        o => panic!("{o:?}"),
    }
    answer_helper(&mut d, CardChoiceReason::Improvise, &thopters[..3]);
    d.finish_casting(&[]);
    d.pass_all();
    assert_eq!(d.g().player(P1).hand.len(), hand - 1 + 3);
    assert!(!d.g().card(thopters[3]).tapped);
}

#[test]
fn affinity_reduces_cost_per_artifact() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Ornithopter", P1, Zone::Battlefield, 3);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 1);
    let frog = put(&db, &mut g, "Frogmite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(frog, &[]);
    d.pass_all();
    assert_eq!(d.g().card(frog).zone, None, "altes Objekt");
    assert_eq!(count(d.g(), "Frogmite", Zone::Battlefield), 1);
}
