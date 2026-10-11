//! Word-importen (`word::convert`). Testene bruker en liten falsk bibel, så de
//! trenger ikke oversettelsene.

use std::collections::HashMap;

use kerygma_core::word::align::words;
use kerygma_core::word::bible_text::{BibleText, VerseId};
use kerygma_core::word::convert::{convert, ConvertOptions, ConvertResult};
use kerygma_core::word::docx::{Paragraph, Run};

/// En liten bibel med versene testene trenger, i stedet for `bibel` og *.bin.
/// Bok-id, navnet `bibel` gir boka, og versene som ("kapittel:vers", tekst).
type Book = (&'static str, &'static str, Vec<(&'static str, &'static str)>);

struct FakeBible {
    verses: Vec<Book>,
    books: HashMap<&'static str, &'static str>,
    translations: Vec<String>,
}

fn fake_bible() -> FakeBible {
    FakeBible {
        // Versene er fra Bibelen 1930 (nb30), som er i det fri.
        verses: vec![
            ("Jer", "Jeremias bok", vec![("7:4", "Sett ikke eders lit til løgnaktige ord, når folk sier: Her er Herrens tempel, Herrens tempel, Herrens tempel!")]),
            (
                "Dan",
                "Daniels bok",
                vec![
                    ("1:1", "I Judas konge Jojakims tredje regjeringsår drog Nebukadnesar, kongen i Babel, til Jerusalem og kringsatte det."),
                    ("1:2", "Og Herren gav Judas konge Jojakim i hans hånd, og likeledes en del av karene i Guds hus, og han førte dem til Sinears land, til sin guds hus - han lot karene sette inn i sin guds skattkammer."),
                    ("1:9", "Og Gud lot Daniel finne velvilje og barmhjertighet hos den øverste hoffmann."),
                    ("1:10", "Og den øverste hoffmann sa til Daniel: Jeg er redd for at min herre kongen, som har fastsatt hvad I skal ete og drikke, vil synes at eders ansikter ser dårligere ut enn de gutters som er på eders alder, og at I således skal føre skyld over mitt hode hos kongen."),
                ],
            ),
            ("Rom", "Paulus' brev til romerne", vec![("4:25", "han som blev gitt for våre overtredelser og opreist til vår rettferdiggjørelse.")]),
            ("Isa", "Jesajas bok", vec![("62:2", "Og folkene skal se din rettferdighet, og alle konger din herlighet, og du skal kalles med et nytt navn, som Herrens munn skal nevne.")]),
            ("John", "Johannes' evangelium", vec![("16:13", "men når han, sannhetens Ånd, kommer, skal han veilede eder til hele sannheten; for han skal ikke tale av sig selv, men det som han hører, skal han tale, og de tilkommende ting skal han forkynne eder.")]),
        ],
        books: [("jer", "Jer"), ("dan", "Dan"), ("daniel", "Dan"), ("rom", "Rom"), ("jes", "Isa"), ("joh", "John")].into(),
        translations: vec!["nb30".to_string()],
    }
}

impl BibleText for FakeBible {
    fn translations(&self) -> &[String] {
        &self.translations
    }

    fn book_id(&self, name: &str) -> Option<String> {
        self.books.get(name.to_lowercase().as_str()).map(|id| id.to_string())
    }

    fn verse(&self, _: &str, id: &VerseId) -> Option<String> {
        let (_, _, verses) = self.verses.iter().find(|(book, _, _)| *book == id.book)?;
        let at = format!("{}:{}", id.chapter, id.verse);
        verses.iter().find(|(v, _)| *v == at).map(|(_, text)| text.to_string())
    }

    fn reference(&self, book: &str, chapter: u32, from: u32, to: u32) -> Option<String> {
        let (_, name, _) = self.verses.iter().find(|(id, _, _)| *id == book)?;
        let verses = if from == to { from.to_string() } else { format!("{from}-{to}") };
        Some(format!("{name} {chapter}:{verses}"))
    }

