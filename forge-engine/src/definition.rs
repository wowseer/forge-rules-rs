//! Kompilierte Kartendefinitionen und Kartendatenbank.
//!
//! Eine [`CardDefinition`] ist unveränderlich und wird per `Arc` von allen
//! Spielobjekten derselben Karte geteilt. Sie enthält das geparste Skript und
//! die übersetzten Fähigkeiten. Alles, was die Engine noch nicht kann, landet
//! in [`CardDefinition::unsupported`] – solche Karten sind nicht spielbar.

use crate::effects::{CompileError, CompiledAbility, Effect, ManaProduce, compile};
use crate::keyword::Keyword;
use forge_script::{AbilityKind, CardFace, CardScript, SplitMode};
use forge_types::{Color, CoreType};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledFace {
    pub keywords: Vec<Keyword>,
    /// `SP$`-Fähigkeit (Spontanzauber/Hexerei; bei bleibenden Karten selten).
    pub spell: Option<Arc<CompiledAbility>>,
    /// Aktivierte Fähigkeiten inkl. Mana-Fähigkeiten.
    pub activated: Vec<Arc<CompiledAbility>>,
    /// Ausgelöste Fähigkeiten (`T:`).
    pub triggers: Vec<Arc<crate::triggers::CompiledTrigger>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CardDefinition {
    pub script: CardScript,
    pub faces: Vec<Option<CompiledFace>>,
    pub unsupported: Vec<String>,
}

fn basic_land_mana(face: &CardFace) -> Vec<Arc<CompiledAbility>> {
    // CR 305.6: Standardlandtypen haben eine intrinsische Mana-Fähigkeit.
    [
        ("Plains", Color::White),
        ("Island", Color::Blue),
        ("Swamp", Color::Black),
        ("Mountain", Color::Red),
        ("Forest", Color::Green),
    ]
    .into_iter()
    .filter(|(t, _)| face.types.has_subtype(t))
    .map(|(t, c)| {
        Arc::new(CompiledAbility {
            kind: AbilityKind::Activated,
            api: "Mana".into(),
            cost: Some(crate::cost::Cost {
                mana: None,
                parts: vec![crate::cost::CostPart::Tap],
            }),
            target: None,
            effect: Effect::AddMana {
                produce: ManaProduce::Fixed(vec![Some(c)]),
                amount: 1,
            },
            sub: None,
            description: format!("{{T}}: Add {{{}}}. ({t})", c.symbol()),
            sorcery_speed: false,
        })
    })
    .collect()
}

fn compile_face(face: &CardFace, unsupported: &mut Vec<String>) -> CompiledFace {
    let mut out = CompiledFace {
        keywords: Vec::new(),
        spell: None,
        activated: basic_land_mana(face),
        triggers: Vec::new(),
    };
    for k in &face.keywords {
        let kw = Keyword::parse(k);
        if !kw.is_supported() {
            unsupported.push(format!("Keyword: {k}"));
        }
        out.keywords.push(kw);
    }
    for ab in &face.abilities {
        match compile(face, ab) {
            Ok(c) => match c.kind {
                AbilityKind::Spell if out.spell.is_none() => out.spell = Some(Arc::new(c)),
                AbilityKind::Spell => {
                    unsupported.push("mehrere SP$-Fähigkeiten (Modal/Charm)".into())
                }
                AbilityKind::Activated => out.activated.push(Arc::new(c)),
                _ => unsupported.push(format!("Fähigkeitsart {:?}", c.kind)),
            },
            Err(e) => unsupported.push(e.to_string()),
        }
    }
    let is_spell_card = face.types.is_instant_or_sorcery();
    if is_spell_card && out.spell.is_none() && face.abilities.is_empty() {
        unsupported.push("Spontanzauber/Hexerei ohne SP$".into());
    }
    if !is_spell_card && out.spell.is_some() {
        // Auren (SP$ Attach) u. ä. – bleibende Karte mit eigener Spell-Fähigkeit
        unsupported.push("bleibende Karte mit SP$-Fähigkeit".into());
    }
    for t in &face.triggers {
        match crate::triggers::compile_trigger(face, t) {
            Ok(c) => out.triggers.push(Arc::new(c)),
            Err(e) => unsupported.push(format!("Trigger: {} ({e})", t.mode)),
        }
    }
    for s in &face.statics {
        unsupported.push(format!("Static: {}", s.mode));
    }
    for r in &face.replacements {
        unsupported.push(format!("Replacement: {}", r.mode));
    }
    if face
        .power
        .as_ref()
        .is_some_and(|p| !matches!(p, forge_script::StatValue::Fixed(_)))
        || face
            .toughness
            .as_ref()
            .is_some_and(|p| !matches!(p, forge_script::StatValue::Fixed(_)))
    {
        unsupported.push("variable Stärke/Widerstandskraft (*)".into());
    }
    if face.types.has_core(CoreType::Planeswalker) {
        unsupported.push("Planeswalker".into());
    }
    if face.types.has_core(CoreType::Battle) {
        unsupported.push("Battle".into());
    }
    out
}

impl CardDefinition {
    pub fn compile(script: CardScript) -> CardDefinition {
        let mut unsupported = Vec::new();
        if script.split_mode != SplitMode::None {
            unsupported.push(format!("mehrseitige Karte ({:?})", script.split_mode));
        }
        if script.is_out_of_scope_type() {
            unsupported.push("Sonderformat (außerhalb des Scopes)".into());
        }
        let faces = script
            .faces
            .iter()
            .map(|f| f.as_ref().map(|f| compile_face(f, &mut unsupported)))
            .collect();
        unsupported.sort();
        unsupported.dedup();
        CardDefinition {
            script,
            faces,
            unsupported,
        }
    }

    pub fn is_supported(&self) -> bool {
        self.unsupported.is_empty()
    }

    pub fn name(&self) -> &str {
        self.script.name()
    }

    pub fn face(&self, idx: usize) -> &CardFace {
        self.script.faces[idx].as_ref().expect("Seite existiert")
    }

    pub fn compiled(&self, idx: usize) -> &CompiledFace {
        self.faces[idx].as_ref().expect("Seite existiert")
    }
}

/// Alle bekannten Karten, nach Namen.
#[derive(Debug, Clone, Default)]
pub struct CardDb {
    by_name: HashMap<String, Arc<CardDefinition>>,
}

impl CardDb {
    pub fn insert(&mut self, def: CardDefinition) -> Arc<CardDefinition> {
        let a = Arc::new(def);
        self.by_name.insert(a.name().to_string(), a.clone());
        // Split-/Modal-/Adventure-Karten heißen in Editionen „A // B“.
        if let (Some(Some(f0)), Some(Some(f1))) = (a.script.faces.first(), a.script.faces.get(1)) {
            self.by_name
                .entry(format!("{} // {}", f0.name, f1.name))
                .or_insert_with(|| a.clone());
        }
        a
    }

    pub fn get(&self, name: &str) -> Option<&Arc<CardDefinition>> {
        self.by_name.get(name)
    }

    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Arc<CardDefinition>> {
        self.by_name.values()
    }

    /// Lädt `cardsfolder` aus dem Forge-Ressourcenverzeichnis.
    pub fn load(res: &std::path::Path) -> std::io::Result<CardDb> {
        let reg = forge_script::corpus::load_type_registry(res)?;
        let mut db = CardDb::default();
        for e in forge_script::corpus::parse_dir(&res.join("cardsfolder"), &reg)? {
            if let Ok(script) = e.result {
                db.insert(CardDefinition::compile(script));
            }
        }
        Ok(db)
    }

    /// Hilfsfunktion für Tests: Karte direkt aus Skripttext.
    pub fn add_script(
        &mut self,
        text: &str,
    ) -> Result<Arc<CardDefinition>, forge_script::ScriptError> {
        let script = CardScript::parse(text, &forge_types::TypeRegistry::default())?;
        Ok(self.insert(CardDefinition::compile(script)))
    }
}

/// Fehler beim Kompilieren als Text (für Berichte).
pub fn describe(e: &CompileError) -> String {
    e.to_string()
}
