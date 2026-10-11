//! Finner den delen av manusteksten som best stemmer med en bibeltekst
//! (lokal sekvenssammenstilling, Smith-Waterman, på ord). Tåler andre
//! oversettelser, utelatte ord og små stavemåteforskjeller.

use std::sync::LazyLock;

use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alignment {
    /// Første og siste ord i manusteksten (indekser i `text`).
    pub start: usize,
    pub end: usize,
    pub score: i32,
    /// Antall ord som stemmer.
    pub matches: usize,
    /// Indeksene i `bible` som ble matchet.
    pub bible_matched: Vec<usize>,
    /// Indeksene i `text` som ble matchet.
    pub text_matched: Vec<usize>,
}

const MATCH: i32 = 2;
const SIMILAR: i32 = 1;
const MISMATCH: i32 = -1;
const GAP: i32 = -1;

pub fn align(text: &[String], bible: &[String]) -> Option<Alignment> {
    let text: Vec<Vec<char>> = text.iter().map(|w| w.chars().collect()).collect();
    let bible: Vec<Vec<char>> = bible.iter().map(|w| w.chars().collect()).collect();
    let rows = text.len() + 1;
    let cols = bible.len() + 1;
    let mut score = vec![0i32; rows * cols];
    let mut best = 0;
    let mut best_at: Option<usize> = None;
    for i in 1..rows {
        for j in 1..cols {
            let s = 0
                .max(score[(i - 1) * cols + j - 1] + similarity(&text[i - 1], &bible[j - 1]))
                .max(score[(i - 1) * cols + j] + GAP)
                .max(score[i * cols + j - 1] + GAP);
            score[i * cols + j] = s;
            if s > best {
                best = s;
                best_at = Some(i * cols + j);
            }
        }
    }
    let best_at = best_at?;

    let mut i = best_at / cols;
    let mut j = best_at % cols;
    let end = i - 1;
    let mut start = end;
    let mut matches = 0;
    let mut bible_matched = Vec::new();
    let mut text_matched = Vec::new();
    while i > 0 && j > 0 && score[i * cols + j] > 0 {
        let s = score[i * cols + j];
        let sim = similarity(&text[i - 1], &bible[j - 1]);
        let diagonal = score[(i - 1) * cols + j - 1] + sim;
        if s == diagonal {
            if sim > 0 {
                matches += 1;
                bible_matched.push(j - 1);
                text_matched.push(i - 1);
                start = i - 1;
            }
            i -= 1;
            j -= 1;
        } else if s == score[(i - 1) * cols + j] + GAP {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    bible_matched.reverse();
    text_matched.reverse();
    Some(Alignment { start, end, score: best, matches, bible_matched, text_matched })
}

fn similarity(a: &[char], b: &[char]) -> i32 {
    if a == b {
        return MATCH;
    }
    if a.len() >= 4 && b.len() >= 4 {
        // "overgav"/"overga", "framgang"/"fremgang", "sten"/"stein".
        if (a.starts_with(b) || b.starts_with(a)) && a.len().abs_diff(b.len()) <= 2 {
            return SIMILAR;
        }
        if edit_distance_is_one(a, b) {
            return SIMILAR;
        }
    }
    MISMATCH
}

fn edit_distance_is_one(a: &[char], b: &[char]) -> bool {
    if a.len().abs_diff(b.len()) > 1 {
        return false;
    }
    let mut i = 0;
    while i < a.len() && i < b.len() && a[i] == b[i] {
        i += 1;
    }
    let tail = |s: &[char], from: usize| s.get(from..).unwrap_or(&[]).to_vec();
    if a.len() == b.len() {
        return tail(a, i + 1) == tail(b, i + 1);
    }
    if a.len() > b.len() {
        tail(a, i + 1) == tail(b, i)
    } else {
        tail(a, i) == tail(b, i + 1)
    }
}

static WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\p{L}\p{N}]+").unwrap());

/// Ordene i en tekst, med små bokstaver.
pub fn words(text: &str) -> Vec<String> {
    WORD.find_iter(&text.to_lowercase()).map(|m| m.as_str().to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finner_sitatet_midt_i_teksten() {
        let text = words("han sa at herren gav juda konge jojakim i hans hånd sånn var det");
        // Daniel 1:2 i Bibelen 1930. «juda» og «Judas» regnes som like.
        let bible = words("Og Herren gav Judas konge Jojakim i hans hånd");
        let a = align(&text, &bible).unwrap();
        assert_eq!((a.start, a.end), (3, 10));
        assert_eq!(a.matches, 8);
    }
}
