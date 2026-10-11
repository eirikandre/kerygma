//! Oppslag av oversettelser og innstillingene for `bibel`, med nb30 og KJV fra `bibler/`.

mod common;

use kerygma_core::bible::{look_up_translations, Bibel, BibelConfig};
use regex::Regex;

fn matches(pattern: &str, text: &str) -> bool {
    Regex::new(pattern).unwrap().is_match(text)
}

#[test]
fn slar_opp_med_standardinnstillingene() {
    let b = common::bibel();
    let one = b.run(&["--format", "{tekst} ({ref})", "joh3:16"]);
    assert_eq!(one.code, 0);
    assert_eq!(
        one.stdout,
        "For så har Gud elsket verden at han gav sin Sønn, den enbårne, forat hver den som tror på ham, ikke skal \
         fortapes, men ha evig liv; (Johannes' evangelium 3:16)\n"
    );

    let both = b.run(&["--format", "{tekst} ({ref})", "ne:joh3:16"]).stdout;
    let both: Vec<&str> = both.trim().split('\n').collect();
    assert_eq!(both.len(), 2);
    assert!(matches(r"\(John 3:16\)$", both[1]), "{}", both[1]);

    let unknown = b.run(&["finnesikke1:1"]);
    assert_eq!(unknown.code, 1);
    assert!(unknown.stdout.starts_with('['), "{}", unknown.stdout);
    assert_eq!(b.run(&["finnesikke1:1", "--alltid-null"]).code, 0);
}

#[test]
fn soker() {
    let b = common::bibel();
    let output = b.run(&["--søk", "tro håp kjærlighet", "--antall", "10"]);
    assert_eq!(output.code, 0);
    assert!(output.stdout.contains("(Første korinterbrev 13:13)"), "{}", output.stdout);
}

#[test]
fn oversettelser_som_ikke_er_installert_hoppes_over_med_en_melding() {
    let mut b = common::bibel();
    let notes = b
        .configure(&BibelConfig { default_translation: "finnesikke".to_string(), aliases: "n=nb30,x=finnesikke".to_string() })
        .unwrap();
    assert_eq!(notes.len(), 2, "{notes:?}");
    assert!(notes[0].contains("«finnesikke» er ikke installert"), "{notes:?}");
    assert!(notes[1].contains("«x=finnesikke»"), "{notes:?}");
    assert_eq!(b.run(&["n:joh3:16"]).code, 0);
    assert_ne!(b.run(&["x:joh3:16"]).code, 0);
}

#[test]
fn ugyldige_snarveier_avvises_uten_a_miste_de_gamle() {
    let mut b = common::bibel();
    assert!(b.set_aliases("ne").is_err());
    assert_eq!(b.run(&["ne:joh3:16"]).code, 0);
}

/// Uten nb30 (bare KJV) skal standardinnstillingene, som viser til nb30, virke uten feil,
/// med KJV som standard.
#[test]
fn bare_kjv_med_standardinnstillingene() {
    let kjv = std::fs::read(common::bibles_dir().join("kjv.bin")).expect("bibler/kjv.bin er i git");
    let mut b = Bibel::from_files(vec![("kjv.bin".to_string(), kjv)]).unwrap();
    let notes = b
        .configure(&BibelConfig { default_translation: "nb30".to_string(), aliases: "n=nb30,e=kjv,ne=nb30+kjv".to_string() })
        .unwrap();
    assert_eq!(notes.len(), 3, "{notes:?}");
    let one = b.run(&["--format", "{tekst} ({ref})", "e:joh3:16"]);
    assert_eq!(one.code, 0);
    assert!(one.stdout.trim_end().ends_with("(John 3:16)"), "{}", one.stdout);
    assert_eq!(b.run(&["joh3:16"]).code, 0);
}

#[test]
fn legger_til_oversettelse() {
    let b = common::bibel();
    let references: Vec<String> =
        ["Jesajas bok 53:5", "Joh 3,16", "Rom 8,28; 12,1-2"].iter().map(|s| s.to_string()).collect();
    let translations = look_up_translations(&b, "kjv", &references);
    assert!(matches(r"\(Isaiah 53:5\)$", &translations["Jesajas bok 53:5"]));
    assert!(matches(r"\(John 3:16\)$", &translations["Joh 3,16"]));
    assert!(!translations.contains_key("Rom 8,28; 12,1-2"));
}

#[test]
fn word_importen_slar_opp_boker_vers_og_sok() {
    use kerygma_core::word::bible_text::{BibelText, BibleText, VerseId};
    use kerygma_core::word::import_translations;

    let b = common::bibel();
    let translations = import_translations(&b, "nb30", "kjv");
    assert_eq!(translations, ["nb30"]);
    let translations: Vec<&str> = translations.iter().map(String::as_str).collect();
    let bible = BibelText::new(&b, &translations).unwrap();
    assert_eq!(bible.translations(), ["nb30"]);
    let dan = bible.book_id("Dan").expect("Dan");
    assert_eq!(bible.book_id("Daniel").as_deref(), Some(dan.as_str()));
    assert!(bible.book_id("2.Kongebok").is_some());
    assert_eq!(bible.book_id("Figur"), None);
    let verse = bible.verse("nb30", &VerseId { book: dan.clone(), chapter: 1, verse: 1 }).unwrap();
    assert_eq!(
        verse,
        "I Judas konge Jojakims tredje regjeringsår drog Nebukadnesar, kongen i Babel, til Jerusalem og kringsatte det."
    );
    assert_eq!(bible.reference(&dan, 1, 1, 2).as_deref(), Some("Daniels bok 1:1-2"));
    let query: Vec<String> = ["enbårne", "elsket", "verden", "fortapes"].iter().map(|s| s.to_string()).collect();
    let hits = bible.search("nb30", &query);
    assert!(hits.iter().any(|h| h.chapter == 3 && h.verse == 16), "{hits:?}");
}
