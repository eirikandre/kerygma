//! Kommandoene grensesnittet kaller. De er tynne kall til `kerygma-core`; feil kommer
//! tilbake som en norsk melding. Typene og kallene i `ui/src/bindings.ts` lages herfra
//! (se `lib.rs`), unntatt `save_export`, som tar imot rå bytes.
//!
//! Ingen kommando holder en lås mens den jobber: den henter en kopi av vaulten
//! (`AppState::workspace`) og biblene (`AppState::bible`) og slipper låsen.

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use kerygma_core::app_config::{AppConfig, Appearance};
use kerygma_core::search::SearchResponse;
use kerygma_core::settings::Settings;
use kerygma_core::vault::{Tree, Vault};
use kerygma_core::verse_query::{self, Suggestion};
use kerygma_core::workspace::{CreateResult, ImportRequest, ImportTarget, ImportedFile, Presenter, Workspace};
use notify::{RecursiveMode, Watcher};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;

use crate::events::VaultChanged;
use crate::state::{AppState, BibleState, OpenVault};

type Result<T> = std::result::Result<T, String>;

fn app_config_path(app: &AppHandle) -> Result<PathBuf> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join("app.json"))
        .map_err(|e| format!("Fant ikke mappa til oppsettet: {e}"))
}

pub fn read_app_config(app: &AppHandle) -> AppConfig {
    app_config_path(app).map(|path| AppConfig::load(&path)).unwrap_or_default()
}

fn write_app_config(app: &AppHandle, config: &AppConfig) -> Result<()> {
    config.save(&app_config_path(app)?)
}

/// Brukerveiledningen som HTML, til hjelpevinduet. Bygget inn i appen fra
/// `docs/Brukerveiledning.md`, så den alltid hører til versjonen som kjører.
#[tauri::command]
#[specta::specta]
pub fn help_html() -> String {
    kerygma_core::markdown::render_document(include_str!("../../docs/Brukerveiledning.md"))
}

/// Mappa som sist ble åpnet, hvis den fortsatt finnes.
#[tauri::command]
#[specta::specta]
pub fn last_vault(app: AppHandle) -> Option<String> {
    read_app_config(&app).last_vault.filter(|path| std::path::Path::new(path).is_dir())
}

/// Skrift og størrelse, lagret i `app.json`.
#[tauri::command]
#[specta::specta]
pub fn appearance(app: AppHandle) -> Appearance {
    read_app_config(&app).appearance.sanitized()
}

#[tauri::command]
#[specta::specta]
pub fn set_appearance(app: AppHandle, appearance: Appearance) -> Result<Appearance> {
    let mut config = read_app_config(&app);
    config.appearance = appearance.sanitized();
    write_app_config(&app, &config)?;
    Ok(config.appearance)
}

/// Sender `VaultChanged` når filer endres utenfra, samlet over et kort øyeblikk.
fn watch(app: &AppHandle, vault: &Vault) -> Option<notify::RecommendedWatcher> {
    let (tx, rx) = mpsc::channel::<notify::Result<notify::Event>>();
    let mut watcher = notify::recommended_watcher(tx).ok()?;
    watcher.watch(vault.root(), RecursiveMode::Recursive).ok()?;
    let app = app.clone();
    let vault = vault.clone();
    std::thread::spawn(move || {
        while let Ok(first) = rx.recv() {
            let mut events = vec![first];
            while let Ok(event) = rx.recv_timeout(Duration::from_millis(250)) {
                events.push(event);
            }
            let mut paths: Vec<String> = events
                .into_iter()
                .filter_map(|event| event.ok())
                .flat_map(|event| event.paths)
                .filter_map(|path| vault.relative(&path))
                // Skjulte filer og mapper (.obsidian, .kerygma, midlertidige filer) vises ikke i treet.
                .filter(|path| !path.split('/').any(|part| part.starts_with('.')))
                .collect();
            paths.sort();
            paths.dedup();
            if !paths.is_empty() {
                let _ = VaultChanged(paths).emit(&app);
            }
        }
    });
    Some(watcher)
}

