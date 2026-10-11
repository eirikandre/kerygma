//! Søk i alle manusene i vaulten, på filnavn og innhold.
//!
//! Søketeksten deles i ord. En fil treffer når hvert ord finnes i navnet
//! eller i teksten (uten hensyn til store og små bokstaver). Filer der alle
//! ordene står i navnet, kommer først, med det korteste navnet øverst. Alt sammenlignes i NFC, siden filnavn
//! fra Finder kan ha «å» som «a» + ring (NFD).

use std::cmp::Ordering;

use serde::Serialize;
use unicode_normalization::UnicodeNormalization;

use crate::path_name;
use crate::vault::Vault;

/// Høyst så mange filer i svaret.
pub const MAX_RESULTS: usize = 100;
/// Høyst så mange linjer per fil i svaret (`total` sier hvor mange det er).
pub const MAX_LINES: usize = 5;
/// Lengre linjer kortes ned rundt det første treffet.
const SNIPPET_CHARS: usize = 160;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    /// Relativt til vault-roten, med `/`.
    pub path: String,
    /// Filnavnet uten `.md`.
    pub name: String,
    /// Alle ordene står i navnet.
    pub name_match: bool,
    /// De første linjene med treff.
    pub lines: Vec<LineMatch>,
    /// Antall linjer med treff i hele fila.
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LineMatch {
    /// Linjenummeret, fra 1.
    pub line: usize,
    /// Linja, kortet ned rundt treffet hvis den er lang.
    pub text: String,
}

/// Ordene i søketeksten, med små bokstaver.
pub fn terms(query: &str) -> Vec<String> {
    query.nfc().collect::<String>().split_whitespace().map(str::to_lowercase).collect()
}

/// Søker i alle manusene i vaulten. Presentasjoner med manuset ved siden av hoppes
/// over, som i treet: treffene står i manuset.
pub fn search(vault: &Vault, query: &str, suffix: &str) -> Vec<SearchResult> {
    let terms = terms(query);
    if terms.is_empty() {
        return vec![];
    }
    let mut results: Vec<SearchResult> = vault
        .notes()
        .into_iter()
        .filter_map(|path| {
            let content = vault.read_note(&path).ok()?;
            if vault.is_presentation_content(&path, &content, suffix) {
                return None;
            }
            search_note(&path, &content, &terms)
        })
        .collect();
    // Treff i navnet først, og blant dem det korteste navnet ("Påske" før "Påske 2").
    // Ellers flest linjer med treff først.
    results.sort_by(|a, b| {
        b.name_match.cmp(&a.name_match).then_with(|| {
            let by_name = if a.name_match { a.name.chars().count().cmp(&b.name.chars().count()) } else { Ordering::Equal };
            by_name.then(b.total.cmp(&a.total)).then(a.path.cmp(&b.path))
        })
    });
    results.truncate(MAX_RESULTS);
    results
}

/// Treffet i én fil, eller `None` hvis ikke alle ordene finnes i navn eller tekst.
pub fn search_note(path: &str, content: &str, terms: &[String]) -> Option<SearchResult> {
    let name: String = path_name(path).nfc().collect();
    let content: String = content.nfc().collect();
    let name_lower = name.to_lowercase();
    let content_lower = content.to_lowercase();
    if !terms.iter().all(|term| name_lower.contains(term) || content_lower.contains(term)) {
        return None;
    }
    let mut lines = Vec::new();
    let mut total = 0;
    for (i, line) in content.lines().enumerate() {
        let lower = line.to_lowercase();
        let Some(first) = terms.iter().filter_map(|term| lower.find(term.as_str())).min() else { continue };
        total += 1;
        if lines.len() < MAX_LINES {
            lines.push(LineMatch { line: i + 1, text: snippet(line, &lower, first) });
        }
    }
    Some(SearchResult {
        path: path.to_string(),
        name_match: terms.iter().all(|term| name_lower.contains(term)),
        name,
        lines,
        total,
    })
}

