//! `suggest_verse`: oppslagsvinduet søker av seg selv når teksten ikke er en referanse,
//! uten at søket må starte med «s=».

mod common;

use kerygma_core::verse_query::{suggest_verse, SEARCH_HITS};

fn texts(found: &[kerygma_core::verse_query::Suggestion]) -> Vec<&str> {
    found.iter().filter_map(|s| s.text.as_deref()).collect()
}

#[test]
fn referanser_slas_opp_som_for() {
    let Some(b) = common::bibel() else { return };
    let one = suggest_verse(&b, "joh3:16");
    assert_eq!(one.len(), 1);
    assert!(one[0].text.as_deref().unwrap().ends_with("(Johannes´ evangelium 3:16)"), "{one:?}");
}

#[test]
fn tekst_uten_referanse_sokes_etter() {
    let Some(b) = common::bibel() else { return };
    let hits = suggest_verse(&b, "tro håp kjærlighet");
    assert!(texts(&hits).iter().any(|t| t.contains("1. Korinterbrev 13")), "{hits:?}");
    assert!(hits.len() <= SEARCH_HITS + 1);
    // Samme svar som med «s=».
    assert_eq!(hits, suggest_verse(&b, "s=tro håp kjærlighet"));
    // For kort til å søke.
    assert!(suggest_verse(&b, "jo").is_empty());
}

#[test]
fn referanse_som_ikke_finnes_gir_meldingen_nar_sok_heller_ikke_finner_noe() {
    let Some(b) = common::bibel() else { return };
    let unknown = suggest_verse(&b, "finnesikke1:1");
    assert_eq!(unknown.len(), 1);
    assert!(unknown[0].message.as_deref().unwrap().contains("ukjent bok"), "{unknown:?}");
    let chapter = suggest_verse(&b, "joh99:1");
    assert!(chapter[0].message.as_deref().unwrap().contains("21 kapitler"), "{chapter:?}");
    let none = suggest_verse(&b, "xyzzyqw");
    assert!(none[0].message.as_deref().unwrap().contains("ingen treff"), "{none:?}");
}
