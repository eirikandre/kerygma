# Kerygma – brukerveiledning

Kerygma er et program for å skrive manus og vise presentasjoner av dem. En presentasjon viser bare overskriftene, sitatene, bibelversene og bildene fra manuset. Hvert bilde får en egen slide, og bibelversene får engelsk oversettelse ved siden av av seg selv.

Kerygma kan også:

- sette inn bibelvers mens du skriver
- vise presentasjonen på to skjermer, med manuset foran deg (talervisning)
- eksportere presentasjonen til PDF, PowerPoint og HTML
- importere gamle manus fra Word (`.docx`)

Manusene er vanlige markdown-filer i en mappe. Den samme mappa kan også åpnes i Obsidian.

Denne veiledningen finner du i programmet under **Hjelp → Brukerveiledning**.

## 1. Installere

**Mac:** åpne `Kerygma.dmg` og dra **Kerygma** til **Programmer**. Første gang starter du programmet ved å høyreklikke på det og velge **Åpne**.

**Windows:** kjør `Kerygma.msi` og følg veiviseren.

Bibelen 1930 (norsk) og King James Version (engelsk) følger med programmet. Flere oversettelser kan legges til etterpå (se [Innstillinger](#9-innstillinger)).

## 2. Mappa med manus

Første gang du starter Kerygma, trykker du **Åpne mappe …** og velger mappa der manusene dine ligger (eller skal ligge). Har du en vault fra Obsidian, velger du den. Neste gang åpnes den samme mappa av seg selv. Vil du bytte mappe, velger du **Fil → Åpne mappe …**.

Til venstre ser du mappene og manusene. Klikk på et manus for å åpne det. Klikk på en mappe for å folde den ut eller inn. Bilder og andre filer vises grå, med filendelse.

Knappene øverst i sidefeltet lager et **nytt manus** og en **ny mappe** der du står. Skriv navnet med en gang, og trykk **Enter**. Du kan alltid gi et manus nytt navn ved å klikke på tittelen over teksten.

Høyreklikk for flere valg:

| På | Valg |
|---|---|
| en mappe | Nytt manus, Ny mappe, Importer Word-manus hit, Gi nytt navn, Slett |
| et manus | Gi nytt navn, Slett, Lag presentasjon, Eksporter presentasjonen som PDF, PowerPoint eller HTML |

Dra et manus eller en mappe til en annen mappe for å flytte det. Presentasjonen og bildene til manuset blir med. Det du sletter, havner i papirkurven. Sletter du et manus, slettes presentasjonen og bildene til det også.

**Skjule treet:** knappen øverst til høyre i sidefeltet, eller **Cmd/Ctrl + Shift + T**, skjuler treet, så teksten får hele vinduet. Trykk på knappen øverst til venstre (eller samme taster) for å få det fram igjen.

**Søk:** feltet øverst i sidefeltet (eller **Cmd/Ctrl + Shift + F**) søker i navnet og teksten i alle manusene. Skriver du flere ord, må alle finnes i manuset. Manus med treff i navnet kommer først, og under hvert manus står linjene som traff. Trykk på en linje for å åpne manuset der. **Esc** i søkefeltet viser mappene igjen.

## 3. Skrive et manus

Et manus er en vanlig markdown-fil. Skriv som du pleier. Det du skriver, lagres av seg selv. Du trenger ikke å trykke på noe.

Presentasjonen henter ut fire ting fra manuset:

- **Overskrifter** (`#`, `##`, `###` …). Hver blir en egen slide.
- **Bibelvers**: teksten etterfulgt av referansen i parentes. Hvert vers blir en egen slide.

  ```
  > Herren er min hyrde, mig fattes intet. (Salmenes bok 23:1)
  ```

  Vanlige forkortelser virker også, som `(Sal 23,1)`, `(Joh 3,16)` og `(1. Kor 13,4-7)`.
- **Sitater**: linjer som starter med `>`. Hvert sitat blir en egen slide.
- **Bilder**: hvert bilde blir en egen slide (se [Bilder](#4-bilder)).

Resten av teksten, altså det du sier, blir ikke med i presentasjonen. Det blir heller ikke tekst mellom `%%` og `%%`, som du kan bruke til kommentarer til deg selv.

Fet (`**tekst**`), kursiv (`*tekst*`), understreket (`<u>tekst</u>`), overstreket (`~~tekst~~`) og utheving (`==tekst==`) blir med i presentasjonen, også i bibelvers.

**Verktøylinja** over teksten, fra venstre:

| Knapp | Gjør |
|---|---|
| lista *Vanlig tekst* | gjør linja markøren står på, til en overskrift (1–3) eller til vanlig tekst |
| **B**, *I*, U, S | fet, kursiv, understreket og overstreket rundt det du har markert (eller ordet markøren står i). Trykk én gang til for å fjerne det. |
| anførselstegn | gjør linjene du har markert, til et sitat (`>` foran), eller fjerner sitatet. Flere avsnitt blir ett sitat. |
| boka | sett inn bibelvers |
| bildet | sett inn bilde |
| øyet / pennen | bytter mellom å skrive og å lese teksten ferdig formatert (**Cmd/Ctrl + E**) |
| tavla | lag presentasjon |
| ▷ | vis presentasjonen |

Knappene fra *Vanlig tekst* til bildet vises bare når du skriver, ikke i lesevisningen.

Hold musepekeren over en knapp for å se hva den gjør, og hurtigtasten.

Trykker du **Enter** på en tom sitatlinje (bare `>`), avsluttes sitatet.

## 4. Bilder

Trykk på bildet i verktøylinja og velg ett eller flere bilder, eller dra bildene fra Finder (Mac) eller Utforsker (Windows) rett inn i manuset. Bildet settes inn der markøren står (eller der du slipper det), som en egen linje:

```
![[korset.jpg]]
```

- I manuset vises bildet lite under linja, og lite i lesevisningen.
- I presentasjonen blir hvert bilde en egen slide. Bildet fyller sliden med litt luft rundt, enten det er stående eller liggende.
- Bildene kopieres til en mappe med samme navn som manuset, ved siden av det: bildene til `Påske.md` ligger i mappa `Påske`. Mappa vises ikke i treet.
- Gir du manuset nytt navn, eller flytter eller sletter det, skjer det samme med bildemappa.
- Fjerner du et bilde fra manuset, legges bildet i papirkurven når du lukker manuset (åpner et annet eller avslutter), hvis ikke et annet manus bruker det. Angrer du mens manuset er åpent, er bildet der fortsatt.

Bilder som ligger andre steder i mappa, kan du også vise med `![[bilde.png]]`. Kerygma finner dem på navnet, som Obsidian.

## 5. Sette inn bibelvers

1. Sett markøren der verset skal stå.
2. Trykk **Cmd/Ctrl + Shift + B**, eller boka i verktøylinja.
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

## 6. Lage presentasjon

1. Åpne manuset.
2. Trykk **Cmd/Ctrl + Shift + M**, eller tavla i verktøylinja.

Det lages en ny fil ved siden av manuset, med samme navn og ` - presentasjon` bak. Manuset blir ikke endret. Presentasjonen åpnes med en gang. Den vises ikke i treet (bare hvis manuset er borte); vil du til den igjen, lager du den på nytt fra manuset.

- Hvert bibelvers får engelsk (King James) ved siden av, i to kolonner.
- Lange vers og sitater deles over flere slides.
- Har du endret manuset, lager du presentasjonen én gang til. Den gamle erstattes uten at Kerygma spør. Du kan også gjøre det fra presentasjonen selv. Finnes det en fil med samme navn som Kerygma ikke har laget, spør Kerygma før den overskrives.

Presentasjonen vises alltid ferdig formatert, som et mellomsteg før du starter den. Den redigeres ikke: trykker du på pennen (eller **Cmd/Ctrl + E**), åpnes manuset, og der gjør du endringene. Lag så presentasjonen på nytt.

Fila har en linje øverst, `<!-- kerygma-presentasjon: Påske.md -->`, som ikke vises, men forteller Kerygma hvilket manus presentasjonen er laget fra. Ikke slett den. Finnes ikke manuset lenger, vises et gult varsel.

## 7. Vise presentasjonen

1. Lag presentasjonen fra manuset (se over). Den åpnes.
2. Trykk **Cmd/Ctrl + Shift + P**, eller ▷ i verktøylinja.

Presentasjonen fyller hele skjermen. Bla med piltastene, mellomrom eller en fjernkontroll for presentasjoner. **Esc** avslutter.

Med én skjerm vises ▷ bare i presentasjoner, ikke i manuset. Slik viser du alltid en presentasjon som er laget fra manuset slik det er nå.

### Talervisning med to skjermer

Er en skjerm eller prosjektor koblet til, starter ▷ (eller **Cmd/Ctrl + Shift + P**) talervisningen i stedet. Da vises ▷ også i manuset.

- Slidene vises i fullskjerm på den andre skjermen.
- Manuset vises ferdig formatert i vinduet ditt, uten treet. Vinduet tar ikke over hele skjermen.
- Alt som kommer på en slide, har en svak farge i manuset. Det som vises nå, er tydelig markert.
- Rull i manuset mens du taler. Den stiplede linja en tredjedel ned er leselinja. Når neste del av manuset når den, byttes sliden. Tekst som ikke er på noen slide, endrer ingenting: forrige slide blir stående.
- **→** og **PageDown** går til neste slide, og **←** og **PageUp** til forrige. Det virker også med en fjernkontroll, og manuset rulles dit. **↓** og **↑** ruller manuset, også fra slidevinduet, og sliden byttes når neste del når leselinja. Rullehjulet ruller som vanlig.
- **Esc**, × øverst til høyre eller å lukke slidevinduet avslutter.

### Talervisning i eget vindu

Velg **Manus → Talervisning i eget vindu**, trykk **Cmd/Ctrl + Alt + Shift + P**, eller Alt-klikk på ▷. Det virker også med bare én skjerm.

- Slidene vises i et vanlig vindu som heter *Kerygma – presentasjon*. I OBS legger du til en kilde for vindusfangst og velger det vinduet. Tittelen er alltid den samme, så OBS finner vinduet igjen neste gang.
- Første gang er vinduet 1280 × 720 (16:9). Du kan flytte vinduet og endre størrelsen på det, og det åpnes likt neste gang.
- Vinduet viser bare sliden, uten piler eller framdriftslinje.
- Alt annet virker som i talervisningen over: rull i manuset, bruk tastene, og avslutt med **Esc** eller ved å lukke vinduet.

Slidene lages fra manuset når talervisningen startes, med de samme reglene som *Lag presentasjon*, så du trenger ikke lage presentasjonen først. Starter du fra en presentasjon som har mistet manuset sitt, vises presentasjonen på vanlig måte.

### Eksportere til PDF, PowerPoint og HTML

Velg **Manus → Eksporter presentasjonen som PDF …**, **… som PowerPoint …** eller **… som HTML …**, eller høyreklikk på manuset i treet. Det som eksporteres, er presentasjonen, det samme som ▷ viser, i bredformat (16:9) og full HD (1920 × 1080) for PDF og PowerPoint. Presentasjonen må være laget først. Du velger hvor fila skal lagres.

- **PDF:** én slide per side, slik den ser ut i programmet, med temaet og skriftene. Slidene er bredere enn på skjermen, så linjene blir lengre. Teksten kan markeres og søkes i. Eksport til PDF virker foreløpig bare på Mac.
- **PowerPoint:** hver slide får tekstbokser du kan rette i PowerPoint eller Keynote, med farger og skrift fra temaet. Den ligner presentasjonen, men er ikke helt lik. Skriftene Lora og Montserrat er ikke med i fila, så installer dem på maskinen der fila åpnes, ellers brukes en annen skrift.
- **HTML:** én fil som åpnes i en nettleser og vises som ▷ i programmet, med temaet, skriftene og bildene bygget inn. Den virker uten programmet og uten nett, så den kan sendes til andre eller vises fra en annen maskin. Bla med piltastene, og trykk F for fullskjerm.

## 8. Importere manus fra Word

1. Velg **Fil → Importer Word-manus …**. Vil du ha manusene i en bestemt mappe, høyreklikker du på mappa og velger **Importer Word-manus hit …**.
2. Velg én eller flere `.docx`-filer.

Hvert Word-manus blir et manus i Kerygma. Overskrifter, fet og kursiv tas med. Bibelversene i manuset blir gjenkjent og gjort om til `> tekst (referanse)`, så de blir med i presentasjonen og får engelsk ved siden av. Versene sammenlignes med Bibelen 1930, og kan være skrevet etter en annen oversettelse: små forskjeller tåles.

Til slutt viser et vindu hvor mange bibelvers som ble funnet i hvert manus. Trykk på et navn for å åpne manuset.

**Sjekk versene med advarsel.** Gjenkjenningen er ikke alltid sikker. Vers den er usikker på, står i en gul boks som sier hvorfor:

```
> [!warning|sjekk] Funnet med søk i Bibelen. Manuset har ingen referanse.
> Du skal kalles med et nytt navn, som Herrens munn skal nevne. (Jesajas bok 62:2)
```

1. Trykk **Søk etter advarsler** i vinduet som vises etter importen. Da vises manusene med advarsler i sidefeltet, og du trykker på en linje for å gå dit. Du kan også søke etter `[!warning|sjekk]` i manuset med **Cmd/Ctrl + F**.
2. Les verset og referansen. Rett det som er feil. Er det ikke et bibelvers, fjerner du `>` foran linjene og referansen bak.
3. Slett den første linja, den som starter med `> [!warning|sjekk]`. Da forsvinner boksen.

Advarslene blir ikke med i presentasjonen. Du kan derfor lage presentasjon før du har sjekket alt.

## 9. Innstillinger

Velg **Fil → Innstillinger …**, eller trykk **Cmd/Ctrl + ,**.

Under **Utseende** velger du tema (lyst, mørkt eller følg systemet, som er standard), og skrift og skriftstørrelse i editoren og lesevisningen. De gjelder alle mapper. Du kan også endre størrelsen ved å holde inne **Cmd/Ctrl** og rulle med musa over teksten (eller knipe på styreflaten).

Under **Presentasjoner og bibel** er innstillingene for mappa som er åpen:

| Innstilling | |
|---|---|
| Postfix for presentasjon | det som står bak navnet på presentasjonen. Standard: ` - presentasjon` |
| Maks tegn per slide | lengre vers og sitater deles over flere slides. Standard: 350 |
| Standardoversettelse | velg oversettelsen du får når du setter inn vers uten prefiks. Standard: `nb30` |
| Legg til oversettelse | slå av hvis du ikke vil ha engelsk ved siden av versene |
| Engelsk oversettelse | velg oversettelsen ved siden av. Standard: `kjv` |
| Snarveier | korte navn på én eller flere oversettelser: `n` (norsk), `e` (engelsk) og `ne` (begge) |
| Mappe for importerte Word-manus | tom: mappa til manuset som er åpent. «Importer Word-manus hit» i treet går foran |

Under **Bibeloversettelser** står oversettelsene programmet har. Har du flere oversettelser som `.bin`-filer, trykker du **Vis mappa**, legger filene i mappa som vises, og trykker **Last inn på nytt**. Da kan de brukes i innstillingene over, når du setter inn bibelvers og i Word-importen, og referansene deres blir gjenkjent i manus. Mappa beholdes når Kerygma oppdateres.

Under **Utseende på presentasjonen** velger du tema for mappa som er åpen. Trykk på et av de fire bildene:

| Tema | |
|---|---|
| Kveld | mørk blå med gylne overskrifter (standard) |
| Papir | lys og varm, passer i lyse rom |
| Kontrast | hvit tekst på svart, lettest å lese på svake prosjektorer og i strømming |
| Skog | dyp grønn med lysegrønne overskrifter |

Hvert tema har sine egne skrifter. Vil du ha en annen skrift, velger du den under **Skrift i overskrifter** og **Skrift i sitater** (sitater og bibelvers). De to kan velges hver for seg. Lora og Montserrat er bygget inn og ser like ut på alle maskiner; de andre må finnes på maskinen som viser presentasjonen.

## Hurtigtaster

På Windows bruker du **Ctrl** der det står Cmd/Ctrl.

| Tast | Gjør |
|---|---|
| Cmd/Ctrl + N | nytt manus |
| Cmd/Ctrl + E | bytt mellom å skrive og å lese (i en presentasjon: rediger manuset) |
| Cmd/Ctrl + Shift + M | lag presentasjon |
| Cmd/Ctrl + Shift + P | vis presentasjon (talervisning med to skjermer) |
| Cmd/Ctrl + Alt + Shift + P | talervisning i eget vindu |
| Cmd/Ctrl + Shift + B | sett inn bibelvers |
| Cmd/Ctrl + , | innstillinger |
| Cmd/Ctrl + Shift + T | vis eller skjul treet |
| Cmd/Ctrl + B | fet |
| Cmd/Ctrl + I | kursiv |
| Cmd/Ctrl + U | understreket |
| Cmd/Ctrl + Shift + X | overstreket |
| Cmd/Ctrl + Shift + . | sitat |
| Cmd/Ctrl + rullehjul | større eller mindre skrift |
| Cmd/Ctrl + F | søk i manuset |
| Cmd/Ctrl + Shift + F | søk i alle manusene |

## Hvis noe ikke virker

| Problem | Løsning |
|---|---|
| «Kunne ikke laste biblene» | installer Kerygma på nytt; oversettelsene ligger inne i programmet |
| Engelsk mangler ved siden av versene | sjekk at *Legg til oversettelse* er på, og lag presentasjonen på nytt |
| Et bilde vises ikke | sjekk at bildet ligger i mappa du har åpnet, og at navnet i `![[…]]` er riktig |
| Et bilde du fjernet, ligger fortsatt i mappa | det legges i papirkurven når du lukker manuset, hvis ikke et annet manus bruker det |
| Noe ser feil ut i presentasjonen | lag presentasjonen på nytt med Cmd/Ctrl + Shift + M |
| Endringer i presentasjonen er borte | presentasjonen lages fra manuset hver gang; gjør endringene i manuset |
