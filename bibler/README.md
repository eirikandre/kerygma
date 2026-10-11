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

Andre oversettelser kan legges her (laget med `bibel-gen` i bible-repoet) og blir med når
appen bygges: de kan velges i innstillingene, boknavnene deres gjenkjennes, og
Word-importen sammenligner med dem også. `.gitignore` stopper alle andre `.bin`-filer enn
de to over, siden oversettelser som ikke er i det fri, ikke skal i git, og en app bygget
med dem kan ikke distribueres uten avtale med rettighetshaveren.
