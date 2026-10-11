//! Lager tabellen over boknavn (`BOOK_IDS` i `book_names.rs`) fra oversettelsene i
//! `bibler/*.bin`: navnet og forkortelsen til hver bok i hver oversettelse, med bok-id-en
//! `bibel` slår opp på. Alle oversettelsene som ligger der når appen bygges, blir med, så
//! referanser i den formen `bibel` skriver dem («Jesajas bok 53:5», «Isaiah 53:5»), blir
//! gjenkjent for alle oversettelsene appen har.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let dir = Path::new(&manifest).join("../../bibler");
    println!("cargo:rerun-if-changed={}", dir.display());

    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map(|entries| entries.filter_map(Result::ok).map(|entry| entry.path()).collect())
        .unwrap_or_default();
    paths.retain(|path| path.extension().is_some_and(|ext| ext == "bin"));
    paths.sort();
    if paths.is_empty() {
        println!("cargo:warning=fant ingen oversettelser i {}; bare de vanlige norske boknavnene blir gjenkjent", dir.display());
    }

    let mut out = String::from(
        "// Generert av build.rs fra bibler/*.bin: navn og forkortelse for hver bok i hver oversettelse.\n\
         pub const BOOK_IDS: &[(&str, &str)] = &[\n",
    );
    for path in &paths {
        println!("cargo:rerun-if-changed={}", path.display());
        let data = std::fs::read(path).unwrap_or_else(|e| panic!("kunne ikke lese {}: {e}", path.display()));
        let bible = bibel_format::Bible::parse(&data).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        writeln!(out, "    // {} ({})", bible.title(), bible.id()).unwrap();
        for book in bible.books() {
            writeln!(out, "    ({:?}, {:?}),", book.name, book.osis_id).unwrap();
            writeln!(out, "    ({:?}, {:?}),", book.abbrev, book.osis_id).unwrap();
        }
    }
    out.push_str("];\n");

    let dest = Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("book_names.rs");
    std::fs::write(dest, out).expect("skrive book_names.rs");
}
