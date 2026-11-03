//! Morph / Megamorph (CR 702.37), verdeckte Objekte (CR 708).

mod common;
use common::*;
use forge_engine::decision::Action;
use forge_engine::keyword::Keyword;
use forge_engine::{Step, Zone};

#[test]
fn cast_face_down_then_turn_up_with_trigger() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 6);
    let card = put(&db, &mut g, "Morph Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    assert!(
        d.g()
            .legal_actions(P1)
            .contains(&Action::CastFaceDown(card))
    );
    d.act(Action::CastFaceDown(card));
    d.finish_casting(&[]);
    d.pass_all();
    let fd = d
        .g()
        .battlefield_of(P1)
        .find(|c| d.g().card(*c).face_down)
        .expect("verdeckt");
    let ch = d.g().chars(fd);
    assert_eq!((ch.power, ch.toughness), (Some(2), Some(2)));
    assert!(ch.name.is_empty() && ch.colors.is_colorless());
    assert!(!ch.has(&Keyword::Flying), "keine Fähigkeiten");
    assert_eq!(
        d.g()
            .battlefield
            .iter()
            .filter(|c| d.g().card(**c).tapped)
            .count(),
        3
    );
    let hand = d.g().player(P1).hand.len();
    d.act(Action::TurnFaceUp(fd));
    d.finish_casting(&[]);
    assert!(!d.g().card(fd).face_down);
    assert_eq!(d.g().chars(fd).power, Some(4));
    assert!(d.g().chars(fd).has(&Keyword::Flying));
    d.pass_all(); // TurnFaceUp-Trigger
    assert_eq!(d.g().player(P1).hand.len(), hand + 1);
}

#[test]
fn megamorph_adds_counter_and_leaving_reveals() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 4);
    let card = put(&db, &mut g, "Mega Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.act(Action::CastFaceDown(card));
    d.finish_casting(&[]);
    d.pass_all();
    let fd = d
        .g()
        .battlefield_of(P1)
        .find(|c| d.g().card(*c).face_down)
        .unwrap();
    d.act(Action::TurnFaceUp(fd));
    d.finish_casting(&[]);
    assert_eq!(d.g().chars(fd).power, Some(4), "3/3 + Marke");
    let mut g = d.g().clone();
    let gy = g.move_card(fd, Zone::Graveyard, true);
    assert!(!g.card(gy).face_down);
}
