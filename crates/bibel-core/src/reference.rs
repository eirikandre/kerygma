//! Tolker en referanse som «1sam2:1-3» og slår den opp i datasettet.
//!
//! Grammatikk:
//!
//! ```text
//! ref  := bok kapittel [ (':'|',') vers ]
//! vers := n                    ett vers         1sam1:3
//!       | n '-' n              intervall        1sam2:1-3
//!       | n '-' k (':'|',') n  kryss-kapittel   1sam1:20-2:5
//! (uten versdel)               hele kapitlet    1sam2
//! ```

use std::fmt;

use bibel_format::{books, Bible, Book, Verse};

/// Skrivemåter som ikke dekkes av OSIS- eller USFM-id-en. Nøkkelen sammenlignes
/// foldet, verdien er en osisId.
///
/// Tabellen gjelder for alle oversettelser, ikke bare den forkortelsen kommer fra:
/// hver oversettelse kjenner bare sine egne forkortelser og navn, så uten den
/// ville «ne:1kor13:1» feile i KJV og «nb88:genesis1:1» feile i NB88. Derfor står
/// forkortelsene og boknavnene fra NB88, Bibel 2024 og KJV her, i tillegg til
/// vanlige kortformer.
pub const ALIASES: &[(&str, &str)] = &[
    ("1mosebok", "Gen"),
    ("2mosebok", "Exod"),
    ("3mosebok", "Lev"),
    ("4mosebok", "Num"),
    ("5mosebok", "Deut"),
    ("josva", "Josh"),
    ("dommerne", "Judg"),
    ("ruts", "Ruth"),
    ("1samuel", "1Sam"),
    ("2samuel", "2Sam"),
    ("1kongebok", "1Kgs"),
    ("2kongebok", "2Kgs"),
    ("1krønikebok", "1Chr"),
    ("2krønikebok", "2Chr"),
    ("nehemja", "Neh"),
    ("ester", "Esth"),
    ("jobs", "Job"),
    ("salme", "Ps"),
    ("salmer", "Ps"),
    ("salmene", "Ps"),
    ("salmenes", "Ps"),
    ("ordspråkene", "Prov"),
    ("ordspr", "Prov"),
    ("pred", "Eccl"),
    ("predikeren", "Eccl"),
    ("høysangen", "Song"),
    ("sang", "Song"),
    ("jesaja", "Isa"),
    ("jeremia", "Jer"),
    ("klagesangene", "Lam"),
    ("esekiel", "Ezek"),
    ("daniel", "Dan"),
    ("hoseas", "Hos"),
    ("obadja", "Obad"),
    ("jona", "Jonah"),
    ("sefanja", "Zeph"),
    ("haggai", "Hag"),
    ("sakarja", "Zech"),
    ("malaki", "Mal"),
    ("matteus", "Matt"),
    ("markus", "Mark"),
    ("lukas", "Luke"),
    ("johannes", "John"),
    ("apgj", "Acts"),
    ("apostlenes", "Acts"),
    ("romerne", "Rom"),
    ("romerbrevet", "Rom"),
    ("1korinter", "1Cor"),
    ("2korinter", "2Cor"),
    ("galaterne", "Gal"),
    ("efeserne", "Eph"),
    ("filipperne", "Phil"),
    ("kolosserne", "Col"),
    ("1tessaloniker", "1Thess"),
    ("2tessaloniker", "2Thess"),
    ("1timoteus", "1Tim"),
    ("2timoteus", "2Tim"),
    ("titus", "Titus"),
    ("filemon", "Phlm"),
    ("hebreerne", "Heb"),
    ("hebreerbrevet", "Heb"),
    ("jakob", "Jas"),
    ("1peter", "1Pet"),
    ("2peter", "2Pet"),
    ("1johannes", "1John"),
    ("2johannes", "2John"),
    ("3johannes", "3John"),
    ("judas", "Jude"),
    ("åp", "Rev"),
    ("åpb", "Rev"),
    ("åpenbaringen", "Rev"),
    // Forkortelser og navn fra NB88 og Bibel 2024.
    ("1mos", "Gen"),
    ("førstemosebok", "Gen"),
    ("2mos", "Exod"),
    ("andremosebok", "Exod"),
    ("3mos", "Lev"),
    ("tredjemosebok", "Lev"),
    ("4mos", "Num"),
    ("fjerdemosebok", "Num"),
    ("5mos", "Deut"),
    ("femtemosebok", "Deut"),
    ("josvasbok", "Josh"),
    ("dom", "Judg"),
    ("dommernesbok", "Judg"),
    ("rutsbok", "Ruth"),
    ("1samuelsbok", "1Sam"),
    ("førstesamuelsbok", "1Sam"),
    ("2samuelsbok", "2Sam"),
    ("andresamuelsbok", "2Sam"),
    ("1kong", "1Kgs"),
    ("førstekongebok", "1Kgs"),
    ("2kong", "2Kgs"),
    ("andrekongebok", "2Kgs"),
    ("1kron", "1Chr"),
    ("1krøn", "1Chr"),
    ("førstekrønikebok", "1Chr"),
    ("2kron", "2Chr"),
    ("2krøn", "2Chr"),
    ("andrekrønikebok", "2Chr"),
    ("esr", "Ezra"),
    ("esrasbok", "Ezra"),
    ("esra", "Ezra"),
    ("nehemjasbok", "Neh"),
    ("estersbok", "Esth"),
    ("jobsbok", "Job"),
    ("sal", "Ps"),
    ("salmenesbok", "Ps"),
    ("ord", "Prov"),
    ("salomosordspråk", "Prov"),
    ("ordsp", "Prov"),
    ("fork", "Eccl"),
    ("predikantensbok", "Eccl"),
    ("forkynneren", "Eccl"),
    ("hoys", "Song"),
    ("salomoshøysang", "Song"),
    ("høys", "Song"),
    ("jes", "Isa"),
    ("profetenjesaja", "Isa"),
    ("profetenjeremia", "Jer"),
    ("klag", "Lam"),
    ("esek", "Ezek"),
    ("profetenesekiel", "Ezek"),
    ("profetendaniel", "Dan"),
    ("profetenhosea", "Hos"),
    ("hosea", "Hos"),
    ("profetenjoel", "Joel"),
    ("profetenamos", "Amos"),
    ("am", "Amos"),
    ("profetenobadja", "Obad"),
    ("ob", "Obad"),
    ("profetenjonas", "Jonah"),
    ("mika", "Mic"),
    ("profetenmika", "Mic"),
    ("mi", "Mic"),
    ("profetennahum", "Nah"),
    ("nahum", "Nah"),
    ("profetenhabakkuk", "Hab"),
    ("habakkuk", "Hab"),
    ("sef", "Zeph"),
    ("profetensefanja", "Zeph"),
    ("profetenhaggai", "Hag"),
    ("sak", "Zech"),
    ("profetensakarias", "Zech"),
    ("profetenmalakias", "Mal"),
    ("matteusevangelium", "Matt"),
    ("evangelietettermatteus", "Matt"),
    ("markusevangelium", "Mark"),
    ("evangelietettermarkus", "Mark"),
    ("lukasevangelium", "Luke"),
    ("evangelietetterlukas", "Luke"),
    ("joh", "John"),
    ("johannesevangelium", "John"),
    ("evangelietetterjohannes", "John"),
    ("apg", "Acts"),
    ("apostlenesgjerninger", "Acts"),
    ("paulusbrevtilromerne", "Rom"),
    ("1kor", "1Cor"),
    ("1korinterbrev", "1Cor"),
    ("paulusførstebrevtilkorinterne", "1Cor"),
    ("2kor", "2Cor"),
    ("2korinterbrev", "2Cor"),
    ("paulusandrebrevtilkorinterne", "2Cor"),
    ("galaterbrevet", "Gal"),
    ("paulusbrevtilgalaterne", "Gal"),
    ("ef", "Eph"),
    ("efeserbrevet", "Eph"),
    ("paulusbrevtilefeserne", "Eph"),
    ("fil", "Phil"),
    ("filipperbrevet", "Phil"),
    ("paulusbrevtilfilipperne", "Phil"),
    ("kol", "Col"),
    ("kolosserbrevet", "Col"),
    ("paulusbrevtilkolosserne", "Col"),
    ("1tess", "1Thess"),
    ("1tessalonikerbrev", "1Thess"),
    ("paulusførstebrevtiltessalonikerne", "1Thess"),
    ("2tess", "2Thess"),
    ("2tessalonikerbrev", "2Thess"),
    ("paulusandrebrevtiltessalonikerne", "2Thess"),
    ("1timoteusbrev", "1Tim"),
    ("paulusførstebrevtiltimoteus", "1Tim"),
    ("2timoteusbrev", "2Tim"),
    ("paulusandrebrevtiltimoteus", "2Tim"),
    ("titusbrev", "Titus"),
    ("paulusbrevtiltitus", "Titus"),
    ("filem", "Phlm"),
    ("filemonsbrev", "Phlm"),
    ("paulusbrevtilfilemon", "Phlm"),
    ("hebr", "Heb"),
    ("brevettilhebreerne", "Heb"),
    ("jak", "Jas"),
    ("jakobsbrev", "Jas"),
    ("1petersbrev", "1Pet"),
    ("petersførstebrev", "1Pet"),
    ("2petersbrev", "2Pet"),
    ("petersandrebrev", "2Pet"),
    ("1joh", "1John"),
    ("1johannesbrev", "1John"),
    ("johannesførstebrev", "1John"),
    ("2joh", "2John"),
    ("2johannesbrev", "2John"),
    ("johannesandrebrev", "2John"),
    ("3joh", "3John"),
    ("3johannesbrev", "3John"),
    ("johannestredjebrev", "3John"),
    ("judasbrev", "Jude"),
    ("aap", "Rev"),
    ("johannesåpenbaring", "Rev"),
    // Engelske navn og kortformer (KJV m.fl.).
    ("genesis", "Gen"),
    ("exodus", "Exod"),
    ("ex", "Exod"),
    ("leviticus", "Lev"),
    ("numbers", "Num"),
    ("deuteronomy", "Deut"),
    ("joshua", "Josh"),
    ("judges", "Judg"),
    ("1kings", "1Kgs"),
    ("2kings", "2Kgs"),
    ("1chronicles", "1Chr"),
    ("2chronicles", "2Chr"),
    ("nehemiah", "Neh"),
    ("esther", "Esth"),
    ("psalms", "Ps"),
    ("psalm", "Ps"),
    ("pss", "Ps"),
    ("proverbs", "Prov"),
    ("ecclesiastes", "Eccl"),
    ("eccles", "Eccl"),
    ("qoh", "Eccl"),
    ("songofsolomon", "Song"),
    ("songofsongs", "Song"),
    ("canticles", "Song"),
    ("isaiah", "Isa"),
    ("jeremiah", "Jer"),
    ("lamentations", "Lam"),
    ("ezekiel", "Ezek"),
    ("eze", "Ezek"),
    ("obadiah", "Obad"),
    ("micah", "Mic"),
    ("zephaniah", "Zeph"),
    ("zechariah", "Zech"),
    ("malachi", "Mal"),
    ("matthew", "Matt"),
    ("mt", "Matt"),
    ("mk", "Mark"),
    ("lk", "Luke"),
    ("jn", "John"),
    ("romans", "Rom"),
    ("1corinthians", "1Cor"),
    ("2corinthians", "2Cor"),
    ("galatians", "Gal"),
    ("ephesians", "Eph"),
    ("philippians", "Phil"),
    ("colossians", "Col"),
    ("1thessalonians", "1Thess"),
    ("2thessalonians", "2Thess"),
    ("1timothy", "1Tim"),
    ("2timothy", "2Tim"),
    ("philemon", "Phlm"),
    ("philem", "Phlm"),
    ("hebrews", "Heb"),
    ("james", "Jas"),
    ("jam", "Jas"),
    ("revelation", "Rev"),
    ("revelations", "Rev"),
    ("apocalypse", "Rev"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefError {
    Empty,
    UnknownBook(String),
    Ambiguous(String, Vec<String>),
    MissingChapter(String),
    BadSyntax(String),
    NoSuchChapter { book: String, chapter: u16, max: u16 },
    NoSuchVerse { book: String, chapter: u16, verse: u16, max: u16 },
    Backwards,
}

impl fmt::Display for RefError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RefError::Empty => write!(f, "tom referanse"),
            RefError::UnknownBook(b) => write!(f, "ukjent bok «{b}»"),
            RefError::Ambiguous(b, cands) => {
                write!(f, "flertydig bok «{b}» - mente du {}?", cands.join(", "))
            }
            RefError::MissingChapter(b) => write!(f, "«{b}» mangler kapittelnummer"),
            RefError::BadSyntax(s) => write!(f, "forstod ikke «{s}»"),
            RefError::NoSuchChapter { book, chapter, max } => {
                write!(f, "{book} har bare {max} kapitler, ikke {chapter}")
            }
            RefError::NoSuchVerse { book, chapter, verse, max } => {
                write!(f, "{book} {chapter} har bare {max} vers, ikke {verse}")
            }
            RefError::Backwards => write!(f, "intervallet slutter før det begynner"),
        }
    }
}