#[tauri::command]
#[specta::specta]
pub fn open_vault(app: AppHandle, state: State<AppState>, path: String) -> Result<Tree> {
    let workspace = Workspace::open(&path)?;
    app.asset_protocol_scope()
        .allow_directory(workspace.vault.root(), true)
        .map_err(|e| format!("Kunne ikke gi tilgang til bildene i mappa: {e}"))?;
    let watcher = watch(&app, &workspace.vault);
    let tree = workspace.tree();
    let mut config = read_app_config(&app);
    config.last_vault = Some(workspace.vault.root().to_string_lossy().into_owned());
    // Mappa er åpnet selv om den ikke kan huskes til neste gang.
    if let Err(message) = write_app_config(&app, &config) {
        eprintln!("{message}");
    }
    let _ = state.configure_bible(&workspace.settings);
    *state.vault.lock().unwrap() = Some(OpenVault { workspace, _watcher: watcher });
    Ok(tree)
}

#[tauri::command]
#[specta::specta]
pub fn tree(state: State<AppState>) -> Result<Tree> {
    Ok(state.workspace()?.tree())
}

#[tauri::command]
#[specta::specta]
pub fn read_note(state: State<AppState>, path: String) -> Result<String> {
    state.workspace()?.vault.read_note(&path)
}

#[tauri::command]
#[specta::specta]
pub fn write_note(state: State<AppState>, path: String, content: String) -> Result<()> {
    state.workspace()?.vault.write_note(&path, &content)
}

#[tauri::command]
#[specta::specta]
pub fn create_note(state: State<AppState>, folder: String) -> Result<String> {
    state.workspace()?.vault.create_note(&folder)
}

#[tauri::command]
#[specta::specta]
pub fn create_folder(state: State<AppState>, parent: String, name: String) -> Result<String> {
    state.workspace()?.vault.create_folder(&parent, &name)
}

#[tauri::command]
#[specta::specta]
pub fn rename(state: State<AppState>, path: String, new_name: String) -> Result<String> {
    state.workspace()?.vault.rename(&path, &new_name)
}

#[tauri::command]
#[specta::specta]
pub fn move_to(state: State<AppState>, path: String, folder: String) -> Result<String> {
    state.workspace()?.move_to(&path, &folder)
}

#[tauri::command]
#[specta::specta]
/// Legger fila eller mappa i papirkurven; et manus tar med seg presentasjonene og bildene sine.
/// Gir stiene som ble slettet.
pub fn delete(state: State<AppState>, path: String) -> Result<Vec<String>> {
    state.workspace()?.delete(&path)
}

/// Søk i navn og innhold i alle manusene.
#[tauri::command(async)]
#[specta::specta]
pub fn search(state: State<'_, AppState>, query: String) -> Result<SearchResponse> {
    Ok(state.workspace()?.search(&query))
}

/// HTML til lesevisningen, med bildene løst opp.
#[tauri::command]
#[specta::specta]
pub fn render_note(state: State<AppState>, path: String, content: String) -> Result<String> {
    Ok(state.workspace()?.render_note(&path, &content))
}

#[tauri::command]
#[specta::specta]
pub fn resolve_link(state: State<AppState>, target: String, from: String) -> Result<Option<String>> {
    Ok(state.workspace()?.vault.resolve_link(&target, &from))
}

/// «Lag presentasjon». `content` er innholdet i editoren, så ulagrede endringer blir med.
#[tauri::command(async)]
#[specta::specta]
pub fn create_presentation(
    state: State<'_, AppState>,
    path: String,
    content: Option<String>,
    overwrite: bool,
) -> Result<CreateResult> {
    let bible = state.bible();
    state.workspace()?.create_presentation(&path, content.as_deref(), overwrite, bible.ready())
}

