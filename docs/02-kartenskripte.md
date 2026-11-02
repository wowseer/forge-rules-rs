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
- Zahlen (`NumDmg$ 3`, `NumAtt$ +2`, SVar-Namen) werden zu `Amount`; SVars werden mit
  `expr::Expr` übersetzt: `Number$N`, `Count$xPaid`, `Count$Valid[Zone] <Filter>`,
  `CardPower`, `CardToughness`, `CardCounters.<Typ>`, `YourLifeTotal`,
  `YourStartingLife`, `CardsInYourHand`, `Domain`, jeweils mit einer Rechenoperation
  `/Plus.N`, `/Minus.N`, `/Times.N`, `/Twice`, `/HalfUp` … (wie Java `doXMath`).
- Ereignis- und Zugwerte: `TriggerCount$DamageAmount`/`LifeAmount` (Menge des
  auslösenden Ereignisses, `TriggerInfo::amount`); bezahlte Kostenobjekte
  `Sacrificed$…`, `Discarded$…`, `Exiled$…` (LKI, `StackItem::paid`);
  Zugstatistik (`GameState::this_turn`, zu Zugbeginn geleert):
  `Count$LifeYouGainedThisTurn`, `LifeYouLostThisTurn`, `LifeOppsLostThisTurn`,
  `YouDrewThisTurn`, `AttackersDeclared`, `ThisTurnCast_<Filter>`,
  `ThisTurnEntered_<Ziel>[_from_<Herkunft>]_<Filter>` (Filter auf das Objekt vor dem
  Wechsel) und `Morbid.<ja>.<nein>`.
- Kostenteile: `T`, `Q`, `Sac<…>`, `PayLife<n>`, `Discard<n/Typ>`, `Exile<1/CARDNAME>`,
  `ExileFromGrave<n/Typ>`, `tapXType<n/Typ>` (ohne die Quelle, wenn `{T}` dabei ist),
  `Return<n/Typ>`, `AddCounter`/`SubCounter`; in Typfiltern trennt `;` Alternativen.
  Bezahlte Objekte stehen als `Sacrificed$`, `Discarded$`, `Exiled$`, `Tapped$`,
  `Returned$` zur Verfügung.
- `ActivationLimit$ N` / `GameActivationLimit$ N` (je Zug bzw. Partie, gezählt je Objekt
  in `CardObj::activations`; bei Mana-Fähigkeiten noch nicht unterstützt).
- Sagas: `K:Chapter:<N>:<SVar1>,…` erzeugt je Kapitel einen `CounterAdded`-Trigger
  (`CounterType$ LORE | CounterAmount$ EQi`), eine Lore-Marke beim Ankommen und
  `final_chapter` für SBA 714.4. `ImmediateTrigger` erzeugt einen reflexiven Trigger
  (`Mode$ Immediate`) mit `TriggerAmount$`.
- Zusatzkosten von Zaubern (`A:SP$ … | Cost$ 1 R Sac<1/Creature>`): die Nicht-Mana-Teile
  müssen zum Wirken zahlbar sein und werden nach dem Mana bezahlt (Java `Cost.sort`).
- Bedingungen: Statics/Trigger `IsPresent$`/`PresentCompare$`/`PresentZone$`/
  `PresentPlayer$`, `CheckSVar$`/`SVarCompare$`, `Condition$` (PlayerTurn, Threshold, …),
  `PlayerTurn$`, `OpponentTurn$`; Effekte `ConditionPresent$`, `ConditionCheckSVar$` …
  Trigger prüfen beim Auslösen und – außer bei `NoResolvingCheck$` – beim Auflösen (CR 603.4).
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
- `UnlessCost$` gilt für **jede** Fähigkeit (`CompiledAbility::unless`, Java
  `AbilityUtils.handleUnlessCost`): Kosten im `Cost$`-Format ohne `T`/`Q`/X
  (Mana, `PayLife<n>`, `Sac<n/Typ>`, `Discard<n/Typ>`). `UnlessPayer$` (Standard
  `TargetedController`) wird nacheinander gefragt, aber nur, wer zahlen kann
  (`Decision::PayUnlessCost`); zahlt einer, entfällt der Effekt. `UnlessSwitched$`
  kehrt das um, `UnlessResolveSubs$ WhenPaid|WhenNotPaid` steuert die Unterfähigkeiten.
- `Sacrifice`: `SacValid$` (Standard `Self`), `Defined$` (wer opfert, Standard du),
  `Amount$`, `Optional$` (bis zu), `StrictAmount$`, `Destroy$` (zerstören statt opfern),
  `RememberSacrificed$`. Alle Spieler wählen zuerst, dann wird gleichzeitig geopfert
  (CR 101.4) – Java opfert spielerweise nacheinander.
