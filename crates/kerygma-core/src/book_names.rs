//! Boknavnene `reference.rs` kjenner igjen.

// `BOOK_IDS`: navn og forkortelse for hver bok i hver oversettelse i `bibler/`, laget av
// build.rs når appen bygges.
include!(concat!(env!("OUT_DIR"), "/book_names.rs"));

/// Vanlige norske boknavn og forkortelser i manus, som ikke er navnene en oversettelse
/// bruker: «1. Mosebok 3:5», «Efeserbrevet 2,8», «Åp 21,4». Navnene fra oversettelsene står
/// i [`BOOK_IDS`] og går foran ved like nøkler. Bok-id-ene er OSIS, som `bibel` slår opp på
/// i alle oversettelsene.
pub const NORWEGIAN_NAMES: &[(&str, &str)] = &[
    // Mosebøkene og de historiske bøkene
    ("1. Mosebok", "Gen"), ("2. Mosebok", "Exod"), ("3. Mosebok", "Lev"), ("4. Mosebok", "Num"), ("5. Mosebok", "Deut"),
    ("Josva", "Josh"), ("Dommerne", "Judg"),
    ("1. Samuelsbok", "1Sam"), ("2. Samuelsbok", "2Sam"), ("1. Kongebok", "1Kgs"), ("2. Kongebok", "2Kgs"),
    ("1. Krønikebok", "1Chr"), ("2. Krønikebok", "2Chr"), ("1 Krøn", "1Chr"), ("2 Krøn", "2Chr"),
    ("Esra", "Ezra"), ("Nehemja", "Neh"), ("Ester", "Esth"),
    // Poesien
    ("Salmene", "Ps"), ("Ordspråkene", "Prov"), ("Ordsp", "Prov"), ("Forkynneren", "Eccl"), ("Høys", "Song"),
    // Profetene
    ("Jesaja", "Isa"), ("Jeremia", "Jer"), ("Esekiel", "Ezek"), ("Am", "Amos"), ("Obadja", "Obad"), ("Ob", "Obad"),
    ("Jona", "Jonah"), ("Mi", "Mic"), ("Sefanja", "Zeph"), ("Sakarja", "Zech"), ("Malaki", "Mal"),
    // Evangeliene
    ("Matteus", "Matt"), ("Markus", "Mark"), ("Lukas", "Luke"), ("Johannes", "John"),
    // Brevene
    ("Romerne", "Rom"), ("Romerbrevet", "Rom"),
    ("1. Korinter", "1Cor"), ("2. Korinter", "2Cor"), ("1. Korinterbrev", "1Cor"), ("2. Korinterbrev", "2Cor"),
    ("Galaterne", "Gal"), ("Galaterbrevet", "Gal"), ("Efeserne", "Eph"), ("Efeserbrevet", "Eph"),
    ("Filipperne", "Phil"), ("Filipperbrevet", "Phil"), ("Kolosserne", "Col"), ("Kolosserbrevet", "Col"),
    ("1. Tessaloniker", "1Thess"), ("2. Tessaloniker", "2Thess"),
    ("1. Tessalonikerbrev", "1Thess"), ("2. Tessalonikerbrev", "2Thess"),
    ("1. Timoteus", "1Tim"), ("2. Timoteus", "2Tim"), ("1. Timoteusbrev", "1Tim"), ("2. Timoteusbrev", "2Tim"),
    ("Filemon", "Phlm"), ("Filemons brev", "Phlm"),
    ("Hebreerne", "Heb"), ("Hebreerbrevet", "Heb"), ("Hebr", "Heb"), ("Jakob", "Jas"),
    ("1. Peters brev", "1Pet"), ("2. Peters brev", "2Pet"),
    ("1. Johannes' brev", "1John"), ("2. Johannes' brev", "2John"), ("3. Johannes' brev", "3John"),
    ("Judas", "Jude"), ("Åp", "Rev"),
];
