//! Typisierte Effekte (Java: `ApiType` + `*Effect.java` in `ability/effects/`).
//!
//! [`compile`] übersetzt eine [`ScriptAbility`] (rohe Parameter) einmalig beim
//! Laden in eine [`CompiledAbility`]. Nicht unterstützte ApiTypes oder
//! Parameterwerte liefern [`CompileError`] – die Karte wird dann als „nicht
//! unterstützt“ markiert (nie stilles Ignorieren).
//!
//! Die Ausführung steht in `resolve.rs`.

pub mod resolve;

use crate::cost::{Cost, CostError};
use crate::keyword::Keyword;
use crate::valid::{ValidError, ValidSpec};
use forge_script::{AbilityKind, CardFace, ScriptAbility, ScriptTrigger};
use forge_types::Color;
use std::fmt;
use std::sync::Arc;

/// Zahlenwert in Parametern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Amount {
    Fixed(i32),
    /// Der für X bezahlte Wert (`Count$xPaid`).
    XPaid,
    /// Beliebiger Ausdruck (`Count$…`), siehe `expr.rs`.
    Expr(Arc<crate::expr::Expr>),
}

/// Wen/was ein Effekt betrifft (`Defined$`, Java `AbilityUtils.getDefined…`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Defined {
    You,
    SelfCard,
    Targeted,
    TargetedController,
    EachPlayer,
    EachOpponent,
    /// Trigger-Kontext (CR 603): betroffenes Objekt nach dem Ereignis.
    TriggeredCard,
    /// … vor dem Ereignis (last known information).
    TriggeredCardLki,
    TriggeredCardController,
    TriggeredPlayer,
    TriggeredTarget,
    TriggeredSource,
    /// Vom verzögerten Trigger gemerkte Objekte, aktueller Zustand …
    DelayTriggerRemembered,
    /// … bzw. wie gemerkt (`DelayTriggerRememberedLKI`).
    DelayTriggerRememberedLki,
    /// Blocker aus dem Trigger-Kontext (bei `Blocks` das Trigger-Objekt selbst).
    TriggeredBlocker,
    /// Ziele der Elternfähigkeit, auch wenn diese Fähigkeit eigene Ziele hat.
    ParentTarget,
    /// Alle betroffenen Objekte eines Sammel-Triggers (`TriggeredCards`).
    TriggeredCards,
    /// Angreifender Spieler aus dem Trigger-Kontext.
    TriggeredAttackingPlayer,
    /// Von der Quelle gemerkte Objekte in ihrem aktuellen Zustand (`Remembered`,
    /// Java `game.getCardState`) …
    Remembered,
    /// … bzw. so, wie sie gemerkt wurden (`RememberedLKI`).
    RememberedLki,
    /// Von der Quelle gewählte Karten in ihrem aktuellen Zustand (`ChosenCard`).
    ChosenCard,
    /// Von der Quelle gewählter Spieler (`ChosenPlayer`).
    ChosenPlayer,
    /// Von der Quelle exilierte Karten (`ExiledWith`, `ExiledWithSource`).
    ExiledWith,
    /// Objekt, an dem die Quelle (Aura/Ausrüstung) hängt (`Enchanted`, `Equipped`).
    Enchanted,
    /// Dessen Controller (`EnchantedController`, `EquippedController`).
    EnchantedController,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetSpec {
    pub valid: ValidSpec,
    pub min: u32,
    pub max: u32,
    pub prompt: Option<String>,
    /// Zonen, in denen Karten-Ziele liegen müssen (Standard: Battlefield).
    pub zones: Vec<crate::zone::Zone>,
}

