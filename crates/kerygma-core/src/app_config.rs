//! Det appen husker mellom hver gang den startes (`app.json` i konfigmappa):
//! mappa som sist ble åpnet, og utseendet. Gjelder appen, ikke vaulten.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::vault::write_atomic;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub last_vault: Option<String>,
    #[serde(flatten)]
    pub appearance: Appearance,
}

/// Skrift og størrelse i editoren og lesevisningen, og lyst eller mørkt tema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Appearance {
    /// Nøkkelen til en skrift i lista i grensesnittet (`ui/src/appearance.ts`).
    pub font: String,
    /// Skriftstørrelse i piksler.
    pub font_size: u32,
    pub theme: Theme,
}

/// Lyst eller mørkt tema i appen, eller det systemet bruker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

impl Appearance {
    pub const MIN_SIZE: u32 = 10;
    pub const MAX_SIZE: u32 = 40;

    pub fn sanitized(self) -> Appearance {
        let default = Appearance::default();
        Appearance {
            font: if self.font.trim().is_empty() { default.font } else { self.font },
            font_size: self.font_size.clamp(Self::MIN_SIZE, Self::MAX_SIZE),
            theme: self.theme,
        }
    }
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance { font: "georgia".to_string(), font_size: 17, theme: Theme::System }
    }
}

impl AppConfig {
    /// Leser `app.json`. Mangler fila, eller er den ugyldig, brukes standardverdiene.
    pub fn load(path: &Path) -> AppConfig {
        std::fs::read_to_string(path).map(|json| AppConfig::from_json(&json)).unwrap_or_default()
    }

    /// Felt som mangler eller har feil type, får standardverdien (som i eldre `app.json`).
    pub fn from_json(json: &str) -> AppConfig {
        crate::json::over_default(json)
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("Kunne ikke lagre oppsettet til appen: {e}"))?;
        }
        let json = serde_json::to_string_pretty(self).expect("oppsettet som JSON");
        write_atomic(path, json.as_bytes()).map_err(|e| format!("Kunne ikke lagre oppsettet til appen: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gammel_app_json_uten_skrift_gir_standardverdier() {
        let config = AppConfig::from_json(r#"{"lastVault":"/tmp/v"}"#);
        assert_eq!(config.last_vault.as_deref(), Some("/tmp/v"));
        assert_eq!(config.appearance, Appearance::default());
    }

    #[test]
    fn skriftstorrelsen_holdes_innenfor_grensene() {
        let config = AppConfig::from_json(r#"{"font":"","fontSize":200,"theme":"blå"}"#);
        let appearance = config.appearance.sanitized();
        assert_eq!(appearance.font, "georgia");
        assert_eq!(appearance.font_size, Appearance::MAX_SIZE);
        assert_eq!(appearance.theme, Theme::System);
        let json = serde_json::to_string(&AppConfig { last_vault: None, appearance }).unwrap();
        assert_eq!(json, r#"{"lastVault":null,"font":"georgia","fontSize":40,"theme":"system"}"#);
    }

    #[test]
    fn lagres_og_leses_igjen() {
        let dir = std::env::temp_dir().join(format!("kerygma-app-config-{}", std::process::id()));
        let path = dir.join("app.json");
        let config = AppConfig { last_vault: Some("/v".into()), appearance: Appearance::default() };
        config.save(&path).unwrap();
        assert_eq!(AppConfig::load(&path).last_vault.as_deref(), Some("/v"));
        assert_eq!(AppConfig::load(&dir.join("finnes-ikke.json")).last_vault, None);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
