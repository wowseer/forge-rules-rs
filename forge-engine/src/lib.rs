//! Regelkern der Forge-Engine in Rust.
//!
//! Architektur: siehe `rust/docs/01-architektur.md`.
//! Einstieg: [`GameState::new`] → [`Engine::new`] → [`Engine::advance`] /
//! [`Engine::submit`].

pub mod actions;
pub mod casting;
pub mod combat;
pub mod cost;
pub mod decision;
pub mod definition;
pub mod effects;
pub mod engine;
pub mod expr;
pub mod flow;
pub mod ids;
pub mod keyword;
pub mod mana_pool;
pub mod replacements;
pub mod rng;
pub mod sba;
pub mod simple_ai;
pub mod state;
pub mod statics;
pub mod targeting;
pub mod triggers;
pub mod turn;
pub mod valid;
pub mod zone;

pub use decision::{Action, Choice, Decision, PayAction, Target};
pub use definition::{CardDb, CardDefinition};
pub use engine::{Controller, Engine, Status, run_game};
pub use ids::{CardId, PlayerId};
pub use state::{Event, Flow, GameRules, GameState};
pub use turn::Step;
pub use zone::Zone;
