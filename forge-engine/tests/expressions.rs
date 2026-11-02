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

#[test]
fn pump_double_power_and_toughness() {
    let (db, mut g) = game_at(Step::Main1, 5);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 1);
    let giant = put(&db, &mut g, "Hill Giant", P1, Zone::Battlefield);
    let dbl = put(&db, &mut g, "Double Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(dbl, &[forge_engine::decision::Target::Card(giant)]);
    d.pass_all();
    assert_eq!(d.g().chars(giant).power, Some(6));
    assert_eq!(d.g().chars(giant).toughness, Some(6));
}

#[test]
fn condition_self_was_cast_from_hand() {
    let (db, mut g) = game_at(Step::Main1, 5);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 1);
    let elf = put(&db, &mut g, "Cast Bonus Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    let hand = d.g().player(P1).hand.len();
    d.cast(elf, &[]);
    d.pass_all(); // Kreatur
    d.pass_all(); // ETB
    assert_eq!(d.g().player(P1).hand.len(), hand, "−1 gewirkt, +1 gezogen");
    // direkt ins Spiel gebracht: nicht gewirkt → kein Ziehen
    let (db, mut g) = game_at(Step::Main1, 5);
    let elf = put(&db, &mut g, "Cast Bonus Lite", P1, Zone::Hand);
    g.move_card(elf, Zone::Battlefield, true);
    let mut d = Driver::new(g);
    let hand = d.g().player(P1).hand.len();
    d.pass_all();
    assert_eq!(d.g().player(P1).hand.len(), hand);
}

#[test]
fn search_any_number() {
    let (db, mut g) = game_at(Step::Main1, 3);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 1);
    let forests: Vec<_> = (0..3)
        .map(|_| put(&db, &mut g, "Forest", P1, Zone::Library))
        .collect();
    let s = put(&db, &mut g, "Search Any Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(s, &[]);
    d.pass_all();
    match d.decision().clone() {
        forge_engine::decision::Decision::ChooseCards { min, max, .. } => {
            assert_eq!((min, max), (0, 3));
        }
        o => panic!("{o:?}"),
    }
    d.submit(forge_engine::decision::Choice::Cards(forests.clone()));
    assert_eq!(count(d.g(), "Forest", Zone::Hand), 3);
}