/// Was eine Mana-Fähigkeit erzeugt (`Produced$`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManaProduce {
    /// Feste Symbole, z. B. `G` oder `R R` oder `C`. `None` = farblos.
    Fixed(Vec<Option<Color>>),
    /// Eine Farbe nach Wahl aus der Liste (`Any` = WUBRG, `Combo R G`).
    OneOf(Vec<Color>),
    /// Die von der Quelle gewählte Farbe (`Produced$ ChosenColor`).
    ChosenColor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    DealDamage {
        amount: Amount,
        to: Defined,
        /// `DamageSource$`: andere Schadensquelle als die Quelle der Fähigkeit
        source: Option<Defined>,
    },
    Draw {
        amount: Amount,
        who: Defined,
    },
    GainLife {
        amount: Amount,
        who: Defined,
    },
    LoseLife {
        amount: Amount,
        who: Defined,
    },
    Destroy {
        what: Defined,
        no_regen: bool,
    },
    Pump {
        what: Defined,
        power: Amount,
        toughness: Amount,
        keywords: Vec<Keyword>,
        /// `NumAtt$ Double`/`NumDef$ Double`: zusätzlich + aktuelle Stärke bzw.
        /// Widerstandskraft (verdoppeln, CR 701.10)
        double: (bool, bool),
    },
    AddMana {
        produce: ManaProduce,
        /// `Amount$` (Zahl oder Ausdruck wie `X`)
        amount: Amount,
    },
    Tap {
        what: Defined,
    },
    Untap {
        what: Defined,
    },
    PutCounter {
        what: Defined,
        counter: String,
        amount: Amount,
        mode: CounterMode,
    },
    /// CR 724.1: Spieler wird Monarch.
    BecomeMonarch {
        who: Defined,
    },
    /// Eine Quelle wählen (Java `ChooseSourceEffect`): bleibende Karten, Objekte auf
    /// dem Stapel und in der Befehlszone; Ergebnis in `chosen_cards` der Quelle.
    ChooseSource {
        choices: ValidSpec,
        remember: bool,
    },
    /// Jeder Spieler aus `who` wählt eine der Unterfähigkeiten (Java `ChooseGenericEffect`).
    GenericChoice {
        who: Defined,
        choices: Vec<Arc<CompiledAbility>>,
        at_random: bool,
        /// `TempRemember$ Chooser`: der Wählende ist während der Auflösung gemerkt
        remember_chooser: bool,
    },
    /// Bibliothek aufdecken, bis passende Karten gefunden sind (Java `DigUntilEffect`).
    DigUntil {
        players: Defined,
        valid: ValidSpec,
        amount: Amount,
        max_revealed: Option<Amount>,
        /// `None` = gefundene Karten bleiben bei den übrigen
        found_dest: Option<(crate::zone::Zone, bool)>,
        /// (Zone, oben in der Bibliothek); `None` = `NoMoveRevealed$`
        revealed_dest: Option<(crate::zone::Zone, bool)>,
        random_order: bool,
        shuffle: bool,
        optional: bool,
        remember_found: bool,
        remember_revealed: bool,
        tapped: bool,
        gain_control: bool,
    },
    /// Zauber/Fähigkeit auf dem Stapel kopieren (CR 707.10, Java `CopySpellAbilityEffect`).
    CopySpellAbility {
        /// `true`: das auslösende Stapelobjekt (`TriggeredSpellAbility`), sonst die Ziele
        triggered_sa: bool,
        amount: Amount,
        may_choose_new_targets: bool,
        controller: Defined,
    },
    /// Karten während der Auflösung spielen/wirken (Java `PlayEffect`, CR 608.2g).
    Play {
        /// Bekannte Karten (`Defined$`/Ziel); sonst `valid` in `zones`.
        defined: Option<Defined>,
        valid: Option<ValidSpec>,
        zones: Vec<crate::zone::Zone>,
        /// `ValidSA$`: nur Zaubersprüche dieser Art (`None` = auch Länder)
        valid_sa: Option<ValidSpec>,
        /// `None` = alle (`Amount$ All`)
        amount: Option<Amount>,
        optional: bool,
        without_mana_cost: bool,
        controller: Defined,
        /// `ReplaceGraveyard$ Exile`
        exile_instead_of_graveyard: bool,
        remember_played: bool,
    },
    /// Marken entfernen (Java `CountersRemoveEffect`, CR 122.8).
    RemoveCounter {
        what: Defined,
        kind: CounterKind,
        /// `None` = alle Marken der Art (`CounterNum$ All`)
        amount: Option<Amount>,
        up_to: bool,
        optional: bool,
    },
    /// Bekannte Karten (Ziel/Defined) aus `origin` nach `destination` (CR 400.7).
    ChangeZone {
        what: Defined,
        origin: Vec<crate::zone::Zone>,
        destination: crate::zone::Zone,
        place: ZonePlacement,
        /// `Shuffle$ True`: danach die Bibliothek(en) der Besitzer mischen
        shuffle: bool,
        /// `Optional$ True`: je Karte „darf“ (Controller entscheidet)
        optional: bool,
    },
    /// Suche in Zonen nach `change_type` (CR 701.19); mehrere Ursprungszonen möglich.
    ChangeZoneSearch {
        origin: Vec<crate::zone::Zone>,
        optional: bool,
        destination: crate::zone::Zone,
        change_type: ValidSpec,
        num: Amount,
        place: ZonePlacement,
        shuffle: bool,
    },
    /// Spielsteine erzeugen (`Token`, CR 111). Skriptnamen aus `res/tokenscripts`.
    Token {
        scripts: Vec<String>,
        amount: Amount,
        owner: Defined,
        tapped: bool,
        power: Option<Amount>,
        toughness: Option<Amount>,
    },
    /// Alle passenden bleibenden Karten bis Zugende verstärken (`PumpAll`).
    PumpAll {
        valid: ValidSpec,
        power: Amount,
        toughness: Amount,
        keywords: Vec<Keyword>,
    },
    /// Alle passenden bleibenden Karten zerstören (`DestroyAll`, gleichzeitig).
    DestroyAll {
        valid: ValidSpec,
        no_regen: bool,
    },
    /// Oberste Karten ansehen, beliebig unten/oben anordnen (`Scry`, CR 701.22).
    Scry {
        who: Defined,
        num: Amount,
    },
    /// Zur Kopie werden (`Clone`, CR 707): `cloner` wird Kopie der gewählten Karte.
    Clone {
        source: CloneSource,
        cloner: Defined,
        /// `Optional$`: „du darfst“
        optional: bool,
        mods: crate::state::CopyMods,
    },
    /// Token-Kopien erzeugen (`CopyPermanent`, CR 707.2, 111.10).
    CopyPermanent {
        source: CloneSource,
        num: Amount,
        controller: Defined,
        tapped: bool,
        mods: crate::state::CopyMods,
        remember: bool,
        /// `AtEOT$`: verzögerter Trigger „zu Beginn des nächsten Endsegments …“
        at_eot: Option<Arc<crate::triggers::CompiledTrigger>>,
    },
    /// Farbe wählen (`ChooseColor`): die Quelle merkt sie (`ChosenColor`).
    ChooseColor {
        who: Defined,
    },
    /// Typ wählen (`ChooseType`): die Quelle merkt ihn (`ChosenType`).
    ChooseType {
        who: Defined,
        kind: TypeChoiceKind,
        invalid: Vec<String>,
        valid: Option<Vec<String>>,
    },
    /// Bibliothek mischen (`Shuffle`, CR 701.20).
    Shuffle {
        who: Defined,
    },
    /// Karten wählen (`ChooseCard`): die Quelle merkt sie als „gewählt“.
    ChooseCard {
        who: Defined,
        zones: Vec<crate::zone::Zone>,
        choices: Option<ValidSpec>,
        /// `DefinedCards$`: statt Zonen/Filter genau diese Karten
        defined_cards: Option<Defined>,
        amount: Amount,
        /// `MinAmount$` (Standard: `Amount$`)
        min: Option<u32>,
        mandatory: bool,
        /// `ControlledByPlayer$ Chooser`
        controlled_by_chooser: bool,
        /// `TargetControls$`: nur Karten der gewählten Spieler
        target_controls: bool,
        at_random: bool,
        remember: bool,
        forget: bool,
    },
    /// Oberste Karten ansehen und ggf. aufdecken (`PeekAndReveal`).
    PeekAndReveal {
        who: Defined,
        num: Amount,
        /// `RevealValid$`: nur passende Karten werden aufgedeckt
        reveal_valid: Option<ValidSpec>,
        /// `NoReveal$`: nur ansehen
        no_reveal: bool,
        /// `RevealOptional$`: Aufdecken ist freiwillig
        reveal_optional: bool,
        /// `RememberRevealed$`: Quelle merkt sich die aufgedeckten Karten
        remember_revealed: bool,
        /// `RememberPeeked$`: … ohne Aufdecken die angesehenen (passenden) Karten
        remember_peeked: bool,
    },
    /// Oberste Karten ansehen, beliebig in den Friedhof oder zurück (`Surveil`, CR 701.25).
    Surveil {
        who: Defined,
        num: Amount,
    },
    /// Oberste Karten in den Friedhof (`Mill`, CR 701.13).
    Mill {
        who: Defined,
        num: Amount,
    },
    /// Oberste `dig` Karten ansehen, bis zu `change` (gefiltert) nach `dest1`, Rest nach `dest2`.
    Dig {
        who: Defined,
        dig: Amount,
        change: Option<Amount>,
        valid: Option<ValidSpec>,
        dest1: crate::zone::Zone,
        dest1_bottom: bool,
        dest2: crate::zone::Zone,
        dest2_bottom: bool,
        optional: bool,
        /// `RememberChanged$`: Quelle merkt sich die nach `dest1` bewegten Karten
        remember: bool,
    },
    /// Regenerationsschild (`Regenerate`, CR 701.15).
    Regenerate {
        what: Defined,
    },
    /// Objekt wird (bis Zugende oder dauerhaft) zu etwas anderem (`Animate`).
    Animate {
        what: Defined,
        power: Option<Amount>,
        toughness: Option<Amount>,
        types: Vec<String>,
        remove_creature_types: bool,
        keywords: Vec<Keyword>,
        colors: Option<(forge_types::ColorSet, bool)>,
        /// `Colors$ ChosenColor`: die von der Quelle gewählte Farbe (Überschreiben wie `colors`)
        colors_chosen: Option<bool>,
        permanent: bool,
    },
    /// Schaden an alle passenden Kreaturen/Planeswalker und Spieler (`DamageAll`).
    DamageAll {
        amount: Amount,
        valid_cards: Option<ValidSpec>,
        valid_players: Option<ValidSpec>,
    },
    /// Abwerfen (`Discard`, CR 701.9).
    Discard {
        who: Defined,
        mode: DiscardMode,
        num: Amount,
        valid: Option<ValidSpec>,
        /// `RememberDiscarded$`: Quelle merkt sich die abgeworfenen Karten (im Friedhof).
        remember: bool,
    },
    /// Zauber neutralisieren (`Counter`, CR 701.5). „Es sei denn, … zahlt“
    /// regelt das allgemeine [`UnlessSpec`] der Fähigkeit.
    Counter {
        /// `Defined$ TriggeredSourceSA`: das auslösende Stapelobjekt (auch Fähigkeiten).
        triggered_sa: bool,
    },
    /// Verzögerten Trigger anlegen (`DelayedTrigger`, CR 603.7): löst beim
    /// nächsten passenden Ereignis einmal aus.
    DelayedTrigger {
        trigger: Arc<crate::triggers::CompiledTrigger>,
        remember: Option<Defined>,
        scope: DelayScope,
    },
    /// Reflexiver Trigger „Wenn du das tust, …“ (`ImmediateTrigger`, CR 603.12):
    /// löst sofort aus, `amount`-mal.
    ImmediateTrigger {
        trigger: Arc<crate::triggers::CompiledTrigger>,
        remember: Option<Defined>,
        amount: Amount,
    },
    /// Für jeden Spieler die Unterfähigkeit ausführen (`RepeatEach | RepeatPlayers$`);
    /// der jeweilige Spieler ist dabei „gemerkt“ (`Remembered`).
    RepeatEach {
        over: RepeatOver,
        sub: Arc<CompiledAbility>,
        starting_with_you: bool,
        clear_remembered_before: bool,
        /// `DamageMap$`: Schaden aller Durchläufe gleichzeitig.
        damage_map: bool,
    },
    /// Zwei Kreaturen kämpfen (`Fight`, CR 701.14): Java `FightEffect.getFighters`.
    Fight {
        /// `Defined$` (erster Kämpfer bzw. beide), sonst nur Ziele.
        defined: Option<Defined>,
    },
    /// Marken auf alle passenden bleibenden Karten (`PutCounterAll`).
    PutCounterAll {
        valid: ValidSpec,
        counter: String,
        amount: Amount,
    },
    /// Kontrolle übernehmen (`GainControl`, CR 613.1b).
    GainControl {
        what: Defined,
        new_controller: Defined,
        until_eot: bool,
        until_end_of_next_turn: bool,
        leaves_play: bool,
        lose_control: bool,
        untap: bool,
        /// `AddKWs$`: Schlüsselwörter bis Zugende (nur mit `LoseControl$ EOT`).
        keywords: Vec<Keyword>,
    },
    /// „Verhindere die nächsten N Schaden, die … in diesem Zug zugefügt würden“
    /// (`PreventDamage`, CR 615.7).
    PreventDamage {
        what: Defined,
        amount: Amount,
    },
    /// Transformieren (`SetState | Mode$ Transform`, CR 701.28).
    Transform {
        what: Defined,
        optional: bool,
    },
    /// Gemerkte Objekte vergessen (`Cleanup`): `ClearRemembered$`.
    Cleanup {
        what: Defined,
        clear_remembered: bool,
        /// `ClearChosenCard/Player/Color/Type$`, `ClearNamedCard$`
        clear_chosen: [bool; 5],
    },
    /// Spieler wählen (Java `ChoosePlayerEffect`) → `CardObj::chosen_player` der Quelle.
    ChoosePlayer {
        who: Defined,
        choices: ValidSpec,
        random: bool,
        remember: bool,
    },
    /// `ManaReflected | ReflectProperty$ Produced`: ein Mana einer Art, die die
    /// auslösende Quelle erzeugt hat (z. B. Mana Flare).
    ManaReflectProduced {
        who: Defined,
    },
    /// Kartennamen nennen (Java `ChooseCardNameEffect`); Auswahl aus den Namen der
    /// Karten im Spiel, die `valid` erfüllen.
    NameCard {
        who: Defined,
        valid: Option<ValidSpec>,
        random: bool,
    },
    /// CR 701.47: Amass <Typ> n
    Amass {
        num: Amount,
        kind: String,
    },
    /// Münzwurf (CR 705): Gewinn/Verlust bzw. Kopf/Zahl (`NoCall$`).
    FlipCoin {
        win: Option<Arc<CompiledAbility>>,
        lose: Option<Arc<CompiledAbility>>,
    },
    /// Zahl wählen → Laufzeitwert `ChosenNumber` der Quelle.
    ChooseNumber {
        who: Defined,
        min: Amount,
        max: Amount,
        random: bool,
    },
    /// CR 701.38: gegoadet bis zum nächsten Zug des Goaders (oder dauerhaft).
    Goad {
        what: Defined,
        permanent: bool,
    },
    /// Hand aufdecken bzw. ansehen (`RevealHand`, `Look$`); optional merken.
    RevealHand {
        who: Defined,
        remember: bool,
    },
    /// Schutz bis Zugende verleihen (Java `ProtectEffect`); `choose`: der
    /// Controller wählt eine Farbe (`Gains$ Choice | Choices$ AnyColor`).
    Protection {
        what: Defined,
        fixed: Vec<Keyword>,
        choose: bool,
    },
    /// Würfeln (CR 706): Summe von `amount` Würfeln mit `sides` Seiten + `modifier`;
    /// je Ergebnisbereich eine Unterfähigkeit, Ergebnis optional als Laufzeitwert.
    RollDice {
        sides: u32,
        amount: Amount,
        modifier: Option<Amount>,
        result_var: Option<String>,
        /// (von, bis, Fähigkeit); `Else` = (0, u32::MAX) am Ende
        results: Vec<(u32, u32, Arc<CompiledAbility>)>,
    },
    /// `Branch`: je nach Bedingung `TrueSubAbility$` oder `FalseSubAbility$`.
    Branch {
        check: crate::expr::Expr,
        cmp: crate::expr::Compare,
        then: Option<Arc<CompiledAbility>>,
        otherwise: Option<Arc<CompiledAbility>>,
    },
    /// Bleibende Karten opfern (`Sacrifice`, CR 701.21) bzw. mit `Destroy$` zerstören.
    Sacrifice {
        /// Spieler, die opfern (`Defined$`, Standard: du).
        who: Defined,
        /// `SacValid$`; `None` = die Quelle selbst (`Self`).
        valid: Option<ValidSpec>,
        amount: Amount,
        /// `Optional$`: der Spieler darf weniger (auch keine) wählen.
        optional: bool,
        /// `StrictAmount$`: bei zu wenigen passenden Karten wird nichts geopfert.
        strict: bool,
        /// `Destroy$`: zerstören statt opfern.
        destroy: bool,
        /// `RememberSacrificed$`: die Quelle merkt sich die geopferten Objekte.
        remember: bool,
    },
    /// Effektobjekt in der Command Zone erzeugen (`Effect`): trägt eigene
    /// Statics/Trigger/Ersatzeffekte, merkt sich Objekte, endet nach `duration`.
    CreateEffect {
        def: EffectDef,
        duration: crate::state::EffectDuration,
        remember: Option<Defined>,
    },
    /// Aura-Zauber (aus `K:Enchant`): die Aura kommt beim Auflösen an das Ziel
    /// angelegt ins Spiel (CR 303.4f) – erledigt `finish_spell`.
    AuraSpell,
    /// `Attach`: `object` (Standard: Quelle) an das Ziel anlegen (CR 701.3).
    Attach {
        object: Defined,
    },
    /// Platzhalter für modale Fähigkeiten; wird vor dem Stapeln durch die
    /// gewählten Modi ersetzt (`GameState::choose_modes`).
    Charm(CharmSpec),
    /// Effekt auf alle passenden bleibenden Karten (`AnimateAll`, `TapAll`,
    /// `UntapAll`): `inner` erhält die Karten als Ziele (`Defined::Targeted`).
    /// `players`: nur Karten dieser Spieler (Ziel bzw. `Defined$`).
    ForAll {
        valid: ValidSpec,
        players: Option<Defined>,
        inner: Box<Effect>,
    },
    /// Alle passenden bleibenden Karten gleichzeitig opfern (`SacrificeAll`).
    SacrificeAll {
        /// `Defined$`: genau diese Karten (statt `ValidCards$` auf dem Battlefield)
        what: Option<Defined>,
        valid: Option<ValidSpec>,
        /// `Controller$`: nur Karten dieser Spieler
        controller: Option<Defined>,
        remember: bool,
    },
    /// Allen Kampfschaden in diesem Zug verhindern (`Fog`, CR 615).
    Fog,
    /// Proliferieren (CR 701.27): beliebig viele bleibende Karten/Spieler mit
    /// Marken wählen, jede erhält je Markenart eine weitere; `times`-mal.
    Proliferate {
        times: Amount,
    },
    /// Alle passenden Karten der Ursprungszonen bewegen (`ChangeZoneAll`).
    ChangeZoneAll {
        origin: Vec<crate::zone::Zone>,
        destination: crate::zone::Zone,
        change_type: ValidSpec,
        place: ZonePlacement,
        shuffle: bool,
    },
}

/// Worüber `RepeatEach` läuft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepeatOver {
    /// `RepeatPlayers$`
    Players(Defined),
    /// `RepeatCards$` (+ `Zone$`, Standard Battlefield)
    Cards(ValidSpec, Vec<crate::zone::Zone>),
    /// `DefinedCards$`
    DefinedCards(Defined),
}

/// Wann ein verzögerter Trigger aktiv ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelayScope {
    /// bis er ausgelöst hat
    Until,
    /// `ThisTurn$`: nur in diesem Zug
    ThisTurn,
    /// `NextTurn$`: nur im nächsten Zug
    NextTurn,
}

/// Synthetische Definition eines Effektobjekts; Gleichheit = dieselbe Definition.
#[derive(Debug, Clone)]
pub struct EffectDef(pub Arc<crate::definition::CardDefinition>);

impl PartialEq for EffectDef {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for EffectDef {}

/// Wer wählt beim Abwerfen?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscardMode {
    /// Der abwerfende Spieler wählt (`TgtChoose`).
    PlayerChooses,
    /// Die ganze Hand (`Hand`).
    Hand,
    /// Zufällig (`Random`).
    Random,
    /// Der Controller der Fähigkeit wählt aus der aufgedeckten Hand (`RevealYouChoose`).
    YouChoose,
}

/// „… es sei denn, [Spieler] zahlt [Kosten]“ (`UnlessCost$`, CR 118.12).
///
/// Java: `AbilityUtils.handleUnlessCost`. Zahlt einer der Spieler, wird der
/// Effekt dieser Fähigkeit nicht ausgeführt (bei `switched` umgekehrt);
/// Unterfähigkeiten laufen je nach `subs` weiter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnlessSpec {
    pub cost: Cost,
    /// `UnlessPayer$` (Standard: Controller des Ziels).
    pub payer: Defined,
    /// `UnlessSwitched$`: der Effekt passiert nur, *wenn* bezahlt wird.
    pub switched: bool,
    pub subs: UnlessSubs,
}

/// `UnlessResolveSubs$`: wann Unterfähigkeiten noch ausgeführt werden.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnlessSubs {
    Always,
    WhenPaid,
    WhenNotPaid,
}

/// Modaler Effekt (`Charm`, CR 700.2): Modi werden beim Wirken/Stapeln gewählt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharmSpec {
    pub choices: Vec<Arc<CompiledAbility>>,
    pub num: u32,
    pub min: u32,
    pub can_repeat: bool,
}

