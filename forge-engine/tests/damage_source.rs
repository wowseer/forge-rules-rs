//! `DealDamage | DamageSource$`: eine andere Quelle fügt den Schaden zu.

mod common;
use common::*;
use forge_engine::decision::Target;
use forge_engine::{Step, Zone};

#[test]
fn bite_uses_creature_as_source_with_its_keywords() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 1);
    let hawk = put(&db, &mut g, "Vampire Nighthawk", P1, Zone::Battlefield);
    let giant = put(&db, &mut g, "Hill Giant", P2, Zone::Battlefield);
    let bite = put(&db, &mut g, "Source Bite Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.act(forge_engine::decision::Action::Cast(bite));
    d.submit(forge_engine::decision::Choice::Targets(vec![Target::Card(
        hawk,
    )]));
    d.finish_casting(&[Target::Card(giant)]);
    d.pass_all();
    assert_eq!(d.g().card(giant).zone, None, "Deathtouch des Falken");
    assert_eq!(d.g().player(P1).life, 22, "Lifelink des Falken (2 Schaden)");
}
