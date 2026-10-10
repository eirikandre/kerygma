# Kerygma

En skrivebordsapp i Rust (Tauri 2) for å skrive manus og vise presentasjoner av dem. Den gjør det Obsidian og pluginen [obsidian-slide-creator](../obsidian-slide-creator) gjør: åpner en mappe med markdown, lager presentasjoner med de samme reglene, viser dem med reveal.js og `slides.css`, setter inn bibelvers og importerer Word-manus. Filene er vanlig markdown, så samme mappe kan åpnes i Obsidian.

Planen ligger i [`plans/kerygma-app-rust.md`](plans/kerygma-app-rust.md), og brukerveiledningen i [`docs/Brukerveiledning.md`](docs/Brukerveiledning.md).

## Oppbygging

```
crates/kerygma-core   all logikk, uten Tauri (presentasjoner, referanser, bibel, markdown, vault, Word-import)
crates/kerygma-cli    `kerygma` i terminalen
src-tauri           appen: vinduet, menyen og kommandoene grensesnittet kaller
ui                  grensesnittet: TypeScript, Vite, CodeMirror 6 og reveal.js
bibler              *.bin fra bible-repoet (ikke i git)
```

## Komme i gang

Krever Rust, Node 22, og at [`bible`](https://github.com/eirikandre/bible) er tilgjengelig over ssh (`bibel-core` hentes derfra).

```sh
cp ../bible/assets/*.bin bibler/        # oversettelsene
(cd ui && npm install)

cargo test --workspace                  # alle testene
(cd ui && npm run tauri dev)            # appen, med Vite og hot reload
(cd ui && npm run tauri build)          # .app og .dmg (macOS) / .msi (Windows)

cargo run -p kerygma-cli -- lag-presentasjon manus.md
cargo run -p kerygma-cli -- vers ne:joh3:16
cargo run -p kerygma-cli -- importer manus.docx --ut mappe/
```
