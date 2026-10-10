//! Oppslag i Bibelen med `bibel`, så sitatene i manuset kan sammenlignes
//! med bibelteksten. Skrevet etter `word-to-markdown/src/bibleText.ts`.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;

use crate::bible::Bibel;
use crate::book_names::BOOK_IDS;
use crate::reference::{is_book_name, to_lookup};
use crate::word::align::words;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerseId {
    pub book: String,
    pub chapter: u32,
    pub verse: u32,
}

pub trait BibleText {
    /// Oversettelsene teksten sammenlignes med, som ["nb88", "nb24"]. Den første gir navnet i referansene.
    fn translations(&self) -> &[String];
    /// Bok-id-en `bibel` slår opp på ("DAN"), for et boknavn som "Dan", "2.Kongebok" eller "Åpenbaringen".
    fn book_id(&self, name: &str) -> Option<String>;
    /// Teksten i ett vers, uten versnummer.
    fn verse(&self, translation: &str, id: &VerseId) -> Option<String>;
    /// Referansen slik `bibel` skriver den: "Profeten Daniel 1:1-2".
    fn reference(&self, book: &str, chapter: u32, from: u32, to: u32) -> Option<String>;
    /// De versene som har flest av ordene (minst tre, eller alle hvis det er færre).
    fn search(&self, translation: &str, words: &[String]) -> Vec<VerseId>;
}

/// Alle versene i en oversettelse, med en ordliste for søk.
struct SearchIndex {
    verses: Vec<VerseId>,
    /// Versene (indekser i `verses`) hvert ord står i.
    words: HashMap<String, Vec<usize>>,
}

/// [`BibleText`] med en `bibel` som allerede er lastet.
pub struct BibelText<'b> {
    bibel: &'b Bibel,
    translations: Vec<String>,
    books: RefCell<HashMap<String, Option<String>>>,
    verses: RefCell<HashMap<String, Option<String>>>,
    indexes: RefCell<HashMap<String, std::rc::Rc<SearchIndex>>>,
}

static KEY_SEPARATORS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\s.]+").unwrap());
static NUMBERED_BOOK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([1-5])\.?(\p{L})").unwrap());
static VERSE_LINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([0-9]+)(?:-[0-9]+)?\s+(.*)$").unwrap());

impl<'b> BibelText<'b> {
    /// Oversettelsene som ikke er lastet, hoppes over.
    pub fn new(bibel: &'b Bibel, translations: &[&str]) -> Result<Self, String> {
        let available: Vec<String> = translations
            .iter()
            .filter(|t| bibel.run(&["--format", "{bok}", &format!("{t}:joh1:1")]).code == 0)
            .map(|t| t.to_string())
            .collect();
        if available.is_empty() {
            return Err(format!("fant ingen av oversettelsene {}", translations.join(", ")));
        }
        Ok(BibelText {
            bibel,
            translations: available,
            books: RefCell::default(),
            verses: RefCell::default(),
            indexes: RefCell::default(),
        })
    }

    fn find_book_id(&self, name: &str, key: &str) -> Option<String> {
        if !is_book_name(&NUMBERED_BOOK.replace(name, "$1. $2")) {
            return None;
        }
        // `bibel` kjenner forkortelser og alias ("2kong", "åpenbaringen"), men gir det
        // fulle navnet; det gjøres om til en bok-id som virker i alle oversettelsene.
        let output = self.bibel.run(&["--format", "{bok}", &format!("{}:{key}1:1", self.translations[0])]);
        if output.code != 0 {
            return None;
        }
        to_lookup(&format!("{} 1:1", output.stdout.trim()))
            .map(|lookup| lookup.strip_suffix("1:1").map(String::from).unwrap_or(lookup))
    }

