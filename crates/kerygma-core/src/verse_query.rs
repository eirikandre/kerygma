//! Tolker det som skrives i oppslagsvinduet for bibelvers, skrevet som i
//! espanso-oppsettet til bible: "1mos3:1", "ne:joh3:16" og "s=tro håp" (søk).
//! Kolon foran og mellomrom rundt, som i ":1mos3:1 ", tåles. `suggest_verse`
//! søker også uten «s=» når teksten ikke er en referanse.
//!
//! Skrevet etter `src/verseQuery.ts` og `src/verseModal.ts` i obsidian-slide-creator.

use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use crate::bible::Bibel;
use crate::extract::len16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerseQueryKind {
    Lookup,
    Search,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerseQuery {
    pub kind: VerseQueryKind,
    /// Det som sendes til `bibel`: referansen eller søket.
    pub query: String,
}

/// Søk må ha minst så mange tegn, så hvert tastetrykk ikke gir hele Bibelen.
const MIN_SEARCH_CHARS: usize = 2;
pub const SEARCH_HITS: usize = 10;

static SEARCH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^s=([^=]*)=?$").unwrap());
static WHITESPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());
static LETTER_THEN_DIGIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\p{L}.*[0-9]").unwrap());

pub fn parse_verse_query(input: &str) -> Option<VerseQuery> {
    let trimmed = input.trim();
    let text = trimmed.strip_prefix(':').unwrap_or(trimmed);
    if let Some(caps) = SEARCH.captures(text) {
        let query = caps[1].trim();
        return (len16(query) >= MIN_SEARCH_CHARS)
            .then(|| VerseQuery { kind: VerseQueryKind::Search, query: query.to_string() });
    }
    // En referanse trenger minst bok og kapittel, som "sal23".
    let query = WHITESPACE.replace_all(text, "");
    LETTER_THEN_DIGIT
        .is_match(&query)
        .then(|| VerseQuery { kind: VerseQueryKind::Lookup, query: query.into_owned() })
}

/// Verset som et markdown-sitat ("> tekst (ref)"), med hver oversettelse på sin
/// linje. Står markøren midt i en linje, skilles sitatet fra teksten rundt med
/// en tom linje, så teksten ikke blir en del av sitatet.
pub fn to_quote(verse: &str, text_before: &str, text_after: &str) -> String {
    let quote = verse.split('\n').map(|line| format!("> {line}")).collect::<Vec<_>>().join("\n");
    let before = if text_before.trim().is_empty() { "" } else { "\n\n" };
    let after = if text_after.trim().is_empty() { "" } else { "\n\n" };
    format!("{before}{quote}{after}")
}

/// Et forslag i oppslagsvinduet: tekst som kan settes inn, eller en melding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Suggestion {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl Suggestion {
    pub fn text(text: impl Into<String>) -> Self {
        Suggestion { text: Some(text.into()), message: None }
    }

    pub fn message(message: impl Into<String>) -> Self {
        Suggestion { text: None, message: Some(message.into()) }
    }
}

/// Meldinger, som "[ukjent bok …]" og "[… og 13 treff til]", står i hakeparentes.
fn as_suggestion(line: &str) -> Suggestion {
    if line.starts_with('[') && line.ends_with(']') && line.len() >= 2 {
        Suggestion::message(&line[1..line.len() - 1])
    } else {
        Suggestion::text(line)
    }
}

/// Forslagene for det som er skrevet i oppslagsvinduet, som `getSuggestions`.
pub fn lookup_verse(bibel: &Bibel, input: &str) -> Vec<Suggestion> {
    let Some(parsed) = parse_verse_query(input) else { return vec![] };
    let hits = SEARCH_HITS.to_string();
    let args: Vec<&str> = match parsed.kind {
        VerseQueryKind::Search => vec!["--søk", &parsed.query, "--antall", &hits, "--alltid-null"],
        VerseQueryKind::Lookup => vec![&parsed.query, "--alltid-null"],
    };
    let output = bibel.run(&args).stdout;
    let output = output.trim();
    match parsed.kind {
        VerseQueryKind::Lookup => vec![as_suggestion(output)],
        VerseQueryKind::Search => {
            output.split('\n').filter(|line| !line.trim().is_empty()).map(as_suggestion).collect()
        }
    }
}

/// Søk uten «s=» starter først ved så mange tegn, så de første bokstavene i en bok ikke gir treff.
pub const MIN_AUTO_SEARCH_CHARS: usize = 3;

/// Forslagene i oppslagsvinduet i appen: som `lookup_verse`, men tekst som ikke
/// er en referanse ("tro håp", "hyrde"), søkes etter uten at «s=» må skrives.
/// Ser teksten ut som en referanse, men gir ikke noe vers ("finnesikke1:1"),
/// søkes det også; finner ikke søket noe, vises meldingen fra oppslaget.
pub fn suggest_verse(bibel: &Bibel, input: &str) -> Vec<Suggestion> {
    let trimmed = input.trim();
    let text = trimmed.strip_prefix(':').unwrap_or(trimmed).trim();
    match parse_verse_query(input).map(|parsed| parsed.kind) {
        Some(VerseQueryKind::Search) => lookup_verse(bibel, input),
        Some(VerseQueryKind::Lookup) => {
            let found = lookup_verse(bibel, input);
            if found.iter().any(|s| s.text.is_some()) {
                return found;
            }
            let hits = auto_search(bibel, text);
            if hits.iter().any(|s| s.text.is_some()) {
                hits
            } else {
                found
            }
        }
        None if len16(text) >= MIN_AUTO_SEARCH_CHARS => auto_search(bibel, text),
        None => vec![],
    }
}

/// Søk som om det var skrevet «s=» foran. «=» kan ikke stå i et søk, så det blir mellomrom.
fn auto_search(bibel: &Bibel, text: &str) -> Vec<Suggestion> {
    lookup_verse(bibel, &format!("s={}", text.replace('=', " ")))
}
