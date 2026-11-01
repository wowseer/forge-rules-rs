//! Kostenänderungen (ReduceCost/RaiseCost, CR 601.2f).

mod common;
use common::*;
use forge_engine::decision::{Action, Decision, Target};
use forge_engine::{Step, Zone};

fn can_cast(d: &Driver, c: forge_engine::CardId) -> bool {
    matches!(d.decision(), Decision::Priority { actions, .. } if actions.contains(&Action::Cast(c)))
}

#[test]
fn self_reduction_with_condition() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Mountain", P1, Zone::Battlefield);
    let s = put(&db, &mut g, "Wizard Bolt Lite", P1, Zone::Hand);
    assert!(
        !can_cast(&Driver::new(g.clone()), s),
        "kostet 3 ohne Zauberer"
    );
    put(&db, &mut g, "Electromancer Lite", P1, Zone::Battlefield);
    // Electromancer (−1) + eigener Effekt (−2): {2}{R} → {R}
    let mut d = Driver::new(g);
    assert!(can_cast(&d, s));
    d.cast(s, &[Target::Player(P2)]);
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 17);
}

#[test]
fn reduction_only_generic_and_only_for_caster() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Electromancer Lite", P1, Zone::Battlefield);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let div = put(&db, &mut g, "Divination", P1, Zone::Hand);
    let s = put(&db, &mut g, "Wizard Bolt Lite", P1, Zone::Hand);
    let costs = (
        g.adjusted_spell_cost(P1, bolt, &g.chars(bolt).mana_cost.unwrap())
            .to_string(),
        g.adjusted_spell_cost(P1, div, &g.chars(div).mana_cost.unwrap())
            .to_string(),
        g.adjusted_spell_cost(P2, div, &g.chars(div).mana_cost.unwrap())
            .to_string(),
        g.adjusted_spell_cost(P1, s, &g.chars(s).mana_cost.unwrap())
            .to_string(),
    );
    assert_eq!(
        costs,
        ("{R}".into(), "{1}{U}".into(), "{2}{U}".into(), "{R}".into())
    );
}

#[test]
fn raise_cost_applies_to_everyone() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put(&db, &mut g, "Thalia Lite", P2, Zone::Battlefield);
    put(&db, &mut g, "Mountain", P1, Zone::Battlefield);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Hand);
    assert_eq!(
        g.adjusted_spell_cost(P1, bolt, &g.chars(bolt).mana_cost.unwrap())
            .to_string(),
        "{1}{R}"
    );
    assert_eq!(
        g.adjusted_spell_cost(P1, bear, &g.chars(bear).mana_cost.unwrap())
            .to_string(),
        "{1}{G}",
        "Kreaturen nicht betroffen"
    );
    assert!(
        !can_cast(&Driver::new(g), bolt),
        "ein Berg reicht nicht mehr"
    );
}
