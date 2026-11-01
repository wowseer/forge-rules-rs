//! Grundtypen der Forge-Rules-Engine in Rust.
//!
//! Enthält nur reine Datentypen ohne Spielzustand: Farben, Manakosten und
//! Kartentypen. Siehe `rust/docs/` für die Architektur.

pub mod card_type;
pub mod color;
pub mod mana;

pub use card_type::{CardType, CoreType, SubtypeKind, Supertype, TypeRegistry};
pub use color::{Color, ColorSet};
pub use mana::{ManaCost, ManaParseError, ManaShard};
