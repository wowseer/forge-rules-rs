//! `ChooseSource` + „das nächste Mal verhindern“ (Circle of Protection, CR 615).

mod common;
use common::*;
use forge_engine::decision::{Action, Choice, Decision, Target};
use forge_engine::{Step, Zone};

#[test]
fn circle_prevents_next_damage_from_chosen_red_source_once() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 1);
    let cop = put(
        &db,
        &mut g,
        "Circle of Protection: Red",
        P1,
        Zone::Battlefield,
    );
    let pyro = put(&db, &mut g, "Prodigal Pyromancer", P2, Zone::Battlefield);
    let pyro2 = put(&db, &mut g, "Prodigal Pyromancer", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.act(Action::Activate(cop, 0));
    d.finish_casting(&[]);
    d.pass_all();
    match d.decision().clone() {
        Decision::ChooseCards { options, .. } => {
            assert!(options.contains(&pyro) && options.contains(&pyro2));
            assert!(!options.contains(&cop), "nur rote Quellen");
            d.submit(Choice::Cards(vec![pyro]));
        }
        o => panic!("{o:?}"),
    }
    assert_eq!(d.g().command.len(), 1, "Effektobjekt");
    // P1 passt, P2 pingt zweimal
    d.act(Action::Pass);
    d.act(Action::Activate(pyro, 0));
    d.finish_casting(&[Target::Player(P1)]);
    d.pass_all();
    assert_eq!(d.g().player(P1).life, 20, "verhindert");
    assert!(d.g().command.is_empty(), "nur einmal");
    d.act(Action::Pass);
    d.act(Action::Activate(pyro2, 0));
    d.finish_casting(&[Target::Player(P1)]);
    d.pass_all();
    assert_eq!(d.g().player(P1).life, 19, "andere Quelle nicht betroffen");
}
