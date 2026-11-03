//! Block 5: ziel-/triggerbezogene Filter, Discover, `Defined$ Parent`, DamageResolve,
//! Transformed, Trigger-Limit, Fähigkeits-/Landtypverlust, Buyback, Living Weapon,
//! Clone bis Zugende, `UntilHostLeavesPlayOrEOT`, `ChangeNum$ Any`.

mod common;
use common::*;
use forge_engine::decision::{Action, Choice, Decision, Target};
use forge_engine::{Step, Zone};

#[test]
fn controlled_by_triggered_target_limits_targets() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 2);
    let pinger = put(&db, &mut g, "Pinger B5", P1, Zone::Battlefield);
    let mine = put(&db, &mut g, "Hill Giant", P1, Zone::Battlefield);
    let theirs = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.act(Action::Activate(pinger, 0));
    d.finish_casting(&[T2]);
    d.pass_all();
    match d.decision().clone() {
        Decision::ChooseTargets { candidates, .. } => {
            assert!(candidates.contains(&Target::Card(theirs)));
            assert!(
                !candidates.contains(&Target::Card(mine)),
                "nur Kreaturen des beschädigten Spielers"
            );
            d.submit(Choice::Targets(vec![Target::Card(theirs)]));
        }
        o => panic!("{o:?}"),
    }
    d.pass_all();
    assert_eq!(d.g().card(theirs).zone, None);
    assert_eq!(d.g().card(mine).zone, Some(Zone::Battlefield));
}

#[test]
fn targeted_player_ctrl_filter() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Swamp", P1, Zone::Battlefield, 2);
    let mine = put(&db, &mut g, "Hill Giant", P1, Zone::Battlefield);
    let theirs = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let s = put(&db, &mut g, "Purge Player B5", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(s, &[T2]);
    d.pass_all();
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Graveyard), 1);
    assert_eq!(d.g().card(mine).zone, Some(Zone::Battlefield));
    let _ = theirs;
}

#[test]
fn delayed_trigger_watches_remembered_creature_only() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Swamp", P1, Zone::Battlefield, 1);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 2);
    let watched = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let other = put(&db, &mut g, "Pinger B5", P2, Zone::Battlefield);
    let pact = put(&db, &mut g, "Death Pact B5", P1, Zone::Hand);
    let b1 = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let b2 = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(pact, &[Target::Card(watched)]);
    d.pass_all();
    let hand = d.g().player(P1).hand.len();
    d.cast(b1, &[Target::Card(other)]);
    d.pass_all();
    assert_eq!(
        d.g().player(P1).hand.len(),
        hand - 1,
        "andere Kreatur: nichts"
    );
    d.cast(b2, &[Target::Card(watched)]);
    d.pass_all();
    d.pass_all();
    assert_eq!(
        d.g().player(P1).hand.len(),
        hand - 2 + 2,
        "gemerkte Kreatur: 2 Karten"
    );
}

#[test]
fn same_name_relative_to_target() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 1);
    let b1 = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let b2 = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let giant = put(&db, &mut g, "Hill Giant", P1, Zone::Battlefield);
    let s = put(&db, &mut g, "Name Pump B5", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(s, &[Target::Card(b1)]);
    d.pass_all();
    assert_eq!(d.g().chars(b1).power, Some(3));
    assert_eq!(d.g().chars(b2).power, Some(4));
    assert_eq!(d.g().chars(giant).power, Some(3));
}

#[test]
fn discover_puts_card_into_hand_when_not_cast() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 3);
    let giant = put(&db, &mut g, "Hill Giant", P1, Zone::Library); // MV 4: zu teuer
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Library);
    let _ = giant;
    // Bibliothek oben: zuletzt gelegte Karte – Bären liegen oben, darunter der Riese.
    let s = put(&db, &mut g, "Discover B5", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(s, &[]);
    d.pass_all();
    match d.decision().clone() {
        Decision::Confirm { .. } => d.submit(Choice::No),
        o => panic!("{o:?}"),
    }
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Hand), 1);
    assert_eq!(d.g().card(bear).zone, None, "neues Objekt in der Hand");
}

#[test]
fn discover_may_cast_for_free() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 3);
    put(&db, &mut g, "Grizzly Bears", P1, Zone::Library);
    let s = put(&db, &mut g, "Discover B5", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(s, &[]);
    d.pass_all();
    d.submit(Choice::Yes);
    d.finish_casting(&[]);
    d.pass_all();
    assert_eq!(count(d.g(), "Grizzly Bears", Zone::Battlefield), 1);
}

#[test]
fn defined_parent_moves_spell_itself() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 1);
    let s = put(&db, &mut g, "Reapproach B5", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(s, &[]);
    d.pass_all();
    assert_eq!(d.g().player(P1).life, 22);
    assert_eq!(count(d.g(), "Reapproach B5", Zone::Library), 1);
    assert_eq!(count(d.g(), "Reapproach B5", Zone::Graveyard), 0);
}

#[test]
fn damage_map_and_damage_resolve() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 1);
    let s = put(&db, &mut g, "Twin Bolt B5", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(s, &[T2]);
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 18);
}

#[test]
fn transformed_trigger_fires_on_back_face() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 1);
    let f = put(&db, &mut g, "Flip B5", P1, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.act(Action::Activate(f, 0));
    d.finish_casting(&[]);
    d.pass_all();
    assert_eq!(d.g().chars(f).name, "Flipped B5");
    d.pass_all();
    assert_eq!(d.g().player(P1).life, 23);
}

