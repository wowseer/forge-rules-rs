//! FlipCoin (CR 705), ChooseNumber, Goad (CR 701.38).

mod common;
use common::*;
use forge_engine::decision::{Choice, Decision, Target};
use forge_engine::state::Event;
use forge_engine::{Step, Zone};

#[test]
fn coin_flip_win_or_lose() {
    let mut seen = [false, false];
    for seed in 0..10 {
        let (db, mut g) = game_at(Step::Main1, 10);
        g.rng = forge_engine::rng::Rng::new(seed);
        put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 1);
        let s = put(&db, &mut g, "Coin Lite", P1, Zone::Hand);
        let mut d = Driver::new(g);
        d.cast(s, &[]);
        d.pass_all();
        let won = d.g().log.iter().rev().find_map(|e| match e {
            Event::CoinFlipped { won, .. } => Some(*won),
            _ => None,
        });
        match won {
            Some(true) => assert_eq!(d.g().player(P2).life, 16),
            Some(false) => assert_eq!(d.g().player(P1).life, 19),
            None => panic!("kein Münzwurf"),
        }
        seen[usize::from(won.unwrap())] = true;
    }
    assert!(seen[0] && seen[1], "beide Ausgänge kommen vor");
}

#[test]
fn chosen_number_is_used() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 1);
    let s = put(&db, &mut g, "Number Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(s, &[]);
    d.pass_all();
    match d.decision() {
        Decision::ChooseX { max, .. } => {
            assert_eq!(*max, 5);
            d.submit(Choice::Number(3));
        }
        o => panic!("{o:?}"),
    }
    assert_eq!(d.g().player(P2).life, 17);
}

#[test]
fn goaded_creature_must_attack() {
    let (db, mut g) = game_at(Step::Main1, 10);
    // P1 goadet den Bären von P2; der Bär muss in P2s Zug angreifen.
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 1);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let s = put(&db, &mut g, "Goad Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(s, &[Target::Card(bear)]);
    d.pass_all();
    assert_eq!(d.g().card(bear).goaded_by, vec![(P1, false)]);
    d.run_until(|g, dec| g.active == P2 && matches!(dec, Decision::DeclareAttackers { .. }));
    assert!(d.g().must_attack(P2).contains(&bear));
    assert!(
        d.try_submit(Choice::Attackers(vec![])).is_err(),
        "muss angreifen"
    );
    d.submit(Choice::Attackers(vec![(bear, Target::Player(P1))]));
    // im nächsten Zug von P1 endet Goad
    d.run_until(|g, _| g.active == P1 && g.step == Step::Upkeep);
    assert!(d.g().card(bear).goaded_by.is_empty());
}

#[test]
fn goaded_prefers_other_player_in_multiplayer() {
    let db = db();
    let island = db.get("Island").unwrap().clone();
    let decks = (0..3)
        .map(|i| (format!("P{i}"), vec![island.clone(); 10]))
        .collect();
    let mut g =
        forge_engine::GameState::new(forge_engine::GameRules::default(), decks, 7, Some(P1));
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    g.card_mut(bear).goaded_by.push((P1, false));
    let p3 = forge_engine::PlayerId(2);
    let defs = vec![Target::Player(P1), Target::Player(p3)];
    assert_eq!(
        g.goad_allowed_targets(bear, &defs),
        vec![Target::Player(p3)]
    );
    assert_eq!(
        g.goad_allowed_targets(bear, &defs[..1]),
        vec![Target::Player(P1)]
    );
}
