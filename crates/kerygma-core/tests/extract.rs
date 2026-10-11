//! «Lag presentasjon» (`extract_slide_content`): hele utdataene sammenlignes tegn for
//! tegn, også linjeskift og `---` mellom slides. Testene for referanser ligger i
//! `tests/reference.rs`.

use std::collections::HashMap;

use kerygma_core::extract::{
    balance_emphasis, emphasis_to_html, extract_slide_content, lone_verse_references, mark_verse_numbers, split_text,
    SlideOptions,
};

fn extract(markdown: &str) -> String {
    extract_slide_content(markdown, &SlideOptions::default())
}

fn with_max(markdown: &str, max_chars: usize) -> String {
    extract_slide_content(markdown, &SlideOptions { max_chars: Some(max_chars), ..Default::default() })
}

fn with_translations(markdown: &str, translations: &[(&str, &str)]) -> String {
    let translations: HashMap<String, String> =
        translations.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    extract_slide_content(markdown, &SlideOptions { translations, ..Default::default() })
}

fn slides(output: &str) -> Vec<String> {
    output.trim().split("\n\n---\n\n").map(String::from).collect()
}

fn strings(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

#[test]
fn beholder_kun_overskrifter_og_bibelvers() {
    let input = "---
tags: preken
---
# Innledning

Dette er innledende tekst som skal bort.

## Guds kjærlighet

Vi leser fra Johannes. «For så har Gud elsket verden at han ga sin Sønn, den enbårne» (Joh 3,16). Dette betyr mye.

Kjærlighet er tålmodig, kjærlighet er god (1 Kor 13,4)

- Et poeng uten vers
- Jesus sa: “Jeg er veien, sannheten og livet” (Joh 14,6)

> Herren er min hyrde, jeg mangler ingen ting.
> (Sal 23,1)

```
kode (Joh 1,1)
```

%% Notat (Rom 1,1) %%

### Avslutning

Takk for oppmerksomheten (se figur 3).
";

    assert_eq!(
        extract(input),
        "---
tags: preken
---

# Innledning

---

## Guds kjærlighet

---

Vi leser fra Johannes. «For så har Gud elsket verden at han ga sin Sønn, den enbårne» (Joh 3,16)

---

Kjærlighet er tålmodig, kjærlighet er god (1 Kor 13,4)

---

Jesus sa: “Jeg er veien, sannheten og livet” (Joh 14,6)

---

Herren er min hyrde, jeg mangler ingen ting. (Sal 23,1)

---

### Avslutning
"
    );
}

#[test]
fn flere_vers_i_samme_avsnitt_blir_separate_slides() {
    assert_eq!(
        extract("«Vers en» (Rom 3,23) og så «vers to» (Rom 6,23)."),
        "«Vers en» (Rom 3,23)\n\n---\n\nog så «vers to» (Rom 6,23)\n"
    );
}

#[test]
fn vers_uten_anforselstegn_med_fulle_boknavn() {
    let input = "
Dette er tekst so ikke skal med i slide

Ved dette vet vi at vi har lært ham å kjenne, om vi holder fast på hans bud. (1. Johannes´ brev 2:3) 

Dette er videre test som ikke skal med

Men Gud vet at den dagen dere eter av det, vil øynene deres åpnes, dere vil bli slik som Gud til å kjenne godt og ondt. (1. Mosebok 3:5) 

OG dette skal ikke med igjen

# Dette er en deloverskrift

og dette skal ikke med
";

    assert_eq!(
        extract(input),
        "Ved dette vet vi at vi har lært ham å kjenne, om vi holder fast på hans bud. (1. Johannes´ brev 2:3)

---

Men Gud vet at den dagen dere eter av det, vil øynene deres åpnes, dere vil bli slik som Gud til å kjenne godt og ondt. (1. Mosebok 3:5)

---

# Dette er en deloverskrift
"
    );
}

#[test]
fn filnavnet_blir_ikke_egen_slide_og_frontmatter_beholdes() {
    assert_eq!(extract("---\ntags: preken\n---\nTekst\n\n# Del 1"), "---\ntags: preken\n---\n\n# Del 1\n");
}

#[test]
fn tittelen_i_warning_sjekk_blir_ikke_med_pa_sliden_men_verset_gjor() {
    let input = "> [!warning|sjekk] Funnet med søk i Bibelen. Manuset har ingen referanse.
> Du skal kalles med et nytt navn. (Profeten Jesaja 62:2)";
    assert_eq!(lone_verse_references(input), strings(&["Profeten Jesaja 62:2"]));
    assert_eq!(extract(input), "Du skal kalles med et nytt navn. (Profeten Jesaja 62:2)\n");
}

#[test]
fn fet_og_kursiv_blir_html_i_kolonnene_siden_markdown_ikke_virker_inne_i_html() {
    let input =
        "> Dette er arven til Herrens tjenere, **den rett de får av meg**, sier *Herren* & ==han==. (Jes 54:17)";
    assert_eq!(
        with_translations(input, &[("Jes 54:17", "This is the heritage. (Isaiah 54:17)")]),
        r#"<div class="bibel-parallell">
<p>Dette er arven til Herrens tjenere, <strong>den rett de får av meg</strong>, sier <em>Herren</em> &amp; <mark>han</mark>. (Jes 54:17)</p>
<p>This is the heritage. (Isaiah 54:17)</p>
</div>
"#
    );
}

#[test]
fn fet_tekst_som_deles_over_flere_slides_lukkes_og_apnes_igjen() {
    assert_eq!(
        balance_emphasis(&strings(&["a **b c", "d e", "f** g *h", "i*"])),
        strings(&["a **b c**", "**d e**", "**f** g *h*", "*i*"])
    );
    assert_eq!(
        emphasis_to_html("***a*** **b** *c* __d__ e*f"),
        "<strong><em>a</em></strong> <strong>b</strong> <em>c</em> <strong>d</strong> e*f"
    );
}

#[test]
fn vers_fra_bibel_beholdes_helt_ogsa_med_dialog_og_versnumre() {
    let verses = [
        "Jesus svarte: «Sannelig, sannelig, jeg sier deg: Den som ikke blir født på ny, kan ikke se Guds rike.» (Johannes 3:3)",
        "<sup>1:3</sup> c <sup>2:1</sup> d <sup>2:2</sup> e (1. Samuelsbok 1:3-2:2)",
        "<sup>16</sup> For så har Gud elsket verden ... <sup>17</sup> For Gud sendte ikke sin Sønn ... (Johannes´ evangelium 3:16-17)",
        "Jeg er veien og sannheten og livet. (Profeten Jesaja 53:5)",
    ];
    let input: Vec<String> = verses.iter().map(|verse| verse.replace("<sup>", "").replace("</sup>", "")).collect();
    assert_eq!(extract(&input.join("\n\n")), verses.join("\n\n---\n\n") + "\n");
}

#[test]
fn norsk_og_engelsk_settes_side_om_side() {
    let input = "Ikke med

For så har Gud elsket verden (Johannes´ evangelium 3:16)
For God so loved the world (John 3:16)

Men nå blir de stående disse tre (1. Korinterbrev 13:13)
Og Gud sa: Bli lys! (1. Mosebok 1:3)";

    assert_eq!(
        extract(input),
        r#"<div class="bibel-parallell">
<p>For så har Gud elsket verden (Johannes´ evangelium 3:16)</p>
<p>For God so loved the world (John 3:16)</p>
</div>

---

Men nå blir de stående disse tre (1. Korinterbrev 13:13)

---

Og Gud sa: Bli lys! (1. Mosebok 1:3)
"#
    );
}

#[test]
fn lang_bibeltekst_deles_over_flere_slides_med_referansen_pa_hver() {
    let verse = "1 Første setning er her. 2 Andre setning følger. 3 Tredje setning kommer. 4 Fjerde og siste.";
    assert_eq!(
        slides(&with_max(&format!("{verse} (Salmenes bok 1:1-4)"), 50)),
        strings(&[
            "<sup>1</sup> Første setning er her. <sup>2</sup> Andre setning følger. (Salmenes bok 1:1-4)",
            "<sup>3</sup> Tredje setning kommer. <sup>4</sup> Fjerde og siste. (Salmenes bok 1:1-4)",
        ])
    );
}

#[test]
fn lange_parallelle_vers_deles_likt_i_begge_kolonner() {
    let input = [
        "1 Første norske setning. 2 Andre norske setning. 3 Tredje norske setning. (Salmenes bok 1:1-3)",
        "1 First sentence. 2 Second sentence. 3 Third. (Psalms 1:1-3)",
    ]
    .join("\n");
    let slides = slides(&with_max(&input, 80));
    assert_eq!(slides.len(), 2);
    let pattern = regex::Regex::new(
        r#"^<div class="bibel-parallell">\n<p>.+\(Salmenes bok 1:1-3\)</p>\n<p>.+\(Psalms 1:1-3\)</p>\n</div>$"#,
    )
    .unwrap();
    for slide in &slides {
        assert!(pattern.is_match(slide), "{slide}");
    }
}

#[test]
fn split_text_deler_ikke_rett_etter_et_versnummer() {
    let ends_with_number = regex::Regex::new(r"\s\d+$").unwrap();
    for part in split_text("16 ord ord ord ord 17 ord ord ord ord ord", 2) {
        assert!(!ends_with_number.is_match(&part), "{part}");
    }
    assert_eq!(split_text("Kort.", 1), strings(&["Kort."]));
}

#[test]
fn parallelle_kolonner_deles_ved_samme_vers() {
    let input = [
        "1 Kort. 2 En mye lengre norsk setning som tar plass. 3 Mer norsk tekst her. 4 Slutt. (Salmenes bok 1:1-4)",
        "1 A much longer English opening sentence here. 2 Short. 3 More. 4 The end of it. (Psalms 1:1-4)",
    ]
    .join("\n");
    let slides = slides(&with_max(&input, 80));
    assert_eq!(slides.len(), 2);
    let first_number = regex::Regex::new(r"<p><sup>(\d+)</sup>").unwrap();
    for slide in &slides {
        let numbers: Vec<&str> = first_number.captures_iter(slide).map(|c| c.get(1).unwrap().as_str()).collect();
        assert_eq!(numbers.len(), 2, "{slide}");
        assert_eq!(numbers[0], numbers[1], "{slide}");
    }
}

#[test]
fn vers_som_star_alene_far_oversettelsen_ved_siden_av() {
    let input = "Prat

Han ble såret for våre overtredelser (Profeten Jesaja 53:5)

For så har Gud elsket verden (Johannes´ evangelium 3:16)
For God so loved the world (John 3:16)";

    assert_eq!(lone_verse_references(input), strings(&["Profeten Jesaja 53:5"]));
    assert_eq!(
        with_translations(
            input,
            &[("Profeten Jesaja 53:5", "But he was wounded for our transgressions (Isaiah 53:5)")]
        ),
        r#"<div class="bibel-parallell">
<p>Han ble såret for våre overtredelser (Profeten Jesaja 53:5)</p>
<p>But he was wounded for our transgressions (Isaiah 53:5)</p>
</div>

---

<div class="bibel-parallell">
<p>For så har Gud elsket verden (Johannes´ evangelium 3:16)</p>
<p>For God so loved the world (John 3:16)</p>
</div>
"#
    );
}

#[test]
fn bare_versnumre_i_rekkefolge_merkes_ikke_andre_tall() {
    let sup = |text: &str| mark_verse_numbers(text).replace('\u{E000}', "[").replace('\u{E001}', "]");
    assert_eq!(sup("1 Han valgte 12 apostler. 2 De gikk ut."), "[1] Han valgte 12 apostler. [2] De gikk ut.");
    assert_eq!(sup("14 a 15-16 b 17 c"), "[14] a [15-16] b [17] c");
    assert_eq!(sup("1:27 a 1:28 b 2:1 c 2 d"), "[1:27] a [1:28] b [2:1] c [2] d");
    assert_eq!(sup("Han ga 5 brød"), "Han ga 5 brød");
}

#[test]
fn alle_sitater_blir_slides_uten_gt_bibelsitater_far_oversettelse() {
    let input = "Prat

> Den som vil vinne, må tåle å tape.
> – Ukjent

> Herren er min hyrde, jeg mangler ingen ting.
> (Salmenes bok 23:1)

> [!note] Husk
> Les sakte (Joh 3,16)

> Kort sitat uten referanse";

    assert_eq!(lone_verse_references(input), strings(&["Salmenes bok 23:1", "Joh 3,16"]));
    assert_eq!(
        with_translations(
            input,
            &[("Salmenes bok 23:1", "The LORD is my shepherd; I shall not want. (Psalms 23:1)")]
        ),
        r#"Den som vil vinne, må tåle å tape.
– Ukjent

---

<div class="bibel-parallell">
<p>Herren er min hyrde, jeg mangler ingen ting. (Salmenes bok 23:1)</p>
<p>The LORD is my shepherd; I shall not want. (Psalms 23:1)</p>
</div>

---

Husk
Les sakte (Joh 3,16)

---

Kort sitat uten referanse
"#
    );
}

#[test]
fn lange_sitater_deles_over_flere_slides_med_full_bredde() {
    let quote = [
        "> Første setning i et langt sitat. Andre setning følger etter.",
        "> Tredje setning kommer her. Fjerde og siste setning.",
        "> – Ukjent",
    ]
    .join("\n");
    assert_eq!(
        slides(&with_max(&quote, 80)),
        strings(&[
            "Første setning i et langt sitat. Andre setning følger etter.",
            "Tredje setning kommer her. Fjerde og siste setning.\n– Ukjent",
        ])
    );
    // Samme lengde som bibeltekst med oversettelse ville gitt flere slides.
    assert!(!with_max(&quote, 140).contains("---"));
}

#[test]
fn hvert_bilde_blir_en_egen_slide_i_rekkefolge_med_teksten_rundt() {
    let input = "# Del 1

Prat ![[kart.png|400]] mer prat

![[Annet notat]]

- Punkt med ![Jerusalem](bilder/jerusalem.jpg) og vers. Gud er kjærlighet (1Joh 4,8)

> ![[korset.webp]]
> Et sitat";

    assert_eq!(
        extract(input),
        "# Del 1

---

![[kart.png|400]]

---

![Jerusalem](bilder/jerusalem.jpg)

---

og vers. Gud er kjærlighet (1Joh 4,8)

---

![[korset.webp]]

---

Et sitat
"
    );
}
