//! Kompakt binærformat for én bibeloversettelse.
//!
//! Formatet er flatt og little-endian, og leses uten allokering og uten `unsafe`:
//! alle oppslag er indeksregning i et `&[u8]`-utsnitt. Det er laget for å bygges
//! inn i en kjørbar med `include_bytes!`, slik at oppslag ikke krever disk-IO.
//!
//! ```text
//! Header      magic "NB88", versjon u16, bookCount u16, chapterCount u32,
//!             verseCount u32, offsets u32 x5 (books, chapters, verses, strings, text),
//!             textLen u32, idStr u32, titleStr u32, copyrightStr u32, numbersOff u32
//! Books[]     bookNo u8, pad u8, chapterCount u16, firstChapter u32,
//!             osisStr u32, abbrevStr u32, nameStr u32
//! Chapters[]  verseCount u16, pad u16, firstVerse u32, titleStr u32
//! Verses[n+1] u32 offset inn i tekstblokka; siste er sentinel og gir lengden på vers n
//! Numbers[n]  start u16, end u16 - versnummeret deltreet dekker
//! Strings     u16 lengdeprefiks + UTF-8
//! Text        all verstekst konkatenert, UTF-8
//! ```
//!
//! Kapitler er sammenhengende og starter alltid på 1. Vers gjør det stort sett
//! også, men ikke alltid: Bibel 2024 slår sammen enkelte versepar («15-16»), og da
//! finnes ikke det andre nummeret for seg. Derfor bærer hvert vers intervallet det
//! dekker, og et oppslag på 1,16 finner verset som er merket 15-16.
//!
//! Hver fil holder én oversettelse og bærer sin egen identitet (`id`, `title`,
//! `copyright`), slik at en kjørbar kan bygge inn flere filer og vite hva hver av
//! dem er uten en sidestilt tabell som kan komme ut av synk.

use std::fmt;

pub mod books;

pub const MAGIC: [u8; 4] = *b"NB88";
pub const VERSION: u16 = 3;

const HEADER_LEN: usize = 56;
const BOOK_LEN: usize = 20;
const CHAPTER_LEN: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatError {
    TooShort,
    BadMagic,
    BadVersion(u16),
    Corrupt(&'static str),
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatError::TooShort => write!(f, "datafila er for kort"),
            FormatError::BadMagic => write!(f, "ukjent filsignatur"),
            FormatError::BadVersion(v) => write!(f, "ustøttet formatversjon {v}"),
            FormatError::Corrupt(what) => write!(f, "ødelagt datafil: {what}"),
        }
    }
}

impl std::error::Error for FormatError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Book<'a> {
    pub index: usize,
    pub book_no: u8,
    pub chapter_count: u16,
    pub osis_id: &'a str,
    pub abbrev: &'a str,
    pub name: &'a str,
    first_chapter: u32,
}

/// Ett vers. `start` og `end` er like for vanlige vers; for sammenslåtte vers
/// dekker de intervallet, f.eks. 15-16.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Verse<'a> {
    pub start: u16,
    pub end: u16,
    pub text: &'a str,
}

impl Verse<'_> {
    /// «15» eller «15-16» - slik verset skal vises.
    pub fn label(&self) -> String {
        if self.end > self.start {
            format!("{}-{}", self.start, self.end)
        } else {
            self.start.to_string()
        }
    }

    pub fn covers(&self, number: u16) -> bool {
        self.start <= number && number <= self.end
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chapter<'a> {
    pub number: u16,
    pub verse_count: u16,
    pub title: &'a str,
    first_verse: u32,
}

/// Leser over et ferdig pakket datasett.
#[derive(Clone, Copy)]
pub struct Bible<'a> {
    data: &'a [u8],
    book_count: usize,
    chapter_count: usize,
    verse_count: usize,
    books_off: usize,
    chapters_off: usize,
    verses_off: usize,
    strings_off: usize,
    text_off: usize,
    numbers_off: usize,
    id_str: u32,
    title_str: u32,
    copyright_str: u32,
}

