//! Zeigt die SVar-Definitionen hinter einer Fehlermeldung (z. B. "NumDmg$ X").
//! `cargo run --release --example svars -- "DealDamage: NumDmg$ X"`
use forge_engine::CardDb;
use std::collections::BTreeMap;
fn main() {
    let needle = std::env::args().nth(1).expect("Fehlertext");
    let svar = needle.rsplit(' ').next().unwrap_or("X").to_string();
    let res = forge_script::corpus::default_res_dir();
    let pool = forge_script::formats::target_pool(&res).unwrap();
    let db = CardDb::load(&res).unwrap();
    let mut forms: BTreeMap<String, usize> = BTreeMap::new();
    for name in pool.keys() {
        let Some(d) = db.get(name) else { continue };
        if d.unsupported.len() != 1 || !d.unsupported[0].contains(needle.as_str()) {
            continue;
        }
        let v = d.face(0).svar(&svar).unwrap_or("<fehlt>").to_string();
        // Zahlen/Filter grob vereinheitlichen
        let key = v.split('/').next().unwrap_or("").to_string();
        *forms.entry(key).or_default() += 1;
    }
    let mut v: Vec<_> = forms.into_iter().collect();
    v.sort_by_key(|e| std::cmp::Reverse(e.1));
    for (f, c) in v.into_iter().take(40) {
        println!("{c:4}  {f}");
    }
}
