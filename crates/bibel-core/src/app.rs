//! Argumentene til `bibel`: tolker dem og lager utdataene. Kerygma slår opp,
//! søker og formaterer vers gjennom [`run`] (se `kerygma-core/src/bible.rs`).
//! Ingenting skrives direkte - utdataene samles i [`Output`].

use std::fmt::Write as _;

use bibel_format::Bible;

use crate::render::{self, RenderOpts};
use crate::translations::Registry;
use crate::{reference, search};

/// Det kjørbaren ville skrevet til stdout, og koden den ville avsluttet med.
pub struct Output {
    pub stdout: String,
    pub code: u8,
}

/// Treffene i et søk har samme format som et oppslag, men står på hver sin linje.
const SEARCH_SEPARATOR: &str = "\n";
pub const SEARCH_LIMIT: usize = 5;

pub const HELP: &str = "\
bibel - slår opp bibelvers

BRUK
    bibel [FLAGG] [OVERSETTELSE:]<referanse>

REFERANSER
    1sam2:1-3      vers 1 til 3 i kapittel 2
    1sam1:3        ett vers
    1sam2          hele kapitlet
    1sam1:20-2:5   fra 1,20 til og med 2,5
    kjv:john3:16   slå opp i en annen oversettelse enn standard
    nb88+kjv:john3:16
                   flere oversettelser etter hverandre
    sal23:1        forkortelse, osisId (ps23:1), alias (salmene23:1)
                   eller fullt navn («Salmenes bok 23:1») går like bra;
                   komma virker som skille: 1sam2,1-3

SØK
    bibel --søk 'tro håp'        alle ordene, i samme vers
    bibel --søk '\"i begynnelsen\"'
                                 frase
    bibel --søk 'tro -lov'       utelat vers som har «lov»
    bibel --søk 'kjv:love'       søk i en annen oversettelse
    Delsøk er standard, så «kjærlighet» finner også «kjærligheten».
    Treffene skrives som et oppslag, ett per linje.

FLAGG
    -t, --oversettelse <id>
                     velg oversettelse; «a+b» gir flere, tom verdi betyr standard
    --mellom <tegn>  mellom oversettelsene, standard: linjeskift
    --format <mal>   standard: \"{tekst} ({ref})\"
                     plassholdere: {tekst} {ref} {bok} {forkortelse}
                                   {kapittel} {vers} {tittel} {oversettelse}
    --skille <tegn>  mellom versene, standard: \" \" (bruk \\n for linjeskift)
    -s, --søk <ord>  fritekstsøk i stedet for et oppslag
    --omfang <hvor>  vers (standard) eller kapittel - hvor leddene i et
                     søk må opptre sammen
    --antall <n>     maks antall treff, 0 = alle (standard: 5)
    --hele-ord       bare hele ord, ikke deler av ord
    --uten-numre     ikke sett versnummer foran hvert vers
    --stjerner       behold *-markørene som peker til studienoter
    --alltid-null    avslutt med kode 0 også når oppslaget eller søket
                     feilet (meldingen står i utdataene)
    --oversettelser  list oversettelsene som er bygget inn
    --bøker          list bøkene i den valgte oversettelsen
    -h, --help       denne teksten";

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

macro_rules! outln {
    ($out:ident, $($arg:tt)*) => {{
        let _ = writeln!($out, $($arg)*);
    }};
}

/// Kjører `bibel` med argumentene (uten programnavnet).
pub fn run(registry: &Registry, args: &[String]) -> Output {
    let mut stdout = String::new();
    let code = run_into(registry, args, &mut stdout);
    // Med --alltid-null er feilen bare en melding i utdataene.
    let code = if args.iter().any(|a| a == "--alltid-null") { 0 } else { code };
    Output { stdout, code }
}

/// Henter verdien etter et flagg, eller forklarer at den mangler.
fn value(args: &[String], i: &mut usize, flag: &str, out: &mut String) -> Option<String> {
    *i += 1;
    match args.get(*i) {
        Some(v) => Some(v.clone()),
        None => {
            let _ = writeln!(out, "[{flag} mangler en verdi]");
            None
        }
    }
}

