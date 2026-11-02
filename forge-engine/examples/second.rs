//! Für Karten mit genau zwei Blockern, von denen einer den Suchbegriff enthält:
//! Häufigkeit des jeweils anderen Blockers.
//! `cargo run --release --example second -- "DoubleFaced"`
use forge_engine::CardDb;
use std::collections::BTreeMap;
fn main() {
    let needle = std::env::args().nth(1).expect("Suchbegriff");
    let res = forge_script::corpus::default_res_dir();
    let pool = forge_script::formats::target_pool(&res).unwrap();
    let db = CardDb::load(&res).unwrap();
    let (mut only, mut m) = (0, BTreeMap::<String, usize>::new());
    for name in pool.keys() {
        let Some(d) = db.get(name) else { continue };
        let u = &d.unsupported;
        if !u.iter().any(|x| x.contains(needle.as_str())) {
            continue;
        }
        match u.len() {
            1 => only += 1,
            2 => {
                let other = u.iter().find(|x| !x.contains(needle.as_str())).unwrap();
                *m.entry(other.clone()).or_default() += 1;
            }
            _ => {}
        }
    }
    println!("nur dieser Blocker: {only}");
    let mut v: Vec<_> = m.into_iter().collect();
    v.sort_by_key(|e| std::cmp::Reverse(e.1));
    for (r, c) in v.into_iter().take(25) {
        println!("{c:5}  {r}");
    }
}
