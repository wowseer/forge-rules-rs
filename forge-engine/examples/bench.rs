use forge_engine::simple_ai::SimpleAi;
use forge_engine::{CardDb, Engine, GameRules, GameState, run_game};
use std::time::Instant;
fn main() {
    let res = forge_script::corpus::default_res_dir();
    let db = CardDb::load(&res).unwrap();
    let mut spells: Vec<_> = db
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
    let lands: Vec<_> = ["Plains", "Island", "Swamp", "Mountain", "Forest"]
        .iter()
        .map(|n| db.get(n).unwrap().clone())
        .collect();
    let arg = |i: usize, d: u64| std::env::args().nth(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (from, to) = (arg(1, 0), arg(2, 20));
    let min_secs: f64 = std::env::var("MIN_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    for seed in from..to {
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
        let (mut a, mut b) = (SimpleAi::new(seed), SimpleAi::new(seed + 1000));
        let t = Instant::now();
        run_game(&mut e, &mut [&mut a, &mut b], 50_000);
        if t.elapsed().as_secs_f64() < min_secs {
            continue;
        }
        println!(
            "seed {seed:2}: {:6.2}s turns {:3} decisions {:6} cards {:6} ",
            t.elapsed().as_secs_f64(),
            e.state().turn,
            e.history.len(),
            e.state().cards.len()
        );
    }
}
