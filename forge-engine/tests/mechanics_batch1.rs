//! Undying, Persist, Bushido, Flanking, Untaps-/Always-Trigger, Zusatzzug,
//! TapOrUntap, Discard<1/Random>, Mill RememberMilled, CanAttackDefender.

mod common;
use common::*;
use forge_engine::decision::{Choice, Decision, Target};
use forge_engine::{Step, Zone};

#[test]
fn undying_and_persist_return_once() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let u = put(&db, &mut g, "Undying Kw", P1, Zone::Battlefield);
    let p = put(&db, &mut g, "Persist Lite", P1, Zone::Battlefield);
    g.move_card(u, Zone::Graveyard, true);
    g.move_card(p, Zone::Graveyard, true);
    let mut d = Driver::new(g);
    d.run_until(|g, dec| {
        g.stack.is_empty()
            && g.pending_triggers.is_empty()
            && matches!(dec, Decision::Priority { .. })
            && find(g, "Persist Lite", Zone::Battlefield).is_some()
            && find(g, "Undying Kw", Zone::Battlefield).is_some()
    });
    let u2 = find(d.g(), "Undying Kw", Zone::Battlefield).expect("zurück");
    let p2 = find(d.g(), "Persist Lite", Zone::Battlefield).expect("zurück");
    assert_eq!(d.g().card(u2).counter("P1P1"), 1);
    assert_eq!(d.g().card(p2).counter("M1M1"), 1);
    // zweites Sterben: mit Marke → bleibt im Friedhof
    let mut g = d.g().clone();
    g.move_card(u2, Zone::Graveyard, true);
    let mut d = Driver::new(g);
    d.pass_all();
    assert_eq!(count(d.g(), "Undying Kw", Zone::Graveyard), 1);
}

#[test]
fn bushido_and_flanking_in_combat() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let samurai = put(&db, &mut g, "Bushido Lite", P1, Zone::Battlefield);
    let knight = put(&db, &mut g, "Flanking Lite", P1, Zone::Battlefield);
    let b1 = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let b2 = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.run_until(|_, dec| matches!(dec, Decision::DeclareAttackers { .. }));
    d.submit(Choice::Attackers(vec![(samurai, T2), (knight, T2)]));
    d.run_until(|_, dec| matches!(dec, Decision::DeclareBlockers { .. }));
    d.submit(Choice::Blocks(vec![(b1, samurai), (b2, knight)]));
    d.run_until(|g, _| g.step == Step::EndCombat);
    assert_eq!(
        d.g().card(samurai).zone,
        Some(Zone::Battlefield),
        "4/4 durch Bushido"
    );
    assert_eq!(d.g().card(b1).zone, None);
    assert_eq!(d.g().card(b2).zone, None, "Flanking: 1/1, stirbt");
    assert_eq!(d.g().card(knight).zone, Some(Zone::Battlefield));
}

#[test]
fn untaps_trigger_and_state_trigger() {
    let (db, mut g) = game_at(Step::End, 10);
    let elf = put(&db, &mut g, "Untap Watch Lite", P2, Zone::Battlefield);
    g.card_mut(elf).tapped = true;
    put(&db, &mut g, "State Lite", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.run_until(|g, dec| {
        g.active == P2 && g.step == Step::Upkeep && matches!(dec, Decision::Priority { .. })
    });
    // Zustandstrigger: Hand leer → ziehen (einmal, nicht endlos)
    d.run_until(|g, dec| g.step == Step::Draw && matches!(dec, Decision::Priority { .. }));
    assert_eq!(d.g().player(P2).life, 22, "Untaps-Trigger");
    assert!(!d.g().player(P2).hand.is_empty());
}

#[test]
fn extra_turn_twiddle_random_discard_mill_memory() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 3);
    put_n(&db, &mut g, "Swamp", P1, Zone::Battlefield, 1);
    let time = put(&db, &mut g, "Time Lite", P1, Zone::Hand);
    let tw = put(&db, &mut g, "Twiddle Lite", P1, Zone::Hand);
    let hymn = put(&db, &mut g, "Hymn Lite", P1, Zone::Hand);
    let mill = put(&db, &mut g, "Mill Count Lite", P1, Zone::Hand);
    put(&db, &mut g, "Forest", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(tw, &[Target::Card(bear)]);
    d.pass_all();
    match d.decision() {
        Decision::Confirm { .. } => d.submit(Choice::Yes),
        o => panic!("{o:?}"),
    }
    assert!(d.g().card(bear).tapped);
    d.cast(mill, &[]);
    d.pass_all();
    assert_eq!(d.g().player(P1).life, 23, "3 gemerkte Karten");
    d.cast(time, &[]);
    d.pass_all();
    let hand = d.g().player(P1).hand.len();
    d.cast(hymn, &[]);
    d.pass_all();
    assert_eq!(
        d.g().player(P1).hand.len(),
        hand - 2,
        "Hymn + eine zufällige Karte"
    );
    assert_eq!(d.g().player(P1).life, 27);
    d.run_until(|g, _| g.turn == 3);
    assert_eq!(d.g().active, P1, "Zusatzzug");
}

#[test]
fn defender_may_attack_with_permission() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let wall = put(&db, &mut g, "Wall Lite", P1, Zone::Battlefield);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 2);
    assert!(!g.attack_candidates(P1).contains(&wall));
    put(&db, &mut g, "Plains", P1, Zone::Battlefield);
    assert!(g.attack_candidates(P1).contains(&wall));
}
