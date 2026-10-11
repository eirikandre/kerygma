//! Fritekstsøk i en oversettelse.
//!
//! Søket går over versene i kanonisk rekkefølge og folder hvert vers til en
//! sammenlignbar form før leddene prøves mot det. Foldingen gjøres på begge
//! sider, så den trenger ikke å være «riktig» - bare lik: små bokstaver,
//! skilletegn bort, mellomrom beholdt som ett. Æ, ø og å står som de er; skal
//! du finne «kjærlighet», skriver du kjærlighet.
//!
//! Et ledd prøves alltid mot ett vers av gangen, aldri mot tekstblokka under.
//! Det er det som gjør at en frase ikke kan treffe slutten av ett vers og
//! starten av det neste.

use std::fmt;

use bibel_format::{Bible, Book, Chapter, Verse};

/// Flere ledd enn dette er nesten alltid en skrivefeil, og hvert ledd koster
/// en gjennomgang til.
pub const MAX_TERMS: usize = 8;
/// Ett tegn treffer så mye at svaret er ubrukelig.
const MIN_TERM: usize = 2;

/// Hvor leddene må opptre sammen for at det skal telle som et treff.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// Alle leddene i samme vers.
    Verse,
    /// Alle leddene i samme kapittel - versene de står i vises hver for seg.
    Chapter,
}

#[derive(Debug, PartialEq, Eq)]
pub enum QueryError {
    Empty,
    TooShort(String),
    TooMany(usize),
    OnlyNegative,
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryError::Empty => write!(f, "tomt søk"),
            QueryError::TooShort(t) => {
                write!(f, "søkeordet «{t}» er for kort - minst {MIN_TERM} tegn")
            }
            QueryError::TooMany(n) => write!(f, "for mange søkeord ({n}) - maks {MAX_TERMS}"),
            QueryError::OnlyNegative => {
                write!(f, "søket må ha minst ett ord som ikke er utelatt")
            }
        }
    }
}

impl std::error::Error for QueryError {}

pub struct Options {
    pub scope: Scope,
    /// Hvor mange treff som vises. 0 betyr alle.
    pub limit: usize,
    pub whole_words: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options { scope: Scope::Verse, limit: 0, whole_words: false }
    }
}

/// Et ferdig tolket søk. Leddene er foldet, så de kan sammenlignes direkte.
#[derive(Debug, PartialEq, Eq)]
pub struct Query {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

/// Folder tekst til formen søket sammenligner i: små bokstaver, uten
/// skilletegn, og med mellomrom slått sammen til ett.
///
/// Skilletegnet forsvinner uten å bli til mellomrom, slik at frasen «Gud som»
/// treffer «Gud, som» - og `*`-markørene mot studienotene blir borte av seg
/// selv. Mellomrommene må bli stående: uten dem fantes det ingen ordgrense å
/// kreve med `--hele-ord`.
fn fold_into(s: &str, out: &mut String) {
    fn space(out: &mut String) {
        if !out.is_empty() && !out.ends_with(' ') {
            out.push(' ');
        }
    }

    out.clear();
    for c in s.chars() {
        // ASCII-greina først: den tar nesten all teksten, og slipper unna
        // unicode-tabellene. Hele bibelen foldes for hvert søk, så det merkes.
        if c.is_ascii() {
            if c.is_ascii_alphanumeric() {
                out.push(c.to_ascii_lowercase());
            } else if c.is_ascii_whitespace() {
                space(out);
            }
        } else if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if c.is_whitespace() {
            space(out);
        }
    }
    while out.ends_with(' ') {
        out.pop();
    }
}

pub fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    fold_into(s, &mut out);
    out
}

/// Tolker søkestrengen.
///
/// ```text
/// tro håp          begge ordene
/// "i begynnelsen"  frase
/// tro -lov         utelat vers med «lov»
/// ```
pub fn parse(input: &str) -> Result<Query, QueryError> {
    let mut query = Query { include: Vec::new(), exclude: Vec::new() };
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        let negated = chars[i] == '-' && i + 1 < chars.len() && !chars[i + 1].is_whitespace();
        if negated {
            i += 1;
        }
        let mut raw = String::new();
        if i < chars.len() && chars[i] == '"' {
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                raw.push(chars[i]);
                i += 1;
            }
            // Et manglende avsluttende anførselstegn er ikke en feil: resten av
            // linja er frasen. Det er alltid det brukeren mente.
            i += 1;
        } else {
            while i < chars.len() && !chars[i].is_whitespace() {
                raw.push(chars[i]);
                i += 1;
            }
        }

        let folded = fold(&raw);
        if folded.is_empty() {
            // Et ledd som bare var skilletegn - en enslig bindestrek, f.eks.
            continue;
        }
        if folded.chars().count() < MIN_TERM {
            return Err(QueryError::TooShort(raw));
        }
        if negated {
            query.exclude.push(folded);
        } else {
            query.include.push(folded);
        }
    }

    let total = query.include.len() + query.exclude.len();
    if total > MAX_TERMS {
        return Err(QueryError::TooMany(total));
    }
    if query.include.is_empty() {
        return Err(if query.exclude.is_empty() {
            QueryError::Empty
        } else {
            QueryError::OnlyNegative
        });
    }
    Ok(query)
}