- Gemerkte Objekte (`CardObj::remembered`, bisher nur Karten): `RememberSacrificed$`,
  `RememberObjects$` (Effect). Lesen über `Defined$ Remembered` (aktuelles Objekt,
  folgt `moved_to` wie Java `getCardState`; nur bei Karten-Effekten), `RememberedLKI`
  (Objekt wie gemerkt), `ConditionDefined$ Remembered[LKI]` + `ConditionPresent$`,
  SVar `Remembered$Amount`. `Cleanup | ClearRemembered$ True` leert die Liste.
- `DelayedTrigger`: alle Parameter außer den Effekt-Parametern (`RememberObjects$`,
  `ThisTurn$`, `NextTurn$`, `Condition…`, Kosten/Ziele, `SubAbility$`) bilden einen Trigger,
  der wie ein `T:`-Trigger kompiliert wird (`compile_trigger`). Er liegt in
  `GameState::delayed_triggers`, ist an keine Zone gebunden und löst einmal aus (CR 603.7c).
  `ThisTurn$` endet im Cleanup, `NextTurn$` gilt nur im nächsten Zug. Gemerkte Objekte
  (auch Spieler) liest die Execute-Fähigkeit über `DelayTriggerRemembered` (folgt
  `moved_to`) bzw. `DelayTriggerRememberedLKI`.
- Objekt-Ausdrücke `<Objekte>$<Eigenschaft>` (Java `calculateAmount`): Objekte
  `Targeted`/`ParentTargeted`, `Remembered`, `TriggeredCard` (LKI nach Verlassen des
  Battlefields), `TargetedPlayer`, `TriggeredPlayer`; Eigenschaften `Amount`, `CardPower`,
  `CardToughness`, `CardManaCost`, `CardsInHand`, `LifeTotal`; Summe über alle Objekte,
  danach `/Plus.N` usw. `EvalCtx` trägt dafür Ziele und Trigger-Kontext.
- Fähigkeiten: `ActivationZone$ Hand|Graveyard` (Standard Battlefield), Kostenteile
  `Discard<1/CARDNAME>`, `AddCounter<n/TYP>`, `SubCounter<n/TYP>` (Quelle, n fest).
  `Planeswalker$ True` = Loyalitätsfähigkeit (Hexerei, einmal pro Zug, CR 606.3);
  Planeswalker kommen mit `Loyalty:` ins Spiel (nur feste Werte).
- Keywords `Cycling:<Kosten>` und `TypeCycling:<Typ>:<Kosten>` werden wie in Java
  zu `AB$ Draw` bzw. `AB$ ChangeZone` aus der Hand übersetzt; `Devoid` macht farblos.
- Trigger `Blocks` (ValidCard, ValidBlocked), `AttackerBlocked` (ValidBlocker[Amount]),
  `AttackerUnblocked` (ValidDefender), `Drawn` (Number$ = n-te Karte im Zug),
  `LifeGained`, `Taps` (über `GameState::tap`), `DamageDoneOnce`; `Surveil` (Amount$).
- Ersatzeffekte `R:Event$ DamageDone`: `Prevent$ True` oder `ReplaceWith$` auf
  `DB$ ReplaceEffect | VarName$ DamageAmount | VarValue$ X` mit
  `X:ReplaceCount$DamageAmount[/Op]` (intern als `Count$xPaid` mit x = Schaden).
  Filter `ValidTarget$`, `ValidSource$`, `IsCombat$`. `PreventDamage | Amount$` legt
  Verhinderungsschilde bis Zugende an.
- `GainControl` (`LoseControl$ EOT|UntilTheEndOfYourNextTurn|LeavesPlay|LoseControl`,
  `Untap$`, `AddKWs$`, `NewController$`) über `GameState::control_effects`.
- `RepeatEach` (`RepeatPlayers$`, `RepeatCards$`+`Zone$`, `DefinedCards$`): der jeweilige
  Spieler/die Karte ist während der Unterfähigkeit „gemerkt“; gemerkte Spieler liegen in
  `CardObj::remembered_players`, Filter `RememberedPlayerCtrl`, `Defined$ Player.IsRemembered`.
- Trigger `ChangesZoneAll` (Bündel je Effekt/SBA/Kosten, `TriggeredCards`),
  `BecomesTarget` (`ValidSource$ Spell|Ability|SpellAbility[.YouCtrl|.OppCtrl]`),
  `Cycled`, `Discarded`, `AttackerBlockedByCreature` (`TriggeredBlocker`).
- Keywords `Ward:<Kosten>` und `Prowess` werden wie in Java zu Triggern übersetzt.
- `Fight` (Kämpfer wie Java `getFighters`), `PutCounterAll`; `Defined$ ParentTarget`
  meint immer die Ziele der Elternfähigkeit. Kostenteil `ExileFromGrave<1/CARDNAME>`.
