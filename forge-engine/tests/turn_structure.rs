//! Zugstruktur, Priorität/Stapel, Mulligan (Forge_Turn_Structure.xlsx).

mod common;
use common::*;
use forge_engine::decision::{Action, Choice, Decision, Target};
use forge_engine::{Event, GameRules, GameState, Step, Zone};

#[test]
fn untap_step_untaps_active_players_permanents() {
    // CR 502.3
    let (db, mut g) = game_at(Step::Cleanup, 10);
    g.active = P2; // Zug von P2 endet, P1 beginnt
    let a = put(&db, &mut g, "Forest", P1, Zone::Battlefield);
    let b = put(&db, &mut g, "Forest", P2, Zone::Battlefield);
    g.card_mut(a).tapped = true;
    g.card_mut(b).tapped = true;
    let mut d = Driver::new(g);
    d.priority_in(Step::Upkeep, P1);
    assert!(!d.g().card(a).tapped);
    assert!(d.g().card(b).tapped, "nur der aktive Spieler enttappt");
    assert_eq!(d.g().turn, 3);
}

#[test]
fn draw_step_draws_and_first_player_skips_first_draw() {
    // CR 504.1, CR 103.8a
    let (_db, mut g) = game_at(Step::Upkeep, 10);
    let mut d = Driver::new(g.clone());
    d.priority_in(Step::Draw, P1);
    assert_eq!(d.g().player(P1).hand.len(), 1);

    g.turn = 1;
    g.starting_player = P1;
    let mut d = Driver::new(g);
    d.priority_in(Step::Draw, P1);
    assert_eq!(d.g().player(P1).hand.len(), 0);
}

