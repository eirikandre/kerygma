//! Testene fra `test/bible.test.ts` i obsidian-slide-creator. De hoppes over
//! når `bibler/` mangler oversettelsene.

mod common;

use std::path::PathBuf;
use std::process::Command;

use kerygma_core::bible::{look_up_translations, BibelConfig};
use regex::Regex;

fn matches(pattern: &str, text: &str) -> bool {
    Regex::new(pattern).unwrap().is_match(text)
}

#[test]
fn slar_opp_som_espanso_oppsettet() {
    let Some(b) = common::bibel() else { return };
    let one = b.run(&["--format", "{tekst} ({ref})", "joh3:16"]);
    assert_eq!(one.code, 0);
    assert!(matches(r"\(Johannes´ evangelium 3:16\)\n$", &one.stdout), "{}", one.stdout);

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
    let Some(b) = common::bibel() else { return };
    let output = b.run(&["--søk", "tro håp kjærlighet", "--antall", "10"]);
    assert_eq!(output.code, 0);
    assert!(output.stdout.contains("1. Korinterbrev 13"), "{}", output.stdout);
}

#[test]
fn avviser_ugyldige_innstillinger_uten_a_miste_de_gamle() {
    let Some(mut b) = common::bibel() else { return };
    let error = b
        .configure(&BibelConfig { default_translation: "finnesikke".to_string(), aliases: String::new() })
        .unwrap_err();
    assert!(error.contains("ukjent oversettelse"), "{error}");
    let error = b.set_aliases("x=finnesikke").unwrap_err();
    assert!(error.contains("finnesikke"), "{error}");
    assert_eq!(b.run(&["ne:joh3:16"]).code, 0);
}

#[test]
fn legger_til_oversettelse() {
    let Some(b) = common::bibel() else { return };
    let references: Vec<String> =
        ["Profeten Jesaja 53:5", "Joh 3,16", "Rom 8,28; 12,1-2"].iter().map(|s| s.to_string()).collect();
    let translations = look_up_translations(&b, "kjv", &references);
    assert!(matches(r"\(Isaiah 53:5\)$", &translations["Profeten Jesaja 53:5"]));
    assert!(matches(r"\(John 3:16\)$", &translations["Joh 3,16"]));
    assert!(!translations.contains_key("Rom 8,28; 12,1-2"));
}

#[test]
fn gir_samme_svar_som_kjorbaren() {
    let Some(b) = common::bibel() else { return };
    let native = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../bible/target/release/bibel");
    if !native.exists() {
        eprintln!("hopper over: fant ikke {}", native.display());
        return;
    }
    // Kjørbaren er bygget med egne BIBEL_DEFAULT/BIBEL_ALIAS, så hvert tilfelle velger oversettelse selv.
    let cases: &[&[&str]] = &[
        &["nb88:1sam2:1-3"],
        &["kjv:john3:16"],
        &["nb88+kjv:sal23"],
        &["nb24:eph1:16"],
        &["--format", "{tekst} ({ref}, {oversettelse})", "--uten-numre", "nb88:1kor13:4-7"],
        &["--søk", "nb88:\"i begynnelsen\"", "--antall", "3"],
        &["--søk", "kjv:love", "--omfang", "kapittel"],
        &["nb88:finnesikke1:1"],
    ];
    for args in cases {
        let wanted = Command::new(&native).args(*args).output().expect("kjøre bibel");
        let got = b.run(args);
        assert_eq!(got.stdout, String::from_utf8_lossy(&wanted.stdout), "{}", args.join(" "));
        assert_eq!(i32::from(got.code), wanted.status.code().unwrap_or(-1), "{}", args.join(" "));
    }
}

#[test]
fn word_importen_slar_opp_boker_vers_og_sok() {
    use kerygma_core::word::bible_text::{BibelText, BibleText, VerseId};
    use kerygma_core::word::IMPORT_TRANSLATIONS;

    let Some(b) = common::bibel() else { return };
    let bible = BibelText::new(&b, IMPORT_TRANSLATIONS).unwrap();
    assert_eq!(bible.translations(), ["nb88", "nb24"]);
    let dan = bible.book_id("Dan").expect("Dan");
    assert_eq!(bible.book_id("Daniel").as_deref(), Some(dan.as_str()));
    assert!(bible.book_id("2.Kongebok").is_some());
    assert_eq!(bible.book_id("Figur"), None);
    let verse = bible.verse("nb88", &VerseId { book: dan.clone(), chapter: 1, verse: 1 }).unwrap();
    assert!(verse.contains("Jojakim"), "{verse}");
    assert_eq!(bible.reference(&dan, 1, 1, 2).as_deref(), Some("Profeten Daniel 1:1-2"));
    let query: Vec<String> = ["enbårne", "elsket", "verden", "fortapes"].iter().map(|s| s.to_string()).collect();
    let hits = bible.search("nb88", &query);
    assert!(hits.iter().any(|h| h.chapter == 3 && h.verse == 16), "{hits:?}");
}
