//! Gründe für nicht unterstützte Karten, deren Skript einen Suchbegriff enthält.
//! `cargo run --release --example why -- "Sacrifice"`
use forge_engine::CardDb;
use std::collections::BTreeMap;
fn main() {
    let needle = std::env::args().nth(1).expect("Suchbegriff");
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
        if d.unsupported.len() == 1 {
            *reasons.entry(d.unsupported[0].clone()).or_default() += 1;
        }
    }
    let mut v: Vec<_> = reasons.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    println!("{n} nicht unterstützte Karten mit '{needle}'; einziger Grund:");
    for (r, c) in v.into_iter().take(30) {
        println!("{c:5}  {r}");
    }
}
