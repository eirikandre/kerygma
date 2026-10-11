//! En vault: en mappe med markdown-filer, som i Obsidian.
//!
//! Alle stier inn og ut er relative til vault-roten og bruker `/`. Stier som
//! går ut av vaulten (`..`, absolutte stier), avvises.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

use serde::Serialize;

use crate::presentation::{presentation_paths, presentation_paths_for};

pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "svg", "webp", "bmp", "avif"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Kind {
    Folder,
    Note,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Tree {
    /// Navnet som vises: manus uten `.md`, andre filer med filendelse.
    pub name: String,
    /// Stien relativt til vault-roten, med `/`. Tom for roten.
    pub path: String,
    pub kind: Kind,
    pub children: Vec<Tree>,
}

#[derive(Debug, Clone)]
pub struct Vault {
    root: PathBuf,
}

/// Er dette et bilde, ut fra filendelsen?
pub fn is_image(path: &str) -> bool {
    path.rsplit_once('.').is_some_and(|(_, ext)| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
}

/// Mappa en relativ sti ligger i, med `/` på slutten (tom for roten).
pub fn parent_folder(path: &str) -> &str {
    path.rfind('/').map_or("", |i| &path[..=i])
}

fn join(folder: &str, name: &str) -> String {
    let folder = folder.trim_matches('/');
    if folder.is_empty() {
        name.to_string()
    } else {
        format!("{folder}/{name}")
    }
}

fn hidden(name: &str) -> bool {
    name.starts_with('.')
}

impl Vault {
    pub fn open(root: impl Into<PathBuf>) -> Result<Vault, String> {
        let root: PathBuf = root.into();
        let root = root.canonicalize().map_err(|e| format!("Fant ikke mappa «{}»: {e}", root.display()))?;
        if !root.is_dir() {
            return Err(format!("«{}» er ikke en mappe.", root.display()));
        }
        Ok(Vault { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Den absolutte stien til en relativ sti i vaulten.
    pub fn resolve(&self, path: &str) -> Result<PathBuf, String> {
        let relative = Path::new(path);
        let mut resolved = self.root.clone();
        for component in relative.components() {
            match component {
                Component::Normal(part) => resolved.push(part),
                Component::CurDir => {}
                _ => return Err(format!("Ugyldig sti «{path}»: den går ut av vaulten.")),
            }
        }
        Ok(resolved)
    }

    /// Den relative stien (med `/`) til en absolutt sti i vaulten.
    pub fn relative(&self, path: &Path) -> Option<String> {
        let relative = path.strip_prefix(&self.root).ok()?;
        let parts: Vec<String> = relative.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
        Some(parts.join("/"))
    }

    /// Alle manus (`.md`) i vaulten, uten skjulte mapper, sortert etter sti.
    pub fn notes(&self) -> Vec<String> {
        let mut notes = Vec::new();
        let mut stack = vec![(self.root.clone(), String::new())];
        while let Some((dir, prefix)) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            for entry in entries.filter_map(Result::ok) {
                let name = entry.file_name().to_string_lossy().into_owned();
                if hidden(&name) {
                    continue;
                }
                let path = join(&prefix, &name);
                if entry.path().is_dir() {
                    stack.push((entry.path(), path));
                } else if name.ends_with(".md") {
                    notes.push(path);
                }
            }
        }
        notes.sort();
        notes
    }

    /// Mappene og filene i vaulten. Presentasjoner (`suffix` i navnet eller markøren fra
    /// «Lag presentasjon») vises ikke når manuset ligger ved siden av; de åpnes fra manuset.
    pub fn tree(&self, suffix: &str) -> Tree {
        Tree {
            name: self.root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
            path: String::new(),
            kind: Kind::Folder,
            children: self.children(&self.root, "", suffix),
        }
    }

    /// Om fila er en presentasjon med manuset i samme mappe. Bare starten av fila
    /// leses: markøren står øverst, etter en eventuell frontmatter.
    fn is_presentation(&self, path: &str, suffix: &str) -> bool {
        const HEAD: u64 = 16 * 1024;
        let mut head = Vec::new();
        let Ok(file) = std::fs::File::open(self.root.join(path)) else { return false };
        if file.take(HEAD).read_to_end(&mut head).is_err() {
            return false;
        }
        self.is_presentation_content(path, &String::from_utf8_lossy(&head), suffix)
    }

    /// Som `is_presentation`, når teksten (eller starten av den) allerede er lest.
    /// Slike presentasjoner vises ikke i treet eller i søket.
    pub fn is_presentation_content(&self, path: &str, content: &str, suffix: &str) -> bool {
        let paths = presentation_paths_for(path, content, suffix);
        paths.from_presentation && paths.manuscript != path && self.root.join(&paths.manuscript).is_file()
    }

    fn children(&self, dir: &Path, prefix: &str, suffix: &str) -> Vec<Tree> {
        let Ok(entries) = std::fs::read_dir(dir) else { return vec![] };
        let mut children: Vec<Tree> = entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let file_name = entry.file_name().to_string_lossy().into_owned();
                if hidden(&file_name) {
                    return None;
                }
                let path = join(prefix, &file_name);
                let file_type = entry.file_type().ok()?;
                let is_dir = file_type.is_dir() || (file_type.is_symlink() && entry.path().is_dir());
                Some(if is_dir {
                    Tree { name: file_name, children: self.children(&entry.path(), &path, suffix), path, kind: Kind::Folder }
                } else if let Some(name) = file_name.strip_suffix(".md") {
                    if self.is_presentation(&path, suffix) {
                        return None;
                    }
                    Tree { name: name.to_string(), path, kind: Kind::Note, children: vec![] }
                } else {
                    Tree { name: file_name, path, kind: Kind::File, children: vec![] }
                })
            })
            .collect();
        children.sort_by(|a, b| match (a.kind == Kind::Folder, b.kind == Kind::Folder) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()).then_with(|| a.name.cmp(&b.name)),
        });
        children
    }

    pub fn read_note(&self, path: &str) -> Result<String, String> {
        std::fs::read_to_string(self.resolve(path)?).map_err(|e| format!("Kunne ikke lese «{path}»: {e}"))
    }

    pub fn write_note(&self, path: &str, content: &str) -> Result<(), String> {
        let full = self.resolve(path)?;
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Kunne ikke lage mappa til «{path}»: {e}"))?;
        }
        write_atomic(&full, content.as_bytes()).map_err(|e| format!("Kunne ikke lagre «{path}»: {e}"))
    }

    pub fn exists(&self, path: &str) -> bool {
        self.resolve(path).is_ok_and(|p| p.exists())
    }

    pub fn is_folder(&self, path: &str) -> bool {
        self.resolve(path).is_ok_and(|p| p.is_dir())
    }

    /// Et ledig navn i mappa: `<navn><endelse>`, `<navn> 1<endelse>` osv.
    fn free_path(&self, folder: &str, name: &str, extension: &str) -> Result<String, String> {
        for n in 0.. {
            let candidate = if n == 0 { format!("{name}{extension}") } else { format!("{name} {n}{extension}") };
            let path = join(folder, &candidate);
            if !self.resolve(&path)?.exists() {
                return Ok(path);
            }
        }
        unreachable!()
    }

    /// Lager `Uten navn.md` (eller `Uten navn 1.md` osv.) og gir stien.
    pub fn create_note(&self, folder: &str) -> Result<String, String> {
        let path = self.free_path(folder, "Uten navn", ".md")?;
        self.write_note(&path, "")?;
        Ok(path)
    }

    pub fn create_folder(&self, parent: &str, name: &str) -> Result<String, String> {
        let name = valid_name(name)?;
        let path = if name.is_empty() { self.free_path(parent, "Ny mappe", "")? } else { join(parent, name) };
        let full = self.resolve(&path)?;
        if full.exists() {
            return Err(format!("«{path}» finnes allerede."));
        }
        std::fs::create_dir_all(&full).map_err(|e| format!("Kunne ikke lage mappa «{path}»: {e}"))?;
        Ok(path)
    }

    /// Gir fila eller mappa et nytt navn i samme mappe. Manus beholder `.md`.
    pub fn rename(&self, path: &str, new_name: &str) -> Result<String, String> {
        let new_name = valid_name(new_name)?;
        if new_name.is_empty() {
            return Err("Navnet kan ikke være tomt.".to_string());
        }
        let from = self.resolve(path)?;
        let is_note = path.ends_with(".md") && from.is_file();
        let file_name = if is_note && !new_name.ends_with(".md") { format!("{new_name}.md") } else { new_name.to_string() };
        let new_path = join(parent_folder(path), &file_name);
        if new_path == path {
            return Ok(new_path);
        }
        let to = self.resolve(&new_path)?;
        // På macOS og Windows er "a.md" og "A.md" samme fil, så bare store/små bokstaver kan endres.
        if to.exists() && new_path.to_lowercase() != path.to_lowercase() {
            return Err(format!("«{new_path}» finnes allerede."));
        }
        std::fs::rename(&from, &to).map_err(|e| format!("Kunne ikke gi nytt navn til «{path}»: {e}"))?;
        Ok(new_path)
    }

    /// Flytter fila eller mappa til `folder` (tom for roten) og gir den nye stien.
    /// Et manus tar med seg presentasjonen sin, som ellers ville dukket opp i treet.
    pub fn move_to(&self, path: &str, folder: &str, suffix: &str) -> Result<String, String> {
        let path = path.trim_matches('/');
        let folder = folder.trim_matches('/');
        if path.is_empty() {
            return Err("Kan ikke flytte hele vaulten.".to_string());
        }
        if folder == path || folder.starts_with(&format!("{path}/")) {
            return Err(format!("Kan ikke flytte «{path}» inn i seg selv."));
        }
        if !self.resolve(folder)?.is_dir() {
            return Err(format!("Fant ikke mappa «{folder}»."));
        }
        let name = path.rsplit('/').next().unwrap_or(path);
        let new_path = join(folder, name);
        if new_path == path {
            return Ok(new_path);
        }
        let from = self.resolve(path)?;
        let to = self.resolve(&new_path)?;
        if to.exists() {
            return Err(format!("«{new_path}» finnes allerede."));
        }
        let presentation = (path.ends_with(".md") && from.is_file()).then(|| {
            let old = presentation_paths(path, suffix).presentation;
            (old.clone(), presentation_paths(&new_path, suffix).presentation)
        });
        std::fs::rename(&from, &to).map_err(|e| format!("Kunne ikke flytte «{path}»: {e}"))?;
        if let Some((old, new)) = presentation {
            let (old, new) = (self.resolve(&old)?, self.resolve(&new)?);
            if old != from && old.is_file() && !new.exists() {
                std::fs::rename(&old, &new).map_err(|e| format!("Kunne ikke flytte presentasjonen: {e}"))?;
            }
        }
        Ok(new_path)
    }

    /// Legger fila eller mappa i papirkurven.
    pub fn delete(&self, path: &str) -> Result<(), String> {
        if path.trim_matches('/').is_empty() {
            return Err("Kan ikke slette hele vaulten.".to_string());
        }
        trash::delete(self.resolve(path)?).map_err(|e| format!("Kunne ikke slette «{path}»: {e}"))
    }

    /// Alle filer i vaulten etter filnavn (uten skjulte mapper), sortert, så
    /// den første med et navn alltid er den samme.
    fn files_by_name(&self) -> HashMap<String, PathBuf> {
        let mut index = HashMap::new();
        let mut stack = vec![self.root.clone()];
        let mut files = Vec::new();
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            for entry in entries.filter_map(Result::ok) {
                if hidden(&entry.file_name().to_string_lossy()) {
                    continue;
                }
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    files.push(path);
                }
            }
        }
        files.sort();
        for path in files {
            if let Some(name) = path.file_name() {
                index.entry(name.to_string_lossy().to_lowercase()).or_insert(path);
            }
        }
        index
    }

    /// Finner et bilde som Obsidian: først relativt til vault-roten, så til
    /// mappa fila ligger i, og til slutt første fil i vaulten med det
    /// filnavnet. `![](…)` (ikke wikilenke) prøver mappa til fila først.
    pub fn find_file(&self, target: &str, note: &str, wiki: bool, index: &OnceLock<HashMap<String, PathBuf>>) -> Option<PathBuf> {
        let target = percent_decode(target.trim());
        let target = target.trim_start_matches('/');
        if target.is_empty() {
            return None;
        }
        let in_root = join("", target);
        let in_folder = join(parent_folder(note), target);
        let candidates = if wiki { [in_root, in_folder] } else { [in_folder, in_root] };
        for candidate in candidates {
            if let Ok(full) = self.resolve(&candidate) {
                if full.is_file() {
                    return Some(full);
                }
            }
        }
        let name = target.rsplit('/').next()?.to_lowercase();
        index.get_or_init(|| self.files_by_name()).get(&name).cloned()
    }

    /// Finner fila en `[[lenke]]` peker på, som Obsidian: `[[Manus]]` er
    /// `Manus.md`, og `#overskrift` på slutten ignoreres.
    pub fn resolve_link(&self, target: &str, from: &str) -> Option<String> {
        let name = target.split('#').next().unwrap_or("").trim();
        if name.is_empty() {
            return None;
        }
        let index = OnceLock::new();
        let note = format!("{name}.md");
        let found = (!name.ends_with(".md"))
            .then(|| self.find_file(&note, from, true, &index))
            .flatten()
            .or_else(|| self.find_file(name, from, true, &index))?;
        self.relative(&found)
    }

    /// Gir URL-en til bilder i en fil, for [`crate::markdown`].
    pub fn image_resolver<'a>(&'a self, note: &'a str) -> impl Fn(&str, bool) -> Option<String> + 'a {
        let index = OnceLock::new();
        move |target: &str, wiki: bool| self.find_file(target, note, wiki, &index).map(|path| asset_url(&path))
    }
}

