# Plan: Kerygma, en app i Rust

**Status:** Steg 1–14 er bygget. Grensesnittet er ikke testet for hånd ennå, og Windows-pakken er ikke bygget (den må bygges på Windows).
**Opprettet:** 2026-10-10
**Navn:** Kerygma

## Mål

En enkel skrivebordsapp, skrevet i Rust, som gjør det Obsidian og Slide Creator-pluginen gjør i dag, og ikke mer:

- åpne en mappe med markdown-filer (en «vault») og vise den som et tre
- skrive og lese markdown, med forhåndsvisning
- lage en presentasjon av et manus («Lag presentasjon»), med nøyaktig samme regler som pluginen
- vise presentasjonen i fullskjerm, med samme utseende som `slides.css`
- sette inn bibelvers med oppslag og søk («Sett inn bibelvers»)
- importere gamle Word-manus (`.docx`)

Filene skal fortsatt være vanlig markdown, så en vault kan åpnes både i Obsidian og i appen.

### Ikke med

Disse tingene skal ikke bygges: plugins, lenkegraf, backlinks, sync, faner, delte vinduer, temaer og mobil. Søk i hele vaulten og «hurtigåpning» kan komme senere (se *Senere*).

## Kildene

Appen bygges i dette repoet (`kerygma`). To andre repoer inneholder det som trengs:

| Repo                                                                 | Hva | Brukes slik |
|----------------------------------------------------------------------|---|---|
| `obsidian-slide-creator`, ligger i  `../obsidian-slide-creator`      | pluginen i TypeScript, testene, `slides.css`, Word-importen | **bare som kilde til informasjon**: hva appen skal gjøre, og hvilke inn- og utdata testene forventer. Testene er fasiten. |
| `bible` (`git@github.com:eirikandre/bible.git`, ligger i `../bible`) | `bibel` i Rust: oppslag, søk og oversettelsene `assets/*.bin` | brukes direkte som crate, uten å portes |

**Ingenting i `obsidian-slide-creator` skal endres.** Ikke skriv, flytt, slett, bygg eller commit noe der, og ikke kjør `npm install` eller andre kommandoer som lager filer i repoet. Les filene direkte fra `../obsidian-slide-creator` når du trenger dem. Det som skal finnes i appen, skrives på nytt i Rust (eller TypeScript i `ui/`) i dette repoet. Unntaket er `slides.css` og testfiksturen `bibel-referanser.txt`, som kopieres inn uendret.

Dette er filene i `obsidian-slide-creator` som beskriver det som skal lages, og hvor det havner her:

```
src/extract.ts                 -> crates/kerygma-core/src/extract.rs
src/bibleReference.ts          -> crates/kerygma-core/src/reference.rs
src/bookNames.ts               -> crates/kerygma-core/src/book_names.rs (generert)
src/bible.ts                   -> crates/kerygma-core/src/bible.rs
src/verseQuery.ts              -> crates/kerygma-core/src/verse_query.rs
src/verseModal.ts              -> ui/src/verseModal.ts (oppførselen)
src/main.ts                    -> innstillinger, kommandoer og flyt
src/importWord.ts              -> src-tauri/src/commands.rs (import)
word-to-markdown/src/*.ts      -> crates/kerygma-core/src/word/*.rs
test/*.test.ts                 -> tester i Rust
test/fixtures/bibel-referanser.txt -> crates/kerygma-core/tests/fixtures/ (kopieres)
word-to-markdown/test/*.test.ts
snippets/slides.css            -> ui/public/slides.css (kopieres uendret)
README.md, docs/Brukerveiledning.md, word-to-markdown/README.md
planer/oversette-sitater.md    (en senere utvidelse)
```

## Teknologivalg

