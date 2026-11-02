//! Commander (CR 903): Befehlszone, Steuer, Rückkehr, 21 Commander-Schaden.

mod common;
use common::*;
use forge_engine::decision::{Action, Choice, Decision};
use forge_engine::state::LossReason;
use forge_engine::{Step, Zone};

fn commander_game(
    forests: usize,
) -> (
    forge_engine::CardDb,
    forge_engine::GameState,
    forge_engine::CardId,
) {
    let (db, mut g) = game_at(Step::Main1, 10);
    g.rules.commander = true;
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, forests);
    let def = db.get("Grizzly Bears").unwrap().clone();
    let cmd = g.add_commander(def, P1);
    (db, g, cmd)
}

#[test]
fn cast_from_command_zone_with_tax_and_return() {
    let (_db, g, cmd) = commander_game(6);
    let spec = forge_engine::valid::ValidSpec::parse("Creature.IsCommander").unwrap();
    assert!(g.card_matches(cmd, &spec, cmd, P1));
    let mut d = Driver::new(g);
    d.cast(cmd, &[]);
    d.pass_all();
    let bear = find(d.g(), "Grizzly Bears", Zone::Battlefield).unwrap();
    // stirbt → Besitzer darf ihn in die Befehlszone legen (CR 903.9a)
    let mut g = d.g().clone();
    g.move_card(bear, Zone::Graveyard, true);
    let mut d = Driver::new(g);
    match d.decision() {
        Decision::Confirm { player, .. } => {
            assert_eq!(*player, P1);
            d.submit(Choice::Yes);
        }
        o => panic!("{o:?}"),
    }
    let back = d
        .g()
        .command
        .iter()
        .copied()
        .find(|c| d.g().card(*c).commander.is_some())
        .unwrap();
    // zweites Wirken kostet {2} mehr: 4 verbleibende Wälder reichen genau
    let untapped = d
        .g()
        .battlefield
        .iter()
        .filter(|c| !d.g().card(**c).tapped)
        .count();
    assert_eq!(untapped, 4);
    assert!(d.g().legal_actions(P1).contains(&Action::Cast(back)));
    d.cast(back, &[]);
    d.pass_all();
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Battlefield), 1);
    assert!(
        d.g()
            .battlefield
            .iter()
            .all(|c| d.g().card(*c).tapped || d.g().card(*c).def.name() == "Grizzly Bears")
    );
}

#[test]
fn tax_makes_commander_uncastable_without_enough_mana() {
    let (_db, mut g, cmd) = commander_game(3);
    g.card_mut(cmd).commander_casts = 1; // einmal gewirkt → {1}{G} + {2}
    assert!(!g.legal_actions(P1).contains(&Action::Cast(cmd)));
}

#[test]
fn declining_return_keeps_commander_in_graveyard() {
    let (_db, mut g, cmd) = commander_game(0);
    let gy = g.move_card(cmd, Zone::Graveyard, true);
    let mut d = Driver::new(g);
    d.submit(Choice::No);
    assert_eq!(d.g().card(gy).zone, Some(Zone::Graveyard));
    assert!(
        matches!(d.decision(), Decision::Priority { .. }),
        "nicht erneut gefragt"
    );
}

#[test]
fn twenty_one_commander_damage_loses() {
    let (_db, mut g, cmd) = commander_game(0);
    let bear = g.move_card(cmd, Zone::Battlefield, true);
    g.card_mut(bear).controlled_since_turn_start = true;
    g.player_mut(P2).commander_damage.insert(0, 20);
    let mut d = Driver::new(g);
    d.run_until(|_, dec| matches!(dec, Decision::DeclareAttackers { .. }));
    d.submit(Choice::Attackers(vec![(bear, T2)]));
    d.run_until(|g, _| g.player(P2).lost.is_some());
    assert_eq!(d.g().player(P2).lost, Some(LossReason::CommanderDamage));
    assert!(d.g().player(P2).life > 0);
}