/// En referanse som er kontrollert mot datasettet og garantert slår opp.
#[derive(Debug)]
pub struct Resolved<'a> {
    pub book: Book<'a>,
    pub start: (u16, u16),
    pub end: (u16, u16),
    pub whole_chapter: bool,
}

impl Resolved<'_> {
    pub fn is_single_verse(&self) -> bool {
        self.start == self.end
    }

    pub fn spans_chapters(&self) -> bool {
        self.start.0 != self.end.0
    }
}

/// Slår sammen skrivemåter: små bokstaver, uten skilletegn, og med æ/ø/å
/// enten som enkel vokal (`å`→`a`) eller som dobbel (`å`→`aa`). Begge trengs
/// fordi datasettet selv er inkonsekvent - `hoys` for Høysangen, men `aap`
/// for Åpenbaringen.
fn fold(s: &str, double: bool) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            'æ' | 'Æ' => out.push_str(if double { "ae" } else { "a" }),
            'ø' | 'Ø' => out.push_str(if double { "oe" } else { "o" }),
            'å' | 'Å' => out.push_str(if double { "aa" } else { "a" }),
            'é' | 'è' | 'ê' => out.push('e'),
            ' ' | '.' | '-' | '_' | '\u{a0}' | '\'' | '´' | '\u{2019}' => {}
            _ => out.extend(c.to_lowercase()),
        }
    }
    out
}