| Del             | Valg                                                                                        | Hvorfor                                                                                                                                                                                                |
|-----------------|---------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| App-ramme       | **Tauri 2**                                                                                 | All logikk er i Rust, mens grensesnittet er HTML/CSS. Da kan `slides.css` (med fontene) og reveal.js brukes uendret, og presentasjonene ser ut som i Obsidian. Appen blir liten (ca. 10 MB + biblene). |
| Grensesnitt     | Vanlig TypeScript med Vite, uten rammeverk                                                  | Appen har få skjermbilder. Et rammeverk gir mer å lære enn det sparer.                                                                                                                                 |
| Editor          | **CodeMirror 6** med `@codemirror/lang-markdown`                                            | Markdown-farger, angre, søk i fila og markørposisjon (som verseinnsetting trenger) er innebygd. En `<textarea>` blir fort for enkel.                                                                   |
| Markdown → HTML | **comrak** (Rust)                                                                           | CommonMark + GFM, gir et AST som presentasjonen kan deles på, og har wikilenker.                                                                                                                       |
| Presentasjon    | **reveal.js 5** (npm, bundlet lokalt)                                                       | Det er det Obsidians Slides bruker. `slides.css` er skrevet for `.slides-container .reveal`.                                                                                                           |
| Bibel           | `bibel-core` fra `bible`-repoet som crate                                                   | Samme svar som pluginen får fra WASM-modulen, men uten WASM.                                                                                                                                           |
| Regex           | **fancy-regex** der det trengs lookbehind/lookahead, ellers `regex`                         | Pluginen bruker `(?<!…)`, `(?=…)` og `\p{L}`. `regex`-craten støtter ikke lookaround.                                                                                                                  |
| Word            | `zip` + `quick-xml`                                                                         | Erstatter den håndskrevne zip/XML-lesingen i `docx.ts`.                                                                                                                                                |
| Filer           | `notify` (endringer utenfra), `trash` (slett til papirkurven), `serde_json` (innstillinger) |                                                                                                                                                                                                        |
| Dialoger        | `tauri-plugin-dialog`                                                                       | Velge vault-mappe og `.docx`-filer, og spørre «Overskriv?».                                                                                                                                            |

Det er vurdert å bruke egui eller iced i stedet (bare Rust, ingen webview). De er avvist fordi presentasjonene da må tegnes på nytt uten CSS: kolonner, versnumre som `<sup>`, innebygde fonter og bildeskalering. Det er mest arbeid og gir minst likhet med det som virker i dag.

## Struktur

```
kerygma/
  Cargo.toml                    workspace: crates/*, src-tauri
  crates/
    kerygma-core/                 all logikk, uten Tauri; testes med cargo test
      src/
        lib.rs
        reference.rs            bibelReference.ts
        book_names.rs           BOOK_IDS, generert
        extract.rs              extract.ts
        verse_query.rs          verseQuery.ts
        bible.rs                laster *.bin, oppslag, oversettelser (bible.ts)
        markdown.rs             markdown -> HTML, og deling i slides
        vault.rs                filtre, stier, oppslag av ![[bilde]]
        settings.rs             innstillinger med standardverdier
        word/
          mod.rs                import: docx -> markdown + rapport
          docx.rs
          convert.rs
          align.rs
          bible_text.rs
      tests/
        extract.rs  reference.rs  verse_query.rs  bible.rs  convert.rs
        fixtures/bibel-referanser.txt
    kerygma-cli/                  valgfri: samme funksjoner fra terminalen (erstatter konverter.ts)
  src-tauri/
    Cargo.toml  tauri.conf.json  capabilities/default.json
    src/main.rs  src/commands.rs  src/state.rs
  ui/
    index.html  vite.config.ts  package.json  tsconfig.json
    src/main.ts  tree.ts  editor.ts  preview.ts  present.ts  verseModal.ts
        settings.ts  importReport.ts  dialogs.ts  api.ts  styles.css
    public/slides.css           kopiert fra snippets/slides.css
  bibler/                       *.bin fra ../bible/assets (ikke i git, se under)
  scripts/
    book-names.mjs              lager book_names.rs (se steg 1)
  README.md  CLAUDE.md
```

`kerygma-core` skal ikke vite om Tauri. All logikk kan da testes med `cargo test`, og CLI-en blir gratis.

## Bibel-avhengigheten

```toml
# crates/kerygma-core/Cargo.toml
[dependencies]
bibel-core = { git = "ssh://git@github.com/eirikandre/bible.git", rev = "9012cdd" }
# eller under utvikling, når ../bible ligger ved siden av:
# bibel-core = { path = "../../../bible/crates/bibel-core" }
```

