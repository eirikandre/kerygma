//! Kerygma som skrivebordsapp. All logikk ligger i `kerygma-core`; her er
//! vinduet, menyen, kommandoene og lastingen av biblene.

mod capture;
mod commands;
mod events;
mod state;

use std::path::PathBuf;
use std::sync::Arc;

use kerygma_core::app_config::Appearance;
use kerygma_core::bible::Bibel;
use tauri::menu::{Menu, MenuItemBuilder, PredefinedMenuItem, Submenu, SubmenuBuilder};
use tauri::{AppHandle, Manager, Wry};
use tauri_specta::{collect_commands, collect_events, ErrorHandlingMode, Event};

use crate::events::{BibleReady, BibleWarning, VaultChanged};
use crate::state::{AppState, BibleState};

/// Mappa med `*.bin` appen kom med: ressursene i appen, eller `bibler/` i repoet under utvikling.
fn bibles_dir(app: &AppHandle) -> Option<PathBuf> {
    let bundled = app.path().resource_dir().ok().map(|dir| dir.join("bibler"));
    let repo = cfg!(debug_assertions).then(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../bibler"));
    let has_bibles = |dir: &PathBuf| {
        std::fs::read_dir(dir).is_ok_and(|entries| {
            entries.filter_map(Result::ok).any(|entry| entry.path().extension().is_some_and(|ext| ext == "bin"))
        })
    };
    [bundled, repo].into_iter().flatten().find(has_bibles)
}

/// Mappa der brukeren kan legge flere oversettelser (`*.bin`) etter at appen er installert:
/// `bibler` i datamappa til appen. Den overlever nye versjoner og krever ikke administrator.
pub(crate) fn user_bibles_dir(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|dir| dir.join("bibler"))
}

/// Laster biblene i en egen tråd, så vinduet ikke venter: de appen kom med, og de brukeren har
/// lagt i sin egen mappe (som går foran ved samme oversettelse). Sender `bible-ready` når de er klare.
pub(crate) fn load_bibles(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        let dirs: Vec<PathBuf> = [bibles_dir(&app), user_bibles_dir(&app)].into_iter().flatten().collect();
        let dirs: Vec<&std::path::Path> = dirs.iter().map(PathBuf::as_path).collect();
        let loaded = Bibel::load_dirs(&dirs).map_err(|e| format!("Fant ingen oversettelser: {e}"));
        match loaded {
            Ok(bibel) => {
                state.set_bible(BibleState::Ready(Arc::new(bibel)));
                // Innstillingene leses etterpå: åpnes vaulten mens biblene lastes, er det dens som gjelder.
                if let Err(message) = state.configure_bible(&state.settings()) {
                    let _ = BibleWarning(message).emit(&app);
                }
            }
            Err(message) => state.set_bible(BibleState::Failed(message)),
        }
        let _ = BibleReady.emit(&app);
    });
}

/// Et menyvalg som sender `Menu` med id-en til grensesnittet. Hurtigtastene
/// håndteres i grensesnittet; på macOS vises de også i menyen.
fn item(app: &AppHandle, id: &str, text: &str, accelerator: &str) -> tauri::Result<tauri::menu::MenuItem<Wry>> {
    let builder = MenuItemBuilder::with_id(id, text);
    let builder = if cfg!(target_os = "macos") && !accelerator.is_empty() {
        builder.accelerator(accelerator)
    } else {
        builder
    };
    builder.build(app)
}

