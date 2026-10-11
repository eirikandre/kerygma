# Kerygma – brukerveiledning

Kerygma er et program for å skrive manus og vise presentasjoner av dem. En presentasjon viser bare overskriftene, sitatene, bibelversene og bildene fra manuset. Hvert bilde får en egen slide, og bibelversene får automatisk engelsk oversettelse ved siden av.

Kerygma kan også:

- sette inn bibelvers mens du skriver
- importere gamle manus fra Word (`.docx`)

Manusene er vanlige markdown-filer i en mappe. Den samme mappa kan også åpnes i Obsidian.

## 1. Installere

**Mac:** åpne `Kerygma.dmg` og dra **Kerygma** til **Programmer**. Første gang starter du programmet ved å høyreklikke på det og velge **Åpne**.

**Windows:** kjør `Kerygma.msi` og følg veiviseren.

Bibeloversettelsene (Norsk Bibel 88/07, Bibel 2024 og King James) følger med programmet.

## 2. Åpne en mappe

Første gang du starter Kerygma, trykker du **Åpne mappe …** og velger mappa der manusene dine ligger (eller skal ligge). Har du en vault fra Obsidian, velger du den. Neste gang åpnes den samme mappa av seg selv. Vil du bytte mappe senere, trykker du på mappeikonet nede i sidefeltet.

Til venstre ser du mappene og notatene. Klikk på et notat for å åpne det. Klikk på en mappe for å folde den ut eller inn.

Høyreklikk for flere valg:

| På | Valg |
|---|---|
| en mappe | Ny note, Ny mappe, Importer Word-manus hit, Gi nytt navn, Slett |
| et notat | Gi nytt navn, Slett, Lag presentasjon |

Det du sletter, havner i papirkurven.

**Søk:** feltet øverst i sidefeltet (eller **Cmd/Ctrl + Shift + F**) søker i navnet og teksten i alle notatene. Skriver du flere ord, må alle finnes i notatet. Notater med treff i navnet kommer først, og under hvert notat står linjene som traff. Trykk på en linje for å åpne notatet der. **Esc** i søkefeltet viser mappene igjen.

**Cmd/Ctrl + N** lager en ny note i mappa du står i. Skriv navnet med en gang, og trykk **Enter**. Du kan alltid gi notatet nytt navn ved å klikke på tittelen over teksten.

## 3. Skrive et manus

Et manus er et vanlig notat. Skriv som du pleier. Det du skriver, lagres av seg selv. Du trenger ikke å trykke på noe.

Presentasjonen henter ut tre ting fra manuset:

- **Overskrifter** (`#`, `##`, `###` …). Hver blir en egen slide.
- **Bibelvers**: teksten etterfulgt av referansen i parentes. Hvert vers blir en egen slide.

  ```
  > Herren er min hyrde, jeg mangler ingen ting. (Salmenes bok 23:1)
  ```

  Vanlige forkortelser virker også, som `(Sal 23,1)`, `(Joh 3,16)` og `(1. Kor 13,4-7)`.
- **Sitater**: linjer som starter med `>`. Hvert sitat blir en egen slide.

Resten av teksten, altså det du sier, blir ikke med i presentasjonen. Det blir heller ikke tekst mellom `%%` og `%%`, som du kan bruke til notater til deg selv.

Fet (`**tekst**`), kursiv (`*tekst*`), understreket (`<u>tekst</u>`), overstreket (`~~tekst~~`) og utheving (`==tekst==`) blir med i presentasjonen, også i bibelvers. Bilder setter du inn med `![[bilde.png]]`.

Ikonene øverst formaterer teksten. Hold musepekeren over et ikon for å se hva det gjør. Marker teksten (eller sett markøren i et ord), og trykk **B** for fet, **I** for kursiv, **U** for understreket eller **S** for overstreket. Trykk én gang til for å fjerne formateringen. Knappen med anførselstegn gjør linjene du har markert, til et sitat (den setter `>` foran), og fjerner det igjen hvis de allerede er et sitat. Markerer du flere avsnitt, blir de ett sitat. Lista til venstre for knappene gjør linja markøren står på, til en overskrift eller til vanlig tekst.