- `bibel_core::Registry::parse(&[&'static [u8]])` vil ha data som lever hele programmet. Les `.bin`-filene med `std::fs::read` og bruk `Box::leak`, slik `bibel-wasm` gjør. De lastes én gang.
- `bibel_core::run(&registry, &args, |_| None)` tar de samme argumentene som `Bibel.run` i pluginen og gir `Output { stdout, code }`. Bruk den, og ikke modulene `reference`/`search` direkte, så svarene blir de samme som før:
  - oppslag: `["joh3:16", "--alltid-null"]`
  - søk: `["--søk", "tro håp", "--antall", "10", "--alltid-null"]`
  - oversettelse: `["--format", "{tekst} ({ref})", "kjv:John3:16"]`
- Standardoversettelse og snarveier: `registry.set_default("nb88")` og `registry.set_aliases("n=nb88,e=kjv,ne=nb88+kjv")`. Bygg registeret på nytt når innstillingene endres (se `State::rebuild` i `bibel-wasm/src/lib.rs`).
- Oversettelsene (`kjv.bin`, `nb88.bin`, `nb24.bin`, 4 MB hver) legges i `bibler/` og pakkes med appen som Tauri-ressurser (`bundle.resources: { "../bibler/*.bin": "bibler/" }`). Finn dem med `app.path().resource_dir()`. De lastes i en egen tråd når appen starter, så vinduet ikke venter.
- Hold registeret i `tauri::State<Mutex<Option<Registry>>>` (eller `OnceLock` + `RwLock` for innstillingene).

## Funksjoner

### 1. Vault og filtre

- Første gang: «Åpne mappe» (dialog). Sist åpnede mappe huskes i appens konfigmappe (`app.path().app_config_dir()/app.json`).
- Treet viser mapper og filer sortert med mapper først, alfabetisk og uten hensyn til store og små bokstaver. Skjul alt som starter med `.` (også `.obsidian`). Vis `.md` uten filendelse. Bilder og andre filer vises med filendelse og grå.
- Mapper kan foldes ut og inn. Hvilke som er åpne, huskes per vault.
- Høyreklikkmeny:
  - på en mappe: *Ny note*, *Ny mappe*, *Gi nytt navn*, *Slett*
  - på en fil: *Gi nytt navn*, *Slett*, *Lag presentasjon*
  - *Slett* legger fila i papirkurven (`trash`), etter at brukeren har bekreftet.
- *Ny note* lager `Uten navn.md` (`Uten navn 1.md` osv.) og åpner fila med navnet markert for redigering.
- Endringer utenfra (Obsidian, Finder, Word-import) fanges opp med `notify` og sendes til grensesnittet som en hendelse (`vault-changed`). Treet bygges da på nytt. Er den åpne fila endret utenfra og ikke i editoren, lastes den på nytt.
- Alle stier mellom grensesnitt og Rust er relative til vault-roten og bruker `/`. Rust avviser stier som går ut av vaulten (`..`, absolutte stier).

### 2. Editor og forhåndsvisning

- Én fil om gangen. Venstre: treet. Høyre: editoren.
- CodeMirror med markdown-farger, linjebryting og ingen linjenumre.
- **Lagrer automatisk** 1 sekund etter siste tastetrykk, og når en annen fil åpnes eller appen lukkes. Ingen «Lagre»-knapp, som i Obsidian.
- **Cmd/Ctrl + E** bytter mellom redigering og lesevisning. Lesevisningen er HTML fra `markdown.rs` (se under), med bilder.
- Tittelen over editoren er filnavnet. Klikk på den for å gi nytt navn.

#### Markdown → HTML (`markdown.rs`)

Bruk comrak med `table`, `strikethrough`, `tasklist`, `autolink`, `footnotes` og `wikilinks_title_after_pipe`, `front_matter_delimiter = "---"` og `render.unsafe_ = true` (presentasjonen inneholder `<div class="bibel-parallell">` og `<sup>`). I tillegg trengs dette, som Obsidian gjør:

| Syntaks | Gjør |
|---|---|
| frontmatter | vises ikke |
| `%% … %%` | fjernes (også over flere linjer, ikke inne i kodeblokker) |
| `==tekst==` | `<mark>tekst</mark>` (bruk comraks `highlight` hvis versjonen har det, ellers forbehandling utenfor kode) |
| `> [!type] Tittel` | callout: `<div class="callout" data-callout="type">` med tittel. Det holder med enkel styling. |
| `![[bilde.png]]`, `![[bilde.png\|400]]` | `<span class="internal-embed image-embed"><img src="…" width="400"></span>`. Samme DOM som Obsidian, så CSS-en for bilder i `slides.css` treffer. |
| `![alt](sti.png)` | `<img>` med stien gjort om (se under) |
| `[[Notat]]`, `[[Notat\|tekst]]` | lenke som åpner notatet i appen |

**Bildestier:** `![[navn.png]]` slås opp som i Obsidian. Først prøves stien relativt til vault-roten, så relativt til mappa notatet ligger i, og til slutt første fil i vaulten med det filnavnet. Relative stier i `![](…)` er relative til notatet. `src` blir en URL via Tauris asset-protokoll (`convertFileSrc` i grensesnittet, eller `asset://localhost/<absolutt sti>` fra Rust). Tillat vault-mappa med `app.asset_protocol_scope().allow_directory(vault, true)` når vaulten åpnes, og slå på `app.security.assetProtocol.enable` i `tauri.conf.json`.

### 3. Lag presentasjon (Cmd/Ctrl + Shift + M)

Skriv `extract.rs` etter `extract.ts` med alle reglene og alle testene i `test/extract.test.ts`. Kort fortalt, slik den er i dag:

- Beholder frontmatter, overskrifter, bibelvers, sitater (`>`) og bilder. Alt annet fjernes. Hvert element blir én slide, skilt med `\n\n---\n\n`.
- Kodeblokker og `%% … %%` hoppes over.
- Et vers er tekst fram til og med en `(referanse)` som `reference.rs` godtar. Samme vers i flere oversettelser på linjene etter hverandre blir `<div class="bibel-parallell">` med én `<p>` per oversettelse.
- Vers som står alene, får oversettelsen fra innstillingene ved siden av (`lookUpTranslations` + `loneVerseReferences`).
- Lange vers og sitater deles over flere slides etter `maxChars` (standard 350). Kolonner får `COLUMN_SHARE` = 0,6 av grensen. Det deles helst ved versnummer, så ved setning, så ved ord. Fet og kursiv lukkes og åpnes igjen over delingen. I kolonner blir markdown til `<strong>`/`<em>`/`<mark>`, og versnumre blir `<sup>`.
- `[!warning|sjekk]`-tittelen fra Word-importen fjernes, og det samme gjelder `>` og `[!type]`.
- Bilder (`![[x.png]]` med bildeendelse og `![](…)`) blir egne slides i rekkefølge med teksten rundt. `![[Notat]]` tas ikke med.

Flyten (fra `createSlideCopy` i `main.ts`):

1. Ta innholdet fra editoren hvis fila er åpen (ulagrede endringer skal med), ellers fra disk.
2. Presentasjonen heter `<navn><suffix>.md` (suffix standard `" - presentasjon"`) og legges i samme mappe som manuset, så relative bildestier virker.
3. Kjøres kommandoen fra en presentasjon, lages den på nytt fra manuset ved siden av. Finnes ikke manuset, vises en melding.
4. Finnes presentasjonen fra før, spør «"<sti>" finnes allerede. Vil du overskrive den?» med knappene *Avbryt* og *Overskriv*.
5. Går oversettelsen ikke (biblene er ikke lastet, ukjent oversettelse), lages presentasjonen uten, og det vises én melding.
6. Melding: «Presentasjon laget: <navn>». Åpne den hvis *Åpne presentasjonen* er på.

### 4. Vis presentasjon (Cmd/Ctrl + Shift + P, og knapp over editoren)