/// Wie ein Objekt in der Zielzone ankommt.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ZonePlacement {
    /// `Tapped$ True` (nur Battlefield)
    pub tapped: bool,
    /// `GainControl$ True`: Controller der Fähigkeit statt Besitzer (nur Battlefield)
    pub gain_control: bool,
    /// `LibraryPosition$`: 0 = oben, -1 = unten (nur Bibliothek)
    pub library_bottom: bool,
    /// `Transformed$ True`: doppelseitige Karte kommt transformiert ins Spiel.
    pub transformed: bool,
    /// `RememberChanged$ True`: die Quelle merkt sich die bewegten Objekte (neue Objekte).
    pub remember: bool,
    /// `Duration$ UntilHostLeavesPlay`: zurück, wenn die Quelle das Battlefield verlässt
    pub until_host_leaves: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledAbility {
    pub kind: AbilityKind,
    pub api: String,
    pub cost: Option<Cost>,
    pub target: Option<TargetSpec>,
    pub effect: Effect,
    pub sub: Option<Arc<CompiledAbility>>,
    pub description: String,
    /// `SorcerySpeed$ True` bei aktivierten Fähigkeiten.
    pub sorcery_speed: bool,
    /// `ConditionPresent$`, `ConditionCheckSVar$` …: nur ausführen, wenn erfüllt.
    pub conditions: Vec<crate::expr::Condition>,
    /// `UnlessCost$`: „… es sei denn, … zahlt“.
    pub unless: Option<UnlessSpec>,
    /// `Planeswalker$ True`: Loyalitätsfähigkeit (CR 606).
    pub planeswalker: bool,
    /// `ActivationZone$`: wo die Fähigkeit aktiviert werden kann (Standard Battlefield).
    pub activation_zone: crate::zone::Zone,
    /// Cycling-Fähigkeit (für den Trigger `Cycled`).
    pub cycling: bool,
    /// `ActivationLimit$ N`: höchstens N Aktivierungen je Zug (je Objekt).
    pub activation_limit: Option<u32>,
    /// `GameActivationLimit$ N`: höchstens N Aktivierungen je Partie.
    pub game_activation_limit: Option<u32>,
}

impl CompiledAbility {
    pub fn is_mana_ability(&self) -> bool {
        // CR 605.1a: kein Ziel, erzeugt Mana, keine Loyalitätsfähigkeit.
        self.kind == AbilityKind::Activated
            && matches!(self.effect, Effect::AddMana { .. })
            && self.target.is_none()
    }

    /// Alle Fähigkeiten der Kette (diese + SubAbilities), in Auflösungsreihenfolge.
    pub fn chain(&self) -> Vec<&CompiledAbility> {
        let mut v = vec![self];
        let mut cur = self;
        while let Some(s) = &cur.sub {
            v.push(s);
            cur = s;
        }
        v
    }
}

/// Markenart für `RemoveCounter`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CounterKind {
    Named(String),
    /// `Any`: je Marke wählt der Spieler die Art
    Any,
    /// `All`: alle Marken aller Arten
    All,
}

/// Schlüsselwort-Aktionen, die Java als Varianten von `PutCounter` skriptet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CounterMode {
    #[default]
    Normal,
    /// CR 701.46a: nur, wenn keine +1/+1-Marken darauf liegen
    Adapt,
    /// CR 701.37a: nur, wenn nicht monströs; danach monströs
    Monstrosity,
    /// CR 701.39a: Kreatur mit der geringsten Widerstandskraft unter den eigenen
    Bolster,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    UnsupportedApi(String),
    UnsupportedParam {
        api: String,
        key: String,
        value: String,
    },
    MissingParam {
        api: String,
        key: &'static str,
    },
    MissingSvar(String),
    Cost(CostError),
    Valid(ValidError),
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileError::UnsupportedApi(a) => write!(f, "Effekt nicht unterstützt: {a}"),
            CompileError::UnsupportedParam { api, key, value } => {
                write!(f, "{api}: {key}$ {value} nicht unterstützt")
            }
            CompileError::MissingParam { api, key } => {
                write!(f, "{api}: Pflichtparameter {key}$ fehlt")
            }
            CompileError::MissingSvar(s) => write!(f, "SVar {s} fehlt"),
            CompileError::Cost(e) => write!(f, "{e}"),
            CompileError::Valid(e) => write!(f, "{e}"),
        }
    }
}

impl From<CostError> for CompileError {
    fn from(e: CostError) -> Self {
        CompileError::Cost(e)
    }
}

impl From<ValidError> for CompileError {
    fn from(e: ValidError) -> Self {
        CompileError::Valid(e)
    }
}

/// Parameter, die das Verhalten nicht ändern (Texte, KI-Hinweise) und daher
/// ignoriert werden dürfen. Alle anderen unbekannten Parameter sind Fehler.
const NEUTRAL_PARAMS: &[&str] = &[
    "SP",
    "AB",
    "DB",
    "SpellDescription",
    "StackDescription",
    "TgtPrompt",
    "AILogic",
    "AITgts",
    "AIPreference",
    "PrecostDesc",
    "CostDesc",
    "PlayerTurn",
    "IsCurse",
    "AiAttackController",
    "AIManaPref",
    "IsPresent",
];

struct Ctx<'a> {
    face: &'a CardFace,
    ab: &'a ScriptAbility,
    used: Vec<&'static str>,
    /// Übrige Parameter prüft ein anderer Compiler (DelayedTrigger → Trigger).
    rest_checked_elsewhere: bool,
}

impl<'a> Ctx<'a> {
    fn get(&mut self, key: &'static str) -> Option<&'a str> {
        self.used.push(key);
        self.ab.params.get(key)
    }

    fn unsupported(&self, key: &str, value: &str) -> CompileError {
        CompileError::UnsupportedParam {
            api: self.ab.api.clone(),
            key: key.into(),
            value: value.into(),
        }
    }

    fn amount(&mut self, key: &'static str, default: Option<i32>) -> Result<Amount, CompileError> {
        match self.get(key) {
            None => default
                .map(Amount::Fixed)
                .ok_or(CompileError::MissingParam {
                    api: self.ab.api.clone(),
                    key,
                }),
            Some(v) => self.amount_value(key, v),
        }
    }

    fn amount_value(&self, key: &str, v: &str) -> Result<Amount, CompileError> {
        use crate::expr::Expr;
        // Der innere Fehler nennt den eigentlichen Ausdruck (`Count$…`) und
        // bündelt so gleiche Ursachen über verschiedene Parameter hinweg.
        let _ = key;
        match Expr::parse_value(self.face, v)? {
            Expr::Const(n) => Ok(Amount::Fixed(n)),
            Expr::XPaid => Ok(Amount::XPaid),
            e => Ok(Amount::Expr(Arc::new(e))),
        }
    }

    fn defined(&mut self, default: Defined) -> Result<Defined, CompileError> {
        self.defined_key("Defined", default)
    }

    /// Wie `defined`, aber für einen anderen Schlüssel (z. B. `TokenOwner$`).
    fn defined_key(
        &mut self,
        key: &'static str,
        default: Defined,
    ) -> Result<Defined, CompileError> {
        // Nur `Defined$` meint ohne Angabe die Ziele; andere Schlüssel haben
        // ihren eigenen Standard (Java: `TokenOwner$` = You, `Object$` = Self …).
        let has_tgt = key == "Defined" && self.ab.params.has("ValidTgts");
        match self.get(key) {
            None if has_tgt => Ok(Defined::Targeted),
            // Java (AbilityUtils.getDefinedCards): ohne Defined$ ist „Self“ gemeint.
            None if default == Defined::Targeted => Ok(Defined::SelfCard),
            None => Ok(default),
            Some(v) => Ok(match v {
                "You" => Defined::You,
                "Self" => Defined::SelfCard,
                "ParentTarget" => Defined::ParentTarget,
                "Targeted" | "TargetedPlayer" | "TargetedCard" => Defined::Targeted,
                "TargetedController" => Defined::TargetedController,
                "Player" => Defined::EachPlayer,
                "Player.Opponent" | "Opponent" => Defined::EachOpponent,
                "TriggeredCard" | "TriggeredNewCardLKICopy" | "TriggeredAttacker" => {
                    Defined::TriggeredCard
                }
                "TriggeredCardLKICopy" | "TriggeredAttackerLKICopy" => Defined::TriggeredCardLki,
                "TriggeredCardController" | "TriggeredAttackerController" => {
                    Defined::TriggeredCardController
                }
                "TriggeredPlayer" | "TriggeredActivator" => Defined::TriggeredPlayer,
                "TriggeredBlocker" | "TriggeredBlockerLKICopy" => Defined::TriggeredBlocker,
                "TriggeredCards" | "TriggeredCardsLKICopy" => Defined::TriggeredCards,
                "TriggeredDefendingPlayer" | "TriggeredSourceSAController" => {
                    Defined::TriggeredPlayer
                }
                "TriggeredTarget" | "TriggeredTargetLKICopy" | "TriggeredAttackedTarget" => {
                    Defined::TriggeredTarget
                }
                "TriggeredAttackingPlayer" => Defined::TriggeredAttackingPlayer,
                "DelayTriggerRemembered" => Defined::DelayTriggerRemembered,
                "DelayTriggerRememberedLKI" => Defined::DelayTriggerRememberedLki,
                "TriggeredSource" | "TriggeredSourceLKICopy" => Defined::TriggeredSource,
                // Gemerkte Karten und Spieler (Java `getDefinedObjects("Remembered")`).
                "Remembered" | "Player.IsRemembered" | "RememberedCard" => Defined::Remembered,
                "ChosenCard" => Defined::ChosenCard,
                "Enchanted" | "Equipped" | "AttachedTo" => Defined::Enchanted,
                "ExiledWith" | "ExiledWithSource" => Defined::ExiledWith,
                "ChosenPlayer" => Defined::ChosenPlayer,
                "EnchantedController" | "EquippedController" => Defined::EnchantedController,
                "RememberedLKI" => Defined::RememberedLki,
                other => return Err(self.unsupported(key, other)),
            }),
        }
    }
}

fn parse_produced(s: &str) -> Option<ManaProduce> {
    let s = s.trim();
    if s == "Any" {
        return Some(ManaProduce::OneOf(Color::ALL.to_vec()));
    }
    if s == "ChosenColor" {
        return Some(ManaProduce::ChosenColor);
    }
    if let Some(rest) = s.strip_prefix("Combo ") {
        if rest.trim() == "Any" {
            return None; // Kombinationen mehrerer Farben: später
        }
        let cols: Option<Vec<Color>> = rest
            .split_whitespace()
            .map(|t| t.chars().next().and_then(Color::from_symbol))
            .collect();
        return cols.map(ManaProduce::OneOf);
    }
    let syms: Option<Vec<Option<Color>>> = s
        .split_whitespace()
        .map(|t| match t {
            "C" => Some(None),
            t if t.len() == 1 => t.chars().next().and_then(Color::from_symbol).map(Some),
            _ => None,
        })
        .collect();
    syms.map(ManaProduce::Fixed)
}

fn parse_zones(
    c: &Ctx,
    key: &'static str,
    v: &str,
) -> Result<Vec<crate::zone::Zone>, CompileError> {
    v.split(',')
        .map(|z| crate::zone::Zone::parse(z).ok_or_else(|| c.unsupported(key, v)))
        .collect()
}

fn compile_change_zone(c: &mut Ctx, api: &str) -> Result<Effect, CompileError> {
    use crate::zone::Zone;
    let origin_s = c.get("Origin").ok_or(CompileError::MissingParam {
        api: api.into(),
        key: "Origin",
    })?;
    let origin = parse_zones(c, "Origin", origin_s)?;
    let dest_s = c.get("Destination").ok_or(CompileError::MissingParam {
        api: api.into(),
        key: "Destination",
    })?;
    let destination = match parse_zones(c, "Destination", dest_s)?.as_slice() {
        [z] => *z,
        _ => return Err(c.unsupported("Destination", dest_s)),
    };
    let place = ZonePlacement {
        tapped: c
            .get("Tapped")
            .is_some_and(|v| v.eq_ignore_ascii_case("true")),
        gain_control: c
            .get("GainControl")
            .is_some_and(|v| v.eq_ignore_ascii_case("true")),
        library_bottom: match c.get("LibraryPosition") {
            None | Some("0") => false,
            Some("-1") => true,
            Some(v) => return Err(c.unsupported("LibraryPosition", v)),
        },
        transformed: c
            .get("Transformed")
            .is_some_and(|v| v.eq_ignore_ascii_case("true")),
        remember: c
            .get("RememberChanged")
            .is_some_and(|v| v.eq_ignore_ascii_case("true")),
        until_host_leaves: match c.get("Duration") {
            None => false,
            Some("UntilHostLeavesPlay") => true,
            Some(v) => return Err(c.unsupported("Duration", v)),
        },
    };
    // reine Text-/Hinweisparameter
    for k in [
        "ChangeTypeDesc",
        "SelectPrompt",
        "Mandatory",
        "ShuffleNonMandatory",
    ] {
        c.get(k);
    }
    let shuffle_param = c.get("Shuffle").map(|v| v.eq_ignore_ascii_case("true"));
    if api == "ChangeZoneAll" {
        let ct = c.get("ChangeType").ok_or(CompileError::MissingParam {
            api: api.into(),
            key: "ChangeType",
        })?;
        let shuffle = shuffle_param
            .unwrap_or(origin.contains(&Zone::Library) && destination != Zone::Library);
        return Ok(Effect::ChangeZoneAll {
            origin,
            destination,
            change_type: ValidSpec::parse(ct)?,
            place,
            shuffle,
        });
    }
    let hidden = c
        .get("Hidden")
        .is_some_and(|v| v.eq_ignore_ascii_case("true"))
        || (origin.iter().all(|z| z.is_hidden())
            && !c.ab.params.has("ValidTgts")
            && !c.ab.params.has("Defined"));
    let optional = c
        .get("Optional")
        .is_some_and(|v| v.eq_ignore_ascii_case("true"));
    if hidden {
        let ct = c.get("ChangeType").unwrap_or("Card");
        // `ChangeNum$ Any`: beliebig viele (CR 701.19b)
        let num = if c.ab.params.get("ChangeNum") == Some("Any") {
            c.get("ChangeNum");
            Amount::Fixed(i32::MAX)
        } else {
            c.amount("ChangeNum", Some(1))?
        };
        let shuffle = shuffle_param.unwrap_or(origin.contains(&Zone::Library));
        return Ok(Effect::ChangeZoneSearch {
            origin,
            optional,
            destination,
            change_type: ValidSpec::parse(ct)?,
            num,
            place,
            shuffle,
        });
    }
    let shuffle = shuffle_param.unwrap_or(false);
    let what = c.defined(Defined::Targeted)?;
    Ok(Effect::ChangeZone {
        what,
        origin,
        destination,
        place,
        shuffle,
        optional,
    })
}

fn compile_effect_object(c: &mut Ctx, face: &CardFace) -> Result<Effect, CompileError> {
    use crate::state::EffectDuration;
    let list = |c: &mut Ctx, key: &'static str| -> Vec<String> {
        c.get(key)
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    };
    let statics_n = list(c, "StaticAbilities");
    let triggers_n = list(c, "Triggers");
    let repl_n = list(c, "ReplacementEffects");
    let mut compiled = crate::definition::CompiledFace {
        keywords: Vec::new(),
        spell: None,
        activated: Vec::new(),
        triggers: Vec::new(),
        statics: Vec::new(),
        replacements: Vec::new(),
        damage_replacements: Vec::new(),
        etb_counters: Vec::new(),
        enchant: None,
        final_chapter: None,
        untap_replacements: Vec::new(),
        cant_counter: Vec::new(),
        etb_other: Vec::new(),
    };
    let svar = |n: &str| {
        face.svar(n)
            .ok_or_else(|| CompileError::MissingSvar(n.to_string()))
    };
    for n in &statics_n {
        let st = forge_script::ScriptStatic::parse(forge_script::ParamMap::parse(svar(n)?))
            .map_err(|_| CompileError::MissingSvar(n.clone()))?;
        for mut cs in crate::statics::compile_static_multi(face, &st)? {
            // Effektobjekte wirken aus der Command Zone.
            *cs.effect_zones_mut() = vec![crate::zone::Zone::Command];
            compiled.statics.push(Arc::new(cs));
        }
    }
    for n in &triggers_n {
        let t = forge_script::ScriptTrigger::parse(forge_script::ParamMap::parse(svar(n)?))
            .map_err(|_| CompileError::MissingSvar(n.clone()))?;
        let mut ct = crate::triggers::compile_trigger(face, &t)?;
        // Effektobjekte liegen in der Command Zone.
        ct.active_zones = vec![crate::zone::Zone::Command];
        compiled.triggers.push(Arc::new(ct));
    }
    for n in &repl_n {
        let r = forge_script::ScriptReplacement::parse(forge_script::ParamMap::parse(svar(n)?))
            .map_err(|_| CompileError::MissingSvar(n.clone()))?;
        if r.mode == "Untap" {
            let mut u = crate::replacements::compile_untap_replacement(face, &r)?;
            u.active_zones = Some(vec![crate::zone::Zone::Command]);
            compiled.untap_replacements.push(Arc::new(u));
            continue;
        }
        if r.mode == "DamageDone" {
            let mut d = crate::replacements::compile_damage_replacement(face, &r)?;
            // Effektobjekte wirken aus der Command Zone.
            d.active_zones = Some(vec![crate::zone::Zone::Command]);
            compiled.damage_replacements.push(Arc::new(d));
            continue;
        }
        compiled
            .replacements
            .push(Arc::new(crate::replacements::compile_replacement(
                face, &r,
            )?));
    }
    let duration = match c.get("Duration") {
        None | Some("EndOfTurn") => EffectDuration::EndOfTurn,
        Some("Permanent") => EffectDuration::Permanent,
        Some("UntilYourNextTurn") => EffectDuration::UntilYourNextTurn,
        Some("UntilTheEndOfYourNextTurn") => EffectDuration::UntilTheEndOfYourNextTurn,
        Some("UntilHostLeavesPlay") => EffectDuration::UntilHostLeavesPlay,
        Some(v) => return Err(c.unsupported("Duration", v)),
    };
    let remember = match c.ab.params.get("RememberObjects") {
        Some(_) => Some(c.defined_key("RememberObjects", Defined::SelfCard)?),
        None => None,
    };
    // Bei Verlassen der Zone ändert sich die ID (CR 400.7) – gemerkte Objekte
    // passen dann ohnehin nicht mehr; daher genügt das hier als Vereinfachung.
    for k in [
        "ExileOnMoved",
        "ForgetOnMoved",
        "Name",
        "Image",
        "EffectOwner",
    ] {
        if k == "EffectOwner" && c.ab.params.has(k) {
            return Err(c.unsupported(k, c.ab.params.get(k).unwrap_or("")));
        }
        c.get(k);
    }
    let name = c.ab.params.get("Name").unwrap_or("Effekt").to_string();
    let effect_face = CardFace {
        name,
        ..CardFace::default()
    };
    let script = forge_script::CardScript {
        split_mode: forge_script::SplitMode::None,
        faces: vec![Some(effect_face)],
        variants: Vec::new(),
        meld_with: None,
        partner_with: None,
        hand_life_modifier: None,
        tokens: Vec::new(),
        ai_hints: Vec::new(),
        deck_hints: Vec::new(),
        copy_face_from: Vec::new(),
    };
    let def = crate::definition::CardDefinition {
        script,
        faces: vec![Some(compiled)],
        unsupported: Vec::new(),
    };
    Ok(Effect::CreateEffect {
        def: EffectDef(Arc::new(def)),
        duration,
        remember,
    })
}