/// Finnes leddet i den foldede teksten? Med `whole` kreves ordgrense på begge
/// sider - og siden teksten er foldet, er en ordgrense nøyaktig «mellomrom
/// eller slutten på strengen».
fn contains(hay: &str, needle: &str, whole: bool) -> bool {
    if !whole {
        return hay.contains(needle);
    }
    let mut from = 0;
    while let Some(pos) = hay[from..].find(needle) {
        let start = from + pos;
        let end = start + needle.len();
        if (start == 0 || hay[..start].ends_with(' '))
            && (end == hay.len() || hay[end..].starts_with(' '))
        {
            return true;
        }
        // Trygt som tegngrense: både start og lengden på nåla ligger på en.
        from = end;
    }
    false
}

/// Ett vers som ble truffet, med nok kontekst til å kunne skrives ut.
#[derive(Clone, Copy, Debug)]
pub struct Hit<'a> {
    pub book: Book<'a>,
    pub chapter: Chapter<'a>,
    pub verse: Verse<'a>,
}

/// `total` teller alle treffene, også dem som ikke fikk plass i `hits`.
pub struct Found<'a> {
    pub hits: Vec<Hit<'a>>,
    pub total: usize,
}

impl<'a> Found<'a> {
    fn push(&mut self, hit: Hit<'a>, limit: usize) {
        self.total += 1;
        if limit == 0 || self.hits.len() < limit {
            self.hits.push(hit);
        }
    }

    /// Treff som ikke ble vist.
    pub fn hidden(&self) -> usize {
        self.total - self.hits.len()
    }
}

