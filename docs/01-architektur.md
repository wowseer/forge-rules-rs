# Architektur der Rust-Engine

Dieses Dokument beschreibt den Aufbau der Rust-Portierung und die Gründe für die
wichtigsten Entscheidungen. Verbindliche Projektregeln stehen in `CLAUDE.md` im Repo-Root.

## Überblick

```
forge-gui/res/cardsfolder/*.txt      (34.135 Kartenskripte, unverändert aus Forge)
forge-gui/res/editions, formats      (Editionen, Formatlisten)
              │
              ▼
┌──────────────────┐   ┌─────────────────────┐   ┌──────────────────────────────┐
│  forge-types     │◄──│  forge-script       │◄──│  forge-engine                │
│  Farben, Mana,   │   │  Parser: Datei →    │   │  Effekt-Compiler, Zustand,   │
│  Kartentypen     │   │  CardScript, Params,│   │  Regeln, Engine-Treiber,     │
│                  │   │  Editionen/Formate  │   │  Test-KI, Berichte           │
└──────────────────┘   └─────────────────────┘   └──────────────────────────────┘
```

| Crate | Inhalt | Java-Entsprechung |
|---|---|---|
| `forge-types` | `Color`, `ColorSet`, `ManaShard`, `ManaCost`, `CardType`, `TypeRegistry` | `forge-core`: `MagicColor`, `ColorSet`, `ManaCost*`, `CardType` |
| `forge-script` | `CardScript`/`CardFace` (Dateiparser), `ParamMap` (`K$ V \| …`), Klassifikation der `A:/T:/S:/R:`-Zeilen, `corpus` (alle Dateien laden), `formats` (Legalität) | `CardRules.Reader`, `FileSection`, `CardEdition`, `GameFormat` |
| `forge-engine` | alles mit Spielzustand, siehe unten | `forge-game` |

## Grundprinzipien

### 1. Daten statt Code
Die Karten bleiben Forge-Skripte. Rust implementiert einen **Interpreter** für die
Skriptsprache: jede ApiType, jeder Trigger-Modus, jede Static-Art usw. ist ein
Baustein. Die Liste dieser Bausteine ist genau die Liste in den xlsx-Trackern.

### 2. Kompilieren beim Laden, nie still ignorieren
`CardDefinition::compile` übersetzt jede Fähigkeit **einmal** in einen typisierten
`CompiledAbility` (`effects/mod.rs`). Unbekannte ApiTypes, Parameter, Filter oder
Kosten erzeugen einen Eintrag in `CardDefinition::unsupported`. Solche Karten sind
nicht spielbar (`legal_actions` bietet sie nicht an). So kann eine Karte nie
„halb“ funktionieren. Java dagegen parst Strings zur Laufzeit neu (`AbilityUtils`).

Parameter, die nur Text oder KI-Hinweise sind, stehen in `NEUTRAL_PARAMS` und dürfen
ignoriert werden. Alle anderen müssen ausgewertet werden.

### 3. Ein Zustand, nur IDs
`GameState` (`state.rs`) ist ein einziger Wert mit `#[derive(Clone)]`:

- `cards: Vec<CardObj>` als Arena, `CardId` ist der Index.
- Spieler-Zonen (`library`, `hand`, `graveyard`) als ID-Listen im `Player`;
  gemeinsame Zonen (`battlefield`, `exile`, `command`) im `GameState`; der Stapel
  als `Vec<StackItem>`.
- **CR 400.7**: Ein Zonenwechsel erzeugt ein *neues* Objekt mit neuer ID
  (`move_card`). Das alte Objekt bleibt mit `zone = None` und `moved_to`
  erhalten, also als „last known information“. Effekte, die an der alten ID
  hängen (z. B. Pump bis Zugende), gelten dadurch automatisch nicht mehr.
