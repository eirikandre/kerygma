# CLAUDE.md

Kerygma: en skrivebordsapp i Rust (Tauri 2) som erstatter Obsidian + pluginen obsidian-slide-creator. Se `plans/kerygma-app-rust.md` for planen og `README.md` for oppbyggingen.

## Kommandoer

```sh
cargo test --workspace                   # alle testene (bibeltestene hoppes over uten bibler/*.bin)
cargo clippy --workspace --all-targets
(cd ui && npx tsc --noEmit)              # typesjekk av grensesnittet
(cd ui && npm run tauri dev)             # kjør appen
(cd ui && npm run tauri build)           # pakk appen
node scripts/book-names.mjs              # lag crates/kerygma-core/src/book_names.rs på nytt
```

`npm run tauri` kjører Tauri-CLI-en fra roten av repoet, så den finner `src-tauri/`.

## Regler

- **`../obsidian-slide-creator` er bare en kilde til informasjon.** Ingenting der skal endres: ikke skriv, flytt, slett, bygg eller commit noe, og ikke kjør `npm install` eller andre kommandoer som lager filer der. Les filene direkte.
- **Testene i `../obsidian-slide-creator` er fasiten.** `test/extract.test.ts`, `test/verseQuery.test.ts`, `test/bible.test.ts` og `word-to-markdown/test/convert.test.ts` er portet til `crates/kerygma-core/tests/` med samme navn og samme strenger. Utdataene skal være like tegn for tegn.
- `kerygma-core` skal ikke vite om Tauri. Logikk hører hjemme der og testes med `cargo test`; `src-tauri/src/commands.rs` er bare tynne kall.
- Lengder som skal være som i pluginen (`maxChars`, deling av vers), telles i UTF-16-enheter som `length` i JavaScript. Indekser i strenger er byte-indekser.
- `\d` i JavaScript er bare ASCII-sifre: skriv `[0-9]` i Rust. Lookaround og tilbakereferanser bruker `fancy-regex`; `\b` fra JavaScript skrives som ASCII-lookaround.
- Alle stier mellom grensesnitt og Rust er relative til vault-roten og bruker `/`.
- Kode, kommentarer og meldinger er på norsk.
