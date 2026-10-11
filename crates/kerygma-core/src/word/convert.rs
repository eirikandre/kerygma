//! Gjør avsnittene fra et Word-manus om til markdown. Bibelsitater blir
//! `> tekst (referanse)`, med referansen slik `bibel` skriver den, så
//! presentasjonen kan sette engelsk ved siden av.
//!
//! Sitatene finnes ved å slå opp referansene i manuset ("Jer 7,4", "Dan 1:3",
//! "V1", "1:4") og lete etter bibelteksten i avsnittene etter (eller før)
//! referansen. Avsnitt uten referanse som er et helt bibelvers, finnes med søk.
//!
//! Se `docs/word-import.md`. Posisjoner i teksten er byte-indekser, og alltid på tegngrenser.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use fancy_regex::Regex as FancyRegex;
use regex::Regex;
use serde::Serialize;

use crate::word::align::{align, words, Alignment};
use crate::word::bible_text::{BibleText, VerseId};
use crate::word::docx::{Paragraph, Run};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FoundQuote {
    pub reference: String,
    /// Hvorfor sitatet bør sjekkes. Står også i markdown, som `> [!warning|sjekk] ...`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    /// Teksten i manuset referansen ble funnet fra, som "Jer 7,4", "V1" eller "søk".
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConvertResult {
    pub markdown: String,
    pub quotes: Vec<FoundQuote>,
    /// Referanser med vers som ikke ble til sitater, til manuell sjekk.
    pub unmatched: Vec<String>,
}

#[derive(Debug, Clone)]
struct Para {
    runs: Vec<Run>,
    text: String,
    heading: Option<usize>,
    list_item: bool,
}

#[derive(Debug, Clone)]
struct Token {
    p: usize,
    start: usize,
    end: usize,
    word: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Context {
    book: String,
    chapter: u32,
}

#[derive(Debug, Clone)]
struct Ref {
    p: usize,
    start: usize,
    end: usize,
    text: String,
    /// Bok og kapittel som skal prøves, i rekkefølge.
    contexts: Vec<Context>,
    from: u32,
    to: u32,
}

#[derive(Debug, Clone, Copy)]
struct Span {
    p: usize,
    start: usize,
    end: usize,
}

#[derive(Debug, Clone)]
struct Quote {
    start_p: usize,
    start_c: usize,
    end_p: usize,
    end_c: usize,
    reference: String,
    source: String,
    warning: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Forward,
    Backward,
    Search,
    Alone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FoundKind {
    Book,
    Verse,
    Mention,
}

#[derive(Debug, Clone)]
struct Found {
    kind: FoundKind,
    start: usize,
    end: usize,
    text: String,
    context: Option<Context>,
    chapter: Option<u32>,
    from: u32,
    to: u32,
}

/// Et sitat funnet i bibelteksten: første og siste ord (indekser i `tokens`).
#[derive(Debug, Clone)]
struct Located {
    start: usize,
    end: usize,
    verses: (u32, u32),
    reference: String,
}

fn fancy(pattern: &str) -> FancyRegex {
    FancyRegex::new(pattern).unwrap()
}

// `\b` i JavaScript er en ASCII-ordgrense, så den skrives som lookaround her.
static BOOK_REF: LazyLock<FancyRegex> = LazyLock::new(|| {
    fancy(r"(?<![\p{L}\p{N}])((?:[1-5]\.?\s?)?\p{L}+\.?)\s?([0-9]{1,3})\s*[,:]\s*([0-9]{1,3})(?:\s*[-–]\s*([0-9]{1,3}))?(?:\s*ff?(?![0-9A-Za-z_])\.?)?")
});
static VERSE_REF: LazyLock<FancyRegex> = LazyLock::new(|| {
    fancy(r"(?<![\p{L}\p{N}])[Vv](?:ers(?:ene)?|\.)?\s?([0-9]{1,3})(?:\s*[-–]\s*([0-9]{1,3}))?(?![\p{L}0-9:,])")
});
static CHAPTER_VERSE_AT_START: LazyLock<FancyRegex> =
    LazyLock::new(|| fancy(r"^\s*([0-9]{1,3})[:,]([0-9]{1,3})(?:\s*[-–]\s*([0-9]{1,3}))?(?=\s)"));
static CHAPTER_MENTION: LazyLock<FancyRegex> =
    LazyLock::new(|| fancy(r"(?i)(?<![\p{L}])(\p{L}+)\s+kap(?:ittel|\.)?\s*([0-9]{1,3})(?![0-9A-Za-z_])"));
/// Oversettelsen skrevet etter et sitat: "(N78)", "(NB88)".
static TRANSLATION_MARK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\(\s*[A-Z]{1,4}\s?[0-9]{0,4}\s*\)").unwrap());
static NUMBER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[0-9]+$").unwrap());
static WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\p{L}\p{N}]+").unwrap());
static LETTER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\p{L}").unwrap());

fn is_sentence_end(c: char) -> bool {
    matches!(c, '.' | '!' | '?' | ':' | ';')
}
fn is_open_quote(c: char) -> bool {
    matches!(c, '«' | '“' | '„' | '"')
}
fn is_close_quote(c: char) -> bool {
    matches!(c, '»' | '”' | '"')
}

/// Hvor mange ord sitatet kan starte etter referansen, som i "står det at ...".
const MAX_LEAD_IN: usize = 40;

/// `text[start..end]`, eller tom tekst når `start >= end`, som `slice` i JavaScript.
fn slice(text: &str, start: usize, end: usize) -> &str {
    let end = end.min(text.len());
    if start >= end {
        ""
    } else {
        &text[start..end]
    }
}

fn is_number(word: &str) -> bool {
    NUMBER.is_match(word)
}

fn len16(text: &str) -> usize {
    text.encode_utf16().count()
}

pub struct ConvertOptions {
    /// Søk etter bibelvers i avsnitt uten referanse. Standard: true.
    pub search: bool,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        ConvertOptions { search: true }
    }
}

pub fn convert(paragraphs: &[Paragraph], bible: &dyn BibleText, options: &ConvertOptions) -> ConvertResult {
    Converter::new(prepare(paragraphs), bible).run(options.search)
}

static ENDS_SENTENCE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"[.!?:;»”"]\s*$"#).unwrap());
static STARTS_LOWERCASE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*\p{Ll}").unwrap());