fn run_into(registry: &Registry, args: &[String], out: &mut String) -> u8 {
    let mut opts = RenderOpts::default();
    let mut input: Option<String> = None;
    let mut chosen: Option<String> = None;
    let mut query: Option<String> = None;
    let mut sopts = search::Options { limit: SEARCH_LIMIT, ..search::Options::default() };
    // Linjeskift mellom søketreffene skal bare gjelde når brukeren ikke har sagt noe selv.
    let mut separator_given = false;
    // Linjeskift mellom oversettelsene: med to tekster etter hverandre blir det
    // uleselig på én linje. Verset skilles fortsatt med --skille.
    let mut between = "\n".to_string();
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        match arg {
            "-h" | "--help" => {
                outln!(out, "{HELP}");
                return 0;
            }
            "-t" | "--oversettelse" | "--translation" => match value(args, &mut i, "--oversettelse", out) {
                // Espanso sender tom streng når triggeren ikke hadde prefiks.
                Some(v) if v.trim().is_empty() => {}
                Some(v) => chosen = Some(v),
                None => return 2,
            },
            "--format" => match value(args, &mut i, "--format", out) {
                Some(v) => opts.format = unescape(&v),
                None => return 2,
            },
            "--skille" | "--separator" => match value(args, &mut i, "--skille", out) {
                Some(v) => {
                    opts.separator = unescape(&v);
                    separator_given = true;
                }
                None => return 2,
            },
            "-s" | "--søk" | "--sok" | "--search" => match value(args, &mut i, "--søk", out) {
                Some(v) => query = Some(v),
                None => return 2,
            },
            "--omfang" | "--scope" => match value(args, &mut i, "--omfang", out) {
                Some(v) => match v.trim() {
                    "vers" | "verse" => sopts.scope = search::Scope::Verse,
                    "kapittel" | "kap" | "chapter" => sopts.scope = search::Scope::Chapter,
                    other => {
                        outln!(out, "[ukjent omfang «{other}», bruk vers eller kapittel]");
                        return 2;
                    }
                },
                None => return 2,
            },
            "--antall" | "--limit" => match value(args, &mut i, "--antall", out) {
                Some(v) => match v.trim().parse::<usize>() {
                    Ok(n) => sopts.limit = n,
                    Err(_) => {
                        outln!(out, "[--antall må være et tall, ikke «{v}»]");
                        return 2;
                    }
                },
                None => return 2,
            },
            "--hele-ord" | "--heleord" | "--whole-words" => sopts.whole_words = true,
            "--mellom" => match value(args, &mut i, "--mellom", out) {
                Some(v) => between = unescape(&v),
                None => return 2,
            },
            "--uten-numre" | "--ingen-numre" => opts.numbers = false,
            // Håndteres i main().
            "--alltid-null" => {}
            "--stjerner" => opts.keep_stars = true,
            "--oversettelser" | "--translations" => {
                for bible in registry.iter() {
                    let mark = if bible.id() == registry.default_bible().id() { "*" } else { " " };
                    outln!(out, 
                        "{mark} {:<6} {:<42} {:>2} bøker, {:>4} kapitler, {} vers",
                        bible.id(),
                        bible.title(),
                        bible.book_count(),
                        bible.chapter_count(),
                        bible.verse_count()
                    );
                }
                if !registry.aliases().is_empty() {
                    outln!(out, "\nSnarveier:");
                    for (name, targets) in registry.aliases() {
                        outln!(out, "  {name:<6} {}", targets.join(" + "));
                    }
                }
                outln!(out, "\n* = standard. Velg en annen med «kjv:john3:16», flere med «nb88+kjv:john3:16».");
                return 0;
            }
            "--bøker" | "--boker" | "--books" => {
                // Behandles etter at oversettelsen er valgt.
                input = Some("\u{0}bøker".to_string());
            }
            other if other.starts_with("--") => {
                outln!(out, "[ukjent flagg «{other}», se --help]");
                return 2;
            }
            other if query.is_some() => {
                // «bibel --søk tro håp» uten anførselstegn: resten hører til søket.
                if let Some(q) = query.as_mut() {
                    q.push(' ');
                    q.push_str(other);
                }
            }
            other => {
                // Flere løse argumenter settes sammen, så «bibel 1 Sam 2:1-3» virker.
                match &mut input {
                    Some(existing) if existing != "\u{0}bøker" => {
                        existing.push(' ');
                        existing.push_str(other);
                    }
                    Some(_) => {}
                    None => input = Some(other.to_string()),
                }
            }
        }
        i += 1;
    }

    let input = input
        .filter(|s| !s.trim().is_empty());
    let chosen = chosen
        .filter(|s| !s.trim().is_empty());

    // Oversettelsen kan oppgis med flagg eller som prefiks i selve referansen.
    let mut bibles = vec![registry.default_bible()];
    if let Some(ref spec) = chosen {
        match registry.select(spec) {
            Ok(found) => bibles = found,
            Err(e) => {
                outln!(out, "[{e} - har {}]", registry.ids().join(", "));
                return 2;
            }
        }
    }

    // Søk har sin egen inndata, sine egne standardvalg og sitt eget utdata.
    if let Some(query) = query {
        let text = if chosen.is_some() {
            query.as_str()
        } else {
            let (prefixed, rest) = registry.split_prefix(&query);
            if let Some(found) = prefixed {
                bibles = found;
            }
            rest
        };
        if !separator_given {
            opts.separator = SEARCH_SEPARATOR.to_string();
        }
        return run_search(&bibles, text, &opts, &sopts, &between, out);
    }

    let Some(input) = input else {
        outln!(out, "{HELP}");
        return 2;
    };

    let reference_text = if input == "\u{0}bøker" {
        for book in bibles[0].books() {
            outln!(out, 
                "{:<3} {:<7} {:<8} {:>3} kap.  {}",
                book.book_no, book.abbrev, book.osis_id, book.chapter_count, book.name
            );
        }
        return 0;
    } else if chosen.is_some() {
        input.as_str()
    } else {
        let (prefixed, rest) = registry.split_prefix(&input);
        if let Some(found) = prefixed {
            bibles = found;
        }
        rest
    };

    // Hver oversettelse slås opp for seg, så ett treff ikke faller bort fordi en
    // annen mangler boka - Sirak finnes i Bibel 2024, men ikke i KJV.
    let mut parts: Vec<String> = Vec::with_capacity(bibles.len());
    let mut failures = 0;
    for bible in &bibles {
        match reference::resolve(bible, reference_text) {
            Ok(r) => parts.push(render::render(bible, &r, &opts, bible.title())),
            Err(e) => {
                failures += 1;
                // Til stdout, ikke stderr: da ser brukeren feilen i stedet for at
                // triggeren bare forsvinner uten spor.
                parts.push(format!("[{e} ({})]", bible.title()));
            }
        }
    }
    outln!(out, "{}", parts.join(&between));
    u8::from(failures > 0)
}

