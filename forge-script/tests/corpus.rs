//! Meilenstein Phase 1: Jede Kartendatei muss fehlerfrei parsen.

use forge_script::ScriptAbility;
use forge_script::corpus::{default_res_dir, load_type_registry, parse_dir};

#[test]
fn all_card_scripts_parse() {
    let res = default_res_dir();
    if !res.exists() {
        eprintln!("{} fehlt – Test übersprungen", res.display());
        return;
    }
    let reg = load_type_registry(&res).expect("TypeLists.txt");
    assert!(reg.len() > 300, "TypeLists.txt unvollständig geladen");

    for sub in ["cardsfolder", "tokenscripts"] {
        let entries = parse_dir(&res.join(sub), &reg).expect("Verzeichnis lesbar");
        assert!(!entries.is_empty());
        let mut errors = Vec::new();
        for e in &entries {
            match &e.result {
                Err(err) => errors.push(format!("{}: {err}", e.path.display())),
                Ok(card) => {
                    for face in card.faces() {
                        for (name, v) in &face.svars {
                            if let Some(Err(err)) = ScriptAbility::from_svar(v) {
                                errors.push(format!("{} SVar {name}: {err}", e.path.display()));
                            }
                        }
                    }
                }
            }
        }
        assert!(
            errors.is_empty(),
            "{} Fehler in {sub}:\n{}",
            errors.len(),
            errors.join("\n")
        );
    }
}
