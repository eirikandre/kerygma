# CLAUDE.md

Kerygma: en skrivebordsapp i Rust (Tauri 2) for å skrive manus og vise presentasjoner av dem. Se `docs/arkitektur.md` for hvordan den er bygget og `README.md` for oppbyggingen.

## Kommandoer

```sh
cargo test --workspace                   # alle testene
cargo clippy --workspace --all-targets -- -D warnings
(cd ui && npx tsc --noEmit)              # typesjekk av grensesnittet
(cd ui && npm test)                      # testene av grensesnittet (Vitest, ui/test/)
KERYGMA_OPPDATER_BINDINGS=1 cargo test -p kerygma-app   # lag ui/src/bindings.ts på nytt
(cd ui && npm run tauri dev)             # kjør appen
(cd ui && npm run tauri build)           # pakk appen
(cd ui && npm run fonts)                 # bygg fontene i ui/fonts/ inn i ui/public/slides.css
```

`npm run tauri` kjører Tauri-CLI-en fra roten av repoet, så den finner `src-tauri/`.

## Regler

- **Testene i `crates/kerygma-core/tests/` er fasiten** for hvordan manus blir presentasjoner, vers og Word-import. De sammenligner hele utdata tegn for tegn. Endres oppførselen med vilje, endres testene i samme commit.
- Appen har ingen avhengighet til andre repoer. Ikke bruk eller vis til `../obsidian-slide-creator`. Bible-repoet (`../bible`) har skriptene som henter tekstene, `bibel-gen` som lager `.bin`, og kommandoen `bibel`; her er bare `crates/bibel-format` og `crates/bibel-core`, det appen trenger for å bruke tekstene. Kerygma bruker dem gjennom `bibel_core::run` i `crates/kerygma-core/src/bible.rs`.
- **Repoet er offentlig.** Bare oversettelser som er i det fri, er i git: `bibler/nb30.bin` (Bibelen 1930, standard) og `bibler/kjv.bin`. Norsk tekst i kode, tester og dokumentasjon skal være fra Bibelen 1930 (slå opp med `bibel`/`Bibel::run`), og navn på andre oversettelser skal ikke forekomme.
- Alle `bibler/*.bin` som finnes ved bygging, blir med: i appen (`tauri.conf.json`), i boknavnene manus gjenkjennes med (`crates/kerygma-core/build.rs`), i Word-importen (`word::import_translations`) og i oppslag på tvers av oversettelser (`bibel_core::reference::resolve_with`). Ikke skriv navnene til en bestemt oversettelse inn i koden; vanlige norske boknavn står i `book_names::NORWEGIAN_NAMES`.
- Testene laster bare nb30 og KJV (`common::bibel()`), så de gir samme svar uansett hva annet som ligger i `bibler/`. Oversettelser som ikke er installert, hoppes over (`Bibel::configured`).
- `kerygma-core` skal ikke vite om Tauri. Logikk hører hjemme der og testes med `cargo test`; `src-tauri/src/commands.rs` er bare tynne kall. Flyter som bruker vaulten og innstillingene (lag presentasjon, talervisning, Word-import), ligger i `kerygma_core::workspace::Workspace`.
- Typene og kommandoene grensesnittet bruker, lages fra Rust med tauri-specta i `ui/src/bindings.ts`. Ikke rediger den for hånd: endre Rust og lag den på nytt (se over). `cargo test` feiler hvis den er utdatert. Ny kommando: `#[specta::specta]` på den, og legg den til i `specta_builder` i `src-tauri/src/lib.rs`. Typer fra `kerygma-core` får `#[cfg_attr(feature = "specta", derive(specta::Type))]`.
- Ingen kommando skal holde en lås mens den jobber: hent `state.workspace()` og `state.bible()` (kopier) først.
- Filer skrives med `vault::write_atomic` (via `Vault::write_note`), så de aldri blir halvveis skrevet.
- CSP er på (`src-tauri/tauri.conf.json`). Nye kilder (skript, bilder, fonter, `fetch`) må tillates der.
- Lengder i delingen av slides (`maxChars`, deling av vers) telles i UTF-16-enheter (`extract::len16`), slik slidene alltid har blitt delt. Indekser i strenger er byte-indekser.
- Sifre i regex skrives `[0-9]` (`\d` i Rust godtar også andre sifre). Lookaround og tilbakereferanser bruker `fancy-regex`; ordgrenser skrives som ASCII-lookaround.
- Alle stier mellom grensesnitt og Rust er relative til vault-roten og bruker `/`.
- Kode, kommentarer og meldinger er på norsk.