/// Linja uten innrykk, kortet ned til et vindu rundt treffet med «…» der noe er tatt bort.
/// `first` er byte-indeksen til treffet i `lower` (linja med små bokstaver).
fn snippet(line: &str, lower: &str, first: usize) -> String {
    let chars: Vec<char> = line.chars().collect();
    if chars.len() <= SNIPPET_CHARS {
        return line.trim().to_string();
    }
    // Små bokstaver har nesten alltid like mange tegn; klem inn for sikkerhets skyld.
    let at = lower[..first].chars().count().min(chars.len());
    let start = at.saturating_sub(SNIPPET_CHARS / 4);
    let end = (start + SNIPPET_CHARS).min(chars.len());
    let start = end.saturating_sub(SNIPPET_CHARS);
    let text: String = chars[start..end].iter().collect();
    format!("{}{}{}", if start > 0 { "…" } else { "" }, text.trim(), if end < chars.len() { "…" } else { "" })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_vault(name: &str) -> (std::path::PathBuf, Vault) {
        let dir = std::env::temp_dir().join(format!("kerygma-sok-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("Prekener")).unwrap();
        std::fs::create_dir_all(dir.join(".obsidian")).unwrap();
        std::fs::write(dir.join("Prekener/Påske.md"), "# Påske\n\nHan er oppstanden.\nTro og håp.\n").unwrap();
        std::fs::write(dir.join("Prekener/Jul.md"), "# Jul\n\nEt barn er født. Påske kommer senere.\n").unwrap();
        std::fs::write(dir.join("Prekener/Påske - presentasjon.md"), "# Påske\n").unwrap();
        std::fs::write(dir.join("Prekener/Påske 2.md"), "Mer om påske.\n").unwrap();
        // Manuset finnes ikke, så presentasjonen kommer med.
        std::fs::write(dir.join("Prekener/Pinse - presentasjon.md"), "# Pinse og påske\n").unwrap();
        std::fs::write(dir.join("Notat.md"), "Ingenting her.\n").unwrap();
        std::fs::write(dir.join(".obsidian/påske.md"), "påske").unwrap();
        std::fs::write(dir.join("påske.txt"), "påske").unwrap();
        (dir.clone(), Vault::open(&dir).unwrap())
    }

    #[test]
    fn treff_i_navnet_kommer_forst_og_skjulte_filer_hoppes_over() {
        let (dir, vault) = temp_vault("navn");
        let results = search(&vault, "PÅSKE", " - presentasjon");
        let paths: Vec<&str> = results.iter().map(|r| r.path.as_str()).collect();
        assert_eq!(
            paths,
            ["Prekener/Påske.md", "Prekener/Påske 2.md", "Prekener/Jul.md", "Prekener/Pinse - presentasjon.md"]
        );
        assert!(results[0].name_match);
        assert_eq!(results[0].lines, [LineMatch { line: 1, text: "# Påske".into() }]);
        assert_eq!(results[2].lines, [LineMatch { line: 3, text: "Et barn er født. Påske kommer senere.".into() }]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn alle_ordene_ma_finnes_i_navn_eller_tekst() {
        let (dir, vault) = temp_vault("ord");
        let results = search(&vault, "påske håp", " - presentasjon");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "Prekener/Påske.md");
        assert!(!results[0].name_match);
        assert_eq!(results[0].total, 2);
        assert!(search(&vault, "   ", " - presentasjon").is_empty());
        assert!(search(&vault, "finnesikke", " - presentasjon").is_empty());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn lange_linjer_kortes_ned_rundt_treffet() {
        let line = format!("{} nål {}", "a".repeat(300), "b".repeat(300));
        let result = search_note("x.md", &line, &terms("NÅL")).unwrap();
        let text = &result.lines[0].text;
        assert!(text.starts_with('…') && text.ends_with('…'), "{text}");
        assert!(text.contains("nål"));
        assert!(text.chars().count() <= SNIPPET_CHARS + 2);
    }

    #[test]
    fn filnavn_i_nfd_treffer_sok_i_nfc() {
        let nfd = "Pa\u{30a}ske.md";
        let result = search_note(nfd, "", &terms("påske")).unwrap();
        assert!(result.name_match);
        assert_eq!(result.name, "Påske");
    }

    #[test]
    fn hoyst_fem_linjer_per_fil() {
        let content = "tro\n".repeat(8);
        let result = search_note("x.md", &content, &terms("tro")).unwrap();
        assert_eq!(result.lines.len(), MAX_LINES);
        assert_eq!(result.total, 8);
        assert_eq!(result.lines[4].line, 5);
    }
}
