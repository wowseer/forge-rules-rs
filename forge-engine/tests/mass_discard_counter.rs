//! PumpAll, DestroyAll, DamageAll, Discard, Counter.

mod common;
use common::*;
use forge_engine::decision::{Action, CardChoiceReason, Choice, Decision, Target};
use forge_engine::keyword::Keyword;
use forge_engine::{Step, Zone};

#[test]
fn pump_all_own_creatures_until_end_of_turn() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 5);
    let s = put(&db, &mut g, "Overrun Lite", P1, Zone::Hand);
    let mine = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let theirs = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(s, &[]);
    d.pass_all();
    assert_eq!(d.g().chars(mine).power, Some(5));
    assert!(d.g().has_keyword(mine, &Keyword::Trample));
    assert_eq!(d.g().chars(theirs).power, Some(2));
    d.run_until(|g, _| g.turn == 3);
    assert_eq!(d.g().chars(mine).power, Some(2));
}

#[test]
fn wrath_destroys_all_simultaneously_with_dies_triggers() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 4);
    let w = put(&db, &mut g, "Wrath of God", P1, Zone::Hand);
    put(&db, &mut g, "Blood Artist Lite", P1, Zone::Battlefield);
    put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let myr = put(&db, &mut g, "Darksteel Myr", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(w, &[]);
    d.pass_all();
    assert_eq!(
        d.g().card(myr).zone,
        Some(Zone::Battlefield),
        "unzerstörbar"
    );
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Graveyard), 1);
    // Blood Artist sieht sich selbst und den Bären sterben (CR 603.10a)
    assert!(
        matches!(d.decision(), Decision::OrderTriggers { descriptions, .. } if descriptions.len() == 2)
    );
}

#[test]
fn damage_all_creatures_and_players() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 3);
    let pyro = put(&db, &mut g, "Pyroclasm", P1, Zone::Hand);
    let quake = put(&db, &mut g, "Earthquake Lite", P1, Zone::Hand);
    put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let spider = put(&db, &mut g, "Giant Spider", P2, Zone::Battlefield);
    let angel = put(&db, &mut g, "Serra Angel", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(pyro, &[]);
    d.pass_all();
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Graveyard), 1);
    assert_eq!(d.g().card(spider).damage, 2);
    d.run_until(|g, _| g.turn == 3);
    let mut g = d.g().clone();
    g.turn = 4;
    g.active = P1;
    g.step = Step::Main1;
    g.flow = forge_engine::state::Flow::BeginStep;
    for c in g.battlefield.clone() {
        g.card_mut(c).tapped = false;
    }
    let mut d = Driver::new(g);
    d.act(Action::Cast(quake));
    d.submit(Choice::Number(2));
    d.finish_casting(&[]);
    d.pass_all();
    assert_eq!(d.g().player(P1).life, 18);
    assert_eq!(d.g().player(P2).life, 18);
    assert_eq!(d.g().card(spider).damage, 2, "Reichweite ist kein Flying");
    assert_eq!(d.g().card(angel).damage, 0, "Flieger nicht betroffen");
}

#[test]
fn target_player_chooses_discard() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Swamp", P1, Zone::Battlefield, 3);
    let mr = put(&db, &mut g, "Mind Rot", P1, Zone::Hand);
    let h = put_n(&db, &mut g, "Grizzly Bears", P2, Zone::Hand, 3);
    let mut d = Driver::new(g);
    d.cast(mr, &[Target::Player(P2)]);
    d.pass_all();
    match d.decision().clone() {
        Decision::ChooseCards {
            player,
            reason: CardChoiceReason::Discard,
            min: 2,
            max: 2,
            ..
        } => assert_eq!(player, P2),
        o => panic!("{o:?}"),
    }
    d.submit(Choice::Cards(vec![h[0], h[2]]));
    assert_eq!(d.g().player(P2).hand.len(), 1);
}

#[test]
fn duress_caster_chooses_noncreature_nonland() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Swamp", P1, Zone::Battlefield);
    let du = put(&db, &mut g, "Duress", P1, Zone::Hand);
    put(&db, &mut g, "Grizzly Bears", P2, Zone::Hand);
    put(&db, &mut g, "Forest", P2, Zone::Hand);
    let bolt = put(&db, &mut g, "Lightning Bolt", P2, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(du, &[Target::Player(P2)]);
    d.pass_all();
    match d.decision().clone() {
        Decision::ChooseCards {
            player, options, ..
        } => {
            assert_eq!(player, P1, "Wirkender wählt");
            assert_eq!(options, vec![bolt]);
        }
        o => panic!("{o:?}"),
    }
    d.submit(Choice::Cards(vec![bolt]));
    assert_eq!(count(d.g(), "Lightning Bolt", Zone::Graveyard), 1);
}

#[test]
fn counterspell_counters_spell() {
    // CR 701.5
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 2);
    put_n(&db, &mut g, "Island", P2, Zone::Battlefield, 2);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Hand);
    let cs = put(&db, &mut g, "Counterspell", P2, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(bear, &[]);
    d.act(Action::Pass);
    let spell = match &d.g().stack[0].kind {
        forge_engine::state::StackKind::Spell { card } => *card,
        _ => unreachable!(),
    };
    d.cast(cs, &[Target::Card(spell)]);
    d.pass_all();
    assert!(d.g().stack.is_empty());
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Graveyard), 1);
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Battlefield), 0);
}

#[test]
fn mana_leak_pay_or_countered() {
    // CR 118.12
    for (pay, can_pay, expect_resolved) in [
        (true, true, true),
        (false, true, false),
        (true, false, false),
    ] {
        let (db, mut g) = game_at(Step::Main1, 10);
        put_n(
            &db,
            &mut g,
            "Forest",
            P1,
            Zone::Battlefield,
            if can_pay { 5 } else { 2 },
        );
        put_n(&db, &mut g, "Island", P2, Zone::Battlefield, 2);
        let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Hand);
        let ml = put(&db, &mut g, "Mana Leak", P2, Zone::Hand);
        let mut d = Driver::new(g);
        d.cast(bear, &[]);
        d.act(Action::Pass);
        let spell = match &d.g().stack[0].kind {
            forge_engine::state::StackKind::Spell { card } => *card,
            _ => unreachable!(),
        };
        d.cast(ml, &[Target::Card(spell)]);
        d.pass_all();
        match d.decision() {
            Decision::PayUnlessCost { player, .. } => assert_eq!(*player, P1),
            o => panic!("{o:?}"),
        }
        d.submit(if pay { Choice::Yes } else { Choice::No });
        d.pass_all();
        assert_eq!(
            find(d.g(), "Grizzly Bears", Zone::Battlefield).is_some(),
            expect_resolved,
            "pay={pay} can_pay={can_pay}"
        );
    }
}

#[test]
fn creature_counter_only_targets_creature_spells() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Mountain", P1, Zone::Battlefield);
    put_n(&db, &mut g, "Island", P2, Zone::Battlefield, 2);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let es = put(&db, &mut g, "Essence Scatter Lite", P2, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(bolt, &[Target::Player(P2)]);
    d.act(Action::Pass);
    match d.decision() {
        Decision::Priority { actions, .. } => assert!(
            !actions.contains(&Action::Cast(es)),
            "kein Kreaturenzauber auf dem Stapel"
        ),
        o => panic!("{o:?}"),
    }
}