/// Rydder i tekst og stiler, slår sammen avsnitt delt midt i en setning og finner overskrifter.
fn prepare(paragraphs: &[Paragraph]) -> Vec<Para> {
    let mut result: Vec<Para> = Vec::new();
    for paragraph in paragraphs {
        let runs: Vec<Run> = paragraph
            .runs
            .iter()
            .map(|run| Run { text: run.text.replace('\t', " "), ..run.clone() })
            .filter(|run| !run.text.is_empty())
            .collect();
        let text: String = runs.iter().map(|r| r.text.as_str()).collect();
        let mut para = Para { runs, text, heading: None, list_item: paragraph.list_item };
        para.heading = heading_level(paragraph.style.as_deref(), &para);

        let continues = result.last().is_some_and(|previous| {
            previous.heading.is_none()
                && para.heading.is_none()
                && !previous.text.trim().is_empty()
                && !ENDS_SENTENCE.is_match(&previous.text)
                && STARTS_LOWERCASE.is_match(&para.text)
        });
        if continues {
            let previous = result.last_mut().unwrap();
            previous.runs.push(Run::plain(" "));
            previous.runs.extend(para.runs);
            previous.text = previous.runs.iter().map(|r| r.text.as_str()).collect();
        } else {
            result.push(para);
        }
    }
    result
}

static STYLED_HEADING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^(?:heading|overskrift)\s*([0-9])$").unwrap());
static TITLE_STYLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^(?:title|tittel)$").unwrap());
static ENDS_PUNCTUATION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[.!?:;,–-]$").unwrap());
static KAPITTEL_N: LazyLock<FancyRegex> = LazyLock::new(|| fancy(r"(?i)(?<![0-9A-Za-z_])kapittel\s+[0-9]+"));
static DEL_N: LazyLock<FancyRegex> = LazyLock::new(|| fancy(r"(?i)(?<![0-9A-Za-z_])del\s+[0-9]+"));

fn heading_level(style: Option<&str>, para: &Para) -> Option<usize> {
    let style = style.unwrap_or("");
    if let Some(caps) = STYLED_HEADING.captures(style) {
        return Some(6.min(caps[1].parse::<usize>().unwrap_or(0) + 1));
    }
    if TITLE_STYLE.is_match(style) {
        return Some(1);
    }
    let text = para.text.trim();
    if text.is_empty() || len16(text) > 80 || ENDS_PUNCTUATION.is_match(text) {
        return None;
    }
    if KAPITTEL_N.is_match(text).unwrap_or(false) && DEL_N.is_match(text).unwrap_or(false) {
        return Some(2);
    }
    let all_bold = para.runs.iter().all(|run| run.bold || run.text.trim().is_empty());
    (all_bold && LETTER.is_match(text)).then_some(2)
}

struct Converter<'b> {
    paras: Vec<Para>,
    bible: &'b dyn BibleText,
    tokens: Vec<Token>,
    /// Indeksen til første ord i hvert avsnitt.
    first_token: Vec<usize>,
    starts_with_ref: Vec<bool>,
    /// Referanser med boknavn. Et sitat går aldri forbi en av dem.
    book_refs: Vec<Span>,
    quotes: Vec<Quote>,
    drops: Vec<Span>,
    /// Avsnitt som bare er en referanse, og som henges på avsnittet før ("han sier i Jer 7,4").
    join_previous: HashSet<usize>,
    touched: HashSet<usize>,
    unmatched: Vec<String>,
}