#[test]
fn steps_in_order_and_combat_skips_without_attackers() {
    // CR 500.1, 508.8
    let (_db, g) = game_at(Step::Untap, 10);
    let mut d = Driver::new(g);
    d.priority_in(Step::Main2, P1);
    let steps: Vec<Step> = d
        .g()
        .log
        .iter()
        .filter_map(|e| {
            if let Event::StepBegan(s) = e {
                Some(*s)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        steps,
        vec![
            Step::Untap,
            Step::Upkeep,
            Step::Draw,
            Step::Main1,
            Step::BeginCombat,
            Step::DeclareAttackers,
            Step::EndCombat,
            Step::Main2
        ]
    );
}

#[test]
fn no_priority_in_skipped_combat_steps() {
    let (_db, g) = game_at(Step::BeginCombat, 10);
    let mut d = Driver::new(g);
    let mut steps_with_priority = Vec::new();
    d.run_until(|g, dec| {
        if let Decision::Priority { .. } = dec {
            steps_with_priority.push(g.step);
        }
        g.step == Step::Main2
    });
    assert!(!steps_with_priority.contains(&Step::DeclareBlockers));
    assert!(!steps_with_priority.contains(&Step::CombatDamage));
}

#[test]
fn cleanup_discards_to_hand_size_removes_damage_ends_effects() {
    // CR 514.1, 514.2
    let (db, mut g) = game_at(Step::End, 10);
    put_n(&db, &mut g, "Island", P1, Zone::Hand, 9);
    let bear = put(&db, &mut g, "Craw Wurm", P1, Zone::Battlefield);
    g.card_mut(bear).damage = 2;
    g.effects.push(forge_engine::state::ContinuousEffect {
        affected: bear,
        power: 3,
        toughness: 3,
        keywords: vec![],
        duration: forge_engine::state::Duration::UntilEndOfTurn,
        timestamp: 99,
    });
    let mut d = Driver::new(g);
    d.run_until(|_, dec| matches!(dec, Decision::ChooseCards { .. }));
    match d.decision().clone() {
        Decision::ChooseCards { min, max, .. } => assert_eq!((min, max), (2, 2)),
        _ => unreachable!(),
    }
    d.run_until(|g, _| g.turn == 3);
    assert_eq!(d.g().player(P1).hand.len(), 7);
    assert_eq!(d.g().card(bear).damage, 0);
    assert!(d.g().effects.is_empty());
}

#[test]
fn stack_resolves_last_in_first_out() {
    // CR 405.5, 608.1
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 2);
    put_n(&db, &mut g, "Forest", P2, Zone::Battlefield, 1);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let growth = put(&db, &mut g, "Giant Growth", P2, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(bolt, &[Target::Card(bear)]);
    // P1 behält Priorität (CR 117.3c) und passt
    d.act(Action::Pass);
    // P2 antwortet
    d.cast(growth, &[Target::Card(bear)]);
    assert_eq!(d.g().stack.len(), 2);
    d.pass_all(); // Giant Growth löst auf
    assert_eq!(d.g().stack.len(), 1);
    d.pass_all(); // Bolt löst auf: 3 Schaden an 5/5
    assert!(d.g().stack.is_empty());
    assert_eq!(d.g().card(bear).zone, Some(Zone::Battlefield));
    assert_eq!(d.g().card(bear).damage, 3);
}

#[test]
fn both_players_must_pass_in_succession() {
    // CR 117.4
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Mountain", P1, Zone::Battlefield);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(bolt, &[Target::Player(P2)]);
    d.act(Action::Pass);
    assert!(matches!(d.decision(), Decision::Priority { player, .. } if *player == P2));
    assert_eq!(d.g().stack.len(), 1);
    d.act(Action::Pass);
    assert!(d.g().stack.is_empty());
    assert_eq!(d.g().player(P2).life, 17);
    // nach Auflösung erhält der aktive Spieler Priorität (CR 117.3b)
    assert!(matches!(d.decision(), Decision::Priority { player, .. } if *player == P1));
    assert_eq!(d.g().step, Step::Main1);
}

#[test]
fn london_mulligan() {
    // CR 103.5
    let db = db();
    let island = db.get("Island").unwrap().clone();
    let decks = vec![
        ("A".to_string(), vec![island.clone(); 30]),
        ("B".to_string(), vec![island; 30]),
    ];
    let g = GameState::new(GameRules::default(), decks, 1, Some(P1));
    let mut d = Driver::new(g);
    assert!(
        matches!(d.decision(), Decision::Mulligan { player, mulligans_taken: 0 } if *player == P1)
    );
    d.submit(Choice::Mulligan);
    assert!(matches!(
        d.decision(),
        Decision::Mulligan {
            mulligans_taken: 1,
            ..
        }
    ));
    assert_eq!(d.g().player(P1).hand.len(), 7);
    d.submit(Choice::Keep);
    let hand = match d.decision().clone() {
        Decision::MulliganBottom { hand, count: 1, .. } => hand,
        other => panic!("{other:?}"),
    };
    d.submit(Choice::Cards(vec![hand[0]]));
    assert_eq!(d.g().player(P1).hand.len(), 6);
    assert_eq!(d.g().player(P1).library.len(), 24);
    assert_eq!(
        d.g().player(P1).library[0],
        d.g().card(hand[0]).moved_to.unwrap(),
        "unter die Bibliothek"
    );
    d.submit(Choice::Keep);
    assert_eq!(d.g().turn, 1);
}

#[test]
fn deterministic_with_same_seed() {
    let db = db();
    let deck: Vec<_> = [
        "Mountain",
        "Lightning Bolt",
        "Grizzly Bears",
        "Forest",
        "Hill Giant",
    ]
    .iter()
    .flat_map(|n| std::iter::repeat_n(db.get(n).unwrap().clone(), 8))
    .collect();
    let run = |seed: u64| {
        let g = GameState::new(
            GameRules::default(),
            vec![("A".into(), deck.clone()), ("B".into(), deck.clone())],
            seed,
            None,
        );
        let mut e = forge_engine::Engine::new(g);
        let mut a = forge_engine::simple_ai::SimpleAi::new(seed);
        let mut b = forge_engine::simple_ai::SimpleAi::new(seed + 1);
        let w = forge_engine::run_game(&mut e, &mut [&mut a, &mut b], 200_000);
        (w, e.state().log.clone())
    };
    let (w1, l1) = run(5);
    let (w2, l2) = run(5);
    assert!(w1.is_some(), "Partie endet");
    assert_eq!(w1, w2);
    assert_eq!(l1, l2);
}
