# Plan: Oversette sitater automatisk

**Status:** Ikke startet
**Opprettet:** 2026-10-06 (som plan for Obsidian-pluginen), skrevet om for Kerygma 2026-10-11

## Mål

Sitater i manuset (`> ...`) som ikke er bibeltekst, skal få en engelsk oversettelse ved siden av når presentasjonen lages, på samme måte som bibelvers får engelsk fra `bibel` i dag.

## Valg av tjeneste

| Tjeneste | Gratis grense | Nøkkel | Kvalitet nb → en | Merknad |
|---|---|---|---|---|
| **DeepL API Free** | 500 000 tegn/mnd | Ja, gratis konto | Best | Ett manus bruker trolig noen tusen tegn |
| MyMemory | 5 000 tegn/dag, 50 000 med e-post | Nei | Varierende | Maks 500 byte per forespørsel |
| LibreTranslate | Gratis hvis du kjører den selv | Nei, når du kjører den selv | Middels | Den offentlige tjenesten krever betalt nøkkel |
| Lokal modell (TranslateLocally/Bergamot, LLM via Ollama) | Ubegrenset | Nei | Middels til god | Virker uten nett, men må installeres |

**Valg: DeepL API Free.** Den gir klart best kvalitet for norsk, og grensen er mer enn nok.

Sitatene sendes til DeepL sine servere. Sjekk vilkårene for gratisversjonen før sensitive sitater sendes. En lokal modell er alternativet hvis det er et problem.

### Forutsetning

Opprett en gratis konto på <https://www.deepl.com/pro-api> og hent API-nøkkelen. Nøkler for gratisversjonen slutter på `:fx` og bruker `https://api-free.deepl.com`.

## Oppførsel

- Sitater som ikke er bibeltekst, blir to kolonner (`<div class="bibel-parallell">`): norsk til venstre og engelsk til høyre. CSS-en finnes allerede i `ui/public/slides.css`.
- Lange sitater deles likt i begge kolonnene. Uten versnumre deles de ved setninger.
- Grensen per kolonne blir den samme som for bibeltekst med oversettelse, `max_chars * COLUMN_SHARE`.
- Ikke oversett:
  - sitater som allerede er på engelsk (DeepL returnerer `detected_source_language`; hopp over hvis den er `EN`)
  - tomme sitater
- Mangler nett eller nøkkel, lages presentasjonen uten oversettelse, og brukeren får én melding (som `warnings` i `CreateResult::Created`).
- Talervisningen (`Workspace::presenter`) bruker de samme oversettelsene som «Lag presentasjon».

## Hvor koden hører hjemme

Alt utenom innstillingssiden skrives i Rust i `kerygma-core`, så det kan testes med `cargo test` og brukes fra CLI-en. Forespørselen til DeepL går fra Rust, ikke fra grensesnittet: da slipper CSP-en å tillate `api-free.deepl.com`, og nøkkelen blir ikke liggende i webvisningen.

### 1. `crates/kerygma-core/src/translate.rs` (ny)

```rust
pub trait Translator {
    /// Oversetter tekstene (nb → en). Gir én oversettelse per tekst, eller `None` for tekst som allerede er engelsk.
    fn translate(&self, texts: &[String]) -> Result<Vec<Option<String>>, String>;
}

pub struct DeepL { api_key: String }
impl Translator for DeepL { … }

/// Oversettelsene for sitatene, fra mellomlageret eller fra `translator`. Nye legges i `cache`.
pub fn translate_quotes(quotes: &[String], translator: &dyn Translator, cache: &mut QuoteCache)
    -> Result<HashMap<String, String>, String>;
```

- HTTP med `ureq` (blokkerende og liten; kommandoene i `src-tauri` kjører allerede utenfor hovedtråden med `#[tauri::command(async)]`). Legg den bak en feature (`translate`), så `kerygma-core` uten featuren ikke trenger nett.
- Send alle sitater som ikke finnes i mellomlageret, i én forespørsel. DeepL tar flere `text` per kall (maks 50 tekster og 128 KiB).
  - `POST {base}/v2/translate`
  - `Authorization: DeepL-Auth-Key <nøkkel>`
  - `source_lang: "NB"`, `target_lang: "EN-GB"`
  - `preserve_formatting: true`, slik at linjeskift beholdes
