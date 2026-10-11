//! `bibel` fra bible-repoet, med oversettelsene (`*.bin`) lest fra disk.
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
        Ok(Bibel { files: leaked, registry, config: None })
    }

    /// Bruker innstillingene. Er de ugyldige, beholdes de gamle.
    pub fn configure(&mut self, config: &BibelConfig) -> Result<(), String> {
        if self.config.as_ref() != Some(config) {
            *self = self.configured(config)?;
        }
        Ok(())
    }

    /// En ny `Bibel` med de samme oversettelsene (dataene deles) og innstillingene.
    /// Appen bytter ut hele `Bibel`-en, så oppslag som pågår, bruker de gamle innstillingene.
    pub fn configured(&self, config: &BibelConfig) -> Result<Bibel, String> {
        let mut registry = Registry::parse(&self.files).map_err(|e| format!("ugyldig datasett: {e}"))?;
        let default = config.default_translation.trim();
        if !default.is_empty() {
            registry.set_default(default)?;
        }
        registry.set_aliases(config.aliases.trim())?;
        Ok(Bibel { files: self.files.clone(), registry, config: Some(config.clone()) })
    }

    /// Innstillingene som er i bruk, hvis de er satt.
    pub fn config(&self) -> Option<&BibelConfig> {
        self.config.as_ref()
    }

    /// Setter bare snarveiene. Er de ugyldige, beholdes de gamle.
    pub fn set_aliases(&mut self, aliases: &str) -> Result<(), String> {
        let default = self.config.as_ref().map(|c| c.default_translation.clone()).unwrap_or_default();
        self.configure(&BibelConfig { default_translation: default, aliases: aliases.to_string() })
    }

    /// Kjører `bibel` med argumentene, som kjørbaren i bible-repoet.
    pub fn run<S: AsRef<str>>(&self, args: &[S]) -> Output {
        let args: Vec<String> = args.iter().map(|a| a.as_ref().to_string()).collect();
        // Ingen miljøvariabler: ESPANSO_REF hører til espanso-oppsettet.
        bibel_core::run(&self.registry, &args, |_| None)
    }

    /// Id-ene til oversettelsene, som "kjv".
    pub fn ids(&self) -> Vec<&'static str> {
        self.registry.ids()
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
