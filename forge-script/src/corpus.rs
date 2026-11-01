//! Laden des gesamten Kartenbestands aus `forge-gui/res/`.

use crate::card::{CardFace, CardScript, ScriptError};
use forge_types::TypeRegistry;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Pfad zu `forge-gui/res` relativ zum Workspace (`rust/`), überschreibbar
/// per Umgebungsvariable `FORGE_RES`.
pub fn default_res_dir() -> PathBuf {
    if let Ok(p) = std::env::var("FORGE_RES") {
        return PathBuf::from(p);
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../forge-gui/res")
}

pub fn load_type_registry(res: &Path) -> std::io::Result<TypeRegistry> {
    let text = std::fs::read_to_string(res.join("lists/TypeLists.txt"))?;
    Ok(TypeRegistry::parse(&text))
}

/// Ergebnis für eine Datei.
pub struct CorpusEntry {
    pub path: PathBuf,
    pub result: Result<CardScript, ScriptError>,
}

/// Alle `.txt`-Dateien unterhalb von `dir`, sortiert (deterministisch).
pub fn script_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d)? {
            let p = e?.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "txt") {
                out.push(p);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Parst alle Kartenskripte in `dir` (z. B. `res/cardsfolder` oder `res/tokenscripts`).
/// `CopyFaceFrom:`-Platzhalter werden anschließend über alle Dateien aufgelöst.
pub fn parse_dir(dir: &Path, registry: &TypeRegistry) -> std::io::Result<Vec<CorpusEntry>> {
    let mut entries: Vec<CorpusEntry> = script_files(dir)?
        .into_iter()
        .map(|path| {
            let bytes = std::fs::read(&path)?;
            let text = String::from_utf8_lossy(&bytes);
            let result = CardScript::parse(&text, registry);
            Ok(CorpusEntry { path, result })
        })
        .collect::<std::io::Result<_>>()?;

    let by_name: HashMap<String, CardFace> = entries
        .iter()
        .filter_map(|e| e.result.as_ref().ok())
        .filter_map(|c| c.faces[0].as_ref())
        .map(|f| (f.name.clone(), f.clone()))
        .collect();
    for e in &mut entries {
        if let Ok(card) = &mut e.result {
            let missing = card.resolve_copied_faces(|n| by_name.get(n));
            if let Some(name) = missing.into_iter().next() {
                e.result = Err(ScriptError::UnresolvedCopyFace(name));
            }
        }
    }
    Ok(entries)
}
