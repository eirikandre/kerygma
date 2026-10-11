//! Formaterer et oppslag til tekst (`--format`, `--skille` osv.).

use bibel_format::{Bible, Book};

use crate::reference::{self, Resolved};
use crate::search::Hit;

pub struct RenderOpts {
    pub format: String,
    pub separator: String,
    pub keep_stars: bool,
    pub numbers: bool,
}

impl Default for RenderOpts {
    fn default() -> Self {
        RenderOpts {
            format: "{tekst} ({ref})".to_string(),
            separator: " ".to_string(),
            keep_stars: false,
            numbers: true,
        }
    }
}

/// «1. Samuelsbok 2:1-3», «1. Samuelsbok 2» eller «1. Samuelsbok 1:20-2:5».
pub fn reference_string(r: &Resolved<'_>) -> String {
    format!("{} {}", r.book.name, locus(r))
}

fn locus(r: &Resolved<'_>) -> String {
    if r.whole_chapter {
        format!("{}", r.start.0)
    } else if r.is_single_verse() {
        format!("{}:{}", r.start.0, r.start.1)
    } else if r.spans_chapters() {
        format!("{}:{}-{}:{}", r.start.0, r.start.1, r.end.0, r.end.1)
    } else {
        format!("{}:{}-{}", r.start.0, r.start.1, r.end.1)
    }
}

fn verse_part(r: &Resolved<'_>) -> String {
    if r.whole_chapter {
        String::new()
    } else if r.is_single_verse() {
        format!("{}", r.start.1)
    } else if r.spans_chapters() {
        format!("{}-{}:{}", r.start.1, r.end.0, r.end.1)
    } else {
        format!("{}-{}", r.start.1, r.end.1)
    }
}

fn clean(text: &str, keep_stars: bool) -> String {
    if keep_stars {
        text.to_string()
    } else {
        // `*` er fotnotemarkører mot studienotene, og står alltid inntil et ord.
        text.replace('*', "")
    }
}

pub fn render(
    bible: &Bible<'_>,
    r: &Resolved<'_>,
    opts: &RenderOpts,
    translation: &str,
) -> String {
    let verses = reference::verses(bible, r);
    let number_them = opts.numbers && verses.len() > 1;
    let cross = r.spans_chapters();

    let mut parts: Vec<String> = Vec::with_capacity(verses.len());
    for (chapter, verse) in &verses {
        let body = clean(verse.text, opts.keep_stars);
        // label() gir «15-16» for sammenslåtte vers, ellers bare tallet.
        let number = verse.label();
        if !number_them {
            parts.push(body);
        } else if cross {
            // Går intervallet over et kapittelskifte, må kapittelnummeret stå på
            // hvert vers - ellers er det ikke til å se om «2» er vers eller kapittel.
            parts.push(format!("{chapter}:{number} {body}"));
        } else {
            parts.push(format!("{number} {body}"));
        }
    }

    let chapter_title = bible
        .chapter(&r.book, r.start.0)
        .map_or("", |c| c.title);

    fill(
        &opts.format,
        &reference_string(r),
        &r.book,
        &r.start.0.to_string(),
        &verse_part(r),
        chapter_title,
        translation,
        &parts.join(&opts.separator),
    )
}

/// Ett søketreff. Referansen bærer både kapittel og vers, så teksten står uten
/// versnummer foran seg.
pub fn render_hit(hit: &Hit<'_>, opts: &RenderOpts, translation: &str) -> String {
    let number = hit.verse.label();
    fill(
        &opts.format,
        &format!("{} {}:{}", hit.book.name, hit.chapter.number, number),
        &hit.book,
        &hit.chapter.number.to_string(),
        &number,
        hit.chapter.title,
        translation,
        &clean(hit.verse.text, opts.keep_stars),
    )
}