/// Skriver til en skjult fil i samme mappe og gir den så navnet til `path`, så
/// fila aldri er halvveis skrevet hvis appen eller maskinen stopper midt i.
/// Den skjulte fila vises ikke i treet og gir ikke `vault-changed`.
pub fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let temp = path.with_file_name(format!(".{name}.{}.tmp", std::process::id()));
    let result = std::fs::write(&temp, data).and_then(|()| std::fs::rename(&temp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

/// Et filnavn uten mappeskille og uten å være skjult.
fn valid_name(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if name.contains('/') || name.contains('\\') || name == "." || name == ".." {
        return Err(format!("Ugyldig navn «{name}»: kan ikke inneholde / eller \\."));
    }
    if hidden(name) {
        return Err(format!("Ugyldig navn «{name}»: kan ikke starte med punktum."));
    }
    Ok(name)
}

/// `%20` og lignende i en URL-del gjort om til tegn (ugyldige `%` beholdes).
pub fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
            if let Some(byte) = hex.and_then(|hex| u8::from_str_radix(hex, 16).ok()) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| text.to_string())
}

/// Som `encodeURIComponent` i JavaScript.
pub fn encode_uri_component(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// URL-en Tauri bruker for filer på disk (asset-protokollen), som `convertFileSrc`.
pub fn asset_url(path: &Path) -> String {
    let path = path.to_string_lossy();
    if cfg!(windows) {
        format!("http://asset.localhost/{}", encode_uri_component(&path))
    } else {
        format!("asset://localhost/{}", encode_uri_component(&path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_vault(name: &str) -> (PathBuf, Vault) {
        let dir = std::env::temp_dir().join(format!("kerygma-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("Prekener/bilder")).unwrap();
        std::fs::create_dir_all(dir.join(".obsidian")).unwrap();
        std::fs::write(dir.join("Prekener/Påske.md"), "# Påske").unwrap();
        std::fs::write(dir.join("Prekener/bilder/kart.png"), "png").unwrap();
        std::fs::write(dir.join("b.md"), "").unwrap();
        std::fs::write(dir.join("A.md"), "").unwrap();
        std::fs::write(dir.join("logo.png"), "png").unwrap();
        let vault = Vault::open(&dir).unwrap();
        (dir, vault)
    }

    #[test]
    fn treet_har_mapper_forst_og_skjuler_punktum() {
        let (dir, vault) = temp_vault("tre");
        let tree = vault.tree(" - presentasjon");
        let names: Vec<(&str, Kind)> = tree.children.iter().map(|c| (c.name.as_str(), c.kind)).collect();
        assert_eq!(names, [("Prekener", Kind::Folder), ("A", Kind::Note), ("b", Kind::Note), ("logo.png", Kind::File)]);
        assert_eq!(tree.children[0].children[1].path, "Prekener/Påske.md");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn presentasjoner_med_manus_vises_ikke_i_treet() {
        let (dir, vault) = temp_vault("presentasjoner");
        let write = |name: &str, text: &str| std::fs::write(dir.join("Prekener").join(name), text).unwrap();
        // Navnet sier at det er en presentasjon, og manuset finnes.
        write("Påske - presentasjon.md", "# Påske");
        // Markøren peker på manuset, selv om navnet er et annet.
        write("Lysbilder.md", "---\ntags: [påske]\n---\n\n<!-- kerygma-presentasjon: Påske.md -->\n\n# Påske");
        // Manuset finnes ikke: vises, så fila ikke blir borte.
        write("Jul - presentasjon.md", "# Jul");
        write("Gammel.md", "<!-- kerygma-presentasjon: Slettet.md -->");
        let tree = vault.tree(" - presentasjon");
        let names: Vec<&str> = tree.children[0].children.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["bilder", "Gammel", "Jul - presentasjon", "Påske"]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn stier_utenfor_vaulten_avvises() {
        let (dir, vault) = temp_vault("stier");
        assert!(vault.resolve("../hemmelig.md").is_err());
        assert!(vault.resolve("/etc/passwd").is_err());
        assert!(vault.resolve("Prekener/Påske.md").is_ok());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn ny_note_navn_og_bilder() {
        let (dir, vault) = temp_vault("ny");
        assert_eq!(vault.create_note("Prekener").unwrap(), "Prekener/Uten navn.md");
        assert_eq!(vault.create_note("Prekener").unwrap(), "Prekener/Uten navn 1.md");
        assert_eq!(vault.rename("Prekener/Uten navn.md", "Jul").unwrap(), "Prekener/Jul.md");
        assert!(vault.rename("Prekener/Jul.md", "Påske").is_err());

        let index = OnceLock::new();
        let found = |target: &str, wiki| vault.find_file(target, "Prekener/Påske.md", wiki, &index);
        assert_eq!(found("logo.png", true), Some(vault.root().join("logo.png")));
        assert_eq!(found("bilder/kart.png", true), Some(vault.root().join("Prekener/bilder/kart.png")));
        assert_eq!(found("kart.png", true), Some(vault.root().join("Prekener/bilder/kart.png")));
        assert_eq!(found("finnes.png", true), None);
        assert_eq!(vault.resolve_link("Påske#Innledning", "b.md").as_deref(), Some("Prekener/Påske.md"));
        assert_eq!(vault.resolve_link("A", "Prekener/Påske.md").as_deref(), Some("A.md"));
        assert_eq!(encode_uri_component("/a b/ø.png"), "%2Fa%20b%2F%C3%B8.png");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn flytte_filer_og_mapper() {
        let (dir, vault) = temp_vault("flytte");
        let suffix = " - presentasjon";
        std::fs::write(dir.join("A - presentasjon.md"), "# A").unwrap();
        assert_eq!(vault.move_to("A.md", "Prekener", suffix).unwrap(), "Prekener/A.md");
        assert!(dir.join("Prekener/A - presentasjon.md").is_file());
        assert!(!dir.join("A - presentasjon.md").exists());

        assert_eq!(vault.move_to("Prekener/bilder", "", suffix).unwrap(), "bilder");
        assert!(dir.join("bilder/kart.png").is_file());
        assert_eq!(vault.move_to("logo.png", "", suffix).unwrap(), "logo.png");

        assert!(vault.move_to("Prekener", "Prekener", suffix).is_err());
        std::fs::create_dir_all(dir.join("Prekener/Under")).unwrap();
        assert!(vault.move_to("Prekener", "Prekener/Under", suffix).is_err());
        std::fs::write(dir.join("Påske.md"), "").unwrap();
        assert!(vault.move_to("Påske.md", "Prekener", suffix).is_err());
        assert!(vault.move_to("b.md", "Finnes ikke", suffix).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