/// Übersetzt eine Fähigkeit samt `SubAbility$`-Kette.
pub fn compile(face: &CardFace, ab: &ScriptAbility) -> Result<CompiledAbility, CompileError> {
    compile_depth(face, ab, 0)
}

/// Java `CountersPutEffect.buildSpellAbility`: Adapt/Bolster/Monstrosity/Support
/// sind Kurzformen von `PutCounter` mit +1/+1-Marken.
fn expand_counter_keywords(face: &CardFace, ab: &ScriptAbility) -> Option<ScriptAbility> {
    if ab.api != "PutCounter" {
        return None;
    }
    let p = &ab.params;
    let mut out = ab.clone();
    if let Some(n) = p.get("Adapt").or(p.get("Monstrosity")).or(p.get("Bolster")) {
        out.params.insert("CounterType", "P1P1");
        out.params.insert("CounterNum", n);
    } else {
        let n = p.get("Support")?;
        out.params.insert("TargetMin", "0");
        out.params.insert("TargetMax", n);
        out.params.insert("CounterType", "P1P1");
        out.params.insert("CounterNum", "1");
        // CR 701.41a: „other“ nur bei bleibenden Karten
        let valid = if face.types.is_permanent() {
            "Creature.Other"
        } else {
            "Creature"
        };
        out.params.insert("ValidTgts", valid);
    }
    Some(out)
}