/// HTML per slide til «Vis presentasjon» og eksporten.
#[tauri::command]
#[specta::specta]
pub fn slides(state: State<AppState>, path: String, content: Option<String>) -> Result<Vec<String>> {
    state.workspace()?.slides(&path, content)
}

/// Talervisningen: slidene laget fra manuset, og manuset med det som er på hver slide merket.
#[tauri::command(async)]
#[specta::specta]
pub fn presenter(state: State<'_, AppState>, path: String, content: Option<String>) -> Result<Presenter> {
    let bible = state.bible();
    state.workspace()?.presenter(&path, content.as_deref(), bible.ready())
}

/// Legger bildet fra `source` (en fil valgt av brukeren) i bildemappa til manuset, og gir
/// markdown som viser det: `![[kart.png]]`.
#[tauri::command(async)]
#[specta::specta]
pub fn add_image_file(state: State<'_, AppState>, path: String, source: String) -> Result<String> {
    state.workspace()?.add_image_file(&path, std::path::Path::new(&source))
}

/// Som `add_image_file`, med bildet som rå bytes (dra og slipp fra Finder eller Utforsker)
/// og manuset og filnavnet i overskriftene `path` og `name` (URL-kodet).
/// Ikke med i `bindings.ts`, siden innholdet ikke er JSON.
#[tauri::command(async)]
pub fn add_image_data(state: State<'_, AppState>, request: tauri::ipc::Request<'_>) -> Result<String> {
    let tauri::ipc::InvokeBody::Raw(data) = request.body() else {
        return Err("Bildet mangler innhold.".to_string());
    };
    let header = |name: &str| {
        request.headers().get(name).and_then(|value| value.to_str().ok()).map(kerygma_core::vault::percent_decode)
    };
    let path = header("path").ok_or("Bildet mangler manus.")?;
    let name = header("name").ok_or("Bildet mangler filnavn.")?;
    state.workspace()?.add_image(&path, &name, data)
}

/// Legger bildene i bildemappa til manuset som ingen manus viser til lenger, i papirkurven.
/// Kalles når manuset lukkes, så angre virker mens man skriver. Gir antallet som ble slettet.
#[tauri::command(async)]
#[specta::specta]
pub fn clean_images(state: State<'_, AppState>, path: String, content: String) -> Result<u32> {
    Ok(state.workspace()?.clean_images(&path, &content)?.len() as u32)
}

/// URL-en til et bilde manuset viser til (som `kart.png` i `![[kart.png]]`), til
/// miniatyrbildene i editoren. `None` når bildet ikke finnes.
#[tauri::command]
#[specta::specta]
pub fn image_url(state: State<AppState>, path: String, target: String) -> Result<Option<String>> {
    let workspace = state.workspace()?;
    let resolver = workspace.vault.image_resolver(&path);
    Ok(resolver(&target, !target.contains('/')))
}

/// Presentasjonen som eksporteres, eller `None` når den ikke er laget ennå.
#[tauri::command]
#[specta::specta]
pub fn presentation_path(state: State<AppState>, path: String, content: String) -> Result<Option<String>> {
    Ok(state.workspace()?.presentation_path(&path, &content))
}

/// Starter en PDF-eksport og gir id-en den bruker i `pdf_page` og `pdf_save`.
#[tauri::command]
#[specta::specta]
pub fn pdf_begin(state: State<AppState>) -> u32 {
    state.pdf_exports.lock().unwrap().begin()
}

/// Lager en PDF-side av `rect` i vinduet (sliden som vises) og legger den til eksporten.
#[tauri::command(async)]
#[specta::specta]
pub fn pdf_page(webview: tauri::Webview, state: State<'_, AppState>, id: u32, rect: crate::capture::Rect) -> Result<()> {
    let page = crate::capture::pdf(&webview, rect)?;
    let mut exports = state.pdf_exports.lock().unwrap();
    exports.pages.get_mut(&id).ok_or("PDF-eksporten er avbrutt.")?.push(page);
    Ok(())
}

