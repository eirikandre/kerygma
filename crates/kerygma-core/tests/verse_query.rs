//! Testene fra `test/verseQuery.test.ts` i obsidian-slide-creator, og
//! forslagene fra `verseModal.ts`.

mod common;

use kerygma_core::verse_query::{lookup_verse, parse_verse_query, to_quote, VerseQuery, VerseQueryKind};

fn query(input: &str) -> Option<String> {
    parse_verse_query(input).map(|q| q.query)
}

#[test]
fn tolker_oppslag() {
    assert_eq!(parse_verse_query("1mos3:1"), Some(VerseQuery { kind: VerseQueryKind::Lookup, query: "1mos3:1".into() }));
    assert_eq!(parse_verse_query(" :joh3:16 "), Some(VerseQuery { kind: VerseQueryKind::Lookup, query: "joh3:16".into() }));
    assert_eq!(query("ne:joh3:16-17").as_deref(), Some("ne:joh3:16-17"));
    assert_eq!(query("nb88+kjv:sal23").as_deref(), Some("nb88+kjv:sal23"));
    assert_eq!(query("1sam1:20-2:5").as_deref(), Some("1sam1:20-2:5"));
    assert_eq!(query("åp 21,4").as_deref(), Some("åp21,4"));
}

#[test]
fn tolker_sok() {
    assert_eq!(parse_verse_query("s=tro håp"), Some(VerseQuery { kind: VerseQueryKind::Search, query: "tro håp".into() }));
    assert_eq!(query("s=kjv:love=").as_deref(), Some("kjv:love"));
    assert_eq!(parse_verse_query("s=t"), None);
}

#[test]
fn venter_pa_mer_tekst() {
    assert_eq!(parse_verse_query(""), None);
    assert_eq!(parse_verse_query("joh"), None);
    assert_eq!(parse_verse_query("12"), None);
}

#[test]
fn setter_inn_verset_som_sitat() {
    assert_eq!(to_quote("tekst (Joh 3:16)", "", ""), "> tekst (Joh 3:16)");
    assert_eq!(
        to_quote("norsk (Johannes 3:16)\nengelsk (John 3:16)", "", ""),
        "> norsk (Johannes 3:16)\n> engelsk (John 3:16)"
    );
    assert_eq!(to_quote("tekst (Joh 3:16)", "Se ", " her"), "\n\n> tekst (Joh 3:16)\n\n");
}

#[test]
fn forslag_for_oppslag_sok_og_feil() {
    let Some(b) = common::bibel() else { return };
    assert!(lookup_verse(&b, "joh").is_empty());

    let one = lookup_verse(&b, "joh3:16");
    assert_eq!(one.len(), 1);
    assert!(one[0].text.as_deref().unwrap().ends_with("(Johannes´ evangelium 3:16)"), "{one:?}");

    let both = lookup_verse(&b, "ne:joh3:16");
    assert_eq!(both.len(), 1);
    assert_eq!(both[0].text.as_deref().unwrap().lines().count(), 2);

    let hits = lookup_verse(&b, "s=tro håp kjærlighet");
    assert!(!hits.is_empty() && hits.len() <= 11, "{hits:?}");
    assert!(hits.iter().any(|h| h.text.as_deref().is_some_and(|t| t.contains("1. Korinterbrev 13"))));

    let unknown = lookup_verse(&b, "finnesikke1:1");
    assert_eq!(unknown.len(), 1);
    assert!(unknown[0].text.is_none() && unknown[0].message.is_some(), "{unknown:?}");
}
