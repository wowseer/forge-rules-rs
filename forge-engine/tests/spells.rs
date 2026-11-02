//! Zauber, Fähigkeiten, Kosten, Effekte, Ziele.

mod common;
use common::*;
use forge_engine::decision::{Action, Choice, Decision, PayAction, Target};
use forge_engine::{Step, Zone};

#[test]
fn land_play_once_per_turn_sorcery_timing() {
    // CR 305.2
    let (db, mut g) = game_at(Step::Main1, 10);
    let f1 = put(&db, &mut g, "Forest", P1, Zone::Hand);
    let f2 = put(&db, &mut g, "Forest", P1, Zone::Hand);
    let mut d = Driver::new(g);
    match d.decision() {
        Decision::Priority { actions, .. } => assert!(actions.contains(&Action::PlayLand(f1))),
        o => panic!("{o:?}"),
    }
    d.act(Action::PlayLand(f1));
    match d.decision() {
        Decision::Priority { actions, .. } => assert!(!actions.contains(&Action::PlayLand(f2))),
        o => panic!("{o:?}"),
    }
}

#[test]
fn sorcery_only_main_phase_empty_stack_instant_any_time() {
    let (db, mut g) = game_at(Step::Upkeep, 10);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 3);
    put(&db, &mut g, "Mountain", P1, Zone::Battlefield);
    let div = put(&db, &mut g, "Divination", P1, Zone::Hand);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let d = Driver::new(g);
    match d.decision() {
        Decision::Priority { actions, .. } => {
            assert!(!actions.contains(&Action::Cast(div)));
            assert!(actions.contains(&Action::Cast(bolt)));
        }
        o => panic!("{o:?}"),
    }
}

#[test]
fn bolt_face_and_creature_and_fizzle() {
    // CR 608.2b: alle Ziele illegal → Zauber tut nichts
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 2);
    let b1 = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let b2 = put(&db, &mut g, "Shock", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(b1, &[Target::Card(bear)]);
    d.act(Action::Pass);
    // P2 passt; danach in Antwort? Wir lassen Shock als Antwort von P1 nach Bolt auflösen:
    d.act(Action::Pass);
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Graveyard), 1);
    d.cast(b2, &[Target::Player(P2)]);
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 18);
    assert_eq!(count(d.g(), "Lightning Bolt", Zone::Graveyard), 1);
    assert_eq!(count(d.g(), "Shock", Zone::Graveyard), 1);
}

#[test]
fn spell_fizzles_when_target_gone() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 2);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let shock = put(&db, &mut g, "Shock", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(bolt, &[Target::Card(bear)]);
    d.cast(shock, &[Target::Card(bear)]); // in Antwort auf eigenen Bolt
    d.pass_all(); // Shock tötet den Bären
    d.pass_all(); // Bolt: Ziel weg → fizzelt
    assert!(
        d.g()
            .log
            .iter()
            .any(|e| matches!(e, forge_engine::Event::Fizzled { .. }))
    );
    assert_eq!(d.g().player(P2).life, 20);
    assert_eq!(count(d.g(), "Lightning Bolt", Zone::Graveyard), 1);
}

#[test]
fn creature_spell_resolves_onto_battlefield_with_summoning_sickness() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 2);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(bear, &[]);
    assert_eq!(count(d.g(), "Forest", Zone::Battlefield), 2);
    assert!(d.g().battlefield.iter().all(|c| d.g().card(*c).tapped));
    d.pass_all();
    let b = find(d.g(), "Grizzly Bears", Zone::Battlefield).expect("auf dem Battlefield");
    assert_eq!(d.g().card(b).controller, P1);
    assert!(d.g().is_summoning_sick(b));
}

