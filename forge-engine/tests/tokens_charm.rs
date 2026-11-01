//! Token (CR 111) und modale Zauber/Fähigkeiten (Charm, CR 700.2).

mod common;
use common::*;
use forge_engine::decision::{Action, Choice, Decision, PayAction, Target};
use forge_engine::{Step, Zone};

#[test]
fn tokens_enter_and_trigger_etb() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 2);
    put(&db, &mut g, "Soul Warden Lite", P1, Zone::Battlefield);
    let rta = put(&db, &mut g, "Raise the Alarm", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(rta, &[]);
    d.pass_all();
    let soldiers: Vec<_> = d
        .g()
        .battlefield
        .iter()
        .copied()
        .filter(|c| d.g().card(*c).def.name() == "Soldier Token")
        .collect();
    assert_eq!(soldiers.len(), 2);
    assert!(
        soldiers
            .iter()
            .all(|s| d.g().card(*s).token && d.g().card(*s).owner == P1)
    );
    assert!(d.g().is_summoning_sick(soldiers[0]));
    // zwei gleichzeitige eigene Trigger → Reihenfolge wählen (CR 603.3b)
    assert!(
        matches!(d.decision(), Decision::OrderTriggers { descriptions, .. } if descriptions.len() == 2)
    );
    d.submit(Choice::Order(vec![0, 1]));
    assert_eq!(d.g().stack.len(), 2, "zwei ETB-Trigger");
    d.pass_all();
    d.pass_all();
    assert_eq!(d.g().player(P1).life, 22);
}

#[test]
fn token_leaving_battlefield_ceases_to_exist() {
    // CR 111.7 / 704.5d
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 2);
    put(&db, &mut g, "Mountain", P1, Zone::Battlefield);
    let rta = put(&db, &mut g, "Raise the Alarm", P1, Zone::Hand);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(rta, &[]);
    d.pass_all();
    let soldier = d
        .g()
        .battlefield
        .iter()
        .copied()
        .find(|c| d.g().card(*c).token)
        .unwrap();
    d.cast(bolt, &[Target::Card(soldier)]);
    d.pass_all();
    assert_eq!(count(d.g(), "Soldier Token", Zone::Graveyard), 0);
    assert_eq!(count(d.g(), "Soldier Token", Zone::Battlefield), 1);
}

#[test]
fn token_owner_targeted_controller_and_tapped() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 3);
    let bw = put(&db, &mut g, "Beast Within", P1, Zone::Hand);
    let wurm = put(&db, &mut g, "Craw Wurm", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(bw, &[Target::Card(wurm)]);
    d.pass_all();
    let beast = d
        .g()
        .battlefield
        .iter()
        .copied()
        .find(|c| d.g().card(*c).def.name() == "Beast Token")
        .unwrap();
    assert_eq!(d.g().card(beast).controller, P2);
    assert!(d.g().card(beast).tapped);
    assert_eq!(count(d.g(), "Craw Wurm", Zone::Graveyard), 1);
}

#[test]
fn token_with_x_power_toughness() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 4);
    let s = put(&db, &mut g, "Hydra Birth Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.act(Action::Cast(s));
    d.submit(Choice::Number(3));
    d.finish_casting(&[]);
    d.pass_all();
    let hydra = d
        .g()
        .battlefield
        .iter()
        .copied()
        .find(|c| d.g().card(*c).def.name() == "Hydra Token")
        .unwrap();
    assert_eq!(
        (d.g().chars(hydra).power, d.g().chars(hydra).toughness),
        (Some(3), Some(3))
    );
}

#[test]
fn treasure_token_sacrifices_for_mana() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Island", P1, Zone::Battlefield);
    let s = put(&db, &mut g, "Treasure Map Lite", P1, Zone::Hand);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(s, &[]);
    d.pass_all();
    let treasure = d
        .g()
        .battlefield
        .iter()
        .copied()
        .find(|c| d.g().card(*c).def.name() == "Treasure Token")
        .unwrap();
    d.act(Action::Cast(bolt));
    d.submit(Choice::Targets(vec![Target::Player(P2)]));
    d.submit(Choice::Pay(PayAction::ActivateMana(treasure, 0)));
    d.submit(Choice::Color(forge_types::Color::Red));
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 17);
    assert_eq!(count(d.g(), "Treasure Token", Zone::Battlefield), 0);
}

#[test]
fn charm_choose_two_resolve_in_printed_order() {
    // CR 700.2: gewählte Modi in Druckreihenfolge
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Island", P1, Zone::Battlefield);
    let charm = put(&db, &mut g, "Triple Charm Lite", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.act(Action::Cast(charm));
    match d.decision() {
        Decision::ChooseModes {
            modes, min, max, ..
        } => {
            assert_eq!(modes.len(), 3);
            assert_eq!((*min, *max), (2, 2));
        }
        o => panic!("{o:?}"),
    }
    assert!(d.try_submit(Choice::Modes(vec![2])).is_err(), "genau zwei");
    assert!(
        d.try_submit(Choice::Modes(vec![2, 2])).is_err(),
        "keine Wiederholung"
    );
    d.submit(Choice::Modes(vec![2, 0]));
    d.finish_casting(&[Target::Card(bear)]);
    d.pass_all();
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Hand), 1);
    assert_eq!(d.g().player(P1).life, 23);
    assert_eq!(d.g().player(P1).hand.len(), 0, "Draw-Modus nicht gewählt");
}

#[test]
fn charm_modes_without_targets_not_offered() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Mountain", P1, Zone::Battlefield);
    let charm = put(&db, &mut g, "Target Only Charm Lite", P1, Zone::Hand);
    let d = Driver::new(g.clone());
    match d.decision() {
        Decision::Priority { actions, .. } => assert!(
            !actions.contains(&Action::Cast(charm)),
            "kein Modus wählbar"
        ),
        o => panic!("{o:?}"),
    }
    put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.act(Action::Cast(charm));
    match d.decision() {
        Decision::ChooseModes { modes, .. } => {
            assert_eq!(modes.iter().map(|m| m.0).collect::<Vec<_>>(), vec![0])
        }
        o => panic!("{o:?}"),
    }
}

#[test]
fn modal_trigger_chooses_mode_when_put_on_stack() {
    // CR 603.3c
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 2);
    let elf = put(&db, &mut g, "Modal Trigger Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(elf, &[]);
    d.pass_all();
    assert!(matches!(d.decision(), Decision::ChooseModes { .. }));
    d.submit(Choice::Modes(vec![1]));
    d.pass_all();
    assert_eq!(d.g().player(P1).life, 22);
}