fn menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let file = SubmenuBuilder::new(app, "Fil")
        .item(&item(app, "new-note", "Nytt manus", "CmdOrCtrl+N")?)
        .item(&item(app, "search", "Søk i alle manus …", "CmdOrCtrl+Shift+F")?)
        .item(&item(app, "open-vault", "Åpne mappe …", "")?)
        .item(&item(app, "import-word", "Importer Word-manus …", "")?)
        .separator()
        .item(&item(app, "settings", "Innstillinger …", "CmdOrCtrl+,")?)
        .separator()
        .item(&PredefinedMenuItem::close_window(app, Some("Lukk vindu"))?)
        .build()?;
    let edit = SubmenuBuilder::new(app, "Rediger")
        .item(&PredefinedMenuItem::undo(app, Some("Angre"))?)
        .item(&PredefinedMenuItem::redo(app, Some("Gjør om"))?)
        .separator()
        .item(&PredefinedMenuItem::cut(app, Some("Klipp ut"))?)
        .item(&PredefinedMenuItem::copy(app, Some("Kopier"))?)
        .item(&PredefinedMenuItem::paste(app, Some("Lim inn"))?)
        .item(&PredefinedMenuItem::select_all(app, Some("Merk alt"))?)
        .build()?;
    let manus = SubmenuBuilder::new(app, "Manus")
        .item(&item(app, "toggle-preview", "Bytt mellom redigering og lesevisning", "CmdOrCtrl+E")?)
        .separator()
        .item(&item(app, "create-presentation", "Lag presentasjon av manus", "CmdOrCtrl+Shift+M")?)
        .item(&item(app, "show-presentation", "Vis presentasjon", "CmdOrCtrl+Shift+P")?)
        .item(&item(app, "presenter-window", "Talervisning i eget vindu", "CmdOrCtrl+Alt+Shift+P")?)
        .item(&item(app, "insert-verse", "Sett inn bibelvers …", "CmdOrCtrl+Shift+B")?)
        .separator()
        .item(&item(app, "export-pdf", "Eksporter presentasjonen som PDF …", "")?)
        .item(&item(app, "export-pptx", "Eksporter presentasjonen som PowerPoint …", "")?)
        .item(&item(app, "export-html", "Eksporter presentasjonen som HTML …", "")?)
        .build()?;
    let help = SubmenuBuilder::new(app, "Hjelp").item(&item(app, "help", "Brukerveiledning", "")?).build()?;
    let window = SubmenuBuilder::new(app, "Vindu")
        .item(&item(app, "toggle-sidebar", "Vis eller skjul navigasjonstreet", "CmdOrCtrl+Shift+T")?)
        .separator()
        .item(&PredefinedMenuItem::minimize(app, Some("Minimer"))?)
        .item(&PredefinedMenuItem::fullscreen(app, Some("Fullskjerm"))?)
        .build()?;

    let mut submenus: Vec<Submenu<Wry>> = Vec::new();
    if cfg!(target_os = "macos") {
        submenus.push(
            SubmenuBuilder::new(app, "Kerygma")
                .item(&PredefinedMenuItem::about(app, Some("Om Kerygma"), None)?)
                .separator()
                .item(&PredefinedMenuItem::services(app, Some("Tjenester"))?)
                .separator()
                .item(&PredefinedMenuItem::hide(app, Some("Skjul Kerygma"))?)
                .item(&PredefinedMenuItem::hide_others(app, Some("Skjul andre"))?)
                .separator()
                .item(&PredefinedMenuItem::quit(app, Some("Avslutt Kerygma"))?)
                .build()?,
        );
    }
    submenus.extend([file, edit, manus, window, help]);
    let items: Vec<&dyn tauri::menu::IsMenuItem<Wry>> =
        submenus.iter().map(|s| s as &dyn tauri::menu::IsMenuItem<Wry>).collect();
    Menu::with_items(app, &items)
}