#[test]
fn manual_mana_payment_and_cancel() {
    // CR 601.2g, 728 (Rücknahme)
    let (db, mut g) = game_at(Step::Main1, 10);
    let f = put(&db, &mut g, "Forest", P1, Zone::Battlefield);
    let elf = put(&db, &mut g, "Llanowar Elves", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.act(Action::Cast(bear));
    d.submit(Choice::Pay(PayAction::Cancel));
    assert!(matches!(d.decision(), Decision::Priority { .. }));
    assert_eq!(d.g().player(P1).hand.len(), 1, "Abbruch nimmt alles zurück");
    assert!(!d.g().card(f).tapped);

    d.act(Action::Cast(bear));
    d.submit(Choice::Pay(PayAction::ActivateMana(f, 0)));
    d.submit(Choice::Pay(PayAction::ActivateMana(elf, 0)));
    assert_eq!(d.g().stack.len(), 1);
    assert!(d.g().card(f).tapped && d.g().card(elf).tapped);
    assert!(d.g().player(P1).mana_pool.is_empty());
}

#[test]
fn mana_pool_empties_between_steps() {
    // CR 500.4
    let (db, mut g) = game_at(Step::Main1, 10);
    let f = put(&db, &mut g, "Forest", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.act(Action::Activate(f, 0));
    assert_eq!(d.g().player(P1).mana_pool.len(), 1);
    d.priority_in(Step::BeginCombat, P1);
    assert!(d.g().player(P1).mana_pool.is_empty());
}

#[test]
fn x_spell() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 5);
    let fb = put(&db, &mut g, "Fireball Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.act(Action::Cast(fb));
    d.submit(Choice::Number(4));
    d.finish_casting(&[Target::Player(P2)]);
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 16);
}

#[test]
fn phyrexian_mana_with_life() {
    // CR 107.4f, 119.4
    let (db, mut g) = game_at(Step::Main1, 10);
    let probe = put(&db, &mut g, "Gitaxian Probe Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.act(Action::Cast(probe));
    d.submit(Choice::Yes);
    d.pass_all();
    assert_eq!(d.g().player(P1).life, 18);
    assert_eq!(d.g().player(P1).hand.len(), 1);
}

#[test]
fn hexproof_only_blocks_opponents() {
    // CR 702.11b
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 1);
    put(&db, &mut g, "Forest", P2, Zone::Battlefield);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let scout = put(&db, &mut g, "Gladecover Scout", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.act(Action::Cast(bolt));
    match d.decision() {
        Decision::ChooseTargets { candidates, .. } => {
            assert!(!candidates.contains(&Target::Card(scout)))
        }
        o => panic!("{o:?}"),
    }
}

#[test]
fn destroy_respects_indestructible() {
    // CR 702.12b
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Swamp", P1, Zone::Battlefield, 3);
    put_n(&db, &mut g, "Swamp", P1, Zone::Battlefield, 3);
    let m1 = put(&db, &mut g, "Murder", P1, Zone::Hand);
    let m2 = put(&db, &mut g, "Murder", P1, Zone::Hand);
    let myr = put(&db, &mut g, "Darksteel Myr", P2, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(m1, &[Target::Card(myr)]);
    d.pass_all();
    d.cast(m2, &[Target::Card(bear)]);
    d.pass_all();
    assert_eq!(d.g().card(myr).zone, Some(Zone::Battlefield));
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Graveyard), 1);
}

#[test]
fn activated_ability_with_tap_cost_uses_stack_and_sickness() {
    // CR 602.2, 302.6
    let (db, mut g) = game_at(Step::Main1, 10);
    let pyro = put(&db, &mut g, "Prodigal Pyromancer", P1, Zone::Battlefield);
    let fresh = put(&db, &mut g, "Prodigal Pyromancer", P1, Zone::Battlefield);
    g.card_mut(fresh).controlled_since_turn_start = false;
    let mut d = Driver::new(g);
    match d.decision() {
        Decision::Priority { actions, .. } => {
            assert!(actions.contains(&Action::Activate(pyro, 0)));
            assert!(!actions.contains(&Action::Activate(fresh, 0)));
        }
        o => panic!("{o:?}"),
    }
    d.act(Action::Activate(pyro, 0));
    d.submit(Choice::Targets(vec![Target::Player(P2)]));
    assert_eq!(d.g().stack.len(), 1);
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 19);
}

