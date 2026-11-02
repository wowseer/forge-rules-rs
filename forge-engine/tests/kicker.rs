//! Kicker (CR 702.33): Zusatzkosten beim Wirken, „wenn gekickt“.

mod common;
use common::*;
use forge_engine::decision::{Action, Choice, Decision};
use forge_engine::{Step, Zone};

fn cast_with_kicker(
    d: &mut Driver,
    c: forge_engine::CardId,
    kick: bool,
    targets: &[forge_engine::decision::Target],
) {
    d.act(Action::Cast(c));
    match d.decision() {
        Decision::Confirm { .. } => d.submit(if kick { Choice::Yes } else { Choice::No }),
        o => panic!("Kicker-Frage erwartet: {o:?}"),
    }
    d.finish_casting(targets);
}

#[test]
fn kicked_spell_pays_more_and_does_more() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 3);
    let bolt = put(&db, &mut g, "Kick Bolt Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    let hand = d.g().player(P1).hand.len();
    cast_with_kicker(&mut d, bolt, true, &[T2]);
    assert!(
        d.g()
            .battlefield
            .iter()
            .filter(|c| d.g().card(**c).tapped)
            .count()
            == 3,
        "R + 2 bezahlt"
    );
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 15);
    assert_eq!(
        d.g().player(P1).hand.len(),
        hand,
        "−1 gewirkt, +1 durch Kicker"
    );
}

#[test]
fn unkicked_spell_and_no_offer_when_unaffordable() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 1);
    let bolt = put(&db, &mut g, "Kick Bolt Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    // nur ein Mana: keine Kicker-Frage
    d.cast(bolt, &[T2]);
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 18);
}

#[test]
fn kicked_creature_enters_with_counters_and_triggers() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 5);
    let kavu = put(&db, &mut g, "Kavu Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    cast_with_kicker(&mut d, kavu, true, &[]);
    d.pass_all();
    let k = find(d.g(), "Kavu Lite", Zone::Battlefield).unwrap();
    assert_eq!(d.g().card(k).counter("P1P1"), 3);
    d.pass_all(); // ETB-Trigger „wenn gekickt“
    assert_eq!(d.g().player(P1).life, 24);
}

#[test]
fn unkicked_creature_plain() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 5);
    let kavu = put(&db, &mut g, "Kavu Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    cast_with_kicker(&mut d, kavu, false, &[]);
    d.pass_all();
    let k = find(d.g(), "Kavu Lite", Zone::Battlefield).unwrap();
    assert_eq!(d.g().card(k).counter("P1P1"), 0);
    assert!(d.g().stack.is_empty(), "kein Trigger");
    assert_eq!(d.g().player(P1).life, 20);
}
