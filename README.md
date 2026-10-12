# Kerygma

En skrivebordsapp i Rust (Tauri 2) for å skrive manus og vise presentasjoner av dem. Den åpner en mappe med markdown, lager presentasjoner av manusene, viser dem med reveal.js (også som talervisning på to skjermer), eksporterer til PDF, PowerPoint og HTML, setter inn bibelvers og importerer Word-manus. Filene er vanlig markdown, så samme mappe kan også åpnes i Obsidian.

Arkitekturen står i [`docs/arkitektur.md`](docs/arkitektur.md), Word-importen i [`docs/word-import.md`](docs/word-import.md), og brukerveiledningen i [`docs/Brukerveiledning.md`](docs/Brukerveiledning.md). Planer for det som ikke er bygget, ligger i [`plans/`](plans/).

## Oppbygging

```
crates/kerygma-core   all logikk, uten Tauri (presentasjoner, referanser, bibel, markdown, vault, Word-import)
crates/kerygma-cli    `kerygma` i terminalen
src-tauri           appen: vinduet, menyen og kommandoene grensesnittet kaller
ui                  grensesnittet: TypeScript, Vite, CodeMirror 6 og reveal.js (typene i ui/src/bindings.ts lages fra Rust)
crates/bibel-core    oppslag, søk og formatering av bibelvers (fra bible-repoet, bare det appen trenger)
crates/bibel-format  binærformatet til oversettelsene
bibler              oversettelsene: nb30.bin og kjv.bin i git, andre blir med ved bygging (se bibler/README.md)
scripts             fontene i slides.css
```

## Komme i gang

Krever Rust og Node 22. Alt annet ligger i repoet, også oversettelsene: Bibelen 1930 (`nb30`) og King James Version (`kjv`). Andre oversettelser kan legges til etter at appen er installert, under **Innstillinger → Bibeloversettelser** (se [`bibler/README.md`](bibler/README.md)).

```sh
(cd ui && npm install)

cargo test --workspace                  # alle testene i Rust
(cd ui && npm test)                     # testene av grensesnittet
(cd ui && npm run tauri dev)            # appen, med Vite og hot reload
(cd ui && npm run tauri build)          # .app og .dmg (macOS) / .msi (Windows)

cargo run -p kerygma-cli -- lag-presentasjon manus.md
cargo run -p kerygma-cli -- vers ne:joh3:16
cargo run -p kerygma-cli -- importer manus.docx --ut mappe/
```

## Opphavsrett

- **Bibelen 1930** (`bibler/nb30.bin`) er i det fri.
- **King James Version** (`bibler/kjv.bin`) er i det fri, unntatt i Storbritannia. Dataene er hentet fra thiagobodruk/bible (MIT, lisensen følger med appen).
- **Andre oversettelser** er ikke i repoet. Legges de i `bibler/`, stopper `.gitignore` dem, og en app bygget med dem kan ikke distribueres uten avtale med rettighetshaveren.
- **Fontene** i presentasjonene (Lora og Montserrat) er under SIL Open Font License 1.1 (`ui/public/lisenser/`).
- **Testene og dokumentasjonen** siterer bare Bibelen 1930 og KJV.
