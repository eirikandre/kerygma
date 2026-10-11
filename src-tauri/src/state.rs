//! Det appen holder i minnet mens den kjører.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use kerygma_core::bible::{Bibel, BibelConfig};
use kerygma_core::settings::Settings;
use kerygma_core::workspace::Workspace;
use notify::RecommendedWatcher;

/// Biblene. `Ready` holder en `Arc`, så en kommando kan kopiere den og slippe låsen
/// før den gjør noe: et langt søk eller en Word-import stopper ikke andre oppslag.
#[derive(Clone)]
pub enum BibleState {
    Loading,
    Ready(Arc<Bibel>),
    Failed(String),
}

impl BibleState {
    pub fn ready(&self) -> Option<&Bibel> {
        match self {
            BibleState::Ready(bibel) => Some(bibel),
            _ => None,
        }
    }
}

pub struct OpenVault {
    pub workspace: Workspace,
    /// Holdes i live så lenge vaulten er åpen; endringer utenfra blir `VaultChanged`.
    pub _watcher: Option<RecommendedWatcher>,
}

pub struct AppState {
    pub vault: Mutex<Option<OpenVault>>,
    bible: Mutex<BibleState>,
    /// Sidene til PDF-ene som eksporteres (én PDF per slide), per eksport, til de slås sammen.
    pub pdf_exports: Mutex<PdfExports>,
}

#[derive(Default)]
pub struct PdfExports {
    next: u32,
    pub pages: HashMap<u32, Vec<Vec<u8>>>,
}

impl PdfExports {
    /// Starter en ny eksport og gir id-en til den.
    pub fn begin(&mut self) -> u32 {
        self.next = self.next.wrapping_add(1);
        self.pages.insert(self.next, Vec::new());
        self.next
    }
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            vault: Mutex::new(None),
            bible: Mutex::new(BibleState::Loading),
            pdf_exports: Mutex::new(PdfExports::default()),
        }
    }
}

impl AppState {
    /// En kopi av vaulten som er åpen, med innstillingene. Låsen slippes med en gang.
    pub fn workspace(&self) -> Result<Workspace, String> {
        self.vault.lock().unwrap().as_ref().map(|open| open.workspace.clone()).ok_or_else(|| "Ingen mappe er åpen.".to_string())
    }

    /// Innstillingene til vaulten som er åpen, eller standardverdiene.
    pub fn settings(&self) -> Settings {
        self.vault.lock().unwrap().as_ref().map(|open| open.workspace.settings.clone()).unwrap_or_default()
    }

    /// Biblene slik de er nå. Låsen slippes med en gang.
    pub fn bible(&self) -> BibleState {
        self.bible.lock().unwrap().clone()
    }

    pub fn set_bible(&self, bible: BibleState) {
        *self.bible.lock().unwrap() = bible;
    }

    /// Bruker innstillingene på biblene, hvis de er lastet. Registeret bygges uten lås,
    /// og byttes inn etterpå. Gir meldinger om oversettelser som ikke er installert
    /// (`Bibel::configured`); en feil betyr at innstillingene er ugyldige.
    pub fn configure_bible(&self, settings: &Settings) -> Result<Vec<String>, String> {
        let BibleState::Ready(bibel) = self.bible() else { return Ok(Vec::new()) };
        let config = BibelConfig { default_translation: settings.default_translation.clone(), aliases: settings.aliases.clone() };
        if bibel.config() == Some(&config) {
            return Ok(Vec::new());
        }
        let (configured, notes) = bibel.configured(&config).map_err(|e| format!("Innstillingene for bibel: {e}"))?;
        self.set_bible(BibleState::Ready(Arc::new(configured)));
        Ok(notes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hver_pdf_eksport_har_egne_sider() {
        let mut exports = PdfExports::default();
        let a = exports.begin();
        let b = exports.begin();
        assert_ne!(a, b);
        exports.pages.get_mut(&a).unwrap().push(vec![1]);
        assert_eq!(exports.pages[&b].len(), 0);
        assert_eq!(exports.pages.remove(&a), Some(vec![vec![1]]));
    }
}
