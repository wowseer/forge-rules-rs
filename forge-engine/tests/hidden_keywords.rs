//! Versteckte Keywords aus Pump-Effekten: „can't block“, „doesn't untap during
//! your next untap step“ (CR 502.3).

mod common;
use common::*;
use forge_engine::decision::{Choice, Decision, Target};
use forge_engine::{Step, Zone};

#[test]
fn pumped_cant_block_until_end_of_turn() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 1);
    let giant = put(&db, &mut g, "Hill Giant", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let spider = put(&db, &mut g, "Giant Spider", P2, Zone::Battlefield);
    let spell = put(&db, &mut g, "No Block Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(spell, &[Target::Card(bear)]);
    d.pass_all();
    d.run_until(|_, dec| matches!(dec, Decision::DeclareAttackers { .. }));
    d.submit(Choice::Attackers(vec![(giant, T2)]));
    d.run_until(|_, dec| matches!(dec, Decision::DeclareBlockers { .. }));
    match d.decision().clone() {
        Decision::DeclareBlockers { candidates, .. } => {
            assert!(!candidates.contains(&bear), "Bär kann nicht blocken");
            assert!(candidates.contains(&spider));
        }
        o => panic!("{o:?}"),
    }
}

#[test]
fn frozen_creature_skips_one_untap_step() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 1);
    let bear = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let frost = put(&db, &mut g, "Frost Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(frost, &[Target::Card(bear)]);
    d.pass_all();
    assert!(d.g().card(bear).tapped);
    d.run_until(|g, _| g.active == P2 && g.step == Step::Upkeep);
    assert!(
        d.g().card(bear).tapped,
        "erstes Enttappsegment übersprungen"
    );
    d.run_until(|g, _| g.active == P1 && g.step == Step::Upkeep);
    d.run_until(|g, _| g.active == P2 && g.step == Step::Upkeep);
    assert!(!d.g().card(bear).tapped, "danach normal");
}
