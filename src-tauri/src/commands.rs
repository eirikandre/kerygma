//! Kommandoene grensesnittet kaller med `invoke`. De er tynne kall til
//! `kerygma-core`; feil kommer tilbake som en norsk melding.

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use kerygma_core::markdown;
use kerygma_core::presentation::{presentation_content, presentation_paths_for, presentation_source, slide_options};
use kerygma_core::settings::Settings;
use kerygma_core::vault::{Tree, Vault};
use kerygma_core::verse_query::{self, Suggestion};
use notify::{RecursiveMode, Watcher};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::state::{AppConfig, AppState, Appearance, BibleState, OpenVault};

type Result<T> = std::result::Result<T, String>;

/// Kjører `f` med vaulten som er åpen.
fn with_vault<T>(state: &AppState, f: impl FnOnce(&OpenVault) -> Result<T>) -> Result<T> {
    let guard = state.vault.lock().unwrap();
    let open = guard.as_ref().ok_or("Ingen mappe er åpen.")?;
    f(open)
}

fn app_config_path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|dir| dir.join("app.json"))
}

pub fn read_app_config(app: &AppHandle) -> AppConfig {
    app_config_path(app)
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

fn write_app_config(app: &AppHandle, config: &AppConfig) {
    if let Some(path) = app_config_path(app) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, serde_json::to_string_pretty(config).unwrap_or_default());
    }
}

/// Mappa som sist ble åpnet, hvis den fortsatt finnes.
#[tauri::command]
pub fn last_vault(app: AppHandle) -> Option<String> {
    read_app_config(&app).last_vault.filter(|path| std::path::Path::new(path).is_dir())
}

/// Skrift og størrelse, lagret i `app.json`.
#[tauri::command]
pub fn appearance(app: AppHandle) -> Appearance {
    read_app_config(&app).appearance.sanitized()
}

#[tauri::command]
pub fn set_appearance(app: AppHandle, appearance: Appearance) -> Appearance {
    let mut config = read_app_config(&app);
    config.appearance = appearance.sanitized();
    write_app_config(&app, &config);
    config.appearance
}

/// Sender `vault-changed` når filer endres utenfra, samlet over et kort øyeblikk.
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
                // Skjulte filer og mapper (.obsidian, .kerygma) vises ikke i treet.
                .filter(|path| !path.split('/').any(|part| part.starts_with('.')))
                .collect();
            paths.sort();
            paths.dedup();
            if !paths.is_empty() {
                let _ = app.emit("vault-changed", paths);
            }
        }
    });
    Some(watcher)
}

#[tauri::command]
pub fn open_vault(app: AppHandle, state: State<AppState>, path: String) -> Result<Tree> {
    let vault = Vault::open(&path)?;
    let settings = std::fs::read_to_string(vault.root().join(".kerygma").join("settings.json"))
        .map(|json| Settings::from_json(&json))
        .unwrap_or_default();
    app.asset_protocol_scope()
        .allow_directory(vault.root(), true)
        .map_err(|e| format!("Kunne ikke gi tilgang til bildene i mappa: {e}"))?;
    let watcher = watch(&app, &vault);
    let tree = vault.tree(&settings.suffix);
    let mut config = read_app_config(&app);
    config.last_vault = Some(vault.root().to_string_lossy().into_owned());
    write_app_config(&app, &config);
    let _ = state.configure_bible(&settings);
    *state.vault.lock().unwrap() = Some(OpenVault { vault, settings, _watcher: watcher });
    Ok(tree)
}

#[tauri::command]
pub fn tree(state: State<AppState>) -> Result<Tree> {
    with_vault(&state, |open| Ok(open.vault.tree(&open.settings.suffix)))
}

#[tauri::command]
pub fn read_note(state: State<AppState>, path: String) -> Result<String> {
    with_vault(&state, |open| open.vault.read_note(&path))
}

#[tauri::command]
pub fn write_note(state: State<AppState>, path: String, content: String) -> Result<()> {
    with_vault(&state, |open| open.vault.write_note(&path, &content))
}

#[tauri::command]
pub fn create_note(state: State<AppState>, folder: String) -> Result<String> {
    with_vault(&state, |open| open.vault.create_note(&folder))
}

#[tauri::command]
pub fn create_folder(state: State<AppState>, parent: String, name: String) -> Result<String> {
    with_vault(&state, |open| open.vault.create_folder(&parent, &name))
}

#[tauri::command]
pub fn rename(state: State<AppState>, path: String, new_name: String) -> Result<String> {
    with_vault(&state, |open| open.vault.rename(&path, &new_name))
}

