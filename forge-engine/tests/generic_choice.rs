//! `GenericChoice` (Java `ChooseGenericEffect`): ein Spieler wählt eine Unterfähigkeit.

mod common;
use common::*;
use forge_engine::decision::{Choice, Decision};
use forge_engine::{Step, Zone};

fn run(pick: usize) -> (i32, usize, usize) {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Swamp", P1, Zone::Battlefield, 1);
    let c = put(&db, &mut g, "Choice Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    let hand = d.g().player(P1).hand.len();
    d.cast(c, &[]);
    d.pass_all();
    match d.decision().clone() {
        Decision::ChooseModes { player, modes, .. } => {
            assert_eq!(player, P2, "der Gegner wählt");
            assert_eq!(modes.len(), 2);
            d.submit(Choice::Modes(vec![pick]));
        }
        o => panic!("{o:?}"),
    }
    let gy = find(d.g(), "Choice Lite", Zone::Graveyard).unwrap();
    (
        d.g().player(P2).life,
        d.g().player(P1).hand.len() + 1 - hand,
        d.g().card(gy).remembered_players.len(),
    )
}

#[test]
fn opponent_chooses_draw_for_caster() {
    let (life, drawn, _) = run(0);
    assert_eq!(life, 20);
    assert_eq!(drawn, 2);
}

#[test]
fn opponent_chooses_to_lose_life_via_remembered_chooser() {
    let (life, drawn, remembered) = run(1);
    assert_eq!(life, 17);
    assert_eq!(drawn, 0);
    assert_eq!(remembered, 0, "TempRemember wird wieder entfernt");
}