#[test]
fn trigger_activation_limit_once_per_turn() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 1);
    put(&db, &mut g, "Once B5", P1, Zone::Battlefield);
    let s = put(&db, &mut g, "Two Soldiers B5", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(s, &[]);
    d.pass_all();
    d.pass_all();
    d.pass_all();
    assert_eq!(count(d.g(), "Soldier Token", Zone::Battlefield), 2);
    assert_eq!(d.g().player(P1).life, 21, "nur einmal je Zug");
}

#[test]
fn remove_all_abilities_drops_keywords_and_activated() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 1);
    let boa = put(&db, &mut g, "River Boa Lite", P2, Zone::Battlefield);
    let other = put(&db, &mut g, "Grizzly Bears", P2, Zone::Battlefield);
    let s = put(&db, &mut g, "Lose Ab B5", P1, Zone::Hand);
    assert_eq!(g.activated(boa).len(), 1);
    let mut d = Driver::new(g);
    d.cast(s, &[Target::Card(boa)]);
    d.pass_all();
    assert!(d.g().activated(boa).is_empty());
    assert!(d.g().chars(boa).lost_abilities);
    assert!(!d.g().chars(other).lost_abilities);
}

#[test]
fn remove_land_types_changes_intrinsic_mana() {
    let (db, mut g) = game_at(Step::Main1, 10);
    let dual = put(&db, &mut g, "Dual B5", P2, Zone::Battlefield);
    assert_eq!(g.activated(dual).len(), 2, "Plains und Island");
    put(&db, &mut g, "Moon B5", P1, Zone::Battlefield);
    let ch = g.chars(dual);
    assert!(ch.types.has_subtype("Mountain"));
    assert!(!ch.types.has_subtype("Island"));
    let abs = g.activated(dual);
    assert_eq!(abs.len(), 1);
    assert!(abs[0].description.contains("{R}"), "{}", abs[0].description);
    let forest = put(&db, &mut g, "Forest", P2, Zone::Battlefield);
    assert!(
        g.chars(forest).types.has_subtype("Forest"),
        "Standardland bleibt"
    );
}

#[test]
fn buyback_returns_spell_to_hand() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 4);
    let s = put(&db, &mut g, "Buyback Bolt B5", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.act(Action::Cast(s));
    match d.decision().clone() {
        Decision::Confirm { prompt, .. } => {
            assert!(prompt.contains("Rückkauf"));
            d.submit(Choice::Yes);
        }
        o => panic!("{o:?}"),
    }
    d.finish_casting(&[T2]);
    d.pass_all();
    assert_eq!(d.g().player(P2).life, 19);
    assert_eq!(count(d.g(), "Buyback Bolt B5", Zone::Hand), 1);
    assert_eq!(count(d.g(), "Mountain", Zone::Battlefield), 4);
    assert!(
        d.g()
            .battlefield
            .iter()
            .filter(|c| d.g().card(**c).tapped)
            .count()
            == 4,
        "1 + 3 bezahlt"
    );
}

#[test]
fn buyback_declined_goes_to_graveyard() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 4);
    let s = put(&db, &mut g, "Buyback Bolt B5", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.act(Action::Cast(s));
    d.submit(Choice::No);
    d.finish_casting(&[T2]);
    d.pass_all();
    assert_eq!(count(d.g(), "Buyback Bolt B5", Zone::Graveyard), 1);
}

#[test]
fn living_weapon_creates_equipped_germ() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Plains", P1, Zone::Battlefield, 1);
    let s = put(&db, &mut g, "Germ Blade B5", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(s, &[]);
    d.pass_all();
    d.pass_all();
    let germ = find(d.g(), "Phyrexian Germ Token", Zone::Battlefield).expect("Keimling");
    assert_eq!(d.g().chars(germ).power, Some(2));
    assert_eq!(d.g().chars(germ).toughness, Some(2));
    let blade = find(d.g(), "Germ Blade B5", Zone::Battlefield).unwrap();
    assert_eq!(d.g().card(blade).attached_to, Some(germ));
}

#[test]
fn clone_until_end_of_turn_reverts() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 1);
    let m = put(&db, &mut g, "Mimic B5", P1, Zone::Battlefield);
    let giant = put(&db, &mut g, "Hill Giant", P2, Zone::Battlefield);
    let mut d = Driver::new(g);
    d.act(Action::Activate(m, 0));
    d.finish_casting(&[Target::Card(giant)]);
    d.pass_all();
    assert_eq!(d.g().chars(m).name, "Hill Giant");
    d.run_until(|g, _| g.active == P2 && g.step == Step::Upkeep);
    assert_eq!(d.g().chars(m).name, "Mimic B5");
    assert_eq!(d.g().chars(m).power, Some(1));
}

#[test]
fn effect_ends_when_host_leaves_play() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 2);
    let host = put(&db, &mut g, "Host Pump B5", P1, Zone::Battlefield);
    let bear = put(&db, &mut g, "Grizzly Bears", P1, Zone::Battlefield);
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.act(Action::Activate(host, 0));
    d.finish_casting(&[]);
    d.pass_all();
    assert_eq!(d.g().chars(bear).power, Some(3));
    d.cast(bolt, &[Target::Card(host)]);
    d.pass_all();
    assert_eq!(
        d.g().chars(bear).power,
        Some(2),
        "Quelle weg → Effekt endet"
    );
}

#[test]
fn dig_change_num_any() {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Island", P1, Zone::Battlefield, 1);
    let s = put(&db, &mut g, "Dig Any B5", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(s, &[]);
    d.pass_all();
    match d.decision().clone() {
        Decision::ChooseCards {
            options, min, max, ..
        } => {
            assert_eq!((min, max), (0, 3));
            d.submit(Choice::Cards(options[..2].to_vec()));
        }
        o => panic!("{o:?}"),
    }
    assert_eq!(count(d.g(), "Island", Zone::Graveyard), 1);
}
