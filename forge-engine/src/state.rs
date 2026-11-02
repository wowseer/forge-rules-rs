//! Spielzustand.
//!
//! Java-Vorbild: `Game`, `Player`, `Card`, `Zone`, `MagicStack`, `Combat`.
//! Unterschied: alles ist ein einziger `Clone`-barer Wert; Objekte verweisen
//! nur über [`CardId`]/[`PlayerId`] aufeinander.

use crate::decision::{Choice, Decision, Target};
use crate::definition::CardDefinition as Def;
use crate::definition::{CardDefinition, CompiledFace};
use crate::effects::CompiledAbility;
use crate::ids::{CardId, PlayerId};
use crate::keyword::Keyword;
use crate::mana_pool::ManaPool;
use crate::rng::Rng;
use crate::turn::Step;
use crate::zone::Zone;
use forge_script::CardFace;
use forge_types::{CardType, ColorSet, ManaCost};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Startparameter einer Partie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameRules {
    pub starting_life: i32,
    pub starting_hand: u32,
    pub max_hand_size: u32,
    /// CR 103.8a: Im Zwei-Spieler-Spiel zieht der Startspieler im ersten Zug nicht.
    /// (Commander-Multiplayer: CR 103.8c – alle ziehen außer… siehe Format.)
    pub first_player_skips_draw: bool,
    pub poison_to_lose: u32,
}

