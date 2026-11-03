//! Amass (CR 701.47).

mod common;
use common::*;
use forge_engine::{Step, Zone};

#[test]
fn amass_creates_army_then_grows_and_gains_type() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Swamp", P1, Zone::Battlefield, 2);
    let a = put(&db, &mut g, "Amass Lite", P1, Zone::Hand);
    let z = put(&db, &mut g, "Amass Zombie Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(a, &[]);
    d.pass_all();
    let army = find(d.g(), "Orc Army Token", Zone::Battlefield).expect("Army");
    assert_eq!(d.g().chars(army).power, Some(2));
    d.cast(z, &[]);
    d.pass_all();
    assert_eq!(
        count(d.g(), "Orc Army Token", Zone::Battlefield),
        1,
        "keine zweite Army"
    );
    assert_eq!(d.g().chars(army).power, Some(3));
    assert!(d.g().chars(army).types.has_subtype("Zombie"));
}
