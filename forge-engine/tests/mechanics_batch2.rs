//! Explore, StoreSVar, RememberTokens, CantTarget, Panharmonicon, SpellCastOrCopy,
//! DamagedBy, wasCastByYou.

mod common;
use common::*;
use forge_engine::decision::{Action, Choice, Decision, Target};
use forge_engine::{Step, Zone};

#[test]
fn explore_land_to_hand_or_counter() {
    // oben ein Land → auf die Hand
    let (db, mut g) = game_at(Step::Main1, 3);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 2);
    put(&db, &mut g, "Forest", P1, Zone::Library);
    let e = put(&db, &mut g, "Explorer Kw", P1, Zone::Hand);
    let mut d = Driver::new(g);
    let hand = d.g().player(P1).hand.len();
    d.cast(e, &[]);
    d.pass_all();
    d.pass_all();
    assert_eq!(d.g().player(P1).hand.len(), hand, "−1 gewirkt +1 Land");
    // oben ein Nichtland → Marke, Karte darf in den Friedhof
    let (db, mut g) = game_at(Step::Main1, 3);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 2);
    put(&db, &mut g, "Grizzly Bears", P1, Zone::Library);
    let e = put(&db, &mut g, "Explorer Kw", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(e, &[]);
    d.pass_all();
    d.pass_all();
    match d.decision() {
        Decision::Confirm { .. } => d.submit(Choice::Yes),
        o => panic!("{o:?}"),
    }
    let x = find(d.g(), "Explorer Kw", Zone::Battlefield).unwrap();
    assert_eq!(d.g().card(x).counter("P1P1"), 1);
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Graveyard), 1);
}

#[test]
fn store_svar_and_remember_tokens() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 1);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 1);
    put_n(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield, 3);
    let st = put(&db, &mut g, "Store Kw", P1, Zone::Hand);
    let tk = put(&db, &mut g, "Token Memory Kw", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(st, &[]);
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 17, "gespeicherter Wert 3");
    d.cast(tk, &[]);
    d.pass_all();
    let tokens: Vec<_> = d
        .g()
        .battlefield
        .iter()
        .filter(|c| d.g().card(**c).token)
        .copied()
        .collect();
    assert_eq!(tokens.len(), 2);
    assert!(tokens.iter().all(|t| d.g().chars(*t).power == Some(3)));
}

#[test]
fn cant_target_my_creatures_with_opponent_spells() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 1);
    put(&db, &mut g, "Ward Off Kw", P2, Zone::Battlefield);
    let theirs = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mine = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.act(Action::Cast(bolt));
    match d.decision().clone() {
        Decision::ChooseTargets { candidates, .. } => {
            assert!(!candidates.contains(&Target::Card(theirs)));
            assert!(candidates.contains(&Target::Card(mine)));
        }
        o => panic!("{o:?}"),
    }
}

#[test]
fn panharmonicon_doubles_etb_triggers() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 2);
    put(&db, &mut g, "Harmonic Kw", P1, Zone::Battlefield);
    put(&db, &mut g, "Soul Warden Lite", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(bear, &[]);
    d.run_until(|g, dec| {
        g.stack.is_empty()
            && g.pending_triggers.is_empty()
            && matches!(dec, Decision::Priority { .. })
            && find(g, "Grizzly Bears", Zone::Battlefield).is_some()
    });
    assert_eq!(d.g().player(P1).life, 22, "Soul Warden zweimal");
}

#[test]
fn spell_cast_or_copy_and_damaged_by() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 1);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 1);
    put(&db, &mut g, "Echo Copy Kw", P1, Zone::Battlefield);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let twin = put(&db, &mut g, "Twin Lite", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(bolt, &[Target::Card(bear)]);
    d.pass_all(); // Trigger (Wirken)
    let spell = d
        .g()
        .stack
        .iter()
        .find_map(|i| match i.kind {
            forge_engine::state::StackKind::Spell { card } => Some(card),
            _ => None,
        })
        .unwrap();
    d.cast(twin, &[Target::Card(spell)]);
    d.run_until(|_, dec| matches!(dec, Decision::Confirm { .. }));
    d.submit(Choice::No); // Kopie behält das Ziel
    d.run_until(|g, dec| g.stack.is_empty() && matches!(dec, Decision::Priority { .. }));
    // Trigger: Bolt gewirkt (+1), Twin gewirkt (+1), Kopie (+1)
    assert_eq!(d.g().player(P1).life, 23);
    assert!(
        d.g().this_turn.damaged.iter().any(|(_, t)| *t == bear),
        "DamagedBy-Verlauf"
    );
}
