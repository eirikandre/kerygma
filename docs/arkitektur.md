# Arkitektur

Et overblikk over hvordan Kerygma er bygget, hvordan delene snakker sammen, og hva som bør vurderes for at appen skal være lett å vedlikeholde over tid. Skrevet 2026-10-11 etter commit `79afab5`, og oppdatert samme dag etter at forslagene under *Vurdering* ble gjennomført (se *Status*).

Appen ble opprinnelig portet fra en Obsidian-plugin (obsidian-slide-creator), men har ingen avhengighet til den lenger: alt den trenger, ligger i dette repoet.

## Kort fortalt

Kerygma er en Tauri 2-app. All logikk som kan testes uten vindu, ligger i Rust-craten `kerygma-core`. Tauri-laget (`src-tauri`) er et tynt skall med vindu, meny, tilstand og kommandoer. Grensesnittet (`ui`) er vanlig TypeScript uten rammeverk, med CodeMirror 6 som editor og reveal.js til presentasjonene. Filene i vaulten er vanlig markdown, så samme mappe kan åpnes i Obsidian.

```
                        ┌──────────────────────────────────────────────┐
                        │ ui/ (TypeScript, Vite)                        │
                        │                                              │
  index.html ──────────▶│ main.ts  ── tilstand, kommandoer, hurtigtaster│
                        │   ├ tree.ts, search.ts, editor.ts, format.ts  │
                        │   ├ verseModal.ts, importReport.ts, settings.ts│
                        │   ├ present.ts (reveal.js) ── slideThemes.ts  │
                        │   ├ presenter.ts ◀── hendelser ──▶ audience.ts│◀── audience.html
                        │   └ export.ts (PDF, PowerPoint, HTML)         │    (vindu 2)
                        │ api.ts ── over bindings.ts (laget fra Rust)  │
                        └──────────────┬───────────────────▲───────────┘
                     commands.*(...)  │                   │ events.*: menu, vaultChanged,
                                       ▼                   │ bibleReady, bibleWarning
                        ┌──────────────────────────────────┴───────────┐
                        │ src-tauri/ (kerygma-app)                      │
                        │ lib.rs      meny, oppstart, biblene, specta    │
                        │ commands.rs 31 tynne kommandoer                │
                        │ events.rs   hendelsene til grensesnittet       │
                        │ state.rs    AppState (vault, biblene, PDF)     │
                        │ capture.rs  PDF av webvisningen (bare macOS)   │
                        └──────────────┬───────────────────────────────┘
                                       ▼
  kerygma-cli ────────▶ ┌──────────────────────────────────────────────┐
  (terminalen)          │ crates/kerygma-core (ingen Tauri)            │
                        │ workspace (flytene), extract, presentation,  │
                        │ presenter, markdown, reference, book_names,  │
                        │ bible, verse_query, vault, search, settings, │
                        │ app_config, json, pdf, word/*                │
                        └──────────────┬───────────────────────────────┘
                                       ▼
                        bibel-core (git, privat repo) + bibler/*.bin
```

## Repoet

| Mappe | Hva | Størrelse (linjer) |
|---|---|---|
| `crates/kerygma-core` | All logikk: presentasjoner, bibelreferanser, bibeloppslag, markdown, vault, søk, Word-import, PDF-sammenslåing | ca. 4 900 kode + 1 000 test |
| `crates/kerygma-cli` | `kerygma lag-presentasjon`, `kerygma vers`, `kerygma importer` | 200 |
| `src-tauri` | Appen: vindu, meny, kommandoer, tilstand | 870 |
| `ui/src` | Grensesnittet | ca. 4 700 TS + 950 CSS |
| `ui/public/slides.css` | Utseendet på presentasjonene (temaet «Kveld»), med fontene bygget inn | 150 |
| `ui/fonts/`, `ui/public/lisenser/` | Fontene (woff2) som bygges inn i `slides.css`, og lisensene (OFL), som følger med appen | |
| `bibler/` | `kjv.bin`, `nb88.bin`, `nb24.bin` (ikke i git) | |
| `scripts/book-names.mjs` | Lager `book_names.rs` fra `scripts/book-names.json`, og henter boknavn og testreferanser på nytt fra `../bible` (`--fra-bible`) | |
| `scripts/embed-fonts.mjs` | Bygger fontene i `ui/fonts/` inn i `slides.css` (`npm run fonts` i `ui/`) | |
| `.github/workflows/ci.yml` | Tester og sjekker på macOS og Windows | |
| `plans/` | Planer for funksjoner som ikke er bygget (`oversette-sitater.md`) | |
| `docs/` | Brukerveiledningen, dette dokumentet og `word-import.md` | |

## Lagene

### `kerygma-core`

