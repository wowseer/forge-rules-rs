//! Massen-Effekte (`AnimateAll`, `TapAll`, `UntapAll`, `SacrificeAll`),
//! Investigate (CR 701.36), Fog (CR 615), Proliferate (CR 701.27).

mod common;
use common::*;
use forge_engine::decision::{Choice, Decision, Target};
use forge_engine::{Step, Zone};

#[test]
fn sacrifice_all_hits_every_player_simultaneously() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 7);
    let dust = put(&db, &mut g, "All Is Dust", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let angel = put(&db, &mut g, "Serra Angel", P2, Zone::Battlefield);
    let myr = put(&db, &mut g, "Darksteel Myr", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(dust, &[]);
    d.pass_all();
    assert_eq!(d.g().card(bear).zone, None, "Bär geopfert");
    assert_eq!(d.g().card(angel).zone, None, "auch die Karten des Gegners");
    assert_eq!(
        d.g().card(myr).zone,
        Some(Zone::Battlefield),
        "farblos bleibt"
    );
    assert_eq!(
        count(d.g(), "Island", Zone::Battlefield),
        7,
        "Länder sind farblos"
    );
    assert_eq!(count(d.g(), "Serra Angel", Zone::Graveyard), 1);
}

#[test]
fn investigate_creates_clue_that_draws() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 3);
    let insp = put(&db, &mut g, "Thraben Inspector", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(insp, &[]);
    d.pass_all(); // Kreatur
    d.pass_all(); // ETB-Trigger
    let clue = find(d.g(), "Clue Token", Zone::Battlefield).expect("Clue");
    assert!(d.g().card(clue).token);
    let hand = d.g().player(P1).hand.len();
    d.act(forge_engine::decision::Action::Activate(clue, 0));
    d.finish_casting(&[]);
    d.pass_all();
    assert_eq!(d.g().player(P1).hand.len(), hand + 1);
    assert_eq!(count(d.g(), "Clue Token", Zone::Battlefield), 0);
}

#[test]
fn fog_prevents_combat_damage_only() {
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    put(&db, &mut g, "Plains", P2, Zone::Battlefield);
    put(&db, &mut g, "Mountain", P2, Zone::Battlefield);
    let fog = put(&db, &mut g, "Holy Day", P2, Zone::Hand);
    let bolt = put(&db, &mut g, "Shock", P2, Zone::Hand);
    let mut d = Driver::new(g);
    d.run_until(|_, dec| matches!(dec, Decision::DeclareAttackers { .. }));
    d.submit(Choice::Attackers(vec![(bear, T2)]));
    d.priority_in(Step::DeclareAttackers, P2);
    d.cast(fog, &[]);
    d.pass_all();
    d.priority_in(Step::DeclareAttackers, P2);
    d.cast(bolt, &[Target::Player(P1)]);
    d.pass_all();
    d.run_until(|g, _| g.step == Step::Main2);
    assert_eq!(d.g().player(P2).life, 20, "Kampfschaden verhindert");
    assert_eq!(d.g().player(P1).life, 18, "anderer Schaden nicht");
}

#[test]
fn proliferate_adds_one_of_each_kind_to_chosen() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Island", P1, Zone::Battlefield);
    let spell = put(&db, &mut g, "Proliferate Lite", P1, Zone::Hand);
    let a = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let b = put(&db, &mut g, "Hill Giant", P2, Zone::Battlefield);
    g.card_mut(a).counters.insert("P1P1".into(), 1);
    g.card_mut(a).counters.insert("LOYALTY".into(), 2);
    g.card_mut(b).counters.insert("P1P1".into(), 1);
    g.player_mut(P2).poison = 3;
    let mut d = Driver::new(g);
    d.cast(spell, &[]);
    d.pass_all();
    match d.decision() {
        Decision::ChooseCards {
            options, min: 0, ..
        } => assert_eq!(options.len(), 2),
        o => panic!("{o:?}"),
    }
    d.submit(Choice::Cards(vec![a]));
    assert!(matches!(d.decision(), Decision::Confirm { .. }));
    d.submit(Choice::Yes);
    assert_eq!(d.g().card(a).counter("P1P1"), 2);
    assert_eq!(d.g().card(a).counter("LOYALTY"), 3);
    assert_eq!(d.g().card(b).counter("P1P1"), 1, "nicht gewählt");
    assert_eq!(d.g().player(P2).poison, 4);
}

#[test]
fn animate_all_tap_all_untap_all() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let f1 = put(&db, &mut g, "Forest", P1, Zone::Battlefield);
    let f2 = put(&db, &mut g, "Forest", P1, Zone::Battlefield);
    let isl = put(&db, &mut g, "Island", P1, Zone::Battlefield);
    let opp_land = put(&db, &mut g, "Forest", P2, Zone::Battlefield);
    let anim = put(&db, &mut g, "Mass Animate Lite", P1, Zone::Hand);
    let tap = put(&db, &mut g, "Mass Tap Lite", P1, Zone::Hand);
    let mine = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let theirs = put(&db, &mut g, "Hill Giant", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(anim, &[]);
    d.pass_all();
    for l in [f1, f2, isl] {
        assert!(d.g().chars(l).types.is_creature());
        assert_eq!(d.g().chars(l).power, Some(3));
    }
    assert!(!d.g().chars(opp_land).types.is_creature());
    d.cast(tap, &[Target::Player(P2)]);
    d.pass_all();
    assert!(d.g().card(theirs).tapped, "Kreaturen des Ziels getappt");
    assert!(!d.g().card(mine).tapped, "eigene nicht");
    assert!(
        [f1, f2, isl].iter().all(|l| !d.g().card(*l).tapped),
        "eigene Länder wieder ungetappt"
    );
    d.run_until(|g, _| g.step == Step::End);
    d.run_until(|g, _| g.turn == 3);
    assert!(!d.g().chars(f1).types.is_creature(), "bis Zugende");
}
