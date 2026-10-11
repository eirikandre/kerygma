//! En åpen vault med innstillingene sine, og flytene appen og CLI-en bruker:
//! «Lag presentasjon», talervisningen, eksport og Word-import.
//!
//! Alt her er uten Tauri. `src-tauri/src/commands.rs` henter en `Workspace`
//! (en billig kopi) og kaller én metode, så ingen lås holdes mens arbeidet gjøres.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::bible::Bibel;
use crate::markdown;
use crate::presentation::{presentation_content, presentation_paths_for, presentation_source, slide_options, PresentationPaths};
use crate::settings::Settings;
use crate::vault::{write_atomic, Tree, Vault};

/// Mappa med innstillingene i vaulten. Skjult, så den ikke vises i treet.
pub const SETTINGS_DIR: &str = ".kerygma";
pub const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Clone)]
pub struct Workspace {
    pub vault: Vault,
    pub settings: Settings,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CreateResult {
    #[serde(rename_all = "camelCase")]
    Created { path: String, warnings: Vec<String> },
    /// En fil uten markøren har navnet; grensesnittet spør og kaller igjen med `overwrite`.
    Exists { path: String },
    MissingManuscript { path: String },
    IsFolder { path: String },
}

/// Talervisningen: slidene laget fra manuset, og manuset med merker for hver slide.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Presenter {
    /// Manuset slidene er laget fra.
    pub path: String,
    pub slides: Vec<String>,
    /// Manuset som HTML, med merkene fra [`crate::presenter`].
    pub manuscript: String,
    pub warnings: Vec<String>,
}