/// Kommandoene, hendelsene og konstantene grensesnittet bruker. `ui/src/bindings.ts` lages
/// herfra (når appen startes under utvikling, og i testen under), så typene i Rust og
/// TypeScript ikke kan gli fra hverandre.
fn specta_builder() -> tauri_specta::Builder<Wry> {
    tauri_specta::Builder::<Wry>::new()
        // Feil kastes som en streng, som med `invoke`.
        .error_handling(ErrorHandlingMode::Throw)
        // `usize` (antall, linjenumre, `maxChars`) blir `number`; tallene er små.
        .dangerously_cast_bigints_to_number()
        .commands(collect_commands![
            commands::help_html,
            commands::last_vault,
            commands::appearance,
            commands::set_appearance,
            commands::open_vault,
            commands::tree,
            commands::read_note,
            commands::write_note,
            commands::create_note,
            commands::create_folder,
            commands::rename,
            commands::move_to,
            commands::delete,
            commands::search,
            commands::render_note,
            commands::resolve_link,
            commands::create_presentation,
            commands::slides,
            commands::presenter,
            commands::presentation_path,
            commands::add_image_file,
            commands::clean_images,
            commands::image_url,
            commands::pdf_begin,
            commands::pdf_page,
            commands::pdf_save,
            commands::pdf_cancel,
            commands::lookup_verse,
            commands::quote_verse,
            commands::get_settings,
            commands::set_settings,
            commands::bible_status,
            commands::bibles_folder,
            commands::reload_bibles,
            commands::import_targets,
            commands::import_word,
        ])
        .events(collect_events![events::Menu, VaultChanged, BibleReady, BibleWarning])
        .constant("PRESENTATION_MARKER_PATTERN", kerygma_core::presentation::MARKER_PATTERN)
        .constant("PRESENTER_START", kerygma_core::presenter::START)
        .constant("PRESENTER_END", kerygma_core::presenter::END)
        .constant("DEFAULT_SETTINGS", kerygma_core::settings::Settings::default())
        .constant("DEFAULT_APPEARANCE", Appearance::default())
        .constant("APPEARANCE_MIN_SIZE", Appearance::MIN_SIZE)
        .constant("APPEARANCE_MAX_SIZE", Appearance::MAX_SIZE)
}

/// Lager `bindings.ts` på `path`.
fn export_bindings(builder: &tauri_specta::Builder<Wry>, path: &std::path::Path) -> Result<(), String> {
    let typescript = specta_typescript::Typescript::default()
        .header("// Laget av src-tauri/src/lib.rs (`specta_builder`). Ikke rediger for hånd.")
        ;
    builder.export(typescript, path).map_err(|e| format!("Kunne ikke lage bindings.ts: {e}"))
}

fn bindings_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../ui/src/bindings.ts")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = specta_builder();
    #[cfg(debug_assertions)]
    if let Err(message) = export_bindings(&builder, &bindings_path()) {
        eprintln!("{message}");
    }
    // `save_export` og `add_image_data` tar imot rå bytes og er ikke med i `bindings.ts`.
    let typed = builder.invoke_handler();
    let raw: fn(tauri::ipc::Invoke<Wry>) -> bool =
        tauri::generate_handler![commands::save_export, commands::add_image_data];
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(move |invoke| match invoke.message.command() {
            "save_export" | "add_image_data" => raw(invoke),
            _ => typed(invoke),
        })
        .setup(move |app| {
            builder.mount_events(app);
            let handle = app.handle();
            app.set_menu(menu(handle)?)?;
            app.on_menu_event(|app, event| {
                let _ = events::Menu(event.id().0.clone()).emit(app);
            });
            load_bibles(handle);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("kunne ikke starte Kerygma");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `ui/src/bindings.ts` skal være laget fra kommandoene slik de er nå. Feiler testen,
    /// kjør `cargo test -p kerygma-app` med `KERYGMA_OPPDATER_BINDINGS=1`, eller start appen.
    #[test]
    fn bindings_ts_er_oppdatert() {
        let builder = specta_builder();
        if std::env::var_os("KERYGMA_OPPDATER_BINDINGS").is_some() {
            export_bindings(&builder, &bindings_path()).unwrap();
        }
        let temp = std::env::temp_dir().join(format!("kerygma-bindings-{}.ts", std::process::id()));
        export_bindings(&builder, &temp).unwrap();
        let fresh = std::fs::read_to_string(&temp).unwrap();
        let _ = std::fs::remove_file(&temp);
        let saved = std::fs::read_to_string(bindings_path()).unwrap_or_default();
        assert!(
            fresh.replace("\r\n", "\n") == saved.replace("\r\n", "\n"),
            "ui/src/bindings.ts er utdatert. Kjør KERYGMA_OPPDATER_BINDINGS=1 cargo test -p kerygma-app"
        );
    }
}
