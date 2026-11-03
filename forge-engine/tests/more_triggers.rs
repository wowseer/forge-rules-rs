//! Trigger `CounterAddedOnce`, `AbilityCast`; Suche in fremder Hand (`DefinedPlayer$`).

mod common;
use common::*;
use forge_engine::decision::{Action, CardChoiceReason, Choice, Decision};
use forge_engine::{Step, Zone};

#[test]
fn counter_added_once_and_ability_cast() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 1);
    put(&db, &mut g, "Counter Watch Lite", P1, Zone::Battlefield);
    put(&db, &mut g, "Ability Watch Lite", P1, Zone::Battlefield);
    let elf = put(&db, &mut g, "Adapt Lite", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.act(Action::Activate(elf, 0));
    d.finish_casting(&[]);
    d.pass_all(); // AbilityCast-Trigger (+3)
    d.pass_all(); // Adapt: 2 Marken in einem Ereignis
    d.pass_all(); // CounterAddedOnce (+1, nicht +2)
    assert_eq!(d.g().card(elf).counter("P1P1"), 2);
    assert_eq!(d.g().player(P1).life, 24);
}

#[test]
fn exile_from_opponents_hand_chosen_by_caster() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Swamp", P1, Zone::Battlefield, 1);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Hand);
    put(&db, &mut g, "Forest", P2, Zone::Hand);
    let s = put(&db, &mut g, "Strip Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(s, &[T2]);
    d.pass_all();
    match d.decision().clone() {
        Decision::ChooseCards {
            player,
            options,
            min,
            reason: CardChoiceReason::Search,
            ..
        } => {
            assert_eq!(player, P1, "Chooser You");
            assert_eq!(options, vec![bear]);
            assert_eq!(min, 1, "Mandatory");
            d.submit(Choice::Cards(vec![bear]));
        }
        o => panic!("{o:?}"),
    }
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Exile), 1);
}