Craten har ingen avhengighet til Tauri, og alt i den kan testes med `cargo test` og brukes fra CLI-en. Modulene, gruppert etter hva de gjør:

**Presentasjoner**

| Modul | Ansvar |
|---|---|
| `extract.rs` | Lager innholdet i en presentasjon fra et manus: beholder overskrifter, vers, sitater og bilder, deler lange vers etter `maxChars` (i UTF-16-enheter), lager kolonner for parallelle oversettelser. Hver slide vet også hvor i manuset den kom fra (`Slide::source`). |
| `presentation.rs` | Hvor presentasjonen havner (`<navn><suffix>.md`), markøren `<!-- kerygma-presentasjon: … -->`, og `slide_options` (oversettelser til vers som står alene). |
| `presenter.rs` | Talervisningen: setter usynlige tegn (privat bruk, `U+F0000 + i` og `U+100000 + i`) der hver slide starter og slutter i manuset, før det gjøres om til HTML. |
| `markdown.rs` | Markdown → HTML med comrak, pluss det Obsidian gjør (frontmatter, `%% %%`, callouts, `==mark==`, `![[bilde]]`, `[[lenke]]`). `slides()` deler AST-et på `---`, som Slides i Obsidian. |

**Bibel**

| Modul | Ansvar |
|---|---|
| `reference.rs` + `book_names.rs` | Gjenkjenner referanser som «Joh 3,16» og «1. Kor 13,4-7», og gjør dem om til oppslag. `book_names.rs` er generert. |
| `bible.rs` | `Bibel`: laster `*.bin` (lekket til `'static`, som `bibel-core` krever), konfigurerer standardoversettelse og snarveier, og kjører `bibel_core::run` med de samme argumentene som kjørbaren `bibel`. |
| `verse_query.rs` | Tolker det som skrives i «Sett inn bibelvers», gir forslag og lager sitatet. |

**Vault og flyter**

| Modul | Ansvar |
|---|---|
| `workspace.rs` | `Workspace` = `Vault` + `Settings`. Flytene appen og CLI-en deler: lagre/lese innstillinger, «Lag presentasjon» (`CreateResult`), talervisningen (`Presenter`), presentasjonen som eksporteres, hvor Word-filer havner og selve importen. Testes med midlertidige mapper. |
| `app_config.rs` | `app.json`: sist åpnede vault og utseendet (`Appearance`, `Theme`). |
| `json.rs` | `over_default`: leser JSON oppå standardverdiene, felt for felt, så gamle og ødelagte filer tåles. Brukes av `Settings` og `AppConfig`. |
| `vault.rs` | `Vault`: relative stier med `/`, avviser stier ut av vaulten, treet (skjuler presentasjoner med manuset ved siden av), opprett/gi nytt navn/flytt/slett (papirkurv), oppslag av bilder og lenker som i Obsidian, asset-URL-er. `write_atomic` skriver via en skjult midlertidig fil. |
| `search.rs` | Søk i navn og innhold i alle manus, i NFC. Svarer med ordene det brukte og treffene (`SearchResponse`). Leser alle filene ved hvert søk. |
| `settings.rs` | Innstillinger per vault, med standardverdier og opprydding (`sanitized`). Et nytt felt legges bare til i structen og i `Default`. |

**Import og eksport**

| Modul | Ansvar |
|---|---|
| `word/docx.rs` | Leser avsnitt, stil, fet og kursiv fra `.docx` (zip + quick-xml). |
| `word/convert.rs` | Finner bibelsitater i et Word-manus (referanser, «V1», kapittel fra overskrifter, søk), og lager markdown og en rapport. 1 000 linjer; den mest kompliserte modulen. |
| `word/align.rs` | Smith-Waterman på ord, for å finne sitatet i teksten. |
| `word/bible_text.rs` | Traitet `BibleText`, så `convert` kan testes med en falsk bibel. |
| `pdf.rs` | Slår sammen PDF-sider (én per slide) og skalerer dem til full HD, med lopdf. |

Det meste er rene funksjoner fra tekst til tekst. Unntakene er `vault.rs`, `workspace.rs`, `app_config.rs` og `search.rs` (filsystemet) og `bible.rs` (data i minnet). Feil er `Result<T, String>` med norske meldinger hele veien.

Med featuren `specta` får typene grensesnittet bruker (`Tree`, `Settings`, `CreateResult`, `Presenter`, `SearchResponse`, `Appearance` …) `specta::Type`, så `src-tauri` kan lage TypeScript av dem. Uten featuren (CLI-en, `cargo test -p kerygma-core`) er specta ikke med.

### `src-tauri`

