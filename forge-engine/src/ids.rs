//! Handles für Spielobjekte. Objekte referenzieren sich nur über IDs,
//! nie über Zeiger – dadurch ist der ganze Zustand `Clone` (Snapshots, KI).

use std::fmt;

/// Index in `GameState::cards`. Eine neue ID entsteht, wenn ein Objekt die
/// Zone wechselt (CR 400.7: es wird ein neues Objekt).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CardId(pub u32);

/// Index in `GameState::players` (Sitzreihenfolge = Zugreihenfolge).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlayerId(pub u8);

impl fmt::Display for CardId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl fmt::Display for PlayerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "P{}", self.0 + 1)
    }
}