/// Slår sammen sidene, skalert til full HD, og lagrer PDF-en på `path` (absolutt sti valgt av brukeren).
#[tauri::command(async)]
#[specta::specta]
pub fn pdf_save(state: State<'_, AppState>, id: u32, path: String) -> Result<()> {
    let pages = state.pdf_exports.lock().unwrap().pages.remove(&id).ok_or("PDF-eksporten er avbrutt.")?;
    let pdf = kerygma_core::pdf::merge(&pages, Some(kerygma_core::pdf::FULL_HD))?;
    write_export(&path, &pdf, "pdf")
}

/// Avbryter en PDF-eksport og glemmer sidene. Gjør ingenting hvis den allerede er lagret.
#[tauri::command]
#[specta::specta]
pub fn pdf_cancel(state: State<AppState>, id: u32) {
    state.pdf_exports.lock().unwrap().pages.remove(&id);
}

/// Lagrer en eksportert fil (PowerPoint eller HTML), med innholdet som rå bytes og stien i
/// overskriften `path` (URL-kodet). Stien er valgt av brukeren i et lagringsvindu.
/// Ikke med i `bindings.ts`, siden innholdet ikke er JSON.
#[tauri::command(async)]
pub fn save_export(request: tauri::ipc::Request<'_>) -> Result<()> {
    let tauri::ipc::InvokeBody::Raw(data) = request.body() else {
        return Err("Fila mangler innhold.".to_string());
    };
    let path = request
        .headers()
        .get("path")
        .and_then(|value| value.to_str().ok())
        .map(kerygma_core::vault::percent_decode)
        .ok_or("Fila mangler sti.")?;
    let extension = std::path::Path::new(&path).extension().and_then(|e| e.to_str()).unwrap_or("");
    let extension = if extension.eq_ignore_ascii_case("html") { "html" } else { "pptx" };
    write_export(&path, data, extension)
}

/// Skriver bare filer med filtypen eksporten lager, så kommandoen ikke kan brukes til annet.
fn write_export(path: &str, data: &[u8], extension: &str) -> Result<()> {
    let path = std::path::Path::new(path);
    if !path.is_absolute() || !path.extension().is_some_and(|e| e.eq_ignore_ascii_case(extension)) {
        return Err(format!("Kan bare lagre .{extension}-filer."));
    }
    kerygma_core::vault::write_atomic(path, data).map_err(|e| format!("Kunne ikke lagre «{}»: {e}", path.display()))
}

#[tauri::command(async)]
#[specta::specta]
pub fn lookup_verse(state: State<'_, AppState>, input: String) -> Vec<Suggestion> {
    match state.bible() {
        BibleState::Ready(bibel) => verse_query::suggest_verse(&bibel, &input),
        BibleState::Loading => vec![Suggestion::message("Biblene lastes …")],
        BibleState::Failed(message) => vec![Suggestion::message(message)],
    }
}

/// Verset som markdown-sitat ved markøren, som `toQuote`.
#[tauri::command]
#[specta::specta]
pub fn quote_verse(verse: String, before: String, after: String) -> String {
    verse_query::to_quote(&verse, &before, &after)
}

/// Viser eller skjuler menylinjen i vinduet, så presentasjonen i fullskjerm fyller hele
/// skjermen. Menyen er i vinduet bare på Windows og Linux; på macOS gjør dette ingenting.
#[tauri::command]
#[specta::specta]
pub fn set_menu_visible(window: tauri::Window, visible: bool) -> Result<()> {
    if visible { window.show_menu() } else { window.hide_menu() }.map_err(|e| format!("Kunne ikke endre menyen: {e}"))
}

