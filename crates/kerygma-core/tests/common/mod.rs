#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::LazyLock;

use kerygma_core::bible::{Bibel, BibelConfig};
use regex::Regex;

/// Oversettelsene i `bibler/`. `nb30.bin` og `kjv.bin` er i git; andre som ligger der,
/// blir med i appen og i [`generated`].
pub fn bibles_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../bibler")
}

fn read(name: &str) -> (String, Vec<u8>) {
    let path = bibles_dir().join(name);
    let data = std::fs::read(&path).unwrap_or_else(|e| panic!("{} er i git: {e}", path.display()));
    (name.to_string(), data)
}

/// `bibel` med nb30 og KJV og standardinnstillingene til appen. Bare disse to lastes, så
/// testene gir samme svar uansett hvilke andre oversettelser som ligger i `bibler/`.
pub fn bibel() -> Bibel {
    let mut bibel = Bibel::from_files(vec![read("kjv.bin"), read("nb30.bin")]).expect("laste biblene");
    let notes = bibel
        .configure(&BibelConfig {
            default_translation: "nb30".to_string(),
            aliases: "n=nb30,e=kjv,ne=nb30+kjv".to_string(),
        })
        .expect("innstillinger");
    assert!(notes.is_empty(), "{notes:?}");
    bibel
}

static BOOK_LINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\S+)\s+[0-9]+ kap\.").unwrap());

/// Referansene `bibel` skriver, som «(Jesajas bok 53:5)» og «(Isaiah 53:5)», for hver bok i
/// hver oversettelse i `bibler/` og hver form (vers, versintervall, kapittel, over
/// kapittelskifte). Alle skal gjenkjennes, siden boknavnene lages fra de samme filene.
pub fn generated() -> Vec<String> {
    let bibel = Bibel::load_dir(&bibles_dir()).expect("laste biblene");
    let mut references: Vec<String> = Vec::new();
    for id in bibel.ids() {
        let books = bibel.run(&["-t", id, "--bøker"]).stdout;
        for book in BOOK_LINE.captures_iter(&books).map(|caps| caps[1].to_string()) {
            for locus in ["1:1", "1:1-2", "1", "1:2-2:1"] {
                let output = bibel.run(&["--format", "({ref})", &format!("{id}:{book}{locus}")]);
                if output.code == 0 {
                    references.push(output.stdout.trim().to_string());
                }
            }
        }
    }
    references.dedup();
    references
}
