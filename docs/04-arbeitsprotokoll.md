# Arbeitsprotokoll

Chronologisches Protokoll der Portierung: was gemacht wurde, welche
Entscheidungen fielen und was als Nächstes ansteht. Neueste Einträge oben.

---

## 2026-11-01 – Sitzung 2: Branch `rust-godot`, Trigger-System

### Ergebnis

| Bereich | Stand |
|---|---|
| Branch `rust-godot`, Repo auf Rust-Engine + Referenzen reduziert | ✅ (Ziel: Neubau in Godot) |
| Auto-Commit-Hook (`.claude/hooks/autocommit.sh`, lokal) | ✅ |
| Trigger-System (CR 603): ChangesZone, Phase, Attacks, SpellCast, DamageDone | ✅ Grundformen |
| Rückblick beim Verlassen des Battlefields inkl. gleichzeitiger Tode | ✅ |
| APNAP, Trigger ordnen, Ziele beim Stapeln, optionale Trigger | ✅ |
| Tests | 95 (9 neue Trigger-Tests), 1.000 Zufallspartien ohne Absturz |
| Unterstützte Karten im Zielpool | 3.738 von 32.336 (11,6 %, vorher 8,4 %), Pauper 22,2 %; nach ChangeZone 4.493 (13,9 %) |

### Entscheidungen

- Trigger werden **zum Ereigniszeitpunkt** geprüft (nicht erst vor der Priorität), damit
  „last known information“ ohne Kopien funktioniert: das alte Objekt bleibt in der Arena.
- Bedingungen wie `IsPresent$`, `CheckSVar$`, `Static$` machen die Karte vorerst „nicht
  unterstützt“, statt falsch zu triggern.

### Nachtrag: ChangeZone / ChangeZoneAll

- Bounce, Exil, Reanimation (GainControl), Bibliothekssuche (Tapped, Shuffle; verdeckte
  Zonen dürfen „nicht finden“, öffentliche nicht), unten in die Bibliothek, Massen-Bounce.
- Ziele in Friedhof/Exil/Hand; Fluchsicherheit wirkt dort nicht.
- Korrekturen nach Java-Abgleich: ohne `Defined$` gilt `Self`; Unterfähigkeiten erben
  Ziele der Elternfähigkeit; `ParentTarget`.
- 9 neue Tests (104 gesamt), 1.000 Zufallspartien ohne Absturz.
- Unterstützung Zielpool: 11,6 % → **13,9 %** (Pauper 25,2 %).

### Nachtrag: Static Continuous + Layer-System

- `statics.rs`: `Affected$`, `AffectedDefined$ Self`, `AffectedZone$`, `EffectZone$`,
  `AddType`, `SetColor`, `AddKeyword`, `SetPower/SetToughness`, `AddPower/AddToughness`
  (feste Werte oder `Number$`).
- Layer 4/5/6/7b/7c mit Zeitstempeln, betroffene Menge nach CR 613.6.
- 9 neue Tests (114 gesamt), 1.000 Zufallspartien ohne Absturz.
- Unterstützung: 13,9 % → **14,7 %**. Kleiner Zuwachs, weil die meisten übrigen
  Statics Bedingungen (`IsPresent$`, `Condition$`, `CheckSVar$`), `X`-Werte
  (`Count$…`) oder Auren/Ausrüstung (`AffectedDefined$ Enchanted/Equipped`, 1.010×) brauchen.

### Nachtrag: Ausdrücke und Bedingungen

- `expr.rs`: `Count$`-Zähler mit Rechenoperationen, Vergleiche (`GE3`, `EQX`),
  Bedingungen für Statics, Trigger (intervening if, CR 603.4) und Effekte.
- CDAs (`CharacteristicDefining$`, Layer 7a, alle Zonen) – Karten mit `*`-P/T spielbar.
- Filter: `powerGE4`, `toughnessLE2`, `cmcEQ3`, `StrictlySelf`; `EffectZone$ All`.
- 8 neue Tests (123 gesamt), 1.000 Zufallspartien ohne Absturz.
- Unterstützung: 14,7 % → **17,4 %** (Pauper 29,0 %, Modern 18,8 %).
- **Bekanntes Problem – Performance:** Der Lasttest dauert ~4× länger (403 s statt 107 s
  für 1.000 Partien), weil `chars()` dynamische Statics bei jeder Abfrage neu auswertet.
  Lösung später: Eigenschaften pro Zustandsversion cachen.

