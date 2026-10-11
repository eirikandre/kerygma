//! Felles bokidentifikatorer.
//!
//! Datasettene bruker to ulike kodeverk for de samme bøkene: NB88 og KJV bruker
//! OSIS-id-er (`John`, `Ps`, `Song`), mens bibel.no bruker USFM (`JHN`, `PSA`,
//! `SNG`). Tabellen her knytter formene sammen, slik at `john3:16` slår opp i alle
//! oversettelsene uansett hvilket kodeverk den enkelte bruker.
//!
//! Paret er entydig: ingen USFM-id er lik en annen boks OSIS-id.

/// (USFM, OSIS) for de 66 bøkene, i kanonisk rekkefølge.
pub const BOOK_IDS: &[(&str, &str)] = &[
    ("GEN", "Gen"),
    ("EXO", "Exod"),
    ("LEV", "Lev"),
    ("NUM", "Num"),
    ("DEU", "Deut"),
    ("JOS", "Josh"),
    ("JDG", "Judg"),
    ("RUT", "Ruth"),
    ("1SA", "1Sam"),
    ("2SA", "2Sam"),
    ("1KI", "1Kgs"),
    ("2KI", "2Kgs"),
    ("1CH", "1Chr"),
    ("2CH", "2Chr"),
    ("EZR", "Ezra"),
    ("NEH", "Neh"),
    ("EST", "Esth"),
    ("JOB", "Job"),
    ("PSA", "Ps"),
    ("PRO", "Prov"),
    ("ECC", "Eccl"),
    ("SNG", "Song"),
    ("ISA", "Isa"),
    ("JER", "Jer"),
    ("LAM", "Lam"),
    ("EZK", "Ezek"),
    ("DAN", "Dan"),
    ("HOS", "Hos"),
    ("JOL", "Joel"),
    ("AMO", "Amos"),
    ("OBA", "Obad"),
    ("JON", "Jonah"),
    ("MIC", "Mic"),
    ("NAM", "Nah"),
    ("HAB", "Hab"),
    ("ZEP", "Zeph"),
    ("HAG", "Hag"),
    ("ZEC", "Zech"),
    ("MAL", "Mal"),
    ("MAT", "Matt"),
    ("MRK", "Mark"),
    ("LUK", "Luke"),
    ("JHN", "John"),
    ("ACT", "Acts"),
    ("ROM", "Rom"),
    ("1CO", "1Cor"),
    ("2CO", "2Cor"),
    ("GAL", "Gal"),
    ("EPH", "Eph"),
    ("PHP", "Phil"),
    ("COL", "Col"),
    ("1TH", "1Thess"),
    ("2TH", "2Thess"),
    ("1TI", "1Tim"),
    ("2TI", "2Tim"),
    ("TIT", "Titus"),
    ("PHM", "Phlm"),
    ("HEB", "Heb"),
    ("JAS", "Jas"),
    ("1PE", "1Pet"),
    ("2PE", "2Pet"),
    ("1JN", "1John"),
    ("2JN", "2John"),
    ("3JN", "3John"),
    ("JUD", "Jude"),
    ("REV", "Rev"),
];

/// Bøker som bare finnes i utgaver med deuterokanon. De står ikke i [`BOOK_IDS`],
/// så [`canonical`] gir `None` for dem og [`is_protestant`] gir `false`.
pub const DEUTEROCANONICAL: &[&str] = &[
    "TOB", // Tobit
    "JDT", // Judit
    "ESG", // Greske Ester
    "WIS", // Salomos visdom
    "SIR", // Sirak
    "BAR", // Baruk
    "LJE", // Jeremias brev
    "1MA", // 1. Makkabeerbok
    "2MA", // 2. Makkabeerbok
];

fn eq_ignore_case(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.eq_ignore_ascii_case(b)
}

/// Gjør en bok-id om til OSIS-formen, uansett om den kom som USFM eller OSIS.
/// Ukjente id-er - blant annet de deuterokanoniske - gir `None`.
pub fn canonical(id: &str) -> Option<&'static str> {
    BOOK_IDS
        .iter()
        .find(|(usfm, osis)| eq_ignore_case(id, usfm) || eq_ignore_case(id, osis))
        .map(|(_, osis)| *osis)
}

/// Om boka hører til de 66 i den protestantiske kanon.
pub fn is_protestant(id: &str) -> bool {
    canonical(id).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn begge_kodeverk_gir_samme_bok() {
        assert_eq!(canonical("JHN"), Some("John"));
        assert_eq!(canonical("john"), Some("John"));
        assert_eq!(canonical("John"), Some("John"));
        assert_eq!(canonical("PSA"), Some("Ps"));
        assert_eq!(canonical("ps"), Some("Ps"));
        assert_eq!(canonical("SNG"), Some("Song"));
        assert_eq!(canonical("1SA"), Some("1Sam"));
        assert_eq!(canonical("finnesikke"), None);
    }

    #[test]
    fn tabellen_er_fullstendig_og_entydig() {
        assert_eq!(BOOK_IDS.len(), 66);
        // Hver id - i begge former - må peke på nøyaktig én bok.
        for (usfm, osis) in BOOK_IDS {
            assert_eq!(canonical(usfm), Some(*osis), "{usfm}");
            assert_eq!(canonical(osis), Some(*osis), "{osis}");
        }
        // Ingen USFM-id får være lik en ANNEN boks OSIS-id, ellers blir oppslaget
        // tvetydig avhengig av hvilken oversettelse man er i.
        for (usfm, osis) in BOOK_IDS {
            for (other_usfm, other_osis) in BOOK_IDS {
                if usfm != other_usfm {
                    assert!(!eq_ignore_case(usfm, other_osis), "{usfm} kolliderer med {other_osis}");
                }
            }
            let _ = osis;
        }
    }

    #[test]
    fn deuterokanon_star_utenfor() {
        assert_eq!(DEUTEROCANONICAL.len(), 9);
        for id in DEUTEROCANONICAL {
            assert!(!is_protestant(id), "{id} skal ikke regnes som protestantisk");
        }
        for (usfm, _) in BOOK_IDS {
            assert!(is_protestant(usfm), "{usfm}");
        }
        // Esters bok finnes i begge former; den hebraiske (EST) skal beholdes.
        assert!(is_protestant("EST"));
        assert!(!is_protestant("ESG"));
    }
}