impl<'b> Converter<'b> {
    fn new(paras: Vec<Para>, bible: &'b dyn BibleText) -> Self {
        let mut tokens = Vec::new();
        let mut first_token = Vec::new();
        for (p, para) in paras.iter().enumerate() {
            first_token.push(tokens.len());
            for m in WORD.find_iter(&para.text) {
                tokens.push(Token { p, start: m.start(), end: m.end(), word: m.as_str().to_lowercase() });
            }
        }
        first_token.push(tokens.len());
        let count = paras.len();
        Converter {
            paras,
            bible,
            tokens,
            first_token,
            starts_with_ref: vec![false; count],
            book_refs: Vec::new(),
            quotes: Vec::new(),
            drops: Vec::new(),
            join_previous: HashSet::new(),
            touched: HashSet::new(),
            unmatched: Vec::new(),
        }
    }

    fn run(mut self, search: bool) -> ConvertResult {
        self.find_referenced_quotes();
        if search {
            self.find_unreferenced_quotes();
        }
        self.quotes.sort_by(|a, b| a.start_p.cmp(&b.start_p).then(a.start_c.cmp(&b.start_c)));
        ConvertResult {
            markdown: self.render(),
            quotes: self
                .quotes
                .iter()
                .map(|q| FoundQuote { reference: q.reference.clone(), source: q.source.clone(), warning: q.warning.clone() })
                .collect(),
            unmatched: self.unmatched,
        }
    }

    // --- Referanser ---

    fn find_referenced_quotes(&mut self) {
        let mut last: Option<Context> = None;
        let mut section: Option<Context> = None;
        let refs_by_para: Vec<Vec<Found>> =
            (0..self.paras.len()).map(|p| if self.paras[p].heading.is_none() { self.refs_in(p) } else { vec![] }).collect();
        for (p, refs) in refs_by_para.iter().enumerate() {
            self.starts_with_ref[p] =
                refs.iter().any(|r| r.kind != FoundKind::Mention && self.words_before(p, r.start) == 0);
            for r in refs.iter().filter(|r| r.kind == FoundKind::Book) {
                self.book_refs.push(Span { p, start: r.start, end: r.end });
            }
        }

        for (p, refs) in refs_by_para.iter().enumerate() {
            if self.paras[p].heading.is_some() {
                section = self.heading_context(&self.paras[p].text.clone(), section.clone());
                last = section.clone();
                continue;
            }
            for found in refs {
                if self.is_inside_quote(p, found.start) || self.is_quote_starting_after(p, found.end) {
                    continue;
                }
                if found.kind == FoundKind::Mention {
                    last = found.context.clone();
                    continue;
                }
                let mut contexts = Vec::new();
                if found.kind == FoundKind::Book {
                    contexts.push(found.context.clone().unwrap());
                    last = found.context.clone();
                } else {
                    // "V1" og "1:4" bruker boka fra forrige referanse, eller fra overskriften.
                    for context in [&last, &section].into_iter().flatten() {
                        contexts.push(Context {
                            book: context.book.clone(),
                            chapter: found.chapter.unwrap_or(context.chapter),
                        });
                    }
                }
                let reference = Ref {
                    p,
                    start: found.start,
                    end: found.end,
                    text: found.text.clone(),
                    contexts,
                    from: found.from,
                    to: found.to,
                };
                if !reference.contexts.is_empty() && !self.quote_for_ref(&reference) {
                    self.unmatched.push(reference.text);
                }
            }
        }
    }

    fn refs_in(&self, p: usize) -> Vec<Found> {
        let text = &self.paras[p].text;
        let number = |s: &str| s.parse::<u32>().unwrap_or(0);
        let mut found: Vec<Found> = Vec::new();
        for caps in BOOK_REF.captures_iter(text).flatten() {
            let whole = caps.get(0).unwrap();
            let name = &caps[1];
            let Some(book) = self.bible.book_id(name.strip_suffix('.').unwrap_or(name)) else { continue };
            let from = number(&caps[3]);
            found.push(Found {
                kind: FoundKind::Book,
                start: whole.start(),
                end: whole.end(),
                text: whole.as_str().to_string(),
                context: Some(Context { book, chapter: number(&caps[2]) }),
                chapter: None,
                from,
                to: caps.get(4).map_or(from, |to| from.max(number(to.as_str()))),
            });
        }
        for caps in CHAPTER_MENTION.captures_iter(text).flatten() {
            let whole = caps.get(0).unwrap();
            if let Some(book) = self.bible.book_id(&caps[1]) {
                found.push(Found {
                    kind: FoundKind::Mention,
                    start: whole.start(),
                    end: whole.start(),
                    text: whole.as_str().to_string(),
                    context: Some(Context { book, chapter: number(&caps[2]) }),
                    chapter: None,
                    from: 0,
                    to: 0,
                });
            }
        }
        for caps in VERSE_REF.captures_iter(text).flatten() {
            let whole = caps.get(0).unwrap();
            if found.iter().any(|f| f.kind == FoundKind::Book && whole.start() >= f.start && whole.start() < f.end) {
                continue;
            }
            let from = number(&caps[1]);
            found.push(Found {
                kind: FoundKind::Verse,
                start: whole.start(),
                end: whole.end(),
                text: whole.as_str().to_string(),
                context: None,
                chapter: None,
                from,
                to: caps.get(2).map_or(from, |to| from.max(number(to.as_str()))),
            });
        }
        if let Ok(Some(caps)) = CHAPTER_VERSE_AT_START.captures(text) {
            let whole = caps.get(0).unwrap();
            if !found.iter().any(|f| f.kind == FoundKind::Book && f.start <= whole.start() + 1) {
                let from = number(&caps[2]);
                found.push(Found {
                    kind: FoundKind::Verse,
                    start: whole.start(),
                    end: whole.end(),
                    text: whole.as_str().trim().to_string(),
                    context: None,
                    chapter: Some(number(&caps[1])),
                    from,
                    to: caps.get(3).map_or(from, |to| from.max(number(to.as_str()))),
                });
            }
        }
        found.sort_by_key(|f| f.start);
        found
    }