Gjør det samme som Obsidians Slides (se `obsidian.asar`: den deler markdown-AST-et på `thematicBreak`):

1. `markdown.rs::slides(markdown, note_path) -> Vec<String>` fjerner frontmatter, parser med comrak, deler rotens barn på `ThematicBreak`, hopper over tomme grupper og gjør hver gruppe om til HTML med reglene over (bilder løses opp på samme måte).
2. Grensesnittet bygger denne DOM-en i et overlegg over hele vinduet og setter vinduet i fullskjerm (`getCurrentWindow().setFullscreen(true)`):
   ```html
   <div class="slides-container">
     <div class="reveal"><div class="slides">
       <section>…</section> …
     </div></div>
   </div>
   ```
3. Last `reveal.js/dist/reveal.css`, temaet `reveal.js/dist/theme/black.css` og så `slides.css`, i den rekkefølgen. Start med `new Reveal(el, { embedded: true, controlsTutorial: false, overview: false, hash: false })` og `initialize()`. Bruk 960 × 700, som er standard.
4. Piltaster, mellomrom og PageUp/PageDown (fjernkontroller for presentasjon) blar. Esc avslutter, går ut av fullskjerm og fjerner overlegget (`deck.destroy()`).
5. Startes visningen fra et manus som ikke er en presentasjon, vises manuset som det er (som i Obsidian). Knappen kan tilby «Lag presentasjon først».

### 5. Sett inn bibelvers (Cmd/Ctrl + Shift + B)

Port `verseQuery.ts` (`parse_verse_query`, `to_quote`) med testene, og lag vinduet etter `verseModal.ts`:

- Et søkevindu midt på skjermen med plassholderen «Bibelreferanse, som joh3:16, ne:sal23 eller s=tro håp».
- For hvert tastetrykk: `parse_verse_query`. Oppslag gir ett forslag, mens søk (`s=…`, minst 2 tegn) gir opptil 10. Linjer i `[hakeparentes]` er meldinger som vises små og ikke kan velges.
- Forslag vises med maks 300 tegn og `…`.
- Piltaster velger. Enter setter inn ved markøren som `> tekst (ref)`, med én linje per oversettelse. Står markøren midt i en linje, kommer sitatet på egne linjer med tom linje før og etter (`to_quote`). Esc avbryter.
- Rust-kommandoen `lookup_verse(input) -> Vec<Suggestion>` gjør tolkningen og kaller `bibel_core::run`, så grensesnittet bare viser resultatet.

### 6. Importer Word-manus

Port `word-to-markdown/src/` til `word/` med testene i `word-to-markdown/test/convert.test.ts`. Testene bruker en falsk bibel (`BibleText`-traitet), så de trenger ikke `bibler/`.

- `docx.rs`: les `word/document.xml` fra zip-fila med `quick-xml`. Ta med avsnitt, stil (`w:pStyle`), fet og kursiv per run, og listeavsnitt. Se `docx.ts` for hvilke elementer som brukes.
- `align.rs`: Smith-Waterman på ord (MATCH 2, SIMILAR 1, MISMATCH −1, GAP −1). Ord som er like med én bokstavs forskjell, regnes som «like».
- `bible_text.rs`: `trait BibleText { translations, book_id, verse, reference, search }`, implementert med `bibel_core` med oversettelsene `["nb88", "nb24"]`.
- `convert.rs`: hele logikken for referanser, `V1`/`vers 7`, kapittel fra overskrifter, søk etter vers uten referanse, `[!warning|sjekk]`-advarslene og rapporten (`ConvertResult`: markdown, funne sitater, referanser uten sitat).
- Flyt: velg én eller flere `.docx` (dialog). Filene legges i *Mappe for importerte Word-manus*, eller i mappa til fila som er åpen. Spør før en eksisterende fil overskrives. Til slutt vises en rapport per fil: lenke til notatet, antall sitater, hva som bør sjekkes og feil. Er det bare én fil, åpnes den.

### 7. Innstillinger

Lagres per vault i `<vault>/.kerygma/settings.json`, med de samme feltene og standardverdiene som pluginen. Ukjente felt fjernes når fila leses (som `loadSettings`).