### Nachtrag: Token und Charm

- Token: Skripte aus `res/tokenscripts`, Besitzer/Controller, getappt, X/X-Werte, ETB-Trigger,
  Treasure & Co. als Mana-Quelle; Token verschwinden außerhalb des Battlefields (704.5d).
- Charm: Moduswahl beim Wirken/Aktivieren/Stapeln, Druckreihenfolge, Wiederholung erlaubt,
  Modi ohne legale Ziele nicht wählbar.
- Lasttest fand einen Fehler in der Test-KI (wiederholte Modi), behoben.
- 8 neue Tests (131 gesamt), 1.000 Zufallspartien ohne Absturz.
- Unterstützung: 17,4 % → **19,8 %** (Pauper 32,0 %, Modern 21,6 %).
- Repo **github.com/wowseer/forge-rules-rs** eingerichtet; jeder Arbeitsschritt wird gepusht.

### Nachtrag: Ersatzeffekte (Moved)

- `replacements.rs`: Umleitung (Rest in Peace), getappt ins Spiel (auch Check-Lands mit
  Bedingung), `etbCounter` (auch X). Angewendet beim Zonenwechsel vor dem Ankommen,
  also vor ETB-Triggern; je Ereignis höchstens einmal (614.5).
- Zonenwechsel setzt den Controller jetzt beim Ankommen (Zauber, Land, „unter deiner
  Kontrolle“) statt danach – ETB-Trigger sehen den richtigen Controller.
- Filter: `!`-Negation (`Card.!token`), `OppOwn`.
- Vereinfachung: Mehrere Umleitungen → Zeitstempel statt Wahl des Betroffenen (CR 616.1).
- 4 neue Tests (135), 300 Zufallspartien; Zielpool 19,8 % → **21,1 %**.

### Nächste Schritte

1. **Effect** (367), **PumpAll** (313), **Discard** (288),
   **Counter** (284), **Animate**, **ReduceCost**, **Dig**, **DamageAll**.
2. **Replacement-Effekte** (`Moved`: 394, ETB getappt usw.).
3. **Auren und Ausrüstung**: `Enchant`, `Attach`, `Equip`, `AffectedDefined$ Enchanted/Equipped`,
   SBA 704.5m/n/p.
3. **Charm** (modale Zauber), **Token**, **Effect**, **Discard**, **Counter**, **PumpAll**.
4. Trigger-Bedingungen (`IsPresent`, `CheckSVar`) und `Count$`-Ausdrücke.

---

## 2026-11-01 – Sitzung 1: Fundament, Parser, Engine-Kern

### Ergebnis

| Bereich | Stand |
|---|---|
| Workspace `rust/` mit `forge-types`, `forge-script`, `forge-engine` | ✅ |
| Parser für **alle** 34.135 Kartenskripte + Token-Skripte, fehlerfrei | ✅ (`forge-script/tests/corpus.rs`) |
| Editionen/Formate → Zielkartenpool (7 Formate + Booster Draft/Cube nutzen dieselben Karten) | ✅ 32.336 Karten |
| Effekt-Compiler mit „nie still ignorieren“ | ✅ 10 ApiTypes in Grundform |
| Engine: Zugstruktur, Priorität/Stapel, Mulligan, Kampf, SBAs, Kosten, Mana | ✅ |
| Regeltests | 72 Tests (Engine) + Parser-/Typ-Tests, alle grün |
| Lasttest | 1.000 Zufallspartien mit echten Karten ohne Absturz (`SOAK_GAMES=1000`) |
| Unterstützte Karten im Zielpool | 2.725 von 32.336 (8,4 %), Pauper 17 % |
| xlsx-Tracker | 40 Regeln `TRUE`, 20 teilweise, 40 `n/a` (Sonderformate) |

### Entscheidungen

