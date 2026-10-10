//! Innstillingene, med de samme feltene og standardverdiene som pluginen.
//! Lagres per vault i `<vault>/.kerygma/settings.json`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::extract::DEFAULT_MAX_CHARS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub suffix: String,
    pub open_copy: bool,
    pub max_chars: usize,
    pub add_translation: bool,
    pub translation: String,
    /// Oversettelsen et oppslag uten prefiks bruker, som `BIBEL_DEFAULT`.
    pub default_translation: String,
    /// Snarveier som "ne:joh3:16", skrevet som `BIBEL_ALIAS`.
    pub aliases: String,
    /// Hvor importerte Word-manus legges. Tom: mappa til notatet som er åpent.
    pub import_folder: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            suffix: " - presentasjon".to_string(),
            open_copy: true,
            max_chars: DEFAULT_MAX_CHARS,
            add_translation: true,
            translation: "kjv".to_string(),
            default_translation: "nb88".to_string(),
            aliases: "n=nb88,e=kjv,ne=nb88+kjv".to_string(),
            import_folder: String::new(),
        }
    }
}

impl Settings {
    /// Leser bare feltene som finnes nå, som `loadSettings`. Ukjente felt og
    /// felt med feil type ignoreres, og ugyldige verdier gir standardverdien.
    pub fn from_json(json: &str) -> Settings {
        let saved: Value = serde_json::from_str(json).unwrap_or(Value::Null);
        let mut settings = Settings::default();
        let text = |key: &str| saved.get(key).and_then(Value::as_str).map(String::from);
        let flag = |key: &str| saved.get(key).and_then(Value::as_bool);
        if let Some(v) = text("suffix") {
            settings.suffix = v;
        }
        if let Some(v) = flag("openCopy") {
            settings.open_copy = v;
        }
        if let Some(v) = saved.get("maxChars").and_then(Value::as_i64) {
            settings.max_chars = usize::try_from(v).unwrap_or(0);
        }
        if let Some(v) = flag("addTranslation") {
            settings.add_translation = v;
        }
        if let Some(v) = text("translation") {
            settings.translation = v;
        }
        if let Some(v) = text("defaultTranslation") {
            settings.default_translation = v;
        }
        if let Some(v) = text("aliases") {
            settings.aliases = v;
        }
        if let Some(v) = text("importFolder") {
            settings.import_folder = v;
        }
        settings.sanitized()
    }

    /// Som innstillingssiden i pluginen: tom suffix, `maxChars` ≤ 0 og tomme
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
            Settings { translation: "nb24".to_string(), open_copy: false, ..Settings::default() }
        );
        assert!(!settings.to_json().contains("trigger"));
        assert_eq!(Settings::from_json("ikke json"), Settings::default());
    }
}
