//! Kartenskript-Dateien aus `forge-gui/res/cardsfolder/`.
//!
//! Java-Vorbild: `forge.card.CardRules.Reader.parseLine` und `CardFace`.
//!
//! Aufbau einer Datei: Zeilen `Key:Value`. Mehrere Seiten werden durch
//! `ALTERNATE` (Seite 2) bzw. `SPECIALIZE:WHITE` … `SPECIALIZE:GREEN`
//! (Seiten 3–7) getrennt. `Variant:<Name>:<Zeile>` ordnet eine Zeile einer
//! funktionalen Variante zu (Universes-Within-Namen u. Ä.).

use crate::ability::{AbilityError, ScriptAbility, ScriptReplacement, ScriptStatic, ScriptTrigger};
use crate::params::ParamMap;
use forge_types::{CardType, ColorSet, ManaCost, ManaParseError, TypeRegistry};
use std::fmt;

/// Wie mehrere Seiten zusammengehören (`AlternateMode:`), Java `CardSplitType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SplitMode {
    None,
    Adventure,
    DoubleFaced,
    Flip,
    Meld,
    Modal,
    Omen,
    Prepare,
    Specialize,
    Split,
    Vengeance,
}

impl SplitMode {
    pub fn parse(s: &str) -> Option<SplitMode> {
        Some(match s.trim() {
            "Adventure" => SplitMode::Adventure,
            "DoubleFaced" | "Transform" => SplitMode::DoubleFaced,
            "Flip" => SplitMode::Flip,
            "Meld" => SplitMode::Meld,
            "Modal" => SplitMode::Modal,
            "Omen" => SplitMode::Omen,
            "Prepare" => SplitMode::Prepare,
            "Specialize" => SplitMode::Specialize,
            "Split" => SplitMode::Split,
            "Vengeance" => SplitMode::Vengeance,
            _ => return None,
        })
    }
}

/// Wert für Stärke, Widerstandskraft, Loyalität oder Verteidigung.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatValue {
    Fixed(i32),
    /// Enthält `*` (z. B. `*`, `1+*`, `7-*`); der eigentliche Wert kommt aus
    /// einer charakteristik-definierenden Fähigkeit (CR 604.3). Als Basiswert
    /// gilt der konstante Anteil (`*` = 0), wie in Java.
    Variable {
        raw: String,
        base: i32,
    },
}

impl StatValue {
    pub fn parse(s: &str) -> StatValue {
        let s = s.trim();
        if let Ok(n) = s.parse::<i32>() {
            return StatValue::Fixed(n);
        }
        let base = s
            .split(['+', '-'])
            .filter(|p| !p.contains('*'))
            .filter_map(|p| p.trim().parse::<i32>().ok())
            .next()
            .unwrap_or(0);
        StatValue::Variable {
            raw: s.to_string(),
            base,
        }
    }

    pub fn base(&self) -> i32 {
        match self {
            StatValue::Fixed(n) => *n,
            StatValue::Variable { base, .. } => *base,
        }
    }
}

impl fmt::Display for StatValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StatValue::Fixed(n) => write!(f, "{n}"),
            StatValue::Variable { raw, .. } => f.write_str(raw),
        }
    }
}

/// Eine Kartenseite mit allen Eigenschaften und rohen, aber bereits
/// klassifizierten Fähigkeiten.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CardFace {
    pub name: String,
    pub mana_cost: Option<ManaCost>,
    pub types: CardType,
    /// Explizite Farbe (`Colors:`), sonst aus den Manakosten abgeleitet.
    pub color_override: Option<ColorSet>,
    pub power: Option<StatValue>,
    pub toughness: Option<StatValue>,
    pub loyalty: Option<StatValue>,
    pub defense: Option<StatValue>,
    pub oracle: String,
    pub text: String,
    /// `K:`-Zeilen, roh (Keyword-Parsing folgt in `forge-engine`).
    pub keywords: Vec<String>,
    pub abilities: Vec<ScriptAbility>,
    pub triggers: Vec<ScriptTrigger>,
    pub statics: Vec<ScriptStatic>,
    pub replacements: Vec<ScriptReplacement>,
    /// SVars in Skriptreihenfolge. Werte können selbst Fähigkeiten sein
    /// (`DB$ …`) oder Ausdrücke (`Count$…`) – das entscheidet erst der Nutzer.
    pub svars: Vec<(String, String)>,
    pub deck_rules: Vec<String>,
    pub draft_actions: Vec<String>,
    pub flavor_name: Option<String>,
}

