//! Hendelsene Rust sender til grensesnittet. Navnet på hendelsen er navnet på typen
//! i kebab-case (`VaultChanged` blir `vault-changed`), og `ui/src/bindings.ts` har
//! `events.vaultChanged.listen(...)` osv.

use serde::{Deserialize, Serialize};

/// Et menyvalg, med id-en fra `lib.rs::menu` (som `"new-note"`).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct Menu(pub String);

/// Filer endret utenfra (Obsidian, Finder, Word-import), relativt til vault-roten.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct VaultChanged(pub Vec<String>);

/// Biblene er lastet, eller kunne ikke lastes (se `bible_status`).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct BibleReady;

/// Innstillingene for bibel kunne ikke brukes da biblene ble lastet.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct BibleWarning(pub String);
