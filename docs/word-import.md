# Word-import

Hvordan gamle manus i Word (`.docx`) blir markdown, og hvordan bibelsitatene i dem blir funnet. For utviklere; brukerne finner det de trenger i [Brukerveiledning.md](Brukerveiledning.md#7-importere-manus-fra-word).

Bibelsitatene blir `> tekst (referanse)`. Da blir de slides, og «Lag presentasjon» setter engelsk ved siden av.

## Bruk

- I appen: **Fil → Importer Word-manus …**, eller **Importer Word-manus hit …** på en mappe i treet (`Workspace::import_targets` og `Workspace::import_word`).
- Fra terminalen:

  ```sh
  kerygma importer "manus.docx"            # lager manus.md ved siden av
  kerygma importer mappe/ --ut vault/Manus # alle .docx i mappa
  kerygma importer manus.docx --uten-søk   # ikke let etter vers i avsnitt uten referanse
  ```

  Bruker oversettelsene i `bibler/` (eller `--bibler <mappe>` / `KERYGMA_BIBLER`). For hver fil skrives antall sitater, hvor mange som har advarsel, og referansene som ikke ble til sitater.

Teksten sammenlignes med oversettelsene fra `word::import_translations`: standardoversettelsen (Bibelen 1930, `nb30`) først, siden den gir navnet i referansen, og så de andre oversettelsene appen er bygget med, unntatt den som legges ved siden av versene (`kjv`).

## Hva som gjøres

- Fet og kursiv blir `**` og `*`.
- Word-stilene *Overskrift 1*, *Overskrift 2* osv. (og *Heading 1* …) blir `##`, `###` osv. Korte avsnitt som er helt i fet skrift, eller som ser ut som «Daniels bok, kapittel 1, del 2», blir også `##`.
- Listepunkter blir `-`.
- Avsnitt som er delt midt i en setning, slås sammen.

### Bibelsitater

Gamle manus skriver referansen på mange måter, og sjelden i parentes etter verset. `convert.rs` finner referansene, slår opp versene med `bibel` (`BibleText`) og leter etter bibelteksten i manuset. Manuset kan bruke en annen oversettelse (som N78), siden ordene sammenlignes ett for ett og små forskjeller tåles (`align.rs`: Smith-Waterman på ord, der ord med én bokstavs forskjell regnes som like).

| I manuset | Blir |
|---|---|
| `Jer 7,4` på egen linje, verset på neste | `> Sett ikke eders lit ... (Jeremias bok 7:4)` |
| `1 Kor 10,13 Eder har ingen fristelse møtt ...` | `> Eder har ingen fristelse møtt ... (Første korinterbrev 10:13)` |
| `I Hos 4:6 sier Herren,` og verset under | teksten beholdes, så kommer sitatet |
| `V1`, `v.8`, `vers 7` | boka og kapitlet fra forrige referanse, `Daniel kap 1` eller overskriften |
| `1:4 unge gutter ...` | boka fra forrige referanse eller overskriften |
| `... rettferdiggjørelse” i Rom 4:25` | sitatet foran referansen skilles ut |
| et avsnitt som er et helt vers, uten referanse | finnes med søk i Bibelen |

- Referansen blir slik `bibel` skriver den («Daniels bok 1:9»), og viser versene som faktisk er sitert. `reference.rs` kjenner alle disse navnene, så presentasjonen kan slå dem opp på engelsk.
- Står referansen alene foran sitatet, flyttes den inn i sitatet. Ellers blir den stående i teksten også.
- `(N78)` og `(N88)` etter et sitat fjernes, siden de ikke er en oversettelse `bibel` kjenner.
- Referanser der manuset ikke siterer verset, som `Matt 13:44-46` midt i en setning, blir stående som tekst og kommer i rapporten (`ConvertResult::unmatched`).

### Advarsler

Sitatene som er mest usikre, får en gul callout som sier hvorfor (`FoundQuote::warning`):

```
> [!warning|sjekk] Få ord stemmer med bibelteksten. Sjekk at dette er Dan 1:9.
> Gud gjorde at den øverste hoffmann fikk velvilje og godhet for Daniel. (Daniels bok 1:9)
```

| Advarsel | Når |
|---|---|
| Funnet med søk i Bibelen | manuset har ingen referanse; kan være en gjenfortelling |
| Få ord stemmer | referansen står alene foran avsnittet, men teksten ligner lite |
| Skilt ut fra setningen foran | sitatet står foran referansen, som i `... rettferdiggjørelse” i Rom 4:25` |
| Manuset viser til … | sitatet starter på et annet vers enn referansen sier |

`extract.rs` tar ikke med `[!warning|sjekk]`-linja på sliden, bare verset, så presentasjonen kan lages før advarslene er sjekket.

## Filene

| Fil | |
|---|---|
| `crates/kerygma-core/src/word/docx.rs` | leser avsnitt, stil, fet og kursiv og listepunkter fra `word/document.xml` (zip + quick-xml) |
| `crates/kerygma-core/src/word/convert.rs` | finner referanser og sitater, og lager markdown og rapporten |
| `crates/kerygma-core/src/word/align.rs` | sammenligner manustekst med bibeltekst |
| `crates/kerygma-core/src/word/bible_text.rs` | traitet `BibleText`, og `BibelText` med oppslag og søk i `bibel` |
| `crates/kerygma-core/src/word/mod.rs` | `import_docx` og navnene på manusene |
| `crates/kerygma-core/tests/convert.rs` | testene, med en liten falsk bibel, så de ikke trenger oversettelsene |