- `lib.rs` bygger menyen, laster biblene i en egen tråd, og setter opp `specta_builder`: kommandoene, hendelsene og konstantene som blir til `ui/src/bindings.ts`. Fila lages når appen startes under utvikling, og testen `bindings_ts_er_oppdatert` feiler hvis den er utdatert (`KERYGMA_OPPDATER_BINDINGS=1 cargo test -p kerygma-app` lager den på nytt). `save_export` tar imot rå bytes og er registrert for seg, utenfor specta.
- `events.rs` har hendelsene til grensesnittet som typer: `Menu`, `VaultChanged`, `BibleReady`, `BibleWarning`.
- `state.rs` holder `AppState`:
  ```rust
  vault:       Mutex<Option<OpenVault>>  // Workspace + notify-watcher
  bible:       Mutex<BibleState>         // Loading | Ready(Arc<Bibel>) | Failed(String)
  pdf_exports: Mutex<PdfExports>         // sidene per PDF-eksport (id → sider)
  ```
  `state.workspace()` og `state.bible()` gir kopier (en `Workspace` og en `Arc`), så låsene bare holdes et øyeblikk. Nye bibelinnstillinger gir en ny `Bibel` som byttes inn (`Bibel::configured`); oppslag som pågår, bruker den gamle.
- `commands.rs` har 31 kommandoer, og alle er tynne: hent `Workspace` og biblene, kall én metode i `kerygma-core`.
- `capture.rs` lager en PDF av et utsnitt av webvisningen med `WKWebView.createPDF` via objc2. Bare macOS.
- Filovervåking: `notify` på hele vaulten, samlet i 250 ms, skjulte stier (også midlertidige filer fra `write_atomic`) filtrert bort, sendt som `VaultChanged` med relative stier.
- CSP er på (`tauri.conf.json`): skript bare fra appen, bilder fra appen, vaulten (asset), `data:` og nettet, `fetch` til appen og vaulten. Tauri legger ikke nonce på `style-src` (`dangerousDisableAssetCspModification`), siden CodeMirror og temaene setter stiler i siden.

### `ui`

To sider bygges av Vite: `index.html` (hovedvinduet) og `audience.html` (slidene på den andre skjermen i talervisningen).

| Fil | Ansvar |
|---|---|
| `main.ts` (640) | Kobler modulene sammen: vault, åpne/lukke filer, visning, presentasjonsknappene og -flytene, knappene. |
| `document.ts` | `OpenDocument`: fila som er åpen, lagret innhold, `dirty`, autolagring og endringer utenfra (`VaultChanged`). |
| `commands.ts` | Kommandoene: hurtigtastene (`SHORTCUTS`, `shortcutFor`), menyen og vern mot doble kall. |
| `bindings.ts` | Laget av Rust: `commands`, `events`, typene og konstantene (markøren, merkene i talervisningen, utseendet). |
| `api.ts` | Tynn modul over `bindings.ts`, og hjelpefunksjoner for stier og markøren. |
| `editor.ts`, `format.ts` | CodeMirror, og formatering (fet, kursiv, overskrift, sitat). |
| `tree.ts`, `search.ts` | Treet (høyreklikk, dra og slipp, omdøping) og søket. |
| `present.ts`, `slideThemes.ts`, `dither.ts` | Viser slidene med reveal.js og `slides.css`, temaer og skrifter lagt oppå som CSS-variabler, og gradering med dithering (WebKit har synlige fargetrinn). |
| `presenter.ts`, `audience.ts`, `presenterEvents.ts` | Talervisningen: manuset med merket tekst i hovedvinduet, slidene i et eget `WebviewWindow`. Vinduene snakker med Tauri-hendelser (`presenter:ready/init/slide/key`). |
| `export.ts` (500) | PDF (viser hver slide og ber Rust lage en PDF-side), PowerPoint (tolker HTML-en til blokker og bygger med pptxgenjs) og HTML (én fil med alt bygget inn). |
| `verseModal.ts`, `importReport.ts`, `settings.ts`, `dialogs.ts` | Vinduene og meldingene. |
| `sanitize.ts` | Fjerner skript og hendelser fra HTML før `innerHTML`. Viktig, siden CSP er av og skript i vinduet kan kalle alle kommandoene. |
| `appearance.ts` | Skrift, størrelse og tema i editoren (lagret i `app.json`). |

## Flytene

### Åpne vault og redigere

```
main.ts                        commands.rs                 kerygma-core
openVault(path) ──open_vault──▶ Vault::open, les settings ──▶ vault.tree(suffix)
                                asset-scope, notify, app.json
                                configure_bible(settings)
openNote ──read_note──────────▶ vault.read_note
tastetrykk → 1 s → save ──write_note──▶ vault.write_note
Cmd+E ──render_note(path, content)──▶ markdown::render_html + image_resolver
                                                  │
notify ──▶ vault-changed ──▶ refreshTree, last fila på nytt hvis den ikke er endret i editoren
```