    /// "Daniels bok, kapittel 1, del 1" gir Daniel 1. Uten bok brukes boka fra forrige overskrift.
    fn heading_context(&self, text: &str, previous: Option<Context>) -> Option<Context> {
        static CHAPTER: LazyLock<FancyRegex> =
            LazyLock::new(|| fancy(r"(?i)(?<![0-9A-Za-z_])kap(?:ittel|\.)?\s*([0-9]+)"));
        static LONG_WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\p{L}{3,}").unwrap());
        let chapter: Option<u32> =
            CHAPTER.captures(text).ok().flatten().and_then(|caps| caps[1].parse().ok());
        let mut book: Option<String> = None;
        for m in LONG_WORD.find_iter(text) {
            if m.as_str().to_lowercase().starts_with("kap") {
                continue;
            }
            book = self.bible.book_id(m.as_str()).or_else(|| {
                let word = m.as_str();
                self.bible.book_id(word.strip_suffix('s').unwrap_or(word))
            });
            if book.is_some() {
                break;
            }
        }
        let book = book.or_else(|| previous.as_ref().map(|c| c.book.clone()));
        let Some(book) = book else { return previous };
        Some(Context { book, chapter: chapter.unwrap_or_else(|| previous.as_ref().map_or(1, |c| c.chapter)) })
    }

    fn quote_for_ref(&mut self, reference: &Ref) -> bool {
        let window = self.forward_window(reference);
        // Alle bøkene prøves strengt først, så en løs match i feil bok ikke vinner.
        for context in &reference.contexts {
            if let Some(forward) =
                self.best_alignment(context, reference.from, reference.to, &window, Direction::Forward)
            {
                self.add_quote(forward, Some(reference), Direction::Forward);
                return true;
            }
        }
        let backward_window = self.backward_window(reference);
        for context in &reference.contexts {
            if let Some(backward) =
                self.best_alignment(context, reference.from, reference.to, &backward_window, Direction::Backward)
            {
                self.add_quote(backward, Some(reference), Direction::Backward);
                return true;
            }
        }
        // En referanse alene i et avsnitt ("Dan 1:9") står foran sitatet, selv om
        // manuset bruker en annen oversettelse og bare noen få ord stemmer.
        if !self.is_ref_alone(reference) || window.is_empty() {
            return false;
        }
        for context in &reference.contexts {
            if let Some(loose) = self.best_alignment(context, reference.from, reference.to, &window, Direction::Alone) {
                if self.tokens[loose.start].p == self.tokens[window[0]].p {
                    self.add_quote(Located { start: window[0], ..loose }, Some(reference), Direction::Alone);
                    return true;
                }
            }
        }
        false
    }

    fn is_ref_alone(&self, reference: &Ref) -> bool {
        let text = &self.paras[reference.p].text;
        !LETTER.is_match(&format!("{}{}", &text[..reference.start], &text[reference.end..]))
    }

    /// Ordene etter referansen, til neste overskrift eller neste avsnitt som starter med en referanse.
    fn forward_window(&self, reference: &Ref) -> Vec<usize> {
        let mut window = Vec::new();
        let mut p = reference.p;
        for i in self.first_token[reference.p]..self.tokens.len() {
            let token = &self.tokens[i];
            if token.p == reference.p && token.start < reference.end {
                continue;
            }
            if token.p != p {
                p = token.p;
                if self.paras[p].heading.is_some() || p - reference.p > 8 {
                    break;
                }
                if self.starts_with_ref[p] && !window.is_empty() {
                    break;
                }
            }
            if self.is_inside_quote(token.p, token.start) || window.len() > 400 {
                break;
            }
            if self.book_refs.iter().any(|r| r.p == token.p && token.start >= r.start && token.start < r.end) {
                break;
            }
            if !is_number(&token.word) {
                window.push(i);
            }
        }
        window
    }

