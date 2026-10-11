//! Innstillingene for en vault, med standardverdier.
//! Lagres per vault i `<vault>/.kerygma/settings.json`.

use serde::{Deserialize, Serialize};

use crate::extract::DEFAULT_MAX_CHARS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Legges til navnet på manuset for å gi navnet på presentasjonen: «Påske - presentasjon.md».
    pub suffix: String,
    pub max_chars: usize,
    pub add_translation: bool,
    pub translation: String,
    /// Oversettelsen et oppslag uten prefiks bruker, som `BIBEL_DEFAULT`.
    pub default_translation: String,
    /// Snarveier som "ne:joh3:16", skrevet som `BIBEL_ALIAS`.
    pub aliases: String,
    /// Hvor importerte Word-manus legges. Tom: mappa til manuset som er åpent.
    pub import_folder: String,
    /// Temaet presentasjonen vises med (id fra `ui/src/slideThemes.ts`).
    pub slide_theme: String,
    /// Skriften i overskriftene på slidene. Tom: temaets skrift.
    pub heading_font: String,
    /// Skriften i sitater og bibelvers på slidene. Tom: temaets skrift.
    pub quote_font: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            suffix: " - presentasjon".to_string(),
            max_chars: DEFAULT_MAX_CHARS,
            add_translation: true,
            translation: "kjv".to_string(),
            default_translation: "nb88".to_string(),
            aliases: "n=nb88,e=kjv,ne=nb88+kjv".to_string(),
            import_folder: String::new(),
            slide_theme: "kveld".to_string(),
            heading_font: String::new(),
            quote_font: String::new(),
        }
    }
}

impl Settings {
    /// Leser bare feltene som finnes nå. Ukjente felt (som `openCopy` fra eldre
    /// versjoner) og felt med feil type ignoreres, og ugyldige verdier gir standardverdien.
    pub fn from_json(json: &str) -> Settings {
        crate::json::over_default::<Settings>(json).sanitized()
    }

    /// Tom suffix, `maxChars` ≤ 0 og tomme
    /// oversettelser gir standardverdien.
    pub fn sanitized(mut self) -> Settings {
        let default = Settings::default();
        if self.suffix.trim().is_empty() {
            self.suffix = default.suffix;
        }
        if self.max_chars == 0 {
            self.max_chars = default.max_chars;
        }
        self.translation = self.translation.trim().to_string();
        if self.translation.is_empty() {
            self.translation = default.translation;
        }
        self.default_translation = self.default_translation.trim().to_string();
        if self.default_translation.is_empty() {
            self.default_translation = default.default_translation;
        }
        self.import_folder = self.import_folder.trim().to_string();
        self.slide_theme = self.slide_theme.trim().to_string();
        if self.slide_theme.is_empty() {
            self.slide_theme = default.slide_theme;
        }
        self.heading_font = self.heading_font.trim().to_string();
        self.quote_font = self.quote_font.trim().to_string();
        self
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("innstillinger som JSON")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ukjente_felt_fjernes_og_ugyldige_verdier_gir_standard() {
        let settings =
            Settings::from_json(r#"{"suffix":"","maxChars":-3,"translation":" nb24 ","trigger":":b","openCopy":false}"#);
        assert_eq!(
            settings,
            Settings { translation: "nb24".to_string(), ..Settings::default() }
        );
        assert!(!settings.to_json().contains("trigger"));
        assert!(!settings.to_json().contains("openCopy"));
        assert_eq!(Settings::from_json("ikke json"), Settings::default());
    }

    #[test]
    fn tema_og_skrifter_for_presentasjonen() {
        let settings = Settings::from_json(r#"{"slideTheme":" papir ","headingFont":"lora","quoteFont":3}"#);
        assert_eq!(settings.slide_theme, "papir");
        assert_eq!(settings.heading_font, "lora");
        assert_eq!(settings.quote_font, "");
        assert_eq!(Settings::from_json(r#"{"slideTheme":""}"#).slide_theme, "kveld");
    }
}
