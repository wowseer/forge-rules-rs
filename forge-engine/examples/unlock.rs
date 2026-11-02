//! Wie viele Karten würden unterstützt, wenn alle Blocker mit einem der Suchbegriffe
//! (getrennt durch `|`) wegfielen? Zusätzlich: Karten mit mindestens einem solchen Blocker.
//! `cargo run --release --example unlock -- "Chapter|Transformed"`
use forge_engine::CardDb;
fn main() {
    let arg = std::env::args().nth(1).expect("Suchbegriffe");
    let needles: Vec<&str> = arg.split('|').collect();
    let res = forge_script::corpus::default_res_dir();
    let pool = forge_script::formats::target_pool(&res).unwrap();
    let db = CardDb::load(&res).unwrap();
    let (mut any, mut all) = (0, 0);
    for name in pool.keys() {
        let Some(d) = db.get(name) else { continue };
        let hit = |u: &String| needles.iter().any(|n| u.contains(n));
        if d.unsupported.iter().any(hit) {
            any += 1;
            if d.unsupported.iter().all(hit) {
                all += 1;
            }
        }
    }
    println!("betroffen: {any}, würden frei: {all}");
}
