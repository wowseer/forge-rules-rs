//! Kampfbeschränkungen: CantBlockBy, CantBlock, CantAttack, MustAttack (CR 508.1c/d, 509.1b).

mod common;
use common::*;
use forge_engine::decision::{Choice, Decision, Target};
use forge_engine::{CardId, GameState, Step, Zone};

fn to_blockers(g: GameState, attackers: &[CardId]) -> Driver {
    let mut d = Driver::new(g);
    d.run_until(|_, dec| matches!(dec, Decision::DeclareAttackers { .. }));
    d.submit(Choice::Attackers(
        attackers.iter().map(|a| (*a, P2)).collect(),
    ));
    d.run_until(|_, dec| matches!(dec, Decision::DeclareBlockers { .. }));
    d
}

#[test]
fn unblockable() {
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    let pw = put(&db, &mut g, "Phantom Warrior Lite", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = to_blockers(g, &[pw]);
    assert!(d.try_submit(Choice::Blocks(vec![(bear, pw)])).is_err());
}

#[test]
fn cant_be_blocked_by_small_creatures_via_effect() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Swamp", P1, Zone::Battlefield, 2);
    let sk = put(&db, &mut g, "Skulk Lite", P1, Zone::Hand);
    let giant = put(&db, &mut g, "Hill Giant", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let wurm = put(&db, &mut g, "Craw Wurm", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.cast(sk, &[Target::Card(giant)]);
    d.pass_all();
    let g = d.g().clone();
    let mut d = to_blockers(g, &[giant]);
    assert!(
        d.try_submit(Choice::Blocks(vec![(bear, giant)])).is_err(),
        "Stärke 2 darf nicht blocken"
    );
    d.submit(Choice::Blocks(vec![(wurm, giant)]));
}

#[test]
fn cant_block() {
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    put(&db, &mut g, "Craven Lite", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.run_until(|_, dec| matches!(dec, Decision::DeclareAttackers { .. }));
    d.submit(Choice::Attackers(vec![(bear, P2)]));
    d.run_until(|g, _| g.step == Step::Main2);
    assert_eq!(
        d.g().player(P2).life,
        18,
        "keine Blockentscheidung, weil niemand blocken kann"
    );
}

#[test]
fn cant_attack() {
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    put(&db, &mut g, "Pacifism Field Lite", P2, Zone::Battlefield);
    put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.run_until(|g, _| g.step == Step::Main2);
    assert!(
        !d.g()
            .log
            .iter()
            .any(|e| matches!(e, forge_engine::Event::AttackersDeclared(v) if !v.is_empty()))
    );
}

#[test]
fn must_attack_if_able() {
    let (db, mut g) = game_at(Step::BeginCombat, 10);
    let b = put(&db, &mut g, "Berserker Lite", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.run_until(|_, dec| matches!(dec, Decision::DeclareAttackers { .. }));
    assert!(
        d.try_submit(Choice::Attackers(vec![(bear, P2)])).is_err(),
        "Berserker muss mit"
    );
    d.submit(Choice::Attackers(vec![(b, P2)]));
}