impl CardFace {
    pub fn svar(&self, name: &str) -> Option<&str> {
        self.svars
            .iter()
            .rev()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    /// Farbe dieser Seite (CR 202.2 / CR 105.2).
    pub fn color(&self) -> ColorSet {
        self.color_override
            .or_else(|| self.mana_cost.as_ref().map(ManaCost::colors))
            .unwrap_or(ColorSet::COLORLESS)
    }
}

/// Eine vollständige Kartendefinition (eine Datei).
#[derive(Debug, Clone, PartialEq)]
pub struct CardScript {
    pub split_mode: SplitMode,
    /// Index 0 = Hauptseite, 1 = `ALTERNATE`, 2–6 = Specialize WUBRG.
    pub faces: Vec<Option<CardFace>>,
    /// Funktionale Varianten: (Variantenname, Seitenindex, Seite).
    pub variants: Vec<(String, usize, CardFace)>,
    pub meld_with: Option<String>,
    pub partner_with: Option<String>,
    pub hand_life_modifier: Option<String>,
    /// Tokens, die irgendwo per `TokenScript$` erzeugt werden.
    pub tokens: Vec<String>,
    pub ai_hints: Vec<String>,
    pub deck_hints: Vec<(String, String)>,
    /// `CopyFaceFrom:` – Platzhalter, Seite wird von anderer Karte kopiert.
    pub copy_face_from: Vec<(usize, String)>,
}

impl CardScript {
    /// Hauptseite. Nach [`CardScript::resolve_copied_faces`] immer vorhanden.
    pub fn main(&self) -> &CardFace {
        self.faces[0]
            .as_ref()
            .expect("Hauptseite fehlt – CopyFaceFrom nicht aufgelöst")
    }

    /// Füllt `CopyFaceFrom:`-Platzhalter mit der Hauptseite der genannten Karte.
    /// Gibt die nicht auflösbaren Namen zurück.
    pub fn resolve_copied_faces<'a>(
        &mut self,
        lookup: impl Fn(&str) -> Option<&'a CardFace>,
    ) -> Vec<String> {
        let mut missing = Vec::new();
        for (idx, name) in std::mem::take(&mut self.copy_face_from) {
            match lookup(&name) {
                Some(face) => {
                    if self.faces.len() <= idx {
                        self.faces.resize(idx + 1, None);
                    }
                    self.faces[idx] = Some(face.clone());
                }
                None => missing.push(name),
            }
        }
        missing
    }

    pub fn name(&self) -> &str {
        &self.main().name
    }

    pub fn faces(&self) -> impl Iterator<Item = &CardFace> {
        self.faces.iter().flatten()
    }

    /// Gehört die Karte zu einem ausgeschlossenen Sonderformat (Typ-basiert)?
    pub fn is_out_of_scope_type(&self) -> bool {
        self.faces().any(|f| f.types.is_out_of_scope())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScriptError {
    MissingName,
    LineWithoutFace { line: usize, text: String },
    Mana { line: usize, err: ManaParseError },
    Colors { line: usize, text: String },
    AlternateMode { line: usize, text: String },
    Ability { line: usize, err: AbilityError },
    MalformedVariant { line: usize, text: String },
    UnresolvedCopyFace(String),
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScriptError::MissingName => f.write_str("Datei enthält keine Name:-Zeile"),
            ScriptError::LineWithoutFace { line, text } => {
                write!(f, "Zeile {line}: vor Name: ({text})")
            }
            ScriptError::Mana { line, err } => write!(f, "Zeile {line}: {err}"),
            ScriptError::Colors { line, text } => {
                write!(f, "Zeile {line}: unbekannte Farbe '{text}'")
            }
            ScriptError::AlternateMode { line, text } => {
                write!(f, "Zeile {line}: unbekannter AlternateMode '{text}'")
            }
            ScriptError::Ability { line, err } => write!(f, "Zeile {line}: {err}"),
            ScriptError::MalformedVariant { line, text } => {
                write!(f, "Zeile {line}: Variant ohne Namen ({text})")
            }
            ScriptError::UnresolvedCopyFace(n) => {
                write!(f, "CopyFaceFrom: Karte '{n}' nicht gefunden")
            }
        }
    }
}

