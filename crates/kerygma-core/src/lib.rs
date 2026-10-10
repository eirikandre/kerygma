//! Logikken i Kerygma, uten Tauri. Alt her testes med `cargo test`.

pub mod bible;
pub mod book_names;
pub mod extract;
pub mod markdown;
pub mod presentation;
pub mod reference;
pub mod settings;
pub mod vault;
pub mod verse_query;
pub mod word;

/// Filnavnet uten mappe og uten `.md`: "Prekener/Påske.md" gir "Påske".
pub fn path_name(path: &str) -> String {
    let file = path.rsplit('/').next().unwrap_or(path);
    file.strip_suffix(".md").unwrap_or(file).to_string()
}
