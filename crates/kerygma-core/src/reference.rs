//! Gjenkjenner bibelreferanser som "Joh 3,16", "1. Kor 13,4-7", "Sal 23",
//! "Rom 8,28; 12,1-2" og "Joh 3:16 NB", og alt `bibel` lager, som
//! "Profeten Jesaja 53:5", "Johannes´ evangelium 3:16-17" og "John 3:16".
//!
//! Skrevet etter `src/bibleReference.ts` i obsidian-slide-creator. `\d` er
//! skrevet `[0-9]`, siden `\d` i JavaScript bare er ASCII-sifre.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;

use crate::book_names::BOOK_IDS;

static LEADING_NUMBER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[1-5]\.?\s*").unwrap());
static SEPARATORS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\s´'’.]+").unwrap());

/// Fjerner tall foran, mellomrom, apostrofer og punktum: "1. Johannes´ brev" -> "johannesbrev".
fn normalize(name: &str) -> String {
    let lower = name.to_lowercase();
    let without_number = LEADING_NUMBER.replace(&lower, "");
    SEPARATORS.replace_all(&without_number, "").into_owned()
}

/// Som [`normalize`], men beholder tallet: "1. Kor" -> "1kor".
fn book_key(name: &str) -> String {
    SEPARATORS.replace_all(&name.to_lowercase(), "").into_owned()
}

/// Navn og forkortelser fra alle oversettelsene i `bible`.
static KNOWN_NAMES: LazyLock<HashSet<String>> =
    LazyLock::new(|| BOOK_IDS.iter().map(|(name, _)| normalize(name)).collect());

/// Senere navn med samme nøkkel vinner, som i `new Map(...)`.
static BOOK_ID_BY_KEY: LazyLock<HashMap<String, &'static str>> =
    LazyLock::new(|| BOOK_IDS.iter().map(|(name, id)| (book_key(name), *id)).collect());

/// Stammer av bøkenes norske navn, for referanser skrevet for hånd. En bok
/// matcher hvis navnet starter med stammen ("Johannesevangeliet") eller er en
/// forkortelse av den på minst tre bokstaver ("Joh").
const BOOK_STEMS: &[&str] = &[
    // GT
    "mosebok", "josva", "dommer", "rut", "samuel", "konge", "krønike", "esra",
    "nehemja", "ester", "job", "salme", "ordspråk", "forkynner", "høysang",
    "jesaja", "jeremia", "klagesang", "esekiel", "daniel", "hosea", "joel",
    "amos", "obadja", "jona", "mika", "nahum", "habakkuk", "sefanja", "haggai",
    "sakarja", "malaki",
    // NT
    "matteus", "markus", "lukas", "johannes", "apostlenesgjerninger", "romer",
    "korinter", "galater", "efeser", "filipper", "kolosser", "tessaloniker",
    "timoteus", "titus", "filemon", "hebreer", "jakob", "peter", "judas",
    "åpenbaring",
];

/// Ord som kan stå etter et boknavn skrevet for hånd: "Johannes´ brev".
static HANDWRITTEN_BOOK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:[1-5]\.?\s*)?\p{L}+[´'’]?\.?(?:\s+(?:brev|evangelium|gjerninger|bok))?$").unwrap()
});

const BOOK: &str = r"(?<book>(?:[1-5]\.?\s*)?\p{L}[\p{L}´'’.]*(?:\s+\p{L}[\p{L}´'’.]*){0,3}?)";
const VERSES: &str = r"[0-9]+[a-z]?(?:\s*[-–]\s*[0-9]+[a-z]?)?";
const TRANSLATION: &str = r"(?:\s+[A-Z][A-Za-z]{1,5}[0-9]{0,4})?";

fn locus() -> String {
    format!(
        r"[0-9]+(?:\s*[,:]\s*{VERSES}(?:\s*[.,]\s*{VERSES})*)?(?:\s*ff?\.?)?(?:\s*[-–]\s*[0-9]+(?:\s*[,:]\s*[0-9]+)?)?"
    )
}

