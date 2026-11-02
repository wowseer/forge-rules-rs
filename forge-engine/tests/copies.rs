//! Kopien (CR 707): Clone beim Ankommen, Token-Kopien (`CopyPermanent`).

mod common;
use common::*;
use forge_engine::decision::{Action, Choice, Decision, Target};
use forge_engine::keyword::Keyword;
use forge_engine::{Step, Zone};

#[test]
fn clone_enters_as_copy_with_etb_and_reverts_when_leaving() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Island", P1, Zone::Battlefield);
    let vis = put(&db, &mut g, "Elvish Visionary", P2, Zone::Battlefield);
    let clone = put(&db, &mut g, "Clone Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    let hand = d.g().player(P1).hand.len();
    d.cast(clone, &[]);
    d.pass_all();
    assert!(matches!(d.decision(), Decision::Confirm { .. }));
    d.submit(Choice::Yes);
    d.submit(Choice::Cards(vec![vis]));
    let c = find(d.g(), "Elvish Visionary", Zone::Battlefield)
        .into_iter()
        .chain(d.g().battlefield.iter().copied())
        .find(|x| d.g().card(*x).owner == P1 && d.g().card(*x).copy.is_some())
        .expect("Kopie im Spiel");
    assert_eq!(d.g().chars(c).name, "Elvish Visionary");
    assert_eq!(d.g().chars(c).power, Some(1), "nicht 0/0 gestorben");
    d.pass_all(); // ETB-Trigger der Kopie
    assert_eq!(d.g().player(P1).hand.len(), hand - 1 + 1);
    let mut g = d.g().clone();
    let new = g.move_card(c, Zone::Graveyard, true);
    assert_eq!(
        g.chars(new).name,
        "Clone Lite",
        "CR 707: Kopie endet mit Zonenwechsel"
    );
}

#[test]
fn token_copy_with_haste_sacrificed_at_end_step() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let kiki = put(&db, &mut g, "Kiki Lite", P1, Zone::Battlefield);
    let giant = put(&db, &mut g, "Hill Giant", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.act(Action::Activate(kiki, 0));
    d.submit(Choice::Targets(vec![Target::Card(giant)]));
    d.pass_all();
    let tok = d
        .g()
        .battlefield
        .iter()
        .copied()
        .find(|x| d.g().card(*x).token)
        .expect("Token-Kopie");
    assert_eq!(d.g().chars(tok).name, "Hill Giant");
    assert!(d.g().has_keyword(tok, &Keyword::Haste));
    d.run_until(|g, dec| g.step == Step::End && matches!(dec, Decision::Priority { .. }));
    d.pass_all();
    assert!(
        !d.g().battlefield.iter().any(|x| d.g().card(*x).token),
        "zu Beginn des Endsegments geopfert"
    );
}
