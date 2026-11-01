//! Kompilierte Kartendefinitionen und Kartendatenbank.
//!
//! Eine [`CardDefinition`] ist unveränderlich und wird per `Arc` von allen
//! Spielobjekten derselben Karte geteilt. Sie enthält das geparste Skript und
//! die übersetzten Fähigkeiten. Alles, was die Engine noch nicht kann, landet
//! in [`CardDefinition::unsupported`] – solche Karten sind nicht spielbar.

use crate::effects::{CompileError, CompiledAbility, Effect, ManaProduce, compile};
use crate::keyword::Keyword;
use forge_script::{AbilityKind, CardFace, CardScript, ScriptAbility, SplitMode};
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
    /// Statische Fähigkeiten (`S:`).
    pub statics: Vec<Arc<crate::statics::CompiledStatic>>,
    /// Ersatzeffekte (`R:`).
    pub replacements: Vec<Arc<crate::replacements::CompiledReplacement>>,
    /// Schadens-Ersatzeffekte (`R:Event$ DamageDone`).
    pub damage_replacements: Vec<Arc<crate::replacements::DamageReplacement>>,
    /// `K:etbCounter`: (Markentyp, Anzahl) beim Ankommen.
    pub etb_counters: Vec<(String, crate::expr::Expr)>,
    /// `K:Enchant:<Filter>`: woran die Aura angelegt sein darf (CR 303.4).
    pub enchant: Option<crate::valid::ValidSpec>,
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
            conditions: Vec::new(),
            unless: None,
            planeswalker: false,
            activation_zone: crate::zone::Zone::Battlefield,
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
        statics: Vec::new(),
        replacements: Vec::new(),
        damage_replacements: Vec::new(),
        etb_counters: Vec::new(),
        enchant: None,
    };
    let mut equip: Vec<Arc<CompiledAbility>> = Vec::new();
    for k in &face.keywords {
        if let Some(spec) = k.strip_prefix("Enchant:") {
            match crate::valid::ValidSpec::parse(spec) {
                Ok(v) if !v.allows_players() => out.enchant = Some(v),
                _ => unsupported.push(format!("Keyword: {k}")),
            }
            continue;
        }
        if k.starts_with("Equip:") {
            match equip_ability(k) {
                Some(a) => equip.push(Arc::new(a)),
                None => unsupported.push(format!("Keyword: {k}")),
            }
            continue;
        }
        // CR 702.29: Cycling / Typecycling (Java CardFactoryUtil: AB$ aus der Hand)
        if let Some(script) = cycling_script(k) {
            match ScriptAbility::from_svar(&script)
                .and_then(Result::ok)
                .ok_or(())
                .and_then(|ab| compile(face, &ab).map_err(|_| ()))
            {
                Ok(a) => out.activated.push(Arc::new(a)),
                Err(()) => unsupported.push(format!("Keyword: {k}")),
            }
            continue;
        }
        if k.starts_with("etbCounter:") {
            match crate::replacements::parse_etb_counter(face, k) {
                Ok(c) => out.etb_counters.push(c),
                Err(e) => unsupported.push(format!("Keyword: {k} ({e})")),
            }
            continue;
        }
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
        // bleibende Karte mit eigener Spell-Fähigkeit (z. B. Bestow) – noch nicht unterstützt
        unsupported.push("bleibende Karte mit SP$-Fähigkeit".into());
    }
    // Aura-Zauber aus `K:Enchant` (CR 303.4a: Aura-Zauber hat ein Ziel)
    if let Some(valid) = &out.enchant {
        out.spell = Some(Arc::new(CompiledAbility {
            kind: AbilityKind::Spell,
            api: "Attach".into(),
            cost: None,
            target: Some(crate::effects::TargetSpec {
                valid: valid.clone(),
                min: 1,
                max: 1,
                prompt: None,
                zones: vec![crate::zone::Zone::Battlefield],
            }),
            effect: Effect::AuraSpell,
            sub: None,
            description: format!("Enchant {}", face.name),
            sorcery_speed: false,
            conditions: Vec::new(),
            unless: None,
            planeswalker: false,
            activation_zone: crate::zone::Zone::Battlefield,
        }));
    } else if face.types.has_subtype("Aura") {
        unsupported.push("Aura ohne Enchant-Keyword".into());
    }
    out.activated.extend(equip);
    for t in &face.triggers {
        match crate::triggers::compile_trigger(face, t) {
            Ok(c) => out.triggers.push(Arc::new(c)),
            Err(e) => unsupported.push(format!("Trigger: {} ({e})", t.mode)),
        }
    }
    for s in &face.statics {
        match crate::statics::compile_static_multi(face, s) {
            Ok(cs) => out.statics.extend(cs.into_iter().map(Arc::new)),
            Err(e) => unsupported.push(format!("Static: {} ({e})", s.mode)),
        }
    }
    for r in &face.replacements {
        if r.mode == "DamageDone" {
            match crate::replacements::compile_damage_replacement(face, r) {
                Ok(c) => out.damage_replacements.push(Arc::new(c)),
                Err(e) => unsupported.push(format!("Replacement: {} ({e})", r.mode)),
            }
            continue;
        }
        match crate::replacements::compile_replacement(face, r) {
            Ok(c) => out.replacements.push(Arc::new(c)),
            Err(e) => unsupported.push(format!("Replacement: {} ({e})", r.mode)),
        }
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
        // `*` ist nur mit charakteristik-definierender Fähigkeit spielbar (CR 604.3).
        let has_cda = out.statics.iter().any(|s| {
            matches!(s.as_ref(), crate::statics::CompiledStatic::Continuous(c) if c.characteristic_defining && c.has_layer7b())
        });
        if !has_cda {
            unsupported.push("variable Stärke/Widerstandskraft (*) ohne CDA".into());
        }
    }
    if face.types.has_core(CoreType::Planeswalker)
        && !matches!(face.loyalty, Some(forge_script::StatValue::Fixed(_)))
    {
        unsupported.push("Planeswalker ohne feste Loyalität".into());
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

    /// Alle in Token-Effekten verwendeten Skriptnamen (inkl. Trigger und Charm-Modi).
    pub fn token_refs(&self) -> Vec<String> {
        fn walk(ab: &CompiledAbility, out: &mut Vec<String>) {
            for a in ab.chain() {
                match &a.effect {
                    crate::effects::Effect::Token { scripts, .. } => {
                        out.extend(scripts.iter().cloned())
                    }
                    crate::effects::Effect::Charm(spec) => {
                        spec.choices.iter().for_each(|m| walk(m, out))
                    }
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        for f in self.faces.iter().flatten() {
            f.spell
                .iter()
                .chain(f.activated.iter())
                .for_each(|a| walk(a, &mut out));
            f.triggers.iter().for_each(|t| walk(&t.execute, &mut out));
        }
        out.sort();
        out.dedup();
        out
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
    /// Token-Definitionen nach Skriptname (Dateiname ohne `.txt`).
    tokens: Arc<HashMap<String, Arc<CardDefinition>>>,
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
        crate::valid::set_type_registry(reg.clone());
        let mut db = CardDb::default();
        let mut tokens = HashMap::new();
        for e in forge_script::corpus::parse_dir(&res.join("tokenscripts"), &reg)? {
            if let (Ok(script), Some(stem)) = (e.result, e.path.file_stem()) {
                tokens.insert(
                    stem.to_string_lossy().into_owned(),
                    Arc::new(CardDefinition::compile(script)),
                );
            }
        }
        db.tokens = Arc::new(tokens);
        for e in forge_script::corpus::parse_dir(&res.join("cardsfolder"), &reg)? {
            if let Ok(script) = e.result {
                let mut def = CardDefinition::compile(script);
                db.check_token_refs(&mut def);
                db.insert(def);
            }
        }
        Ok(db)
    }

    /// Token-Definitionen (für `GameState::token_defs`).
    pub fn token_map(&self) -> Arc<HashMap<String, Arc<CardDefinition>>> {
        self.tokens.clone()
    }

    pub fn token(&self, name: &str) -> Option<&Arc<CardDefinition>> {
        self.tokens.get(name)
    }

    /// Hilfsfunktion für Tests: Token-Skript unter Namen registrieren.
    pub fn add_token_script(
        &mut self,
        name: &str,
        text: &str,
    ) -> Result<Arc<CardDefinition>, forge_script::ScriptError> {
        let script = CardScript::parse(text, &forge_types::TypeRegistry::default())?;
        let def = Arc::new(CardDefinition::compile(script));
        Arc::make_mut(&mut self.tokens).insert(name.to_string(), def.clone());
        Ok(def)
    }

    /// Unbekannte oder nicht unterstützte Token machen die Karte nicht spielbar.
    pub fn check_token_refs(&self, def: &mut CardDefinition) {
        let mut missing = Vec::new();
        for name in def.token_refs() {
            match self.tokens.get(&name) {
                None => missing.push(format!("Token-Skript fehlt: {name}")),
                Some(t) if !t.is_supported() => missing.push(format!(
                    "Token nicht unterstützt: {name} ({})",
                    t.unsupported.join("; ")
                )),
                _ => {}
            }
        }
        def.unsupported.extend(missing);
        def.unsupported.sort();
        def.unsupported.dedup();
    }

    /// Hilfsfunktion für Tests: Karte direkt aus Skripttext.
    pub fn add_script(
        &mut self,
        text: &str,
    ) -> Result<Arc<CardDefinition>, forge_script::ScriptError> {
        let script = CardScript::parse(text, &forge_types::TypeRegistry::default())?;
        let mut def = CardDefinition::compile(script);
        self.check_token_refs(&mut def);
        Ok(self.insert(def))
    }
}

/// Fehler beim Kompilieren als Text (für Berichte).
pub fn describe(e: &CompileError) -> String {
    e.to_string()
}

/// `K:Equip:<Kosten>[:<Filter>[:<Text>]]` → „Kosten: An Ziel-Kreatur anlegen. Nur als Hexerei.“
/// (CR 702.6, Java `CardFactoryUtil` Equip)
fn equip_ability(kw: &str) -> Option<CompiledAbility> {
    let kw = match kw.find(":Flavor ") {
        Some(i) => &kw[..i],
        None => kw,
    };
    let k: Vec<&str> = kw.split(':').collect();
    if k.len() > 4 && !k[4].is_empty() {
        return None; // Sonderformen (ReduceCost, AlternateCost)
    }
    let cost = crate::cost::Cost::parse(k.get(1)?).ok()?;
    let valid = k
        .get(2)
        .filter(|v| !v.is_empty())
        .copied()
        .unwrap_or("Creature.YouCtrl");
    Some(CompiledAbility {
        kind: AbilityKind::Activated,
        api: "Attach".into(),
        cost: Some(cost),
        target: Some(crate::effects::TargetSpec {
            valid: crate::valid::ValidSpec::parse(valid).ok()?,
            min: 1,
            max: 1,
            prompt: None,
            zones: vec![crate::zone::Zone::Battlefield],
        }),
        effect: Effect::Attach {
            object: crate::effects::Defined::SelfCard,
        },
        sub: None,
        description: format!("Equip {}", k[1]),
        sorcery_speed: true,
        conditions: Vec::new(),
        unless: None,
        planeswalker: false,
        activation_zone: crate::zone::Zone::Battlefield,
    })
}

/// Übersetzt `Cycling:<Kosten>` / `TypeCycling:<Typ>:<Kosten>` in ein Fähigkeitsskript
/// (wie Java `CardFactoryUtil.addSpellAbility`).
fn cycling_script(k: &str) -> Option<String> {
    if let Some(cost) = k.strip_prefix("Cycling:") {
        return Some(format!(
            "AB$ Draw | Cost$ {cost} Discard<1/CARDNAME> | ActivationZone$ Hand | NumCards$ 1 | SpellDescription$ Cycling {cost}"
        ));
    }
    let rest = k.strip_prefix("TypeCycling:")?;
    let mut parts = rest.split(':');
    let ty = parts.next()?;
    let cost = parts.next()?;
    Some(format!(
        "AB$ ChangeZone | Cost$ {cost} Discard<1/CARDNAME> | ActivationZone$ Hand | Origin$ Library | Destination$ Hand | ChangeType$ {ty} | SpellDescription$ {ty}cycling {cost}"
    ))
}
