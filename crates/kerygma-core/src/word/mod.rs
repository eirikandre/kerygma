//! Import av gamle Word-manus (.docx) til markdown, med bibelsitatene som
//! `> tekst (referanse)`. Skrevet etter `word-to-markdown/` i obsidian-slide-creator.

pub mod align;
pub mod bible_text;
pub mod convert;
pub mod docx;

use crate::bible::Bibel;
use bible_text::BibelText;
pub use convert::{convert, ConvertOptions, ConvertResult, FoundQuote};

/// Oversettelsene sitatene i gamle manus sammenlignes med. Den første gir navnet i referansen.
pub const IMPORT_TRANSLATIONS: &[&str] = &["nb88", "nb24"];

/// Gjør en .docx-fil om til markdown.
pub fn import_docx(data: &[u8], bibel: &Bibel, options: &ConvertOptions) -> Result<ConvertResult, String> {
    let paragraphs = docx::read_docx(data)?;
    let bible = BibelText::new(bibel, IMPORT_TRANSLATIONS)?;
    Ok(convert(&paragraphs, &bible, options))
}

/// Navnet på notatet en Word-fil blir til: "Påske  2019.docx" gir "Påske 2019".
pub fn note_name(file_name: &str) -> String {
    let stem = file_name
        .strip_suffix(".docx")
        .or_else(|| file_name.strip_suffix(".DOCX"))
        .unwrap_or(file_name);
    stem.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Navnene på notatene når flere Word-filer importeres samtidig. To filer med samme
/// navn (fra forskjellige mapper) skal ikke bli samme notat: den andre blir "Navn 2".
/// Store og små bokstaver regnes som like, siden filsystemet ofte gjør det.
pub fn note_names<S: AsRef<str>>(file_names: &[S]) -> Vec<String> {
    let mut used = std::collections::HashSet::new();
    file_names
        .iter()
        .map(|file_name| {
            let base = note_name(file_name.as_ref());
            let name = (1..)
                .map(|n| if n == 1 { base.clone() } else { format!("{base} {n}") })
                .find(|name| !used.contains(&name.to_lowercase()))
                .expect("et ledig navn");
            used.insert(name.to_lowercase());
            name
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn like_filnavn_blir_forskjellige_notater() {
        let names = note_names(&["a/Preken.docx", "Preken.docx", "Annen.docx", "preken.DOCX", "Preken 2.docx"]
            .map(|path| path.rsplit('/').next().unwrap()));
        assert_eq!(names, ["Preken", "Preken 2", "Annen", "preken 3", "Preken 2 2"]);
    }
}
