//! Parser für Forge-Kartenskripte.
//!
//! Ebene 1 der Skriptverarbeitung: Dateien → [`CardScript`] mit Seiten,
//! Eigenschaften und klassifizierten (aber noch nicht typisierten)
//! Fähigkeiten. Die Übersetzung in typisierte Effekte geschieht in der
//! Engine. Siehe `rust/docs/02-kartenskripte.md`.

pub mod ability;
pub mod card;
pub mod corpus;
pub mod formats;
pub mod params;

pub use ability::{AbilityKind, ScriptAbility, ScriptReplacement, ScriptStatic, ScriptTrigger};
pub use card::{CardFace, CardScript, ScriptError, SplitMode, StatValue};
pub use params::ParamMap;
