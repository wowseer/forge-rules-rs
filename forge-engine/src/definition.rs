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
    /// `K:Chapter:<N>:…`: höchste Kapitelnummer einer Saga (CR 714.2d).
    pub final_chapter: Option<u32>,
    /// `R:Event$ Untap` (enttappt nicht).
    pub untap_replacements: Vec<Arc<crate::replacements::UntapReplacement>>,
    /// „kann nicht neutralisiert werden“ (`R:Event$ Counter | Layer$ CantHappen`)
    pub cant_counter: Vec<Arc<crate::replacements::CantCounterReplacement>>,
    /// `K:ETBReplacement:Other:<SVar>[:Optional]`: beim Ankommen auszuführen
    /// (Wahl von Typ/Farb usw.); `true` = optional.
    pub etb_other: Vec<(Arc<CompiledAbility>, bool)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CardDefinition {
    pub script: CardScript,
    pub faces: Vec<Option<CompiledFace>>,
    pub unsupported: Vec<String>,
}

/// `Chapter:<N>:<SVar1>,…,<SVarN>` → (N, Trigger je Kapitel).
fn chapter_triggers(
    face: &CardFace,
    rest: &str,
) -> Result<(u32, Vec<crate::triggers::CompiledTrigger>), String> {
    let (n, abs) = rest.split_once(':').ok_or("Format")?;
    let n: u32 = n.trim().parse().map_err(|_| "Kapitelzahl")?;
    let abs: Vec<&str> = abs.split(',').map(str::trim).collect();
    if abs.len() != n as usize {
        return Err("Kapitelzahl ≠ Fähigkeiten".into());
    }
    let mut out = Vec::new();
    for (i, name) in abs.iter().enumerate() {
        let ch = i + 1;
        let st = forge_script::ScriptTrigger::parse(forge_script::ParamMap::parse(&format!(
            "Mode$ CounterAdded | ValidCard$ Card.Self | TriggerZones$ Battlefield | CounterType$ LORE | CounterAmount$ EQ{ch} | Execute$ {name} | TriggerDescription$ Chapter {ch}"
        )))
        .map_err(|e| format!("{e:?}"))?;
        let mut t = crate::triggers::compile_trigger(face, &st).map_err(|e| e.to_string())?;
        t.chapter = true;
        out.push(t);
    }
    Ok((n, out))
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
                amount: crate::effects::Amount::Fixed(1),
            },
            sub: None,
            description: format!("{{T}}: Add {{{}}}. ({t})", c.symbol()),
            sorcery_speed: false,
            conditions: Vec::new(),
            unless: None,
            planeswalker: false,
            activation_zone: crate::zone::Zone::Battlefield,
            cycling: false,
            activation_limit: None,
            game_activation_limit: None,
        })
    })
    .collect()
}

/// Filtervariablen (`cmcLEX`) müssen als auswertbare SVar der Karte vorliegen.
fn check_filter_vars(face: &CardFace, unsupported: &mut Vec<String>) {
    let mut texts: Vec<String> = Vec::new();
    texts.extend(face.svars.iter().map(|(_, v)| v.clone()));
    texts.extend(face.abilities.iter().map(|a| format!("{a:?}")));
    texts.extend(face.triggers.iter().map(|t| format!("{t:?}")));
    texts.extend(face.statics.iter().map(|s| format!("{s:?}")));
    texts.extend(face.replacements.iter().map(|r| format!("{r:?}")));
    let mut vars: Vec<String> = texts
        .iter()
        .flat_map(|t| crate::valid::filter_vars_in(t))
        .collect();
    vars.sort();
    vars.dedup();
    for v in vars {
        match face.svar(&v) {
            None => unsupported.push(format!("Filtervariable {v} ohne SVar")),
            Some(sv) => {
                if let Err(e) = crate::expr::Expr::parse_svar(face, sv) {
                    unsupported.push(format!("Filtervariable {v} ({e})"));
                }
            }
        }
    }
}

/// `ResultSVar$ <Name>` (RollDice u. a.) setzt zur Laufzeit einen Wert, den spätere
/// Fähigkeiten als `<Name>` lesen: fehlt die SVar, wird sie als Laufzeitvariable ergänzt.
fn with_runtime_svars(face: &CardFace) -> std::borrow::Cow<'_, CardFace> {
    let texts = face
        .svars
        .iter()
        .map(|(_, v)| v.clone())
        .chain(face.abilities.iter().map(|a| format!("{a:?}")))
        .chain(face.triggers.iter().map(|t| format!("{t:?}")))
        .collect::<Vec<_>>();
    let mut names: Vec<String> = Vec::new();
    for t in &texts {
        let mut rest = t.as_str();
        while let Some(i) = rest.find("ResultSVar") {
            rest = &rest[i + "ResultSVar".len()..];
            let name: String = rest
                .trim_start_matches(|c: char| !c.is_ascii_alphanumeric())
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .collect();
            if !name.is_empty() && face.svar(&name).is_none() && !names.contains(&name) {
                names.push(name);
            }
        }
    }
    if names.is_empty() {
        return std::borrow::Cow::Borrowed(face);
    }
    let mut f = face.clone();
    for n in names {
        f.svars.push((n.clone(), format!("Count$RuntimeVar.{n}")));
    }
    std::borrow::Cow::Owned(f)
}

