//! Alle Blocker (nicht nur den einzigen) mit Suchbegriff zählen.
//! `cargo run --release --example blockers -- Remembered`
use forge_engine::CardDb;
use std::collections::BTreeMap;
fn main() {
    let needle = std::env::args().nth(1).unwrap_or_default();
    let res = forge_script::corpus::default_res_dir();
    let pool = forge_script::formats::target_pool(&res).unwrap();
    let db = CardDb::load(&res).unwrap();
    let mut m: BTreeMap<String, usize> = BTreeMap::new();
    for name in pool.keys() {
        let Some(d) = db.get(name) else { continue };
        for u in &d.unsupported {
            if u.contains(needle.as_str()) {
                *m.entry(u.clone()).or_default() += 1;
            }
        }
    }
    let mut v: Vec<_> = m.into_iter().collect();
    v.sort_by_key(|e| std::cmp::Reverse(e.1));
    let limit = std::env::var("LIMIT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(40);
    for (r, c) in v.into_iter().take(limit) {
        println!("{c:5}  {r}");
    }
}
