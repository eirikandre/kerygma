//! `bibel-core` med oversettelsene (`bibler/*.bin`) lest fra disk.
//!
//! `bibel-core` brukes gjennom `run`, med de samme argumentene og svarene som
//! kjørbaren `bibel`, så oppslag og søk oppfører seg som på kommandolinjen.

use std::collections::HashMap;
use std::path::Path;

use bibel_core::Registry;
pub use bibel_core::Output;

use crate::reference::to_lookup;

/// Standardoversettelse og snarveier, som `BIBEL_DEFAULT` og `BIBEL_ALIAS` når `bibel` bygges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BibelConfig {
    pub default_translation: String,
    pub aliases: String,
}

pub struct Bibel {
    /// Dataene lever like lenge som programmet, så `Registry` kan låne fra dem.
    files: Vec<&'static [u8]>,
    registry: Registry,
    config: Option<BibelConfig>,
}

impl Bibel {
    /// Leser `*.bin` i mappene, i rekkefølge. Finnes samme oversettelse (id) i flere mapper,
    /// brukes den fra den siste, så oversettelser brukeren har lagt inn selv, går foran dem
    /// appen kom med. Mapper som ikke finnes, hoppes over.
    pub fn load_dirs(dirs: &[&Path]) -> Result<Bibel, String> {
        let mut by_id: Vec<(String, String, Vec<u8>)> = Vec::new();
        for dir in dirs {
            let Ok(entries) = std::fs::read_dir(dir) else { continue };
            let mut paths: Vec<_> = entries
                .filter_map(|entry| entry.ok().map(|e| e.path()))
                .filter(|path| path.extension().is_some_and(|ext| ext == "bin"))
                .collect();
            paths.sort();
            for path in paths {
                let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
                let data = std::fs::read(&path).map_err(|e| format!("{name}: {e}"))?;
                let id = bibel_format::Bible::parse(&data)
                    .map_err(|e| format!("{name}: ugyldig datasett: {e}"))?
                    .id()
                    .to_lowercase();
                by_id.retain(|(other, _, _)| *other != id);
                by_id.push((id, name, data));
            }
        }
        by_id.sort_by(|a, b| a.0.cmp(&b.0));
        Bibel::from_files(by_id.into_iter().map(|(_, name, data)| (name, data)).collect())
    }