fn same(a: &str, needle_single: &str, needle_double: &str) -> bool {
    fold(a, false) == needle_single || fold(a, true) == needle_double
}

fn find_book<'a>(bible: &Bible<'a>, needle: &str) -> Result<Book<'a>, RefError> {
    let single = fold(needle, false);
    let double = fold(needle, true);
    if single.is_empty() {
        return Err(RefError::Empty);
    }

    for book in bible.books() {
        if same(book.abbrev, &single, &double) {
            return Ok(book);
        }
    }
    for book in bible.books() {
        if same(book.osis_id, &single, &double) {
            return Ok(book);
        }
    }
    // Datasettene bruker ulike kodeverk for bok-id: NB88 og KJV har OSIS («John»),
    // bibel.no har USFM («JHN»). Her oversettes begge til samme nøkkel, slik at
    // «john3:16» og «jhn3:16» treffer i alle oversettelsene.
    if let Some(wanted) = books::canonical(&single) {
        for book in bible.books() {
            if books::canonical(book.osis_id) == Some(wanted) {
                return Ok(book);
            }
        }
    }
    if let Some((_, osis)) = ALIASES
        .iter()
        .find(|(alias, _)| same(alias, &single, &double))
    {
        let wanted = books::canonical(osis);
        if let Some(book) = bible.books().find(|b| {
            b.osis_id == *osis || (wanted.is_some() && books::canonical(b.osis_id) == wanted)
        }) {
            return Ok(book);
        }
    }
    for book in bible.books() {
        if same(book.name, &single, &double) {
            return Ok(book);
        }
    }

    // Til slutt: entydig prefiks. Flere treff er en feil, ikke et tilfeldig valg.
    let mut hits: Vec<Book<'a>> = Vec::new();
    for book in bible.books() {
        let matches = [book.abbrev, book.osis_id, book.name]
            .iter()
            .any(|s| fold(s, false).starts_with(&single) || fold(s, true).starts_with(&double));
        if matches {
            hits.push(book);
        }
    }
    match hits.len() {
        0 => Err(RefError::UnknownBook(needle.to_string())),
        1 => Ok(hits[0]),
        _ => Err(RefError::Ambiguous(
            needle.to_string(),
            hits.iter().take(6).map(|b| b.abbrev.to_string()).collect(),
        )),
    }
}