    /// Ordene foran referansen i samme avsnitt.
    fn backward_window(&self, reference: &Ref) -> Vec<usize> {
        let mut window = Vec::new();
        for i in self.first_token[reference.p]..self.first_token[reference.p + 1] {
            let token = &self.tokens[i];
            if token.start >= reference.start {
                break;
            }
            if self.is_inside_quote(token.p, token.start) {
                window.clear();
            } else if !is_number(&token.word) {
                window.push(i);
            }
        }
        window
    }

    fn best_alignment(
        &self,
        context: &Context,
        from: u32,
        to: u32,
        window: &[usize],
        direction: Direction,
    ) -> Option<Located> {
        if window.is_empty() {
            return None;
        }
        let text: Vec<String> = window.iter().map(|&i| self.tokens[i].word.clone()).collect();
        let mut best: Option<(usize, usize, i32, (u32, u32))> = None;

        for translation in self.bible.translations() {
            // Manuset siterer ofte litt mer enn referansen sier.
            let mut bible_words = Vec::new();
            let mut verse_of = Vec::new();
            for verse in from..=to + 3 {
                let id = VerseId { book: context.book.clone(), chapter: context.chapter, verse };
                let Some(verse_text) = self.bible.verse(translation, &id) else { break };
                for word in words(&verse_text) {
                    bible_words.push(word);
                    verse_of.push(verse);
                }
            }
            if bible_words.is_empty() {
                continue;
            }
            let Some(a) = align(&text, &bible_words) else { continue };
            if !is_good_alignment(&a, &text, bible_words.len(), direction) {
                continue;
            }
            if best.is_some_and(|(_, _, score, _)| score >= a.score) {
                continue;
            }
            best = Some((window[a.start], window[a.end], a.score, quoted_verses(&a.bible_matched, &verse_of)));
        }
        let (start, end, _, verses) = best?;
        let reference = self.bible.reference(&context.book, context.chapter, verses.0, verses.1)?;
        Some(Located { start, end, verses, reference })
    }