- Kartendefinitionen werden per `Arc<CardDefinition>` geteilt, damit sind Kopien billig.
- `this_turn: TurnHistory` hält die Zugstatistik (Zonenwechsel, gewirkte Zauber,
  Angreifer), `Player` zusätzlich Lebensgewinn/-verlust des Zuges; beides wird in
  `start_turn` geleert und von `Count$ThisTurn…`, Morbid u. a. gelesen.
- `StackItem::paid` merkt die als Kosten bezahlten Objekte (LKI) für `Sacrificed$…`.

### 4. Die Engine ruft niemanden auf
Java fragt Spieler über `PlayerController` (113 Methoden, blockierend). Hier liefert
die Engine stattdessen eine `Decision` und wartet auf eine `Choice`:

```rust
let mut engine = Engine::new(GameState::new(rules, decks, seed, None));
let mut status = engine.advance();
while let Status::NeedsDecision(d) = status {
    let choice = /* GUI, KI, Test, Replay */;
    status = engine.submit(choice)?;   // prüft die Wahl, läuft weiter
}
```

**Wiederaufsetzpunkte** (`engine.rs`): `GameState::step_once` führt genau einen
atomaren Übergang aus (`Flow`: Setup → Mulligan → BeginStep → Priority → EndStep …).
Er läuft immer auf einer Kopie des letzten stabilen Zustands. Fehlt unterwegs eine
Antwort, wird die Kopie verworfen und die Entscheidung zurückgegeben. Mit der
Antwort wird derselbe Schritt deterministisch bis zur selben Stelle wiederholt und
kommt diesmal weiter. Daraus folgt:

- keine Callbacks, Threads oder Locks;
- **Abbrechen** (`PayAction::Cancel`, CR 728 „Rücknahme“) verwirft einfach die Kopie;
- **ungültige Antworten** (z. B. ein Block, den Flying verbietet) werden abgelehnt,
  und der Zustand bleibt unverändert;
- jede Partie ist aus Seed und `Engine::history` reproduzierbar.

### 5. Determinismus
`Rng` (SplitMix64) liegt im Zustand. Mischen und Startspieler hängen nur vom Seed ab.
Regelrelevante Iterationen laufen über geordnete Strukturen (`Vec`, `BTreeMap`).
Der Test `turn_structure::deterministic_with_same_seed` prüft das.

## Module von `forge-engine`

| Modul | Aufgabe | CR |
|---|---|---|
| `ids`, `zone`, `rng` | Grundtypen | 400 |
| `keyword` | typisierte Keywords (statische Kampf-/Schutz-Keywords) | 702 |
| `valid` | Filterausdrücke `Creature.nonBlack+YouCtrl,Planeswalker` | – |
| `cost` | `Cost$`-Parser (Mana, T, Q, Sac, PayLife, Discard) | 118, 602 |
| `effects/mod.rs` | Effekt-Compiler (`ScriptAbility` → `CompiledAbility`) | – |
| `effects/resolve.rs` | Effekte ausführen, Stapelobjekte auflösen, Fizzle | 608 |
| `definition` | `CardDefinition` (kompiliert), `CardDb` | – |
| `state` | `GameState`, `Player`, `CardObj`, `StackItem`, `Combat`, `Characteristics` (Layer) | 613 |
| `actions` | Zonenwechsel, Ziehen, Schaden, Leben, Zerstören, Marken | 120, 121, 701 |
| `targeting` | Filter auswerten, legale Ziele, Hexproof/Shroud | 115 |
| `casting` | legale Aktionen, Land spielen, wirken, aktivieren, Kosten und Mana bezahlen, Auto-Bezahlung | 117, 305, 601, 602, 605 |
| `combat` | Angreifer (Ziel: Spieler oder Planeswalker, `AttackerInfo::target`), Blocker, Kampfschaden | 506–511 |
| `replacements` | Ersatzeffekte `Moved` (Umleitung, getappt, mit Marken), angewendet in `move_card_as` vor dem Ankommen | 614 |
| `sba` | zustandsbasierte Aktionen | 704 |
| `expr` | Ausdrücke (`Count$…`, `Number$`, Rechenoperationen) und Bedingungen (`IsPresent$`, `CheckSVar$`, `Condition$`, `ConditionPresent$` …) | 603.4, 608.2c |
| `statics` | statische Fähigkeiten (`S:Mode$ Continuous`) kompilieren | 604 |
| `triggers` | Trigger kompilieren (`T:`), Ereignisse (`fire`), Rückblick beim Verlassen des Battlefields, APNAP-Stapeln, optionale Trigger | 603 |
| `flow` | Aufbau, Mulligan, Schritte, Priorität | 103, 117, 500–514 |
| `engine` | Treiber, `Controller`-Trait, `run_game` | – |
| `simple_ai` | seedbare Zufalls-KI für Lasttests (kein Ersatz für `forge-ai`) | – |
| `bin/support-report` | Bericht: Anteil unterstützter Karten im Zielpool | – |