Lagring skjer 1 sekund etter siste tastetrykk, når en annen fil åpnes, når vinduet mister fokus og når det lukkes (`document.ts`). Endringer utenfra lastes inn hvis editoren ikke har ulagrede endringer; ellers vinner editoren, og brukeren får en melding. Fila skrives atomisk.

### Lag presentasjon

1. `main.ts` sender stien og innholdet i editoren (ulagrede endringer skal med).
2. `Workspace::create_presentation` finner stiene (`presentation_paths_for`: markøren i fila går foran navnet), sjekker om manuset finnes, om målet er en mappe, og om en fil uten markør ville blitt overskrevet (da svarer den `Exists`, og grensesnittet spør og kaller igjen med `overwrite`).
3. `presentation_content` → `slide_options` (slår opp oversettelser til vers som står alene) → `extract_slide_content` → setter inn markøren.
4. Fila skrives, og grensesnittet åpner den.

Presentasjonen er en vanlig markdown-fil ved siden av manuset. Den vises ikke i treet når manuset finnes, men åpnes fra manuset.

### Vis presentasjon og talervisning

- **Én skjerm:** `slides(path)` → `markdown::slides` (HTML per slide) → `present.ts` bygger samme DOM som Obsidian og starter reveal.js i fullskjerm.
- **To skjermer (eller Alt-klikk):** `presenter(path, content)` lager slidene *fra manuset* med `extract_slides` og merker hvor hver slide står (`presenter.rs`). Hovedvinduet viser manuset som HTML og finner merkene (de private tegnene) for å markere teksten. `audience.html` åpnes i et nytt vindu, ber om slidene (`ready` → `init`), og hovedvinduet styrer hvilken som vises. Taster i slidevinduet sendes tilbake.

Merk at én skjerm viser *presentasjonsfila* (som kan være redigert for hånd), mens talervisningen lager slidene *fra manuset* på nytt.

### Eksport

Alle tre eksportene tar HTML-en fra `slides()`:

- **PDF:** `pdf_begin` gir en id, `export.ts` viser hver slide, `pdf_page(id, …)` lager én PDF-side av utsnittet med WebKit, og `pdf_save(id, …)` slår dem sammen med `pdf::merge` og skriver fila. Går noe galt, glemmer `pdf_cancel(id)` sidene. Bare macOS.
- **PowerPoint:** `parseSlide` tolker HTML-en til overskrifter, avsnitt, kolonner og bilder, og pptxgenjs bygger fila. Sendes til `save_export` som rå bytes med stien i en header.
- **HTML:** én fil med reveal.js, `slides.css`, temaet og bildene bygget inn.

### Sett inn bibelvers

Hvert tastetrykk i vinduet kaller `lookup_verse` → `verse_query::suggest_verse` → `bibel_core::run`. Enter kaller `quote_verse`, som lager sitatet med teksten før og etter markøren.

### Word-import

`importReport.ts` lar brukeren velge filer, `Workspace::import_targets` sier hvor de havner og om de finnes, grensesnittet spør om overskriving, og `Workspace::import_word` leser hver fil (`docx` → `convert` med `BibelText`) og skriver manuset. Biblene er ikke låst imens. Til slutt vises en rapport.

## Tilstand og lagring

| Hva | Hvor | Eies av |
|---|---|---|
| Manus og presentasjoner | `<vault>/**/*.md` | brukeren (deles med Obsidian) |
| Innstillinger per vault | `<vault>/.kerygma/settings.json` | `settings.rs` (format), `workspace.rs` (sti, lesing og lagring) |
| Sist åpnede vault, skrift, tema | `<app_config_dir>/app.json` | `app_config.rs` |
| Visning, skjult tre, mapper som er åpne | `localStorage` i webvisningen | `main.ts`, `tree.ts`, `presenter.ts` |
| Åpen vault, biblene, PDF-sider | `AppState` i minnet | `src-tauri` |
| Åpen fil, ulagret innhold | `OpenDocument` i `document.ts` | grensesnittet |

## Kontrakter mellom lagene

Det meste lages nå fra Rust i `ui/src/bindings.ts`, og `tsc` gir feil når en kommando eller type endres. Dette er det som fortsatt må holdes likt for hånd:

