//! Lasttest: viele Zufallspartien mit echten, unterstützten Karten aus
//! `cardsfolder`. Findet Abstürze und Endlosschleifen, prüft Invarianten.

use forge_engine::simple_ai::SimpleAi;
use forge_engine::state::Flow;
use forge_engine::{CardDb, Engine, GameRules, GameState, Zone, run_game_limited};
use std::sync::Arc;

fn invariants(g: &GameState) {
    // Jede ID steht in genau der Zone, die das Objekt angibt.
    let mut seen = std::collections::HashSet::new();
    let lists = g
        .players
        .iter()
        .flat_map(|p| {
            [
                (&p.library, Zone::Library),
                (&p.hand, Zone::Hand),
                (&p.graveyard, Zone::Graveyard),
            ]
        })
        .chain([
            (&g.battlefield, Zone::Battlefield),
            (&g.exile, Zone::Exile),
            (&g.command, Zone::Command),
        ]);
    for (list, z) in lists {
        for id in list {
            assert!(seen.insert(*id), "{id} doppelt");
            assert_eq!(g.card(*id).zone, Some(z), "{id} steht in {z:?}");
        }
    }
    for c in &g.cards {
        if let Some(z) = c.zone
            && z != Zone::Stack
        {
            assert!(seen.contains(&c.id), "{} ({z:?}) in keiner Liste", c.id);
        }
    }
}

#[test]
fn random_games_with_real_cards() {
    let res = forge_script::corpus::default_res_dir();
    if !res.exists() {
        return;
    }
    let db = CardDb::load(&res).unwrap();
    let mut spells: Vec<Arc<forge_engine::CardDefinition>> = db
        .iter()
        .filter(|d| d.is_supported() && !d.face(0).types.is_land())
        .filter(|d| {
            d.face(0)
                .mana_cost
                .as_ref()
                .is_some_and(|m| m.mana_value() <= 5 && !m.no_cost)
        })
        .cloned()
        .collect();
    spells.sort_by(|a, b| a.name().cmp(b.name()));
    spells.dedup_by(|a, b| a.name() == b.name());
    assert!(
        spells.len() > 500,
        "nur {} unterstützte Zauber",
        spells.len()
    );
    let lands: Vec<_> = ["Plains", "Island", "Swamp", "Mountain", "Forest"]
        .iter()
        .map(|n| db.get(n).unwrap().clone())
        .collect();

    let games = std::env::var("SOAK_GAMES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(if cfg!(debug_assertions) { 8 } else { 40 });
    let mut finished = 0;
    for seed in 0..games {
        let mut rng = forge_engine::rng::Rng::new(seed * 7919 + 1);
        let mut deck = || {
            let mut d: Vec<_> = (0..36)
                .map(|_| spells[rng.below(spells.len() as u64) as usize].clone())
                .collect();
            for i in 0..24 {
                d.push(lands[i % 5].clone());
            }
            d
        };
        let decks = vec![("A".to_string(), deck()), ("B".to_string(), deck())];
        let mut g = GameState::new(GameRules::default(), decks, seed, None);
        g.token_defs = db.token_map();
        let mut e = Engine::new(g);
        let mut a = SimpleAi::new(seed);
        let mut b = SimpleAi::new(seed + 1000);
        let result = run_game_limited(&mut e, &mut [&mut a, &mut b], 50_000, 300);
        invariants(e.state());
        if result.is_some() {
            finished += 1;
            assert_eq!(e.state().flow, Flow::GameOver);
        }
    }
    assert!(
        finished * 10 >= games * 9,
        "nur {finished}/{games} Partien beendet"
    );
}