#[test]
fn draw_gain_lose_life_and_pump() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 3);
    put_n(&db, &mut g, "Swamp", P1, Zone::Battlefield, 3);
    put(&db, &mut g, "Plains", P1, Zone::Battlefield);
    put(&db, &mut g, "Forest", P1, Zone::Battlefield);
    let div = put(&db, &mut g, "Divination", P1, Zone::Hand);
    let rot = put(&db, &mut g, "Mind Rot Lite", P1, Zone::Hand);
    let salve = put(&db, &mut g, "Healing Salve", P1, Zone::Hand);
    let growth = put(&db, &mut g, "Giant Growth", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(div, &[]);
    d.pass_all();
    assert_eq!(d.g().player(P1).hand.len(), 5);
    d.cast(rot, &[Target::Player(P2)]);
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 18);
    d.cast(salve, &[Target::Player(P1)]);
    d.pass_all();
    assert_eq!(d.g().player(P1).life, 23);
    d.cast(growth, &[Target::Card(bear)]);
    d.pass_all();
    assert_eq!(d.g().chars(bear).power, Some(5));
    d.run_until(|g, _| g.turn == 3);
    assert_eq!(d.g().chars(bear).power, Some(2), "bis Ende des Zuges");
}

#[test]
fn real_card_files_are_supported() {
    let res = forge_script::corpus::default_res_dir();
    if !res.exists() {
        return;
    }
    let db = forge_engine::CardDb::load(&res).unwrap();
    for name in [
        "Lightning Bolt",
        "Grizzly Bears",
        "Serra Angel",
        "Llanowar Elves",
        "Murder",
        "Giant Growth",
        "Divination",
        "Forest",
    ] {
        let c = db.get(name).unwrap_or_else(|| panic!("{name} fehlt"));
        assert!(c.is_supported(), "{name}: {:?}", c.unsupported);
    }
}

#[test]
fn flash_allows_instant_speed_casting() {
    // CR 702.8
    let (db, mut g) = game_at(Step::Upkeep, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 2);
    let viper = put(&db, &mut g, "Ambush Viper", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Hand);
    let mut d = Driver::new(g);
    match d.decision() {
        Decision::Priority { actions, .. } => {
            assert!(actions.contains(&Action::Cast(viper)));
            assert!(!actions.contains(&Action::Cast(bear)));
        }
        o => panic!("{o:?}"),
    }
    d.cast(viper, &[]);
    d.pass_all();
    assert!(find(d.g(), "Ambush Viper", Zone::Battlefield).is_some());
}

