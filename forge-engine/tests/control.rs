//! Kontrollwechsel (`GainControl`, CR 613.1b, 611.2b).

mod common;
use common::*;
use forge_engine::decision::{Action, Choice, Decision, Target};
use forge_engine::keyword::Keyword;
use forge_engine::{Step, Zone};

#[test]
fn threaten_until_end_of_turn_untaps_and_grants_haste() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 3);
    let th = put(&db, &mut g, "Threaten Lite", P1, Zone::Hand);
    let giant = put(&db, &mut g, "Hill Giant", P2, Zone::Battlefield);
    g.card_mut(giant).tapped = true;
    let mut d = Driver::new(g);
    d.cast(th, &[Target::Card(giant)]);
    d.pass_all();
    assert_eq!(d.g().card(giant).controller, P1);
    assert!(!d.g().card(giant).tapped);
    assert!(d.g().has_keyword(giant, &Keyword::Haste));
    // kann angreifen (Eile trotz Kontrollwechsel, CR 302.6)
    d.run_until(|_, dec| matches!(dec, Decision::DeclareAttackers { .. }));
    d.submit(Choice::Attackers(vec![(giant, T2)]));
    d.run_until(|g, _| g.step == Step::Main2);
    assert_eq!(d.g().player(P2).life, 17);
    d.run_until(|g, _| g.turn == 3);
    assert_eq!(d.g().card(giant).controller, P2, "zurück im Cleanup");
}

#[test]
fn permanent_control_change() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Island", P1, Zone::Battlefield);
    let s = put(&db, &mut g, "Steal Lite", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(s, &[Target::Card(bear)]);
    d.pass_all();
    d.run_until(|g, _| g.turn == 4 && g.step == Step::Main1);
    assert_eq!(d.g().card(bear).controller, P1);
}

#[test]
fn control_ends_when_source_leaves_battlefield() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let seizer = put(&db, &mut g, "Seizer Lite", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.act(Action::Activate(seizer, 0));
    d.finish_casting(&[Target::Card(bear)]);
    d.pass_all();
    assert_eq!(d.g().card(bear).controller, P1);
    // Seizer verlässt das Battlefield → Kontrolle endet sofort (CR 611.2b)
    let mut g = d.g().clone();
    g.destroy(seizer);
    let mut d = Driver::new(g);
    d.pass_all();
    assert_eq!(d.g().card(bear).controller, P2);
}