    fn add_quote(&mut self, found: Located, reference: Option<&Ref>, direction: Direction) {
        let first = self.tokens[found.start].clone();
        let last = self.tokens[found.end].clone();
        let boundary = match reference {
            Some(r) if direction != Direction::Backward && first.p == r.p => r.end,
            _ => 0,
        };
        let limit = reference.filter(|_| direction == Direction::Backward).map(|r| r.start);
        let mut quote = Quote {
            start_p: first.p,
            start_c: self.snap_start(&first, boundary),
            end_p: last.p,
            end_c: self.snap_end(&last, limit),
            reference: found.reference.clone(),
            source: reference.map_or_else(|| "søk".to_string(), |r| r.text.clone()),
            warning: warning_for(found.verses, reference, direction),
        };
        // Hele avsnittet etter referansen er sitatet.
        if direction == Direction::Alone && quote.end_p == quote.start_p {
            quote.end_c = quote.end_c.max(self.paras[quote.end_p].text.trim_end().len());
        }
        for p in quote.start_p..=quote.end_p {
            self.touched.insert(p);
        }
        let quote_start_p = quote.start_p;
        let quote_start_c = quote.start_c;
        let quote_end_c = quote.end_c;
        self.quotes.push(quote);
        let Some(reference) = reference else { return };
        self.touched.insert(reference.p);
        let text = self.paras[reference.p].text.clone();

        if direction == Direction::Backward {
            // "... rettferdiggjørelse” i Rom 4:25" - "i" og referansen fjernes.
            let between = slice(&text, quote_end_c, reference.start);
            if words(between).len() <= 2 {
                self.drops.push(Span { p: reference.p, start: quote_end_c, end: self.ref_end(reference) });
            }
            return;
        }
        if !self.is_right_before(reference, quote_start_p, quote_start_c) {
            return;
        }
        let before = text[..reference.start].trim();
        let ref_span = Span {
            p: reference.p,
            start: reference.start,
            end: if quote_start_p == reference.p { quote_start_c } else { text.len() },
        };
        if before.is_empty() {
            static ENDS_WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\p{L}0-9]$").unwrap());
            let dangling = reference.p > 0 && {
                let previous = &self.paras[reference.p - 1];
                previous.heading.is_none() && ENDS_WORD.is_match(previous.text.trim())
            };
            if dangling {
                self.join_previous.insert(reference.p);
            } else {
                self.drops.push(ref_span);
            }
        } else if before.ends_with(['.', '!', '?', ':', ';', ',']) {
            self.drops.push(ref_span);
        }
    }

    /// Står referansen rett foran sitatet, med bare tegnsetting og tomme avsnitt mellom?
    fn is_right_before(&self, reference: &Ref, start_p: usize, start_c: usize) -> bool {
        static ONLY_PUNCTUATION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[\s.,;:–-]*$").unwrap());
        let text = &self.paras[reference.p].text;
        if start_p == reference.p {
            return ONLY_PUNCTUATION.is_match(slice(text, reference.end, start_c));
        }
        let mut between = vec![&text[reference.end..]];
        between.extend(self.paras[reference.p + 1..start_p].iter().map(|para| para.text.as_str()));
        between.push(slice(&self.paras[start_p].text, 0, start_c));
        between.iter().all(|text| ONLY_PUNCTUATION.is_match(text))
    }

    fn ref_end(&self, reference: &Ref) -> usize {
        static AFTER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*\)?[.,;:]?").unwrap());
        let after = &self.paras[reference.p].text[reference.end..];
        reference.end + AFTER.find(after).map_or(0, |m| m.len())
    }

    /// Starter sitatet på setningen hvis bare et par ord mangler ("Men", "Og Herren").
    fn snap_start(&self, token: &Token, boundary: usize) -> usize {
        let text = &self.paras[token.p].text;
        let mut start = boundary;
        let is_digit = |c: Option<char>| c.is_some_and(|c| c.is_ascii_digit());
        for (offset, c) in slice(text, boundary, token.start).char_indices().rev() {
            let i = boundary + offset;
            let previous = text[..i].chars().next_back();
            let next = text[i + c.len_utf8()..].chars().next();
            let verse_punctuation = is_digit(previous) && is_digit(next);
            if (is_sentence_end(c) && !verse_punctuation) || is_open_quote(c) {
                start = i + c.len_utf8();
                break;
            }
        }
        // Versnumre foran ("og 6:19 Vet dere") hører ikke til sitatet.
        let skipped = words(slice(text, start, token.start));
        let at = if skipped.len() <= 2 && !skipped.iter().any(|w| is_number(w)) { start } else { token.start };
        let rest = &text[at..];
        at + (rest.len() - rest.trim_start().len())
    }

    /// Avslutter sitatet ved slutten av setningen hvis bare et par ord gjenstår, og tar med "(N78)".
    fn snap_end(&self, token: &Token, limit: Option<usize>) -> usize {
        static SENTENCE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[.!?]").unwrap());
        static TRAILING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^[.,;:!?»”"’']*"#).unwrap());
        static MARK: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^([^()]*?)\(\s*[A-Z]{1,4}\s?[0-9]{0,4}\s*\)").unwrap());
        let text = &self.paras[token.p].text;
        let stop = limit.unwrap_or(text.len());
        let mut end = token.end;
        let sentence = SENTENCE.find(slice(text, token.end, stop));
        // Bakover går sitatet ikke helt fram til referansen: "... rettferdiggjørelse” i Rom 4:25".
        let sentence_end = match sentence {
            Some(m) => token.end + m.start() + 1,
            None if limit.is_none() => stop,
            None => token.end,
        };
        if words(slice(text, token.end, sentence_end)).len() <= 4 {
            end = sentence_end;
        }
        end += TRAILING.find(slice(text, end, stop)).map_or(0, |m| m.len());
        // "(N78)" etter sitatet viser hvor det slutter.
        if let Some(caps) = MARK.captures(slice(text, end, stop)) {
            if words(&caps[1]).len() <= 25 {
                end += caps[0].len();
            }
        }
        end
    }

    fn is_inside_quote(&self, p: usize, at: usize) -> bool {
        self.quotes.iter().any(|q| {
            (p > q.start_p || (p == q.start_p && at >= q.start_c)) && (p < q.end_p || (p == q.end_p && at < q.end_c))
        })
    }

    /// "21:2 Manasse gjorde ..." der sitatet allerede er funnet fra en referanse før.
    fn is_quote_starting_after(&self, p: usize, at: usize) -> bool {
        self.quotes.iter().any(|q| {
            q.start_p == p && q.start_c >= at && !LETTER.is_match(slice(&self.paras[p].text, at, q.start_c))
        })
    }

    fn words_before(&self, p: usize, at: usize) -> usize {
        words(&self.paras[p].text[..at]).len()
    }

    // --- Søk etter vers uten referanse ---

    fn find_unreferenced_quotes(&mut self) {
        for p in 0..self.paras.len() {
            if self.paras[p].heading.is_some() || self.touched.contains(&p) {
                continue;
            }
            let window: Vec<usize> =
                (self.first_token[p]..self.first_token[p + 1]).filter(|&i| !is_number(&self.tokens[i].word)).collect();
            if window.len() < 6 || window.len() > 80 {
                continue;
            }
            let mut query: Vec<String> = Vec::new();
            for &i in &window {
                let word = &self.tokens[i].word;
                if !query.contains(word) {
                    query.push(word.clone());
                }
            }
            query.retain(|w| len16(w) >= 4);
            query.sort_by_key(|w| std::cmp::Reverse(len16(w)));
            query.truncate(5);
            if query.len() < 3 {
                continue;
            }

            'translations: for translation in self.bible.translations().to_vec() {
                for hit in self.bible.search(&translation, &query) {
                    let context = Context { book: hit.book.clone(), chapter: hit.chapter };
                    if let Some(found) = self.best_alignment(&context, hit.verse, hit.verse, &window, Direction::Search) {
                        self.add_quote(found, None, Direction::Search);
                        break 'translations;
                    }
                }
            }
        }
    }

    // --- Markdown ---

    fn render(&self) -> String {
        static WHITESPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());
        static OPEN_AT_END: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"\s*[«“„"]$"#).unwrap());
        static NUMBER_PUNCTUATION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"([0-9])\s*[,;:]$").unwrap());
        static COMMA_AT_END: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*[,;]$").unwrap());
        let mut blocks: Vec<String> = Vec::new();
        let mut p = 0;
        let mut pos = 0;
        while p < self.paras.len() {
            let para = &self.paras[p];
            if let Some(level) = para.heading {
                blocks.push(format!("{} {}", "#".repeat(level), WHITESPACE.replace_all(para.text.trim(), " ")));
                p += 1;
                continue;
            }
            let quote = self.quotes.iter().find(|q| q.start_p == p && q.start_c >= pos);
            let prose = self.render_prose(p, pos, quote.map_or(para.text.len(), |q| q.start_c));
            if !prose.is_empty() {
                if self.join_previous.contains(&p) && pos == 0 && !blocks.is_empty() {
                    let last = blocks.last_mut().unwrap();
                    last.push(' ');
                    last.push_str(&prose);
                } else {
                    blocks.push(if para.list_item && pos == 0 { format!("- {prose}") } else { prose });
                }
            }
            let Some(quote) = quote else {
                p += 1;
                pos = 0;
                continue;
            };
            // Teksten rett før sitatet leder inn til det: "han sier," blir "han sier:".
            if quote.source != "søk" {
                if let Some(last) = blocks.last_mut() {
                    let text = OPEN_AT_END.replace(last, "");
                    let text = NUMBER_PUNCTUATION.replace(&text, "$1");
                    *last = COMMA_AT_END.replace(&text, ":").into_owned();
                }
            }
            blocks.push(self.render_quote(quote));
            p = quote.end_p;
            pos = quote.end_c;
        }
        blocks.join("\n\n") + "\n"
    }

    fn render_prose(&self, p: usize, start: usize, end: usize) -> String {
        static NUMBERED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([0-9]+)\.(\s)").unwrap());
        static MARKER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([-+*>#])(\s)").unwrap());
        let drops: Vec<Span> = self.drops.iter().filter(|d| d.p == p).copied().collect();
        let text = clean(&render_runs(&self.paras[p].runs, start, end, &drops));
        if !LETTER.is_match(&text) {
            return String::new();
        }
        // Tekst som ellers ville blitt liste, overskrift eller sitat i markdown.
        let text = NUMBERED.replace(&text, "$1\\.$2");
        MARKER.replace(&text, "\\$1$2").into_owned()
    }

    fn render_quote(&self, quote: &Quote) -> String {
        static VERSE_NUMBER: LazyLock<FancyRegex> = LazyLock::new(|| fancy(r"^[0-9]+(?:[:,][0-9]+)?\s+(?=\S)"));
        static TRAILING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\s,;:–-]+$").unwrap());
        static OPEN_START: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^(\**)[«“„"]"#).unwrap());
        static CLOSE_END: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"[»”"](\**)$"#).unwrap());
        let mut parts = Vec::new();
        for p in quote.start_p..=quote.end_p {
            let start = if p == quote.start_p { quote.start_c } else { 0 };
            let end = if p == quote.end_p { quote.end_c } else { self.paras[p].text.len() };
            parts.push(render_runs(&self.paras[p].runs, start, end, &[]));
        }
        let joined = parts.join(" ");
        let cleaned = clean(&TRANSLATION_MARK.replace_all(&joined, ""));
        let without_number = VERSE_NUMBER.replace(&cleaned, "");
        let mut text = TRAILING.replace(&without_number, "").into_owned();
        // Anførselstegn rundt sitatet trengs ikke, og et som bare står i én ende, skal bort.
        let plain = text.replace('*', "");
        let mut chars = plain.chars();
        let first = chars.next();
        let rest: String = chars.collect();
        if first.is_some_and(is_open_quote) && !rest.replace('"', "").chars().any(is_close_quote) {
            text = OPEN_START.replace(&text, "$1").into_owned();
        }
        let last = plain.chars().next_back();
        let init: String = plain.chars().take(plain.chars().count().saturating_sub(1)).collect();
        if last.is_some_and(is_close_quote) && !init.replace('"', "").chars().any(is_open_quote) {
            text = CLOSE_END.replace(&text, "$1").into_owned();
        }
        let line = format!("> {} ({})", text.trim(), quote.reference);
        match &quote.warning {
            Some(warning) => format!("> [!warning|sjekk] {warning}\n{line}"),
            None => line,
        }
    }
}