impl std::error::Error for ScriptError {}

struct Reader<'r> {
    registry: &'r TypeRegistry,
    script: CardScript,
    cur: usize,
}

impl<'r> Reader<'r> {
    fn face(
        &mut self,
        idx: usize,
        line_no: usize,
        raw: &str,
    ) -> Result<&mut CardFace, ScriptError> {
        self.script.faces[idx]
            .as_mut()
            .ok_or_else(|| ScriptError::LineWithoutFace {
                line: line_no,
                text: raw.to_string(),
            })
    }

    fn line(
        &mut self,
        line_no: usize,
        raw: &str,
        variant: Option<(String, usize)>,
    ) -> Result<(), ScriptError> {
        let (key, value) = match raw.split_once(':') {
            Some((k, v)) if !k.is_empty() => (k, v.trim()),
            _ => (raw, ""),
        };

        if let Some(idx) = value.find("TokenScript$") {
            let rest = value[idx + "TokenScript$".len()..].trim();
            let end = rest.find('|').unwrap_or(rest.len());
            for t in rest[..end].trim().split(',') {
                let t = t.trim();
                if !t.is_empty() && !self.script.tokens.iter().any(|x| x == t) {
                    self.script.tokens.push(t.to_string());
                }
            }
        }

        // Kopf-Zeilen, die keine Seite brauchen.
        match key {
            "ALTERNATE" => {
                self.cur = 1;
                return Ok(());
            }
            k if k.starts_with("SPECIALIZE") => {
                self.cur = match value {
                    "WHITE" => 2,
                    "BLUE" => 3,
                    "BLACK" => 4,
                    "RED" => 5,
                    "GREEN" => 6,
                    _ => self.cur,
                };
                return Ok(());
            }
            "AlternateMode" => {
                self.script.split_mode =
                    SplitMode::parse(value).ok_or_else(|| ScriptError::AlternateMode {
                        line: line_no,
                        text: value.into(),
                    })?;
                return Ok(());
            }
            "Name" if variant.is_none() => {
                self.script.faces[self.cur] = Some(CardFace {
                    name: value.to_string(),
                    ..CardFace::default()
                });
                return Ok(());
            }
            "MeldPair" => {
                self.script.meld_with = Some(value.into());
                return Ok(());
            }
            "HandLifeModifier" => {
                self.script.hand_life_modifier = Some(value.into());
                return Ok(());
            }
            "AI" => {
                self.script.ai_hints.push(value.into());
                return Ok(());
            }
            "DeckHints" | "DeckNeeds" | "DeckHas" => {
                self.script.deck_hints.push((key.into(), value.into()));
                return Ok(());
            }
            "CopyFaceFrom" => {
                self.script.copy_face_from.push((self.cur, value.into()));
                return Ok(());
            }
            "Variant" => {
                let (name, rest) = value
                    .split_once(':')
                    .filter(|(n, _)| !n.is_empty())
                    .ok_or_else(|| ScriptError::MalformedVariant {
                        line: line_no,
                        text: value.into(),
                    })?;
                return self.line(line_no, rest, Some((name.to_string(), self.cur)));
            }
            "SETCOLORID" | "Lights" => return Ok(()), // nur für ausgeschlossene Formate/Anzeige
            _ => {}
        }

        if key == "K"
            && let Some(p) = value.strip_prefix("Partner with:")
        {
            self.script.partner_with = Some(p.to_string());
        }

        let registry = self.registry;
        let cur = self.cur;
        let face: &mut CardFace = match &variant {
            Some((name, idx)) => {
                if let Some(pos) = self
                    .script
                    .variants
                    .iter()
                    .position(|(n, i, _)| n == name && i == idx)
                {
                    &mut self.script.variants[pos].2
                } else {
                    self.script
                        .variants
                        .push((name.clone(), *idx, CardFace::default()));
                    &mut self.script.variants.last_mut().expect("eben eingefügt").2
                }
            }
            None => self.face(cur, line_no, raw)?,
        };

        match key {
            "Name" => face.name = value.into(),
            "ManaCost" => {
                face.mana_cost = Some(
                    ManaCost::parse(value)
                        .map_err(|err| ScriptError::Mana { line: line_no, err })?,
                )
            }
            "Types" => face.types = CardType::parse(value, registry),
            "Colors" => {
                face.color_override =
                    Some(
                        ColorSet::from_names(value).ok_or_else(|| ScriptError::Colors {
                            line: line_no,
                            text: value.into(),
                        })?,
                    )
            }
            "PT" => {
                if let Some((p, t)) = value.split_once('/') {
                    face.power = Some(StatValue::parse(p));
                    face.toughness = Some(StatValue::parse(t));
                }
            }
            "Loyalty" => face.loyalty = Some(StatValue::parse(value)),
            "Defense" => face.defense = Some(StatValue::parse(value)),
            "Oracle" => face.oracle = value.into(),
            "Text" => face.text = value.into(),
            "FlavorName" => face.flavor_name = Some(value.into()),
            "K" => face.keywords.push(value.into()),
            "A" => face.abilities.push(
                ScriptAbility::parse(ParamMap::parse(value))
                    .map_err(|err| ScriptError::Ability { line: line_no, err })?,
            ),
            "T" => face.triggers.push(
                ScriptTrigger::parse(ParamMap::parse(value))
                    .map_err(|err| ScriptError::Ability { line: line_no, err })?,
            ),
            "S" => face.statics.push(
                ScriptStatic::parse(ParamMap::parse(value))
                    .map_err(|err| ScriptError::Ability { line: line_no, err })?,
            ),
            "R" => face.replacements.push(
                ScriptReplacement::parse(ParamMap::parse(value))
                    .map_err(|err| ScriptError::Ability { line: line_no, err })?,
            ),
            "SVar" => {
                let (name, v) = value.split_once(':').unwrap_or((value, ""));
                face.svars.push((name.to_string(), v.to_string()));
            }
            "DeckRule" => face.deck_rules.push(value.into()),
            "Draft" => face.draft_actions.push(value.into()),
            _ => {} // unbekannte Schlüssel ignoriert Java ebenfalls
        }
        Ok(())
    }
}