    fn search(&self, _: &str, query: &[String]) -> Vec<VerseId> {
        let mut hits = Vec::new();
        for (book, _, verses) in &self.verses {
            for (at, text) in verses {
                let found = words(text);
                if query.iter().filter(|w| found.contains(w)).count() >= 3 {
                    let (chapter, verse) = at.split_once(':').unwrap();
                    hits.push(VerseId { book: book.to_string(), chapter: chapter.parse().unwrap(), verse: verse.parse().unwrap() });
                }
            }
        }
        hits
    }
}

fn paragraph(text: &str) -> Paragraph {
    styled(text, false)
}

fn styled(text: &str, bold: bool) -> Paragraph {
    let runs = if text.is_empty() { vec![] } else { vec![Run { text: text.to_string(), bold, italic: false }] };
    Paragraph { runs, style: None, list_item: false }
}

fn run(paragraphs: Vec<Paragraph>) -> ConvertResult {
    convert(&paragraphs, &fake_bible(), &ConvertOptions::default())
}

fn markdown(texts: &[&str]) -> String {
    run(texts.iter().map(|t| paragraph(t)).collect()).markdown
}

#[test]
fn referanse_alene_i_et_avsnitt_foran_sitatet() {
    assert_eq!(
        markdown(&["Jer 7,4", "Sett ikke eders lit til løgnaktige ord, når folk sier: Her er Herrens tempel, Herrens tempel, Herrens tempel!"]),
        "> Sett ikke eders lit til løgnaktige ord, når folk sier: Her er Herrens tempel, Herrens tempel, Herrens tempel! (Jeremias bok 7:4)\n"
    );
}

#[test]
fn referanse_foran_sitatet_i_samme_avsnitt() {
    assert_eq!(
        markdown(&["Det står i Jer 7:4 Sett ikke eders lit til løgnaktige ord, når folk sier: Her er Herrens tempel, Herrens tempel, Herrens tempel!"]),
        "Det står i Jer 7:4\n\n> Sett ikke eders lit til løgnaktige ord, når folk sier: Her er Herrens tempel, Herrens tempel, Herrens tempel! (Jeremias bok 7:4)\n"
    );
}

#[test]
fn innledning_som_slutter_med_komma_far_kolon_og_referansen_flyttes_til_sitatet() {
    assert_eq!(
        markdown(&["Profeten advarte dem, Jer 7,4 Sett ikke eders lit til løgnaktige ord, når folk sier: Her er Herrens tempel, Herrens tempel, Herrens tempel!"]),
        "Profeten advarte dem:\n\n> Sett ikke eders lit til løgnaktige ord, når folk sier: Her er Herrens tempel, Herrens tempel, Herrens tempel! (Jeremias bok 7:4)\n"
    );
}

#[test]
fn referanse_som_henger_igjen_etter_avsnittet_for_blir_staende_der() {
    assert_eq!(
        markdown(&["Jeremia sier i", "Jer 7,4", "Sett ikke eders lit til løgnaktige ord, når folk sier: Her er Herrens tempel, Herrens tempel!"]),
        "Jeremia sier i Jer 7,4\n\n> Sett ikke eders lit til løgnaktige ord, når folk sier: Her er Herrens tempel, Herrens tempel! (Jeremias bok 7:4)\n"
    );
}

#[test]
fn v1_bruker_boka_og_kapitlet_fra_overskriften_og_sitatet_kan_ga_over_flere_avsnitt_og_vers() {
    let result = run(vec![
        styled("Daniels bok, kapittel 1, del 1", true),
        paragraph("V1"),
        paragraph("I Judas konge Jojakims tredje regjeringsår dro Nebukadnesar, kongen i Babel, til Jerusalem og kringsatte byen."),
        paragraph("Og Herren gav Judas konge Jojakim i hans hånd, og likeledes en del av karene i Guds hus."),
        paragraph(""),
        paragraph("Det var vanskelige tider for Guds folk."),
    ]);
    assert_eq!(
        result.markdown,
        "## Daniels bok, kapittel 1, del 1\n\n".to_string()
            + "> I Judas konge Jojakims tredje regjeringsår dro Nebukadnesar, kongen i Babel, til Jerusalem og kringsatte byen. "
            + "Og Herren gav Judas konge Jojakim i hans hånd, og likeledes en del av karene i Guds hus. (Daniels bok 1:1-2)\n\n"
            + "Det var vanskelige tider for Guds folk.\n"
    );
}