/// Setter inn plassholderne. Metadata først, teksten til slutt - ellers ville en
/// verstekst som tilfeldigvis inneholdt «{ref}» blitt tolket som en plassholder.
#[allow(clippy::too_many_arguments)]
fn fill(
    format: &str,
    reference: &str,
    book: &Book<'_>,
    chapter: &str,
    verse: &str,
    title: &str,
    translation: &str,
    text: &str,
) -> String {
    format
        .replace("{ref}", reference)
        .replace("{bok}", book.name)
        .replace("{forkortelse}", book.abbrev)
        .replace("{kapittel}", chapter)
        .replace("{vers}", verse)
        .replace("{tittel}", title)
        .replace("{oversettelse}", translation)
        .replace("{tekst}", text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bibel_format::build::{BuildVerse, Builder};

    fn data() -> Vec<u8> {
        let mut b = Builder::new("test", "Testbibel", "");
        let sam = b.add_book(9, "1Sam", "1sam", "1. Samuelsbok");
        b.add_chapter(
            sam,
            "Elkana og Hanna",
            vec![BuildVerse::new(1, "a"), BuildVerse::new(2, "b*"), BuildVerse::new(3, "c")],
        );
        b.add_chapter(
            sam,
            "Hannas lovsang",
            vec![BuildVerse::new(1, "d"), BuildVerse::new(2, "e")],
        );
        b.finish()
    }

    fn out(input: &str, opts: &RenderOpts) -> String {
        let bytes = data();
        let bible = Bible::parse(&bytes).unwrap();
        let r = reference::resolve(&bible, input).unwrap();
        render(&bible, &r, opts, "Testbibel")
    }

    #[test]
    fn standardformat() {
        let o = RenderOpts::default();
        assert_eq!(out("1sam1:1-3", &o), "1 a 2 b 3 c (1. Samuelsbok 1:1-3)");
        // Ett vers: referansen sier alt, så versnummeret utelates i teksten.
        assert_eq!(out("1sam1:1", &o), "a (1. Samuelsbok 1:1)");
        assert_eq!(out("1sam2", &o), "1 d 2 e (1. Samuelsbok 2)");
        assert_eq!(out("1sam1:3-2:2", &o), "1:3 c 2:1 d 2:2 e (1. Samuelsbok 1:3-2:2)");
    }

    #[test]
    fn stjerner_og_flagg() {
        let mut o = RenderOpts::default();
        assert_eq!(out("1sam1:2", &o), "b (1. Samuelsbok 1:2)");
        o.keep_stars = true;
        assert_eq!(out("1sam1:2", &o), "b* (1. Samuelsbok 1:2)");

        let o = RenderOpts { numbers: false, ..RenderOpts::default() };
        assert_eq!(out("1sam1:1-3", &o), "a b c (1. Samuelsbok 1:1-3)");

        let o = RenderOpts { separator: "\n".into(), ..RenderOpts::default() };
        assert_eq!(out("1sam1:1-2", &o), "1 a\n2 b (1. Samuelsbok 1:1-2)");
    }

    #[test]
    fn sammenslatt_vers_vises_med_intervall() {
        let mut b = Builder::new("test", "Testbibel", "");
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
        let o = RenderOpts::default();

        let r = reference::resolve(&bible, "ef1:14-17").unwrap();
        assert_eq!(
            render(&bible, &r, &o, "Testbibel"),
            "14 fjorten 15-16 femten og seksten 17 sytten (Efeserne 1:14-17)"
        );
        // Oppslag på 16 alene gir det sammenslåtte verset, uten nabotekst.
        let r = reference::resolve(&bible, "ef1:16").unwrap();
        assert_eq!(render(&bible, &r, &o, "Testbibel"), "femten og seksten (Efeserne 1:16)");
    }

    #[test]
    fn plassholdere() {
        let o = RenderOpts {
            format: "{bok}|{forkortelse}|{kapittel}|{vers}|{tittel}|{ref}|{tekst}".into(),
            ..RenderOpts::default()
        };
        assert_eq!(
            out("1sam1:1-3", &o),
            "1. Samuelsbok|1sam|1|1-3|Elkana og Hanna|1. Samuelsbok 1:1-3|1 a 2 b 3 c"
        );
    }

    #[test]
    fn soketreff_bruker_samme_plassholdere() {
        let bytes = data();
        let bible = Bible::parse(&bytes).unwrap();
        let book = bible.book(0).unwrap();
        let chapter = bible.chapter(&book, 1).unwrap();
        let verse = bible.verse(&chapter, 2).unwrap();
        let hit = Hit { book, chapter, verse };

        let opts = RenderOpts::default();
        // Samme format som et oppslag: referansen med kapittel og vers i parentes
        // til slutt, og stjerna er borte som ellers.
        assert_eq!(render_hit(&hit, &opts, "Testbibel"), "b (1. Samuelsbok 1:2)");

        let opts = RenderOpts {
            format: "{bok}|{forkortelse}|{kapittel}|{vers}|{tittel}|{oversettelse}|{tekst}".into(),
            keep_stars: true,
            ..RenderOpts::default()
        };
        assert_eq!(
            render_hit(&hit, &opts, "Testbibel"),
            "1. Samuelsbok|1sam|1|2|Elkana og Hanna|Testbibel|b*"
        );
    }

    #[test]
    fn verstekst_tolkes_ikke_som_plassholder() {
        let mut b = Builder::new("test", "Testbibel", "");
        let id = b.add_book(1, "Gen", "1mos", "1. Mosebok");
        b.add_chapter(id, "", vec![BuildVerse::new(1, "{ref} og {bok}")]);
        let bytes = b.finish();
        let bible = Bible::parse(&bytes).unwrap();
        let r = reference::resolve(&bible, "1mos1:1").unwrap();
        assert_eq!(
            render(&bible, &r, &RenderOpts::default(), "Testbibel"),
            "{ref} og {bok} (1. Mosebok 1:1)"
        );
    }
}