fn compile_depth(
    face: &CardFace,
    ab: &ScriptAbility,
    depth: usize,
) -> Result<CompiledAbility, CompileError> {
    let expanded = expand_counter_keywords(face, ab);
    let ab = expanded.as_ref().unwrap_or(ab);
    let mut c = Ctx {
        face,
        ab,
        used: Vec::new(),
        rest_checked_elsewhere: false,
    };
    let api = ab.api.as_str();

    let cost = match c.get("Cost") {
        Some(s) => Some(Cost::parse(s)?),
        None => None,
    };
    let target = match c.get("ValidTgts") {
        Some(v) => {
            let min = match c.get("TargetMin") {
                Some(s) => s.parse().map_err(|_| c.unsupported("TargetMin", s))?,
                None => 1,
            };
            let max = match c.get("TargetMax") {
                Some(s) => s.parse().map_err(|_| c.unsupported("TargetMax", s))?,
                None => 1,
            };
            // Ziele außerhalb des Battlefields: TgtZone$, bei ChangeZone die Origin-Zone.
            let zone_param = if c.ab.params.get("TargetType") == Some("Spell") {
                Some("Stack")
            } else {
                c.get("TgtZone")
            };
            let zone_param = zone_param.or_else(|| {
                if api == "ChangeZone" {
                    c.ab.params.get("Origin")
                } else {
                    None
                }
            });
            let zones = match zone_param {
                Some(z) => z
                    .split(',')
                    .map(|x| crate::zone::Zone::parse(x).ok_or_else(|| c.unsupported("TgtZone", z)))
                    .collect::<Result<Vec<_>, _>>()?,
                None => vec![crate::zone::Zone::Battlefield],
            };
            Some(TargetSpec {
                valid: ValidSpec::parse(v)?,
                min,
                max,
                prompt: c.get("TgtPrompt").map(str::to_string),
                zones,
            })
        }
        None => None,
    };
    let sorcery_speed = c
        .get("SorcerySpeed")
        .is_some_and(|v| v.eq_ignore_ascii_case("true"));
    let description = c
        .get("SpellDescription")
        .or(c.ab.params.get("StackDescription"))
        .unwrap_or("")
        .to_string();

    let effect = match api {
        "DealDamage" => {
            let amount = c.amount("NumDmg", None)?;
            let to = c.defined(Defined::Targeted)?;
            let source = if c.ab.params.has("DamageSource") {
                Some(c.defined_key("DamageSource", Defined::SelfCard)?)
            } else {
                None
            };
            Effect::DealDamage { amount, to, source }
        }
        "Draw" => {
            let amount = c.amount("NumCards", Some(1))?;
            let who = c.defined(Defined::You)?;
            Effect::Draw { amount, who }
        }
        "GainLife" => {
            let amount = c.amount("LifeAmount", None)?;
            let who = c.defined(Defined::You)?;
            Effect::GainLife { amount, who }
        }
        "LoseLife" => {
            let amount = c.amount("LifeAmount", None)?;
            let who = c.defined(Defined::You)?;
            Effect::LoseLife { amount, who }
        }
        "Destroy" => {
            let what = c.defined(Defined::Targeted)?;
            let no_regen = c
                .get("NoRegen")
                .is_some_and(|v| v.eq_ignore_ascii_case("true"));
            Effect::Destroy { what, no_regen }
        }
        "Pump" => {
            let what = c.defined(Defined::SelfCard)?;
            let dbl = |c: &Ctx, k: &str| c.ab.params.get(k) == Some("Double");
            let double = (dbl(&c, "NumAtt"), dbl(&c, "NumDef"));
            let power = if double.0 {
                c.get("NumAtt");
                Amount::Fixed(0)
            } else {
                c.amount("NumAtt", Some(0))?
            };
            let toughness = if double.1 {
                c.get("NumDef");
                Amount::Fixed(0)
            } else {
                c.amount("NumDef", Some(0))?
            };
            let keywords = c.get("KW").map(Keyword::parse_list).unwrap_or_default();
            if let Some(k) = keywords.iter().find(|k| !k.is_supported()) {
                return Err(c.unsupported("KW", &format!("{k:?}")));
            }
            // `Duration$ Permanent` nur für einmalige Flags („enttappt nächstes Mal nicht“)
            match c.get("Duration") {
                None => {}
                Some("Permanent")
                    if !keywords.is_empty()
                        && keywords.iter().all(|k| *k == Keyword::NoUntapNext)
                        && power == Amount::Fixed(0)
                        && toughness == Amount::Fixed(0) => {}
                Some(v) => return Err(c.unsupported("Duration", v)),
            }
            Effect::Pump {
                what,
                power,
                toughness,
                keywords,
                double,
            }
        }
        "Mana" => {
            let produced = c.get("Produced").ok_or(CompileError::MissingParam {
                api: api.into(),
                key: "Produced",
            })?;
            let produce =
                parse_produced(produced).ok_or_else(|| c.unsupported("Produced", produced))?;
            let amount = c.amount("Amount", Some(1))?;
            Effect::AddMana { produce, amount }
        }
        "Tap" => Effect::Tap {
            what: c.defined(Defined::Targeted)?,
        },
        "DigUntil" => {
            for k in ["ValidDescription", "IsCurse"] {
                c.get(k);
            }
            let flag = |c: &mut Ctx, k: &'static str| {
                c.get(k).is_some_and(|v| v.eq_ignore_ascii_case("true"))
            };
            let dest = |c: &mut Ctx,
                        zk: &'static str,
                        pk: &'static str|
             -> Result<Option<(crate::zone::Zone, bool)>, CompileError> {
                let Some(z) = c.get(zk) else { return Ok(None) };
                let zone = crate::zone::Zone::parse(z).ok_or_else(|| c.unsupported(zk, z))?;
                let top = match c.get(pk) {
                    None | Some("0") => true,
                    Some("-1") => false,
                    Some(v) => return Err(c.unsupported(pk, v)),
                };
                Ok(Some((zone, top)))
            };
            let found_dest = dest(&mut c, "FoundDestination", "FoundLibraryPosition")?;
            let no_move_revealed = flag(&mut c, "NoMoveRevealed");
            let revealed_dest = dest(&mut c, "RevealedDestination", "RevealedLibraryPosition")?;
            if revealed_dest.is_none() && !no_move_revealed {
                return Err(CompileError::MissingParam {
                    api: api.into(),
                    key: "RevealedDestination",
                });
            }
            let valid = ValidSpec::parse(c.get("Valid").unwrap_or("Card"))?;
            let max_revealed = match c.get("MaxRevealed") {
                Some(v) => Some(c.amount_value("MaxRevealed", v)?),
                None => None,
            };
            Effect::DigUntil {
                players: c.defined(Defined::You)?,
                valid,
                amount: c.amount("Amount", Some(1))?,
                max_revealed,
                found_dest,
                revealed_dest: if no_move_revealed {
                    None
                } else {
                    revealed_dest
                },
                random_order: flag(&mut c, "RevealRandomOrder"),
                shuffle: flag(&mut c, "Shuffle"),
                optional: flag(&mut c, "Optional"),
                remember_found: flag(&mut c, "RememberFound"),
                remember_revealed: flag(&mut c, "RememberRevealed"),
                tapped: flag(&mut c, "Tapped"),
                gain_control: flag(&mut c, "GainControl"),
            }
        }
        "CopySpellAbility" => {
            match c.get("TargetType") {
                Some("Spell") | None => {}
                Some(t) => return Err(c.unsupported("TargetType", t)),
            }
            let triggered_sa = match c.get("Defined") {
                None if c.ab.params.has("ValidTgts") => false,
                Some("Targeted") => false,
                Some("TriggeredSourceSA" | "TriggeredSpellAbility") => true,
                v => return Err(c.unsupported("Defined", v.unwrap_or("fehlt"))),
            };
            let may_choose_new_targets = c
                .get("MayChooseTarget")
                .is_some_and(|v| v.eq_ignore_ascii_case("true"));
            Effect::CopySpellAbility {
                triggered_sa,
                amount: c.amount("Amount", Some(1))?,
                may_choose_new_targets,
                controller: c.defined_key("Controller", Defined::You)?,
            }
        }
        "Play" => {
            for k in ["ValidSATitle", "ShowCardToActivator"] {
                c.get(k);
            }
            let valid = c.get("Valid").map(ValidSpec::parse).transpose()?;
            let zones = match c.get("ValidZone") {
                Some(z) => parse_zones(&c, "ValidZone", z)?,
                None => vec![crate::zone::Zone::Hand],
            };
            let defined = if valid.is_none() {
                Some(c.defined(Defined::Targeted)?)
            } else {
                None
            };
            // `Spell` = jeder Zauberspruch; als Kartenfilter `Card` ohne Länder.
            let valid_sa = match c.get("ValidSA") {
                None => None,
                Some(v) => {
                    let spec: Vec<String> = v
                        .split(',')
                        .map(|a| {
                            let a = a.trim();
                            match a.split_once('.') {
                                Some(("Spell", rest)) => format!("Card.{rest}"),
                                _ if a == "Spell" => "Card".to_string(),
                                _ => a.to_string(),
                            }
                        })
                        .collect();
                    Some(ValidSpec::parse(&spec.join(","))?)
                }
            };
            let amount = match c.get("Amount") {
                Some("All") => None,
                Some(v) => Some(c.amount_value("Amount", v)?),
                None => Some(Amount::Fixed(1)),
            };
            let flag = |c: &mut Ctx, k: &'static str| {
                c.get(k).is_some_and(|v| v.eq_ignore_ascii_case("true"))
            };
            let optional = flag(&mut c, "Optional");
            let without_mana_cost = flag(&mut c, "WithoutManaCost");
            let remember_played = flag(&mut c, "RememberPlayed");
            let exile_instead_of_graveyard = match c.get("ReplaceGraveyard") {
                None => false,
                Some("Exile") => true,
                Some(v) => return Err(c.unsupported("ReplaceGraveyard", v)),
            };
            Effect::Play {
                defined,
                valid,
                zones,
                valid_sa,
                amount,
                optional,
                without_mana_cost,
                controller: c.defined_key("Controller", Defined::You)?,
                exile_instead_of_graveyard,
                remember_played,
            }
        }
        "RemoveCounter" => {
            let kind = match c.get("CounterType").ok_or(CompileError::MissingParam {
                api: api.into(),
                key: "CounterType",
            })? {
                "Any" => CounterKind::Any,
                "All" => CounterKind::All,
                k => CounterKind::Named(k.to_ascii_uppercase()),
            };
            let amount = match c.get("CounterNum") {
                Some("All") => None,
                Some(v) => Some(c.amount_value("CounterNum", v)?),
                None => Some(Amount::Fixed(1)),
            };
            let up_to = c
                .get("UpTo")
                .is_some_and(|v| v.eq_ignore_ascii_case("true"));
            let optional = c
                .get("Optional")
                .is_some_and(|v| v.eq_ignore_ascii_case("true"));
            Effect::RemoveCounter {
                what: c.defined(Defined::SelfCard)?,
                kind,
                amount,
                up_to,
                optional,
            }
        }
        "BecomeMonarch" => Effect::BecomeMonarch {
            who: c.defined(Defined::You)?,
        },
        "Untap" => Effect::Untap {
            what: c.defined(Defined::Targeted)?,
        },
        "PutCounter" => {
            let mode = if c.get("Adapt").is_some() {
                CounterMode::Adapt
            } else if c.get("Monstrosity").is_some() {
                CounterMode::Monstrosity
            } else if c.get("Bolster").is_some() {
                CounterMode::Bolster
            } else {
                c.get("Support");
                CounterMode::Normal
            };
            let what = c.defined(Defined::SelfCard)?;
            let counter = c.get("CounterType").ok_or(CompileError::MissingParam {
                api: api.into(),
                key: "CounterType",
            })?;
            let amount = c.amount("CounterNum", Some(1))?;
            Effect::PutCounter {
                what,
                counter: counter.to_string(),
                amount,
                mode,
            }
        }
        "ChangeZone" | "ChangeZoneAll" => compile_change_zone(&mut c, api)?,
        "PumpAll" => {
            let valid =
                ValidSpec::parse(c.get("ValidCards").ok_or(CompileError::MissingParam {
                    api: api.into(),
                    key: "ValidCards",
                })?)?;
            let power = c.amount("NumAtt", Some(0))?;
            let toughness = c.amount("NumDef", Some(0))?;
            let keywords = c.get("KW").map(Keyword::parse_list).unwrap_or_default();
            if let Some(k) = keywords.iter().find(|k| !k.is_supported()) {
                return Err(c.unsupported("KW", &format!("{k:?}")));
            }
            Effect::PumpAll {
                valid,
                power,
                toughness,
                keywords,
            }
        }
        "DestroyAll" => {
            let valid =
                ValidSpec::parse(c.get("ValidCards").ok_or(CompileError::MissingParam {
                    api: api.into(),
                    key: "ValidCards",
                })?)?;
            let no_regen = c
                .get("NoRegen")
                .is_some_and(|v| v.eq_ignore_ascii_case("true"));
            Effect::DestroyAll { valid, no_regen }
        }
        "DamageAll" => {
            let amount = c.amount("NumDmg", None)?;
            let valid_cards = c.get("ValidCards").map(ValidSpec::parse).transpose()?;
            let valid_players = c.get("ValidPlayers").map(ValidSpec::parse).transpose()?;
            c.get("ValidDescription");
            Effect::DamageAll {
                amount,
                valid_cards,
                valid_players,
            }
        }
        "Discard" => {
            let mode = match c.get("Mode").unwrap_or("TgtChoose") {
                "TgtChoose" => DiscardMode::PlayerChooses,
                "Hand" => DiscardMode::Hand,
                "Random" => DiscardMode::Random,
                "RevealYouChoose" => DiscardMode::YouChoose,
                other => return Err(c.unsupported("Mode", other)),
            };
            let num = c.amount("NumCards", Some(1))?;
            let valid = c.get("DiscardValid").map(ValidSpec::parse).transpose()?;
            let who = c.defined(Defined::You)?;
            let remember = c.get("RememberDiscarded").is_some();
            Effect::Discard {
                who,
                mode,
                num,
                valid,
                remember,
            }
        }
        "Counter" => {
            match c.get("TargetType") {
                Some("Spell") | None => {}
                Some(t) => return Err(c.unsupported("TargetType", t)),
            }
            let triggered_sa = match c.get("Defined") {
                None => false,
                Some("TriggeredSourceSA" | "TriggeredSpellAbility") => true,
                Some(v) => return Err(c.unsupported("Defined", v)),
            };
            Effect::Counter { triggered_sa }
        }
        "DelayedTrigger" => compile_delayed(&mut c, face)?,
        "ImmediateTrigger" => compile_immediate(&mut c, face)?,
        "RepeatEach" => {
            let over = if c.ab.params.has("RepeatPlayers") {
                RepeatOver::Players(c.defined_key("RepeatPlayers", Defined::EachPlayer)?)
            } else if let Some(v) = c.get("RepeatCards") {
                let zones = match c.get("Zone") {
                    Some(z) => parse_zones(&c, "Zone", z)?,
                    None => vec![crate::zone::Zone::Battlefield],
                };
                RepeatOver::Cards(ValidSpec::parse(v)?, zones)
            } else if c.ab.params.has("DefinedCards") {
                RepeatOver::DefinedCards(c.defined_key("DefinedCards", Defined::Targeted)?)
            } else {
                return Err(c.unsupported("RepeatEach", "ohne RepeatPlayers/RepeatCards"));
            };
            let name = c
                .get("RepeatSubAbility")
                .ok_or(CompileError::MissingParam {
                    api: api.into(),
                    key: "RepeatSubAbility",
                })?;
            let sv = face
                .svar(name)
                .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?;
            let sub_ab = ScriptAbility::from_svar(sv)
                .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?
                .map_err(|_| CompileError::MissingSvar(name.to_string()))?;
            let sub = Arc::new(compile_depth(face, &sub_ab, depth + 1)?);
            // Ziele werden nur für die Hauptkette gewählt (CR 601.2c).
            if sub.chain().iter().any(|a| a.target.is_some()) {
                return Err(c.unsupported("RepeatSubAbility", "mit eigenen Zielen"));
            }
            let starting_with_you = match c.get("StartingWith") {
                None => false,
                Some("You") => true,
                Some(v) => return Err(c.unsupported("StartingWith", v)),
            };
            let clear_remembered_before = c.get("ClearRememberedBeforeLoop").is_some();
            let damage_map = c.get("DamageMap").is_some();
            // ChangeZoneTable$: betrifft nur ChangesZoneAll-Trigger (noch nicht unterstützt).
            c.get("ChangeZoneTable");
            Effect::RepeatEach {
                over,
                sub,
                starting_with_you,
                clear_remembered_before,
                damage_map,
            }
        }
        "Fight" => Effect::Fight {
            defined: match c.ab.params.get("Defined") {
                Some(_) => Some(c.defined(Defined::SelfCard)?),
                None => None,
            },
        },
        "PutCounterAll" => {
            let valid =
                ValidSpec::parse(c.get("ValidCards").ok_or(CompileError::MissingParam {
                    api: api.into(),
                    key: "ValidCards",
                })?)?;
            let counter = c
                .get("CounterType")
                .ok_or(CompileError::MissingParam {
                    api: api.into(),
                    key: "CounterType",
                })?
                .to_string();
            Effect::PutCounterAll {
                valid,
                counter,
                amount: c.amount("CounterNum", Some(1))?,
            }
        }
        "GainControl" => {
            let (mut eot, mut next, mut leaves, mut lose) = (false, false, false, false);
            if let Some(v) = c.get("LoseControl") {
                for part in v.split(',').map(str::trim) {
                    match part {
                        "EOT" => eot = true,
                        "UntilTheEndOfYourNextTurn" => next = true,
                        "LeavesPlay" => leaves = true,
                        "LoseControl" => lose = true,
                        other => return Err(c.unsupported("LoseControl", other)),
                    }
                }
            }
            let untap = c
                .get("Untap")
                .is_some_and(|v| v.eq_ignore_ascii_case("true"));
            let keywords = c.get("AddKWs").map(Keyword::parse_list).unwrap_or_default();
            if let Some(k) = keywords.iter().find(|k| !k.is_supported()) {
                return Err(c.unsupported("AddKWs", &format!("{k:?}")));
            }
            if !keywords.is_empty() && !eot {
                return Err(c.unsupported("AddKWs", "ohne LoseControl$ EOT"));
            }
            let new_controller = if c.ab.params.has("NewController") {
                c.defined_key("NewController", Defined::You)?
            } else {
                Defined::You
            };
            Effect::GainControl {
                what: c.defined(Defined::Targeted)?,
                new_controller,
                until_eot: eot,
                until_end_of_next_turn: next,
                leaves_play: leaves,
                lose_control: lose,
                untap,
                keywords,
            }
        }
        "PreventDamage" => Effect::PreventDamage {
            amount: c.amount("Amount", None)?,
            what: c.defined(Defined::Targeted)?,
        },
        "SetState" => {
            match c.get("Mode") {
                Some("Transform") => {}
                Some(m) => return Err(c.unsupported("Mode", m)),
                None => return Err(c.unsupported("Mode", "fehlt")),
            }
            let optional = c.get("Optional").is_some();
            Effect::Transform {
                what: c.defined(Defined::SelfCard)?,
                optional,
            }
        }
        "Cleanup" => {
            let clear_remembered = c.get("ClearRemembered").is_some();
            // Zustände, die die Engine (noch) nicht erzeugt: Leeren ohne Wirkung.
            for k in ["ClearImprinted", "ClearCoinFlips", "Log"] {
                c.get(k);
            }
            let flag = |c: &mut Ctx, k: &'static str| c.get(k).is_some();
            let clear_chosen = [
                flag(&mut c, "ClearChosenCard"),
                flag(&mut c, "ClearChosenPlayer"),
                flag(&mut c, "ClearChosenColor"),
                flag(&mut c, "ClearChosenType"),
                flag(&mut c, "ClearNamedCard"),
            ];
            Effect::Cleanup {
                what: c.defined(Defined::SelfCard)?,
                clear_remembered,
                clear_chosen,
            }
        }
        "ChoosePlayer" => {
            for k in ["AILogic", "ChoiceTitle"] {
                c.get(k);
            }
            let flag = |c: &mut Ctx, k: &'static str| {
                c.get(k).is_some_and(|v| v.eq_ignore_ascii_case("true"))
            };
            Effect::ChoosePlayer {
                who: c.defined(Defined::You)?,
                choices: ValidSpec::parse(c.get("Choices").unwrap_or("Player"))?,
                random: flag(&mut c, "Random"),
                remember: flag(&mut c, "RememberChosen"),
            }
        }
        "FlipCoin" => {
            if let Some(v) = c.get("Flipper")
                && v != "You"
            {
                return Err(c.unsupported("Flipper", v));
            }
            let no_call = c
                .get("NoCall")
                .is_some_and(|v| v.eq_ignore_ascii_case("true"));
            let (wk, lk) = if no_call {
                ("HeadsSubAbility", "TailsSubAbility")
            } else {
                ("WinSubAbility", "LoseSubAbility")
            };
            let sub = |c: &mut Ctx,
                       k: &'static str|
             -> Result<Option<Arc<CompiledAbility>>, CompileError> {
                let Some(name) = c.get(k) else {
                    return Ok(None);
                };
                let sv = face
                    .svar(name)
                    .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?;
                let ab = ScriptAbility::from_svar(sv)
                    .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?
                    .map_err(|_| CompileError::MissingSvar(name.to_string()))?;
                let compiled = compile_depth(face, &ab, depth + 1)?;
                if compiled.chain().iter().any(|a| a.target.is_some()) {
                    return Err(c.unsupported(k, "mit Zielen"));
                }
                Ok(Some(Arc::new(compiled)))
            };
            let win = sub(&mut c, wk)?;
            let lose = sub(&mut c, lk)?;
            Effect::FlipCoin { win, lose }
        }
        "ChooseNumber" => {
            for k in ["AILogic", "ListTitle", "ChooseAnyNumber"] {
                c.get(k);
            }
            Effect::ChooseNumber {
                who: c.defined(Defined::You)?,
                min: c.amount("Min", Some(0))?,
                max: c.amount("Max", Some(10))?,
                random: c
                    .get("Random")
                    .is_some_and(|v| v.eq_ignore_ascii_case("true")),
            }
        }
        "Goad" => {
            let permanent = match c.get("Duration") {
                None => false,
                Some("Permanent") => true,
                Some(v) => return Err(c.unsupported("Duration", v)),
            };
            Effect::Goad {
                what: c.defined(Defined::Targeted)?,
                permanent,
            }
        }
        "RevealHand" => {
            c.get("Look");
            Effect::RevealHand {
                who: c.defined(Defined::You)?,
                remember: c
                    .get("RememberRevealed")
                    .is_some_and(|v| v.eq_ignore_ascii_case("true")),
            }
        }
        "Protection" => {
            let what = c.defined(Defined::SelfCard)?;
            let gains = c.get("Gains").unwrap_or("");
            let (fixed, choose) = if gains == "Choice" {
                match c.get("Choices") {
                    Some("AnyColor") => (Vec::new(), true),
                    v => return Err(c.unsupported("Choices", v.unwrap_or("fehlt"))),
                }
            } else {
                let mut kws = Vec::new();
                for g in gains.split(',').map(str::trim) {
                    let kw = match forge_types::Color::from_name(g) {
                        Some(_) => Keyword::parse(&format!("Protection from {g}")),
                        None => Keyword::parse(&format!("Protection:{g}")),
                    };
                    if !kw.is_supported() {
                        return Err(c.unsupported("Gains", gains));
                    }
                    kws.push(kw);
                }
                (kws, false)
            };
            Effect::Protection {
                what,
                fixed,
                choose,
            }
        }
        "RollDice" => {
            c.get("AILogic");
            let sides = match c.get("Sides") {
                None => 6,
                Some(v) => v.trim().parse().map_err(|_| c.unsupported("Sides", v))?,
            };
            let amount = c.amount("Amount", Some(1))?;
            let modifier = match c.get("Modifier") {
                Some(v) => Some(c.amount_value("Modifier", v)?),
                None => None,
            };
            let result_var = c.get("ResultSVar").map(str::to_string);
            let mut results = Vec::new();
            if let Some(spec) = c.get("ResultSubAbilities") {
                for part in spec.split(',') {
                    let (range, name) = part
                        .split_once(':')
                        .ok_or_else(|| c.unsupported("ResultSubAbilities", spec))?;
                    let (lo, hi) = match range.trim() {
                        "Else" => (0, u32::MAX),
                        r => match r.split_once('-') {
                            Some((a, b)) => (
                                a.trim()
                                    .parse()
                                    .map_err(|_| c.unsupported("ResultSubAbilities", spec))?,
                                b.trim()
                                    .parse()
                                    .map_err(|_| c.unsupported("ResultSubAbilities", spec))?,
                            ),
                            None => {
                                let n: u32 = r
                                    .parse()
                                    .map_err(|_| c.unsupported("ResultSubAbilities", spec))?;
                                (n, n)
                            }
                        },
                    };
                    let sv = face
                        .svar(name.trim())
                        .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?;
                    let ab = ScriptAbility::from_svar(sv)
                        .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?
                        .map_err(|_| CompileError::MissingSvar(name.to_string()))?;
                    let compiled = compile_depth(face, &ab, depth + 1)?;
                    if compiled.chain().iter().any(|a| a.target.is_some()) {
                        return Err(c.unsupported("ResultSubAbilities", "mit Zielen"));
                    }
                    results.push((lo, hi, Arc::new(compiled)));
                }
            }
            Effect::RollDice {
                sides,
                amount,
                modifier,
                result_var,
                results,
            }
        }
        "Branch" => {
            let sv = c
                .get("BranchConditionSVar")
                .ok_or(CompileError::MissingParam {
                    api: api.into(),
                    key: "BranchConditionSVar",
                })?;
            let check = crate::expr::Expr::parse_value(face, sv)?;
            let cmp = crate::expr::Compare::parse(
                face,
                c.get("BranchConditionSVarCompare").unwrap_or("GE1"),
            )?;
            let sub = |c: &mut Ctx,
                       k: &'static str|
             -> Result<Option<Arc<CompiledAbility>>, CompileError> {
                let Some(name) = c.get(k) else {
                    return Ok(None);
                };
                let sv = face
                    .svar(name)
                    .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?;
                let ab = ScriptAbility::from_svar(sv)
                    .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?
                    .map_err(|_| CompileError::MissingSvar(name.to_string()))?;
                let compiled = compile_depth(face, &ab, depth + 1)?;
                if compiled.chain().iter().any(|a| a.target.is_some()) {
                    return Err(c.unsupported(k, "mit Zielen"));
                }
                Ok(Some(Arc::new(compiled)))
            };
            let then = sub(&mut c, "TrueSubAbility")?;
            let otherwise = sub(&mut c, "FalseSubAbility")?;
            Effect::Branch {
                check,
                cmp,
                then,
                otherwise,
            }
        }
        "Sacrifice" => {
            let valid = match c.get("SacValid").map(str::trim) {
                None | Some("Self") => None,
                Some(v) => Some(ValidSpec::parse(v)?),
            };
            let amount = c.amount("Amount", Some(1))?;
            let flag = |c: &mut Ctx, k: &'static str| c.get(k).is_some();
            let optional = flag(&mut c, "Optional");
            let strict = flag(&mut c, "StrictAmount");
            let destroy = flag(&mut c, "Destroy");
            let remember = flag(&mut c, "RememberSacrificed");
            c.get("SacMessage");
            let who = c.defined(Defined::You)?;
            Effect::Sacrifice {
                who,
                valid,
                amount,
                optional,
                strict,
                destroy,
                remember,
            }
        }
        "Effect" => compile_effect_object(&mut c, face)?,
        "Clone" => {
            if let Some(v) = c.get("Duration") {
                return Err(c.unsupported("Duration", v));
            }
            let source = clone_source(&mut c)?;
            let cloner = if c.ab.params.has("CloneTarget") {
                c.defined_key("CloneTarget", Defined::SelfCard)?
            } else if c.ab.params.has("Choices") && c.ab.params.has("ValidTgts") {
                Defined::Targeted
            } else {
                Defined::SelfCard
            };
            let optional = c.get("Optional").is_some();
            c.get("ChoiceTitle");
            Effect::Clone {
                source,
                cloner,
                optional,
                mods: copy_mods(&mut c)?,
            }
        }
        "CopyPermanent" => {
            let source = clone_source(&mut c)?;
            let at_eot = match c.get("AtEOT") {
                None => None,
                Some(v) => Some(at_eot_trigger(face, v).ok_or_else(|| c.unsupported("AtEOT", v))?),
            };
            let controller = if c.ab.params.has("Controller") {
                c.defined_key("Controller", Defined::You)?
            } else {
                Defined::You
            };
            Effect::CopyPermanent {
                source,
                num: c.amount("NumCopies", Some(1))?,
                controller,
                tapped: c
                    .get("TokenTapped")
                    .is_some_and(|v| v.eq_ignore_ascii_case("true")),
                mods: copy_mods(&mut c)?,
                remember: c.get("RememberTokens").is_some(),
                at_eot,
            }
        }
        "ChooseColor" => Effect::ChooseColor {
            who: c.defined(Defined::You)?,
        },
        "ChooseType" => {
            let kind = match c.get("Type") {
                Some("Creature") => TypeChoiceKind::Creature,
                Some("Basic Land") | Some("Basic") => TypeChoiceKind::BasicLand,
                Some("Land") => TypeChoiceKind::Land,
                Some("Card") => TypeChoiceKind::Card,
                Some(v) => return Err(c.unsupported("Type", v)),
                None => return Err(c.unsupported("Type", "fehlt")),
            };
            let list = |v: Option<&str>| -> Vec<String> {
                v.map(|v| v.split(',').map(|t| t.trim().to_string()).collect())
                    .unwrap_or_default()
            };
            let invalid = list(c.get("InvalidTypes"));
            let valid = c.get("ValidTypes").map(|v| list(Some(v)));
            c.get("Note");
            Effect::ChooseType {
                who: c.defined(Defined::You)?,
                kind,
                invalid,
                valid,
            }
        }
        "Shuffle" => Effect::Shuffle {
            who: c.defined(Defined::You)?,
        },
        "ChooseCard" => {
            let has = |c: &mut Ctx, k: &'static str| c.get(k).is_some();
            // reine Anzeige-/Informationsparameter
            for k in [
                "ChoiceTitle",
                "ChoiceTitleAppend",
                "Reveal",
                "RevealTitle",
                "Secretly",
                "ChoiceDesc",
                "LockInText",
            ] {
                c.get(k);
            }
            let zones = match c.get("ChoiceZone") {
                Some(z) => parse_zones(&c, "ChoiceZone", z)?,
                None => vec![crate::zone::Zone::Battlefield],
            };
            let controlled_by_chooser = match c.get("ControlledByPlayer") {
                None => false,
                Some("Chooser") => true,
                Some(v) => return Err(c.unsupported("ControlledByPlayer", v)),
            };
            let min = match c.get("MinAmount") {
                None => None,
                Some(v) => Some(
                    v.trim()
                        .parse()
                        .map_err(|_| c.unsupported("MinAmount", v))?,
                ),
            };
            let defined_cards = if c.ab.params.has("DefinedCards") {
                Some(c.defined_key("DefinedCards", Defined::SelfCard)?)
            } else {
                None
            };
            Effect::ChooseCard {
                zones,
                choices: c.get("Choices").map(ValidSpec::parse).transpose()?,
                defined_cards,
                amount: c.amount("Amount", Some(1))?,
                min,
                mandatory: has(&mut c, "Mandatory"),
                controlled_by_chooser,
                target_controls: has(&mut c, "TargetControls"),
                at_random: has(&mut c, "AtRandom"),
                remember: has(&mut c, "RememberChosen"),
                forget: has(&mut c, "ForgetChosen"),
                who: c.defined(Defined::You)?,
            }
        }
        "PeekAndReveal" => {
            if let Some(z) = c.get("SourceZone")
                && z != "Library"
            {
                return Err(c.unsupported("SourceZone", z));
            }
            let has = |c: &mut Ctx, k: &'static str| c.get(k).is_some();
            // NoPeek$: nur Informationsfluss (wer sieht die Karten) – ohne Regelwirkung
            has(&mut c, "NoPeek");
            Effect::PeekAndReveal {
                num: c.amount("PeekAmount", Some(1))?,
                reveal_valid: c.get("RevealValid").map(ValidSpec::parse).transpose()?,
                no_reveal: has(&mut c, "NoReveal"),
                reveal_optional: has(&mut c, "RevealOptional"),
                remember_revealed: has(&mut c, "RememberRevealed"),
                remember_peeked: has(&mut c, "RememberPeeked"),
                who: c.defined(Defined::You)?,
            }
        }
        "Scry" => {
            let num = c.amount("ScryNum", Some(1))?;
            Effect::Scry {
                who: c.defined(Defined::You)?,
                num,
            }
        }
        "Surveil" => {
            let num = c.amount("Amount", Some(1))?;
            Effect::Surveil {
                who: c.defined(Defined::You)?,
                num,
            }
        }
        "Mill" => {
            let num = c.amount("NumCards", Some(1))?;
            Effect::Mill {
                who: c.defined(Defined::You)?,
                num,
            }
        }
        "Dig" => {
            use crate::zone::Zone;
            let dig = c.amount("DigNum", None)?;
            let change = match c.get("ChangeNum") {
                Some("All") => None,
                Some(v) => Some(c.amount_value("ChangeNum", v)?),
                None => Some(Amount::Fixed(1)),
            };
            let valid = c.get("ChangeValid").map(ValidSpec::parse).transpose()?;
            let d1 = c.get("DestinationZone");
            let dest1 = match d1 {
                Some(z) => Zone::parse(z).ok_or_else(|| c.unsupported("DestinationZone", z))?,
                None => Zone::Hand,
            };
            let d2 = c.get("DestinationZone2");
            let dest2 = match d2 {
                Some(z) => Zone::parse(z).ok_or_else(|| c.unsupported("DestinationZone2", z))?,
                None => Zone::Library,
            };
            let mut pos = |k: &'static str| -> Result<bool, CompileError> {
                match c.get(k) {
                    None | Some("-1") => Ok(true),
                    Some("0") => Ok(false),
                    Some(v) => Err(c.unsupported(k, v)),
                }
            };
            let dest1_bottom = pos("LibraryPosition")?;
            let dest2_bottom = pos("LibraryPosition2")?;
            let optional = c
                .get("Optional")
                .is_some_and(|v| v.eq_ignore_ascii_case("true"));
            for k in [
                "Reveal",
                "NoReveal",
                "ForceRevealToController",
                "RestRandomOrder",
                "ChangeValidDesc",
                "PrimaryPrompt",
            ] {
                c.get(k);
            }
            Effect::Dig {
                who: c.defined(Defined::You)?,
                dig,
                change,
                valid,
                dest1,
                dest1_bottom,
                dest2,
                dest2_bottom,
                optional,
                remember: c.get("RememberChanged").is_some(),
            }
        }
        "Regenerate" => Effect::Regenerate {
            what: c.defined(Defined::SelfCard)?,
        },
        "Animate" => {
            let what = c.defined(Defined::SelfCard)?;
            compile_animate(&mut c, what)?
        }
        "AnimateAll" | "TapAll" | "UntapAll" => {
            let valid = ValidSpec::parse(c.get("ValidCards").unwrap_or("Card"))?;
            match c.get("Zone") {
                None | Some("Battlefield") => {}
                Some(v) => return Err(c.unsupported("Zone", v)),
            }
            let players = if c.ab.params.has("ValidTgts") || c.ab.params.has("Defined") {
                Some(c.defined(Defined::Targeted)?)
            } else {
                None
            };
            let inner = match api {
                "AnimateAll" => compile_animate(&mut c, Defined::Targeted)?,
                "TapAll" => Effect::Tap {
                    what: Defined::Targeted,
                },
                _ => Effect::Untap {
                    what: Defined::Targeted,
                },
            };
            Effect::ForAll {
                valid,
                players,
                inner: Box::new(inner),
            }
        }
        "Investigate" => Effect::Token {
            // CR 701.36: einen Clue-Spielstein erschaffen
            scripts: vec!["c_a_clue_draw".to_string()],
            amount: c.amount("Num", Some(1))?,
            owner: c.defined(Defined::You)?,
            tapped: false,
            power: None,
            toughness: None,
        },
        "ManaReflected" => {
            match c.get("ReflectProperty") {
                Some("Produced") => {}
                v => return Err(c.unsupported("ReflectProperty", v.unwrap_or("fehlt"))),
            }
            c.get("ColorOrType");
            Effect::ManaReflectProduced {
                who: c.defined(Defined::You)?,
            }
        }
        "NameCard" => {
            for k in ["ValidDescription", "AILogic"] {
                c.get(k);
            }
            Effect::NameCard {
                who: c.defined(Defined::You)?,
                valid: c.get("ValidCards").map(ValidSpec::parse).transpose()?,
                random: c
                    .get("AtRandom")
                    .is_some_and(|v| v.eq_ignore_ascii_case("true")),
            }
        }
        "Amass" => Effect::Amass {
            num: c.amount("Num", Some(1))?,
            kind: c
                .get("Type")
                .ok_or(CompileError::MissingParam {
                    api: api.into(),
                    key: "Type",
                })?
                .to_string(),
        },
        "Fog" => Effect::Fog,
        "Proliferate" => Effect::Proliferate {
            times: c.amount("Amount", Some(1))?,
        },
        "SacrificeAll" => {
            let what = match c.ab.params.has("Defined") {
                true => Some(c.defined(Defined::SelfCard)?),
                false => None,
            };
            let valid = match (&what, c.get("ValidCards")) {
                (None, v) => Some(ValidSpec::parse(v.unwrap_or("Card"))?),
                (Some(_), _) => None,
            };
            let controller = match c.get("Controller") {
                None => None,
                Some("You") => Some(Defined::You),
                Some("TriggeredPlayer") => Some(Defined::TriggeredPlayer),
                Some(v) => return Err(c.unsupported("Controller", v)),
            };
            let remember = c.get("RememberSacrificed").is_some();
            Effect::SacrificeAll {
                what,
                valid,
                controller,
                remember,
            }
        }
        "Attach" => {
            if c.ab.params.get("ValidTgts").is_none() {
                return Err(c.unsupported("Attach", "ohne ValidTgts"));
            }
            let object = c.defined_key("Object", Defined::SelfCard)?;
            Effect::Attach { object }
        }
        "Token" => {
            let scripts_s = c.get("TokenScript").ok_or(CompileError::MissingParam {
                api: api.into(),
                key: "TokenScript",
            })?;
            let scripts = scripts_s
                .split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect();
            let amount = c.amount("TokenAmount", Some(1))?;
            let owner = c.defined_key("TokenOwner", Defined::You)?;
            let tapped = c
                .get("TokenTapped")
                .is_some_and(|v| v.eq_ignore_ascii_case("true"));
            let power = match c.get("TokenPower") {
                Some(v) => Some(c.amount_value("TokenPower", v)?),
                None => None,
            };
            let toughness = match c.get("TokenToughness") {
                Some(v) => Some(c.amount_value("TokenToughness", v)?),
                None => None,
            };
            Effect::Token {
                scripts,
                amount,
                owner,
                tapped,
                power,
                toughness,
            }
        }
        "ChooseSource" => {
            for k in ["AILogic", "ChoiceTitle"] {
                c.get(k);
            }
            let choices = ValidSpec::parse(c.get("Choices").unwrap_or("Card,Emblem"))?;
            let remember = c
                .get("RememberChosen")
                .is_some_and(|v| v.eq_ignore_ascii_case("true"));
            Effect::ChooseSource { choices, remember }
        }
        "GenericChoice" => {
            for k in ["ShowChoice", "ChoicePrompt", "AILogic", "ChoiceTitle"] {
                c.get(k);
            }
            let choices_s = c.get("Choices").ok_or(CompileError::MissingParam {
                api: api.into(),
                key: "Choices",
            })?;
            let mut choices = Vec::new();
            for name in choices_s.split(',').map(str::trim) {
                let sv = face
                    .svar(name)
                    .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?;
                let sub_ab = ScriptAbility::from_svar(sv)
                    .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?
                    .map_err(|_| CompileError::MissingSvar(name.to_string()))?;
                let sub = compile_depth(face, &sub_ab, depth + 1)?;
                if sub.chain().iter().any(|a| a.target.is_some()) {
                    return Err(c.unsupported("Choices", "mit Zielen"));
                }
                choices.push(Arc::new(sub));
            }
            let at_random = c
                .get("AtRandom")
                .is_some_and(|v| v.eq_ignore_ascii_case("true"));
            let remember_chooser = match c.get("TempRemember") {
                None => false,
                Some("Chooser") => true,
                Some(v) => return Err(c.unsupported("TempRemember", v)),
            };
            Effect::GenericChoice {
                who: c.defined(Defined::You)?,
                choices,
                at_random,
                remember_chooser,
            }
        }
        "Charm" => {
            // Modi werden beim Wirken gewählt; als Unterfähigkeit nicht unterstützt.
            if depth > 0 {
                return Err(c.unsupported("Charm", "als Unterfähigkeit"));
            }
            let choices_s = c.get("Choices").ok_or(CompileError::MissingParam {
                api: api.into(),
                key: "Choices",
            })?;
            let mut choices = Vec::new();
            for name in choices_s.split(',').map(str::trim) {
                let sv = face
                    .svar(name)
                    .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?;
                let sub_ab = ScriptAbility::from_svar(sv)
                    .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?
                    .map_err(|_| CompileError::MissingSvar(name.to_string()))?;
                choices.push(Arc::new(compile_depth(face, &sub_ab, depth + 1)?));
            }
            let num = match c.get("CharmNum") {
                Some(v) => v.trim().parse().map_err(|_| c.unsupported("CharmNum", v))?,
                None => 1,
            };
            let min = match c.get("MinCharmNum") {
                Some(v) => v
                    .trim()
                    .parse()
                    .map_err(|_| c.unsupported("MinCharmNum", v))?,
                None => num,
            };
            let can_repeat = c
                .get("CanRepeatModes")
                .is_some_and(|v| v.eq_ignore_ascii_case("true"));
            c.get("AdditionalDescription");
            Effect::Charm(CharmSpec {
                choices,
                num,
                min,
                can_repeat,
            })
        }
        other => return Err(CompileError::UnsupportedApi(other.to_string())),
    };

    let sub = match c.get("SubAbility") {
        Some(name) if depth < 32 => {
            let sv = face
                .svar(name)
                .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?;
            let sub_ab = ScriptAbility::from_svar(sv)
                .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?
                .map_err(|_| CompileError::MissingSvar(name.to_string()))?;
            Some(Arc::new(compile_depth(face, &sub_ab, depth + 1)?))
        }
        Some(name) => return Err(c.unsupported("SubAbility", name)),
        None => None,
    };

    let unless = compile_unless(&mut c)?;
    let planeswalker = c
        .get("Planeswalker")
        .is_some_and(|v| v.eq_ignore_ascii_case("true"));
    c.get("Ultimate");
    let cycling = c.get("IsCycling").is_some();
    let mut limit = |key: &'static str| -> Result<Option<u32>, CompileError> {
        match c.get(key) {
            None => Ok(None),
            // Mana-Fähigkeiten werden auch beim Bezahlen aktiviert – dort zählt
            // die Engine (noch) nicht mit.
            Some(v) if api == "Mana" => Err(c.unsupported(key, v)),
            Some(v) => v
                .trim()
                .parse()
                .map(Some)
                .map_err(|_| c.unsupported(key, v)),
        }
    };
    let activation_limit = limit("ActivationLimit")?;
    let game_activation_limit = limit("GameActivationLimit")?;
    let activation_zone = match c.get("ActivationZone") {
        None | Some("Battlefield") => crate::zone::Zone::Battlefield,
        Some("Hand") => crate::zone::Zone::Hand,
        Some("Graveyard") => crate::zone::Zone::Graveyard,
        Some(v) => return Err(c.unsupported("ActivationZone", v)),
    };

    let (conditions, cond_keys) =
        crate::expr::parse_conditions(face, &ab.params, crate::expr::CondStyle::Effect)?;
    c.used.extend(cond_keys);

    // Alle übrigen Parameter müssen neutral sein.
    for (k, v) in ab.params.iter() {
        // Bei DelayedTrigger prüft compile_trigger die Trigger-Parameter; die
        // Effekt-Parameter (Bedingungen, Kosten …) müssen hier gelesen sein.
        let elsewhere = c.rest_checked_elsewhere
            && !k.starts_with("Condition")
            && !DELAYED_EFFECT_KEYS
                .iter()
                .any(|e| e.eq_ignore_ascii_case(k));
        let known = elsewhere
            || c.used.iter().any(|u| u.eq_ignore_ascii_case(k))
            || NEUTRAL_PARAMS.iter().any(|n| n.eq_ignore_ascii_case(k));
        if !known {
            return Err(c.unsupported(k, v));
        }
    }

    Ok(CompiledAbility {
        kind: ab.kind,
        api: ab.api.clone(),
        cost,
        target,
        effect,
        sub,
        description,
        sorcery_speed,
        conditions,
        unless,
        planeswalker,
        activation_zone,
        cycling,
        activation_limit,
        game_activation_limit,
    })
}

