//! `NameCard` (Kartennamen nennen) und Filter `NamedCard`.

mod common;
use common::*;
use forge_engine::decision::{Action, Choice, Decision};
use forge_engine::{Step, Zone};

#[test]
fn named_card_abilities_are_blocked() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 2);
    put(&db, &mut g, "Adapt Lite", P2, Zone::Battlefield);
    let mine = put(&db, &mut g, "Adapt Lite", P1, Zone::Battlefield);
    let needle = put(&db, &mut g, "Needle Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(needle, &[]);
    d.pass_all();
    match d.decision().clone() {
        Decision::NameCard { options, .. } => {
            assert!(!options.iter().any(|n| n == "Forest"), "ValidCards nonLand");
            let i = options.iter().position(|n| n == "Adapt Lite").unwrap();
            d.submit(Choice::Number(i as u32));
        }
        o => panic!("{o:?}"),
    }
    let n = find(d.g(), "Needle Lite", Zone::Battlefield).unwrap();
    assert_eq!(d.g().card(n).named_card.as_deref(), Some("Adapt Lite"));
    assert!(!d.g().legal_actions(P1).contains(&Action::Activate(mine, 0)));
}