impl CardScript {
    /// Parst den Inhalt einer Kartendatei.
    pub fn parse(text: &str, registry: &TypeRegistry) -> Result<CardScript, ScriptError> {
        let mut r = Reader {
            registry,
            script: CardScript {
                split_mode: SplitMode::None,
                faces: vec![None; 7],
                variants: Vec::new(),
                meld_with: None,
                partner_with: None,
                hand_life_modifier: None,
                tokens: Vec::new(),
                ai_hints: Vec::new(),
                deck_hints: Vec::new(),
                copy_face_from: Vec::new(),
            },
            cur: 0,
        };
        for (i, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            r.line(i + 1, line, None)?;
        }
        let mut s = r.script;
        if s.faces[0].is_none() && !s.copy_face_from.iter().any(|(i, _)| *i == 0) {
            return Err(ScriptError::MissingName);
        }
        while s.faces.len() > 1 && s.faces.last().is_some_and(Option::is_none) {
            s.faces.pop();
        }
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;

    const BOLT: &str = "Name:Lightning Bolt\nManaCost:R\nTypes:Instant\nA:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | SpellDescription$ CARDNAME deals 3 damage to any target.\nOracle:Lightning Bolt deals 3 damage to any target.\n";

    #[test]
    fn bolt() {
        let c = CardScript::parse(BOLT, &TypeRegistry::default()).unwrap();
        let f = c.main();
        assert_eq!(f.name, "Lightning Bolt");
        assert_eq!(f.mana_cost.as_ref().unwrap().mana_value(), 1);
        assert_eq!(f.abilities.len(), 1);
        assert_eq!(f.abilities[0].kind, AbilityKind::Spell);
        assert_eq!(f.abilities[0].api, "DealDamage");
    }

    #[test]
    fn dfc() {
        let txt = "Name:Front\nManaCost:1 G\nTypes:Creature Elf\nPT:*/1+*\nAlternateMode:DoubleFaced\nOracle:x\n\nALTERNATE\n\nName:Back\nManaCost:no cost\nColors:green\nTypes:Creature Elf\nPT:3/3\nOracle:y\n";
        let c = CardScript::parse(txt, &TypeRegistry::default()).unwrap();
        assert_eq!(c.split_mode, SplitMode::DoubleFaced);
        assert_eq!(c.faces().count(), 2);
        let back = c.faces[1].as_ref().unwrap();
        assert!(back.mana_cost.as_ref().unwrap().no_cost);
        assert!(!back.color().is_colorless());
        assert_eq!(c.main().toughness.as_ref().unwrap().base(), 1);
    }
}
