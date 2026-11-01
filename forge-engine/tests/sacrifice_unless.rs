//! Sacrifice (CR 701.21) und allgemeines `UnlessCost$` (CR 118.12).

mod common;
use common::*;
use forge_engine::decision::{CardChoiceReason, Choice, Decision, Target};
use forge_engine::{Step, Zone};

fn expect_sacrifice_choice(
    d: &Driver,
    who: forge_engine::PlayerId,
) -> (Vec<forge_engine::CardId>, u32, u32) {
    match d.decision() {
        Decision::ChooseCards {
            player,
            reason: CardChoiceReason::Sacrifice,
            options,
            min,
            max,
        } => {
            assert_eq!(*player, who);
            (options.clone(), *min, *max)
        }
        o => panic!("erwartet Opfer-Wahl, kam {o:?}"),
    }
}

#[test]
fn edict_target_player_chooses_and_indestructible_can_be_sacrificed() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Swamp", P1, Zone::Battlefield, 2);
    let edict = put(&db, &mut g, "Diabolic Edict", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let myr = put(&db, &mut g, "Darksteel Myr", P2, Zone::Battlefield);
    put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(edict, &[Target::Player(P2)]);
    d.pass_all();
    let (opts, min, max) = expect_sacrifice_choice(&d, P2);
    assert_eq!((min, max), (1, 1));
    assert_eq!(opts.len(), 2, "nur eigene Kreaturen des Ziels");
    // CR 701.21a: Opfern ist kein Zerstören – Unzerstörbarkeit hilft nicht.
    d.submit(Choice::Cards(vec![myr]));
    d.pass_all();
    assert_eq!(count(d.g(), "Darksteel Myr", Zone::Graveyard), 1);
    assert_eq!(d.g().card(bear).zone, Some(Zone::Battlefield));
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Battlefield), 2);
}

#[test]
fn each_player_sacrifices_simultaneously_dies_triggers_see_all() {
    // CR 101.4: erst wählen alle in APNAP-Reihenfolge, dann gleichzeitig opfern.
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Swamp", P1, Zone::Battlefield);
    let ib = put(&db, &mut g, "Innocent Blood", P1, Zone::Hand);
    let artist = put(&db, &mut g, "Blood Artist Lite", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(ib, &[]);
    d.pass_all();
    let _ = expect_sacrifice_choice(&d, P1);
    d.submit(Choice::Cards(vec![artist]));
    let _ = expect_sacrifice_choice(&d, P2);
    d.submit(Choice::Cards(vec![bear]));
    // Blood Artist sieht sich selbst und den Bären sterben (CR 603.10a)
    assert!(
        matches!(d.decision(), Decision::OrderTriggers { descriptions, .. } if descriptions.len() == 2),
        "{:?}",
        d.decision()
    );
}

#[test]
fn player_without_creatures_is_not_asked() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Swamp", P1, Zone::Battlefield);
    let ib = put(&db, &mut g, "Innocent Blood", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(ib, &[]);
    d.pass_all();
    let _ = expect_sacrifice_choice(&d, P2);
    d.submit(Choice::Cards(vec![bear]));
    d.pass_all();
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Graveyard), 1);
}

#[test]
fn optional_up_to_amount_and_remembered() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Swamp", P1, Zone::Battlefield);
    let s = put(&db, &mut g, "Optional Offering Lite", P1, Zone::Hand);
    let b1 = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    put(&db, &mut g, "Hill Giant", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(s, &[]);
    let on_stack = match d.g().stack[0].kind {
        forge_engine::state::StackKind::Spell { card } => card,
        _ => unreachable!(),
    };
    d.pass_all();
    let (_, min, max) = expect_sacrifice_choice(&d, P1);
    assert_eq!((min, max), (0, 2));
    d.submit(Choice::Cards(vec![b1]));
    d.pass_all();
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Graveyard), 1);
    // Die Quelle (Objekt auf dem Stapel) merkt sich das geopferte Objekt (LKI).
    assert_eq!(d.g().card(on_stack).remembered, vec![b1]);
}

#[test]
fn strict_amount_sacrifices_nothing_if_not_enough() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Swamp", P1, Zone::Battlefield);
    let s = put(&db, &mut g, "Strict Pact Lite", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(s, &[]);
    d.pass_all();
    assert!(!matches!(d.decision(), Decision::ChooseCards { .. }));
    assert_eq!(d.g().card(bear).zone, Some(Zone::Battlefield));
}

#[test]
fn upkeep_sacrifice_unless_you_pay() {
    // CR 118.12: Zahlung freiwillig; ohne Mana wird gar nicht gefragt.
    for (lands, pay, survives) in [(1, true, true), (1, false, false), (0, true, false)] {
        let (db, mut g) = game_at(Step::Untap, 10);
        put_n(&db, &mut g, "Swamp", P1, Zone::Battlefield, lands);
        let horror = put(&db, &mut g, "Upkeep Tax Lite", P1, Zone::Battlefield);
        let mut d = Driver::new(g);
        d.priority_in(Step::Upkeep, P1);
        d.pass_all();
        if lands > 0 {
            match d.decision() {
                Decision::PayUnlessCost { player, .. } => assert_eq!(*player, P1),
                o => panic!("{o:?}"),
            }
            d.submit(if pay { Choice::Yes } else { Choice::No });
            d.pass_all();
        }
        assert_eq!(
            d.g().card(horror).zone == Some(Zone::Battlefield),
            survives,
            "lands={lands} pay={pay}"
        );
    }
}