fn take_num(bytes: &[u8], pos: &mut usize) -> Option<u16> {
    let start = *pos;
    let mut n: u32 = 0;
    while let Some(d) = bytes.get(*pos).filter(|b| b.is_ascii_digit()) {
        n = n.checked_mul(10)?.checked_add((d - b'0') as u32)?;
        if n > u16::MAX as u32 {
            return None;
        }
        *pos += 1;
    }
    if *pos == start {
        None
    } else {
        Some(n as u16)
    }
}

fn is_verse_sep(b: u8) -> bool {
    b == b':' || b == b','
}

enum Parsed {
    Chapter(u16),
    Verse(u16, u16),
    Range((u16, u16), (u16, u16)),
}

fn parse_tail(tail: &str) -> Result<Parsed, RefError> {
    let bad = || RefError::BadSyntax(tail.to_string());
    let b = tail.as_bytes();
    let mut p = 0;

    let chapter = take_num(b, &mut p).ok_or_else(bad)?;
    if p == b.len() {
        return Ok(Parsed::Chapter(chapter));
    }
    if !is_verse_sep(b[p]) {
        return Err(bad());
    }
    p += 1;

    let first = take_num(b, &mut p).ok_or_else(bad)?;
    if p == b.len() {
        return Ok(Parsed::Verse(chapter, first));
    }
    if b[p] != b'-' {
        return Err(bad());
    }
    p += 1;

    let second = take_num(b, &mut p).ok_or_else(bad)?;
    if p == b.len() {
        // «2:1-3» - samme kapittel
        return Ok(Parsed::Range((chapter, first), (chapter, second)));
    }
    if !is_verse_sep(b[p]) {
        return Err(bad());
    }
    p += 1;
    let third = take_num(b, &mut p).ok_or_else(bad)?;
    if p != b.len() {
        return Err(bad());
    }
    // «1:20-2:5» - andre tallet var et kapittel
    Ok(Parsed::Range((chapter, first), (second, third)))
}

