//! «Lag presentasjon»: hvor presentasjonen havner, og innholdet i den.
//!
//! En presentasjon er en vanlig markdown-fil ved siden av manuset, merket med
//! [`marker`]. Filer uten markør (laget før markøren fantes) kjennes igjen på
//! navnet: `<manus><suffix>.md`. Stiene er relative til vault-roten og bruker `/`.

use std::sync::LazyLock;

use regex::Regex;

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

/// Stiene for en fil. Er fila selv en presentasjon, er manuset fila ved siden av.
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

/// Linja som merker en presentasjon laget av appen: `<!-- kerygma-presentasjon: Påske.md -->`.
/// En HTML-kommentar vises ikke i lesevisningen, presentasjonen eller Obsidian.
/// Navnet er manuset presentasjonen er laget fra, i samme mappe.
pub fn marker(source: &str) -> String {
    format!("<!-- kerygma-presentasjon: {source} -->")
}

/// Mønsteret for markørlinja, uten flagg. Brukes også i grensesnittet (med `m`), via `bindings.ts`.
pub const MARKER_PATTERN: &str = r"^<!-- kerygma-presentasjon: (.+?) -->[ \t]*\r?$";

static MARKER: LazyLock<Regex> = LazyLock::new(|| Regex::new(&format!("(?m){MARKER_PATTERN}")).unwrap());

/// Manuset (filnavnet, i samme mappe) en presentasjon er laget fra, eller `None`
/// hvis teksten ikke er en presentasjon laget av appen.
pub fn presentation_source(content: &str) -> Option<String> {
    MARKER.captures(content).map(|caps| caps[1].to_string())
}

/// Setter markøren øverst i presentasjonen, etter frontmatter hvis den finnes.
fn with_marker(slides: &str, source: &str) -> String {
    let marker = marker(source);
    // `extract_slide_content` gir frontmatter som "---\n…\n---\n\n" først.
    if let Some(rest) = slides.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---\n") {
            let split = 4 + end + 5;
            let (frontmatter, body) = slides.split_at(split);
            let body = body.strip_prefix('\n').unwrap_or(body);
            return format!("{frontmatter}\n{marker}\n\n{body}");
        }
    }
    format!("{marker}\n\n{slides}")
}

/// Stiene når fila kan være en presentasjon: har den markøren, er manuset fila
/// markøren peker på (uansett hva presentasjonen heter). Ellers som `presentation_paths`.
pub fn presentation_paths_for(note: &str, content: &str, suffix: &str) -> PresentationPaths {
    match presentation_source(content) {
        Some(source) => {
            let folder = note.rfind('/').map_or("", |i| &note[..=i]);
            PresentationPaths {
                manuscript: format!("{folder}{source}"),
                presentation: note.to_string(),
                from_presentation: true,
            }
        }
        None => presentation_paths(note, suffix),
    }
}

/// Innholdet i presentasjonen, med markøren som sier at den er laget fra `source`
/// (filnavnet til manuset), og meldinger om oversettelsen ikke kunne legges til.
pub fn presentation_content(
    manuscript: &str,
    source: &str,
    settings: &Settings,
    bibel: Option<&Bibel>,
) -> (String, Vec<String>) {
    let (options, warnings) = slide_options(manuscript, settings, bibel);
    (with_marker(&extract_slide_content(manuscript, &options), source), warnings)
}

/// Innstillingene til slidene: maks tegn, og oversettelsene som legges ved siden
/// av vers som står alene. Meldingene sier hvorfor oversettelsen ikke kunne legges til.
pub fn slide_options(manuscript: &str, settings: &Settings, bibel: Option<&Bibel>) -> (SlideOptions, Vec<String>) {
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
    (options, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presentasjonen_merkes_med_manuset_den_er_laget_fra() {
        let settings = Settings { add_translation: false, ..Default::default() };
        let (content, _) = presentation_content("# Påske\n\nTekst.\n", "Påske.md", &settings, None);
        assert_eq!(content, "<!-- kerygma-presentasjon: Påske.md -->\n\n# Påske\n");
        assert_eq!(presentation_source(&content).as_deref(), Some("Påske.md"));
        assert_eq!(presentation_source("# Påske\n<!-- annen kommentar -->\n"), None);
    }

    #[test]
    fn markoren_kommer_etter_frontmatter() {
        let settings = Settings { add_translation: false, ..Default::default() };
        let (content, _) = presentation_content("---\ntags: a\n---\n# Påske\n", "Påske.md", &settings, None);
        assert_eq!(content, "---\ntags: a\n---\n\n<!-- kerygma-presentasjon: Påske.md -->\n\n# Påske\n");
        assert_eq!(presentation_source(&content).as_deref(), Some("Påske.md"));
    }

    #[test]
    fn en_merket_presentasjon_lages_fra_manuset_i_markoren() {
        let content = "<!-- kerygma-presentasjon: Påske.md -->\r\n\r\n# Påske\r\n";
        assert_eq!(
            presentation_paths_for("Prekener/Lysbilder.md", content, " - presentasjon"),
            PresentationPaths {
                manuscript: "Prekener/Påske.md".into(),
                presentation: "Prekener/Lysbilder.md".into(),
                from_presentation: true,
            }
        );
        // Uten markør avgjør navnet, som før.
        assert_eq!(
            presentation_paths_for("Prekener/Påske.md", "# Påske", " - presentasjon"),
            presentation_paths("Prekener/Påske.md", " - presentasjon")
        );
    }

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
