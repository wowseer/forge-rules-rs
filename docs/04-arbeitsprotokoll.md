# Arbeitsprotokoll

Chronologisches Protokoll der Portierung: was gemacht wurde, welche
Entscheidungen fielen und was als Nächstes ansteht. Neueste Einträge oben.

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
