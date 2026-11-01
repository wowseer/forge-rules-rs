//! Treiber der Engine: Zustandsautomat mit Wiederaufsetzpunkten.
//!
//! Funktionsweise: `Engine` hält den letzten stabilen Zustand
//! (Checkpoint) und die Antworten, die seitdem gegeben wurden. Ein atomarer
//! Schritt ([`GameState::step_once`]) läuft immer auf einer Kopie des
//! Checkpoints. Braucht er eine Entscheidung, die noch nicht beantwortet
//! ist, wird die Kopie verworfen und die [`Decision`] zurückgegeben. Mit der
//! Antwort läuft derselbe Schritt erneut – deterministisch bis zur selben
//! Stelle – und kommt diesmal weiter. Ist der Schritt fertig, wird er zum
//! neuen Checkpoint.
//!
//! Vorteile: keine Callbacks/Threads, Abbrechen (`Cancel`) ist trivial
//! (Kopie verwerfen), jede Partie ist aus Seed + Antwortfolge reproduzierbar.

use crate::decision::{Choice, Decision, InvalidChoice};
use crate::ids::PlayerId;
use crate::state::{Choices, Flow, GameState, Interrupt};

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    NeedsDecision(Decision),
    /// Spiel beendet; `None` = Unentschieden.
    GameOver(Option<PlayerId>),
}

#[derive(Debug, Clone)]
pub struct Engine {
    state: GameState,
    queued: Vec<Choice>,
    pending: Option<Decision>,
    /// Alle akzeptierten Antworten seit Spielbeginn (Replay).
    pub history: Vec<Choice>,
}

impl Engine {
    pub fn new(state: GameState) -> Engine {
        Engine {
            state,
            queued: Vec::new(),
            pending: None,
            history: Vec::new(),
        }
    }

    /// Letzter stabiler Zustand.
    pub fn state(&self) -> &GameState {
        &self.state
    }

    pub fn pending(&self) -> Option<&Decision> {
        self.pending.as_ref()
    }

    /// Läuft, bis eine Entscheidung nötig ist oder das Spiel endet.
    pub fn advance(&mut self) -> Status {
        match self.run() {
            Ok(s) => s,
            Err(e) => panic!("Engine-Fehler ohne offene Antwort: {e}"),
        }
    }

    fn run(&mut self) -> Result<Status, InvalidChoice> {
        loop {
            if self.state.flow == Flow::GameOver {
                self.pending = None;
                return Ok(Status::GameOver(self.state.winner.flatten()));
            }
            let mut st = self.state.clone();
            let mut ch = Choices::new(&self.queued);
            match st.step_once(&mut ch) {
                Ok(()) => {
                    let used = ch.consumed();
                    self.history.extend(self.queued.drain(..used));
                    self.state = st;
                }
                Err(Interrupt::Need(d)) => {
                    self.pending = Some(d.clone());
                    return Ok(Status::NeedsDecision(d));
                }
                Err(Interrupt::Cancelled) => {
                    assert!(
                        !self.queued.is_empty(),
                        "Abbruch ohne Spielerantwort – Engine-Fehler im Zustand {:?}",
                        self.state.flow
                    );
                    self.queued.clear();
                }
                Err(Interrupt::Invalid(e)) => {
                    self.queued.pop();
                    return Err(e);
                }
            }
        }
    }

    /// Beantwortet die offene Entscheidung und läuft weiter.
    pub fn submit(&mut self, c: Choice) -> Result<Status, InvalidChoice> {
        let d = self
            .pending
            .clone()
            .ok_or_else(|| InvalidChoice("keine offene Entscheidung".into()))?;
        d.validate(&c)?;
        self.queued.push(c);
        match self.run() {
            Ok(s) => Ok(s),
            Err(e) => {
                // Zustand vor der ungültigen Antwort wiederherstellen.
                self.pending = Some(d);
                Err(e)
            }
        }
    }
}

/// Etwas, das Entscheidungen trifft (GUI-Brücke, KI, Testskript).
pub trait Controller {
    fn decide(&mut self, state: &GameState, decision: &Decision) -> Choice;
}

/// Spielt eine Partie mit je einem Controller pro Spieler zu Ende.
/// `max_decisions` schützt vor Endlosschleifen.
pub fn run_game(
    engine: &mut Engine,
    controllers: &mut [&mut dyn Controller],
    max_decisions: usize,
) -> Option<Option<PlayerId>> {
    let mut status = engine.advance();
    for _ in 0..max_decisions {
        match status {
            Status::GameOver(w) => return Some(w),
            Status::NeedsDecision(d) => {
                let p = d.player();
                let choice = {
                    // Controller sehen den Zustand nur lesend.
                    let _cache = crate::state::CharsCache::enter();
                    controllers[p.0 as usize].decide(engine.state(), &d)
                };
                status = match engine.submit(choice) {
                    Ok(s) => s,
                    Err(e) => panic!("Controller von {p} lieferte ungültige Wahl für {d:?}: {e}"),
                };
            }
        }
    }
    None
}
