//! Det appen holder i minnet mens den kjører.

use std::path::PathBuf;
use std::sync::Mutex;

use kerygma_core::bible::{Bibel, BibelConfig};
use kerygma_core::settings::Settings;
use kerygma_core::vault::Vault;
use notify::RecommendedWatcher;
use serde::{Deserialize, Serialize};

pub enum BibleState {
    Loading,
    Ready(Bibel),
    Failed(String),
}

pub struct OpenVault {
    pub vault: Vault,
    pub settings: Settings,
    /// Holdes i live så lenge vaulten er åpen; endringer utenfra blir `vault-changed`.
    pub _watcher: Option<RecommendedWatcher>,
}

impl OpenVault {
    pub fn settings_path(&self) -> PathBuf {
        self.vault.root().join(".kerygma").join("settings.json")
    }
}

pub struct AppState {
    pub vault: Mutex<Option<OpenVault>>,
    pub bible: Mutex<BibleState>,
}

impl Default for AppState {
    fn default() -> Self {
        AppState { vault: Mutex::new(None), bible: Mutex::new(BibleState::Loading) }
    }
}

impl AppState {
    /// Innstillingene til vaulten som er åpen, eller standardverdiene.
    pub fn settings(&self) -> Settings {
        self.vault.lock().unwrap().as_ref().map(|open| open.settings.clone()).unwrap_or_default()
    }

    /// Bruker innstillingene på `bibel`, hvis biblene er lastet.
    pub fn configure_bible(&self, settings: &Settings) -> Result<(), String> {
        if let BibleState::Ready(bibel) = &mut *self.bible.lock().unwrap() {
            bibel
                .configure(&BibelConfig {
                    default_translation: settings.default_translation.clone(),
                    aliases: settings.aliases.clone(),
                })
                .map_err(|e| format!("Innstillingene for bibel: {e}"))?;
        }
        Ok(())
    }
}

/// Det appen husker mellom hver gang den startes (`app.json` i konfigmappa).
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppConfig {
    pub last_vault: Option<String>,
    #[serde(flatten)]
    pub appearance: Appearance,
}

/// Skrift og størrelse i editoren og lesevisningen. Gjelder appen, ikke vaulten.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Appearance {
    /// Nøkkelen til en skrift i lista i grensesnittet (`ui/src/appearance.ts`).
    pub font: String,
    /// Skriftstørrelse i piksler.
    pub font_size: u32,
}

impl Appearance {
    pub const MIN_SIZE: u32 = 10;
    pub const MAX_SIZE: u32 = 40;

    pub fn sanitized(self) -> Appearance {
        let default = Appearance::default();
        Appearance {
            font: if self.font.trim().is_empty() { default.font } else { self.font },
            font_size: self.font_size.clamp(Self::MIN_SIZE, Self::MAX_SIZE),
        }
    }
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance { font: "georgia".to_string(), font_size: 17 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gammel_app_json_uten_skrift_gir_standardverdier() {
        let config: AppConfig = serde_json::from_str(r#"{"lastVault":"/tmp/v"}"#).unwrap();
        assert_eq!(config.last_vault.as_deref(), Some("/tmp/v"));
        assert_eq!(config.appearance.font, "georgia");
        assert_eq!(config.appearance.font_size, 17);
    }

    #[test]
    fn skriftstorrelsen_holdes_innenfor_grensene() {
        let config: AppConfig = serde_json::from_str(r#"{"font":"","fontSize":200}"#).unwrap();
        let appearance = config.appearance.sanitized();
        assert_eq!(appearance.font, "georgia");
        assert_eq!(appearance.font_size, Appearance::MAX_SIZE);
        let json = serde_json::to_string(&AppConfig { last_vault: None, appearance }).unwrap();
        assert_eq!(json, r#"{"lastVault":null,"font":"georgia","fontSize":40}"#);
    }
}
