//! Unterstützungsstatus einzelner Karten: `cargo run --release --example card -- "Fling" "Bone Splinters"`
use forge_engine::CardDb;
fn main() {
    let res = forge_script::corpus::default_res_dir();
    let db = CardDb::load(&res).unwrap();
    for name in std::env::args().skip(1) {
        match db.get(&name) {
            Some(d) => println!("{name}: {:?}", d.unsupported),
            None => println!("{name}: unbekannt"),
        }
    }
}
