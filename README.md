# Kerygma

En skrivebordsapp i Rust (Tauri 2) for å skrive manus og vise presentasjoner av dem. Den åpner en mappe med markdown, lager presentasjoner av manusene, viser dem med reveal.js (også som talervisning på to skjermer), eksporterer til PDF, PowerPoint og HTML, setter inn bibelvers og importerer Word-manus. Filene er vanlig markdown, så samme mappe kan også åpnes i Obsidian.

Arkitekturen står i [`docs/arkitektur.md`](docs/arkitektur.md), Word-importen i [`docs/word-import.md`](docs/word-import.md), og brukerveiledningen i [`docs/Brukerveiledning.md`](docs/Brukerveiledning.md). Planer for det som ikke er bygget, ligger i [`plans/`](plans/).

## Oppbygging

```
crates/kerygma-core   all logikk, uten Tauri (presentasjoner, referanser, bibel, markdown, vault, Word-import)
crates/kerygma-cli    `kerygma` i terminalen
src-tauri           appen: vinduet, menyen og kommandoene grensesnittet kaller
ui                  grensesnittet: TypeScript, Vite, CodeMirror 6 og reveal.js (typene i ui/src/bindings.ts lages fra Rust)
bibler              *.bin fra bible-repoet (ikke i git)
scripts             boknavn fra bible-repoet, og fontene i slides.css
```

## Komme i gang

Krever Rust, Node 22, og at [`bible`](https://github.com/eirikandre/bible) er tilgjengelig over ssh (`bibel-core` hentes derfra).

```sh
cp ../bible/assets/*.bin bibler/        # oversettelsene
(cd ui && npm install)

cargo test --workspace                  # alle testene i Rust
(cd ui && npm test)                     # testene av grensesnittet
(cd ui && npm run tauri dev)            # appen, med Vite og hot reload
(cd ui && npm run tauri build)          # .app og .dmg (macOS) / .msi (Windows)

cargo run -p kerygma-cli -- lag-presentasjon manus.md
cargo run -p kerygma-cli -- vers ne:joh3:16
cargo run -p kerygma-cli -- importer manus.docx --ut mappe/
```