**Cmd/Ctrl + E**, eller øyet øverst, bytter mellom å skrive og å lese teksten ferdig formatert.

## 4. Sette inn bibelvers

1. Sett markøren der verset skal stå.
2. Trykk **Cmd/Ctrl + Shift + B**, eller boka øverst (*Sett inn bibelvers*).
3. Skriv en referanse, eller ord fra verset. Verset vises mens du skriver.
4. Trykk **Enter**.

| Du skriver | Du får |
|---|---|
| `joh3:16` | Johannes 3:16 |
| `sal23` | hele Salme 23 |
| `1kor13:4-7` | vers 4 til 7 |
| `ne:joh3:16` | norsk og engelsk under hverandre |
| `kjv:john3:16` | bare engelsk |
| `tro håp` | søk; velg et av treffene med piltastene |

Er det du skriver ikke en referanse, søkes det etter versene som har alle ordene. Du trenger ikke skrive `s=` foran, men det virker også. Finner ikke søket noe, står det hvorfor, for eksempel at boka ikke finnes.

Verset settes inn som et sitat med referansen bak: `> tekst (referanse)`.

## 5. Lage presentasjon

1. Åpne manuset.
2. Trykk **Cmd/Ctrl + Shift + M**, eller tavla øverst (*Lag presentasjon*).

Det lages et nytt notat ved siden av manuset, med samme navn og ` - presentasjon` bak. Manuset blir ikke endret. Presentasjonen åpnes med en gang. Den vises ikke i lista til venstre (bare hvis manuset er borte); vil du til den igjen, lager du den på nytt fra manuset.

- Hvert bibelvers får engelsk (King James) ved siden av, i to kolonner.
- Lange vers deles over flere slides.
- Har du endret manuset, gjør du det samme én gang til. Den gamle presentasjonen erstattes uten at Kerygma spør. Du kan også gjøre det fra presentasjonen selv. Finnes det et notat med samme navn som Kerygma ikke har laget, spør Kerygma før det overskrives.

Øverst i presentasjonen står en linje som `<!-- kerygma-presentasjon: Påske.md -->`. Den vises ikke i lesevisningen eller presentasjonen, men forteller Kerygma at notatet er en presentasjon, og hvilket manus den er laget fra. Ikke slett den. Uten den er notatet et vanlig notat.

Redigerer du presentasjonen, vises et gult varsel om at endringene blir overskrevet neste gang presentasjonen lages. Vil du endre noe varig, endrer du manuset. Knappen **Åpne manuset** i varselet tar deg dit.

## 6. Vise presentasjonen

1. Lag presentasjonen fra manuset (se over). Den åpnes.
2. Trykk **Cmd/Ctrl + Shift + P**, eller ▷ øverst (*Vis presentasjon*). ▷ står rett etter knappen som lager presentasjonen.

Presentasjonen fyller hele skjermen. Bla med piltastene, mellomrom eller en fjernkontroll for presentasjoner. **Esc** avslutter.

▷ vises bare i presentasjoner, ikke i manuset. Slik viser du alltid en presentasjon som er laget fra manuset slik det er nå.

## 7. Importere manus fra Word

1. Trykk på dokumentet med pil opp nede i sidefeltet, eller velg **Fil → Importer Word-manus …**. Vil du ha manusene i en bestemt mappe, høyreklikker du på mappa og velger **Importer Word-manus hit …**.
2. Velg én eller flere `.docx`-filer.

Hvert manus blir et notat. Bibelversene i manuset blir gjenkjent og gjort om til `> tekst (referanse)`, så de blir med i presentasjonen og får engelsk ved siden av.

Til slutt viser et vindu hvor mange bibelvers som ble funnet i hvert manus. Trykk på et navn for å åpne notatet.

**Sjekk versene med advarsel.** Gjenkjenningen er ikke alltid sikker. Vers den er usikker på, står i en gul boks som sier hvorfor:

```
> [!warning|sjekk] Funnet med søk i Bibelen. Manuset har ingen referanse.
> Du skal kalles med et nytt navn, som Herrens munn skal nevne. (Profeten Jesaja 62:2)
```