/// Søker i hver oversettelse for seg, på samme måte som et oppslag: at ett ord
/// ikke finnes i King James skal ikke skjule treffene i den norske teksten.
fn run_search(
    bibles: &[&Bible<'static>],
    text: &str,
    opts: &RenderOpts,
    sopts: &search::Options,
    between: &str,
    out: &mut String,
) -> u8 {
    let query = match search::parse(text) {
        Ok(q) => q,
        Err(e) => {
            outln!(out, "[{e}]");
            return 2;
        }
    };

    let mut parts: Vec<String> = Vec::with_capacity(bibles.len());
    let mut empty = 0;
    for bible in bibles {
        let found = search::search(bible, &query, sopts);
        if found.hits.is_empty() {
            empty += 1;
            parts.push(format!("[ingen treff på «{text}» ({})]", bible.title()));
            continue;
        }
        let mut lines: Vec<String> = found
            .hits
            .iter()
            .map(|hit| render::render_hit(hit, opts, bible.title()))
            .collect();
        // Uten denne linja ser et begrenset søk ut som et uttømmende et.
        if found.hidden() > 0 {
            lines.push(format!("[… og {} treff til]", found.hidden()));
        }
        parts.push(lines.join(&opts.separator));
    }
    outln!(out, "{}", parts.join(between));
    u8::from(empty > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> Registry {
        crate::translations::test_registry().expect("assets/*.bin")
    }

    #[test]
    fn alle_innebygde_datasett_er_gyldige() {
        let r = registry();
        assert!(!r.ids().is_empty(), "ingen oversettelser bygget inn");
        for bible in r.iter() {
            assert!(!bible.id().is_empty(), "oversettelse uten id");
            assert!(!bible.title().is_empty(), "{} mangler tittel", bible.id());
            assert!(bible.book_count() >= 66, "{} har bare {} bøker", bible.id(), bible.book_count());
        }
        // Id-ene må være unike, ellers kan ikke en referanse peke entydig.
        let mut ids = r.ids();
        ids.sort_unstable();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n, "to oversettelser deler id");
    }

    #[test]
    fn hver_oversettelse_slar_opp_alle_sine_boker() {
        for bible in registry().iter() {
            for book in bible.books() {
                let r = reference::resolve(bible, &format!("{}1:1", book.abbrev))
                    .unwrap_or_else(|e| panic!("{} {}: {e}", bible.id(), book.abbrev));
                assert_eq!(r.book.osis_id, book.osis_id);

                // Siste kapittel og siste vers i boka skal også være nåbart.
                let chapter = bible.chapter(&book, book.chapter_count).unwrap();
                let last = bible
                    .verse_at(&chapter, chapter.verse_count - 1)
                    .unwrap_or_else(|| panic!("{} {} siste vers", bible.id(), book.abbrev));
                let s = format!("{}{}:{}", book.abbrev, book.chapter_count, last.end);
                assert!(
                    reference::resolve(bible, &s).is_ok(),
                    "{} {s}",
                    bible.id()
                );
            }
        }
    }

    #[test]
    fn prefiks_velger_oversettelse() {
        let r = registry();
        for id in r.ids() {
            let input = format!("{id}:john3:16");
            let (bibles, rest) = r.split_prefix(&input);
            assert_eq!(bibles.map(|b| b.iter().map(|x| x.id()).collect::<Vec<_>>()), Some(vec![id]));
            assert_eq!(rest, "john3:16");
        }
        // En vanlig referanse må ikke tolkes som et prefiks.
        let (bibles, rest) = r.split_prefix("1sam2:1-3");
        assert!(bibles.is_none());
        assert_eq!(rest, "1sam2:1-3");
    }

    #[test]
    fn kombinasjoner_velger_flere() {
        let r = registry();
        let ids = r.ids();
        if ids.len() < 2 {
            return;
        }
        let spec = format!("{}+{}", ids[0], ids[1]);
        let picked = r.select(&spec).expect("kombinasjon");
        assert_eq!(
            picked.iter().map(|b| b.id()).collect::<Vec<_>>(),
            vec![ids[0], ids[1]]
        );
        // Rekkefølgen skal være den som er skrevet, ikke registerets.
        let omvendt = r.select(&format!("{}+{}", ids[1], ids[0])).unwrap();
        assert_eq!(
            omvendt.iter().map(|b| b.id()).collect::<Vec<_>>(),
            vec![ids[1], ids[0]]
        );
        // Som prefiks i en referanse.
        let input = format!("{spec}:john3:16");
        let (bibles, rest) = r.split_prefix(&input);
        assert_eq!(bibles.map(|b| b.len()), Some(2));
        assert_eq!(rest, "john3:16");
        assert!(r.select("nb88+finnesikke").is_err());
    }

    #[test]
    fn to_oversettelser_skilles_med_linjeskift() {
        let r = registry();
        let ids = r.ids();
        if ids.len() < 2 {
            return;
        }
        let bibles = r.select(&format!("{}+{}", ids[0], ids[1])).unwrap();
        let opts = RenderOpts::default();
        let parts: Vec<String> = bibles
            .iter()
            .map(|b| {
                // Hver oversettelse navngir bøkene sine ulikt, så bruk dens egen
                // første bok i stedet for en referanse som antas å finnes overalt.
                let first = b.book(0).expect("minst én bok");
                let spec = format!("{}1:1", first.abbrev);
                let res = reference::resolve(b, &spec)
                    .unwrap_or_else(|e| panic!("{} {spec}: {e}", b.id()));
                render::render(b, &res, &opts, b.title())
            })
            .collect();

        let joined = parts.join("\n");
        assert_eq!(joined.matches('\n').count(), 1, "nøyaktig ett linjeskift");
        assert!(!joined.contains("\n\n"), "ingen tomme linjer");
        for line in joined.lines() {
            assert!(!line.trim().is_empty(), "tom linje i utdata");
        }
    }

    #[test]
    fn samme_boknavn_virker_i_alle_oversettelsene() {
        // Datasettene bruker ulike kodeverk: NB88 og KJV har OSIS-id-er, bibel.no
        // har USFM. Begge formene skal treffe samme bok overalt.
        let r = registry();
        for (usfm, osis) in bibel_format::books::BOOK_IDS {
            for bible in r.iter() {
                for form in [usfm, osis] {
                    let spec = format!("{form}1:1");
                    let resolved = reference::resolve(bible, &spec)
                        .unwrap_or_else(|e| panic!("{} {spec}: {e}", bible.id()));
                    assert_eq!(
                        bibel_format::books::canonical(resolved.book.osis_id),
                        Some(*osis),
                        "{} {spec} havnet i {}",
                        bible.id(),
                        resolved.book.osis_id
                    );
                }
            }
        }
    }

    #[test]
    fn forkortelser_og_navn_virker_pa_tvers_av_oversettelsene() {
        // «ne:1kor13:1» skal treffe 1. Korinterbrev også i KJV, og «e:1cor13:1»
        // også i NB88: hver oversettelses forkortelser og boknavn må virke i alle.
        let r = registry();
        let mut feil: Vec<String> = Vec::new();
        for source in r.iter() {
            for book in source.books() {
                let wanted = bibel_format::books::canonical(book.osis_id);
                for form in [book.abbrev, book.name] {
                    let spec = format!("{form} 1:1");
                    for target in r.iter() {
                        match reference::resolve(target, &spec) {
                            Ok(res) if bibel_format::books::canonical(res.book.osis_id) == wanted => {}
                            Ok(res) => feil.push(format!(
                                "{} → {} {spec}: havnet i {}",
                                source.id(),
                                target.id(),
                                res.book.osis_id
                            )),
                            Err(e) => feil.push(format!("{} → {} {spec}: {e}", source.id(), target.id())),
                        }
                    }
                }
            }
        }
        assert!(feil.is_empty(), "{} feil:\n{}", feil.len(), feil.join("\n"));
    }

    #[test]
    fn ingen_deuterokanoniske_boker_er_med() {
        let r = registry();
        for bible in r.iter() {
            assert_eq!(bible.book_count(), 66, "{} har {} bøker", bible.id(), bible.book_count());
            for book in bible.books() {
                assert!(
                    bibel_format::books::is_protestant(book.osis_id),
                    "{} inneholder {}",
                    bible.id(),
                    book.osis_id
                );
            }
        }
        // Sirak fantes i Bibel 2024 før filtreringen og skal nå være borte.
        let Some(nb24) = r.by_id("nb24") else { return };
        assert!(reference::resolve(nb24, "sir1:1").is_err());
    }

    #[test]
    fn alle_kombinasjonsnavn_lar_seg_sla_opp() {
        let r = registry();
        for (name, targets) in r.aliases() {
            let picked = r
                .select(name)
                .unwrap_or_else(|e| panic!("kombinasjonen «{name}»: {e}"));
            assert_eq!(picked.len(), targets.len(), "{name}");
            assert_eq!(
                picked.iter().map(|b| b.id()).collect::<Vec<_>>(),
                targets.to_vec(),
                "{name}"
            );
        }
    }

    #[test]
    fn alle_aliaser_peker_pa_en_bok() {
        // Aliasene er norske og kontrolleres mot den norske standardoversettelsen.
        let r = registry();
        let Some(bible) = r.by_id("nb88") else { return };
        for (alias, osis) in reference::ALIASES {
            assert!(
                bible.books().any(|b| b.osis_id == *osis),
                "aliaset «{alias}» peker på ukjent osisId «{osis}»"
            );
            let resolved = reference::resolve(bible, &format!("{alias}1:1"))
                .unwrap_or_else(|e| panic!("{alias}: {e}"));
            assert_eq!(resolved.book.osis_id, *osis, "{alias}");
        }
    }

    #[test]
    fn sammenslatte_vers_i_ekte_data() {
        // Efeserne 1,15-16 står som én enhet i Bibel 2024. Begge numrene skal treffe.
        let r = registry();
        let Some(bible) = r.by_id("nb24") else { return };
        let fifteen = reference::resolve(bible, "eph1:15").expect("eph1:15");
        let sixteen = reference::resolve(bible, "eph1:16").expect("eph1:16");
        let a = reference::verses(bible, &fifteen);
        let b = reference::verses(bible, &sixteen);
        assert_eq!(a.len(), 1);
        assert_eq!(b.len(), 1);
        assert_eq!(a[0].1.text, b[0].1.text, "15 og 16 må gi samme vers");
        assert_eq!(a[0].1.label(), "15-16");
    }

    #[test]
    fn unescape_handterer_linjeskift() {
        assert_eq!(unescape(r"a\nb"), "a\nb");
        assert_eq!(unescape(r"a\\b"), r"a\b");
        assert_eq!(unescape(r"a\qb"), r"a\qb");
    }
}