/// Hvor en Word-fil havner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ImportTarget {
    /// Den absolutte stien til .docx-fila.
    pub file: String,
    pub name: String,
    /// Manuset den blir til, relativt til vault-roten.
    pub path: String,
    pub exists: bool,
    pub is_folder: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ImportRequest {
    /// Den absolutte stien til .docx-fila.
    pub file: String,
    /// Manuset den skal bli til, relativt til vault-roten.
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ImportedFile {
    pub name: String,
    pub path: Option<String>,
    pub quotes: usize,
    /// Sitater med `[!warning|sjekk]`.
    pub uncertain: usize,
    pub unmatched: Vec<String>,
    pub error: Option<String>,
}

impl Workspace {
    pub fn new(vault: Vault, settings: Settings) -> Workspace {
        Workspace { vault, settings }
    }

    /// Åpner mappa og leser innstillingene. Mangler de, eller er de ugyldige, brukes standardverdiene.
    pub fn open(path: impl Into<PathBuf>) -> Result<Workspace, String> {
        let vault = Vault::open(path)?;
        let settings = std::fs::read_to_string(settings_path(vault.root()))
            .map(|json| Settings::from_json(&json))
            .unwrap_or_default();
        Ok(Workspace { vault, settings })
    }

    pub fn settings_path(&self) -> PathBuf {
        settings_path(self.vault.root())
    }

    /// Rydder i innstillingene, lagrer dem i vaulten og bruker dem.
    pub fn save_settings(&mut self, settings: Settings) -> Result<(), String> {
        let settings = settings.sanitized();
        let path = self.settings_path();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("Kunne ikke lagre innstillingene: {e}"))?;
        }
        write_atomic(&path, settings.to_json().as_bytes()).map_err(|e| format!("Kunne ikke lagre innstillingene: {e}"))?;
        self.settings = settings;
        Ok(())
    }

    pub fn tree(&self) -> Tree {
        self.vault.tree(&self.settings.suffix)
    }

    pub fn move_to(&self, path: &str, folder: &str) -> Result<String, String> {
        self.vault.move_to(path, folder, &self.settings.suffix)
    }

    pub fn search(&self, query: &str) -> crate::search::SearchResponse {
        crate::search::search(&self.vault, query, &self.settings.suffix)
    }

    /// HTML til lesevisningen, med bildene løst opp.
    pub fn render_note(&self, path: &str, content: &str) -> String {
        markdown::render_html(content, &self.vault.image_resolver(path))
    }

    /// HTML per slide til «Vis presentasjon» og eksporten. `content` er innholdet i
    /// editoren; mangler det, leses fila.
    pub fn slides(&self, path: &str, content: Option<String>) -> Result<Vec<String>, String> {
        let content = match content {
            Some(content) => content,
            None => self.vault.read_note(path)?,
        };
        Ok(markdown::slides(&content, &self.vault.image_resolver(path)))
    }

    /// Stiene for fila, og innholdet i den: `content` hvis det er gitt, ellers fra disk.
    fn paths(&self, path: &str, content: Option<&str>) -> (PresentationPaths, String) {
        let note = match content {
            Some(content) => content.to_string(),
            None => self.vault.read_note(path).unwrap_or_default(),
        };
        (presentation_paths_for(path, &note, &self.settings.suffix), note)
    }

    /// Manuset presentasjonen lages fra. `content` (editoren) brukes bare når fila er selve manuset.
    fn manuscript(&self, paths: &PresentationPaths, note: String, content: Option<&str>) -> Result<String, String> {
        match content {
            Some(_) if !paths.from_presentation => Ok(note),
            _ => self
                .vault
                .read_note(&paths.manuscript)
                .map_err(|_| format!("Fant ikke manuset «{}».", paths.manuscript)),
        }
    }

    /// Innholdet i presentasjonen for fila, uten å skrive den: stiene, teksten og meldinger om
    /// oversettelsen ikke kunne legges til. Brukes av `kerygma lag-presentasjon` uten `--skriv`.
    pub fn presentation(
        &self,
        path: &str,
        content: Option<&str>,
        bibel: Option<&Bibel>,
    ) -> Result<(PresentationPaths, String, Vec<String>), String> {
        let (paths, note) = self.paths(path, content);
        if paths.manuscript == paths.presentation {
            return Err(format!("«{path}» er merket som en presentasjon laget fra seg selv."));
        }
        let manuscript = self.manuscript(&paths, note, content)?;
        let source = crate::file_name(&paths.manuscript);
        let (slides, warnings) = presentation_content(&manuscript, &source, &self.settings, bibel);
        Ok((paths, slides, warnings))
    }

    /// «Lag presentasjon». `content` er innholdet i editoren, så ulagrede endringer blir med;
    /// det brukes bare når `path` er selve manuset. Kjøres den fra en presentasjon, lages den
    /// på nytt fra manuset.
    pub fn create_presentation(
        &self,
        path: &str,
        content: Option<&str>,
        overwrite: bool,
        bibel: Option<&Bibel>,
    ) -> Result<CreateResult, String> {
        let (paths, _) = self.paths(path, content);
        if paths.manuscript == paths.presentation {
            return Err(format!("«{path}» er merket som en presentasjon laget fra seg selv."));
        }
        if paths.from_presentation && !self.vault.exists(&paths.manuscript) {
            return Ok(CreateResult::MissingManuscript { path: paths.manuscript });
        }
        if self.vault.is_folder(&paths.presentation) {
            return Ok(CreateResult::IsFolder { path: paths.presentation });
        }
        // En presentasjon appen har laget (med markøren), erstattes uten å spørre: den varsler selv
        // om at endringer blir overskrevet. Andre filer med samme navn spør vi om først.
        if self.vault.exists(&paths.presentation) && !overwrite {
            let generated =
                self.vault.read_note(&paths.presentation).is_ok_and(|text| presentation_source(&text).is_some());
            if !generated {
                return Ok(CreateResult::Exists { path: paths.presentation });
            }
        }
        let (paths, slides, warnings) = self.presentation(path, content, bibel)?;
        self.vault.write_note(&paths.presentation, &slides)?;
        Ok(CreateResult::Created { path: paths.presentation, warnings })
    }

    /// Talervisningen: slidene lages fra manuset (også når `path` er presentasjonen), og
    /// manuset vises med det som er på hver slide merket. `content` som i `create_presentation`.
    pub fn presenter(&self, path: &str, content: Option<&str>, bibel: Option<&Bibel>) -> Result<Presenter, String> {
        if content.is_none() {
            self.vault.read_note(path)?;
        }
        let (paths, note) = self.paths(path, content);
        let manuscript = self.manuscript(&paths, note, content)?;
        let (options, warnings) = slide_options(&manuscript, &self.settings, bibel);
        let view =
            crate::presenter::presenter_view(&manuscript, &options, &self.vault.image_resolver(&paths.manuscript));
        Ok(Presenter { path: paths.manuscript, slides: view.slides, manuscript: view.manuscript, warnings })
    }

    /// Presentasjonen som eksporteres: fila selv hvis det er en presentasjon, ellers
    /// presentasjonen som er laget fra manuset. `None` når den ikke er laget ennå.
    pub fn presentation_path(&self, path: &str, content: &str) -> Option<String> {
        let paths = presentation_paths_for(path, content, &self.settings.suffix);
        self.vault.exists(&paths.presentation).then_some(paths.presentation)
    }

    /// Hvor hver Word-fil havner: i mappa brukeren valgte i treet (`folder_chosen`), ellers i
    /// «Mappe for importerte Word-manus», eller i mappa til fila som er åpen (`current_folder`).
    pub fn import_targets(&self, files: &[String], current_folder: &str, folder_chosen: bool) -> Vec<ImportTarget> {
        let folder = match self.settings.import_folder.trim() {
            folder if !folder.is_empty() && !folder_chosen => folder.trim_matches('/'),
            _ => current_folder.trim_matches('/'),
        };
        let file_names: Vec<String> = files
            .iter()
            .map(|file| Path::new(file).file_name().unwrap_or_default().to_string_lossy().into_owned())
            .collect();
        let names = crate::word::note_names(&file_names);
        files
            .iter()
            .zip(names)
            .map(|(file, name)| {
                let path = if folder.is_empty() { format!("{name}.md") } else { format!("{folder}/{name}.md") };
                ImportTarget {
                    exists: self.vault.exists(&path),
                    is_folder: self.vault.is_folder(&path),
                    file: file.clone(),
                    name,
                    path,
                }
            })
            .collect()
    }

    /// Gjør Word-filene om til manus. En fil som ikke kan leses eller gjøres om, gir en feil
    /// i rapporten for den fila, og de andre importeres likevel.
    pub fn import_word(&self, files: &[ImportRequest], bibel: &Bibel) -> Vec<ImportedFile> {
        let options = crate::word::ConvertOptions::default();
        let translations =
            crate::word::import_translations(bibel, &self.settings.default_translation, &self.settings.translation);
        files
            .iter()
            .map(|request| {
                let name = crate::path_name(&request.path);
                let result = std::fs::read(&request.file)
                    .map_err(|e| format!("kunne ikke lese fila: {e}"))
                    .and_then(|data| crate::word::import_docx(&data, bibel, &translations, &options))
                    .and_then(|result| self.vault.write_note(&request.path, &result.markdown).map(|()| result));
                match result {
                    Ok(result) => ImportedFile {
                        name,
                        path: Some(request.path.clone()),
                        quotes: result.quotes.len(),
                        uncertain: result.quotes.iter().filter(|q| q.warning.is_some()).count(),
                        unmatched: result.unmatched,
                        error: None,
                    },
                    Err(error) => {
                        ImportedFile { name, path: None, quotes: 0, uncertain: 0, unmatched: vec![], error: Some(error) }
                    }
                }
            })
            .collect()
    }
}

