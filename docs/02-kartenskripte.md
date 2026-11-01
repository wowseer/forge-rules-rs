# Kartenskripte: Format und Verarbeitung

Die Rust-Engine liest die Forge-Kartenskripte unverändert. Hier steht, wie das
Format aufgebaut ist und wo es in Rust verarbeitet wird.

## Datei (`forge-script/src/card.rs`)

```
Name:Lightning Bolt
ManaCost:R
Types:Instant
A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | SpellDescription$ CARDNAME deals 3 damage to any target.
Oracle:Lightning Bolt deals 3 damage to any target.
```

| Schlüssel | Bedeutung | Rust |
|---|---|---|
| `Name`, `ManaCost`, `Types`, `PT`, `Loyalty`, `Defense`, `Colors`, `Oracle`, `Text` | Eigenschaften der aktuellen Seite | `CardFace` |
| `K:` | Keyword (roh) | `CardFace::keywords` → `keyword::Keyword` |
| `A:` | Fähigkeit (`SP$`, `AB$`, selten `ST$`) | `ScriptAbility` |
| `T:` / `S:` / `R:` | Trigger / Static / Replacement, Pflichtschlüssel `Mode$` bzw. `Event$` | `ScriptTrigger` / `ScriptStatic` / `ScriptReplacement` |
| `SVar:Name:Wert` | Variablen: Unterfähigkeiten (`DB$ …`) oder Ausdrücke (`Count$…`) | `CardFace::svars` |
| `AlternateMode:` | Art der Mehrseitigkeit (DoubleFaced, Split, Adventure, Modal, Flip, Meld, …) | `SplitMode` |
| `ALTERNATE` | ab hier Seite 2 | `faces[1]` |
| `SPECIALIZE:WHITE` … | Seiten 3–7 (Alchemy-Specialize) | `faces[2..=6]` |
| `Variant:<Name>:<Zeile>` | funktionale Variante (z. B. Universes-Within-Namen) | `CardScript::variants` |
| `CopyFaceFrom:` | Platzhalter, Seite wird von anderer Karte kopiert | aufgelöst in `corpus::parse_dir` |
| `DeckHints`, `DeckNeeds`, `DeckHas`, `AI:` | nur für Deckbau/KI | gespeichert, keine Regelwirkung |

## Parameterlisten (`forge-script/src/params.rs`)

`Key$ Value | Key2$ Value2`. Genau wie Java (`FileSection.parseToMap`):

- **Schlüssel sind case-insensitive** (`NumDmg` = `numdmg`);
- Werte werden getrimmt, ein Schlüssel ohne `$` bekommt den Wert `""`;
- bei doppelten Schlüsseln gewinnt der letzte.

## Manakosten (`forge-types/src/mana.rs`)

Token durch Leerzeichen getrennt; Zahlen = generisch. Hybrid ohne Schrägstrich
(`WU`, `2R`, `BP`, `RWP`, `CG`) oder mit (`W/U`). `no cost` heißt „keine
Manakosten“ (CR 202.1b) und ist nicht dasselbe wie `{0}`.

## Typzeile (`forge-types/src/card_type.rs`)

Wörter werden in Supertyp, Kartentyp oder Untertyp einsortiert. Mehrwortige
Untertypen („Time Lord“) kommen aus `res/lists/TypeLists.txt`. Unbekannte Wörter
werden wie in Java als Untertyp übernommen. `Tribal` wird als `Kindred` gelesen.

## Vom Skript zum Effekt (`forge-engine/src/effects/mod.rs`)

```
ScriptAbility { kind: Spell, api: "DealDamage", params }
    │  compile()
    ▼
CompiledAbility {
    effect: Effect::DealDamage { amount: Amount::Fixed(3), to: Defined::Targeted },
    target: Some(TargetSpec { valid: "Any", min: 1, max: 1 }),
    cost, sub (SubAbility$-Kette), description, sorcery_speed,
}
```

- `SubAbility$ DBName` verweist auf eine SVar mit `DB$ …` und wird rekursiv zu
  einer Kette kompiliert (`CompiledAbility::chain`).
- Zahlen (`NumDmg$ 3`, `NumAtt$ +2`, `X` mit `SVar:X:Count$xPaid`, `Number$N`)
  werden zu `Amount`. Andere `Count$`-Ausdrücke sind noch nicht unterstützt.
- `Defined$` (You, Self, Targeted/ParentTarget, TargetedController, Player,
  Opponent, Triggered…) wird zu `Defined`. Ohne `Defined$` gilt `Targeted`, falls
  `ValidTgts$` existiert; sonst bei Karten-Effekten `Self` (wie Java
  `AbilityUtils.getDefinedCards`), bei Spieler-Effekten `You`.
- Unterfähigkeiten ohne eigene Ziele erben die Ziele der Elternfähigkeit
  (`Targeted`, `TargetedController` in `SubAbility$`-Ketten).
- Ziele liegen standardmäßig auf dem Battlefield; `TgtZone$` bzw. bei
  `ChangeZone` die `Origin$`-Zone erlauben andere Zonen (z. B. Friedhof).
- Filterwörter (`Land.Basic`, `Creature.Elf`) werden gegen `TypeLists.txt`
  geprüft; unbekannte Wörter machen die Karte „nicht unterstützt“.
- Filter (`ValidTgts$`, `Sac<1/Creature.Other>`) werden mit `valid::ValidSpec` geparst.

## Neue Effekte hinzufügen (Checkliste)

1. Java-Quelle lesen: Pfad steht in der xlsx-Spalte „Java Source“
   (relativ zu `forge-game/src/main/java/forge/game/`).
2. Häufigste Parameterkombinationen im Kartenbestand prüfen, z. B.
   `grep -rhoE 'DB\$ Discard[^|]*(\|[^|]*)*' forge-gui/res/cardsfolder | …`
   oder `docs/generated/kartenbestand.md`.
3. Variante in `Effect` ergänzen, Parameter in `compile_depth` auslesen (jeden
   gelesenen Schlüssel über `Ctx::get`, damit der Rest-Check greift).
4. Ausführung in `effects/resolve.rs::apply_effect`.
5. Regeltests in `forge-engine/tests/` mit kleinen Testkarten schreiben.
6. `tools/regelstatus.csv` aktualisieren, `python3 rust/tools/sync_regelstatus.py`
   ausführen und die Berichte neu erzeugen (siehe `README.md`).
