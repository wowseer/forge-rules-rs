//! Kartentypen (CR 205): Supertypen, Kartentypen (hier „Core Types“) und Untertypen.
//!
//! Java-Vorbild: `forge.card.CardType` (forge-core). Bekannte Untertypen und
//! mehrwortige Typen („Time Lord“, „Serra's Realm“) stehen in
//! `forge-gui/res/lists/TypeLists.txt` und werden über [`TypeRegistry`] geladen.

use std::collections::{BTreeSet, HashMap};
use std::fmt;

/// Kartentypen (CR 205.2a). Sondertypen ausgeschlossener Formate
/// (Scheme, Plane, Phenomenon, Vanguard, Conspiracy, Stickers) werden
/// erkannt, damit solche Karten sauber aussortiert werden können.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CoreType {
    Kindred,
    Artifact,
    Battle,
    Conspiracy,
    Enchantment,
    Creature,
    Dungeon,
    Instant,
    Land,
    Phenomenon,
    Plane,
    Planeswalker,
    Scheme,
    Sorcery,
    Stickers,
    Vanguard,
}

impl CoreType {
    pub const ALL: [CoreType; 16] = [
        CoreType::Kindred,
        CoreType::Artifact,
        CoreType::Battle,
        CoreType::Conspiracy,
        CoreType::Enchantment,
        CoreType::Creature,
        CoreType::Dungeon,
        CoreType::Instant,
        CoreType::Land,
        CoreType::Phenomenon,
        CoreType::Plane,
        CoreType::Planeswalker,
        CoreType::Scheme,
        CoreType::Sorcery,
        CoreType::Stickers,
        CoreType::Vanguard,
    ];

    pub fn name(self) -> &'static str {
        match self {
            CoreType::Kindred => "Kindred",
            CoreType::Artifact => "Artifact",
            CoreType::Battle => "Battle",
            CoreType::Conspiracy => "Conspiracy",
            CoreType::Enchantment => "Enchantment",
            CoreType::Creature => "Creature",
            CoreType::Dungeon => "Dungeon",
            CoreType::Instant => "Instant",
            CoreType::Land => "Land",
            CoreType::Phenomenon => "Phenomenon",
            CoreType::Plane => "Plane",
            CoreType::Planeswalker => "Planeswalker",
            CoreType::Scheme => "Scheme",
            CoreType::Sorcery => "Sorcery",
            CoreType::Stickers => "Stickers",
            CoreType::Vanguard => "Vanguard",
        }
    }

    pub fn from_name(s: &str) -> Option<CoreType> {
        // "Tribal" ist der alte Name von Kindred.
        if s == "Tribal" {
            return Some(CoreType::Kindred);
        }
        CoreType::ALL.into_iter().find(|t| t.name() == s)
    }

    /// Bleibende Karte (CR 110.4)?
    pub fn is_permanent(self) -> bool {
        matches!(
            self,
            CoreType::Artifact
                | CoreType::Battle
                | CoreType::Enchantment
                | CoreType::Creature
                | CoreType::Land
                | CoreType::Planeswalker
        )
    }

    /// Gehört zu einem ausgeschlossenen Sonderformat (siehe CLAUDE.md).
    pub fn is_out_of_scope(self) -> bool {
        matches!(
            self,
            CoreType::Conspiracy
                | CoreType::Phenomenon
                | CoreType::Plane
                | CoreType::Scheme
                | CoreType::Stickers
                | CoreType::Vanguard
        )
    }
}

/// Supertypen (CR 205.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Supertype {
    Basic,
    Elite,
    Host,
    Legendary,
    Snow,
    Ongoing,
    World,
}