pub fn resolve<'a>(bible: &Bible<'a>, input: &str) -> Result<Resolved<'a>, RefError> {
    let cleaned: String = input
        .trim()
        .trim_start_matches(':')
        .chars()
        .map(|c| match c {
            '\u{2013}' | '\u{2014}' | '\u{2212}' => '-', // tankestrek og minus
            other => other,
        })
        // Apostrofer forekommer i boknavn som «Lukas´ evangelium».
        .filter(|c| !matches!(c, ' ' | '.' | '\u{a0}' | '\'' | '´' | '\u{2019}'))
        .flat_map(char::to_lowercase)
        .collect();
    if cleaned.is_empty() {
        return Err(RefError::Empty);
    }

    // Boknavn er valgfritt ledende siffer etterfulgt av bokstaver; resten er tall
    // og skilletegn. Ingen forkortelse inneholder siffer utenom det ledende, så
    // delingen er entydig.
    let chars: Vec<char> = cleaned.chars().collect();
    let mut i = usize::from(chars[0].is_ascii_digit());
    while i < chars.len() && chars[i].is_alphabetic() {
        i += 1;
    }
    let book_part: String = chars[..i].iter().collect();
    let tail: String = chars[i..].iter().collect();

    let book = find_book(bible, &book_part)?;
    if tail.is_empty() {
        return Err(RefError::MissingChapter(book.name.to_string()));
    }

    let parsed = parse_tail(&tail)?;
    let last_verse = |c: u16| -> u16 {
        bible
            .chapter(&book, c)
            .and_then(|ch| ch.verse_count.checked_sub(1).and_then(|i| bible.verse_at(&ch, i)))
            .map_or(0, |v| v.end)
    };
    let chapter_of = |n: u16| {
        bible
            .chapter(&book, n)
            .ok_or_else(|| RefError::NoSuchChapter {
                book: book.name.to_string(),
                chapter: n,
                max: book.chapter_count,
            })
    };
    let check_verse = |c: u16, v: u16| -> Result<(), RefError> {
        let chapter = chapter_of(c)?;
        if bible.verse(&chapter, v).is_none() {
            return Err(RefError::NoSuchVerse {
                book: book.name.to_string(),
                chapter: c,
                verse: v,
                max: last_verse(c),
            });
        }
        Ok(())
    };

    match parsed {
        Parsed::Chapter(c) => {
            let chapter = chapter_of(c)?;
            let first = bible.verse_at(&chapter, 0).map_or(1, |v| v.start);
            Ok(Resolved {
                book,
                start: (c, first),
                end: (c, last_verse(c)),
                whole_chapter: true,
            })
        }
        Parsed::Verse(c, v) => {
            check_verse(c, v)?;
            Ok(Resolved { book, start: (c, v), end: (c, v), whole_chapter: false })
        }
        Parsed::Range(start, end) => {
            check_verse(start.0, start.1)?;
            check_verse(end.0, end.1)?;
            if end < start {
                return Err(RefError::Backwards);
            }
            Ok(Resolved { book, start, end, whole_chapter: false })
        }
    }
}

