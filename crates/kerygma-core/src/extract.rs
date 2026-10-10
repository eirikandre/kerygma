//! Lager innholdet til en presentasjon av et manus.
//!
//! Skrevet etter `src/extract.ts` i obsidian-slide-creator, og gir de samme
//! utdataene tegn for tegn. Lengder telles i UTF-16-enheter, som `length` i
//! JavaScript, så delingen av lange vers blir lik. Indekser i strengene er
//! byte-indekser, og regnes om til UTF-16 der de sammenlignes med lengder.

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

use crate::reference::{find_bible_references, reference_locus};

static HEADING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^#{1,6}\s").unwrap());
static FENCE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*(```|~~~)").unwrap());
static LIST_ITEM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*(?:[-*+]|[0-9]+[.)])\s+").unwrap());
static BLOCKQUOTE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*>").unwrap());
/// `![[bilde.png|300]]` (bare bildefiler, ikke andre notater) og `![alt](bilde.png)`.
static IMAGE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)!\[\[[^\]|#]+\.(?:png|jpe?g|gif|svg|webp|bmp|avif)(?:[|#][^\]]*)?\]\]|!\[[^\]]*\]\([^)]+\)")
        .unwrap()
});

pub const DEFAULT_MAX_CHARS: usize = 350;
/// Kolonnene er halvparten så brede, men får mindre skrift (se `slides.css`).
const COLUMN_SHARE: f64 = 0.6;

#[derive(Debug, Clone, Default)]
pub struct SlideOptions {
    /// Maks antall tegn bibeltekst per slide før verset deles over flere.
    pub max_chars: Option<usize>,
    /// Utdata fra `bibel` ("tekst (ref)") per referanse i manuset. Settes ved
    /// siden av vers som står alene, som en ekstra kolonne.
    pub translations: HashMap<String, String>,
}

/// En overskrift, et bilde, et sitat som ikke er bibeltekst, eller et vers i én eller flere oversettelser.
#[derive(Debug, Clone)]
enum Item {
    Heading(String),
    Image(String),
    Quote(String),
    Verses(Vec<Verse>),
}

#[derive(Debug, Clone)]
struct Verse {
    text: String,
    reference: String,
}

/// Lengden som `length` i JavaScript.
pub(crate) fn len16(text: &str) -> usize {
    text.encode_utf16().count()
}

fn ceil_div(len: usize, max: usize) -> usize {
    (len as f64 / max as f64).ceil() as usize
}

/// Lager innholdet til presentasjonen: beholder frontmatter, overskrifter og
/// bibelvers, sitater og bilder. Alt annet fjernes. Hver overskrift, hvert vers
/// og hvert bilde blir en egen slide, skilt med "---".
pub fn extract_slide_content(markdown: &str, options: &SlideOptions) -> String {
    let max_chars = options.max_chars.unwrap_or(DEFAULT_MAX_CHARS);
    let (frontmatter, items) = parse(markdown);
    let slides: Vec<String> = items
        .into_iter()
        .flat_map(|item| match item {
            Item::Heading(line) => vec![line],
            Item::Image(image) => vec![image],
            Item::Quote(quote) => {
                let parts = ceil_div(len16(&quote), max_chars);
                balance_emphasis(&split_text(&quote, parts))
            }
            Item::Verses(group) => verse_slides(&with_translation(group, &options.translations), max_chars),
        })
        .collect();
    format!("{frontmatter}{}\n", slides.join("\n\n---\n\n"))
}

/// Referansene til vers i manuset som bare finnes i én oversettelse.
pub fn lone_verse_references(markdown: &str) -> Vec<String> {
    parse(markdown)
        .1
        .into_iter()
        .filter_map(|item| match item {
            Item::Verses(mut group) if group.len() == 1 => Some(group.remove(0).reference),
            _ => None,
        })
        .collect()
}

/// Deler på `\r?\n`, som `split(/\r?\n/)`.
fn split_lines(markdown: &str) -> Vec<&str> {
    markdown.split('\n').map(|line| line.strip_suffix('\r').unwrap_or(line)).collect()
}

fn parse(markdown: &str) -> (String, Vec<Item>) {
    let lines = split_lines(markdown);
    let mut items = Vec::new();
    let mut frontmatter = String::new();
    let mut i = 0;

    if lines[0] == "---" {
        if let Some(end) = lines.iter().skip(1).position(|line| *line == "---").map(|p| p + 1) {
            frontmatter = lines[..=end].join("\n") + "\n\n";
            i = end + 1;
        }
    }

    let mut block: Vec<&str> = Vec::new();
    let flush = |block: &mut Vec<&str>, items: &mut Vec<Item>| {
        if !block.is_empty() {
            items.extend(verses_in_block(block));
        }
        block.clear();
    };

    while i < lines.len() {
        let line = lines[i];

        if FENCE.is_match(line) {
            flush(&mut block, &mut items);
            let fence: String = line.trim().chars().take(3).collect();
            i += 1;
            while i < lines.len() && !lines[i].trim().starts_with(&fence) {
                i += 1;
            }
            i += 1;
            continue;
        }
        if line.trim().starts_with("%%") {
            flush(&mut block, &mut items);
            if !line.trim()[2..].contains("%%") {
                i += 1;
                while i < lines.len() && !lines[i].contains("%%") {
                    i += 1;
                }
            }
            i += 1;
            continue;
        }
        if HEADING.is_match(line) {
            flush(&mut block, &mut items);
            items.push(Item::Heading(line.to_string()));
            i += 1;
            continue;
        }
        if line.trim().is_empty() {
            flush(&mut block, &mut items);
            i += 1;
            continue;
        }
        let first = block.first().copied().unwrap_or(line);
        let starts_new_block = LIST_ITEM.is_match(line) || BLOCKQUOTE.is_match(line) != BLOCKQUOTE.is_match(first);
        if starts_new_block {
            flush(&mut block, &mut items);
        }
        block.push(line);
        i += 1;
    }
    flush(&mut block, &mut items);

    (frontmatter, items)
}

static QUOTE_MARKER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*>\s?").unwrap());
static CHECK_CALLOUT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\[![^\]|]*\|sjekk\][+-]?[^\n]*\n?").unwrap());
static CALLOUT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\[![^\]]*\][+-]?\s*").unwrap());

/// Finner bildene og bibelversene i en blokk (avsnitt, listepunkt eller sitat),
/// eller returnerer sitatet uten `>` hvis det ikke er bibeltekst. Bildene
/// kommer i rekkefølge med teksten rundt, som behandles hver for seg. Et vers
/// er teksten fra starten av blokken, eller fra forrige referanse, til og med
/// referansen. Samme vers i flere oversettelser (`bibel ne:joh3:16`) samles,
/// så de kan settes side om side.
fn verses_in_block(block: &[&str]) -> Vec<Item> {
    let text = block.join("\n");
    let is_quote = BLOCKQUOTE.is_match(block[0]);
    // `>` (og `[!type]` i callouts) viser bare i manuset at det er et sitat.
    // `[!warning|sjekk] Hvorfor` merker sitater som bør sjekkes etter import fra
    // Word; tittelen er en beskjed til den som skriver, og blir ikke med.
    let body = if is_quote {
        let joined = block.iter().map(|line| QUOTE_MARKER.replace(line, "")).collect::<Vec<_>>().join("\n");
        let without_check = CHECK_CALLOUT.replace(&joined, "");
        CALLOUT.replace(&without_check, "").trim().to_string()
    } else {
        LIST_ITEM.replace(&text, "").into_owned()
    };

    let mut items = Vec::new();
    let mut previous_end = 0;
    for m in IMAGE.find_iter(&body) {
        items.extend(verses_in_text(body[previous_end..m.start()].trim(), is_quote));
        items.push(Item::Image(m.as_str().to_string()));
        previous_end = m.end();
    }
    items.extend(verses_in_text(body[previous_end..].trim(), is_quote));
    items
}

static LEADING_PUNCTUATION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[\s.,;:!?–-]+").unwrap());

fn verses_in_text(body: &str, is_quote: bool) -> Vec<Item> {
    let references = find_bible_references(body);

    // Sitater blir alltid slides. Er de bibeltekst, behandles de som andre vers.
    if is_quote && references.is_empty() {
        return if body.is_empty() { vec![] } else { vec![Item::Quote(body.to_string())] };
    }

    let mut groups: Vec<Vec<Verse>> = Vec::new();
    let mut previous_end = 0;
    let mut previous_locus: Option<String> = None;
    for reference in references {
        let verse = Verse {
            text: LEADING_PUNCTUATION.replace(&body[previous_end..reference.start], "").trim().to_string(),
            reference: reference.reference.clone(),
        };
        let locus = reference_locus(&reference.reference);
        match groups.last_mut() {
            Some(group) if previous_locus.as_ref() == Some(&locus) => group.push(verse),
            _ => groups.push(vec![verse]),
        }
        previous_end = reference.end;
        previous_locus = Some(locus);
    }
    groups.into_iter().map(Item::Verses).collect()
}

fn with_translation(group: Vec<Verse>, translations: &HashMap<String, String>) -> Vec<Verse> {
    if group.len() != 1 {
        return group;
    }
    let Some(translated) = translations.get(&group[0].reference) else { return group };
    let Some(reference) = find_bible_references(translated).pop() else { return group };
    let mut group = group;
    group.push(Verse { text: translated[..reference.start].trim().to_string(), reference: reference.reference });
    group
}

// Omkranser versnumre i teksten mens den deles, så de kan settes som <sup>.
const OPEN: char = '\u{E000}';
const CLOSE: char = '\u{E001}';
static MARKED_NUMBER: LazyLock<Regex> = LazyLock::new(|| Regex::new("\u{E000}([^\u{E001}]*)\u{E001}").unwrap());

/// Ett vers blir én eller flere slides; flere oversettelser blir kolonner.
fn verse_slides(translations: &[Verse], max_chars: usize) -> Vec<String> {
    let render = |text: &str, verse: &Verse, html: bool| {
        // Markdown virker ikke inne i HTML, så fet og kursiv blir <strong> og <em> i kolonnene.
        let text = if html { emphasis_to_html(&escape_html(text)) } else { text.to_string() };
        let body = MARKED_NUMBER.replace_all(&text, "<sup>$1</sup>");
        if body.is_empty() {
            format!("({})", verse.reference)
        } else {
            format!("{body} ({})", verse.reference)
        }
    };
    let texts: Vec<String> = translations.iter().map(|verse| mark_verse_numbers(&verse.text)).collect();

    if translations.len() == 1 {
        let parts = ceil_div(len16(&translations[0].text), max_chars);
        return balance_emphasis(&split_text(&texts[0], parts))
            .iter()
            .map(|part| render(part, &translations[0], false))
            .collect();
    }

    let column_chars = (max_chars as f64 * COLUMN_SHARE).round() as usize;
    let parts = translations.iter().map(|verse| ceil_div(len16(&verse.text), column_chars)).max().unwrap_or(0);
    let columns: Vec<Vec<String>> = split_parallel(&texts, parts).iter().map(|c| balance_emphasis(c)).collect();
    (0..parts.max(1))
        .map(|i| {
            let mut lines = vec![r#"<div class="bibel-parallell">"#.to_string()];
            for (column, verse) in translations.iter().enumerate() {
                lines.push(format!("<p>{}</p>", render(&columns[column][i], verse, true)));
            }
            lines.push("</div>".to_string());
            lines.join("\n")
        })
        .collect()
}

static VERSE_NUMBER: LazyLock<fancy_regex::Regex> =
    LazyLock::new(|| fancy_regex::Regex::new(r"(^|\s)(?:([0-9]+):)?([0-9]+)(?:-([0-9]+))?(?=\s)").unwrap());

/// Finner versnumrene i tekst fra `bibel` ("1 Herren er ... 2 Han lar ..." og
/// "1:27 ... 2:1 ...") og omkranser dem med OPEN/CLOSE. Teksten må starte med
/// et versnummer, og numrene må komme i rekkefølge, så andre tall i teksten
/// ikke tas for versnumre.
pub fn mark_verse_numbers(text: &str) -> String {
    let number = |s: &str| s.parse::<u64>().unwrap_or(u64::MAX);
    let mut last_chapter: Option<u64> = None;
    let mut last_verse: Option<u64> = None;
    let mut result = String::with_capacity(text.len() + 16);
    let mut copied = 0;
    for caps in VERSE_NUMBER.captures_iter(text) {
        let caps = caps.expect("versnummer-regex");
        let whole = caps.get(0).unwrap();
        let space = caps.get(1).map_or("", |m| m.as_str());
        let chapter = caps.get(2).map(|m| number(m.as_str()));
        let verse = number(caps.get(3).unwrap().as_str());
        let end = caps.get(4).map(|m| number(m.as_str()));

        let c = chapter.or(last_chapter);
        let is_first = last_verse.is_none() && space.is_empty();
        let is_next = match last_verse {
            Some(last) if c == last_chapter => verse == last.saturating_add(1),
            Some(_) => verse == 1,
            None => false,
        };
        if !is_first && !is_next {
            continue;
        }
        last_chapter = c;
        last_verse = Some(end.unwrap_or(verse));
        result.push_str(&text[copied..whole.start()]);
        result.push_str(space);
        result.push(OPEN);
        result.push_str(&whole.as_str()[space.len()..]);
        result.push(CLOSE);
        copied = whole.end();
    }
    result.push_str(&text[copied..]);
    result
}

/// Byte-indekser og UTF-16-indekser for de samme posisjonene i en tekst.
struct Positions {
    /// UTF-16-indeksen til hver byte-indeks som er en tegngrense.
    utf16: HashMap<usize, usize>,
    len: usize,
}

impl Positions {
    fn new(text: &str) -> Self {
        let mut utf16 = HashMap::new();
        let mut at = 0;
        for (byte, c) in text.char_indices() {
            utf16.insert(byte, at);
            at += c.len_utf16();
        }
        utf16.insert(text.len(), at);
        Positions { utf16, len: at }
    }

    fn of(&self, byte: usize) -> usize {
        self.utf16[&byte]
    }
}

static WHITESPACE_RUN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());
static ENDS_WITH_NUMBER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("(?:^|\\s)\u{E000}?[0-9:-]+\u{E001}?$").unwrap());
static ENDS_SENTENCE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"[.!?»”"]$"#).unwrap());

/// Et sted teksten kan deles: byte-indeks og UTF-16-indeks.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Break {
    byte: usize,
    at: usize,
}

/// Deler teksten i `parts` omtrent like lange deler. Deler helst mellom vers,
/// så mellom setninger og til slutt mellom ord, men aldri rett etter et
/// versnummer.
pub fn split_text(text: &str, parts: usize) -> Vec<String> {
    if parts <= 1 {
        return vec![text.to_string()];
    }
    let positions = Positions::new(text);

    let verse_breaks: Vec<Break> =
        verse_markers(text).into_iter().map(|(_, byte)| Break { byte, at: positions.of(byte) }).collect();
    let mut sentence_breaks = Vec::new();
    let mut word_breaks = Vec::new();
    for m in WHITESPACE_RUN.find_iter(text) {
        let before = &text[..m.start()];
        if ENDS_WITH_NUMBER.is_match(before) {
            continue;
        }
        let at = Break { byte: m.end(), at: positions.of(m.end()) };
        word_breaks.push(at);
        if ENDS_SENTENCE.is_match(before) {
            sentence_breaks.push(at);
        }
    }

    let part_length = positions.len as f64 / parts as f64;
    let mut cuts: Vec<Break> = Vec::new();
    for k in 1..parts {
        let target = k as f64 * part_length;
        let previous = cuts.last().map_or(0, |cut| cut.at);
        let nearest = |breaks: &[Break]| {
            closest(breaks.iter().copied().filter(|b| b.at > previous), |b| (b.at as f64 - target).abs())
        };
        let near = |b: Option<Break>| b.is_some_and(|b| (b.at as f64 - target).abs() <= part_length / 4.0);
        let verse = nearest(&verse_breaks);
        let sentence = nearest(&sentence_breaks);
        let cut = if near(verse) {
            verse
        } else if near(sentence) {
            sentence
        } else {
            nearest(&word_breaks)
        };
        if let Some(cut) = cut {
            cuts.push(cut);
        }
    }
    cut_at(text, &cuts.iter().map(|cut| cut.byte).collect::<Vec<_>>(), parts)
}

/// Deler flere oversettelser av samme tekst i like mange deler. Har alle
/// versnumre, deles de ved samme vers, så kolonnene holder takt.
fn split_parallel(texts: &[String], parts: usize) -> Vec<Vec<String>> {
    if parts <= 1 {
        return texts.iter().map(|text| vec![text.clone()]).collect();
    }

    let markers: Vec<Vec<(String, usize)>> = texts.iter().map(|text| verse_markers(text)).collect();
    let get = |m: &[(String, usize)], verse: &str| m.iter().find(|(key, _)| key == verse).map(|(_, byte)| *byte);
    let common: Vec<&String> =
        markers[0].iter().map(|(key, _)| key).filter(|verse| markers.iter().all(|m| get(m, verse).is_some())).collect();
    if common.len() + 1 < parts {
        return texts.iter().map(|text| split_text(text, parts)).collect();
    }

    let positions: Vec<Positions> = texts.iter().map(|text| Positions::new(text)).collect();
    // Hvor langt ut i teksten verset starter, i snitt over oversettelsene (0-1).
    let share = |verse: &str| {
        markers.iter().enumerate().fold(0.0, |sum, (i, m)| {
            sum + positions[i].of(get(m, verse).unwrap()) as f64 / positions[i].len as f64
        }) / texts.len() as f64
    };
    let mut chosen: Vec<usize> = Vec::new();
    for k in 1..parts {
        let after = chosen.last().copied();
        let candidates = (0..common.len()).filter(|i| after.is_none_or(|last| *i > last));
        let target = k as f64 / parts as f64;
        if let Some(index) = closest(candidates, |i| (share(common[i]) - target).abs()) {
            chosen.push(index);
        }
    }
    texts
        .iter()
        .zip(&markers)
        .map(|(text, m)| {
            let cuts: Vec<usize> = chosen.iter().map(|&index| get(m, common[index]).unwrap()).collect();
            cut_at(text, &cuts, parts)
        })
        .collect()
}

/// Hvor hvert vers starter (byte-indeks) i tekst merket av [`mark_verse_numbers`],
/// med versnummeret ("2" eller "2:1") som nøkkel, i rekkefølge. Det første
/// verset tas ikke med, siden teksten ikke kan deles der. Som en `Map` i
/// JavaScript beholder et nummer som kommer igjen, plassen sin, men får den
/// nye posisjonen.
fn verse_markers(text: &str) -> Vec<(String, usize)> {
    let mut markers: Vec<(String, usize)> = Vec::new();
    for caps in MARKED_NUMBER.captures_iter(text) {
        let start = caps.get(0).unwrap().start();
        if start == 0 {
            continue;
        }
        let key = &caps[1];
        match markers.iter_mut().find(|(k, _)| k == key) {
            Some(entry) => entry.1 = start,
            None => markers.push((key.to_string(), start)),
        }
    }
    markers
}

/// Den første verdien med minst avstand.
fn closest<T: Copy>(values: impl IntoIterator<Item = T>, distance: impl Fn(T) -> f64) -> Option<T> {
    let mut best: Option<(T, f64)> = None;
    for value in values {
        let d = distance(value);
        if best.is_none_or(|(_, best_d)| d < best_d) {
            best = Some((value, d));
        }
    }
    best.map(|(value, _)| value)
}

fn cut_at(text: &str, cuts: &[usize], parts: usize) -> Vec<String> {
    let mut bounds = vec![0];
    bounds.extend_from_slice(cuts);
    bounds.push(text.len());
    let mut chunks: Vec<String> = bounds.windows(2).map(|w| text[w[0]..w[1]].trim().to_string()).collect();
    while chunks.len() < parts {
        chunks.push(String::new());
    }
    chunks
}

/// Stjerner, `~~` og `<u>`/`</u>`: det `balance_emphasis` må holde rede på.
static MARKERS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\*{1,3}|~~|</?u>").unwrap());

/// Når teksten deles midt i fet, kursiv, overstreket eller understreket tekst,
/// lukkes den på slutten av delen og åpnes igjen i neste, så "**a b**" delt i
/// to blir "**a**" og "**b**".
pub fn balance_emphasis(parts: &[String]) -> Vec<String> {
    let mut bold = false;
    let mut italic = false;
    let mut strike = false;
    let mut underline = false;
    let on = |flag: bool, marker: &'static str| if flag { marker } else { "" };
    parts
        .iter()
        .map(|part| {
            let open = [on(bold, "**"), on(italic, "*"), on(strike, "~~"), on(underline, "<u>")].concat();
            for m in MARKERS.find_iter(part) {
                match m.as_str() {
                    "~~" => strike = !strike,
                    "<u>" => underline = true,
                    "</u>" => underline = false,
                    stars => {
                        if stars.len() != 1 {
                            bold = !bold;
                        }
                        if stars.len() != 2 {
                            italic = !italic;
                        }
                    }
                }
            }
            let close = [on(underline, "</u>"), on(strike, "~~"), on(italic, "*"), on(bold, "**")].concat();
            if part.is_empty() {
                String::new()
            } else {
                format!("{open}{part}{close}")
            }
        })
        .collect()
}

static BOLD_ITALIC: LazyLock<fancy_regex::Regex> =
    LazyLock::new(|| fancy_regex::Regex::new(r"\*\*\*(?=\S)([\s\S]*?\S)\*\*\*").unwrap());
static BOLD: LazyLock<fancy_regex::Regex> =
    LazyLock::new(|| fancy_regex::Regex::new(r"(\*\*|__)(?=\S)([\s\S]*?\S)\1").unwrap());
static ITALIC: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
    fancy_regex::Regex::new(r"(?<![\p{L}\p{N}])([*_])(?=\S)([\s\S]*?\S)\1(?![\p{L}\p{N}])").unwrap()
});
static HIGHLIGHT: LazyLock<fancy_regex::Regex> =
    LazyLock::new(|| fancy_regex::Regex::new(r"==(?=\S)([\s\S]*?\S)==").unwrap());
static STRIKETHROUGH: LazyLock<fancy_regex::Regex> =
    LazyLock::new(|| fancy_regex::Regex::new(r"~~(?=\S)([\s\S]*?\S)~~").unwrap());
/// `<u>` etter `escape_html`. Understreking finnes ikke i markdown, så HTML-taggen brukes (som i Obsidian).
static UNDERLINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"&lt;u&gt;([\s\S]*?)&lt;/u&gt;").unwrap());

/// Fet, kursiv, utheving, overstreket og understreket fra markdown som HTML:
/// "**a**" blir "<strong>a</strong>". `text` er escapet med `escape_html`.
pub fn emphasis_to_html(text: &str) -> String {
    let text = BOLD_ITALIC.replace_all(text, "<strong><em>$1</em></strong>");
    let text = BOLD.replace_all(&text, "<strong>$2</strong>");
    let text = ITALIC.replace_all(&text, "<em>$2</em>");
    let text = HIGHLIGHT.replace_all(&text, "<mark>$1</mark>");
    let text = STRIKETHROUGH.replace_all(&text, "<del>$1</del>");
    UNDERLINE.replace_all(&text, "<u>$1</u>").into_owned()
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn overstreket_og_understreket_blir_html_i_kolonnene() {
        assert_eq!(
            emphasis_to_html(&escape_html("~~a b~~ &lt;u&gt; <u>c</u> 1 < 2")),
            "<del>a b</del> &amp;lt;u&amp;gt; <u>c</u> 1 &lt; 2"
        );
    }

    #[test]
    fn overstreket_og_understreket_som_deles_lukkes_og_apnes_igjen() {
        assert_eq!(
            balance_emphasis(&strings(&["a ~~b <u>c", "d</u> e~~ f"])),
            strings(&["a ~~b <u>c</u>~~", "~~<u>d</u> e~~ f"])
        );
        assert_eq!(
            balance_emphasis(&strings(&["**a <u>b", "c</u> d**"])),
            strings(&["**a <u>b</u>**", "**<u>c</u> d**"])
        );
    }
}