- Trigger `Sacrificed`, `AttackersDeclared`, `AttackersDeclaredOneTarget`
  (`TriggeredAttackingPlayer`, `TriggeredAttackedTarget`).
- Kostenhilfen-Keywords `Convoke`, `Improvise`, `Delve` werden beim Wirken nach den
  Kostenänderungen und vor dem Mana abgefragt (`Decision::ChooseCards` mit
  `CardChoiceReason::Convoke|Improvise|Delve`, Java `CostAdjustment`); `Affinity:<Filter>`
  senkt den generischen Anteil in `adjusted_spell_cost`.
- Filter `counters_<LT|LE|EQ|GE|GT|NE><n>_<Art>` (nur feste Zahlen), `HasCounters`,
  `IsMonstrous`/`nonMonstrous`.
- `PutCounter`-Kurzformen `Adapt$ n`, `Monstrosity$ n`, `Bolster$ n`, `Support$ n` werden
  beim Laden wie in Java (`CountersPutEffect.buildSpellAbility`) zu `CounterType$ P1P1`
  usw. umgeschrieben; die Sonderregeln trägt `effects::CounterMode`.
- `RemoveCounter`: `CounterType$ <Art>|Any|All`, `CounterNum$ n|All`, `UpTo$`, `Optional$`.
- Spieler-Statics (`Mode$ Continuous | Affected$ You|Player|Opponent`) werden zu
  `CompiledStatic::Player`: `SetMaxHandSize$ n|Unlimited`, `RaiseMaxHandSize$`,
  `AddKeyword$ Hexproof|Shroud`.

- `Play`: `Defined$`/Ziel oder `Valid$`+`ValidZone$` (Standard Hand), `ValidSA$`
  (`Spell` = jeder Zauberspruch), `Amount$ n|All`, `Optional$`, `WithoutManaCost$`
  (sonst normal bezahlt; Abbruch stellt den Zustand wieder her), `Controller$`,
  `ReplaceGraveyard$ Exile` (`CardObj::exile_from_stack`), `RememberPlayed$`.
- `CopySpellAbility`: Ziel-Zauber (`TargetType$ Spell`) oder `Defined$ TriggeredSpellAbility`
  (SpellCast-Trigger tragen ihr Stapelobjekt), `Amount$`, `Controller$`, `MayChooseTarget$`.
  Zauberkopien sind Token-Objekte in der Stapelzone.
- `DigUntil`: `Valid$`, `Amount$`, `MaxRevealed$`, `FoundDestination$`/`RevealedDestination$`
  mit `…LibraryPosition$ 0|-1`, `RevealRandomOrder$`, `Shuffle$`, `Remember…`, `Tapped$`, `GainControl$`.
- Statics `CantBeCast` (`Caster$`, `Origin$`, `NumLimitEachTurn$`, `OnlySorcerySpeed$`) und
  `CantBeActivated` (`Activator$`, `ValidSA$ Activated[.!ManaAbility|.Loyalty]`, `AffectedZone$`).
- `R:Event$ Counter | Layer$ CantHappen` → `CompiledFace::cant_counter`.
- Auren: `K:Enchant:<Filter>:<Beschreibung>`, `Defined$ Enchanted|Equipped|EnchantedController`,
  `AffectedDefined$ AttachedBy Self`, Static `GainControl$ You` (Layer 2, gegen
  `GainControl`-Effekte per Zeitstempel), Filter `Player.EnchantedController`,
  `enchanted`/`equipped`.
- Wichtig: Ohne Angabe meint nur `Defined$` die Ziele der Fähigkeit; andere Spieler-/
  Kartenschlüssel (`Controller$`, `TokenOwner$`, `Object$` …) haben ihren eigenen Standard.

## Token und modale Fähigkeiten

- `Token`: `TokenScript$` verweist auf Dateien in `res/tokenscripts` (Name ohne `.txt`,
  auch als Liste). `CardDb::load` lädt diese Skripte; Karten mit fehlenden oder nicht
  unterstützten Token gelten als nicht unterstützt (`check_token_refs`). Im Spiel stehen
  sie über `GameState::token_defs` bereit. Token lösen beim Erzeugen ETB-Trigger aus.
- `Charm`: `Choices$` nennt SVars mit den Modi. Beim Wirken (bzw. bei Triggern beim
  Stapeln) fragt `Decision::ChooseModes` nach den Modi; nur Modi mit legalen Zielen sind
  wählbar (CR 700.2a). Die gewählten Modi werden in **Druckreihenfolge** zu einer
  Fähigkeitskette zusammengesetzt (CR 700.2), danach wie jede Kette gestapelt.

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