fn is_good_alignment(a: &Alignment, text: &[String], bible_length: usize, direction: Direction) -> bool {
    let span = a.end - a.start + 1;
    let strong = a.text_matched.iter().filter(|&&i| len16(&text[i]) >= 4).count();
    let density = a.matches as f64 / span as f64;
    let short_verse = bible_length < 10;
    if direction == Direction::Alone {
        return a.matches >= 2;
    }
    let min_matches = match direction {
        Direction::Forward if short_verse => 4,
        Direction::Forward => 5,
        _ => 7,
    };
    if a.matches < min_matches || strong < 3 || density < 0.6 {
        return false;
    }
    match direction {
        Direction::Forward => a.start <= MAX_LEAD_IN,
        Direction::Backward => text.len() - 1 - a.end <= 4,
        _ => density >= 0.75 && span as f64 / text.len() as f64 >= 0.8,
    }
}

/// Hvorfor et sitat bør sjekkes, eller `None` hvis gjenkjenningen er sikker.
fn warning_for(verses: (u32, u32), reference: Option<&Ref>, direction: Direction) -> Option<String> {
    let Some(reference) = reference else {
        return Some("Funnet med søk i Bibelen. Manuset har ingen referanse.".to_string());
    };
    match direction {
        Direction::Alone => Some(format!("Få ord stemmer med bibelteksten. Sjekk at dette er {}.", reference.text)),
        Direction::Backward => {
            Some(format!("Skilt ut fra setningen foran {}. Sjekk hvor sitatet starter.", reference.text))
        }
        // At manuset siterer flere vers enn referansen sier, er vanlig. At det starter et annet sted, er det ikke.
        _ if verses.0 != reference.from => Some(format!("Manuset viser til «{}». Sjekk versene.", reference.text)),
        _ => None,
    }
}