static FIRST_PART: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^{BOOK}\s*{}{TRANSLATION}$", locus())).unwrap());
static FOLLOWING_PART: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^(?:{BOOK}\s*)?{}{TRANSLATION}$", locus())).unwrap());
static PART_SEPARATOR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*;\s*").unwrap());

pub fn is_book_name(name: &str) -> bool {
    let normalized = normalize(name);
    if KNOWN_NAMES.contains(&normalized) {
        return true;
    }
    if !HANDWRITTEN_BOOK.is_match(name.trim()) {
        return false;
    }
    BOOK_STEMS.iter().any(|stem| {
        normalized.starts_with(stem) || (normalized.chars().count() >= 3 && stem.starts_with(&normalized))
    })
}

/// Er teksten (innholdet i en parentes) én eller flere bibelreferanser?
pub fn is_bible_reference(text: &str) -> bool {
    PART_SEPARATOR.split(text.trim()).enumerate().all(|(i, part)| {
        let pattern = if i == 0 { &*FIRST_PART } else { &*FOLLOWING_PART };
        match pattern.captures(part) {
            None => false,
            Some(caps) => caps.name("book").is_none_or(|book| is_book_name(book.as_str())),
        }
    })
}

static SIMPLE_LOCUS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[0-9]+(?::[0-9]+(?:-[0-9]+(?::[0-9]+)?)?)?$").unwrap());

/// Gjør en referanse om til et oppslag `bibel` forstår i alle oversettelser:
/// "Profeten Jesaja 53:5" -> "Isa53:5", "1. Kor 13,4-7" -> "1Cor13:4-7".
/// Gir `None` for boknavn som ikke finnes i `bible` og for former `bibel`
/// ikke støtter, som "Rom 8,28; 12,1-2" og "Luk 15,11ff".
pub fn to_lookup(reference: &str) -> Option<String> {
    let caps = FIRST_PART.captures(reference.trim())?;
    let book = caps.name("book")?.as_str();
    if book.is_empty() {
        return None;
    }
    let id = BOOK_ID_BY_KEY.get(&book_key(book))?;
    let locus = reference_locus(reference);
    SIMPLE_LOCUS.is_match(&locus).then(|| format!("{id}{locus}"))
}

static LOCUS_AT_END: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[0-9][0-9\s,:.;–-]*(?:ff?\.?)?(?:\s+[A-Z][A-Za-z]{1,5}[0-9]{0,4})?$").unwrap()
});
static TRANSLATION_AT_END: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s+[A-Z][A-Za-z]{1,5}[0-9]{0,4}$").unwrap());
static WHITESPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

/// Kapittel og vers uten boknavn: "Johannes´ evangelium 3:16-17" og "John 3,16–17" gir "3:16-17".
pub fn reference_locus(reference: &str) -> String {
    let locus = LOCUS_AT_END.find(reference.trim()).map_or("", |m| m.as_str());
    let locus = TRANSLATION_AT_END.replace(locus, "");
    WHITESPACE.replace_all(&locus, "").replace(',', ":").replace('–', "-")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceMatch {
    /// Byte-indeks til `(`.
    pub start: usize,
    /// Byte-indeks rett etter `)`.
    pub end: usize,
    pub reference: String,
}

static PARENTHESES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\(([^()\n]+)\)").unwrap());

/// Finner alle "(referanse)" i teksten som er bibelreferanser.
pub fn find_bible_references(text: &str) -> Vec<ReferenceMatch> {
    PARENTHESES
        .captures_iter(text)
        .filter(|caps| is_bible_reference(&caps[1]))
        .map(|caps| {
            let whole = caps.get(0).unwrap();
            ReferenceMatch { start: whole.start(), end: whole.end(), reference: caps[1].trim().to_string() }
        })
        .collect()
}