impl Supertype {
    pub const ALL: [Supertype; 7] = [
        Supertype::Basic,
        Supertype::Elite,
        Supertype::Host,
        Supertype::Legendary,
        Supertype::Snow,
        Supertype::Ongoing,
        Supertype::World,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Supertype::Basic => "Basic",
            Supertype::Elite => "Elite",
            Supertype::Host => "Host",
            Supertype::Legendary => "Legendary",
            Supertype::Snow => "Snow",
            Supertype::Ongoing => "Ongoing",
            Supertype::World => "World",
        }
    }

    pub fn from_name(s: &str) -> Option<Supertype> {
        Supertype::ALL.into_iter().find(|t| t.name() == s)
    }
}

/// Zu welcher Typfamilie ein Untertyp gehört (CR 205.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubtypeKind {
    BasicLand,
    Land,
    Creature,
    Spell,
    Enchantment,
    Artifact,
    Planeswalker,
    Dungeon,
    Battle,
    Planar,
}

/// Verzeichnis der bekannten Untertypen, geladen aus `TypeLists.txt`.
#[derive(Debug, Clone, Default)]
pub struct TypeRegistry {
    kinds: HashMap<String, SubtypeKind>,
    plurals: HashMap<String, String>,
    /// Mehrwortige Typen, längste zuerst (für gieriges Matching).
    multiword: Vec<String>,
}

impl TypeRegistry {
    /// Liest das Format von `forge-gui/res/lists/TypeLists.txt`:
    /// Abschnitte `[CreatureTypes]` usw., Zeilen `Name` oder `Name:Plural`.
    pub fn parse(text: &str) -> TypeRegistry {
        let mut reg = TypeRegistry::default();
        let mut section: Option<SubtypeKind> = None;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                section = match name {
                    "BasicTypes" => Some(SubtypeKind::BasicLand),
                    "LandTypes" => Some(SubtypeKind::Land),
                    "CreatureTypes" => Some(SubtypeKind::Creature),
                    "SpellTypes" => Some(SubtypeKind::Spell),
                    "EnchantmentTypes" => Some(SubtypeKind::Enchantment),
                    "ArtifactTypes" => Some(SubtypeKind::Artifact),
                    "WalkerTypes" => Some(SubtypeKind::Planeswalker),
                    "DungeonTypes" => Some(SubtypeKind::Dungeon),
                    "BattleTypes" => Some(SubtypeKind::Battle),
                    "PlanarTypes" => Some(SubtypeKind::Planar),
                    _ => None,
                };
                continue;
            }
            let Some(kind) = section else { continue };
            let (name, plural) = match line.split_once(':') {
                Some((n, p)) => (n.trim(), Some(p.trim())),
                None => (line, None),
            };
            if reg.kinds.contains_key(name) {
                continue;
            }
            reg.kinds.insert(name.to_string(), kind);
            if let Some(p) = plural {
                reg.plurals.insert(name.to_string(), p.to_string());
            }
            if name.contains(' ') {
                reg.multiword.push(name.to_string());
            }
        }
        reg.multiword.sort_by_key(|s| std::cmp::Reverse(s.len()));
        reg
    }

    pub fn kind_of(&self, subtype: &str) -> Option<SubtypeKind> {
        self.kinds.get(subtype).copied()
    }

    pub fn plural_of<'a>(&'a self, subtype: &'a str) -> &'a str {
        self.plurals
            .get(subtype)
            .map(String::as_str)
            .unwrap_or(subtype)
    }

    pub fn creature_types(&self) -> impl Iterator<Item = &str> {
        self.kinds
            .iter()
            .filter(|(_, k)| **k == SubtypeKind::Creature)
            .map(|(n, _)| n.as_str())
    }

    pub fn len(&self) -> usize {
        self.kinds.len()
    }

    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }
}

/// Typzeile eines Objekts.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CardType {
    pub supertypes: BTreeSet<Supertype>,
    pub core_types: BTreeSet<CoreType>,
    /// Untertypen in Skriptreihenfolge, ohne Duplikate.
    pub subtypes: Vec<String>,
}