    /// Raskere enn `bibel --søk` når mange avsnitt skal sjekkes.
    fn index(&self, translation: &str) -> std::rc::Rc<SearchIndex> {
        if let Some(index) = self.indexes.borrow().get(translation) {
            return index.clone();
        }
        let mut index = SearchIndex { verses: Vec::new(), words: HashMap::new() };
        let mut books = HashSet::new();
        let mut seen_ids = HashSet::new();
        let mut verses = self.verses.borrow_mut();
        for (_, id) in BOOK_IDS {
            if !seen_ids.insert(*id) {
                continue;
            }
            let output = self.bibel.run(&["--format", "{bok}", &format!("{translation}:{id}1:1")]);
            if output.code != 0 || !books.insert(output.stdout) {
                continue;
            }
            for chapter in 1.. {
                let output = self.bibel.run(&[
                    "--format",
                    "{tekst}",
                    "--skille",
                    "\n",
                    &format!("{translation}:{id}{chapter}"),
                ]);
                if output.code != 0 {
                    break;
                }
                for line in output.stdout.trim().split('\n') {
                    let Some(caps) = VERSE_LINE.captures(line) else { continue };
                    let verse = VerseId { book: id.to_string(), chapter, verse: caps[1].parse().unwrap_or(0) };
                    let at = index.verses.len();
                    verses.insert(format!("{translation}:{id}{chapter}:{}", verse.verse), Some(caps[2].to_string()));
                    index.verses.push(verse);
                    let mut unique = HashSet::new();
                    for word in words(&caps[2]) {
                        if unique.insert(word.clone()) {
                            index.words.entry(word).or_default().push(at);
                        }
                    }
                }
            }
        }
        let index = std::rc::Rc::new(index);
        self.indexes.borrow_mut().insert(translation.to_string(), index.clone());
        index
    }
}

impl BibleText for BibelText<'_> {
    fn translations(&self) -> &[String] {
        &self.translations
    }

    fn book_id(&self, name: &str) -> Option<String> {
        let key = KEY_SEPARATORS.replace_all(&name.to_lowercase(), "").into_owned();
        if let Some(found) = self.books.borrow().get(&key) {
            return found.clone();
        }
        let found = self.find_book_id(name, &key);
        self.books.borrow_mut().insert(key, found.clone());
        found
    }

    fn verse(&self, translation: &str, id: &VerseId) -> Option<String> {
        let lookup = format!("{translation}:{}{}:{}", id.book, id.chapter, id.verse);
        if let Some(found) = self.verses.borrow().get(&lookup) {
            return found.clone();
        }
        let output = self.bibel.run(&["--format", "{tekst}", "--uten-numre", &lookup]);
        let found = (output.code == 0).then(|| output.stdout.trim().to_string());
        self.verses.borrow_mut().insert(lookup, found.clone());
        found
    }

    fn reference(&self, book: &str, chapter: u32, from: u32, to: u32) -> Option<String> {
        let verses = if from == to { from.to_string() } else { format!("{from}-{to}") };
        let output =
            self.bibel.run(&["--format", "{ref}", &format!("{}:{book}{chapter}:{verses}", self.translations[0])]);
        (output.code == 0).then(|| output.stdout.trim().to_string())
    }

    fn search(&self, translation: &str, query: &[String]) -> Vec<VerseId> {
        let index = self.index(translation);
        // Telling i rekkefølgen versene først ble funnet, som en `Map` i JavaScript.
        let mut order: Vec<usize> = Vec::new();
        let mut counts: HashMap<usize, usize> = HashMap::new();
        let mut unique = HashSet::new();
        for word in query {
            if !unique.insert(word) {
                continue;
            }
            for &verse in index.words.get(word).map(Vec::as_slice).unwrap_or(&[]) {
                let count = counts.entry(verse).or_insert_with(|| {
                    order.push(verse);
                    0
                });
                *count += 1;
            }
        }
        let needed = query.len().min(3);
        let mut hits: Vec<(usize, usize)> =
            order.into_iter().map(|verse| (verse, counts[&verse])).filter(|(_, count)| *count >= needed).collect();
        hits.sort_by_key(|&(_, count)| std::cmp::Reverse(count));
        hits.into_iter().take(5).map(|(verse, _)| index.verses[verse].clone()).collect()
    }
}