/// Alle vers i referansen, som (kapittel, vers). Et sammenslått vers tas med
/// én gang selv om intervallet dekker begge numrene det står for.
pub fn verses<'a>(bible: &Bible<'a>, r: &Resolved<'a>) -> Vec<(u16, Verse<'a>)> {
    let mut out: Vec<(u16, Verse<'a>)> = Vec::new();
    for c in r.start.0..=r.end.0 {
        let Some(chapter) = bible.chapter(&r.book, c) else {
            continue;
        };
        for i in 0..chapter.verse_count {
            let Some(v) = bible.verse_at(&chapter, i) else {
                continue;
            };
            // Ta med verset hvis intervallet overlapper det referansen spør om.
            let after_start = c > r.start.0 || v.end >= r.start.1;
            let before_end = c < r.end.0 || v.start <= r.end.1;
            if after_start && before_end {
                out.push((c, v));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use bibel_format::build::{BuildVerse, Builder};

    fn data() -> Vec<u8> {
        let mut b = Builder::new("test", "Testbibel", "");
        let sam = b.add_book(9, "1Sam", "1sam", "1. Samuelsbok");
        b.add_chapter(sam, "Elkana og Hanna", (1..=28).map(|n| BuildVerse::new(n, format!("1:{n}"))).collect());
        b.add_chapter(sam, "Hannas lovsang", (1..=36).map(|n| BuildVerse::new(n, format!("2:{n}"))).collect());
        let ps = b.add_book(19, "Ps", "sal", "Salmenes bok");
        b.add_chapter(ps, "Herren er min hyrde", (1..=6).map(|n| BuildVerse::new(n, format!("sal:{n}"))).collect());
        let rev = b.add_book(66, "Rev", "aap", "Åpenbaringen");
        b.add_chapter(rev, "Jeg kommer snart", (1..=21).map(|n| BuildVerse::new(n, format!("aap:{n}"))).collect());
        let job = b.add_book(18, "Job", "job", "Jobs bok");
        b.add_chapter(job, "", vec![BuildVerse::new(1, "job")]);
        b.finish()
    }

    fn resolve_ok(bytes: &[u8], input: &str) -> ((u16, u16), (u16, u16), String) {
        let bible = Bible::parse(bytes).unwrap();
        let r = resolve(&bible, input).unwrap();
        (r.start, r.end, r.book.osis_id.to_string())
    }

    #[test]
    fn alle_formene() {
        let d = data();
        assert_eq!(resolve_ok(&d, "1sam1:3"), ((1, 3), (1, 3), "1Sam".into()));
        assert_eq!(resolve_ok(&d, "1sam2:1-3"), ((2, 1), (2, 3), "1Sam".into()));
        assert_eq!(resolve_ok(&d, "1sam1:20-2:5"), ((1, 20), (2, 5), "1Sam".into()));
        assert_eq!(resolve_ok(&d, "1sam2"), ((2, 1), (2, 36), "1Sam".into()));
    }

    #[test]
    fn skrivemater() {
        let d = data();
        for input in [":1sam2:1-3", "1 Sam 2:1-3", "1. Samuelsbok 2,1-3", "1sam2:1\u{2013}3"] {
            assert_eq!(resolve_ok(&d, input), ((2, 1), (2, 3), "1Sam".into()), "{input}");
        }
        // Forkortelse, osisId, alias og fullt navn skal alle treffe Salmenes bok.
        for input in ["sal23:1", "ps23:1", "salmene23:1", "Salmenes bok 23:1"] {
            // Testdatasettet har bare ett kapittel, så bruk kapittel 1.
            let input = input.replace("23", "1");
            assert_eq!(resolve_ok(&d, &input).2, "Ps", "{input}");
        }
        // Åpenbaringen: abbrev «aap» nås både som «åp» (alias) og «aap».
        for input in ["aap1:21", "åp1:21", "Åpenbaringen 1:21", "rev1:21"] {
            assert_eq!(resolve_ok(&d, input).2, "Rev", "{input}");
        }
    }

    #[test]
    fn feiltilfeller() {
        let d = data();
        let bible = Bible::parse(&d).unwrap();
        let err = |s: &str| resolve(&bible, s).unwrap_err();

        assert!(matches!(err("xyz1:1"), RefError::UnknownBook(_)));
        assert!(matches!(err("1sam"), RefError::MissingChapter(_)));
        assert!(matches!(err("1sam99:1"), RefError::NoSuchChapter { .. }));
        assert!(matches!(err("1sam2:99"), RefError::NoSuchVerse { .. }));
        assert!(matches!(err("1sam2:5-1"), RefError::Backwards));
        assert!(matches!(err("1sam2:5-1:1"), RefError::Backwards));
        assert!(matches!(err("1sam2:1-"), RefError::BadSyntax(_)));
        assert!(matches!(err("1sam2:1-3x"), RefError::BadSyntax(_)));
        assert!(matches!(err("1sam2:0"), RefError::NoSuchVerse { .. }));
        assert!(matches!(err(""), RefError::Empty));
        // «jo» er prefiks for både Job og flere andre bøker i det ekte datasettet;
        // her er Job eneste treff, så prefikset er entydig.
        assert_eq!(resolve(&bible, "jo1:1").unwrap().book.osis_id, "Job");
    }

    #[test]
    fn flertydig_prefiks() {
        let mut b = Builder::new("test", "Testbibel", "");
        for (no, osis, abbrev, name) in [
            (18u8, "Job", "job", "Jobs bok"),
            (29, "Joel", "joel", "Joel"),
            (32, "Jonah", "jon", "Jonas bok"),
        ] {
            let id = b.add_book(no, osis, abbrev, name);
            b.add_chapter(id, "", vec![BuildVerse::new(1, "x")]);
        }
        let d = b.finish();
        let bible = Bible::parse(&d).unwrap();
        assert!(matches!(resolve(&bible, "jo1:1"), Err(RefError::Ambiguous(_, _))));
    }

    #[test]
    fn versuthenting() {
        let d = data();
        let bible = Bible::parse(&d).unwrap();
        let r = resolve(&bible, "1sam1:27-2:2").unwrap();
        let got: Vec<_> = verses(&bible, &r).iter().map(|(c, v)| (*c, v.start)).collect();
        assert_eq!(got, vec![(1, 27), (1, 28), (2, 1), (2, 2)]);

        let r = resolve(&bible, "1sam2").unwrap();
        assert_eq!(verses(&bible, &r).len(), 36);
    }
}