| Kontrakt | Rust | TypeScript | Sjekkes av |
|---|---|---|---|
| Typene og kommandoene | `commands.rs`, structene i `kerygma-core` | `bindings.ts` (laget) | `tsc`, `bindings_ts_er_oppdatert` |
| Hendelsene fra Rust | `events.rs` | `events` i `bindings.ts` (laget) | `tsc` |
| Markøren i presentasjoner | `presentation::MARKER_PATTERN` | `PRESENTATION_MARKER_PATTERN` (laget) | `test/api.test.ts` |
| Merkene i talervisningen | `presenter::START/END` | `PRESENTER_START/END` (laget) | testene i `presenter.rs` |
| Utseende (standardverdier, grenser, tema) | `app_config::Appearance` | `DEFAULT_APPEARANCE`, `APPEARANCE_*` (laget) | `tsc` |
| `save_export` (rå bytes) | `commands.rs` | `invoke` i `export.ts` | for hånd |
| Menyvalg | id-ene i `lib.rs::menu` | nøklene i `commands` i `main.ts` | for hånd |
| Temaer og skrifter | `settings.rs` lagrer id-en | `slideThemes.ts` eier lista | for hånd |
| Hendelsene mellom vinduene i talervisningen | – | `presenterEvents.ts` (begge sider er TS) | `tsc` |
| HTML-strukturen til slidene | `markdown.rs`, `extract.rs` (`bibel-parallell`, `<sup>`) | `slides.css`, `export.ts::parseSlide` | `test/export.test.ts` |

## Testene

- 93 tester i Rust: 91 i `kerygma-core` og 2 i `src-tauri`. Testene i `tests/` sammenligner hele utdata tegn for tegn; de er fasiten for hvordan manus blir presentasjoner, og endres med vilje når oppførselen skal endres. Referansene i `fixtures/bibel-referanser.txt` lages fra `../bible`. Flytene i `workspace.rs` testes med midlertidige mapper. Bibeltestene hoppes over uten `bibler/*.bin`.
- 29 tester av grensesnittet med Vitest og happy-dom (`ui/test/`): formatering, `sanitize.ts`, `parseSlide`, hurtigtastene, markøren og `OpenDocument` (autolagring og endringer utenfra, med falsk `api`).
- Ikke testet automatisk: talervisningen i `presenter.ts` (to vinduer), PDF-eksporten (WebKit) og selve vinduet. Disse testes for hånd.
- CI (`.github/workflows/ci.yml`): `cargo test`, `cargo clippy -D warnings` og bygging av grensesnittet på macOS og Windows, og `tsc` og Vitest. Krever hemmeligheten `BIBEL_DEPLOY_KEY`.

## Avhengigheter og bygging

- `bibel-core` hentes fra et privat git-repo over ssh (`rev = "9012cdd"`, `.cargo/config.toml` bruker git fra kommandolinja). Den som bygger, må ha tilgang.
- `bibler/*.bin` kopieres for hånd fra `../bible/assets` og pakkes som ressurser.
- `book_names.rs` lages fra `scripts/book-names.json` i dette repoet. Boknavnene og testreferansene kan hentes på nytt fra `../bible` med `node scripts/book-names.mjs --fra-bible` (krever `cargo build --release` der). Ellers trengs ikke `../bible` utenom `bibel-core` (git) og `*.bin`.
- `specta`, `specta-typescript` og `tauri-specta` er i beta og låst med `=` i `src-tauri/Cargo.toml` og `crates/kerygma-core/Cargo.toml`. Oppgrader dem samtidig.
- Windows-pakken er ikke bygget, og PDF-eksporten virker bare på macOS.

---

## Vurdering

### Det som fungerer godt

- **Skillet mellom kjerne og app holder.** `kerygma-core` vet ikke om Tauri, og CLI-en bruker samme kode. Det meste av logikken er rene funksjoner som er enkle å teste.
- **Testene med hele utdata** gjør det trygt å endre `extract.rs`, `reference.rs` og `word/convert.rs`, som er de vanskeligste delene.
- **Filformatet er åpent.** Alt er markdown, presentasjoner er vanlige filer med en HTML-kommentar som markør, og innstillingene ligger i en skjult mappe. Ingen database, ingen lås til appen.
- **Sikkerheten i stiene** er samlet i `Vault::resolve`, og eksporten kan bare skrive filer med riktig filtype.
- **Få avhengigheter i grensesnittet** og ingen rammeverk, som planen sa.

### Risikoer og teknisk gjeld

Ordnet etter hvor mye de betyr for vedlikehold over tid.

#### 1. Kontraktene mellom Rust og TypeScript er skrevet to ganger

Typene i `api.ts`, markøren, merkene i talervisningen, søkeordene og utseendet finnes både i Rust og TypeScript (se tabellen over). Et nytt felt i `Settings` må i dag legges inn fem steder: structen, `Default`, `from_json`, `api.ts` og `settings.ts`. Feil her gir ingen kompileringsfeil, bare oppførsel som er litt feil.

