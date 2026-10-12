# Oversettelsene

Kerygma slår opp og søker i oversettelsene i denne mappa (`*.bin`, formatet i
`crates/bibel-format`). Alle `.bin`-filene som ligger her når appen bygges, blir med:
de pakkes med appen (`src-tauri/tauri.conf.json`), og boknavnene deres blir gjenkjent
i manus (`crates/kerygma-core/build.rs`).

## I git

Bare oversettelser som er i det fri, ligger i git:

- **`nb30.bin`**: Bibelen 1930, norsk. Standardoversettelsen i appen og den norske
  teksten testene sammenligner med. Word-importen sammenligner med den.
- **`kjv.bin`**: King James Version, engelsk, som legges ved siden av versene. Den er i
  det fri, unntatt i Storbritannia, der Crown har rettighetene (letters patent). Teksten
  er hentet fra [thiagobodruk/bible](https://github.com/thiagobodruk/bible)
  (`en_kjv.json`) og pakket med `bibel-gen` i bible-repoet. Dataene derfra er
  MIT-lisensiert; lisensen ligger i `ui/public/lisenser/MIT-thiagobodruk-bible.txt` og
  følger med appen.

## Andre oversettelser

Andre oversettelser (laget med `bibel-gen` i bible-repoet) trenger ikke å være med når appen
bygges. Etter at appen er installert, legges de i mappa for oversettelser
(**Innstillinger → Bibeloversettelser → Vis mappa**):

- macOS: `~/Library/Application Support/no.eidsa.kerygma/bibler`
- Windows: `%APPDATA%\no.eidsa.kerygma\bibler`

Trykk **Last inn på nytt** (eller start appen på nytt). Da kan de velges i innstillingene,
boknavnene deres gjenkjennes i manus (de legges til når oversettelsen lastes), og
Word-importen sammenligner med dem også. Har mappa en oversettelse med samme id som appen
kom med, brukes den fra mappa. Mappa beholdes når appen oppdateres.

Oversettelser som legges her i `bibler/`, blir med i appen når den bygges. `.gitignore`
stopper alle andre `.bin`-filer enn de to over, siden oversettelser som ikke er i det fri,
ikke skal i git, og en app bygget med dem kan ikke distribueres uten avtale med
rettighetshaveren.