fn compile_animate(c: &mut Ctx, what: Defined) -> Result<Effect, CompileError> {
    let power = match c.get("Power") {
        Some(v) => Some(c.amount_value("Power", v)?),
        None => None,
    };
    let toughness = match c.get("Toughness") {
        Some(v) => Some(c.amount_value("Toughness", v)?),
        None => None,
    };
    let types: Vec<String> = c
        .get("Types")
        .map(|t| {
            t.split(',')
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect()
        })
        .unwrap_or_default();
    if types.iter().any(|t| {
        t.contains(' ') || t.starts_with("All") || (t.starts_with("Chosen") && t != "ChosenType")
    }) {
        return Err(c.unsupported("Types", c.ab.params.get("Types").unwrap_or("")));
    }
    let remove_creature_types = c
        .get("RemoveCreatureTypes")
        .is_some_and(|v| v.eq_ignore_ascii_case("true"));
    let keywords = c
        .get("Keywords")
        .map(Keyword::parse_list)
        .unwrap_or_default();
    if let Some(k) = keywords.iter().find(|k| !k.is_supported()) {
        return Err(c.unsupported("Keywords", &format!("{k:?}")));
    }
    let overwrite = c
        .get("OverwriteColors")
        .is_some_and(|v| v.eq_ignore_ascii_case("true"));
    let colors_chosen = (c.ab.params.get("Colors") == Some("ChosenColor")).then_some(overwrite);
    let colors = match c.get("Colors") {
        Some("ChosenColor") => None,
        Some(v) => Some((
            forge_types::ColorSet::from_names(v).ok_or_else(|| c.unsupported("Colors", v))?,
            overwrite,
        )),
        None => None,
    };
    let permanent = match c.get("Duration") {
        None => false,
        Some("Permanent") => true,
        Some(v) => return Err(c.unsupported("Duration", v)),
    };
    Ok(Effect::Animate {
        what,
        power,
        toughness,
        types,
        remove_creature_types,
        keywords,
        colors,
        colors_chosen,
        permanent,
    })
}