| Felt | Standard | Vises som |
|---|---|---|
| `suffix` | `" - presentasjon"` | Postfix for presentasjon |
| `maxChars` | `350` | Maks tegn per slide |
| `addTranslation` | `true` | Legg til oversettelse |
| `translation` | `"kjv"` | Oversettelse |
| `defaultTranslation` | `"nb88"` | Standardoversettelse |
| `aliases` | `"n=nb88,e=kjv,ne=nb88+kjv"` | Snarveier |
| `importFolder` | `""` | Mappe for importerte Word-manus |
| `openCopy` | `true` | Åpne presentasjonen |

Beskrivelsene står i `SlideCreatorSettingTab` i `src/main.ts` og kan kopieres. Ugyldige verdier (tom suffix, `maxChars` ≤ 0) gir standardverdien. Innstillingene åpnes med Cmd/Ctrl + , og et tannhjul nederst i treet.

### 8. Hurtigtaster og meny

| Tast | Gjør |
|---|---|
| Cmd/Ctrl + N | Ny note i mappa til fila som er åpen |
| Cmd/Ctrl + E | Bytt mellom redigering og lesevisning |
| Cmd/Ctrl + Shift + M | Lag presentasjon av manus |
| Cmd/Ctrl + Shift + P | Vis presentasjon |
| Cmd/Ctrl + Shift + B | Sett inn bibelvers |
| Cmd/Ctrl + , | Innstillinger |

Tauri-menyen (macOS-menylinjen) har de samme valgene og *Åpne mappe* og *Importer Word-manus*. Det skal ikke være en kommandopalett, og tastene kan ikke endres i første versjon.

Meldinger (Obsidians `Notice`) er en liten boks nederst til høyre som forsvinner etter 4 sekunder. Feil blir stående til de klikkes bort.

## Tauri-kommandoer

Alle i `src-tauri/src/commands.rs`, med tynne kall til `kerygma-core`. Feil returneres som `Result<T, String>` med en norsk melding.

```rust
open_vault(path: String) -> Tree                 // husker den, starter notify, tillater asset-scope
tree() -> Tree                                   // Tree { name, path, kind: Folder|Note|File, children }
read_note(path) -> String
write_note(path, content)
create_note(folder) -> String                    // gir stien til "Uten navn.md"
create_folder(parent, name) -> String
rename(path, new_name) -> String
delete(path)                                     // til papirkurven
render_note(path, content) -> String             // HTML til lesevisning
create_presentation(path, content: Option<String>, overwrite: bool) -> CreateResult
    // CreateResult::Created{path} | Exists{path} (grensesnittet spør og kaller igjen med overwrite)
    //   | MissingManuscript{path}, + warnings: Vec<String>
slides(path) -> Vec<String>                      // HTML per slide
lookup_verse(input) -> Vec<Suggestion>           // Suggestion { text: Option, message: Option }
import_word(files: Vec<String>, overwrite: Vec<String>) -> Vec<ImportedFile>
get_settings() -> Settings
set_settings(Settings)                           // bygger bibel-registeret på nytt ved behov
bible_status() -> BibleStatus                    // Loading | Ready{ids} | Failed{message}
```

Hendelser fra Rust: `vault-changed`, `bible-ready`.

## Rekkefølge

Hvert steg skal være ferdig, med tester som passerer, før det neste startes.

1. **Oppsett.**
   - Lag workspace og `create-tauri-app` (vanilla-ts), og få et tomt vindu opp.
   - Lag `scripts/book-names.mjs`, som leser `BOOK_IDS` fra `../obsidian-slide-creator/src/bookNames.ts` (bare lesing) og skriver `book_names.rs` som en `phf`-map eller sortert `&[(&str, &str)]`.
   - Skriv `CLAUDE.md` med kommandoene og regelen om at testene i `../obsidian-slide-creator` er fasiten, og at ingenting i det repoet skal endres.