fn compile_face(face: &CardFace, unsupported: &mut Vec<String>) -> CompiledFace {
    let face_owned = with_runtime_svars(face);
    let face: &CardFace = &face_owned;
    check_filter_vars(face, unsupported);
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
        final_chapter: None,
        untap_replacements: Vec::new(),
        cant_counter: Vec::new(),
        etb_other: Vec::new(),
    };
    let mut equip: Vec<Arc<CompiledAbility>> = Vec::new();
    for k in &face.keywords {
        if let Some(spec) = k.strip_prefix("Enchant:") {
            // `Enchant:<Filter>[:<Beschreibung>]`
            let spec = spec.split_once(':').map_or(spec, |(v, _)| v);
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
        // CR 702.21: Ward (Java CardFactoryUtil: BecomesTarget-Trigger + Counter unless)
        if let Some(cost) = k.strip_prefix("Ward:") {
            let mut f = face.clone();
            f.svars.push((
                "WardCounter".into(),
                format!(
                    "DB$ Counter | Defined$ TriggeredSourceSA | UnlessCost$ {cost} | UnlessPayer$ TriggeredSourceSAController"
                ),
            ));
            let st = forge_script::ScriptTrigger::parse(forge_script::ParamMap::parse(
                "Mode$ BecomesTarget | ValidSource$ SpellAbility.OppCtrl | ValidTarget$ Card.Self | TriggerZones$ Battlefield | Execute$ WardCounter | TriggerDescription$ Ward",
            ));
            match st
                .map_err(|_| ())
                .and_then(|st| crate::triggers::compile_trigger(&f, &st).map_err(|_| ()))
            {
                Ok(t) => out.triggers.push(Arc::new(t)),
                Err(()) => unsupported.push(format!("Keyword: {k}")),
            }
            continue;
        }
        // CR 702.83: Exalted (Java: Attacks-Trigger „allein“, +1/+1 bis Zugende)
        if k == "Exalted" {
            let mut f = face.clone();
            f.svars.push((
                "ExaltedPump".into(),
                "DB$ Pump | Defined$ TriggeredAttacker | NumAtt$ +1 | NumDef$ +1".into(),
            ));
            let st = forge_script::ScriptTrigger::parse(forge_script::ParamMap::parse(
                "Mode$ Attacks | ValidCard$ Creature.YouCtrl | Alone$ True | TriggerZones$ Battlefield | Execute$ ExaltedPump | TriggerDescription$ Exalted",
            ));
            match st
                .map_err(|_| ())
                .and_then(|st| crate::triggers::compile_trigger(&f, &st).map_err(|_| ()))
            {
                Ok(t) => out.triggers.push(Arc::new(t)),
                Err(()) => unsupported.push(format!("Keyword: {k}")),
            }
            out.keywords.push(Keyword::Exalted);
            continue;
        }
        // CR 702.108: Prowess (Java: SpellCast-Trigger mit Pump)
        if k == "Prowess" {
            let mut f = face.clone();
            f.svars.push((
                "ProwessPump".into(),
                "DB$ Pump | Defined$ Self | NumAtt$ +1 | NumDef$ +1".into(),
            ));
            let st = forge_script::ScriptTrigger::parse(forge_script::ParamMap::parse(
                "Mode$ SpellCast | ValidCard$ Card.nonCreature | ValidActivatingPlayer$ You | TriggerZones$ Battlefield | Execute$ ProwessPump | TriggerDescription$ Prowess",
            ));
            match st
                .map_err(|_| ())
                .and_then(|st| crate::triggers::compile_trigger(&f, &st).map_err(|_| ()))
            {
                Ok(t) => out.triggers.push(Arc::new(t)),
                Err(()) => unsupported.push(format!("Keyword: {k}")),
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
        // CR 714: Saga-Kapitel (Java CardFactoryUtil: CounterAdded-Trigger je Kapitel,
        // Lore-Marke beim Ankommen)
        if let Some(rest) = k.strip_prefix("Chapter:") {
            match chapter_triggers(face, rest) {
                Ok((n, ts)) => {
                    out.triggers.extend(ts.into_iter().map(Arc::new));
                    out.final_chapter = Some(out.final_chapter.unwrap_or(0).max(n));
                    if !out.etb_counters.iter().any(|(t, _)| t == "LORE") {
                        out.etb_counters
                            .push(("LORE".into(), crate::expr::Expr::Const(1)));
                    }
                }
                Err(e) => unsupported.push(format!("Keyword: Chapter ({e})")),
            }
            continue;
        }
        // Java CardFactoryUtil: ETBReplacement:Other → Ersatzeffekt „wie es ins Spiel kommt“
        if let Some(rest) = k
            .strip_prefix("ETBReplacement:Other:")
            .or_else(|| k.strip_prefix("ETBReplacement:Copy:"))
        {
            let mut it = rest.split(':');
            let name = it.next().unwrap_or("");
            let optional = match it.next() {
                None => Some(false),
                Some("Optional") => Some(true),
                Some(_) => None,
            };
            let ab = optional.and_then(|_| face.svar(name)).and_then(|sv| {
                ScriptAbility::from_svar(sv)
                    .and_then(Result::ok)
                    .map(|ab| compile(face, &ab))
            });
            match (ab, optional) {
                (Some(Ok(a)), Some(opt)) if a.chain().iter().all(|x| x.target.is_none()) => {
                    out.etb_other.push((Arc::new(a), opt));
                }
                (Some(Err(e)), _) => {
                    unsupported.push(format!("Keyword: ETBReplacement ({e})"));
                }
                _ => unsupported.push(format!("Keyword: {k}")),
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
            cycling: false,
            activation_limit: None,
            game_activation_limit: None,
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
        if r.mode == "Untap" {
            match crate::replacements::compile_untap_replacement(face, r) {
                Ok(c) => out.untap_replacements.push(Arc::new(c)),
                Err(e) => unsupported.push(format!("Replacement: {} ({e})", r.mode)),
            }
            continue;
        }
        if r.mode == "Counter" {
            match crate::replacements::compile_cant_counter_replacement(face, r) {
                Ok(c) => out.cant_counter.push(Arc::new(c)),
                Err(e) => unsupported.push(format!("Replacement: {} ({e})", r.mode)),
            }
            continue;
        }
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
        if !matches!(
            script.split_mode,
            SplitMode::None
                | SplitMode::Adventure
                | SplitMode::Modal
                | SplitMode::DoubleFaced
                | SplitMode::Split
        ) {
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

    /// Index der alternativ wirkbaren Seite (Adventure, modale doppelseitige Karte).
    pub fn alt_cast_face(&self) -> Option<usize> {
        matches!(
            self.script.split_mode,
            SplitMode::Adventure | SplitMode::Modal | SplitMode::Split
        )
        .then_some(1)
        .filter(|i| self.faces.get(*i).is_some_and(Option::is_some))
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

/// Kompilierte Fähigkeiten eines verdeckten Objekts (CR 708.2): keine, bzw. bei
/// Disguise Ward {2} (CR 702.168b).
pub(crate) fn face_down_compiled(disguise: bool) -> &'static CompiledFace {
    static PLAIN: std::sync::OnceLock<CompiledFace> = std::sync::OnceLock::new();
    static DISGUISED: std::sync::OnceLock<CompiledFace> = std::sync::OnceLock::new();
    let build = |text: &str| {
        let script = CardScript::parse(text, &forge_types::TypeRegistry::default())
            .expect("verdeckte Seite parst");
        let mut unsupported = Vec::new();
        compile_face(script.main(), &mut unsupported)
    };
    if disguise {
        DISGUISED.get_or_init(|| {
            build("Name:\nManaCost:no cost\nTypes:Creature\nPT:2/2\nK:Ward:2\nOracle:\n")
        })
    } else {
        PLAIN.get_or_init(|| build("Name:\nManaCost:no cost\nTypes:Creature\nPT:2/2\nOracle:\n"))
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
        cycling: false,
        activation_limit: None,
        game_activation_limit: None,
    })
}

/// Übersetzt `Cycling:<Kosten>` / `TypeCycling:<Typ>:<Kosten>` in ein Fähigkeitsskript
/// (wie Java `CardFactoryUtil.addSpellAbility`).
fn cycling_script(k: &str) -> Option<String> {
    // CR 702.122: Crew (Java CardFactoryUtil): Animate zum Artefakt-Kreatur bis Zugende
    if let Some(rest) = k.strip_prefix("Crew:") {
        let mut parts = rest.split(':');
        let n = parts.next()?.trim();
        n.parse::<u32>().ok()?;
        if parts.next().is_some() {
            return None;
        }
        return Some(format!(
            "AB$ Animate | Cost$ tapXType<Any/Creature.Other+withTotalPowerGE{n}> | Defined$ Self | Types$ Artifact,Creature | SpellDescription$ Crew {n}"
        ));
    }
    if let Some(cost) = k.strip_prefix("Cycling:") {
        return Some(format!(
            "AB$ Draw | Cost$ {cost} Discard<1/CARDNAME> | ActivationZone$ Hand | IsCycling$ True | NumCards$ 1 | SpellDescription$ Cycling {cost}"
        ));
    }
    let rest = k.strip_prefix("TypeCycling:")?;
    let mut parts = rest.split(':');
    let ty = parts.next()?;
    let cost = parts.next()?;
    Some(format!(
        "AB$ ChangeZone | Cost$ {cost} Discard<1/CARDNAME> | ActivationZone$ Hand | IsCycling$ True | Origin$ Library | Destination$ Hand | ChangeType$ {ty} | SpellDescription$ {ty}cycling {cost}"
    ))
}