/// Was kopiert wird (`Choices$` mit Auswahl oder `Defined$`/Ziel).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloneSource {
    Choices {
        zones: Vec<crate::zone::Zone>,
        spec: ValidSpec,
        /// `ChoiceOptional$`
        optional: bool,
    },
    Defined(Defined),
}

fn clone_source(c: &mut Ctx) -> Result<CloneSource, CompileError> {
    if let Some(v) = c.get("Choices") {
        let spec = ValidSpec::parse(v)?;
        let zones = match c.get("ChoiceZone") {
            Some(z) => parse_zones(c, "ChoiceZone", z)?,
            None => vec![crate::zone::Zone::Battlefield],
        };
        let optional = c.get("ChoiceOptional").is_some();
        return Ok(CloneSource::Choices {
            zones,
            spec,
            optional,
        });
    }
    Ok(CloneSource::Defined(c.defined(Defined::Targeted)?))
}

/// Änderungen einer Kopie (`AddTypes$`, `AddKeywords$`, `SetPower$` …).
fn copy_mods(c: &mut Ctx) -> Result<crate::state::CopyMods, CompileError> {
    let words = |v: &str| -> Vec<String> {
        v.split('&')
            .map(|w| w.trim().to_string())
            .filter(|w| !w.is_empty())
            .collect()
    };
    let mut m = crate::state::CopyMods::default();
    if let Some(v) = c.get("AddTypes") {
        m.add_types = words(v);
    }
    if let Some(v) = c.get("SetCreatureTypes") {
        m.set_creature_types = Some(words(v));
    }
    if let Some(v) = c.get("AddKeywords") {
        let kws = crate::keyword::Keyword::parse_list(v);
        if kws.iter().any(|k| !k.is_supported()) {
            return Err(c.unsupported("AddKeywords", v));
        }
        m.add_keywords = kws;
    }
    for (k, slot) in [("SetPower", 0), ("SetToughness", 1)] {
        if let Some(v) = c.get(k) {
            let n: i32 = v.trim().parse().map_err(|_| c.unsupported(k, v))?;
            if slot == 0 {
                m.set_power = Some(n);
            } else {
                m.set_toughness = Some(n);
            }
        }
    }
    m.non_legendary = c
        .get("NonLegendary")
        .is_some_and(|v| v.eq_ignore_ascii_case("true"));
    if let Some(v) = c.get("SetColor") {
        m.set_color = Some(
            forge_types::ColorSet::from_names(&v.replace('&', ","))
                .ok_or_else(|| c.unsupported("SetColor", v))?,
        );
    }
    Ok(m)
}