- **Status `TRUE` in den xlsx-Dateien** heißt: umgesetzt und durch Regeltests gegen die
  Comprehensive Rules abgesichert. Der Abgleich mit der Java-Engine (Differential-Tests)
  ist noch nicht möglich, weil auf dem Entwicklungsrechner kein Maven installiert ist
  (`mvn: command not found`). Er wird nachgeholt, sobald der Java-Build läuft (siehe unten).
- **Kampfschaden nach Foundations-Regeln** (2024): keine Schadenszuweisungsreihenfolge,
  der Angreifer verteilt frei. Trample verlangt tödlichen Schaden an alle Blocker.
- **Ausgeschlossene Typen** (Scheme, Plane, Phenomenon, Vanguard, Conspiracy, Stickers)
  werden erkannt und aus dem Pool genommen, auch wenn sie in Papier-Editionen
  gedruckt wurden.
- **Commander/Pauper ohne Set-Liste**: nur Drucke aus Papier-Editionen zählen
  (`Funny`/`Online` nicht), Alchemy-`A-`-Karten sind ausgeschlossen.
- **Auto-Bezahlung** (`PayAction::AutoPay`) nutzt nur einfache `{T}: ein Mana`-Quellen.
  Komplexere Quellen müssen manuell über `ActivateMana` aktiviert werden.

### Bekannte Lücken und Vereinfachungen

- **Trigger, Statics, Replacements**: noch gar nicht vorhanden. Das ist der größte
  Blocker (siehe `generated/unterstuetzung.md`: `Trigger: ChangesZone` allein betrifft
  6.701 Karten).
- **Layer-System** nur 6 + 7c.
- **Kampf**: nur Spieler als Verteidiger (keine Planeswalker/Battles), keine
  Angriffs-/Blockpflichten, kein Banding.
- **Mulligan**: kein freier erster Mulligan (Commander/Multiplayer). Alle Spieler
  legen direkt nach dem Behalten ihre Karten unter die Bibliothek (in Java ebenso
  sequenziell).
- **Snow-Mana**: Auto-Bezahlung berücksichtigt `{S}` nicht.
- **Performance**: Jeder atomare Schritt klont den ganzen Zustand inklusive Protokoll.
  Ca. 0,1 s pro Zufallspartie (Release). Optimierung später, z. B. Protokoll außerhalb
  des Zustands oder persistente Collections.
- **Commander-Regeln** (Command Zone, Commander-Steuer, 21 Schaden, Farbidentität)
  fehlen noch.
- **Mehrseitige Karten** (DFC, Split, Adventure, Modal …) sind als „nicht unterstützt“
  markiert.

### Nächste Schritte (nach Hebelwirkung)

1. **Trigger-System** (CR 603): `ChangesZone` (ETB/Dies), `Phase`, `Attacks`, `SpellCast`,
   `DamageDone`. Dazu APNAP-Reihenfolge und Zielwahl beim Auf-den-Stapel-Legen.
2. **ChangeZone/ChangeZoneAll** (Bounce, Exile, Reanimate, Tutor) und **Token**
   (Token-Skripte aus `res/tokenscripts` sind bereits parsbar).
3. **Static `Continuous` + Layer-System** vollständig (CR 613 inkl. Abhängigkeiten).
4. **Replacement-Effekte** (`Moved`, `DamageDone`, ETB-Tapped `ETBReplacement`).
5. **Auren/Equipment** (`Enchant`, `Attach`, SBA 704.5m/n/p).
6. **Charm** (modale Zauber), **Counter**, **Discard**, **Dig**, **PumpAll**, **Effect**.
7. **Java-Orakel**: Maven installieren, headless Recorder in Java (Seed, Decks,
   Entscheidungen, normalisierte State-Dumps je Schritt) und Replay in
   `forge-harness`. Danach den Status bestehender Regeln mit Differential-Tests nachprüfen.

Priorität laut Bericht „Einziger fehlender Baustein“: ChangesZone-Trigger (3.177 Karten),
ChangeZone-Effekt (1.030), Static Continuous (981), Phase-Trigger (809), SpellCast-Trigger (607).
