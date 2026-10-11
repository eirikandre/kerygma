//! Kerygma som skrivebordsapp. All logikk ligger i `kerygma-core`; her er
//! vinduet, menyen, kommandoene og lastingen av biblene.

mod capture;
mod commands;
mod state;

use std::path::PathBuf;

use kerygma_core::bible::Bibel;
use tauri::menu::{Menu, MenuItemBuilder, PredefinedMenuItem, Submenu, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::state::{AppState, BibleState};

/// Mappa med `*.bin`: ressursene i appen, eller `bibler/` i repoet under utvikling.
fn bibles_dir(app: &AppHandle) -> Option<PathBuf> {
    let bundled = app.path().resource_dir().ok().map(|dir| dir.join("bibler"));
    let repo = cfg!(debug_assertions).then(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../bibler"));
    [bundled, repo].into_iter().flatten().find(|dir| dir.join("nb88.bin").exists() || dir.join("kjv.bin").exists())
}

/// Laster biblene i en egen tråd, så vinduet ikke venter. Sender `bible-ready` når de er klare.
fn load_bibles(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        let loaded = match bibles_dir(&app) {
            Some(dir) => Bibel::load_dir(&dir),
            None => Err("Fant ingen oversettelser (*.bin) i appen.".to_string()),
        };
        match loaded {
            Ok(bibel) => {
                *state.bible.lock().unwrap() = BibleState::Ready(bibel);
                // Innstillingene leses etterpå: åpnes vaulten mens biblene lastes, er det dens som gjelder.
                if let Err(message) = state.configure_bible(&state.settings()) {
                    let _ = app.emit("bible-warning", message);
                }
            }
            Err(message) => *state.bible.lock().unwrap() = BibleState::Failed(message),
        }
        let _ = app.emit("bible-ready", ());
    });
}

/// Et menyvalg som sender `menu` med id-en til grensesnittet. Hurtigtastene
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
    let window = SubmenuBuilder::new(app, "Vindu")
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
    submenus.extend([file, edit, manus, window]);
    let items: Vec<&dyn tauri::menu::IsMenuItem<Wry>> =
        submenus.iter().map(|s| s as &dyn tauri::menu::IsMenuItem<Wry>).collect();
    Menu::with_items(app, &items)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .setup(|app| {
            let handle = app.handle();
            app.set_menu(menu(handle)?)?;
            app.on_menu_event(|app, event| {
                let _ = app.emit("menu", event.id().0.as_str());
            });
            load_bibles(handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
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
            commands::pdf_begin,
            commands::pdf_page,
            commands::pdf_save,
            commands::save_export,
            commands::lookup_verse,
            commands::quote_verse,
            commands::get_settings,
            commands::set_settings,
            commands::bible_status,
            commands::import_targets,
            commands::import_word,
        ])
        .run(tauri::generate_context!())
        .expect("kunne ikke starte Kerygma");
}