/// `AtEOT$ Sacrifice|Exile|Destroy|Hand`: verzögerter Trigger zu Beginn des nächsten
/// Endsegments für die gemerkten Objekte (Java `registerDelayedTrigger`).
fn at_eot_trigger(face: &CardFace, kind: &str) -> Option<Arc<crate::triggers::CompiledTrigger>> {
    let exec = match kind {
        "Sacrifice" => "DB$ Sacrifice | Defined$ DelayTriggerRemembered",
        "Exile" => {
            "DB$ ChangeZone | Defined$ DelayTriggerRemembered | Origin$ Battlefield | Destination$ Exile"
        }
        "Hand" => {
            "DB$ ChangeZone | Defined$ DelayTriggerRemembered | Origin$ Battlefield | Destination$ Hand"
        }
        "Destroy" => "DB$ Destroy | Defined$ DelayTriggerRemembered",
        _ => return None,
    };
    let mut f = face.clone();
    f.svars.push(("AtEOTExec".into(), exec.into()));
    let st = ScriptTrigger::parse(forge_script::ParamMap::parse(&format!(
        "Mode$ Phase | Phase$ End of Turn | Execute$ AtEOTExec | TriggerDescription$ At the beginning of the next end step, {kind}."
    )))
    .ok()?;
    crate::triggers::compile_trigger(&f, &st).ok().map(Arc::new)
}

/// Welche Typen `ChooseType` anbietet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeChoiceKind {
    Creature,
    BasicLand,
    Land,
    Card,
}

/// Parameter, die zum `DelayedTrigger`-Effekt selbst gehören (nicht zum Trigger).
const DELAYED_EFFECT_KEYS: &[&str] = &[
    "DB",
    "SP",
    "AB",
    "SpellDescription",
    "StackDescription",
    "Cost",
    "RememberObjects",
    "ThisTurn",
    "NextTurn",
    "SubAbility",
    "AILogic",
    "ValidTgts",
    "TgtPrompt",
    "TargetMin",
    "TargetMax",
    "TgtZone",
    "UnlessCost",
    "UnlessPayer",
    "UnlessSwitched",
    "UnlessResolveSubs",
];

/// `DelayedTrigger` (Java `DelayedTriggerEffect`): die übrigen Parameter
/// bilden einen Trigger, der wie ein `T:`-Trigger kompiliert wird.
fn compile_immediate(c: &mut Ctx, face: &CardFace) -> Result<Effect, CompileError> {
    let mut params = forge_script::ParamMap::default();
    params.insert("Mode", "Immediate");
    for (k, v) in c.ab.params.iter() {
        let effect_level = DELAYED_EFFECT_KEYS
            .iter()
            .chain(["TriggerAmount"].iter())
            .any(|e| e.eq_ignore_ascii_case(k))
            || k.starts_with("Condition");
        if !effect_level {
            params.insert(k, v);
        }
    }
    if !params.has("TriggerDescription")
        && let Some(d) = c.ab.params.get("SpellDescription")
    {
        params.insert("TriggerDescription", d);
    }
    let st = ScriptTrigger::parse(params).map_err(|_| CompileError::MissingParam {
        api: "ImmediateTrigger".into(),
        key: "Mode",
    })?;
    let trigger = Arc::new(crate::triggers::compile_trigger(face, &st)?);
    let remember = match c.ab.params.get("RememberObjects") {
        Some(_) => Some(c.defined_key("RememberObjects", Defined::SelfCard)?),
        None => None,
    };
    let amount = c.amount("TriggerAmount", Some(1))?;
    c.rest_checked_elsewhere = true;
    Ok(Effect::ImmediateTrigger {
        trigger,
        remember,
        amount,
    })
}

fn compile_delayed(c: &mut Ctx, face: &CardFace) -> Result<Effect, CompileError> {
    let mut params = forge_script::ParamMap::default();
    for (k, v) in c.ab.params.iter() {
        let effect_level = DELAYED_EFFECT_KEYS
            .iter()
            .any(|e| e.eq_ignore_ascii_case(k))
            || k.starts_with("Condition");
        if !effect_level {
            params.insert(k, v);
        }
    }
    if !params.has("TriggerDescription")
        && let Some(d) = c.ab.params.get("SpellDescription")
    {
        params.insert("TriggerDescription", d);
    }
    let st = ScriptTrigger::parse(params).map_err(|_| CompileError::MissingParam {
        api: "DelayedTrigger".into(),
        key: "Mode",
    })?;
    let trigger = Arc::new(crate::triggers::compile_trigger(face, &st)?);
    let remember = match c.ab.params.get("RememberObjects") {
        Some(_) => Some(c.defined_key("RememberObjects", Defined::SelfCard)?),
        None => None,
    };
    let this_turn = c.get("ThisTurn").is_some();
    let next_turn = c.get("NextTurn").is_some();
    let scope = match (this_turn, next_turn) {
        (false, false) => DelayScope::Until,
        (true, false) => DelayScope::ThisTurn,
        (false, true) => DelayScope::NextTurn,
        (true, true) => return Err(c.unsupported("NextTurn", "mit ThisTurn")),
    };
    c.rest_checked_elsewhere = true;
    Ok(Effect::DelayedTrigger {
        trigger,
        remember,
        scope,
    })
}

/// `UnlessCost$`/`UnlessPayer$`/`UnlessSwitched$`/`UnlessResolveSubs$`.
fn compile_unless(c: &mut Ctx) -> Result<Option<UnlessSpec>, CompileError> {
    let Some(cost_s) = c.get("UnlessCost") else {
        return Ok(None);
    };
    let cost = Cost::parse(cost_s.trim())?;
    // {T}/{Q} beziehen sich auf die Quelle und ergeben als „Unless“-Kosten keinen Sinn.
    if cost
        .parts
        .iter()
        .any(|p| matches!(p, crate::cost::CostPart::Tap | crate::cost::CostPart::Untap))
        || cost.mana.as_ref().is_some_and(|m| m.x_count() > 0)
    {
        return Err(c.unsupported("UnlessCost", cost_s));
    }
    let payer = if c.ab.params.has("UnlessPayer") {
        c.defined_key("UnlessPayer", Defined::TargetedController)?
    } else {
        Defined::TargetedController
    };
    let switched = c.get("UnlessSwitched").is_some();
    let subs = match c.get("UnlessResolveSubs") {
        None => UnlessSubs::Always,
        Some("WhenPaid") => UnlessSubs::WhenPaid,
        Some("WhenNotPaid") => UnlessSubs::WhenNotPaid,
        Some(v) => return Err(c.unsupported("UnlessResolveSubs", v)),
    };
    // Weitere Unless-Parameter (z. B. `UnlessAI$`) sind KI-Hinweise.
    c.get("UnlessAI");
    Ok(Some(UnlessSpec {
        cost,
        payer,
        switched,
        subs,
    }))
}

impl CompiledAbility {
    /// Hängt `next` als letzte Unterfähigkeit an eine Kopie von `self` an.
    pub fn append(&self, next: Arc<CompiledAbility>) -> CompiledAbility {
        let mut me = self.clone();
        me.sub = Some(match &self.sub {
            Some(s) => Arc::new(s.append(next)),
            None => next,
        });
        me
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_script::CardScript;
    use forge_types::TypeRegistry;

    fn first(txt: &str) -> Result<CompiledAbility, CompileError> {
        let c = CardScript::parse(txt, &TypeRegistry::default()).unwrap();
        compile(c.main(), &c.main().abilities[0])
    }

    #[test]
    fn bolt_and_x() {
        let a = first("Name:B\nManaCost:R\nTypes:Instant\nA:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | SpellDescription$ x\n").unwrap();
        assert_eq!(
            a.effect,
            Effect::DealDamage {
                amount: Amount::Fixed(3),
                to: Defined::Targeted,
                source: None,
            }
        );
        let x = first("Name:F\nManaCost:X R\nTypes:Sorcery\nA:SP$ DealDamage | ValidTgts$ Any | NumDmg$ X\nSVar:X:Count$xPaid\n").unwrap();
        assert_eq!(
            x.effect,
            Effect::DealDamage {
                amount: Amount::XPaid,
                to: Defined::Targeted,
                source: None,
            }
        );
    }

    #[test]
    fn subability_chain() {
        let a = first("Name:C\nManaCost:1 U\nTypes:Instant\nA:SP$ Draw | NumCards$ 1 | SubAbility$ DBGain\nSVar:DBGain:DB$ GainLife | LifeAmount$ 2\n").unwrap();
        assert_eq!(a.chain().len(), 2);
    }

    #[test]
    fn unknown_param_rejected() {
        let e = first("Name:C\nManaCost:1 U\nTypes:Instant\nA:SP$ Draw | NumCards$ 1 | Weird$ 3\n")
            .unwrap_err();
        assert!(matches!(e, CompileError::UnsupportedParam { .. }));
    }

    #[test]
    fn mana_ability() {
        let a = first("Name:E\nManaCost:G\nTypes:Creature Elf\nPT:1/1\nA:AB$ Mana | Cost$ T | Produced$ G | SpellDescription$ Add {G}.\n").unwrap();
        assert!(a.is_mana_ability());
    }
}
