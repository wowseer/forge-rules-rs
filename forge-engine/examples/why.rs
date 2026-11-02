//! Gründe für nicht unterstützte Karten, deren Skript einen Suchbegriff enthält.
//! `cargo run --release --example why -- "Sacrifice"`
//! Mit `--all` als zweitem Argument: alle Gründe zählen (nicht nur einzige),
//! jeweils der innerste Teil in Klammern (z. B. der eigentliche Ausdruck).
use forge_engine::CardDb;
use std::collections::BTreeMap;
fn main() {
    let needle = std::env::args().nth(1).expect("Suchbegriff");
    let all = std::env::args().nth(2).is_some_and(|a| a == "--all");
    let res = forge_script::corpus::default_res_dir();
    let pool = forge_script::formats::target_pool(&res).unwrap();
    let db = CardDb::load(&res).unwrap();
    let mut reasons: BTreeMap<String, usize> = BTreeMap::new();
    let mut n = 0;
    for name in pool.keys() {
        let Some(d) = db.get(name) else { continue };
        if d.is_supported() || !format!("{:?}", d.script).contains(needle.as_str()) {
            continue;
        }
        n += 1;
        if all {
            let inner: std::collections::BTreeSet<String> = d
                .unsupported
                .iter()
                .map(|r| {
                    let r = r.trim_end_matches(')');
                    r.rsplit_once('(').map_or(r, |(_, i)| i).to_string()
                })
                .collect();
            for r in inner {
                *reasons.entry(r).or_default() += 1;
            }
        } else if d.unsupported.len() == 1 {
            *reasons.entry(d.unsupported[0].clone()).or_default() += 1;
        }
    }
    let mut v: Vec<_> = reasons.into_iter().collect();
    v.sort_by_key(|e| std::cmp::Reverse(e.1));
    println!(
        "{n} nicht unterstützte Karten mit '{needle}'; {}:",
        if all {
            "alle Gründe"
        } else {
            "einziger Grund"
        }
    );
    for (r, c) in v.into_iter().take(if all { 400 } else { 30 }) {
        println!("{c:5}  {r}");
    }
}