fn settings_path(root: &Path) -> PathBuf {
    root.join(SETTINGS_DIR).join(SETTINGS_FILE)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// En tom vault i en midlertidig mappe, med oversettelser av.
    fn temp_workspace(name: &str) -> (PathBuf, Workspace) {
        let dir = std::env::temp_dir().join(format!("kerygma-workspace-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("Prekener")).unwrap();
        let mut workspace = Workspace::open(&dir).unwrap();
        workspace.settings.add_translation = false;
        (dir, workspace)
    }

    fn write(dir: &Path, path: &str, content: &str) {
        std::fs::write(dir.join(path), content).unwrap();
    }

    fn read(dir: &Path, path: &str) -> String {
        std::fs::read_to_string(dir.join(path)).unwrap()
    }

    #[test]
    fn innstillingene_lagres_i_vaulten_og_leses_igjen() {
        let (dir, mut workspace) = temp_workspace("innstillinger");
        assert_eq!(Workspace::open(&dir).unwrap().settings, Settings::default());
        workspace
            .save_settings(Settings { max_chars: 0, translation: " nb30 ".into(), ..Settings::default() })
            .unwrap();
        assert_eq!(workspace.settings.translation, "nb30");
        assert_eq!(workspace.settings.max_chars, Settings::default().max_chars);
        assert_eq!(Workspace::open(&dir).unwrap().settings, workspace.settings);
        assert!(dir.join(".kerygma/settings.json").is_file());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn presentasjonen_lages_med_ulagrede_endringer_og_erstattes_uten_a_sporre() {
        let (dir, workspace) = temp_workspace("lag");
        write(&dir, "Prekener/Påske.md", "# Gammel\n");
        let result = workspace.create_presentation("Prekener/Påske.md", Some("# Ny\n"), false, None).unwrap();
        let path = "Prekener/Påske - presentasjon.md";
        assert_eq!(result, CreateResult::Created { path: path.into(), warnings: vec![] });
        assert_eq!(read(&dir, path), "<!-- kerygma-presentasjon: Påske.md -->\n\n# Ny\n");

        // Fra presentasjonen lages den på nytt fra manuset på disk; innholdet i editoren gjelder ikke.
        let result = workspace.create_presentation(path, Some("# Redigert\n"), false, None).unwrap();
        assert!(matches!(result, CreateResult::Created { .. }));
        assert_eq!(read(&dir, path), "<!-- kerygma-presentasjon: Påske.md -->\n\n# Gammel\n");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn en_fil_uten_markor_overskrives_bare_etter_sporsmal() {
        let (dir, workspace) = temp_workspace("overskriv");
        write(&dir, "Påske.md", "# Påske\n");
        write(&dir, "Påske - presentasjon.md", "Skrevet for hånd");
        assert_eq!(
            workspace.create_presentation("Påske.md", None, false, None).unwrap(),
            CreateResult::Exists { path: "Påske - presentasjon.md".into() }
        );
        assert_eq!(read(&dir, "Påske - presentasjon.md"), "Skrevet for hånd");
        assert!(matches!(
            workspace.create_presentation("Påske.md", None, true, None).unwrap(),
            CreateResult::Created { .. }
        ));
        assert_eq!(read(&dir, "Påske - presentasjon.md"), "<!-- kerygma-presentasjon: Påske.md -->\n\n# Påske\n");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn manglende_manus_mappe_og_markor_til_seg_selv() {
        let (dir, workspace) = temp_workspace("feil");
        write(&dir, "Lysbilder.md", "<!-- kerygma-presentasjon: Borte.md -->\n");
        assert_eq!(
            workspace.create_presentation("Lysbilder.md", None, false, None).unwrap(),
            CreateResult::MissingManuscript { path: "Borte.md".into() }
        );
        write(&dir, "Jul.md", "# Jul\n");
        std::fs::create_dir_all(dir.join("Jul - presentasjon.md")).unwrap();
        assert_eq!(
            workspace.create_presentation("Jul.md", None, false, None).unwrap(),
            CreateResult::IsFolder { path: "Jul - presentasjon.md".into() }
        );
        write(&dir, "Seg selv.md", "<!-- kerygma-presentasjon: Seg selv.md -->\n");
        assert!(workspace.create_presentation("Seg selv.md", None, false, None).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn talervisningen_bruker_manuset_ogsa_fra_presentasjonen() {
        let (dir, workspace) = temp_workspace("taler");
        write(&dir, "Påske.md", "# Påske\n\n> Et sitat\n");
        write(&dir, "Lysbilder.md", "<!-- kerygma-presentasjon: Påske.md -->\n\n# Noe annet\n");
        let view = workspace.presenter("Lysbilder.md", None, None).unwrap();
        assert_eq!(view.path, "Påske.md");
        assert_eq!(view.slides.len(), 2);
        assert!(view.manuscript.contains("Et sitat"));

        // Fra manuset brukes innholdet i editoren.
        let view = workspace.presenter("Påske.md", Some("# Ulagret\n"), None).unwrap();
        assert_eq!(view.slides, ["<h1>Ulagret</h1>\n"]);

        write(&dir, "Gammel.md", "<!-- kerygma-presentasjon: Borte.md -->\n");
        assert_eq!(workspace.presenter("Gammel.md", None, None).unwrap_err(), "Fant ikke manuset «Borte.md».");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn presentasjonen_som_eksporteres() {
        let (dir, workspace) = temp_workspace("eksport");
        write(&dir, "Påske.md", "# Påske\n");
        assert_eq!(workspace.presentation_path("Påske.md", "# Påske\n"), None);
        workspace.create_presentation("Påske.md", None, false, None).unwrap();
        assert_eq!(workspace.presentation_path("Påske.md", "# Påske\n").as_deref(), Some("Påske - presentasjon.md"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn word_filer_havner_i_valgt_mappe_eller_importmappa() {
        let (dir, mut workspace) = temp_workspace("import");
        write(&dir, "Prekener/Påske.md", "");
        let files = ["/a/Påske.docx".to_string(), "/b/påske.docx".to_string()];
        let paths = |targets: Vec<ImportTarget>| targets.into_iter().map(|t| (t.path, t.exists)).collect::<Vec<_>>();
        assert_eq!(
            paths(workspace.import_targets(&files, "Prekener", false)),
            [("Prekener/Påske.md".to_string(), true), ("Prekener/påske 2.md".to_string(), false)]
        );
        workspace.settings.import_folder = "/Word/".into();
        assert_eq!(workspace.import_targets(&files, "Prekener", false)[0].path, "Word/Påske.md");
        // Mappa valgt i treet går foran innstillingen.
        assert_eq!(workspace.import_targets(&files, "", true)[0].path, "Påske.md");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