/// Versene som er sitert: de der en god del av ordene er med.
fn quoted_verses(matched: &[usize], verse_of: &[u32]) -> (u32, u32) {
    let mut length: HashMap<u32, usize> = HashMap::new();
    for &verse in verse_of {
        *length.entry(verse).or_default() += 1;
    }
    let mut order: Vec<u32> = Vec::new();
    let mut hits: HashMap<u32, usize> = HashMap::new();
    for &i in matched {
        let verse = verse_of[i];
        let count = hits.entry(verse).or_insert_with(|| {
            order.push(verse);
            0
        });
        *count += 1;
    }
    let verses: Vec<u32> = order
        .iter()
        .copied()
        .filter(|verse| {
            let count = hits[verse];
            let len = length[verse];
            count >= len.min(3) || count as f64 >= len as f64 * 0.3
        })
        .collect();
    let all = if verses.is_empty() { order } else { verses };
    (*all.iter().min().unwrap_or(&0), *all.iter().max().unwrap_or(&0))
}

/// Teksten fra `start` til `end` med fet og kursiv som markdown, uten `drops`.
fn render_runs(runs: &[Run], start: usize, end: usize, drops: &[Span]) -> String {
    static PIECE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^([\s\p{P}]*)([\s\S]*?)([\s\p{P}]*)$").unwrap());
    static HAS_WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\p{L}\p{N}]").unwrap());
    let mut pieces: Vec<Run> = Vec::new();
    let mut at = 0;
    for run in runs {
        let run_start = at;
        at += run.text.len();
        for (offset, c) in run.text.char_indices() {
            let i = run_start + offset;
            if i < start || i >= end || drops.iter().any(|d| i >= d.start && i < d.end) {
                continue;
            }
            match pieces.last_mut() {
                Some(last) if last.bold == run.bold && last.italic == run.italic => last.text.push(c),
                _ => pieces.push(Run { text: c.to_string(), bold: run.bold, italic: run.italic }),
            }
        }
    }
    pieces
        .iter()
        .map(|piece| {
            let marker = match (piece.bold, piece.italic) {
                (true, true) => "***",
                (true, false) => "**",
                (false, true) => "*",
                _ => "",
            };
            // Mellomrom og tegnsetting utenfor markørene, ellers virker ikke "barn**, og**".
            let caps = PIECE.captures(&piece.text).unwrap();
            if marker.is_empty() || !HAS_WORD.is_match(&caps[2]) {
                return piece.text.clone();
            }
            format!("{}{marker}{}{marker}{}", &caps[1], &caps[2], &caps[3])
        })
        .collect()
}

fn clean(text: &str) -> String {
    static NEWLINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[ \t]*\n[ \t]*").unwrap());
    static SPACES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[ \t]+").unwrap());
    let text = text.replace("****", "");
    let text = NEWLINE.replace_all(&text, "\n");
    SPACES.replace_all(&text, " ").trim().to_string()
}