impl Default for GameRules {
    fn default() -> Self {
        GameRules {
            starting_life: 20,
            starting_hand: 7,
            max_hand_size: 7,
            first_player_skips_draw: true,
            poison_to_lose: 10,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LossReason {
    /// CR 704.5a
    LifeZero,
    /// CR 704.5b
    DrewFromEmptyLibrary,
    /// CR 704.5c
    Poison,
    Conceded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Player {
    pub id: PlayerId,
    pub name: String,
    pub life: i32,
    pub poison: u32,
    /// Oberste Karte = letztes Element.
    pub library: Vec<CardId>,
    pub hand: Vec<CardId>,
    /// Oberste Karte = letztes Element.
    pub graveyard: Vec<CardId>,
    pub mana_pool: ManaPool,
    pub lands_played_this_turn: u32,
    /// Gezogene Karten in diesem Zug (Trigger `Drawn` mit `Number$`).
    pub cards_drawn_this_turn: u32,
    pub land_plays_per_turn: u32,
    pub drew_from_empty_library: bool,
    pub lost: Option<LossReason>,
    pub mulligans: u32,
    pub kept_hand: bool,
    /// Verhinderungsschild: die nächsten N Schaden in diesem Zug (CR 615.7).
    pub prevent_next: u32,
    /// In diesem Zug erhaltenes bzw. verlorenes Leben (`Count$LifeYou…ThisTurn`).
    pub life_gained_this_turn: i32,
    pub life_lost_this_turn: i32,
}

impl Player {
    pub fn is_in_game(&self) -> bool {
        self.lost.is_none()
    }

    pub fn zone(&self, z: Zone) -> &Vec<CardId> {
        match z {
            Zone::Library => &self.library,
            Zone::Hand => &self.hand,
            Zone::Graveyard => &self.graveyard,
            _ => panic!("{z:?} ist keine Spielerzone"),
        }
    }

    pub fn zone_mut(&mut self, z: Zone) -> &mut Vec<CardId> {
        match z {
            Zone::Library => &mut self.library,
            Zone::Hand => &mut self.hand,
            Zone::Graveyard => &mut self.graveyard,
            _ => panic!("{z:?} ist keine Spielerzone"),
        }
    }
}

/// Ein physisches Objekt in einer Zone. Ein Zonenwechsel erzeugt ein neues
/// Objekt mit neuer ID (CR 400.7); das alte bleibt mit `zone = None` als
/// „last known information“ erhalten.
#[derive(Debug, Clone, PartialEq)]
pub struct CardObj {
    pub id: CardId,
    pub def: Arc<CardDefinition>,
    pub face: usize,
    pub owner: PlayerId,
    pub controller: PlayerId,
    pub zone: Option<Zone>,
    pub token: bool,
    pub tapped: bool,
    /// CR 302.6: seit Beginn des letzten eigenen Zuges ununterbrochen kontrolliert.
    pub controlled_since_turn_start: bool,
    /// Controller ohne Kontrolleffekte (beim Ankommen festgelegt, CR 110.2).
    pub base_controller: PlayerId,
    pub damage: u32,
    /// CR 704.5h
    pub deathtouch_damage: bool,
    pub counters: BTreeMap<String, u32>,
    pub timestamp: u64,
    /// Nachfolger nach Zonenwechsel.
    pub moved_to: Option<CardId>,
    /// Basis-Stärke/-Widerstandskraft eines Tokens mit festgelegten Werten (`TokenPower$`).
    pub base_pt: Option<(i32, i32)>,
    /// Für X bezahlter Wert (auf dem Stapel, und als bleibende Karte danach).
    pub cast_x: u32,
    /// Gemerkte Objekte (`RememberObjects$`, `Card.IsRemembered`).
    pub remembered: Vec<CardId>,
    /// Gemerkte Spieler (`RememberObjects$` mit Spielern, `RepeatEach`).
    pub remembered_players: Vec<PlayerId>,
    /// Gewählte Karten (`ChooseCard`, Java `Card.chosenCards`).
    pub chosen_cards: Vec<CardId>,
    /// Gewählter Typ (`ChooseType`, Java `Card.chosenType`).
    pub chosen_type: Option<String>,
    /// Gewählte Farbe (`ChooseColor`, Java `Card.chosenColors`).
    pub chosen_color: Option<forge_types::Color>,
    /// Nur bei Effektobjekten (`Effect`): wann sie enden.
    pub effect: Option<EffectInfo>,
    /// Aura/Ausrüstung: an welches Objekt angelegt (CR 301.5, 303.4).
    pub attached_to: Option<CardId>,
    /// CR 715.4: „auf Abenteuer“ im Exil (als Kreatur wirkbar).
    pub on_adventure: bool,
    /// Regenerationsschilde bis Zugende (CR 701.15).
    pub regen_shields: u32,
    /// Verhinderungsschild: die nächsten N Schaden in diesem Zug (CR 615.7).
    pub prevent_next: u32,
    /// Zug, in dem zuletzt eine Loyalitätsfähigkeit aktiviert wurde (CR 606.3).
    pub loyalty_used_turn: Option<u32>,
    /// Aktivierungen (Fähigkeitsindex, Zug) für `ActivationLimit$`/`GameActivationLimit$`.
    pub activations: Vec<(usize, u32)>,
    /// Nutzungen von `MayPlay`-Erlaubnissen dieser Quelle (Static-Index, Zug).
    pub may_play_uses: Vec<(usize, u32)>,
    /// CR 701.37b: ist monströs (durch Monstrosity).
    pub monstrous: bool,
    /// Zauber auf dem Stapel: statt in den Friedhof ins Exil (`Play | ReplaceGraveyard$ Exile`).
    pub exile_from_stack: bool,
    /// Kopie eines anderen Objekts (CR 707): `def`/`face`/`base_pt` sind dann die
    /// kopierten Werte; hier die eigenen Werte und die Änderungen der Kopie.
    pub copy: Option<Box<CopyInfo>>,
}

/// Änderungen einer Kopie an den kopierbaren Werten (CR 707.9), Layer 1.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CopyMods {
    pub add_types: Vec<String>,
    /// `SetCreatureTypes$`: Kreaturtypen ersetzen
    pub set_creature_types: Option<Vec<String>>,
    pub add_keywords: Vec<crate::keyword::Keyword>,
    pub set_power: Option<i32>,
    pub set_toughness: Option<i32>,
    pub non_legendary: bool,
    pub set_color: Option<forge_types::ColorSet>,
}

impl CopyMods {
    /// CR 707.9b: Änderungen einer Kopie einer Kopie gelten zusätzlich.
    pub fn then(&self, later: &CopyMods) -> CopyMods {
        let mut m = self.clone();
        m.add_types.extend(later.add_types.iter().cloned());
        if later.set_creature_types.is_some() {
            m.set_creature_types = later.set_creature_types.clone();
        }
        m.add_keywords.extend(later.add_keywords.iter().cloned());
        m.set_power = later.set_power.or(m.set_power);
        m.set_toughness = later.set_toughness.or(m.set_toughness);
        m.non_legendary |= later.non_legendary;
        m.set_color = later.set_color.or(m.set_color);
        m
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CopyInfo {
    /// Eigene Definition/Seite/Basiswerte (beim Zonenwechsel wiederhergestellt)
    pub orig_def: Arc<CardDefinition>,
    pub orig_face: usize,
    pub orig_base_pt: Option<(i32, i32)>,
    pub mods: CopyMods,
}

/// Dauer eines Effektobjekts (`Duration$` von `Effect`, CR 611.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectDuration {
    /// Standard: bis Zugende (Cleanup, CR 514.2)
    EndOfTurn,
    Permanent,
    /// bis zum Beginn deines nächsten Zuges
    UntilYourNextTurn,
    /// bis zum Ende deines nächsten Zuges
    UntilTheEndOfYourNextTurn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectInfo {
    pub duration: EffectDuration,
    pub created_turn: u32,
}

impl CardObj {
    pub fn face(&self) -> &CardFace {
        self.def.face(self.face)
    }

    pub fn compiled(&self) -> &CompiledFace {
        self.def.compiled(self.face)
    }

    pub fn counter(&self, kind: &str) -> u32 {
        self.counters.get(kind).copied().unwrap_or(0)
    }
}

/// Dauer eines Effekts (CR 611.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Duration {
    #[default]
    UntilEndOfTurn,
    /// ohne Ende (`Duration$ Permanent`)
    Permanent,
}

/// Fortlaufender Effekt auf ein bestimmtes Objekt (CR 611, Layer 6/7c).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContinuousEffect {
    pub affected: CardId,
    /// Layer 7c
    pub power: i32,
    pub toughness: i32,
    /// Layer 6
    pub keywords: Vec<Keyword>,
    /// Layer 4: hinzugefügte Typen; `remove_creature_types` entfernt vorher alle Kreaturtypen
    pub add_types: Vec<String>,
    pub remove_creature_types: bool,
    /// Layer 5: (Farben, überschreiben statt hinzufügen)
    pub colors: Option<(forge_types::ColorSet, bool)>,
    /// Layer 7b
    pub set_power: Option<i32>,
    pub set_toughness: Option<i32>,
    pub duration: Duration,
    pub timestamp: u64,
}

/// Ein Zonenwechsel: (altes Objekt, neues Objekt, von, nach).
pub type ZoneMove = (CardId, CardId, Option<Zone>, Zone);

/// Was in diesem Zug geschah (für `Count$ThisTurn…`, Morbid, `AttackersDeclared`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TurnHistory {
    /// Alle Zonenwechsel (altes Objekt = Zustand vor dem Wechsel).
    pub moves: Vec<ZoneMove>,
    /// Gewirkte Zauber (Objekt auf dem Stapel).
    pub cast: Vec<CardId>,
    /// Kreaturen, die angegriffen haben, mit ihrem Controller.
    pub attackers: Vec<(PlayerId, CardId)>,
    /// `Fog`: aller Kampfschaden dieses Zuges wird verhindert.
    pub prevent_combat_damage: bool,
}

/// Kontrollwechsel (Layer 2, CR 613.1b) durch `GainControl`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlEffect {
    pub card: CardId,
    pub controller: PlayerId,
    pub source: CardId,
    /// Controller der Quelle beim Entstehen (für „solange du … kontrollierst“).
    pub source_controller: PlayerId,
    pub created_turn: u32,
    /// `LoseControl$ EOT`
    pub until_eot: bool,
    /// `LoseControl$ UntilTheEndOfYourNextTurn`
    pub until_end_of_next_turn: bool,
    /// `LoseControl$ LeavesPlay`: endet, wenn die Quelle das Battlefield verlässt.
    pub leaves_play: bool,
    /// `LoseControl$ LoseControl`: endet, wenn der Controller die Quelle nicht mehr kontrolliert.
    pub lose_control: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StackKind {
    /// Zauberspruch; `card` ist das Kartenobjekt in der Stack-Zone.
    Spell { card: CardId },
    /// Aktivierte Fähigkeit von `source`.
    Ability,
    /// Ausgelöste Fähigkeit (CR 603) mit dem Wissen zum Auslösezeitpunkt.
    Trigger {
        info: crate::triggers::TriggerInfo,
        optional: bool,
        description: String,
        /// Beim Auflösen erneut zu prüfende Bedingungen (CR 603.4).
        conditions: Vec<crate::expr::Condition>,
        /// Kapitelfähigkeit einer Saga (CR 714.4).
        chapter: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct StackItem {
    pub id: u32,
    pub kind: StackKind,
    pub source: CardId,
    pub controller: PlayerId,
    /// `None` bei Bleibende-Karte-Zaubersprüchen ohne Spell-Fähigkeit.
    pub ability: Option<Arc<CompiledAbility>>,
    /// Ziele je Fähigkeit der `SubAbility`-Kette.
    pub targets: Vec<Vec<Target>>,
    pub x: u32,
    /// Als Kosten bezahlte Objekte (`Sacrificed$…`, `Discarded$…`, `Exiled$…`).
    pub paid: Vec<Paid>,
}

/// Art einer Kostenzahlung mit Objekt (Java `CostPart.getHashForLKIList`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaidKind {
    Sacrificed,
    Discarded,
    Exiled,
    Tapped,
    Returned,
}

/// Ein als Kosten bezahltes Objekt (LKI: das Objekt vor dem Zonenwechsel).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Paid {
    pub kind: PaidKind,
    pub card: CardId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttackerInfo {
    pub card: CardId,
    /// Verteidigender Spieler (bei Planeswalkern deren Controller).
    pub defender: PlayerId,
    /// Angegriffen: Spieler oder Planeswalker (CR 508.1b).
    pub target: crate::decision::Target,
    /// CR 509.1h: bleibt geblockt, auch wenn Blocker entfernt werden.
    pub blocked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Combat {
    pub attackers: Vec<AttackerInfo>,
    /// (Blocker, Angreifer) in Deklarationsreihenfolge.
    pub blocks: Vec<(CardId, CardId)>,
    /// Kreaturen, die im Erstschlag-Schritt Schaden zugefügt haben.
    pub dealt_first_strike: Vec<CardId>,
}

impl Combat {
    pub fn attacker(&self, c: CardId) -> Option<&AttackerInfo> {
        self.attackers.iter().find(|a| a.card == c)
    }

    pub fn blockers_of(&self, attacker: CardId) -> Vec<CardId> {
        self.blocks
            .iter()
            .filter(|(_, a)| *a == attacker)
            .map(|(b, _)| *b)
            .collect()
    }

    pub fn blocked_by(&self, blocker: CardId) -> Vec<CardId> {
        self.blocks
            .iter()
            .filter(|(b, _)| *b == blocker)
            .map(|(_, a)| *a)
            .collect()
    }

    pub fn is_attacking(&self, c: CardId) -> bool {
        self.attacker(c).is_some()
    }

    pub fn is_blocking(&self, c: CardId) -> bool {
        self.blocks.iter().any(|(b, _)| *b == c)
    }
}

/// Wo im Ablauf sich das Spiel befindet. Jeder Wert ist ein stabiler
/// Wiederaufsetzpunkt (siehe `engine.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Setup,
    Mulligan { player: PlayerId },
    BeginStep,
    Priority { player: PlayerId, passes: u8 },
    EndStep,
    GameOver,
}

/// Protokoll für Tests, Replays und den Abgleich mit der Java-Engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Karten wurden aufgedeckt (CR 701.20).
    Revealed {
        player: PlayerId,
        cards: Vec<CardId>,
    },
    /// Ein Stapelobjekt wurde kopiert (CR 707.10).
    Copied {
        item: u32,
        original: u32,
    },
    TurnBegan {
        turn: u32,
        player: PlayerId,
    },
    StepBegan(Step),
    Drew {
        player: PlayerId,
        card: CardId,
    },
    LandPlayed {
        player: PlayerId,
        card: CardId,
    },
    Cast {
        player: PlayerId,
        card: CardId,
    },
    Activated {
        player: PlayerId,
        card: CardId,
        index: usize,
    },
    Resolved {
        item: u32,
    },
    Fizzled {
        item: u32,
    },
    ZoneChange {
        old: CardId,
        new: CardId,
        from: Option<Zone>,
        to: Zone,
    },
    Damage {
        source: CardId,
        target: Target,
        amount: u32,
    },
    LifeChanged {
        player: PlayerId,
        delta: i32,
    },
    Destroyed(CardId),
    /// CR 701.21: geopfert (Kosten oder Effekt).
    Sacrificed(CardId),
    AttackersDeclared(Vec<(CardId, crate::decision::Target)>),
    BlockersDeclared(Vec<(CardId, CardId)>),
    PlayerLost {
        player: PlayerId,
        reason: LossReason,
    },
    GameWon(Option<PlayerId>),
}

/// Abgeleitete Eigenschaften eines Objekts nach Anwendung fortlaufender
/// Effekte (CR 613). Wird bei jeder Abfrage neu berechnet.
#[derive(Debug, Clone, PartialEq)]
pub struct Characteristics {
    pub name: String,
    pub types: CardType,
    pub colors: ColorSet,
    pub mana_cost: Option<ManaCost>,
    pub power: Option<i32>,
    pub toughness: Option<i32>,
    pub keywords: Vec<Keyword>,
}

impl Characteristics {
    pub fn has(&self, k: &Keyword) -> bool {
        self.keywords.contains(k)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameState {
    pub rules: GameRules,
    pub players: Vec<Player>,
    pub cards: Vec<CardObj>,
    pub battlefield: Vec<CardId>,
    pub exile: Vec<CardId>,
    pub command: Vec<CardId>,
    /// Oberstes Element = letztes.
    pub stack: Vec<StackItem>,
    pub turn: u32,
    pub active: PlayerId,
    pub step: Step,
    pub flow: Flow,
    pub combat: Option<Combat>,
    pub effects: Vec<ContinuousEffect>,
    pub rng: Rng,
    pub next_timestamp: u64,
    pub next_stack_id: u32,
    pub starting_player: PlayerId,
    pub winner: Option<Option<PlayerId>>,
    /// CR 514.3a: Cleanup wiederholen
    pub repeat_cleanup: bool,
    /// Ausgelöste, noch nicht auf den Stapel gelegte Trigger (CR 603.3).
    pub pending_triggers: Vec<crate::triggers::PendingTrigger>,
    /// Objekte, deren `ETBReplacement:Other`-Fähigkeiten („wenn … ins Spiel kommt,
    /// wähle …“) noch ausgeführt werden müssen (vor den nächsten SBAs).
    pub pending_etb: Vec<CardId>,
    /// Während `fire_only`: nur Trigger dieses Objekts prüfen.
    pub fire_only: Option<CardId>,
    /// Während gleichzeitiger Zonenwechsel: Battlefield davor (CR 603.10a).
    pub lki_battlefield: Option<Vec<CardId>>,
    /// Verzögerte Trigger (CR 603.7), in Erzeugungsreihenfolge.
    pub delayed_triggers: Vec<crate::triggers::DelayedTrigger>,
    /// Gebündelte Zonenwechsel (alt, neu, von, nach) für `ChangesZoneAll`,
    /// mit Verschachtelungstiefe; `None` = jeder Wechsel ein eigenes Ereignis.
    pub zone_batch: Option<Vec<ZoneMove>>,
    pub zone_batch_depth: u32,
    /// Kontrolleffekte in Zeitstempelreihenfolge (jüngster zuletzt).
    pub control_effects: Vec<ControlEffect>,
    /// Ziele der Elternfähigkeit während des Auflösens (`Defined$ ParentTarget`).
    pub parent_targets: Vec<crate::decision::Target>,
    /// Gleichzeitig zugefügter Schaden (Quelle, Ziel, Menge, Kampf) für
    /// `DamageDoneOnce`; `None` = jeder Schaden ist ein eigenes Ereignis.
    pub damage_batch: Option<Vec<(CardId, crate::decision::Target, u32, bool)>>,
    pub log: Vec<Event>,
    /// Zugstatistik (Java `Zone.cardsAddedThisTurn`, `MagicStack.spellsCastThisTurn`,
    /// `Player.creaturesAttackedThisTurn`); wird zu Beginn jedes Zuges geleert.
    pub this_turn: TurnHistory,
    /// Token-Definitionen (`res/tokenscripts`), nach Skriptname.
    pub token_defs: Arc<std::collections::HashMap<String, Arc<Def>>>,
}

/// Ergebnis eines Abbruchs innerhalb eines atomaren Schritts.
#[derive(Debug, Clone, PartialEq)]
pub enum Interrupt {
    /// Eine Entscheidung wird benötigt.
    Need(Decision),
    /// Spieler hat eine Aktion abgebrochen – Zustand wird verworfen.
    Cancelled,
    /// Eine Antwort verstößt gegen zustandsabhängige Regeln.
    Invalid(crate::decision::InvalidChoice),
}

/// Quelle für bereits getroffene Entscheidungen während eines Schritts.
pub struct Choices<'a> {
    queue: &'a [Choice],
    pub(crate) pos: usize,
}

impl<'a> Choices<'a> {
    pub fn new(queue: &'a [Choice]) -> Choices<'a> {
        Choices { queue, pos: 0 }
    }

    pub fn ask(&mut self, d: Decision) -> Result<Choice, Interrupt> {
        match self.queue.get(self.pos) {
            Some(c) => {
                self.pos += 1;
                Ok(c.clone())
            }
            None => Err(Interrupt::Need(d)),
        }
    }

    pub fn consumed(&self) -> usize {
        self.pos
    }
}

/// Ergebnis eines Teilschritts, der eine Entscheidung brauchen kann.
pub type Res<T> = Result<T, Interrupt>;

impl GameState {
    pub fn card(&self, id: CardId) -> &CardObj {
        &self.cards[id.0 as usize]
    }

    pub fn card_mut(&mut self, id: CardId) -> &mut CardObj {
        &mut self.cards[id.0 as usize]
    }

    pub fn player(&self, id: PlayerId) -> &Player {
        &self.players[id.0 as usize]
    }

    pub fn player_mut(&mut self, id: PlayerId) -> &mut Player {
        &mut self.players[id.0 as usize]
    }

    pub fn timestamp(&mut self) -> u64 {
        self.next_timestamp += 1;
        self.next_timestamp
    }

    /// Spieler, die noch im Spiel sind, in Zugreihenfolge ab `from`.
    pub fn players_from(&self, from: PlayerId) -> Vec<PlayerId> {
        let n = self.players.len();
        (0..n)
            .map(|i| PlayerId(((from.0 as usize + i) % n) as u8))
            .filter(|p| self.player(*p).is_in_game())
            .collect()
    }

    /// APNAP-Reihenfolge (CR 101.4).
    pub fn apnap(&self) -> Vec<PlayerId> {
        self.players_from(self.active)
    }

    pub fn next_player(&self, after: PlayerId) -> PlayerId {
        let order = self.players_from(PlayerId(
            ((after.0 as usize + 1) % self.players.len()) as u8,
        ));
        order.first().copied().unwrap_or(after)
    }

    pub fn opponents(&self, of: PlayerId) -> Vec<PlayerId> {
        self.players_from(of)
            .into_iter()
            .filter(|p| *p != of)
            .collect()
    }

    pub fn players_in_game(&self) -> usize {
        self.players.iter().filter(|p| p.is_in_game()).count()
    }

    pub fn battlefield_of(&self, controller: PlayerId) -> impl Iterator<Item = CardId> + '_ {
        self.battlefield
            .iter()
            .copied()
            .filter(move |c| self.card(*c).controller == controller)
    }

    /// Eigenschaften nach Layer-System (aktuell: Basis + Layer 6 Fähigkeiten
    /// hinzufügen + Layer 7c P/T-Modifikationen und Marken).
    /// Eigenschaften nach dem Layer-System (CR 613). Umgesetzt: Layer 4
    /// (Typ hinzufügen), 5 (Farbe setzen), 6 (Keywords hinzufügen), 7b (P/T
    /// setzen), 7c (P/T ändern inkl. ±1/±1-Marken). Innerhalb eines Layers nach
    /// Zeitstempel (613.7); die betroffene Menge eines statischen Effekts wird
    /// im ersten Layer bestimmt, in dem er wirkt (613.6). Abhängigkeiten
    /// (613.8) fehlen noch.
    pub fn chars(&self, id: CardId) -> Characteristics {
        // Während eines Aufrufs ist der Zustand unveränderlich (`&self`): Cache
        // aktivieren, damit verschachtelte Auswertungen (Statics, die Eigenschaften
        // anderer Objekte brauchen) nicht quadratisch oft neu rechnen. Verschachtelte
        // Ergebnisse sind absichtlich unvollständig (nur nicht-dynamische Statics)
        // und haben daher einen eigenen Schlüssel.
        let _guard = CharsCache::enter();
        let nested = crate::expr::DepthGuard::active();
        let key = (self as *const GameState as usize, id, nested);
        if let Some(hit) =
            CHARS_CACHE.with(|c| c.borrow().as_ref().and_then(|m| m.get(&key).cloned()))
        {
            return hit;
        }
        let ch = self.chars_uncached(id);
        CHARS_CACHE.with(|c| {
            if let Some(m) = c.borrow_mut().as_mut() {
                m.insert(key, ch.clone());
            }
        });
        ch
    }

    /// Aktuelles Objekt nach Zonenwechseln (folgt `moved_to`).
    pub fn current_object(&self, mut id: CardId) -> CardId {
        while let Some(n) = self.card(id).moved_to {
            id = n;
        }
        id
    }

    /// Gemerkte Objekte von `source` in ihrem aktuellen Zustand (Java
    /// `game.getCardState`). Effektobjekte bleiben beim gemerkten Objekt, da
    /// `ForgetOnMoved$` vereinfacht über die neue ID (CR 400.7) abgebildet ist.
    pub fn remembered_current(&self, source: CardId) -> Vec<CardId> {
        let src = self.card(source);
        if src.effect.is_some() {
            return src.remembered.clone();
        }
        src.remembered
            .iter()
            .map(|r| self.current_object(*r))
            .collect()
    }

    fn chars_uncached(&self, id: CardId) -> Characteristics {
        use crate::statics::{CompiledStatic, ContinuousStatic};
        let c = self.card(id);
        let face = c.face();
        let compiled = c.compiled();
        let mut ch = Characteristics {
            name: face.name.clone(),
            types: face.types.clone(),
            // CR 702.114a: Devoid – farblos (charakteristik-definierend, alle Zonen)
            colors: if compiled.keywords.contains(&crate::keyword::Keyword::Devoid) {
                forge_types::ColorSet::default()
            } else {
                face.color()
            },
            // CR 712.8e: Manawert der Rückseite = der der Vorderseite (vereinfacht:
            // die Manakosten der Vorderseite).
            mana_cost: if c.face == 1
                && c.def.script.split_mode == forge_script::SplitMode::DoubleFaced
            {
                c.def.face(0).mana_cost.clone()
            } else {
                face.mana_cost.clone()
            },
            power: c
                .base_pt
                .map(|b| b.0)
                .or_else(|| face.power.as_ref().map(|p| p.base())),
            toughness: c
                .base_pt
                .map(|b| b.1)
                .or_else(|| face.toughness.as_ref().map(|t| t.base())),
            keywords: compiled.keywords.clone(),
        };
        // Layer 1 (CR 613.1a): Änderungen einer Kopie an den kopierbaren Werten
        if let Some(cp) = &c.copy {
            let m = &cp.mods;
            if let Some(cts) = &m.set_creature_types {
                if let Some(reg) = crate::valid::type_registry() {
                    ch.types
                        .subtypes
                        .retain(|t| reg.kind_of(t) != Some(forge_types::SubtypeKind::Creature));
                }
                for t in cts {
                    ch.types.add(t);
                }
            }
            for t in &m.add_types {
                ch.types.add(t);
            }
            if m.non_legendary {
                ch.types
                    .supertypes
                    .remove(&forge_types::Supertype::Legendary);
            }
            if let Some(col) = m.set_color {
                ch.colors = col;
            }
            if m.set_power.is_some() {
                ch.power = m.set_power;
            }
            if m.set_toughness.is_some() {
                ch.toughness = m.set_toughness;
            }
            ch.keywords.extend(m.add_keywords.iter().cloned());
        }

        // Wirkende statische Fähigkeiten: Quelle in einer EffectZone, Objekt in einer AffectedZone.
        struct Src<'a> {
            st: &'a ContinuousStatic,
            source: CardId,
            you: PlayerId,
            ts: u64,
            affected: Option<bool>,
        }
        let mut statics: Vec<Src> = Vec::new();
        let nested = crate::expr::DepthGuard::active();
        if let Some(zone) = c.zone {
            let sources = self
                .battlefield
                .iter()
                .chain(self.exile.iter())
                .chain(self.command.iter())
                .copied();
            let player_zone_sources = self
                .players
                .iter()
                .flat_map(|p| p.graveyard.iter().chain(p.hand.iter()).copied());
            // Das Objekt selbst immer (CDAs gelten in jeder Zone, auch Bibliothek/Stapel).
            let own = std::iter::once(id).filter(|_| {
                !matches!(
                    zone,
                    Zone::Battlefield | Zone::Exile | Zone::Command | Zone::Graveyard | Zone::Hand
                )
            });
            for sid in sources.chain(player_zone_sources).chain(own) {
                let s = self.card(sid);
                let Some(szone) = s.zone else { continue };
                for cs in &s.compiled().statics {
                    let CompiledStatic::Continuous(st) = cs.as_ref() else {
                        continue;
                    };
                    // Rekursionsschutz: in verschachtelten Auswertungen nur statische Werte.
                    if nested && st.is_dynamic() {
                        continue;
                    }
                    if st.effect_zones.contains(&szone) && st.affected_zones.contains(&zone) {
                        statics.push(Src {
                            st,
                            source: sid,
                            you: s.controller,
                            ts: s.timestamp,
                            affected: None,
                        });
                    }
                }
            }
        }
        statics.sort_by_key(|s| s.ts);
        let ectx = |s: &Src| crate::expr::EvalCtx {
            source: s.source,
            controller: s.you,
            x: 0,
            targets: &[],
            trig: None,
            paid: &[],
        };
        {
            let _g = crate::expr::DepthGuard::enter();
            // Die Bedingung hängt nur von der Quelle ab, nicht vom betroffenen
            // Objekt: bei aktivem Cache einmal je Static auswerten.
            statics.retain(|s| {
                if s.st.conditions.is_empty() {
                    return true;
                }
                let key = (
                    self as *const GameState as usize,
                    s.st as *const ContinuousStatic as usize,
                    s.source,
                    nested,
                );
                if let Some(hit) =
                    COND_CACHE.with(|c| c.borrow().as_ref().and_then(|m| m.get(&key).copied()))
                {
                    return hit;
                }
                let r = self.check_conditions(&s.st.conditions, &ectx(s));
                COND_CACHE.with(|c| {
                    if let Some(m) = c.borrow_mut().as_mut() {
                        m.insert(key, r);
                    }
                });
                r
            });
        }
        let value = |this: &GameState, s: &Src, e: &Option<crate::expr::Expr>| -> Option<i32> {
            let _g = crate::expr::DepthGuard::enter();
            e.as_ref().map(|e| this.eval(e, &ectx(s)))
        };
        let mut pumps: Vec<&ContinuousEffect> =
            self.effects.iter().filter(|e| e.affected == id).collect();
        pumps.sort_by_key(|e| e.timestamp);

        // Betroffen? Im ersten wirkenden Layer festgelegt (CR 613.6).
        let applies = |this: &GameState, s: &mut Src, ch: &Characteristics| -> bool {
            *s.affected.get_or_insert_with(|| {
                (!s.st.affects_self_only || id == s.source)
                    && (!s.st.affects_attached || this.card(s.source).attached_to == Some(id))
                    && s.st
                        .affected
                        .as_ref()
                        .is_none_or(|a| this.card_matches_with(id, ch, a, s.source, s.you))
            })
        };

        // Layer 4 (statisch und Effekte nach Zeitstempel)
        let mut l4: Vec<(u64, Vec<String>, bool)> = Vec::new();
        for s in statics.iter_mut().filter(|s| s.st.has_layer4()) {
            if applies(self, s, &ch) {
                // `AddType$ ChosenType`: der von der Quelle gewählte Typ
                let chosen = self.card(s.source).chosen_type.clone();
                let types =
                    s.st.add_types
                        .iter()
                        .filter_map(|t| {
                            if t == "ChosenType" {
                                chosen.clone()
                            } else {
                                Some(t.clone())
                            }
                        })
                        .collect();
                l4.push((s.ts, types, false));
            }
        }
        l4.extend(
            pumps
                .iter()
                .filter(|e| !e.add_types.is_empty() || e.remove_creature_types)
                .map(|e| (e.timestamp, e.add_types.clone(), e.remove_creature_types)),
        );
        l4.sort_by_key(|x| x.0);
        for (_, types, remove_ct) in l4 {
            if remove_ct && let Some(reg) = crate::valid::type_registry() {
                ch.types
                    .subtypes
                    .retain(|t| reg.kind_of(t) != Some(forge_types::SubtypeKind::Creature));
            }
            for t in &types {
                ch.types.add(t);
            }
        }
        // Layer 5
        let mut l5: Vec<(u64, forge_types::ColorSet, bool)> = Vec::new();
        for s in statics.iter_mut().filter(|s| s.st.has_layer5()) {
            // `SetColor$ ChosenColor`: die von der Quelle gewählte Farbe
            let col = if s.st.set_color_chosen {
                self.card(s.source)
                    .chosen_color
                    .map(|c| std::iter::once(c).collect())
            } else {
                s.st.set_color
            };
            if applies(self, s, &ch)
                && let Some(c) = col
            {
                l5.push((s.ts, c, true));
            }
        }
        l5.extend(
            pumps
                .iter()
                .filter_map(|e| e.colors.map(|(c, o)| (e.timestamp, c, o))),
        );
        l5.sort_by_key(|x| x.0);
        for (_, c, overwrite) in l5 {
            ch.colors = if overwrite { c } else { ch.colors.union(c) };
        }
        // Layer 6 (statisch und Pump-Effekte gemeinsam nach Zeitstempel)
        let mut l6: Vec<(u64, Vec<Keyword>)> = Vec::new();
        for s in statics.iter_mut().filter(|s| s.st.has_layer6()) {
            if applies(self, s, &ch) {
                l6.push((s.ts, s.st.add_keywords.clone()));
            }
        }
        l6.extend(pumps.iter().map(|e| (e.timestamp, e.keywords.clone())));
        l6.sort_by_key(|x| x.0);
        for (_, kws) in l6 {
            for k in kws {
                if !ch.keywords.contains(&k) {
                    ch.keywords.push(k);
                }
            }
        }
        // Layer 7a (CDAs), dann 7b
        for cda_pass in [true, false] {
            for s in statics
                .iter_mut()
                .filter(|s| s.st.has_layer7b() && s.st.characteristic_defining == cda_pass)
            {
                if applies(self, s, &ch) {
                    if let Some(p) = value(self, s, &s.st.set_power) {
                        ch.power = Some(p);
                    }
                    if let Some(t) = value(self, s, &s.st.set_toughness) {
                        ch.toughness = Some(t);
                    }
                }
            }
        }
        // Layer 7b aus Effekten (Animate) – nach den statischen 7b-Werten derselben Schicht;
        // Zeitstempel-Reihenfolge innerhalb 7b: Effekte sind i. d. R. jünger.
        for e in pumps
            .iter()
            .filter(|e| e.set_power.is_some() || e.set_toughness.is_some())
        {
            if let Some(p) = e.set_power {
                ch.power = Some(p);
            }
            if let Some(t) = e.set_toughness {
                ch.toughness = Some(t);
            }
        }
        // Layer 7c
        let mut dp = 0;
        let mut dt = 0;
        for s in statics.iter_mut().filter(|s| s.st.has_layer7c()) {
            if applies(self, s, &ch) {
                dp += value(self, s, &s.st.add_power).unwrap_or(0);
                dt += value(self, s, &s.st.add_toughness).unwrap_or(0);
            }
        }
        for e in &pumps {
            dp += e.power;
            dt += e.toughness;
        }
        let counters = c.counter("P1P1") as i32 - c.counter("M1M1") as i32;
        if let (Some(p), Some(t)) = (ch.power.as_mut(), ch.toughness.as_mut()) {
            *p += dp + counters;
            *t += dt + counters;
        }
        ch
    }

    /// Alle Objekte, die in einer Zone sind (ohne alte LKI-Objekte der Arena).
    /// Deutlich schneller als `cards.iter()`, weil die Arena mit jedem
    /// Zonenwechsel wächst.
    pub fn live_cards(&self) -> impl Iterator<Item = CardId> + '_ {
        self.battlefield
            .iter()
            .chain(self.exile.iter())
            .chain(self.command.iter())
            .copied()
            .chain(self.players.iter().flat_map(|p| {
                p.hand
                    .iter()
                    .chain(p.graveyard.iter())
                    .chain(p.library.iter())
                    .copied()
            }))
            .chain(self.stack.iter().filter_map(|s| match s.kind {
                StackKind::Spell { card } => Some(card),
                _ => None,
            }))
    }

    /// CR 714.1: Saga mit Kapitelfähigkeiten (Java `isSaga() && hasChapter()`).
    pub fn is_saga(&self, id: CardId) -> bool {
        self.card(id).compiled().final_chapter.is_some() && self.chars(id).types.has_subtype("Saga")
    }

    /// Wählbare Typen für `ChooseType` (sortiert). Ohne geladene Typliste: die
    /// Kreaturtypen aller Karten im Spiel.
    pub fn type_choice_options(&self, kind: crate::effects::TypeChoiceKind) -> Vec<String> {
        use crate::effects::TypeChoiceKind as K;
        use forge_types::SubtypeKind;
        let basics = ["Plains", "Island", "Swamp", "Mountain", "Forest"];
        let mut v: Vec<String> = match kind {
            K::BasicLand => basics.iter().map(|s| s.to_string()).collect(),
            K::Card => [
                "Artifact",
                "Battle",
                "Creature",
                "Enchantment",
                "Instant",
                "Kindred",
                "Land",
                "Planeswalker",
                "Sorcery",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            K::Creature | K::Land => {
                let want = if kind == K::Creature {
                    SubtypeKind::Creature
                } else {
                    SubtypeKind::Land
                };
                match crate::valid::type_registry() {
                    Some(r) => r
                        .creature_types()
                        .filter(|_| want == SubtypeKind::Creature)
                        .map(str::to_string)
                        .chain(
                            (want == SubtypeKind::Land)
                                .then(|| basics.iter().map(|s| s.to_string()))
                                .into_iter()
                                .flatten(),
                        )
                        .collect(),
                    None => self
                        .cards
                        .iter()
                        .filter(|c| {
                            c.def.face(0).types.is_creature() == (want == SubtypeKind::Creature)
                        })
                        .flat_map(|c| c.def.face(0).types.subtypes.clone())
                        .collect(),
                }
            }
        };
        v.sort();
        v.dedup();
        v
    }

    /// Kopierbare Werte von `of` (CR 707.2/707.3): Definition, Seite, Basis-P/T und
    /// ggf. die Änderungen einer Kopie, auf der `of` selbst beruht.
    pub fn copiable_values(
        &self,
        of: CardId,
    ) -> (Arc<CardDefinition>, usize, Option<(i32, i32)>, CopyMods) {
        let o = self.card(of);
        let mods = o.copy.as_ref().map(|c| c.mods.clone()).unwrap_or_default();
        (o.def.clone(), o.face, o.base_pt, mods)
    }

    /// `c` wird zur Kopie von `of` (mit zusätzlichen Änderungen `mods`).
    pub fn become_copy(&mut self, c: CardId, of: CardId, mods: &CopyMods) {
        let (def, face, base_pt, prev) = self.copiable_values(of);
        let o = self.card_mut(c);
        let (orig_def, orig_face, orig_base_pt) = match o.copy.take() {
            Some(cp) => (cp.orig_def, cp.orig_face, cp.orig_base_pt),
            None => (o.def.clone(), o.face, o.base_pt),
        };
        o.def = def;
        o.face = face;
        o.base_pt = base_pt;
        o.copy = Some(Box::new(CopyInfo {
            orig_def,
            orig_face,
            orig_base_pt,
            mods: prev.then(mods),
        }));
    }

    pub fn is_creature(&self, id: CardId) -> bool {
        self.chars(id).types.is_creature()
    }

    pub fn has_keyword(&self, id: CardId, k: &Keyword) -> bool {
        self.chars(id).has(k)
    }

    pub fn push_event(&mut self, e: Event) {
        self.log.push(e);
    }
}

impl GameState {
    /// Neue Partie. `decks[i]` = (Spielername, Bibliothek). Der Startspieler
    /// wird per Seed bestimmt (CR 103.1), sofern `starting_player` fehlt.
    pub fn new(
        rules: GameRules,
        decks: Vec<(String, Vec<Arc<Def>>)>,
        seed: u64,
        starting_player: Option<PlayerId>,
    ) -> GameState {
        let mut rng = Rng::new(seed);
        let n = decks.len();
        let start = starting_player.unwrap_or_else(|| PlayerId(rng.below(n as u64) as u8));
        let mut g = GameState {
            players: Vec::new(),
            cards: Vec::new(),
            battlefield: Vec::new(),
            exile: Vec::new(),
            command: Vec::new(),
            stack: Vec::new(),
            turn: 0,
            active: start,
            step: Step::Untap,
            flow: Flow::Setup,
            combat: None,
            effects: Vec::new(),
            rng,
            next_timestamp: 0,
            next_stack_id: 0,
            starting_player: start,
            winner: None,
            repeat_cleanup: false,
            pending_triggers: Vec::new(),
            pending_etb: Vec::new(),
            fire_only: None,
            lki_battlefield: None,
            delayed_triggers: Vec::new(),
            control_effects: Vec::new(),
            parent_targets: Vec::new(),
            zone_batch: None,
            zone_batch_depth: 0,
            damage_batch: None,
            log: Vec::new(),
            this_turn: TurnHistory::default(),
            token_defs: Arc::default(),
            rules: rules.clone(),
        };
        for (i, (name, deck)) in decks.into_iter().enumerate() {
            let id = PlayerId(i as u8);
            g.players.push(Player {
                id,
                name,
                life: rules.starting_life,
                poison: 0,
                library: Vec::new(),
                hand: Vec::new(),
                graveyard: Vec::new(),
                mana_pool: ManaPool::default(),
                lands_played_this_turn: 0,
                cards_drawn_this_turn: 0,
                life_gained_this_turn: 0,
                life_lost_this_turn: 0,
                land_plays_per_turn: 1,
                drew_from_empty_library: false,
                lost: None,
                mulligans: 0,
                kept_hand: false,
                prevent_next: 0,
            });
            for def in deck {
                g.create_card(def, id, Zone::Library, false);
            }
        }
        g
    }
}

/// (Zustandsadresse, Objekt, verschachtelt) → Eigenschaften.
type CondMap = std::collections::HashMap<(usize, usize, CardId, bool), bool>;
type CharsMap = std::collections::HashMap<(usize, CardId, bool), Characteristics>;

thread_local! {
    static CHARS_CACHE: std::cell::RefCell<Option<CharsMap>> =
        const { std::cell::RefCell::new(None) };
    /// Ergebnisse von Static-Bedingungen (Zustand, Static, Quelle, verschachtelt);
    /// aktiv genau dann, wenn `CHARS_CACHE` aktiv ist.
    static COND_CACHE: std::cell::RefCell<Option<CondMap>> =
        const { std::cell::RefCell::new(None) };
}

/// Aktiviert für die Lebensdauer des Wächters einen Cache für `chars()`.
///
/// **Nur in Phasen verwenden, in denen der Zustand nicht verändert wird**
/// (legale Aktionen, Zielkandidaten, Controller-Entscheidungen). Verschachtelte
/// Wächter teilen den äußeren Cache.
pub struct CharsCache {
    owner: bool,
}

impl CharsCache {
    pub fn enter() -> CharsCache {
        let owner = CHARS_CACHE.with(|c| {
            let mut c = c.borrow_mut();
            if c.is_none() {
                *c = Some(Default::default());
                COND_CACHE.with(|k| *k.borrow_mut() = Some(Default::default()));
                true
            } else {
                false
            }
        });
        CharsCache { owner }
    }
}

impl Drop for CharsCache {
    fn drop(&mut self) {
        if self.owner {
            CHARS_CACHE.with(|c| *c.borrow_mut() = None);
            COND_CACHE.with(|c| *c.borrow_mut() = None);
        }
    }
}