## Trigger (CR 603)

Spielaktionen melden Ereignisse (`TriggerEvent`: Zonenwechsel, Schrittbeginn,
Angriff, Zauber gewirkt, Schaden) über `GameState::fire`. Dort werden **sofort**
alle Trigger in ihren aktiven Zonen geprüft und als `PendingTrigger` mit dem
Wissen zum Ereigniszeitpunkt (`TriggerInfo`) vorgemerkt. Für das Verlassen des
Battlefields zählen auch das alte Objekt und – bei gleichzeitigen Zonenwechseln
der SBAs – das ganze vorherige Battlefield (`lki_battlefield`, CR 603.10a).
Bevor jemand Priorität bekommt, legt `put_triggers_on_stack` sie in
APNAP-Reihenfolge auf den Stapel; mehrere eigene Trigger ordnet der Spieler
(`Decision::OrderTriggers`). Ziele werden dabei gewählt; ohne legale Ziele
entfällt der Trigger (603.3d). „You may“ (`OptionalDecider$ You`) wird beim
Auflösen gefragt (`Decision::OptionalTrigger`). Effekte erreichen den Kontext
über `Defined$ Triggered…`.

**Verzögerte Trigger** (`DelayedTrigger`, CR 603.7) liegen in
`GameState::delayed_triggers`, sind an keine Zone gebunden, werden in `fire` nach
den normalen Triggern geprüft und nach dem ersten Auslösen entfernt.
**Reflexive Trigger** (`ImmediateTrigger`, CR 603.12) werden beim Auflösen direkt
als `PendingTrigger` eingereiht. **Sagas** (CR 714): Kapitel sind `CounterAdded`-
Trigger mit `chapter = true`; die SBA 714.4 prüft wartende Trigger und Stapel.
**Gleichzeitiger Schaden** (Kampf, `DealDamage`, `DamageAll`) wird über
`begin_damage_batch`/`end_damage_batch` gebündelt; `DamageDoneOnce` löst danach
einmal je Empfänger aus. Ausdrücke sehen über `EvalCtx` die Ziele der
auflösenden Fähigkeit und den Trigger-Kontext (`TriggerInfo`), auch schon beim
Prüfen von Trigger-Bedingungen.

**Filterkontext:** Filter wie `Creature.ControlledBy TriggeredTarget`,
`TargetedPlayerCtrl`, `IsTriggerRemembered` oder `Targeted.sameName` brauchen die
Fähigkeit, gegen die geprüft wird (Java übergibt dazu die `SpellAbility`). Statt
diesen Kontext durch alle Filteraufrufe zu reichen, setzt die Engine ihn als
thread-lokalen `targeting::FilterCtx` (Trigger-Info, bisher gewählte Ziele, vom
verzögerten Trigger gemerkte Objekte) über einen Guard (`FilterCtxGuard`), der
beim Verlassen des Gültigkeitsbereichs – auch bei `Interrupt` – den vorigen
Kontext wiederherstellt. Gesetzt wird er beim Stapeln von Triggern, je Teil der
Zielwahl (`choose_targets`), beim Auflösen eines Stapelobjekts und beim Prüfen
verzögerter Trigger. Der Kontext ist kein Spielzustand und nie Teil von
`GameState`.

