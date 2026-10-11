//! Gjenkjenning av bibelreferanser: skrevet for hånd, og alle formene `bibel` skriver for
//! oversettelsene i `bibler/` (`common::generated`).

mod common;

use kerygma_core::extract::{extract_slide_content, SlideOptions};
use kerygma_core::reference::{is_bible_reference, to_lookup};

#[test]
fn gjenkjenner_bibelreferanser() {
    for reference in [
        "Joh 3,16",
        "Joh. 3:16",
        "1. Kor 13,4-7",
        "1 Kor 13,4–7",
        "1Joh 4,8",
        "Sal 23",
        "Salme 23,1",
        "Rom 8,28; 12,1-2",
        "Joh 3,16; Rom 5,8",
        "Matt 5,3.5.7",
        "Ef 2,8-9",
        "Åp 21,4",
        "Apg 2,38",
        "Apostlenes gjerninger 2,38",
        "Joh 3,16 NB",
        "Luk 15,11ff",
        "Romerbrevet 8,28",
        "1. Johannes´ brev 2:3",
        "1. Johannes' brev 2:3",
        "1. Mosebok 3:5",
        "Johannes evangelium 3,16",
        "Salmenes bok 23",
        "Første Johannesbrev 2:3",
        "Jesajas bok 53:5",
        "Paulus' brev til romerne 8:28",
        "Isaiah 53:5",
    ] {
        assert!(is_bible_reference(reference), "{reference}");
    }
}

#[test]
fn avviser_ting_som_ikke_er_bibelreferanser() {
    for text in ["se figur 3", "Figur 3", "2024", "Mars 2024", "Kap 3", "Sak 2024/123", "ca. 300 personer", "Side 4"] {
        assert!(!is_bible_reference(text), "{text}");
    }
}

#[test]
fn alle_referanser_bibel_skriver_gjenkjennes() {
    let generated = common::generated();
    assert!(generated.len() > 400, "{}", generated.len());
    for reference in &generated {
        assert_eq!(
            extract_slide_content(&format!("Ikke med\n\nVers {reference}"), &SlideOptions::default()),
            format!("Vers {reference}\n"),
            "{reference}"
        );
    }
}

#[test]
fn referanser_gjores_om_til_oppslag_bibel_forstar_i_alle_oversettelser() {
    assert_eq!(to_lookup("Jesajas bok 53:5").as_deref(), Some("Isa53:5"));
    assert_eq!(to_lookup("Johannes' evangelium 3:16-17").as_deref(), Some("John3:16-17"));
    assert_eq!(to_lookup("1. Kor 13,4-7").as_deref(), Some("1Cor13:4-7"));
    assert_eq!(to_lookup("1. Samuelsbok 1:27-2:1").as_deref(), Some("1Sam1:27-2:1"));
    assert_eq!(to_lookup("Første Samuelsbok 1:27-2:1").as_deref(), Some("1Sam1:27-2:1"));
    // Bok-id-en kan være OSIS («Rev») eller USFM («REV»), etter hvilke oversettelser
    // appen er bygget med; `bibel` slår opp begge i alle.
    let lookup = to_lookup("Åp 21,4").expect("Åp 21,4");
    let found = common::bibel().run(&["--format", "{ref}", &format!("kjv:{lookup}")]);
    assert_eq!(found.stdout, "Revelation 21:4\n");
    assert_eq!(to_lookup("Salmenes bok 23").as_deref(), Some("Ps23"));
    assert_eq!(to_lookup("Rom 8,28; 12,1-2"), None);
    assert_eq!(to_lookup("Luk 15,11ff"), None);
    for reference in common::generated() {
        let inner = &reference[1..reference.len() - 1];
        assert!(to_lookup(inner).is_some(), "{reference}");
    }
}
