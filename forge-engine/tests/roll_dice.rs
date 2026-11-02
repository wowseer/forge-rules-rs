//! Würfeln (CR 706): Ergebnisvariable und Ergebnistabellen.

mod common;
use common::*;
use forge_engine::state::Event;
use forge_engine::{Step, Zone};

fn last_roll(g: &forge_engine::GameState) -> i32 {
    g.log
        .iter()
        .rev()
        .find_map(|e| match e {
            Event::DiceRolled { total, .. } => Some(*total),
            _ => None,
        })
        .expect("gewürfelt")
}

#[test]
fn kitten_gains_life_equal_to_roll() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 1);
    let kitten = put(&db, &mut g, "Adorable Kitten", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(kitten, &[]);
    d.pass_all();
    d.pass_all();
    let r = last_roll(d.g());
    assert!((1..=6).contains(&r));
    assert_eq!(d.g().player(P1).life, 20 + r);
}

#[test]
fn d20_table_picks_range() {
    for seed in 0..12u64 {
        let (db, mut g) = game_at(Step::Main1, 10);
        g.rng = forge_engine::rng::Rng::new(seed);
        put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 1);
        let s = put(&db, &mut g, "D20 Lite", P1, Zone::Hand);
        let mut d = Driver::new(g);
        d.cast(s, &[]);
        d.pass_all();
        let r = last_roll(d.g());
        let dmg = match r {
            1..=9 => 1,
            10..=19 => 2,
            _ => 3,
        };
        assert_eq!(d.g().player(P2).life, 20 - dmg, "Wurf {r}");
    }
}