pub fn search<'a>(bible: &Bible<'a>, query: &Query, opts: &Options) -> Found<'a> {
    let mut found = Found { hits: Vec::new(), total: 0 };
    let mut buf = String::new();
    // Hvilke ledd som er sett i kapitlet - bare i bruk ved kapittelomfang.
    let mut seen = vec![false; query.include.len()];

    for book in bible.books() {
        for number in 1..=book.chapter_count {
            let Some(chapter) = bible.chapter(&book, number) else { continue };
            match opts.scope {
                Scope::Verse => {
                    for verse in bible.chapter_verses(&chapter) {
                        fold_into(verse.text, &mut buf);
                        let hit = query.include.iter().all(|t| contains(&buf, t, opts.whole_words))
                            && !query.exclude.iter().any(|t| contains(&buf, t, opts.whole_words));
                        if hit {
                            found.push(Hit { book, chapter, verse }, opts.limit);
                        }
                    }
                }
                Scope::Chapter => {
                    // Første gjennomgang avgjør om kapitlet treffer i det hele tatt.
                    seen.iter_mut().for_each(|s| *s = false);
                    let mut excluded = false;
                    for verse in bible.chapter_verses(&chapter) {
                        fold_into(verse.text, &mut buf);
                        if query.exclude.iter().any(|t| contains(&buf, t, opts.whole_words)) {
                            excluded = true;
                            break;
                        }
                        for (i, term) in query.include.iter().enumerate() {
                            if !seen[i] && contains(&buf, term, opts.whole_words) {
                                seen[i] = true;
                            }
                        }
                    }
                    if excluded || !seen.iter().all(|s| *s) {
                        continue;
                    }
                    // Andre gjennomgang plukker ut versene som faktisk bidrar.
                    // Den koster bare noe i de få kapitlene som traff.
                    for verse in bible.chapter_verses(&chapter) {
                        fold_into(verse.text, &mut buf);
                        if query.include.iter().any(|t| contains(&buf, t, opts.whole_words)) {
                            found.push(Hit { book, chapter, verse }, opts.limit);
                        }
                    }
                }
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use bibel_format::build::{BuildVerse, Builder};

    fn data() -> Vec<u8> {
        let mut b = Builder::new("test", "Testbibel", "");
        let joh = b.add_book(43, "John", "joh", "Johannes");
        b.add_chapter(
            joh,
            "Ordet ble menneske",
            vec![
                BuildVerse::new(1, "I begynnelsen var Ordet, og Ordet var hos Gud."),
                BuildVerse::new(2, "Han var i begynnelsen hos Gud."),
                BuildVerse::new(3, "Alt er blitt til ved ham*, og uten ham ble ingenting til."),
            ],
        );
        let kor = b.add_book(46, "1Cor", "1kor", "1. Korinterbrev");
        b.add_chapter(
            kor,
            "Kjærligheten",
            vec![
                BuildVerse::new(1, "Om jeg taler med tunger, men ikke har kjærlighet."),
                BuildVerse::new(2, "Kjærligheten er tålmodig, den er velvillig."),
                BuildVerse::new(3, "Så blir de stående: tro, håp og kjærlighet."),
            ],
        );
        b.finish()
    }

    fn found(bytes: &[u8], q: &str, opts: &Options) -> (Vec<String>, usize) {
        let bible = Bible::parse(bytes).expect("testdata");
        let query = parse(q).unwrap_or_else(|e| panic!("«{q}»: {e}"));
        let f = search(&bible, &query, opts);
        let refs = f
            .hits
            .iter()
            .map(|h| format!("{} {}:{}", h.book.abbrev, h.chapter.number, h.verse.label()))
            .collect();
        (refs, f.total)
    }

    fn refs(bytes: &[u8], q: &str) -> Vec<String> {
        found(bytes, q, &Options::default()).0
    }

    #[test]
    fn folding_er_lik_pa_begge_sider() {
        assert_eq!(fold("I BEGYNNELSEN"), "i begynnelsen");
        // Skilletegn og fotnotemarkører forsvinner, mellomrom blir stående.
        assert_eq!(fold("Gud, som *er\n  her"), "gud som er her");
        assert_eq!(fold("  kant  "), "kant");
        // Æ, ø og å står som de er - de skrives som de staves.
        assert_eq!(fold("Kjærlighet"), "kjærlighet");
    }

    #[test]
    fn tolker_ledd_frase_og_utelatelse() {
        let q = parse("tro håp").unwrap();
        assert_eq!(q.include, vec!["tro", "håp"]);
        assert!(q.exclude.is_empty());

        let q = parse("\"i begynnelsen\"").unwrap();
        assert_eq!(q.include, vec!["i begynnelsen"]);

        let q = parse("tro -lov -\"stor glede\"").unwrap();
        assert_eq!(q.include, vec!["tro"]);
        assert_eq!(q.exclude, vec!["lov", "stor glede"]);

        // Uavsluttet anførselstegn: resten av linja er frasen.
        assert_eq!(parse("\"tro og håp").unwrap().include, vec!["tro og håp"]);
    }

    #[test]
    fn avviser_ubrukelige_sok() {
        assert_eq!(parse(""), Err(QueryError::Empty));
        assert_eq!(parse("   "), Err(QueryError::Empty));
        assert_eq!(parse("-lov"), Err(QueryError::OnlyNegative));
        assert_eq!(parse("i"), Err(QueryError::TooShort("i".into())));
        let many = "en to tre fire fem seks sju atte ni";
        assert_eq!(parse(many), Err(QueryError::TooMany(9)));
        // Et ledd som bare var skilletegn teller ikke som noe.
        assert_eq!(parse("tro -"), Ok(Query { include: vec!["tro".into()], exclude: vec![] }));
    }

    #[test]
    fn flere_ledd_ma_sta_i_samme_vers() {
        let d = data();
        assert_eq!(refs(&d, "tro håp"), ["1kor 1:3"]);
        // «kjærlighet» alene finnes i tre vers; sammen med «tålmodig» i ett.
        assert_eq!(refs(&d, "kjærlighet").len(), 3);
        assert_eq!(refs(&d, "kjærlighet tålmodig"), ["1kor 1:2"]);
        // Rekkefølgen på leddene betyr ingenting.
        assert_eq!(refs(&d, "tålmodig kjærlighet"), ["1kor 1:2"]);
        assert!(refs(&d, "kjærlighet tunger tålmodig").is_empty());
    }

    #[test]
    fn frase_hopper_over_skilletegn_men_ikke_versgrenser() {
        let d = data();
        assert_eq!(refs(&d, "\"i begynnelsen\"").len(), 2);
        // Kommaet mellom «Ordet» og «og» skal ikke stoppe frasen.
        assert_eq!(refs(&d, "\"ordet og ordet\""), ["joh 1:1"]);
        // Vers 1 slutter på «hos Gud», vers 2 begynner på «Han var». Teksten
        // ligger inntil hverandre i datasettet, men er to vers - ikke ett treff.
        assert!(refs(&d, "\"gud han var\"").is_empty());
    }

    #[test]
    fn utelatelse_fjerner_vers() {
        let d = data();
        assert_eq!(refs(&d, "kjærlighet -tålmodig").len(), 2);
        assert_eq!(refs(&d, "begynnelsen -ordet"), ["joh 1:2"]);
    }

    #[test]
    fn hele_ord_skiller_ord_fra_orddel() {
        let d = data();
        let hele = Options { whole_words: true, ..Options::default() };
        // Uten flagget er delsøk standard, så «ord» finner «Ordet».
        assert_eq!(refs(&d, "ord").len(), 1);
        assert!(found(&d, "ord", &hele).0.is_empty());
        assert_eq!(found(&d, "ordet", &hele).0, ["joh 1:1"]);
        // Ordgrense ved slutten av verset teller også.
        assert_eq!(found(&d, "gud", &hele).0.len(), 2);
    }

    #[test]
    fn kapittelomfang_samler_ledd_over_flere_vers() {
        let d = data();
        // «tunger» og «tålmodig» står i hvert sitt vers: ingen treff på versnivå.
        assert!(refs(&d, "tunger tålmodig").is_empty());
        let kapittel = Options { scope: Scope::Chapter, ..Options::default() };
        // På kapittelnivå treffer kapitlet, og versene som bidrar vises.
        assert_eq!(found(&d, "tunger tålmodig", &kapittel).0, ["1kor 1:1", "1kor 1:2"]);
        // Et ledd som ikke finnes i kapitlet gir fortsatt ingenting.
        assert!(found(&d, "tunger ingenting", &kapittel).0.is_empty());
        // Utelatelse gjelder hele kapitlet.
        assert!(found(&d, "tunger tålmodig -håp", &kapittel).0.is_empty());
    }

    #[test]
    fn antall_begrenser_uten_a_skjule_hvor_mange_som_fantes() {
        let d = data();
        let to = Options { limit: 2, ..Options::default() };
        let (hits, total) = found(&d, "kjærlighet", &to);
        assert_eq!(hits.len(), 2);
        assert_eq!(total, 3, "totalen skal telle alle treffene");
        let (alle, total) = found(&d, "kjærlighet", &Options::default());
        assert_eq!(alle.len(), 3);
        assert_eq!(total, 3);
    }

    #[test]
    fn sok_i_ekte_data() {
        let Some(registry) = crate::translations::test_registry() else { return };
        let Some(bible) = registry.by_id("nb88") else { return };

        // Frasen står i 1. Mosebok 1:1 og i Matteus 19:4 («fra begynnelsen
        // skapte han dem»), og skal finnes begge steder - ikke bare det ene.
        let q = parse("\"begynnelsen skapte\"").unwrap();
        let f = search(bible, &q, &Options::default());
        assert_eq!(f.hits.len(), 2);
        assert_eq!(f.hits[0].book.osis_id, "Gen");
        assert_eq!(f.hits[0].chapter.number, 1);
        assert_eq!(f.hits[0].verse.start, 1);
        assert_eq!(f.hits[1].book.osis_id, "Matt");

        // Tre ledd som står i samme vers, med skilletegn mellom seg.
        let q = parse("tro håp kjærlighet").unwrap();
        let f = search(bible, &q, &Options::default());
        assert!(
            f.hits.iter().any(|h| h.book.osis_id == "1Cor" && h.chapter.number == 13),
            "1. Kor 13 skal være blant treffene"
        );

        // Et vanlig ord skal gi mange treff, og totalen skal overleve --antall.
        let q = parse("kjærlighet").unwrap();
        let begrenset = search(bible, &q, &Options { limit: 3, ..Options::default() });
        assert_eq!(begrenset.hits.len(), 3);
        assert!(begrenset.total > 3);
        assert_eq!(begrenset.hidden(), begrenset.total - 3);
        // Treffene kommer i kanonisk rekkefølge.
        let numbers: Vec<u8> = begrenset.hits.iter().map(|h| h.book.book_no).collect();
        assert!(numbers.windows(2).all(|p| p[0] <= p[1]), "ikke i bokrekkefølge");
    }
}
