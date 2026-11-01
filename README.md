# Forge-Rules-Engine in Rust

Neubau der Forge-Regelengine (`forge-game`) in Rust. Die Engine interpretiert die
unveränderten Forge-Kartenskripte aus `forge-gui/res/cardsfolder`.

Ziel-Formate: Standard, Pioneer, Modern, Legacy, Vintage, Pauper, Commander,
Booster Draft, Cube. Sonderformate sind ausgeschlossen (Details in `../CLAUDE.md`).

## Schnellstart

```sh
cd rust
cargo test                       # alle Tests (inkl. Parse aller 34k Karten)
cargo clippy --all-targets       # muss warnungsfrei sein
cargo fmt

# Lasttest mit 1000 Zufallspartien aus echten Karten
SOAK_GAMES=1000 cargo test --release -p forge-engine --test soak

# Berichte neu erzeugen
cargo run -q -p forge-script --bin corpus-report > docs/generated/kartenbestand.md
cargo run -q --release -p forge-engine --bin support-report > docs/generated/unterstuetzung.md

# Regelstatus → xlsx-Tracker + docs/03-regelstand.md (aus dem Repo-Root)
python3 rust/tools/sync_regelstatus.py
```

Der Pfad zu den Forge-Ressourcen wird relativ ermittelt
(`rust/../forge-gui/res`). Mit `FORGE_RES=/pfad/zu/res` lässt er sich überschreiben.

## Dokumentation

| Datei | Inhalt |
|---|---|
| [docs/01-architektur.md](docs/01-architektur.md) | Aufbau, Prinzipien, Module, Tests |
| [docs/02-kartenskripte.md](docs/02-kartenskripte.md) | Skriptformat, Parser, Effekt-Compiler, Checkliste für neue Effekte |
| [docs/03-regelstand.md](docs/03-regelstand.md) | Status jeder Regel (generiert) |
| [docs/04-arbeitsprotokoll.md](docs/04-arbeitsprotokoll.md) | Verlauf, Entscheidungen, bekannte Lücken, nächste Schritte |
| [docs/generated/kartenbestand.md](docs/generated/kartenbestand.md) | Nutzungshäufigkeit aller Bausteine (generiert) |
| [docs/generated/unterstuetzung.md](docs/generated/unterstuetzung.md) | Anteil spielbarer Karten je Format, fehlende Bausteine (generiert) |

## Beispiel

```rust
use forge_engine::{CardDb, Engine, GameRules, GameState, Status};

let db = CardDb::load(&forge_script::corpus::default_res_dir())?;
let deck = vec![db.get("Lightning Bolt").unwrap().clone(); 60];
let state = GameState::new(GameRules::default(), vec![("A".into(), deck.clone()), ("B".into(), deck)], 42, None);
let mut engine = Engine::new(state);
let mut status = engine.advance();
while let Status::NeedsDecision(decision) = status {
    let choice = /* GUI / KI / Test */;
    status = engine.submit(choice)?;
}
```
