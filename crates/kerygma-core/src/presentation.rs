//! «Lag presentasjon»: hvor presentasjonen havner, og innholdet i den.
//!
//! Skrevet etter `createSlideCopy` og `translations` i `src/main.ts` i
//! obsidian-slide-creator. Stiene er relative til vault-roten og bruker `/`.

use crate::bible::{look_up_translations, Bibel};
use crate::extract::{extract_slide_content, lone_verse_references, SlideOptions};
use crate::settings::Settings;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentationPaths {
    /// Manuset presentasjonen lages fra.
    pub manuscript: String,
    /// Presentasjonen: `<navn><suffix>.md` i samme mappe som manuset.
    pub presentation: String,
    /// Kjøres kommandoen fra en presentasjon, lages den på nytt fra manuset ved siden av.
    pub from_presentation: bool,
}

/// Stiene for en note. Er noten selv en presentasjon, er manuset fila ved siden av.
pub fn presentation_paths(note: &str, suffix: &str) -> PresentationPaths {
    let trimmed_suffix = suffix.trim();
    let (folder, file) = match note.rfind('/') {
        Some(i) => (&note[..=i], &note[i + 1..]),
        None => ("", note),
    };
    let basename = file.strip_suffix(".md").unwrap_or(file);
    let from_presentation = !trimmed_suffix.is_empty() && basename.ends_with(trimmed_suffix);
    let base = if from_presentation {
        basename[..basename.len() - trimmed_suffix.len()].trim_end()
    } else {
        basename
    };
    PresentationPaths {
        manuscript: if from_presentation { format!("{folder}{base}.md") } else { note.to_string() },
        presentation: format!("{folder}{base}{suffix}.md"),
        from_presentation,
    }
}

/// Innholdet i presentasjonen, og meldinger om oversettelsen ikke kunne legges til.
pub fn presentation_content(manuscript: &str, settings: &Settings, bibel: Option<&Bibel>) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let mut options = SlideOptions { max_chars: Some(settings.max_chars), ..Default::default() };
    if settings.add_translation {
        let translation = &settings.translation;
        match bibel {
            None => warnings.push(format!("Kunne ikke legge til {translation}: biblene er ikke lastet.")),
            Some(bibel) => match bibel.check_translation(translation) {
                Err(message) => warnings.push(format!("Kunne ikke legge til {translation}: {message}")),
                Ok(()) => {
                    options.translations =
                        look_up_translations(bibel, translation, &lone_verse_references(manuscript));
                }
            },
        }
    }
    (extract_slide_content(manuscript, &options), warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presentasjonen_legges_ved_siden_av_manuset() {
        let suffix = " - presentasjon";
        assert_eq!(
            presentation_paths("Prekener/Påske.md", suffix),
            PresentationPaths {
                manuscript: "Prekener/Påske.md".into(),
                presentation: "Prekener/Påske - presentasjon.md".into(),
                from_presentation: false,
            }
        );
        assert_eq!(
            presentation_paths("Påske - presentasjon.md", suffix),
            PresentationPaths {
                manuscript: "Påske.md".into(),
                presentation: "Påske - presentasjon.md".into(),
                from_presentation: true,
            }
        );
    }
}