#[test]
fn en_referanse_alene_foran_et_fritt_gjengitt_vers_blir_likevel_et_sitat() {
    assert_eq!(
        markdown(&["Dan 1:9", "Gud gjorde at den øverste hoffmann fikk velvilje og godhet for Daniel.", "Han belønnet beslutningen."]),
        "> [!warning|sjekk] Få ord stemmer med bibelteksten. Sjekk at dette er Dan 1:9.\n".to_string()
            + "> Gud gjorde at den øverste hoffmann fikk velvilje og godhet for Daniel. (Daniels bok 1:9)\n\nHan belønnet beslutningen.\n"
    );
}

#[test]
fn sitat_foran_referansen_i_samme_setning() {
    assert_eq!(
        markdown(&["Det er samme ordet som er brukt om at Jesus «blev gitt for våre overtredelser og opreist til vår rettferdiggjørelse” i Rom 4:25"]),
        "Det er samme ordet som er brukt om at Jesus\n\n".to_string()
            + "> [!warning|sjekk] Skilt ut fra setningen foran Rom 4:25. Sjekk hvor sitatet starter.\n"
            + "> blev gitt for våre overtredelser og opreist til vår rettferdiggjørelse (Paulus' brev til romerne 4:25)\n"
    );
}

#[test]
fn oversettelsen_etter_sitatet_som_n78_fjernes() {
    assert_eq!(
        markdown(&["Dan 1:9 Og Gud lot Daniel finne velvilje og barmhjertighet hos den øverste hoffmann. (N78)"]),
        "> Og Gud lot Daniel finne velvilje og barmhjertighet hos den øverste hoffmann. (Daniels bok 1:9)\n"
    );
}

#[test]
fn referanse_uten_sitat_blir_staende_som_tekst() {
    let result = run(vec![paragraph("DHÅ skal også lede oss til hele sannheten. Joh 16:13")]);
    assert_eq!(result.markdown, "DHÅ skal også lede oss til hele sannheten. Joh 16:13\n");
    assert_eq!(result.unmatched, ["Joh 16:13"]);
}

#[test]
fn vers_uten_referanse_finnes_med_sok() {
    assert_eq!(
        markdown(&["Jesaja sier", "", "Du skal kalles med et nytt navn, som Herrens munn skal nevne."]),
        "Jesaja sier\n\n".to_string()
            + "> [!warning|sjekk] Funnet med søk i Bibelen. Manuset har ingen referanse.\n"
            + "> Du skal kalles med et nytt navn, som Herrens munn skal nevne. (Jesajas bok 62:2)\n"
    );
}

#[test]
fn fet_skrift_blir_stjerner_med_tegnsetting_utenfor() {
    let result = run(vec![Paragraph {
        runs: vec![
            Run { text: "Gud tok vare på sine barn".into(), bold: false, italic: false },
            Run { text: ", og ingen gikk tapt.".into(), bold: true, italic: false },
        ],
        style: None,
        list_item: false,
    }]);
    assert_eq!(result.markdown, "Gud tok vare på sine barn, **og ingen gikk tapt**.\n");
}

#[test]
fn sitat_som_starter_pa_et_annet_vers_enn_referansen_merkes() {
    let result = run(vec![
        paragraph("Dan 1:9"),
        paragraph("Og den øverste hoffmann sa til Daniel: Jeg er redd for at min herre kongen vil synes at eders ansikter ser dårligere ut."),
    ]);
    assert_eq!(
        result.markdown,
        "> [!warning|sjekk] Manuset viser til «Dan 1:9». Sjekk versene.\n".to_string()
            + "> Og den øverste hoffmann sa til Daniel: Jeg er redd for at min herre kongen vil synes at eders ansikter ser dårligere ut. (Daniels bok 1:10)\n"
    );
    assert_eq!(result.quotes[0].warning.as_deref(), Some("Manuset viser til «Dan 1:9». Sjekk versene."));
}

#[test]
fn avsnitt_delt_midt_i_en_setning_slas_sammen() {
    assert_eq!(markdown(&["Det er vel ikke", "alt som er i harmoni med Guds ord."]), "Det er vel ikke alt som er i harmoni med Guds ord.\n");
}
