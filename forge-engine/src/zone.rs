//! Zonen (CR 400).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Zone {
    Library,
    Hand,
    Battlefield,
    Graveyard,
    Stack,
    Exile,
    Command,
}

impl Zone {
    /// Zone mit Besitzer-Liste pro Spieler (CR 400.2: Bibliothek, Hand,
    /// Friedhof sind spielereigen). Battlefield, Stack, Exile, Command sind
    /// gemeinsame Zonen.
    pub fn is_player_zone(self) -> bool {
        matches!(self, Zone::Library | Zone::Hand | Zone::Graveyard)
    }

    /// Verdeckte Zone (CR 400.2).
    pub fn is_hidden(self) -> bool {
        matches!(self, Zone::Library | Zone::Hand)
    }

    pub fn parse(s: &str) -> Option<Zone> {
        Some(match s.trim() {
            "Library" => Zone::Library,
            "Hand" => Zone::Hand,
            "Battlefield" => Zone::Battlefield,
            "Graveyard" => Zone::Graveyard,
            "Stack" => Zone::Stack,
            "Exile" => Zone::Exile,
            "Command" => Zone::Command,
            _ => return None,
        })
    }
}
