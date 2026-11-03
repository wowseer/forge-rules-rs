//! `Attach | Defined$` und `Reveal`.

mod common;
use common::*;
use forge_engine::decision::{Choice, Decision};
use forge_engine::{Step, Zone};

#[test]
fn equipment_attaches_to_entering_creature() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 2);
    let eq = put(&db, &mut g, "Auto Equip Lite", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(bear, &[]);
    d.pass_all(); // Kreatur
    d.pass_all(); // Trigger
    let b = find(d.g(), "Grizzly Bears", Zone::Battlefield).unwrap();
    assert_eq!(d.g().card(eq).attached_to, Some(b));
    assert_eq!(d.g().chars(b).power, Some(4));
}

#[test]
fn reveal_creature_from_hand_and_use_it() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 1);
    let giant = put(&db, &mut g, "Hill Giant", P1, Zone::Hand);
    put(&db, &mut g, "Forest", P1, Zone::Hand);
    let s = put(&db, &mut g, "Reveal Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(s, &[]);
    d.pass_all();
    match d.decision().clone() {
        Decision::ChooseCards { options, .. } => {
            assert_eq!(options, vec![giant], "nur Kreaturen");
            d.submit(Choice::Cards(vec![giant]));
        }
        o => panic!("{o:?}"),
    }
    assert_eq!(d.g().player(P1).life, 23);
    assert_eq!(d.g().card(giant).zone, Some(Zone::Hand));
}
