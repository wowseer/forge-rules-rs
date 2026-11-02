//! Mehrseitige Karten: Adventure (CR 715), modale doppelseitige Karten (CR 712).

mod common;
use common::*;
use forge_engine::decision::{Action, Decision, Target};
use forge_engine::{Step, Zone};

fn actions(d: &Driver) -> Vec<Action> {
    match d.decision() {
        Decision::Priority { actions, .. } => actions.clone(),
        o => panic!("{o:?}"),
    }
}

#[test]
fn adventure_goes_to_exile_and_creature_can_be_cast_later() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 5);
    let giant = put(&db, &mut g, "Giant Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    let acts = actions(&d);
    assert!(acts.contains(&Action::Cast(giant)));
    assert!(acts.contains(&Action::CastAlt(giant, 1)));
    d.act(Action::CastAlt(giant, 1));
    d.finish_casting(&[Target::Player(P2)]);
    // Auf dem Stapel hat die Karte nur die Abenteuer-Eigenschaften (CR 715.3a)
    let on_stack = match d.g().stack[0].kind {
        forge_engine::state::StackKind::Spell { card } => card,
        _ => unreachable!(),
    };
    assert_eq!(d.g().chars(on_stack).name, "Stomp Lite");
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 18);
    let exiled = find(d.g(), "Giant Lite", Zone::Exile).expect("auf Abenteuer im Exil");
    assert!(d.g().card(exiled).on_adventure);
    assert!(actions(&d).contains(&Action::Cast(exiled)));
    d.act(Action::Cast(exiled));
    d.finish_casting(&[]);
    d.pass_all();
    assert_eq!(count(d.g(), "Giant Lite", Zone::Battlefield), 1);
}

#[test]
fn modal_double_faced_land_side_enters_with_that_face() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let v = put(&db, &mut g, "Valakut Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    let acts = actions(&d);
    assert!(acts.contains(&Action::PlayLandAlt(v, 1)), "{acts:?}");
    assert!(
        !acts.contains(&Action::Cast(v)),
        "kein Mana für die Vorderseite"
    );
    d.act(Action::PlayLandAlt(v, 1));
    let land = find(d.g(), "Valakut Lite", Zone::Battlefield).unwrap();
    assert_eq!(d.g().chars(land).name, "Valakut Cliffs Lite");
    assert!(d.g().chars(land).types.is_land());
    assert!(d.g().card(land).tapped, "Ersatzeffekt der Rückseite");
}

#[test]
fn modal_double_faced_front_is_a_spell() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 3);
    let v = put(&db, &mut g, "Valakut Lite", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(v, &[Target::Player(P2)]);
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 17);
    assert_eq!(count(d.g(), "Valakut Lite", Zone::Graveyard), 1);
}

#[test]
fn transform_keeps_object_damage_and_counters() {
    // CR 712.18: Transformieren erzeugt kein neues Objekt
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 2);
    let v = put(&db, &mut g, "Villager Lite", P1, Zone::Battlefield);
    g.add_counters(v, "P1P1", 1);
    g.card_mut(v).damage = 1;
    let mut d = Driver::new(g);
    d.act(Action::Activate(v, 0));
    d.finish_casting(&[]);
    d.pass_all();
    let ch = d.g().chars(v);
    assert_eq!(ch.name, "Howler Lite");
    assert_eq!(ch.power, Some(5), "4/4 + Marke");
    assert!(
        d.g()
            .has_keyword(v, &forge_engine::keyword::Keyword::Trample)
    );
    assert_eq!(d.g().card(v).damage, 1);
    assert_eq!(
        ch.mana_cost.as_ref().unwrap().mana_value(),
        2,
        "CR 712.8e: Manawert der Vorderseite"
    );
}
