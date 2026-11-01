//! Zugstruktur (CR 500–514).
//!
//! Java-Vorbild: `forge.game.phase.PhaseType` / `PhaseHandler`.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Step {
    Untap,
    Upkeep,
    Draw,
    Main1,
    BeginCombat,
    DeclareAttackers,
    DeclareBlockers,
    FirstStrikeDamage,
    CombatDamage,
    EndCombat,
    Main2,
    End,
    Cleanup,
}

impl Step {
    pub const ALL: [Step; 13] = [
        Step::Untap,
        Step::Upkeep,
        Step::Draw,
        Step::Main1,
        Step::BeginCombat,
        Step::DeclareAttackers,
        Step::DeclareBlockers,
        Step::FirstStrikeDamage,
        Step::CombatDamage,
        Step::EndCombat,
        Step::Main2,
        Step::End,
        Step::Cleanup,
    ];

    /// Nächster Schritt im selben Zug; `None` nach Cleanup.
    pub fn next(self) -> Option<Step> {
        let i = Step::ALL.iter().position(|s| *s == self)?;
        Step::ALL.get(i + 1).copied()
    }

    pub fn is_main(self) -> bool {
        matches!(self, Step::Main1 | Step::Main2)
    }

    pub fn is_combat(self) -> bool {
        matches!(
            self,
            Step::BeginCombat
                | Step::DeclareAttackers
                | Step::DeclareBlockers
                | Step::FirstStrikeDamage
                | Step::CombatDamage
                | Step::EndCombat
        )
    }

    /// Schritte, in denen normalerweise niemand Priorität erhält
    /// (CR 502.4 Untap, CR 514.3 Cleanup).
    pub fn has_priority(self) -> bool {
        !matches!(self, Step::Untap | Step::Cleanup)
    }

    /// Name wie in Forge-Skripten (`Phase$ Upkeep`, `Phase$ End of Turn`).
    pub fn script_name(self) -> &'static str {
        match self {
            Step::Untap => "Untap",
            Step::Upkeep => "Upkeep",
            Step::Draw => "Draw",
            Step::Main1 => "Main1",
            Step::BeginCombat => "BeginCombat",
            Step::DeclareAttackers => "Declare Attackers",
            Step::DeclareBlockers => "Declare Blockers",
            Step::FirstStrikeDamage => "First Strike Damage",
            Step::CombatDamage => "Combat Damage",
            Step::EndCombat => "EndCombat",
            Step::Main2 => "Main2",
            Step::End => "End of Turn",
            Step::Cleanup => "Cleanup",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order() {
        assert_eq!(Step::Untap.next(), Some(Step::Upkeep));
        assert_eq!(Step::EndCombat.next(), Some(Step::Main2));
        assert_eq!(Step::Cleanup.next(), None);
        assert!(!Step::Untap.has_priority());
        assert!(Step::DeclareBlockers.is_combat());
    }
}
