# CLAUDE.md

Kerygma: en skrivebordsapp i Rust (Tauri 2) for å skrive manus og vise presentasjoner av dem. Se `docs/arkitektur.md` for hvordan den er bygget og `README.md` for oppbyggingen.

## Kommandoer

```sh
cargo test --workspace                   # alle testene (bibeltestene hoppes over uten bibler/*.bin)
cargo clippy --workspace --all-targets -- -D warnings
(cd ui && npx tsc --noEmit)              # typesjekk av grensesnittet
(cd ui && npm test)                      # testene av grensesnittet (Vitest, ui/test/)
KERYGMA_OPPDATER_BINDINGS=1 cargo test -p kerygma-app   # lag ui/src/bindings.ts på nytt
(cd ui && npm run tauri dev)             # kjør appen
(cd ui && npm run tauri build)           # pakk appen
node scripts/book-names.mjs              # lag book_names.rs fra scripts/book-names.json
node scripts/book-names.mjs --fra-bible  # hent boknavn og testreferanser på nytt fra ../bible
(cd ui && npm run fonts)                 # bygg fontene i ui/fonts/ inn i ui/public/slides.css
```

`npm run tauri` kjører Tauri-CLI-en fra roten av repoet, så den finner `src-tauri/`.

## Regler

- **Testene i `crates/kerygma-core/tests/` er fasiten** for hvordan manus blir presentasjoner, vers og Word-import. De sammenligner hele utdata tegn for tegn. Endres oppførselen med vilje, endres testene i samme commit.
- Appen har ingen avhengighet til andre repoer enn `bible` (`bibel-core` over git, `*.bin` i `bibler/`, og `../bible` bare for `book-names.mjs --fra-bible`). Ikke bruk eller vis til `../obsidian-slide-creator`.
- `kerygma-core` skal ikke vite om Tauri. Logikk hører hjemme der og testes med `cargo test`; `src-tauri/src/commands.rs` er bare tynne kall. Flyter som bruker vaulten og innstillingene (lag presentasjon, talervisning, Word-import), ligger i `kerygma_core::workspace::Workspace`.
- Typene og kommandoene grensesnittet bruker, lages fra Rust med tauri-specta i `ui/src/bindings.ts`. Ikke rediger den for hånd: endre Rust og lag den på nytt (se over). `cargo test` feiler hvis den er utdatert. Ny kommando: `#[specta::specta]` på den, og legg den til i `specta_builder` i `src-tauri/src/lib.rs`. Typer fra `kerygma-core` får `#[cfg_attr(feature = "specta", derive(specta::Type))]`.
- Ingen kommando skal holde en lås mens den jobber: hent `state.workspace()` og `state.bible()` (kopier) først.
- Filer skrives med `vault::write_atomic` (via `Vault::write_note`), så de aldri blir halvveis skrevet.
- CSP er på (`src-tauri/tauri.conf.json`). Nye kilder (skript, bilder, fonter, `fetch`) må tillates der.
- Lengder i delingen av slides (`maxChars`, deling av vers) telles i UTF-16-enheter (`extract::len16`), slik slidene alltid har blitt delt. Indekser i strenger er byte-indekser.
- Sifre i regex skrives `[0-9]` (`\d` i Rust godtar også andre sifre). Lookaround og tilbakereferanser bruker `fancy-regex`; ordgrenser skrives som ASCII-lookaround.
- Alle stier mellom grensesnitt og Rust er relative til vault-roten og bruker `/`.
- Kode, kommentarer og meldinger er på norsk.