2. **`reference.rs`** med testene for gjenkjenning, avvisning, `toLookup` og fasiten med over 600 referanser (`bibel-referanser.txt`).
3. **`extract.rs`** med alle de 22 testene fra `extract.test.ts`, med samme inn- og utdata, tegn for tegn.
4. **`bible.rs`**: last biblene, `lookup_translations`, `configure`. Port `test/bible.test.ts`. Testene som trenger `.bin`, hoppes over når `bibler/` mangler.
5. **`verse_query.rs`** med testene.
6. **CLI** (`kerygma lag-presentasjon <fil.md>`, `kerygma vers <ref>`). Kjør på ekte manus og sammenlign med presentasjoner pluginen allerede har laget i Obsidian (`diff`). Ikke bygg eller kjør pluginen fra repoet.
7. **Vault + tre + editor + autolagring** i appen.
8. **Lesevisning** (`markdown.rs`) med bilder.
9. **Lag presentasjon** i appen, med spørsmål om overskriving og meldinger.
10. **Vis presentasjon** med reveal.js og `slides.css`. Sjekk kolonner, versnumre, lange vers, bilder stående og liggende, og fonter uten nett.
11. **Sett inn bibelvers.**
12. **Innstillinger.**
13. **Word-import** med testene fra `convert.test.ts`, så i CLI-en (`kerygma importer`), så i appen med rapporten.
14. **Pakking**: `cargo tauri build` for macOS (`.dmg`) og Windows (`.msi`), med `bibler/*.bin` som ressurser. Ikoner. Skriv brukerveiledningen på nytt ut fra `docs/Brukerveiledning.md`.

## Fallgruver ved porting fra TypeScript

- **Tegnlengder:** JavaScript teller `length` i UTF-16-enheter, mens Rust gir bytes. `maxChars`, delingen i `splitText` og `PREVIEW_CHARS` skal telle tegn (`chars().count()`). Indekser fra regex er byte-indekser i Rust. Del alltid strenger på `char_indices`-grenser.
- **Regex:** `\p{L}`, navngitte grupper og `u`-flagget finnes i `regex`. Lookbehind `(?<![\p{L}\p{N}])` og lookahead `(?=\s)` gjør det ikke, så bruk `fancy-regex` for de mønstrene, eller skriv sjekken i kode. Bygg mønstrene én gang med `LazyLock`.
- **Private tegn:** `extract.ts` bruker ``/`` som midlertidige markører for versnumre. Det samme kan brukes i Rust (`'\u{E000}'`).
- **`String.replace` med regex uten `g`** erstatter bare første treff. Bruk `replacen(…, 1)` / `Regex::replace` (ikke `replace_all`).
- **Linjeskift:** Manus fra Windows har `\r\n`. Del med `lines()`, eller på `\r?\n` som i `parse`.
- **Frontmatter:** bare når første linje er nøyaktig `---`.
- **Utdata skal være like tegn for tegn**, også linjeskiftet på slutten og `\n\n---\n\n` mellom slides. Testene sammenligner hele strenger.

## Tester

- `cargo test -p kerygma-core` kjører alle porterte tester. Hver test i `.test.ts` blir én `#[test]` med samme norske navn som funksjonsnavn (`fn hvert_bilde_blir_en_egen_slide()`), og samme strenger.
- Tester som trenger ekte oversettelser, ser etter `bibler/kjv.bin` og hopper over med en melding når den mangler.
- Grensesnittet testes for hånd med en testvault: noen manus, et Word-manus, bilder stående og liggende, og en eksisterende Obsidian-vault, for å sjekke at ingenting i `.obsidian` blir rørt.
- Første gang presentasjoner lages, sammenlign utdataene fra `kerygma lag-presentasjon` med presentasjoner pluginen har laget i Obsidian for 5–10 ekte manus.

## Senere (ikke i første versjon)

- Vise presentasjonen i et eget vindu på projektoren, med notater og neste slide på egen skjerm (Tauri `WebviewWindow` på valgt skjerm).
- Søk i hele vaulten og hurtigåpning (Cmd/Ctrl + O).
- Automatisk oversettelse av sitater med DeepL (se `../obsidian-slide-creator/planer/oversette-sitater.md`; bruk `reqwest` i Rust i stedet for `requestUrl`).
- Eksportere presentasjonen til PDF.
