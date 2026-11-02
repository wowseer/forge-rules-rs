//! Cycling/Typecycling (CR 702.29), Devoid (CR 702.114).

mod common;
use common::*;
use forge_engine::decision::{Action, Choice, Decision};
use forge_engine::{Step, Zone};

#[test]
fn cycling_from_hand_discards_and_draws() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 2);
    let c = put(&db, &mut g, "Cycler Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    match d.decision() {
        Decision::Priority { actions, .. } => assert!(actions.contains(&Action::Activate(c, 0))),
        o => panic!("{o:?}"),
    }
    d.act(Action::Activate(c, 0));
    d.finish_casting(&[]);
    assert_eq!(
        count(d.g(), "Cycler Lite", Zone::Graveyard),
        1,
        "Abwerfen ist Kosten"
    );
    d.pass_all();
    assert_eq!(d.g().player(P1).hand.len(), 1, "eine Karte gezogen");
}

#[test]
fn cycling_not_offered_from_battlefield_or_without_mana() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Island", P1, Zone::Battlefield);
    let h = put(&db, &mut g, "Cycler Lite", P1, Zone::Hand);
    let b = put(&db, &mut g, "Cycler Lite", P1, Zone::Battlefield);
    let d = Driver::new(g);
    match d.decision() {
        Decision::Priority { actions, .. } => {
            assert!(!actions.contains(&Action::Activate(h, 0)), "nur 1 Mana");
            assert!(
                !actions.contains(&Action::Activate(b, 0)),
                "nur aus der Hand"
            );
        }
        o => panic!("{o:?}"),
    }
}

#[test]
fn typecycling_searches_library_for_type() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Plains", P1, Zone::Battlefield);
    let plains = put(&db, &mut g, "Plains", P1, Zone::Library);
    let c = put(&db, &mut g, "Plains Cycler Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.act(Action::Activate(c, 0));
    d.finish_casting(&[]);
    d.pass_all();
    match d.decision() {
        Decision::ChooseCards { options, .. } => {
            assert_eq!(options, &vec![plains], "nur Ebenen");
            d.submit(Choice::Cards(vec![plains]));
        }
        o => panic!("{o:?}"),
    }
    assert_eq!(count(d.g(), "Plains", Zone::Hand), 1);
}

#[test]
fn devoid_is_colorless_in_all_zones() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let h = put(&db, &mut g, "Devoid Lite", P1, Zone::Hand);
    let b = put(&db, &mut g, "Devoid Lite", P1, Zone::Battlefield);
    assert!(g.chars(h).colors.is_colorless());
    assert!(g.chars(b).colors.is_colorless());
    assert_eq!(g.chars(b).mana_cost.as_ref().unwrap().mana_value(), 2);
}

#[test]
fn prowess_pumps_on_noncreature_spell() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Mountain", P1, Zone::Battlefield);
    let monk = put(&db, &mut g, "Prowess Monk Lite", P1, Zone::Battlefield);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(bolt, &[forge_engine::decision::Target::Player(P2)]);
    d.pass_all();
    d.pass_all();
    assert_eq!(d.g().chars(monk).power, Some(2));
    assert_eq!(d.g().chars(monk).toughness, Some(3));
}

#[test]
fn cycled_and_discarded_triggers() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Plains", P1, Zone::Battlefield);
    put(&db, &mut g, "Discard Punisher Lite", P1, Zone::Battlefield);
    let c = put(&db, &mut g, "Cycle Reward Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.act(Action::Activate(c, 0));
    d.finish_casting(&[]);
    // Cycled (aus der Hand, Rückblick) + Discarded (Verzauberung): P1 ordnet sie
    assert!(
        matches!(d.decision(), Decision::OrderTriggers { descriptions, .. } if descriptions.len() == 2),
        "{:?}",
        d.decision()
    );
    d.run_until(|g, _| g.stack.is_empty());
    assert_eq!(d.g().player(P1).life, 23);
    assert_eq!(d.g().player(P2).life, 19);
    assert_eq!(d.g().player(P1).hand.len(), 1);
}

#[test]
fn exile_from_graveyard_cost_activates_from_graveyard() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Mountain", P1, Zone::Battlefield);
    let s = put(&db, &mut g, "Grave Spark Lite", P1, Zone::Graveyard);
    let mut d = Driver::new(g);
    d.act(Action::Activate(s, 0));
    d.finish_casting(&[forge_engine::decision::Target::Player(P2)]);
    assert_eq!(count(d.g(), "Grave Spark Lite", Zone::Exile), 1);
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 18);
}

#[test]
fn activation_limit_per_turn_and_per_game() {
    // Java SpellAbilityRestriction: ActivationLimit$ (je Zug), GameActivationLimit$
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 4);
    let elf = put(&db, &mut g, "Once Pump Lite", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    for i in [0, 1] {
        assert!(d.g().legal_actions(P1).contains(&Action::Activate(elf, i)));
        d.act(Action::Activate(elf, i));
        d.finish_casting(&[]);
        d.pass_all();
        assert!(!d.g().legal_actions(P1).contains(&Action::Activate(elf, i)));
    }
    assert_eq!(d.g().chars(elf).power, Some(2));
    // Im nächsten eigenen Zug: Pump wieder erlaubt, Leben nicht mehr
    d.run_until(|g, dec| {
        g.turn == 4
            && g.step == Step::Main1
            && matches!(dec, forge_engine::decision::Decision::Priority { .. })
    });
    let acts = d.g().legal_actions(P1);
    assert!(acts.contains(&Action::Activate(elf, 0)));
    assert!(!acts.contains(&Action::Activate(elf, 1)));
}