#[tauri::command]
#[specta::specta]
pub fn get_settings(state: State<AppState>) -> Settings {
    state.settings()
}

/// Lagrer innstillingene i vaulten og bygger bibel-registeret på nytt ved behov.
/// Gir en advarsel hvis innstillingene for bibel er ugyldige.
#[tauri::command(async)]
#[specta::specta]
pub fn set_settings(state: State<'_, AppState>, settings: Settings) -> Result<Vec<String>> {
    let mut workspace = state.workspace()?;
    workspace.save_settings(settings)?;
    let settings = workspace.settings.clone();
    if let Some(open) = state.vault.lock().unwrap().as_mut() {
        // Samme vault som da innstillingene ble lagret (den kan ha blitt byttet imens).
        if open.workspace.vault.root() == workspace.vault.root() {
            open.workspace.settings = settings.clone();
        }
    }
    // Meldinger om oversettelser som ikke er installert, vises når brukeren lagrer,
    // men ikke ved oppstart: en vault kan vise til oversettelser denne versjonen ikke har.
    Ok(state.configure_bible(&settings).unwrap_or_else(|error| vec![error]))
}

/// Mappa der brukeren kan legge flere oversettelser (`*.bin`). Lages hvis den ikke finnes.
#[tauri::command]
#[specta::specta]
pub fn bibles_folder(app: AppHandle) -> Result<String> {
    let dir = crate::user_bibles_dir(&app).ok_or("Fant ikke mappa til appen.")?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Kunne ikke lage «{}»: {e}", dir.display()))?;
    Ok(dir.to_string_lossy().into_owned())
}

/// Laster oversettelsene på nytt, etter at brukeren har lagt til eller fjernet filer i mappa.
/// Sender `bible-ready` når de er klare.
#[tauri::command]
#[specta::specta]
pub fn reload_bibles(app: AppHandle, state: State<AppState>) {
    state.set_bible(BibleState::Loading);
    crate::load_bibles(&app);
}

/// En oversettelse som er lastet: id-en (som "kjv") og det fulle navnet.
#[derive(Serialize, specta::Type)]
pub struct Translation {
    pub id: String,
    pub title: String,
}

#[derive(Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum BibleStatus {
    Loading,
    Ready { translations: Vec<Translation> },
    Failed { message: String },
}

#[tauri::command(async)]
#[specta::specta]
pub fn bible_status(state: State<'_, AppState>) -> BibleStatus {
    match state.bible() {
        BibleState::Loading => BibleStatus::Loading,
        BibleState::Ready(bibel) => BibleStatus::Ready {
            translations: bibel
                .titles()
                .into_iter()
                .map(|(id, title)| Translation { id: id.to_string(), title: title.to_string() })
                .collect(),
        },
        BibleState::Failed(message) => BibleStatus::Failed { message },
    }
}

/// Hvor hver Word-fil havner. Grensesnittet spør før det overskriver.
#[tauri::command]
#[specta::specta]
pub fn import_targets(
    state: State<AppState>,
    files: Vec<String>,
    current_folder: String,
    folder_chosen: bool,
) -> Result<Vec<ImportTarget>> {
    Ok(state.workspace()?.import_targets(&files, &current_folder, folder_chosen))
}

/// Gjør Word-filene om til markdown. Kjøres utenfor hovedtråden, siden søk i Bibelen kan ta litt tid.
#[tauri::command(async)]
#[specta::specta]
pub fn import_word(state: State<'_, AppState>, files: Vec<ImportRequest>) -> Result<Vec<ImportedFile>> {
    let workspace = state.workspace()?;
    match state.bible() {
        BibleState::Ready(bibel) => Ok(workspace.import_word(&files, &bibel)),
        BibleState::Loading => Err("Biblene lastes fortsatt. Prøv igjen om litt.".to_string()),
        BibleState::Failed(message) => Err(format!("Kunne ikke laste biblene: {message}")),
    }
}