1. Finn boksene ved å søke etter `[!warning|sjekk]` i notatet med **Cmd/Ctrl + F**.
2. Les verset og referansen. Rett det som er feil. Er det ikke et bibelvers, fjerner du `>` foran linjene og referansen bak.
3. Slett den første linja, den som starter med `> [!warning|sjekk]`. Da forsvinner boksen.

Advarslene blir ikke med i presentasjonen. Du kan derfor lage presentasjon før du har sjekket alt.

## 8. Innstillinger

Trykk på tannhjulet nede i sidefeltet, eller **Cmd/Ctrl + ,**.

Under **Utseende** velger du tema (lyst, mørkt eller følg systemet, som er standard), og skrift og skriftstørrelse i editoren og lesevisningen. De gjelder alle mapper. Du kan også endre størrelsen ved å holde inne **Cmd/Ctrl** og rulle med musa over teksten (eller knipe på styreflaten).

Under **Presentasjoner og bibel** er innstillingene for mappa som er åpen:

| Innstilling | |
|---|---|
| Postfix for presentasjon | det som står bak navnet på presentasjonen. Standard: ` - presentasjon` |
| Maks tegn per slide | lengre vers deles over flere slides. Standard: 350 |
| Legg til oversettelse | slå av hvis du ikke vil ha engelsk ved siden av versene |
| Oversettelse | oversettelsen ved siden av: `kjv`, `nb88` eller `nb24` |
| Standardoversettelse | oversettelsen du får når du setter inn vers uten prefiks |
| Snarveier | korte navn som `ne` for flere oversettelser |
| Mappe for importerte Word-manus | tom: mappa til notatet som er åpent. «Importer Word-manus hit» i treet går foran |

Under **Utseende på presentasjonen** velger du tema for mappa som er åpen. Trykk på et av de fire bildene:

| Tema | |
|---|---|
| Kveld | mørk blå med gylne overskrifter (standard) |
| Papir | lys og varm, passer i lyse rom |
| Kontrast | hvit tekst på svart, lettest å lese på svake prosjektorer og i strømming |
| Skog | dyp grønn med lysegrønne overskrifter |

Hvert tema har sine egne skrifter. Vil du ha en annen skrift, velger du den under **Skrift i overskrifter** og **Skrift i sitater** (sitater og bibelvers). De to kan velges hver for seg. Lora og Montserrat er bygget inn og ser like ut på alle maskiner; de andre må finnes på maskinen som viser presentasjonen. Obsidian viser alltid temaet Kveld.

## Hurtigtaster

| Tast | Gjør |
|---|---|
| Cmd/Ctrl + N | ny note |
| Cmd/Ctrl + E | bytt mellom å skrive og å lese |
| Cmd/Ctrl + Shift + M | lag presentasjon |
| Cmd/Ctrl + Shift + P | vis presentasjon |
| Cmd/Ctrl + Shift + B | sett inn bibelvers |
| Cmd/Ctrl + , | innstillinger |
| Cmd/Ctrl + B | fet |
| Cmd/Ctrl + I | kursiv |
| Cmd/Ctrl + U | understreket |
| Cmd/Ctrl + Shift + X | overstreket |
| Cmd/Ctrl + Shift + . | sitat |
| Cmd/Ctrl + rullehjul | større eller mindre skrift |
| Cmd/Ctrl + F | søk i notatet |
| Cmd/Ctrl + Shift + F | søk i alle notatene |

## Hvis noe ikke virker

| Problem | Løsning |
|---|---|
| «Kunne ikke laste biblene» | installer Kerygma på nytt; oversettelsene ligger inne i programmet |
| Engelsk mangler ved siden av versene | sjekk at *Legg til oversettelse* er på, og lag presentasjonen på nytt |
| Et bilde vises ikke | sjekk at bildet ligger i mappa du har åpnet, og at navnet i `![[…]]` er riktig |
| Noe ser feil ut i presentasjonen | lag presentasjonen på nytt med Cmd/Ctrl + Shift + M |