- Velg `base` ut fra nøkkelen: `api-free.deepl.com` hvis den slutter på `:fx`, ellers `api.deepl.com`.
- Feil med norske meldinger: `403` (feil nøkkel), `456` (kvoten er brukt opp) og nettverksfeil.

### 2. `crates/kerygma-core/src/extract.rs`

- Legg til `quote_translations: HashMap<String, String>` i `SlideOptions`.
- Ny funksjon `quotes_to_translate(markdown) -> Vec<String>`. Den henter sitatene (`Item::Quote`) fra `parse()`, slik `lone_verse_references` gjør for vers.
- I `extract_slides`: Har et sitat en oversettelse, blir det kolonner. Gjenbruk `split_parallel` og kolonne-HTML-en i `verse_slides`, men uten referanse og uten `mark_verse_numbers`.
  - Trekk ut en felles `column_slides(columns)` fra `verse_slides`, så HTML-en bare skrives ett sted.
  - Husk `escape_html` på begge kolonnene.
  - Behold `Slide::source`, så talervisningen fortsatt markerer sitatet.

### 3. `presentation.rs` og `workspace.rs`

- `slide_options` henter sitatoversettelsene ved siden av bibeloversettelsene, når innstillingen er på.
- Mellomlageret lagres i vaulten, i `<vault>/.kerygma/sitater.json` (norsk tekst → engelsk), med `vault::write_atomic`. Da oversettes et sitat bare én gang, og sitater som er oversatt før, virker uten nett. Det hører til vaulten, ikke maskinen, så det følger med manusene.

### 4. Innstillinger

- `Settings`: `translate_quotes: bool` (standard `false`).
- DeepL-nøkkelen er personlig og skal ikke ligge i vaulten (som kan deles eller synkroniseres). Lagre den i `app.json` (`AppConfig`), eller i nøkkelringen til systemet med craten `keyring`.
- `ui/src/settings.ts` får:
  - en bryter for å oversette sitater
  - et felt for DeepL-nøkkelen (type `password`)
  - en knapp for å tømme mellomlageret
  - helst en knapp som viser hvor mye av kvoten som er brukt (`GET /v2/usage`), som en kommando i Rust
- CLI-en: `kerygma lag-presentasjon --oversett-sitater`, med nøkkelen fra `DEEPL_API_KEY`.

### 5. Dokumentasjon

Legg til et avsnitt i `docs/Brukerveiledning.md` om sitatoversettelse: hvordan man får nøkkel, hva som sendes til DeepL, og hvordan mellomlageret virker.

## Tester

Logikken i `extract.rs` testes uten nett:

- Et sitat med oversettelse i `quote_translations` blir to kolonner.
- Et langt sitat med oversettelse deles i like mange slides i begge kolonnene.
- Et sitat uten oversettelse blir som i dag, én kolonne.
- `quotes_to_translate` gir bare sitater som ikke er bibeltekst. Callout-titler er allerede fjernet.
- HTML i sitatet (`<`, `&`) blir escapet i begge kolonnene.
- Talervisningen markerer et oversatt sitat (testen `alle_slides_har_merkene_sine_i_html_en` med en falsk oversettelse).

`translate.rs` testes med en falsk `Translator` og en lokal HTTP-tjener (eller ved å teste forespørselen og svaret hver for seg):

- Det blir én forespørsel for flere sitater.
- Sitater som ligger i mellomlageret, sendes ikke.
- `403` og `456` gir forståelige feilmeldinger.
- Sitater på engelsk hoppes over.

Ende til ende: Kjør `kerygma lag-presentasjon --oversett-sitater` med en ekte nøkkel mot et testmanus, og sjekk resultatet i appen.

## Åpne spørsmål

- Skal oversatte sitater merkes, for eksempel med «(maskinoversatt)» eller en egen CSS-klasse, så det synes at oversettelsen ikke er gjennomgått?
- Britisk (`EN-GB`) eller amerikansk (`EN-US`) engelsk? KJV, som brukes til bibelversene, er britisk.
- Skal man kunne overstyre oversettelsen i manuset, for eksempel med en linje `> EN: ...` under sitatet?