impl CardType {
    /// Parst eine `Types:`-Zeile wie `Legendary Creature Human Wizard`.
    /// Unbekannte Wörter werden – wie in Java – als Untertyp übernommen.
    pub fn parse(text: &str, registry: &TypeRegistry) -> CardType {
        let mut t = CardType::default();
        let mut rest = text.trim();
        while !rest.is_empty() {
            let word = registry
                .multiword
                .iter()
                .find(|m| {
                    rest.starts_with(m.as_str())
                        && rest[m.len()..].chars().next().is_none_or(|c| c == ' ')
                })
                .map(String::as_str)
                .unwrap_or_else(|| rest.split(' ').next().unwrap_or(rest));
            t.add(word);
            rest = rest[word.len()..].trim_start();
        }
        t
    }

    pub fn add(&mut self, word: &str) {
        if let Some(c) = CoreType::from_name(word) {
            self.core_types.insert(c);
        } else if let Some(s) = Supertype::from_name(word) {
            self.supertypes.insert(s);
        } else if !word.is_empty() && !self.subtypes.iter().any(|s| s == word) {
            self.subtypes.push(word.to_string());
        }
    }

    pub fn has_core(&self, c: CoreType) -> bool {
        self.core_types.contains(&c)
    }

    pub fn has_super(&self, s: Supertype) -> bool {
        self.supertypes.contains(&s)
    }

    pub fn has_subtype(&self, s: &str) -> bool {
        self.subtypes.iter().any(|x| x == s)
    }

    pub fn is_creature(&self) -> bool {
        self.has_core(CoreType::Creature)
    }

    pub fn is_land(&self) -> bool {
        self.has_core(CoreType::Land)
    }

    pub fn is_instant_or_sorcery(&self) -> bool {
        self.has_core(CoreType::Instant) || self.has_core(CoreType::Sorcery)
    }

    pub fn is_permanent(&self) -> bool {
        self.core_types.iter().any(|c| c.is_permanent())
    }

    pub fn is_legendary(&self) -> bool {
        self.has_super(Supertype::Legendary)
    }

    pub fn is_basic_land(&self) -> bool {
        self.is_land() && self.has_super(Supertype::Basic)
    }

    pub fn is_out_of_scope(&self) -> bool {
        self.core_types.iter().any(|c| c.is_out_of_scope())
    }
}

impl fmt::Display for CardType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut words: Vec<&str> = Vec::new();
        words.extend(self.supertypes.iter().map(|s| s.name()));
        words.extend(self.core_types.iter().map(|c| c.name()));
        let head = words.join(" ");
        if self.subtypes.is_empty() {
            f.write_str(&head)
        } else {
            write!(f, "{head} — {}", self.subtypes.join(" "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg() -> TypeRegistry {
        TypeRegistry::parse(
            "[BasicTypes]\nPlains\nIsland:Islands\n[CreatureTypes]\nHuman:Humans\nTime Lord:Time Lords\nWizard\n[EnchantmentTypes]\nAura\n",
        )
    }

    #[test]
    fn parse_typeline() {
        let t = CardType::parse("Legendary Creature Human Wizard", &reg());
        assert!(t.is_legendary() && t.is_creature());
        assert_eq!(t.subtypes, vec!["Human", "Wizard"]);
        assert_eq!(t.to_string(), "Legendary Creature — Human Wizard");
    }

    #[test]
    fn multiword_and_unknown() {
        let t = CardType::parse("Creature Time Lord Doctor", &reg());
        assert_eq!(t.subtypes, vec!["Time Lord", "Doctor"]);
        let b = CardType::parse("Basic Snow Land Island", &reg());
        assert!(b.is_basic_land() && b.has_super(Supertype::Snow));
        assert_eq!(reg().kind_of("Island"), Some(SubtypeKind::BasicLand));
        assert_eq!(reg().plural_of("Human"), "Humans");
    }

    #[test]
    fn scope() {
        assert!(CardType::parse("Scheme", &reg()).is_out_of_scope());
        assert!(!CardType::parse("Kindred Instant Elf", &reg()).is_out_of_scope());
        assert!(CardType::parse("Tribal Sorcery", &reg()).has_core(CoreType::Kindred));
    }
}