#[test]
fn shroud_blocks_all_targeting() {
    // CR 702.18
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Forest", P1, Zone::Battlefield);
    let growth = put(&db, &mut g, "Giant Growth", P1, Zone::Hand);
    let own = put(&db, &mut g, "Silhana Lite", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.act(Action::Cast(growth));
    match d.decision() {
        Decision::ChooseTargets { candidates, .. } => {
            assert!(
                !candidates.contains(&Target::Card(own)),
                "auch nicht durch den eigenen Controller"
            );
            assert!(candidates.contains(&Target::Card(bear)));
        }
        o => panic!("{o:?}"),
    }
}

#[test]
fn sacrifice_self_cost() {
    // CR 701.21, Kosten werden vor dem Auflösen bezahlt
    let (db, mut g) = game_at(Step::Main1, 10);
    let fan = put(&db, &mut g, "Mogg Fanatic Lite", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.act(Action::Activate(fan, 0));
    d.submit(Choice::Targets(vec![Target::Player(P2)]));
    assert_eq!(
        count(d.g(), "Mogg Fanatic Lite", Zone::Graveyard),
        1,
        "geopfert beim Aktivieren"
    );
    d.pass_all();
    assert_eq!(
        d.g().player(P2).life,
        19,
        "Fähigkeit löst unabhängig von der Quelle auf"
    );
}

#[test]
fn untap_symbol_cost() {
    // CR 107.6
    let (db, mut g) = game_at(Step::Main1, 10);
    let monk = put(&db, &mut g, "Untap Monk Lite", P1, Zone::Battlefield);
    let mut d = Driver::new(g.clone());
    match d.decision() {
        Decision::Priority { actions, .. } => assert!(
            !actions.contains(&Action::Activate(monk, 0)),
            "nur wenn getappt"
        ),
        o => panic!("{o:?}"),
    }
    g.card_mut(monk).tapped = true;
    d = Driver::new(g);
    d.act(Action::Activate(monk, 0));
    assert!(!d.g().card(monk).tapped);
    d.pass_all();
    assert_eq!(d.g().player(P1).life, 22);
}

#[test]
fn may_play_exiled_card_this_turn_as_land_drop() {
    // Java StaticAbilityContinuous MayPlay$: Effektobjekt erlaubt die exilierte Karte
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Mountain", P1, Zone::Battlefield);
    let imp = put(&db, &mut g, "Impulse Exile Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(imp, &[]);
    d.pass_all();
    let ex = find(d.g(), "Island", Zone::Exile).expect("Insel exiliert");
    assert!(d.g().legal_actions(P1).contains(&Action::PlayLand(ex)));
    d.act(Action::PlayLand(ex));
    assert_eq!(count(d.g(), "Island", Zone::Battlefield), 1);
    assert_eq!(d.g().player(P1).lands_played_this_turn, 1);
}

#[test]
fn may_play_without_mana_cost_with_limit() {
    // MayPlayWithoutManaCost$ + MayPlayLimit$ 1 (je Zug)
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Free Grave Lite", P1, Zone::Battlefield);
    let b1 = put(&db, &mut g, "Lightning Bolt", P1, Zone::Graveyard);
    let b2 = put(&db, &mut g, "Lightning Bolt", P1, Zone::Graveyard);
    let mut d = Driver::new(g);
    assert!(d.g().legal_actions(P1).contains(&Action::Cast(b1)));
    d.cast(b1, &[T2]);
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 17, "ohne Mana gewirkt");
    assert!(
        !d.g().legal_actions(P1).contains(&Action::Cast(b2)),
        "Limit 1"
    );
}

#[test]
fn alternative_cost_instead_of_mana_cost() {
    // CR 118.9, Java StaticAbility AlternativeCost (ValidSA$ Spell.Self)
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Mountain", P1, Zone::Battlefield);
    let force = put(&db, &mut g, "Force Lite", P1, Zone::Hand);
    let leak = put(&db, &mut g, "Mana Leak", P1, Zone::Hand);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    assert!(
        !g.legal_actions(P1).contains(&Action::Cast(force)),
        "kein Mana"
    );
    let mut d = Driver::new(g);
    d.cast(bolt, &[T2]);
    let acts = d.g().legal_actions(P1);
    assert!(acts.contains(&Action::CastWithAltCost(force, 0)));
    d.act(Action::CastWithAltCost(force, 0));
    let spell = d.g().stack[0].source;
    loop {
        match d.decision().clone() {
            Decision::ChooseTargets { .. } => d.submit(Choice::Targets(vec![Target::Card(spell)])),
            Decision::ChooseCards { options, .. } => {
                assert_eq!(options, vec![leak], "nur andere blaue Karten");
                d.submit(Choice::Cards(vec![leak]));
            }
            _ => break,
        }
    }
    assert_eq!(d.g().player(P1).life, 19);
    assert_eq!(count(d.g(), "Mana Leak", Zone::Exile), 1);
    d.pass_all();
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 20, "Blitz neutralisiert");
}

#[test]
fn mana_ability_with_variable_amount() {
    // Java ManaEffect Amount$ X; grobe Obergrenze in legal_actions berücksichtigt die Menge
    let (db, mut g) = game_at(Step::Main1, 10);
    let cradle = put(&db, &mut g, "Cradle Lite", P1, Zone::Battlefield);
    put_n(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield, 3);
    let div = put(&db, &mut g, "Giant Growth", P1, Zone::Hand);
    let mut d = Driver::new(g);
    assert!(d.g().legal_actions(P1).contains(&Action::Cast(div)));
    d.act(Action::Activate(cradle, 0));
    assert_eq!(d.g().player(P1).mana_pool.len(), 3);
}