#[test]
fn unless_discard_paid_by_targeted_player() {
    for pay in [true, false] {
        let (db, mut g) = game_at(Step::Main1, 10);
        put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 2);
        let s = put(&db, &mut g, "Painful Choice Lite", P1, Zone::Hand);
        let card = put(&db, &mut g, "Hill Giant", P2, Zone::Hand);
        let mut d = Driver::new(g);
        d.cast(s, &[Target::Player(P2)]);
        d.pass_all();
        match d.decision() {
            Decision::PayUnlessCost { player, .. } => assert_eq!(*player, P2),
            o => panic!("{o:?}"),
        }
        d.submit(if pay { Choice::Yes } else { Choice::No });
        if pay {
            d.submit(Choice::Cards(vec![card]));
        }
        d.pass_all();
        assert_eq!(d.g().player(P2).life, if pay { 20 } else { 16 });
        assert_eq!(d.g().card(card).zone == Some(Zone::Hand), !pay);
    }
}

#[test]
fn unless_switched_and_subs_only_when_paid() {
    for pay in [true, false] {
        let (db, mut g) = game_at(Step::Main1, 10);
        put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 2);
        let s = put(&db, &mut g, "Tribute Lite", P1, Zone::Hand);
        let mut d = Driver::new(g);
        d.cast(s, &[]);
        d.pass_all();
        d.submit(if pay { Choice::Yes } else { Choice::No });
        d.pass_all();
        let pl = d.g().player(P1);
        if pay {
            assert_eq!((pl.hand.len(), pl.life), (2, 18), "20 − 3 + 1");
        } else {
            assert_eq!((pl.hand.len(), pl.life), (0, 20));
        }
    }
}

fn stack_card(d: &Driver) -> forge_engine::CardId {
    match d.g().stack.last().unwrap().kind {
        forge_engine::state::StackKind::Spell { card } => card,
        _ => unreachable!(),
    }
}

#[test]
fn condition_defined_remembered_and_cleanup() {
    for sac in [true, false] {
        let (db, mut g) = game_at(Step::Main1, 10);
        put(&db, &mut g, "Swamp", P1, Zone::Battlefield);
        let s = put(&db, &mut g, "Offering Draw Lite", P1, Zone::Hand);
        let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
        let mut d = Driver::new(g);
        d.cast(s, &[]);
        let sc = stack_card(&d);
        d.pass_all();
        d.submit(Choice::Cards(if sac { vec![bear] } else { vec![] }));
        d.pass_all();
        assert_eq!(
            d.g().player(P1).hand.len(),
            if sac { 2 } else { 0 },
            "sac={sac}"
        );
        assert!(d.g().card(sc).remembered.is_empty(), "ClearRemembered");
    }
}

#[test]
fn remembered_amount_counts_sacrificed() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Swamp", P1, Zone::Battlefield);
    let s = put(&db, &mut g, "Mass Offering Lite", P1, Zone::Hand);
    let a = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let b = put(&db, &mut g, "Hill Giant", P1, Zone::Battlefield);
    put(&db, &mut g, "Goblin Piker", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(s, &[]);
    d.pass_all();
    d.submit(Choice::Cards(vec![a, b]));
    d.pass_all();
    assert_eq!(d.g().player(P1).life, 24);
}

#[test]
fn defined_remembered_follows_card_to_new_zone() {
    // Java: AbilityUtils.getDefinedCards("Remembered") → game.getCardState(card)
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Swamp", P1, Zone::Battlefield);
    let s = put(&db, &mut g, "Recycle Lite", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(s, &[]);
    d.pass_all();
    d.submit(Choice::Cards(vec![bear]));
    d.pass_all();
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Hand), 1);
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Graveyard), 0);
}

#[test]
fn sacrificed_trigger_only_for_own_sacrifices_not_destroy() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Swamp", P1, Zone::Battlefield, 3);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 4);
    put(&db, &mut g, "Sac Watcher Lite", P1, Zone::Battlefield);
    let ib = put(&db, &mut g, "Innocent Blood", P1, Zone::Hand);
    let mine = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let theirs = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(ib, &[]);
    d.pass_all();
    d.submit(Choice::Cards(vec![mine]));
    d.submit(Choice::Cards(vec![theirs]));
    // nur das eigene Opfer löst aus (ValidPlayer$ You)
    assert_eq!(d.g().stack.len(), 1);
    d.pass_all();
    assert_eq!(d.g().player(P1).hand.len(), 1);
}

#[test]
fn self_sacrifice_as_cost_triggers_from_lki() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let m = put(&db, &mut g, "Martyr Lite", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.act(forge_engine::decision::Action::Activate(m, 0));
    // Fähigkeit und Trigger („wenn du ~ opferst“) liegen auf dem Stapel
    assert_eq!(d.g().stack.len(), 2);
    d.pass_all();
    d.pass_all();
    assert_eq!(d.g().player(P1).life, 24);
}