impl fmt::Debug for Bible<'_> {
    // Egen implementasjon: et derive ville skrevet ut hele tekstblokka.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Bible")
            .field("id", &self.id())
            .field("books", &self.book_count)
            .field("chapters", &self.chapter_count)
            .field("verses", &self.verse_count)
            .field("bytes", &self.data.len())
            .finish()
    }
}

fn u16_at(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(o..o + 2)?.try_into().ok()?))
}

fn u32_at(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?))
}

impl<'a> Bible<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, FormatError> {
        if data.len() < HEADER_LEN {
            return Err(FormatError::TooShort);
        }
        if data[0..4] != MAGIC {
            return Err(FormatError::BadMagic);
        }
        let version = u16_at(data, 4).ok_or(FormatError::TooShort)?;
        if version != VERSION {
            return Err(FormatError::BadVersion(version));
        }
        let get = |o| u32_at(data, o).ok_or(FormatError::TooShort);
        let bible = Bible {
            data,
            book_count: u16_at(data, 6).ok_or(FormatError::TooShort)? as usize,
            chapter_count: get(8)? as usize,
            verse_count: get(12)? as usize,
            books_off: get(16)? as usize,
            chapters_off: get(20)? as usize,
            verses_off: get(24)? as usize,
            strings_off: get(28)? as usize,
            text_off: get(32)? as usize,
            numbers_off: get(52)? as usize,
            id_str: get(40)?,
            title_str: get(44)?,
            copyright_str: get(48)?,
        };
        let text_len = get(36)? as usize;

        // Sjekk at hver seksjon ligger innenfor fila, slik at oppslag senere bare
        // kan feile på ødelagte interne offsets - ikke på lengden.
        let end = bible
            .books_off
            .checked_add(bible.book_count * BOOK_LEN)
            .filter(|&e| e <= bible.chapters_off)
            .and_then(|_| bible.chapters_off.checked_add(bible.chapter_count * CHAPTER_LEN))
            .filter(|&e| e <= bible.verses_off)
            .and_then(|_| bible.verses_off.checked_add((bible.verse_count + 1) * 4))
            .filter(|&e| e <= bible.numbers_off)
            .and_then(|_| bible.numbers_off.checked_add(bible.verse_count * 4))
            .filter(|&e| e <= bible.strings_off)
            .and_then(|_| bible.text_off.checked_add(text_len));
        match end {
            Some(e) if e <= data.len() && bible.strings_off <= bible.text_off => Ok(bible),
            _ => Err(FormatError::Corrupt("seksjonsoffsets utenfor fila")),
        }
    }

    /// Kort id, f.eks. «nb88» - det som velger oversettelse i en referanse.
    pub fn id(&self) -> &'a str {
        self.string_at(self.id_str).unwrap_or("")
    }

    /// Fullt navn, f.eks. «Norsk Bibel 88/07».
    pub fn title(&self) -> &'a str {
        self.string_at(self.title_str).unwrap_or("")
    }

    pub fn copyright(&self) -> &'a str {
        self.string_at(self.copyright_str).unwrap_or("")
    }

    pub fn book_count(&self) -> usize {
        self.book_count
    }

    pub fn chapter_count(&self) -> usize {
        self.chapter_count
    }

    pub fn verse_count(&self) -> usize {
        self.verse_count
    }

    fn string_at(&self, rel: u32) -> Option<&'a str> {
        let o = self.strings_off.checked_add(rel as usize)?;
        let len = u16_at(self.data, o)? as usize;
        let start = o.checked_add(2)?;
        std::str::from_utf8(self.data.get(start..start.checked_add(len)?)?).ok()
    }

    pub fn book(&self, index: usize) -> Option<Book<'a>> {
        if index >= self.book_count {
            return None;
        }
        let o = self.books_off + index * BOOK_LEN;
        Some(Book {
            index,
            book_no: *self.data.get(o)?,
            chapter_count: u16_at(self.data, o + 2)?,
            first_chapter: u32_at(self.data, o + 4)?,
            osis_id: self.string_at(u32_at(self.data, o + 8)?)?,
            abbrev: self.string_at(u32_at(self.data, o + 12)?)?,
            name: self.string_at(u32_at(self.data, o + 16)?)?,
        })
    }

    pub fn books(&self) -> impl Iterator<Item = Book<'a>> + '_ {
        (0..self.book_count).filter_map(move |i| self.book(i))
    }

    /// Kapittel `number` (1-basert) i `book`.
    pub fn chapter(&self, book: &Book<'a>, number: u16) -> Option<Chapter<'a>> {
        if number == 0 || number > book.chapter_count {
            return None;
        }
        let idx = book.first_chapter as usize + (number - 1) as usize;
        if idx >= self.chapter_count {
            return None;
        }
        let o = self.chapters_off + idx * CHAPTER_LEN;
        Some(Chapter {
            number,
            verse_count: u16_at(self.data, o)?,
            first_verse: u32_at(self.data, o + 4)?,
            title: self.string_at(u32_at(self.data, o + 8)?)?,
        })
    }

    /// Verset på plass `index` (0-basert) i kapitlet.
    pub fn verse_at(&self, chapter: &Chapter<'a>, index: u16) -> Option<Verse<'a>> {
        if index >= chapter.verse_count {
            return None;
        }
        let idx = chapter.first_verse as usize + index as usize;
        if idx >= self.verse_count {
            return None;
        }
        let o = self.verses_off + idx * 4;
        let from = u32_at(self.data, o)? as usize;
        let to = u32_at(self.data, o + 4)? as usize;
        if to < from {
            return None;
        }
        let n = self.numbers_off + idx * 4;
        Some(Verse {
            start: u16_at(self.data, n)?,
            end: u16_at(self.data, n + 2)?,
            text: std::str::from_utf8(
                self.data
                    .get(self.text_off.checked_add(from)?..self.text_off.checked_add(to)?)?,
            )
            .ok()?,
        })
    }

    /// Verset som dekker `number`. For sammenslåtte vers gir både 15 og 16
    /// verset som er merket 15-16.
    pub fn verse(&self, chapter: &Chapter<'a>, number: u16) -> Option<Verse<'a>> {
        if number == 0 {
            return None;
        }
        // Nesten alle kapitler er sammenhengende, så prøv den direkte plassen først.
        if let Some(v) = number
            .checked_sub(1)
            .and_then(|i| self.verse_at(chapter, i))
            .filter(|v| v.covers(number))
        {
            return Some(v);
        }
        // Ellers binærsøk; versene ligger alltid i stigende rekkefølge.
        let (mut lo, mut hi) = (0u16, chapter.verse_count);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let v = self.verse_at(chapter, mid)?;
            if v.covers(number) {
                return Some(v);
            }
            if number < v.start {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        None
    }

    /// Alle vers i et kapittel, i rekkefølge.
    pub fn chapter_verses(&self, chapter: &Chapter<'a>) -> impl Iterator<Item = Verse<'a>> + '_ {
        let chapter = *chapter;
        (0..chapter.verse_count).filter_map(move |i| self.verse_at(&chapter, i))
    }
}

/// Bygger et datasett. Brukes av `bibel-gen` i bible-repoet, og av testene her.
pub mod build {
    use super::{BOOK_LEN, CHAPTER_LEN, HEADER_LEN, MAGIC, VERSION};
    use std::collections::HashMap;

    struct BuildBook {
        book_no: u8,
        osis_id: String,
        abbrev: String,
        name: String,
        chapters: Vec<BuildChapter>,
    }

    struct BuildChapter {
        title: String,
        verses: Vec<BuildVerse>,
    }

    /// Ett vers på vei inn. `end` er lik `start` unntatt for sammenslåtte vers.
    pub struct BuildVerse {
        pub start: u16,
        pub end: u16,
        pub text: String,
    }

    impl BuildVerse {
        pub fn new(number: u16, text: impl Into<String>) -> Self {
            BuildVerse { start: number, end: number, text: text.into() }
        }

        pub fn range(start: u16, end: u16, text: impl Into<String>) -> Self {
            BuildVerse { start, end: end.max(start), text: text.into() }
        }
    }

    #[derive(Default)]
    pub struct Builder {
        id: String,
        title: String,
        copyright: String,
        books: Vec<BuildBook>,
    }

    fn intern<'s>(pool: &mut Vec<u8>, map: &mut HashMap<&'s str, u32>, s: &'s str) -> u32 {
        if let Some(&off) = map.get(s) {
            return off;
        }
        let off = pool.len() as u32;
        let len = u16::try_from(s.len()).expect("streng over 65535 byte");
        pool.extend_from_slice(&len.to_le_bytes());
        pool.extend_from_slice(s.as_bytes());
        map.insert(s, off);
        off
    }

    impl Builder {
        pub fn new(id: &str, title: &str, copyright: &str) -> Self {
            Builder {
                id: id.to_string(),
                title: title.to_string(),
                copyright: copyright.to_string(),
                ..Builder::default()
            }
        }

        pub fn id(&self) -> &str {
            &self.id
        }

        pub fn title(&self) -> &str {
            &self.title
        }

        pub fn add_book(&mut self, book_no: u8, osis_id: &str, abbrev: &str, name: &str) -> usize {
            self.books.push(BuildBook {
                book_no,
                osis_id: osis_id.to_string(),
                abbrev: abbrev.to_string(),
                name: name.to_string(),
                chapters: Vec::new(),
            });
            self.books.len() - 1
        }

        pub fn add_chapter(&mut self, book: usize, title: &str, verses: Vec<BuildVerse>) {
            self.books[book].chapters.push(BuildChapter {
                title: title.to_string(),
                verses,
            });
        }

        pub fn book_count(&self) -> usize {
            self.books.len()
        }

        pub fn chapter_count(&self) -> usize {
            self.books.iter().map(|b| b.chapters.len()).sum()
        }

        pub fn verse_count(&self) -> usize {
            self.books
                .iter()
                .flat_map(|b| b.chapters.iter())
                .map(|c| c.verses.len())
                .sum()
        }

        /// Verset på plass `index` slik det ble lagt inn - brukt av selvtesten.
        pub fn verse_at(&self, book: usize, chapter: u16, index: u16) -> Option<(u16, u16, &str)> {
            let c = self.books.get(book)?.chapters.get(chapter.checked_sub(1)? as usize)?;
            c.verses
                .get(index as usize)
                .map(|v| (v.start, v.end, v.text.as_str()))
        }

        pub fn chapter_title(&self, book: usize, chapter: u16) -> Option<&str> {
            let c = self.books.get(book)?.chapters.get(chapter.checked_sub(1)? as usize)?;
            Some(c.title.as_str())
        }

        pub fn chapters_in(&self, book: usize) -> usize {
            self.books.get(book).map_or(0, |b| b.chapters.len())
        }

        pub fn verses_in(&self, book: usize, chapter: u16) -> usize {
            self.books
                .get(book)
                .and_then(|b| chapter.checked_sub(1).and_then(|i| b.chapters.get(i as usize)))
                .map_or(0, |c| c.verses.len())
        }

        pub fn finish(&self) -> Vec<u8> {
            let mut strings: Vec<u8> = Vec::new();
            let mut interned: HashMap<&str, u32> = HashMap::new();
            let mut text: Vec<u8> = Vec::new();
            let mut verse_offsets: Vec<u32> = Vec::new();
            let mut verse_numbers: Vec<[u8; 4]> = Vec::new();
            let mut chapter_records: Vec<[u8; CHAPTER_LEN]> = Vec::new();
            let mut book_records: Vec<[u8; BOOK_LEN]> = Vec::new();

            for book in &self.books {
                let first_chapter = chapter_records.len() as u32;
                for chapter in &book.chapters {
                    let first_verse = verse_offsets.len() as u32;
                    for verse in &chapter.verses {
                        verse_offsets.push(text.len() as u32);
                        text.extend_from_slice(verse.text.as_bytes());
                        let mut nums = [0u8; 4];
                        nums[0..2].copy_from_slice(&verse.start.to_le_bytes());
                        nums[2..4].copy_from_slice(&verse.end.to_le_bytes());
                        verse_numbers.push(nums);
                    }
                    let title = intern(&mut strings, &mut interned, &chapter.title);
                    let mut rec = [0u8; CHAPTER_LEN];
                    rec[0..2].copy_from_slice(
                        &u16::try_from(chapter.verses.len())
                            .expect("over 65535 vers i ett kapittel")
                            .to_le_bytes(),
                    );
                    rec[4..8].copy_from_slice(&first_verse.to_le_bytes());
                    rec[8..12].copy_from_slice(&title.to_le_bytes());
                    chapter_records.push(rec);
                }

                let osis = intern(&mut strings, &mut interned, &book.osis_id);
                let abbrev = intern(&mut strings, &mut interned, &book.abbrev);
                let name = intern(&mut strings, &mut interned, &book.name);
                let mut rec = [0u8; BOOK_LEN];
                rec[0] = book.book_no;
                rec[2..4].copy_from_slice(
                    &u16::try_from(book.chapters.len())
                        .expect("over 65535 kapitler i én bok")
                        .to_le_bytes(),
                );
                rec[4..8].copy_from_slice(&first_chapter.to_le_bytes());
                rec[8..12].copy_from_slice(&osis.to_le_bytes());
                rec[12..16].copy_from_slice(&abbrev.to_le_bytes());
                rec[16..20].copy_from_slice(&name.to_le_bytes());
                book_records.push(rec);
            }
            // Sentinel, slik at lengden på siste vers kan leses likt som de andre.
            verse_offsets.push(text.len() as u32);

            let id_str = intern(&mut strings, &mut interned, &self.id);
            let title_str = intern(&mut strings, &mut interned, &self.title);
            let copyright_str = intern(&mut strings, &mut interned, &self.copyright);

            let books_off = HEADER_LEN;
            let chapters_off = books_off + book_records.len() * BOOK_LEN;
            let verses_off = chapters_off + chapter_records.len() * CHAPTER_LEN;
            let numbers_off = verses_off + verse_offsets.len() * 4;
            let strings_off = numbers_off + verse_numbers.len() * 4;
            let text_off = strings_off + strings.len();

            let mut out = Vec::with_capacity(text_off + text.len());
            out.extend_from_slice(&MAGIC);
            out.extend_from_slice(&VERSION.to_le_bytes());
            out.extend_from_slice(&(book_records.len() as u16).to_le_bytes());
            out.extend_from_slice(&(chapter_records.len() as u32).to_le_bytes());
            out.extend_from_slice(&((verse_offsets.len() - 1) as u32).to_le_bytes());
            for off in [books_off, chapters_off, verses_off, strings_off, text_off] {
                out.extend_from_slice(&(off as u32).to_le_bytes());
            }
            out.extend_from_slice(&(text.len() as u32).to_le_bytes());
            for offset in [id_str, title_str, copyright_str] {
                out.extend_from_slice(&offset.to_le_bytes());
            }
            out.extend_from_slice(&(numbers_off as u32).to_le_bytes());
            debug_assert_eq!(out.len(), HEADER_LEN);

            for rec in &book_records {
                out.extend_from_slice(rec);
            }
            for rec in &chapter_records {
                out.extend_from_slice(rec);
            }
            for off in &verse_offsets {
                out.extend_from_slice(&off.to_le_bytes());
            }
            for nums in &verse_numbers {
                out.extend_from_slice(nums);
            }
            out.extend_from_slice(&strings);
            out.extend_from_slice(&text);
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::build::{BuildVerse, Builder};
    use super::*;

    fn sample() -> Vec<u8> {
        let mut b = Builder::new("nb88", "Norsk Bibel 88/07", "Norsk Bibel AS");
        let gen = b.add_book(1, "Gen", "1mos", "1. Mosebok");
        b.add_chapter(
            gen,
            "Skapelsen",
            vec![BuildVerse::new(1, "første"), BuildVerse::new(2, "andre")],
        );
        b.add_chapter(gen, "", vec![BuildVerse::new(1, "tredje")]);
        let rev = b.add_book(66, "Rev", "aap", "Åpenbaringen");
        b.add_chapter(rev, "Siste", vec![BuildVerse::new(1, "ærlig talt æøå")]);
        b.finish()
    }

    #[test]
    fn round_trip() {
        let bytes = sample();
        let bible = Bible::parse(&bytes).unwrap();
        assert_eq!(bible.id(), "nb88");
        assert_eq!(bible.title(), "Norsk Bibel 88/07");
        assert_eq!(bible.copyright(), "Norsk Bibel AS");
        assert_eq!(bible.book_count(), 2);
        assert_eq!(bible.chapter_count(), 3);
        assert_eq!(bible.verse_count(), 4);

        let gen = bible.book(0).unwrap();
        assert_eq!((gen.osis_id, gen.abbrev, gen.name), ("Gen", "1mos", "1. Mosebok"));
        assert_eq!(gen.chapter_count, 2);

        let c1 = bible.chapter(&gen, 1).unwrap();
        assert_eq!(c1.title, "Skapelsen");
        assert_eq!(bible.verse(&c1, 1).unwrap().text, "første");
        assert_eq!(bible.verse(&c1, 2).unwrap().text, "andre");
        assert_eq!(bible.verse(&c1, 3), None);
        assert_eq!(bible.verse(&c1, 0), None);

        let c2 = bible.chapter(&gen, 2).unwrap();
        assert_eq!(bible.verse(&c2, 1).unwrap().text, "tredje");
        assert_eq!(bible.chapter(&gen, 3), None);

        // Siste vers i fila leses via sentinelen.
        let rev = bible.book(1).unwrap();
        let last = bible.chapter(&rev, 1).unwrap();
        assert_eq!(bible.verse(&last, 1).unwrap().text, "ærlig talt æøå");

        let names: Vec<_> = bible.books().map(|b| b.name).collect();
        assert_eq!(names, vec!["1. Mosebok", "Åpenbaringen"]);
    }

    #[test]
    fn sammenslatte_vers_og_hull() {
        // Som Efeserne 1 i Bibel 2024: vers 15-16 står som én enhet, og 16
        // finnes ikke for seg.
        let mut b = Builder::new("nb24", "Bibel 2024", "");
        let eph = b.add_book(49, "Eph", "ef", "Efeserne");
        b.add_chapter(
            eph,
            "",
            vec![
                BuildVerse::new(14, "fjorten"),
                BuildVerse::range(15, 16, "femten og seksten"),
                BuildVerse::new(17, "sytten"),
            ],
        );
        let bytes = b.finish();
        let bible = Bible::parse(&bytes).unwrap();
        let book = bible.book(0).unwrap();
        let chapter = bible.chapter(&book, 1).unwrap();

        // Både 15 og 16 skal finne det sammenslåtte verset.
        for n in [15, 16] {
            let v = bible.verse(&chapter, n).unwrap_or_else(|| panic!("vers {n}"));
            assert_eq!(v.text, "femten og seksten");
            assert_eq!(v.label(), "15-16");
        }
        // Naboene skal ikke bli påvirket av binærsøket.
        assert_eq!(bible.verse(&chapter, 14).unwrap().label(), "14");
        assert_eq!(bible.verse(&chapter, 17).unwrap().text, "sytten");
        // Utenfor kapitlet.
        assert_eq!(bible.verse(&chapter, 13), None);
        assert_eq!(bible.verse(&chapter, 18), None);

        let labels: Vec<_> = bible.chapter_verses(&chapter).map(|v| v.label()).collect();
        assert_eq!(labels, vec!["14", "15-16", "17"]);
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(Bible::parse(b"kort").unwrap_err(), FormatError::TooShort);
        let mut bytes = sample();
        bytes[0] = b'X';
        assert_eq!(Bible::parse(&bytes).unwrap_err(), FormatError::BadMagic);
        let mut bytes = sample();
        bytes[4] = 1; // v1-fil mot v3-leser
        assert!(matches!(Bible::parse(&bytes), Err(FormatError::BadVersion(_))));
        let bytes = sample();
        assert!(Bible::parse(&bytes[..HEADER_LEN + 4]).is_err());
    }
}
