//! Corpus provenance for reproducible analysis reports.
//!
//! A provenance record names the resolved input, hashes the exact bytes that
//! were read, and records the parser rule and normalization that produced the
//! n-gram counts.  Reporting is observational: it never changes a score.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::config::Config;
use crate::corpus::{self, Corpus};
use crate::hash::sha1_hex;

#[derive(Clone, Debug)]
pub struct FileDigest {
    pub name: String,
    pub bytes: u64,
    pub sha1: String,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Totals {
    pub letters: f64,
    pub bigrams: f64,
    pub trigrams: f64,
    pub letter_entries: usize,
    pub bigram_entries: usize,
    pub trigram_entries: usize,
}

#[derive(Clone, Debug)]
pub struct Provenance {
    pub label: String,
    pub weight: f64,
    pub kind: &'static str,
    pub path: PathBuf,
    pub combined_sha1: String,
    pub parser: &'static str,
    pub parser_rule: &'static str,
    pub normalization: &'static str,
    pub files: Vec<FileDigest>,
    pub totals: Totals,
}

impl Provenance {
    pub fn to_json(&self) -> Value {
        json!({
            "label": self.label,
            "weight": self.weight,
            "kind": self.kind,
            "path": self.path.to_string_lossy(),
            "combined_sha1": self.combined_sha1,
            "parser": self.parser,
            "parser_rule": self.parser_rule,
            "normalization": self.normalization,
            "files": self.files.iter().map(|file| json!({
                "name": file.name,
                "bytes": file.bytes,
                "sha1": file.sha1,
            })).collect::<Vec<_>>(),
            "totals": {
                "letters": self.totals.letters,
                "bigrams": self.totals.bigrams,
                "trigrams": self.totals.trigrams,
                "letter_entries": self.totals.letter_entries,
                "bigram_entries": self.totals.bigram_entries,
                "trigram_entries": self.totals.trigram_entries,
            },
        })
    }
}

fn digest(path: &Path) -> Result<(FileDigest, Vec<u8>)> {
    let data =
        std::fs::read(path).map_err(|e| anyhow!("failed to read {}: {e}", path.display()))?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("?")
        .to_string();
    let digest = FileDigest {
        name,
        bytes: data.len() as u64,
        sha1: sha1_hex(&data),
    };
    Ok((digest, data))
}

fn combined_sha1(files: &[FileDigest]) -> String {
    let joined = files
        .iter()
        .map(|file| file.sha1.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    sha1_hex(joined.as_bytes())
}

fn map_total(data: &[u8], width: usize) -> Result<(f64, usize)> {
    let map: HashMap<String, f64> = serde_json::from_slice(data)?;
    let mut total = 0.0;
    let mut entries = 0;
    for (key, value) in map {
        if key.chars().count() == width {
            total += value;
            entries += 1;
        }
    }
    Ok((total, entries))
}

fn parsed_totals(files: &[Vec<u8>]) -> Result<Totals> {
    let (letters, letter_entries) = map_total(&files[0], 1)?;
    let (bigrams, bigram_entries) = map_total(&files[1], 2)?;
    let (trigrams, trigram_entries) = map_total(&files[2], 3)?;
    Ok(Totals {
        letters,
        bigrams,
        trigrams,
        letter_entries,
        bigram_entries,
        trigram_entries,
    })
}

fn corpus_totals(corpus: &Corpus) -> Totals {
    Totals {
        letters: corpus.total_letters,
        bigrams: corpus.total_bigrams,
        trigrams: corpus.total_trigrams,
        letter_entries: corpus.letters.len(),
        bigram_entries: corpus.bigrams.len(),
        trigram_entries: corpus.trigrams.len(),
    }
}

#[derive(Deserialize)]
struct ExternalCorpus {
    letters: HashMap<String, f64>,
    bigrams: HashMap<String, f64>,
    trigrams: HashMap<String, f64>,
}

fn external_totals(data: &[u8]) -> Result<Totals> {
    let payload: ExternalCorpus = serde_json::from_slice(data)?;
    let letters: f64 = payload.letters.values().sum();
    let bigrams: f64 = payload.bigrams.values().sum();
    let trigrams: f64 = payload.trigrams.values().sum();
    Ok(Totals {
        letters,
        bigrams,
        trigrams,
        letter_entries: payload.letters.len(),
        bigram_entries: payload.bigrams.len(),
        trigram_entries: payload.trigrams.len(),
    })
}

/// Describe one resolved corpus input.
pub fn describe(label: &str, weight: f64, path: &Path) -> Result<Provenance> {
    let resolved = corpus::expand_user(path);
    let (kind, parser, parser_rule, normalization, paths, totals) = if let Some(dir) =
        corpus::parsed_dir(&resolved)
    {
        let paths: Vec<PathBuf> = corpus::PARSED_FILES.iter().map(|n| dir.join(n)).collect();
        let mut files = Vec::new();
        let mut data = Vec::new();
        for path in &paths {
            let (file, bytes) = digest(path)?;
            files.push(file);
            data.push(bytes);
        }
        let totals = parsed_totals(&data)?;
        (
            "parsed",
            "parsed-json-v1",
            "read letter/bigram/trigram JSON maps; keep single-char, 2-char, and 3-char keys",
            "none (pre-parsed counts)",
            files,
            totals,
        )
    } else if resolved.is_file()
        && resolved
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("json"))
            .unwrap_or(false)
    {
        let (file, data) = digest(&resolved)?;
        let totals = external_totals(&data)?;
        (
            "external-json",
            "external-ngram-json-v1",
            "require top-level letters/bigrams/trigrams maps of string counts",
            "none (external counts)",
            vec![file],
            totals,
        )
    } else if let Some(source) = corpus::source_file(&resolved) {
        let (file, _) = digest(&source)?;
        let corpus = Corpus::load(&resolved)?;
        (
            "raw",
            "raw-text-v1",
            "maximal [a-z]+ runs after normalization; non-letters break runs",
            "lowercase, Unicode NFKD, strip combining marks",
            vec![file],
            corpus_totals(&corpus),
        )
    } else {
        return Err(anyhow!(
                "cannot describe corpus {}: expected a parsed directory, parsed file, raw .txt/.xz/.zip source, or external .json bundle",
                resolved.display()
            ));
    };
    let combined = combined_sha1(&paths);
    Ok(Provenance {
        label: label.to_string(),
        weight,
        kind,
        path: resolved,
        combined_sha1: combined,
        parser,
        parser_rule,
        normalization,
        files: paths,
        totals,
    })
}

/// Describe a comparison-only external n-gram bundle.
pub fn describe_external(label: &str, weight: f64, path: &Path) -> Result<Provenance> {
    let resolved = corpus::expand_user(path);
    if !resolved.is_file() {
        return Err(anyhow!(
            "external corpus {} is not a file",
            resolved.display()
        ));
    }
    let (file, data) = digest(&resolved)?;
    let totals = external_totals(&data)?;
    let combined = combined_sha1(std::slice::from_ref(&file));
    Ok(Provenance {
        label: label.to_string(),
        weight,
        kind: "external-json",
        path: resolved,
        combined_sha1: combined,
        parser: "external-ngram-json-v1",
        parser_rule: "require top-level letters/bigrams/trigrams maps of string counts",
        normalization: "none (external counts)",
        files: vec![file],
        totals,
    })
}

/// Describe every corpus configured for a role.
pub fn describe_entries(cfg: &Config, role: &str) -> Result<Vec<Provenance>> {
    let entries = if role == "primary" {
        &cfg.primary_corpus
    } else {
        &cfg.secondary_corpus
    };
    entries
        .iter()
        .map(|entry| {
            let path = cfg.corpus_path(entry);
            describe(&entry.label(), entry.weight, &path)
        })
        .collect()
}

/// Describe corpora supplied directly on the command line.
pub fn describe_paths(paths: &[String]) -> Result<Vec<Provenance>> {
    paths
        .iter()
        .map(|path| {
            let expanded = corpus::expand_user(Path::new(path));
            describe(path, 1.0, &expanded)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "shirolyzer-provenance-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn parsed_provenance_is_stable() {
        let dir = temp("parsed");
        fs::write(dir.join("letter_freq.json"), r#"{"a": 2.0, "b": 1.0}"#).unwrap();
        fs::write(dir.join("bigram_freq.json"), r#"{"ab": 1.5, "x": 9.0}"#).unwrap();
        fs::write(dir.join("trigram_freq.json"), r#"{"abc": 0.5, "ab": 9.0}"#).unwrap();
        let first = describe("fixture", 100.0, &dir).unwrap();
        assert_eq!(first.kind, "parsed");
        assert_eq!(first.parser, "parsed-json-v1");
        assert_eq!(first.normalization, "none (pre-parsed counts)");
        assert_eq!(first.files.len(), 3);
        assert_eq!(first.totals.letters, 3.0);
        assert_eq!(first.totals.letter_entries, 2);
        assert_eq!(first.totals.bigrams, 1.5);
        assert_eq!(first.totals.bigram_entries, 1);
        assert_eq!(first.totals.trigrams, 0.5);
        assert_eq!(first.totals.trigram_entries, 1);
        assert_eq!(
            first.combined_sha1,
            "51db3973e2e3138855ae7c6ae1c5f400661f6604"
        );
        let second = describe("fixture", 100.0, &dir).unwrap();
        assert_eq!(first.combined_sha1, second.combined_sha1);
    }

    #[test]
    fn external_bundle_is_recognized() {
        let dir = temp("external");
        let path = dir.join("ngrams.json");
        fs::write(
            &path,
            r#"{"letters": {"a": 1.0}, "bigrams": {"ab": 2.0}, "trigrams": {"abc": 3.0}}"#,
        )
        .unwrap();
        let prov = describe("external", 1.0, &path).unwrap();
        assert_eq!(prov.kind, "external-json");
        assert_eq!(prov.parser, "external-ngram-json-v1");
        assert_eq!(prov.totals.letters, 1.0);
        assert_eq!(prov.totals.bigrams, 2.0);
        assert_eq!(prov.totals.trigrams, 3.0);
        assert_eq!(prov.totals.trigram_entries, 1);
    }
}