    /// Leser alle `*.bin` i mappa, sortert etter navn.
    pub fn load_dir(dir: &Path) -> Result<Bibel, String> {
        let mut paths: Vec<_> = std::fs::read_dir(dir)
            .map_err(|e| format!("fant ingen oversettelser i {}: {e}", dir.display()))?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "bin"))
            .collect();
        paths.sort();
        let mut files = Vec::with_capacity(paths.len());
        for path in paths {
            let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
            let data = std::fs::read(&path).map_err(|e| format!("{name}: {e}"))?;
            files.push((name, data));
        }
        Bibel::from_files(files)
    }

    /// Starter `bibel` med oversettelsene (innholdet i `*.bin`, med filnavnet til feilmeldinger).
    pub fn from_files(files: Vec<(String, Vec<u8>)>) -> Result<Bibel, String> {
        if files.is_empty() {
            return Err("fant ingen oversettelser (*.bin)".to_string());
        }
        let mut leaked: Vec<&'static [u8]> = Vec::with_capacity(files.len());
        for (name, data) in files {
            let data: &'static [u8] = Box::leak(data.into_boxed_slice());
            Registry::parse(&[data]).map_err(|e| format!("{name}: ugyldig datasett: {e}"))?;
            leaked.push(data);
        }
        let registry = Registry::parse(&leaked).map_err(|e| format!("ugyldig datasett: {e}"))?;
        // Boknavnene gjenkjennes i manus også når oversettelsen ikke fantes da appen ble bygget.
        crate::reference::add_book_names(
            registry.iter().flat_map(|bible| bible.books().flat_map(|book| [(book.name, book.osis_id), (book.abbrev, book.osis_id)])),
        );
        Ok(Bibel { files: leaked, registry, config: None })
    }

    /// Bruker innstillingene, og gir meldingene fra [`Bibel::configured`]. Er de ugyldige,
    /// beholdes de gamle.
    pub fn configure(&mut self, config: &BibelConfig) -> Result<Vec<String>, String> {
        if self.config.as_ref() == Some(config) {
            return Ok(Vec::new());
        }
        let (bibel, notes) = self.configured(config)?;
        *self = bibel;
        Ok(notes)
    }

    /// En ny `Bibel` med de samme oversettelsene (dataene deles) og innstillingene.
    /// Appen bytter ut hele `Bibel`-en, så oppslag som pågår, bruker de gamle innstillingene.
    ///
    /// Oversettelser som ikke er installert, er ikke en feil: innstillingene for en vault kan
    /// vise til oversettelser som bare finnes i noen versjoner av appen (`bibler/` ved bygging).
    /// En standardoversettelse som mangler, gir den første som er installert, og snarveier til
    /// oversettelser som mangler, hoppes over. Meldingene sier hva som er hoppet over.
    /// Ugyldige snarveier (feil skrivemåte) er fortsatt en feil.
    pub fn configured(&self, config: &BibelConfig) -> Result<(Bibel, Vec<String>), String> {
        let mut registry = Registry::parse(&self.files).map_err(|e| format!("ugyldig datasett: {e}"))?;
        let installed = registry.ids();
        let mut notes = Vec::new();
        let default = config.default_translation.trim();
        if !default.is_empty() {
            if installed.contains(&default) {
                registry.set_default(default)?;
            } else {
                let used = registry.default_bible().id();
                notes.push(format!("Standardoversettelsen «{default}» er ikke installert. Bruker «{used}»."));
            }
        }
        let mut aliases = Vec::new();
        for entry in config.aliases.split(',').map(str::trim).filter(|e| !e.is_empty()) {
            let missing: Vec<&str> = entry
                .split_once('=')
                .map(|(_, targets)| targets.split('+').map(str::trim).filter(|t| !installed.contains(t)).collect())
                .unwrap_or_default();
            if missing.is_empty() {
                aliases.push(entry);
            } else {
                notes.push(format!("Snarveien «{entry}» er hoppet over: {} er ikke installert.", missing.join(", ")));
            }
        }
        registry.set_aliases(&aliases.join(","))?;
        Ok((Bibel { files: self.files.clone(), registry, config: Some(config.clone()) }, notes))
    }

    /// Innstillingene som er i bruk, hvis de er satt.
    pub fn config(&self) -> Option<&BibelConfig> {
        self.config.as_ref()
    }

    /// Setter bare snarveiene. Er de ugyldige, beholdes de gamle.
    pub fn set_aliases(&mut self, aliases: &str) -> Result<Vec<String>, String> {
        let default = self.config.as_ref().map(|c| c.default_translation.clone()).unwrap_or_default();
        self.configure(&BibelConfig { default_translation: default, aliases: aliases.to_string() })
    }

    /// Kjører `bibel` med argumentene, som kommandoen `bibel` i bible-repoet.
    pub fn run<S: AsRef<str>>(&self, args: &[S]) -> Output {
        let args: Vec<String> = args.iter().map(|a| a.as_ref().to_string()).collect();
        bibel_core::run(&self.registry, &args)
    }

    /// Id-ene til oversettelsene, som "kjv".
    pub fn ids(&self) -> Vec<&'static str> {
        self.registry.ids()
    }

    /// Id-en og det fulle navnet til hver oversettelse, som ("nb30", "Bibelen 1930").
    pub fn titles(&self) -> Vec<(&'static str, &'static str)> {
        self.registry.iter().map(|bible| (bible.id(), bible.title())).collect()
    }
}

/// Slår opp referansene i en annen oversettelse, og gir utdataene ("tekst (ref)")
/// per referanse. Referanser som ikke kan slås opp, eller som allerede er i den
/// oversettelsen, utelates.
pub fn look_up_translations(bibel: &Bibel, translation: &str, references: &[String]) -> HashMap<String, String> {
    let mut result = HashMap::new();
    for reference in references {
        if result.contains_key(reference) {
            continue;
        }
        let Some(lookup) = to_lookup(reference) else { continue };
        let output = bibel.run(&["--format", "{tekst} ({ref})", &format!("{translation}:{lookup}")]);
        // Ukjent vers i denne oversettelsen gir bare en feilkode.
        let text = output.stdout.trim();
        if output.code == 0 && !text.ends_with(&format!("({reference})")) {
            result.insert(reference.clone(), text.to_string());
        }
    }
    result
}

impl Bibel {
    /// Finnes oversettelsen (eller snarveien)? Gir meldingen fra `bibel` hvis ikke.
    pub fn check_translation(&self, spec: &str) -> Result<(), String> {
        self.registry.select(spec).map(|_| ())
    }
}
