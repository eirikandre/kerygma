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
