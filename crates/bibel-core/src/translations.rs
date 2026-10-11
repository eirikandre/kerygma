//! Oversettelsene et oppslag kan velge mellom.
//!
//! Kerygma leser dem fra disk ved kjøring. Hver fil bærer sin egen id og tittel, så det finnes
//! ingen sidestilt tabell som kan komme ut av synk med dataene.

use bibel_format::{Bible, FormatError};

/// Et kombinasjonsnavn og oversettelsene det peker på, som «ne» → nb30 + kjv.
pub type Alias = (String, Vec<String>);

pub struct Registry {
    bibles: Vec<Bible<'static>>,
    default: usize,
    aliases: Vec<Alias>,
}

impl Registry {
    /// Tolker hver fil. Standard er den første til [`Registry::set_default`] sier noe annet.
    pub fn parse(files: &[&'static [u8]]) -> Result<Self, FormatError> {
        let mut bibles = Vec::with_capacity(files.len());
        for data in files {
            bibles.push(Bible::parse(data)?);
        }
        Ok(Registry { bibles, default: 0, aliases: Vec::new() })
    }

    pub fn is_empty(&self) -> bool {
        self.bibles.is_empty()
    }

    pub fn set_default(&mut self, id: &str) -> Result<(), String> {
        match self.bibles.iter().position(|b| b.id().eq_ignore_ascii_case(id)) {
            Some(i) => {
                self.default = i;
                Ok(())
            }
            None => Err(format!("ukjent oversettelse «{id}» - har {}", self.ids().join(", "))),
        }
    }

    /// Setter kombinasjonsnavnene, skrevet som `BIBEL_ALIAS`: «n=nb30,e=kjv,ne=nb30+kjv».
    ///
    /// Kontrolleres på samme måte som build.rs gjør for kjørbaren: et navn kan
    /// ikke skygge for en oversettelse, og må peke på oversettelser som finnes.
    pub fn set_aliases(&mut self, spec: &str) -> Result<(), String> {
        let mut aliases = Vec::new();
        for entry in spec.split(',').map(str::trim).filter(|e| !e.is_empty()) {
            let Some((name, targets)) = entry.split_once('=') else {
                return Err(format!("«{entry}» mangler = (ventet navn=id1+id2)"));
            };
            let name = name.trim();
            let targets: Vec<String> =
                targets.split('+').map(str::trim).filter(|t| !t.is_empty()).map(String::from).collect();
            if name.is_empty() || targets.is_empty() {
                return Err(format!("«{entry}» er tomt på den ene siden av ="));
            }
            if self.by_id(name).is_some() {
                return Err(format!("«{name}» kolliderer med en oversettelse som heter det samme"));
            }
            if let Some(missing) = targets.iter().find(|t| self.by_id(t).is_none()) {
                return Err(format!(
                    "«{name}» peker på «{missing}», som ikke finnes (har {})",
                    self.ids().join(", ")
                ));
            }
            aliases.push((name.to_string(), targets));
        }
        self.aliases = aliases;
        Ok(())
    }

    pub fn default_bible(&self) -> &Bible<'static> {
        &self.bibles[self.default]
    }

    pub fn by_id(&self, id: &str) -> Option<&Bible<'static>> {
        self.bibles
            .iter()
            .find(|b| b.id().eq_ignore_ascii_case(id))
    }

    pub fn iter(&self) -> impl Iterator<Item = &Bible<'static>> {
        self.bibles.iter()
    }

    pub fn ids(&self) -> Vec<&'static str> {
        self.bibles.iter().map(|b| b.id()).collect()
    }

    /// Kombinasjonsnavn satt med [`Registry::set_aliases`].
    pub fn aliases(&self) -> &[Alias] {
        &self.aliases
    }

    fn alias(&self, name: &str) -> Option<&[String]> {
        self.aliases
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, targets)| targets.as_slice())
    }

    /// Tolker «nb30», «nb30+kjv» eller et aliasnavn til oversettelsene det peker på.
    ///
    /// Aliaser utvides ett nivå - de kan ikke peke på andre aliaser - slik at en
    /// skrivefeil i et alias ikke kan gi en uendelig løkke.
    pub fn select(&self, spec: &str) -> Result<Vec<&Bible<'static>>, String> {
        let spec = spec.trim();
        if spec.is_empty() {
            return Ok(vec![self.default_bible()]);
        }
        let mut out: Vec<&Bible<'static>> = Vec::new();
        for part in spec.split('+').map(str::trim).filter(|p| !p.is_empty()) {
            if let Some(targets) = self.alias(part) {
                for target in targets {
                    match self.by_id(target) {
                        Some(b) => out.push(b),
                        None => {
                            return Err(format!("aliaset «{part}» peker på ukjent «{target}»"))
                        }
                    }
                }
            } else {
                match self.by_id(part) {
                    Some(b) => out.push(b),
                    None => return Err(format!("ukjent oversettelse «{part}»")),
                }
            }
        }
        if out.is_empty() {
            return Err(format!("forstod ikke «{spec}»"));
        }
        Ok(out)
    }

    /// Deler «kjv:john3:16» eller «bk:john3:16» i oversettelse(r) og referanse.
    /// Uten et kjent prefiks blir hele strengen stående som referanse - det er
    /// det som gjør at «1sam2:1-3» ikke blir misforstått.
    pub fn split_prefix<'s>(&self, input: &'s str) -> (Option<Vec<&Bible<'static>>>, &'s str) {
        if let Some((head, rest)) = input.split_once(':') {
            if !rest.is_empty() {
                if let Ok(bibles) = self.select(head) {
                    return (Some(bibles), rest);
                }
            }
        }
        (None, input)
    }
}

/// Oversettelsene i `bibler/` i roten av repoet, med de samme snarveiene som
/// standardinnstillingene i Kerygma. Tester som trenger en bestemt oversettelse
/// (som nb30), hopper over seg selv uten den.
#[cfg(test)]
pub(crate) fn test_registry() -> Option<Registry> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bibler");
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "bin"))
        .collect();
    paths.sort();
    let files: Vec<&'static [u8]> = paths
        .iter()
        .map(|p| &*Box::leak(std::fs::read(p).expect("les assets").into_boxed_slice()))
        .collect();
    let mut registry = Registry::parse(&files).expect("gyldige datasett");
    if registry.is_empty() {
        return None;
    }
    let _ = registry.set_default("nb30");
    let ids = registry.ids();
    let wanted = ["n=nb30", "e=kjv", "ne=nb30+kjv"];
    let spec: Vec<&str> = wanted
        .iter()
        .copied()
        .filter(|a| a[a.find('=').unwrap() + 1..].split('+').all(|t| ids.contains(&t)))
        .collect();
    registry.set_aliases(&spec.join(",")).expect("aliaser");
    Some(registry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliaser_kontrolleres() {
        let Some(mut r) = test_registry() else { return };
        let id = r.ids()[0];
        assert!(r.set_aliases(&format!("{id}={id}")).is_err(), "skygger for en oversettelse");
        assert!(r.set_aliases("x=finnesikke").is_err());
        assert!(r.set_aliases("x").is_err());
        assert!(r.set_aliases("=nb30").is_err());
        r.set_aliases(&format!("x={id}")).unwrap();
        assert_eq!(r.select("x").unwrap()[0].id(), id);
        r.set_aliases("").unwrap();
        assert!(r.aliases().is_empty());
        assert!(r.set_default("finnesikke").is_err());
    }
}