**Forslag:**
- Generer TypeScript-typene og kommandokallene fra Rust med [`tauri-specta`](https://github.com/specta-rs/tauri-specta) (eller `ts-rs` for bare typene). Da forsvinner det meste av `api.ts`, og en endret kommando gir feil i `tsc`.
- Gjør `Settings::from_json` om til `#[serde(default)]` på structen pluss `sanitized()`. Det gir samme oppførsel (ukjente felt ignoreres, feil type gir standard med en egen `deserialize_with` om nødvendig) med ett sted å legge til felt.
- La Rust svare på det grensesnittet nå regner ut selv: f.eks. at `read_note` (eller en ny `open_note`) returnerer `{ content, presentationSource }`, så regexen ikke trengs i `api.ts`; og at søket returnerer ordene det brukte.

#### 2. Logikk har sneket seg inn i `commands.rs`

CLAUDE.md sier at `commands.rs` bare skal være tynne kall. Det stemmer for filkommandoene, men ikke for:

- `create_presentation`: hele beslutningen om å spørre, overskrive, eller melde om manglende manus.
- `presenter`: hvilket manus som brukes og hva som leses fra disk.
- `import_targets`: hvilken mappe Word-filene havner i.
- `open_vault`/`set_settings`/`state.rs`: stien `.kerygma/settings.json` står to steder, og lesing og skriving av innstillinger ligger i Tauri-laget.
- `read_app_config`/`write_app_config`: feil ved skriving ignoreres stille.

CLI-en har dermed sin egen versjon av noe av dette (den kaller `presentation_paths_for` og `presentation_content` selv), og ingenting av det er testet.

**Forslag:** Flytt dette inn i `kerygma-core`, f.eks. en `Workspace` (vault + innstillinger) med metoder som `create_presentation(path, content, overwrite, bibel) -> CreateResult`, `presenter(...)`, `import_targets(...)`, `load_settings()`/`save_settings()`. Da kan flytene testes med en midlertidig mappe, og CLI-en og appen bruker samme kode. `commands.rs` blir igjen bare kall og låsing.

#### 3. `main.ts` er limet for alt

`main.ts` har den globale tilstanden og all flyt mellom modulene (730 linjer, og den vokser med hver funksjon). Lagring, «ulagret»-logikk, presentasjonsknappene og `vault-changed` henger tett sammen der.

**Forslag:** Del i en modul for dokumentet som er åpent (`document.ts`: sti, lagret innhold, `dirty`, autolagring, last på nytt ved `vault-changed`, med enkle hendelser ut) og en for kommandoene (`commands.ts`: id → handling, hurtigtaster, menyen). `main.ts` kobler bare sammen. Dette er ikke et argument for et rammeverk; det holder med mindre moduler med tydelig eierskap til tilstanden.

#### 4. Låsene kan blokkere hverandre

- `import_word` holder `state.bible` låst mens alle Word-filene gjøres om (med søk i Bibelen). Imens henger `lookup_verse`, `create_presentation`, `presenter` og `bible_status`.
- `create_presentation` og `presenter` holder bibel-låsen mens oversettelsene slås opp.
- `set_settings` tar bibel-låsen for å bygge registeret på nytt.

Det er sjelden et problem med én bruker, men gir et grensesnitt som «fryser» uten forklaring.

**Forslag:** Hold biblene som `RwLock<BibleState>` (mange lesere samtidig), eller som `Arc<Bibel>` som byttes ut helt når innstillingene endres (`ArcSwap` eller `Mutex<Arc<Bibel>>` der låsen bare holdes for å klone `Arc`-en). Da trenger ingen kommando å holde en lås mens den jobber.

#### 5. PDF-eksporten har tilstand mellom kall

`pdf_begin` → `pdf_page` × n → `pdf_save` deler `AppState.pdf_pages`. To eksporter samtidig, eller en eksport som avbrytes, blander sidene. Det er dessuten bare macOS (`capture.rs`), og Windows gir en feilmelding.

**Forslag:** Gi hver eksport en id (`pdf_begin` returnerer den), eller samle sidene i grensesnittet og send dem i ett kall. Vurder på sikt om PDF kan lages på en måte som virker på begge plattformer (f.eks. WebView2 `PrintToPdf` på Windows bak samme `capture::pdf`).

#### 6. Ingen tester av grensesnittet og ingen CI

Det som er mest sannsynlig å gå i stykker ved endringer, er i `ui/`: `format.ts` (redigering), `export.ts::parseSlide` (HTML → PowerPoint), merkene i `presenter.ts`, og samspillet med `vault-changed`. Ingenting av dette testes.

**Forslag:**
- Legg til Vitest med jsdom for de rene delene: `format.ts` (med en `EditorState`), `parseSlide`, `sanitize.ts`, `searchTerms`, `presenterKey`, funksjonene i `api.ts`.
- En enkel CI (GitHub Actions) med `cargo test`, `cargo clippy -D warnings` og `tsc --noEmit` på macOS og Windows. Bibeltestene hoppes over, og `bibel-core` krever en deploy-nøkkel.

#### 7. To måter å lese markdown på

`extract.rs` leser manuset linje for linje med regex (som pluginen), mens `markdown.rs` bruker comraks AST. Talervisningen bygger bro med posisjoner i teksten og usynlige tegn. Det virker, og er godt testet i `presenter.rs`, men koblingen er skjør: endres hvordan comrak håndterer tegnene (f.eks. normalisering eller escaping), forsvinner markeringene stille.

**Forslag:** Ikke bytt ut `extract.rs` så lenge pluginen er fasiten. Men legg en test som går gjennom hele kjeden (manus → `presenter_view` → finner alle merkene i HTML-en) for et par realistiske manus, så en oppgradering av comrak fanges opp.

#### 8. Bindingen til pluginen

«Testene i pluginen er fasiten» var riktig for å porte, men blir en bremse når appen skal utvikle seg videre. `book_names.rs` kan dessuten bare lages på nytt når pluginen ligger ved siden av.

**Forslag:** Bestem når appen er ferdig portet, og skriv det i CLAUDE.md. Etter det er testene i `crates/kerygma-core/tests/` fasiten, og de kan endres med vilje. Legg kilden til boknavnene (JSON) i dette repoet, eller hent dem fra `bibel-core`, så `scripts/book-names.mjs` ikke trenger `../obsidian-slide-creator`.

#### 9. Mindre ting

- **CSP er av** (`"csp": null`), så `sanitize.ts` er eneste vern mot skript i manus. En streng CSP (`script-src 'self'`, `img-src 'self' asset: http://asset.localhost data:`) ville vært et ekstra lag. Sjekk at reveal.js og den innebygde HTML-eksporten tåler det.
- **Lagring er ikke atomisk.** `write_note` bruker `fs::write` rett på fila. Krasjer appen eller maskinen midt i, kan manuset bli tomt. Skriv til en midlertidig fil i samme mappe og gi den nytt navn.
- **Søket leser alle filene hver gang.** Greit for noen hundre manus. Blir vaulten stor, kan en indeks som oppdateres av `notify` vurderes.
- **Feil som `String`** er enkelt og passer appen, men gjør det vanskelig å skille feiltyper (f.eks. «finnes ikke» fra «ingen tilgang»). Vurder en `enum` i `kerygma-core` med `Display` på norsk hvis grensesnittet trenger å reagere ulikt.
- **Planen er utdatert.** Oppdater statusen i `plans/kerygma-app-rust.md`, eller la dette dokumentet erstatte delene om struktur.
- **`localStorage` og `app.json`** lagrer begge innstillinger for appen. Det er greit, men vurder å samle det som bør overleve en ny installasjon i `app.json`.

### Rekkefølge

Et forslag, med det som gir mest igjen for minst arbeid først:

1. CI med det som finnes av tester og sjekker (liten innsats, stopper regresjoner).
2. Flytt flytene fra `commands.rs` til `kerygma-core` og test dem (punkt 2).
3. Generer typene til grensesnittet fra Rust, og forenkle `Settings` (punkt 1).
4. `RwLock`/`Arc` for biblene og id per PDF-eksport (punkt 4 og 5).
5. Vitest for de rene delene av grensesnittet, og del opp `main.ts` (punkt 3 og 6).
6. Bestem når pluginen slutter å være fasiten (punkt 8).

### Status

Gjennomført 2026-10-11. Punktene over beskriver situasjonen før endringene.

| Punkt | Status |
|---|---|
| 1. Kontrakter skrevet to ganger | **Gjort.** `ui/src/bindings.ts` lages fra Rust med tauri-specta (kommandoer, hendelser, typer og konstanter), og `cargo test` feiler hvis den er utdatert. `Settings` og `AppConfig` leses felt for felt med `json::over_default`, så et nytt felt bare legges til i structen og `Default`. Søket svarer med ordene det brukte, så `searchTerms` i TypeScript er borte. `Theme` er en enum i Rust. |
| 2. Logikk i `commands.rs` | **Gjort.** Flytene ligger i `kerygma_core::workspace::Workspace` med egne tester, og CLI-en bruker den samme koden. `app.json` er flyttet til `kerygma_core::app_config`, og feil ved lagring vises (eller logges når vaulten åpnes). |
| 3. `main.ts` er limet | **Delvis.** Dokumentet som er åpent (`document.ts`) og kommandoene (`commands.ts`) er skilt ut og testet. Presentasjonsflytene (lag, vis, eksporter) ligger fortsatt i `main.ts` og kan skilles ut på samme måte. |
| 4. Låser | **Gjort.** Biblene er `Arc<Bibel>`, og kommandoene kopierer `Workspace` og `Arc` og slipper låsen før de jobber. Nye innstillinger gir en ny `Bibel` som byttes inn. |
| 5. PDF-eksport | **Gjort for tilstanden:** hver eksport har en id, og sidene glemmes hvis noe går galt (`pdf_cancel`). **Ikke gjort:** PDF på Windows (WebView2 `PrintToPdf`). |
| 6. Tester og CI | **Gjort.** Vitest med happy-dom for de rene delene, og CI på macOS og Windows. Hemmeligheten `BIBEL_DEPLOY_KEY` må legges inn i GitHub før CI kan hente `bibel-core`. |
| 7. To måter å lese markdown | **Gjort:** testen `alle_slides_har_merkene_sine_i_html_en` går gjennom hele kjeden. |
| 8. Bindingen til pluginen | **Gjort.** Appen har ingen avhengighet til `../obsidian-slide-creator`. Testene i `crates/kerygma-core/tests/` er fasiten. Det som fortsatt bare fantes i pluginen, er flyttet hit: fontene og lisensene (`ui/fonts/`, `ui/public/lisenser/`) med `scripts/embed-fonts.mjs`, generatoren for boknavn og testreferanser (nå i `scripts/book-names.mjs --fra-bible`, rett fra `../bible`), planen for sitatoversettelse (`plans/oversette-sitater.md`) og beskrivelsen av Word-importen (`docs/word-import.md`). Det ubrukte feltet `openCopy` er fjernet, og standardverdiene til innstillingene kommer fra Rust (`DEFAULT_SETTINGS`). |
| 9. Mindre ting | **Gjort:** CSP, atomisk lagring (manus, innstillinger, `app.json`, eksport), og den gamle porteringsplanen er erstattet av dette dokumentet (teknologivalgene og videre arbeid står nederst). **Ikke gjort (vurderinger, ikke feil):** søkeindeks, feiltyper som `enum`, og å samle `localStorage` i `app.json`. |

CSP-en og resten av grensesnittet er sjekket med `tsc`, Vitest og bygging, men ikke prøvd for hånd i appen. Prøv særlig lesevisning med bilder, presentasjon med temaer, talervisning og alle tre eksportene.

## Teknologivalg

| Del | Valg | Hvorfor |
|---|---|---|
| App-ramme | **Tauri 2** | All logikk er i Rust, mens grensesnittet er HTML/CSS. Da kan `slides.css` (med fontene) og reveal.js brukes, og HTML-eksporten ser ut som i appen. Appen blir liten (ca. 10 MB + biblene). |
| Grensesnitt | Vanlig TypeScript med Vite, uten rammeverk | Appen har få skjermbilder. Et rammeverk gir mer å lære enn det sparer. |
| Kontrakt Rust ↔ TS | **tauri-specta** | Typene og kommandoene lages fra Rust, så de ikke kan gli fra hverandre. |
| Editor | **CodeMirror 6** med `@codemirror/lang-markdown` | Markdown-farger, angre, søk i fila og markørposisjon (som verseinnsetting trenger) er innebygd. |
| Markdown → HTML | **comrak** (Rust) | CommonMark + GFM, gir et AST som presentasjonen kan deles på, og har wikilenker. |
| Presentasjon | **reveal.js 5** (npm, bundlet lokalt) | Moden, virker uten nett, og kan bygges inn i én HTML-fil ved eksport. |
| Bibel | `bibel-core` fra `bible`-repoet som crate | Oppslag, søk og oversettelser uten egen kode; samme svar som kjørbaren `bibel`. |
| Regex | **fancy-regex** der det trengs lookbehind/lookahead, ellers `regex` | Referansene trenger `(?<!…)`, `(?=…)` og `\p{L}`. `regex`-craten støtter ikke lookaround. |
| Word | `zip` + `quick-xml` | Leser `word/document.xml` direkte. |
| PDF | `createPDF` i WKWebView + `lopdf` | Teksten blir tekst med de samme skriftene som på skjermen. |
| PowerPoint | `pptxgenjs` (lastes først når den brukes) | Gir tekstbokser som kan rettes. |
| Filer | `notify` (endringer utenfra), `trash` (slett til papirkurven), `serde_json` (innstillinger) | |
| Tester av grensesnittet | Vitest + happy-dom | Raskt, og bruker samme oppsett som Vite. |

Det er vurdert å bruke egui eller iced i stedet (bare Rust, ingen webview). De er avvist fordi presentasjonene da må tegnes uten CSS: kolonner, versnumre som `<sup>`, innebygde fonter og bildeskalering.

## Videre arbeid

- Oversette sitater automatisk med DeepL: [`plans/oversette-sitater.md`](../plans/oversette-sitater.md).
- Hurtigåpning (Cmd/Ctrl + O).
- PDF-eksport på Windows (WebView2 `PrintToPdf` bak `capture::pdf`), og å bygge og prøve Windows-pakken.
- Skille presentasjonsflytene (lag, vis, eksporter) ut av `main.ts`.
- Søkeindeks som oppdateres av `notify`, hvis vaultene blir store.