`ActivationLimit$` bei Triggern zählt Auslösungen je (Quelle, Trigger) in
`TurnHistory::trigger_counts`.

Aktive Zonen wie in Java: `TriggerZones$`, sonst Battlefield; Selbst-ETB
(`ValidCard$ Card.Self`, `Origin$ Any`) aktiv in der Zielzone, Verlassen des
Battlefields immer Battlefield.

## Layer-System (Stand)

`GameState::chars(id)` berechnet die Eigenschaften bei jeder Abfrage neu:

1. Basiswerte der Kartenseite.
2. Wirkende statische Fähigkeiten sammeln: Quelle in einer ihrer `EffectZone$`
   (Standard Battlefield), Objekt in einer `AffectedZone$` (Standard Battlefield).
   Zeitstempel = Zeitstempel der Quelle (CR 613.7a).
3. Layer 4 (Typ hinzufügen, Kreatur-/Landtypen entfernen) → 5 (Farbe setzen) →
   6 (Keywords, verliehene Fähigkeiten/Trigger, „verliert alle Fähigkeiten“;
   statisch und Pump-Effekte gemeinsam nach Zeitstempel) → 7b (P/T setzen) →
   7c (P/T ändern, ±1/±1-Marken).
   Aktivierte Fähigkeiten und Trigger eines Objekts liefern `GameState::activated`
   bzw. `all_triggers` (eigene, sofern nicht verloren, plus verliehene).
   Intrinsische Mana-Fähigkeiten der Standardlandtypen (CR 305.6) sind geteilte
   Objekte (`definition::intrinsic_mana`) und folgen den aktuellen Landtypen.
4. Ob ein Objekt betroffen ist, wird im ersten Layer entschieden, in dem der Effekt
   wirkt, gegen die Zwischen-Eigenschaften (CR 613.6) – deshalb
   `card_matches_with(id, &chars, …)` statt `card_matches`.

Dynamische Werte (`AddPower$ X` mit `Count$…`) und Bedingungen werden bei jeder
Abfrage ausgewertet. Rekursionsschutz (`expr::DepthGuard`): Innerhalb einer solchen
Auswertung ignorieren verschachtelte `chars()`-Aufrufe dynamische Statics. „+1/+1 für
jede Kreatur“ zählt also Kreaturen, ohne sich selbst endlos auszuwerten.
CDAs (`CharacteristicDefining$ True`) gelten in allen Zonen in Layer 7a.

Fehlt noch: Layer 3 (Text), Abhängigkeiten (613.8); statische Fähigkeiten und
Ersatzeffekte eines Objekts, das alle Fähigkeiten verloren hat, wirken noch weiter.

## Tests

| Ort | Art |
|---|---|
| `*/src/**` `#[cfg(test)]` | Unit-Tests der Parser und Datentypen |
| `forge-script/tests/corpus.rs` | **alle** Kartenskripte und Token-Skripte müssen parsen |
| `forge-engine/tests/turn_structure.rs`, `sba.rs`, `combat.rs`, `spells.rs` | Regeltests je CR-Regel mit kleinen Testkarten (`tests/common/mod.rs`) |
| `forge-engine/tests/soak.rs` | Zufallspartien mit echten Karten; prüft Invarianten (jede ID in genau einer Zone) und dass Partien enden. `SOAK_GAMES=1000 cargo test --release --test soak` |

Jede Regel, die in `tools/regelstatus.csv` als `ja` markiert ist, nennt dort ihren Test.