#[tauri::command]
pub fn move_to(state: State<AppState>, path: String, folder: String) -> Result<String> {
    with_vault(&state, |open| open.vault.move_to(&path, &folder, &open.settings.suffix))
}

#[tauri::command]
pub fn delete(state: State<AppState>, path: String) -> Result<()> {
    with_vault(&state, |open| open.vault.delete(&path))
}

/// Søk i navn og innhold i alle manusene.
#[tauri::command(async)]
pub fn search(state: State<'_, AppState>, query: String) -> Result<Vec<kerygma_core::search::SearchResult>> {
    let (vault, suffix) = with_vault(&state, |open| Ok((open.vault.clone(), open.settings.suffix.clone())))?;
    Ok(kerygma_core::search::search(&vault, &query, &suffix))
}

/// HTML til lesevisningen, med bildene løst opp.
#[tauri::command]
pub fn render_note(state: State<AppState>, path: String, content: String) -> Result<String> {
    with_vault(&state, |open| {
        let images = open.vault.image_resolver(&path);
        Ok(markdown::render_html(&content, &images))
    })
}

#[tauri::command]
pub fn resolve_link(state: State<AppState>, target: String, from: String) -> Result<Option<String>> {
    with_vault(&state, |open| Ok(open.vault.resolve_link(&target, &from)))
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CreateResult {
    #[serde(rename_all = "camelCase")]
    Created { path: String, warnings: Vec<String> },
    /// En fil uten markøren har navnet; grensesnittet spør og kaller igjen med `overwrite`.
    Exists { path: String },
    MissingManuscript { path: String },
    IsFolder { path: String },
}

/// «Lag presentasjon». `content` er innholdet i editoren, så ulagrede
/// endringer blir med; det brukes bare når `path` er selve manuset.
#[tauri::command(async)]
pub fn create_presentation(
    state: State<AppState>,
    path: String,
    content: Option<String>,
    overwrite: bool,
) -> Result<CreateResult> {
    let (vault, settings) = with_vault(&state, |open| Ok((open.vault.clone(), open.settings.clone())))?;
    // Markøren i en presentasjon sier hvilket manus den er laget fra; ellers avgjør navnet.
    let note = match &content {
        Some(content) => content.clone(),
        None => vault.read_note(&path).unwrap_or_default(),
    };
    let paths = presentation_paths_for(&path, &note, &settings.suffix);
    if paths.manuscript == paths.presentation {
        return Err(format!("«{path}» er merket som en presentasjon laget fra seg selv."));
    }
    if paths.from_presentation && !vault.exists(&paths.manuscript) {
        return Ok(CreateResult::MissingManuscript { path: paths.manuscript });
    }
    if vault.is_folder(&paths.presentation) {
        return Ok(CreateResult::IsFolder { path: paths.presentation });
    }
    // En presentasjon appen har laget (med markøren), erstattes uten å spørre: den varsler selv
    // om at endringer blir overskrevet. Andre filer med samme navn spør vi om først.
    if vault.exists(&paths.presentation) && !overwrite {
        let generated = vault.read_note(&paths.presentation).is_ok_and(|text| presentation_source(&text).is_some());
        if !generated {
            return Ok(CreateResult::Exists { path: paths.presentation });
        }
    }
    let manuscript = match content {
        Some(content) if !paths.from_presentation => content,
        _ => vault.read_note(&paths.manuscript)?,
    };
    let (slides, warnings) = {
        let bible = state.bible.lock().unwrap();
        let bibel = match &*bible {
            BibleState::Ready(bibel) => Some(bibel),
            _ => None,
        };
        presentation_content(&manuscript, &kerygma_core::file_name(&paths.manuscript), &settings, bibel)
    };
    vault.write_note(&paths.presentation, &slides)?;
    Ok(CreateResult::Created { path: paths.presentation, warnings })
}

/// HTML per slide til «Vis presentasjon».
#[tauri::command]
pub fn slides(state: State<AppState>, path: String, content: Option<String>) -> Result<Vec<String>> {
    with_vault(&state, |open| {
        let content = match content {
            Some(content) => content,
            None => open.vault.read_note(&path)?,
        };
        let images = open.vault.image_resolver(&path);
        Ok(markdown::slides(&content, &images))
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Presenter {
    /// Manuset slidene er laget fra.
    path: String,
    slides: Vec<String>,
    /// Manuset som HTML, med merkene fra `kerygma_core::presenter`.
    manuscript: String,
    warnings: Vec<String>,
}

/// Talervisningen: slidene lages fra manuset (også når `path` er presentasjonen), og
/// manuset vises med det som er på hver slide merket. `content` er innholdet i editoren,
/// så ulagrede endringer blir med; det brukes bare når `path` er selve manuset.
#[tauri::command(async)]
pub fn presenter(state: State<AppState>, path: String, content: Option<String>) -> Result<Presenter> {
    let (vault, settings) = with_vault(&state, |open| Ok((open.vault.clone(), open.settings.clone())))?;
    let note = match &content {
        Some(content) => content.clone(),
        None => vault.read_note(&path)?,
    };
    let paths = presentation_paths_for(&path, &note, &settings.suffix);
    let manuscript = match content {
        Some(content) if !paths.from_presentation => content,
        _ => vault
            .read_note(&paths.manuscript)
            .map_err(|_| format!("Fant ikke manuset «{}».", paths.manuscript))?,
    };
    let (options, warnings) = {
        let bible = state.bible.lock().unwrap();
        let bibel = match &*bible {
            BibleState::Ready(bibel) => Some(bibel),
            _ => None,
        };
        slide_options(&manuscript, &settings, bibel)
    };
    let view = {
        let images = vault.image_resolver(&paths.manuscript);
        kerygma_core::presenter::presenter_view(&manuscript, &options, &images)
    };
    Ok(Presenter { path: paths.manuscript, slides: view.slides, manuscript: view.manuscript, warnings })
}

/// Presentasjonen som eksporteres: fila selv hvis det er en presentasjon, ellers
/// presentasjonen som er laget fra manuset. `None` når den ikke er laget ennå.
#[tauri::command]
pub fn presentation_path(state: State<AppState>, path: String, content: String) -> Result<Option<String>> {
    with_vault(&state, |open| {
        let paths = presentation_paths_for(&path, &content, &open.settings.suffix);
        Ok(open.vault.exists(&paths.presentation).then_some(paths.presentation))
    })
}

/// Starter en PDF-eksport: sidene som er laget før, glemmes.
#[tauri::command]
pub fn pdf_begin(state: State<AppState>) {
    state.pdf_pages.lock().unwrap().clear();
}

/// Lager en PDF-side av `rect` i vinduet (sliden som vises) og legger den til.
#[tauri::command(async)]
pub fn pdf_page(webview: tauri::Webview, state: State<'_, AppState>, rect: crate::capture::Rect) -> Result<()> {
    let page = crate::capture::pdf(&webview, rect)?;
    state.pdf_pages.lock().unwrap().push(page);
    Ok(())
}

/// Slår sammen sidene, skalert til full HD, og lagrer PDF-en på `path` (absolutt sti valgt av brukeren).
#[tauri::command(async)]
pub fn pdf_save(state: State<'_, AppState>, path: String) -> Result<()> {
    let pages = std::mem::take(&mut *state.pdf_pages.lock().unwrap());
    let pdf = kerygma_core::pdf::merge(&pages, Some(kerygma_core::pdf::FULL_HD))?;
    write_export(&path, &pdf, "pdf")
}

/// Lagrer en eksportert fil (PowerPoint eller HTML), med innholdet som rå bytes og stien i
/// overskriften `path` (URL-kodet). Stien er valgt av brukeren i et lagringsvindu.
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
    std::fs::write(path, data).map_err(|e| format!("Kunne ikke lagre «{}»: {e}", path.display()))
}

#[tauri::command(async)]
pub fn lookup_verse(state: State<AppState>, input: String) -> Vec<Suggestion> {
    match &*state.bible.lock().unwrap() {
        BibleState::Ready(bibel) => verse_query::suggest_verse(bibel, &input),
        BibleState::Loading => vec![Suggestion::message("Biblene lastes …")],
        BibleState::Failed(message) => vec![Suggestion::message(message.clone())],
    }
}

/// Verset som markdown-sitat ved markøren, som `toQuote`.
#[tauri::command]
pub fn quote_verse(verse: String, before: String, after: String) -> String {
    verse_query::to_quote(&verse, &before, &after)
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Settings {
    state.settings()
}

/// Lagrer innstillingene i vaulten og bygger bibel-registeret på nytt ved behov.
/// Gir en advarsel hvis innstillingene for bibel er ugyldige.
#[tauri::command(async)]
pub fn set_settings(state: State<AppState>, settings: Settings) -> Result<Vec<String>> {
    let settings = settings.sanitized();
    let path = {
        let mut guard = state.vault.lock().unwrap();
        let open = guard.as_mut().ok_or("Ingen mappe er åpen.")?;
        open.settings = settings.clone();
        open.settings_path()
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("Kunne ikke lagre innstillingene: {e}"))?;
    }
    std::fs::write(&path, settings.to_json()).map_err(|e| format!("Kunne ikke lagre innstillingene: {e}"))?;
    Ok(state.configure_bible(&settings).err().into_iter().collect())
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum BibleStatus {
    Loading,
    Ready { ids: Vec<String> },
    Failed { message: String },
}

#[tauri::command(async)]
pub fn bible_status(state: State<AppState>) -> BibleStatus {
    match &*state.bible.lock().unwrap() {
        BibleState::Loading => BibleStatus::Loading,
        BibleState::Ready(bibel) => BibleStatus::Ready { ids: bibel.ids().iter().map(|s| s.to_string()).collect() },
        BibleState::Failed(message) => BibleStatus::Failed { message: message.clone() },
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportTarget {
    /// Den absolutte stien til .docx-fila.
    file: String,
    name: String,
    /// Manuset den blir til, relativt til vault-roten.
    path: String,
    exists: bool,
    is_folder: bool,
}

/// Hvor hver Word-fil havner: i mappa brukeren valgte i treet (`folder_chosen`), ellers i
/// «Mappe for importerte Word-manus», eller i mappa til fila som er åpen. Grensesnittet
/// spør før det overskriver.
#[tauri::command]
pub fn import_targets(
    state: State<AppState>,
    files: Vec<String>,
    current_folder: String,
    folder_chosen: bool,
) -> Result<Vec<ImportTarget>> {
    with_vault(&state, |open| {
        let folder = match open.settings.import_folder.trim() {
            folder if !folder.is_empty() && !folder_chosen => folder.trim_matches('/').to_string(),
            _ => current_folder.trim_matches('/').to_string(),
        };
        let file_names: Vec<String> = files
            .iter()
            .map(|file| std::path::Path::new(file).file_name().unwrap_or_default().to_string_lossy().into_owned())
            .collect();
        let names = kerygma_core::word::note_names(&file_names);
        Ok(files
            .into_iter()
            .zip(names)
            .map(|(file, name)| {
                let path = if folder.is_empty() { format!("{name}.md") } else { format!("{folder}/{name}.md") };
                ImportTarget {
                    exists: open.vault.exists(&path),
                    is_folder: open.vault.is_folder(&path),
                    file,
                    name,
                    path,
                }
            })
            .collect())
    })
}

#[derive(serde::Deserialize)]
pub struct ImportRequest {
    file: String,
    path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedFile {
    name: String,
    path: Option<String>,
    quotes: usize,
    /// Sitater med `[!warning|sjekk]`.
    uncertain: usize,
    unmatched: Vec<String>,
    error: Option<String>,
}

/// Gjør Word-filene om til markdown. Kjøres utenfor hovedtråden, siden søk i Bibelen kan ta litt tid.
#[tauri::command(async)]
pub fn import_word(state: State<'_, AppState>, files: Vec<ImportRequest>) -> Result<Vec<ImportedFile>> {
    let vault = with_vault(&state, |open| Ok(open.vault.clone()))?;
    let bible = state.bible.lock().unwrap();
    let bibel = match &*bible {
        BibleState::Ready(bibel) => bibel,
        BibleState::Loading => return Err("Biblene lastes fortsatt. Prøv igjen om litt.".to_string()),
        BibleState::Failed(message) => return Err(format!("Kunne ikke laste biblene: {message}")),
    };
    let options = kerygma_core::word::ConvertOptions::default();
    Ok(files
        .into_iter()
        .map(|request| {
            let name = kerygma_core::path_name(&request.path);
            let result = std::fs::read(&request.file)
                .map_err(|e| format!("kunne ikke lese fila: {e}"))
                .and_then(|data| kerygma_core::word::import_docx(&data, bibel, &options))
                .and_then(|result| vault.write_note(&request.path, &result.markdown).map(|_| result));
            match result {
                Ok(result) => ImportedFile {
                    name,
                    path: Some(request.path),
                    quotes: result.quotes.len(),
                    uncertain: result.quotes.iter().filter(|q| q.warning.is_some()).count(),
                    unmatched: result.unmatched,
                    error: None,
                },
                Err(error) => ImportedFile { name, path: None, quotes: 0, uncertain: 0, unmatched: vec![], error: Some(error) },
            }
        })
        .collect())
}
