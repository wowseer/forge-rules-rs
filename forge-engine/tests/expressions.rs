//! Ausdrücke (`Count$…`, `PlayerCount…`) – Java `AbilityUtils.calculateAmount`.

mod common;
use common::*;
use forge_engine::expr::{EvalCtx, Expr};
use forge_engine::{Step, Zone};

fn eval(db: &forge_engine::CardDb, g: &forge_engine::GameState, sv: &str) -> i32 {
    let face = db.get("Grizzly Bears").unwrap().face(0).clone();
    let e = Expr::parse_svar(&face, sv).unwrap_or_else(|e| panic!("{sv}: {e}"));
    let src = g.battlefield[0];
    g.eval(
        &e,
        &EvalCtx {
            source: src,
            controller: P1,
            x: 0,
            targets: &[],
            trig: None,
            paid: &[],
        },
    )
}

#[test]
fn party_counts_each_role_once_with_best_assignment() {
    let (db, mut g) = game_at(Step::Main1, 5);
    // Kleriker/Zauberer + Zauberer: beide Rollen belegbar → 2
    put(&db, &mut g, "Cleric Wizard Lite", P1, Zone::Battlefield);
    put(&db, &mut g, "Prodigal Pyromancer", P1, Zone::Battlefield);
    put(&db, &mut g, "Firebrand Lite", P1, Zone::Battlefield);
    put(&db, &mut g, "Prodigal Pyromancer", P2, Zone::Battlefield);
    assert_eq!(eval(&db, &g, "Count$Party"), 2);
}

#[test]
fn valid_aggregates() {
    let (db, mut g) = game_at(Step::Main1, 5);
    put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    put(&db, &mut g, "Hill Giant", P1, Zone::Battlefield);
    put(&db, &mut g, "Serra Angel", P2, Zone::Battlefield);
    assert_eq!(
        eval(&db, &g, "Count$Valid Creature.YouCtrl$GreatestCardPower"),
        3
    );
    assert_eq!(eval(&db, &g, "Count$Valid Creature.YouCtrl$CardPower"), 5);
    assert_eq!(eval(&db, &g, "Count$Valid Creature$Colors"), 3);
    assert_eq!(eval(&db, &g, "Count$Valid Creature$DifferentCardPower"), 3);
    assert_eq!(
        eval(&db, &g, "Count$Valid Creature$GreatestCardManaCost/Plus.1"),
        6
    );
}

#[test]
fn player_counts() {
    let (db, mut g) = game_at(Step::Main1, 5);
    put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    put_n(&db, &mut g, "Forest", P2, Zone::Battlefield, 3);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 1);
    g.player_mut(P2).life = 7;
    assert_eq!(eval(&db, &g, "PlayerCountOpponents$Amount"), 1);
    assert_eq!(eval(&db, &g, "PlayerCountPlayers$Amount"), 2);
    assert_eq!(
        eval(&db, &g, "PlayerCountOpponents$HighestValid Land.YouCtrl"),
        3
    );
    assert_eq!(eval(&db, &g, "PlayerCountPlayers$LowestLifeTotal"), 7);
    assert_eq!(eval(&db, &g, "PlayerCountPlayers$LifeTotal"), 27);
}
