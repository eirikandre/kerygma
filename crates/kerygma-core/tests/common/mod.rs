#![allow(dead_code)]

/// Fasiten med referanser fra bibel-prosjektet, som i `test/fixtures` i pluginen.
pub fn generated() -> Vec<String> {
    include_str!("../fixtures/bibel-referanser.txt").trim().split('\n').map(String::from).collect()
}

use std::path::PathBuf;

use kerygma_core::bible::{Bibel, BibelConfig};

/// `bibler/` i roten av repoet, med `*.bin` kopiert fra bible-repoet.
pub fn bibles_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../bibler")
}

/// `bibel` med innstillingene fra pluginen, eller `None` (med en melding) når
/// `bibler/` mangler nb88.bin eller kjv.bin.
pub fn bibel() -> Option<Bibel> {
    let dir = bibles_dir();
    if !["nb88.bin", "kjv.bin"].iter().all(|name| dir.join(name).exists()) {
        eprintln!("hopper over: fant ikke nb88.bin og kjv.bin i {}", dir.display());
        return None;
    }
    let mut bibel = Bibel::load_dir(&dir).expect("laste biblene");
    bibel
        .configure(&BibelConfig {
            default_translation: "nb88".to_string(),
            aliases: "n=nb88,e=kjv,ne=nb88+kjv".to_string(),
        })
        .expect("innstillinger");
    Some(bibel)
}
