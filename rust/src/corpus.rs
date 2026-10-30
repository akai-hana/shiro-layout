//! N-gram corpus loading (parsed JSON or raw text sources).

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

pub const SPACE_INDEX: usize = 26;
const PARSED_FILES: [&str; 3] = [
    "letter_freq.json",
    "bigram_freq.json",
    "trigram_freq.json",
];

#[derive(Clone, Debug, Default)]
pub struct Corpus {
    pub letters: HashMap<char, f64>,
    pub bigrams: HashMap<(char, char), f64>,
    pub trigrams: HashMap<(char, char, char), f64>,
    pub bi_ids: Vec<[usize; 2]>,
    pub bi_counts: Vec<f64>,
    pub tri_ids: Vec<[usize; 3]>,
    pub tri_counts: Vec<f64>,
    pub total_letters: f64,
    pub total_bigrams: f64,
    pub total_trigrams: f64,
}

fn cid(ch: char) -> Option<usize> {
    match ch {
        'a'..='z' => Some(ch as usize - 'a' as usize),
        ' ' => Some(SPACE_INDEX),
        _ => None,
    }
}

impl Corpus {
    pub fn new(
        letters: HashMap<char, f64>,
        bigrams: HashMap<(char, char), f64>,
        trigrams: HashMap<(char, char, char), f64>,
    ) -> Self {
        let mut letter_keys: Vec<char> = letters.keys().copied().collect();
        letter_keys.sort_unstable();
        let total_letters = letter_keys.iter().map(|k| letters[k]).sum();
        let mut bigram_keys: Vec<(char, char)> = bigrams.keys().copied().collect();
        bigram_keys.sort_unstable();
        let total_bigrams = bigram_keys.iter().map(|k| bigrams[k]).sum();
        let mut trigram_keys: Vec<(char, char, char)> = trigrams.keys().copied().collect();
        trigram_keys.sort_unstable();
        let total_trigrams = trigram_keys.iter().map(|k| trigrams[k]).sum();
        let mut bi_ids = Vec::new();
        let mut bi_counts = Vec::new();
        for &(a, b) in &bigram_keys {
            if let (Some(ai), Some(bi)) = (cid(a), cid(b)) {
                bi_ids.push([ai, bi]);
                bi_counts.push(bigrams[&(a, b)]);
            }
        }
        let mut tri_ids = Vec::new();
        let mut tri_counts = Vec::new();
        for &(a, b, c) in &trigram_keys {
            if let (Some(ai), Some(bi), Some(ci)) = (cid(a), cid(b), cid(c)) {
                tri_ids.push([ai, bi, ci]);
                tri_counts.push(trigrams[&(a, b, c)]);
            }
        }
        Self {
            letters,
            bigrams,
            trigrams,
            bi_ids,
            bi_counts,
            tri_ids,
            tri_counts,
            total_letters,
            total_bigrams,
            total_trigrams,
        }
    }

    pub fn scaled(&self, factor: f64) -> Self {
        if factor == 1.0 {
            return self.clone();
        }
        Self::new(
            self.letters.iter().map(|(k, v)| (*k, v * factor)).collect(),
            self.bigrams.iter().map(|(k, v)| (*k, v * factor)).collect(),
            self.trigrams.iter().map(|(k, v)| (*k, v * factor)).collect(),
        )
    }

    pub fn merge(corpora: &[Corpus]) -> Result<Self> {
        if corpora.is_empty() {
            return Err(anyhow!("no corpora to merge"));
        }
        if corpora.len() == 1 {
            return Ok(corpora[0].clone());
        }
        let mut letters = HashMap::new();
        let mut bigrams = HashMap::new();
        let mut trigrams = HashMap::new();
        for corpus in corpora {
            let mut letter_keys: Vec<char> = corpus.letters.keys().copied().collect();
            letter_keys.sort_unstable();
            for k in letter_keys {
                *letters.entry(k).or_insert(0.0) += corpus.letters[&k];
            }
            let mut bigram_keys: Vec<(char, char)> = corpus.bigrams.keys().copied().collect();
            bigram_keys.sort_unstable();
            for k in bigram_keys {
                *bigrams.entry(k).or_insert(0.0) += corpus.bigrams[&k];
            }
            let mut trigram_keys: Vec<(char, char, char)> =
                corpus.trigrams.keys().copied().collect();
            trigram_keys.sort_unstable();
            for k in trigram_keys {
                *trigrams.entry(k).or_insert(0.0) += corpus.trigrams[&k];
            }
        }
        Ok(Self::new(letters, bigrams, trigrams))
    }

    pub fn load(path: &Path) -> Result<Self> {
        if let Some(parsed) = parsed_dir(path) {
            return Self::load_parsed(&parsed);
        }
        if let Some(source) = source_file(path) {
            let text = read_source(&source)?;
            let normalized: String = text
                .to_lowercase()
                .nfkd()
                .filter(|c| !is_combining_mark(*c))
                .collect();
            return Ok(Self::count_text(&normalized));
        }
        Err(anyhow!(
            "cannot resolve corpus {}: expected a parsed directory, a corpus directory containing parsed/, or a raw .txt/.xz/.zip source",
            path.display()
        ))
    }

    fn load_parsed(dir: &Path) -> Result<Self> {
        let letters_raw: HashMap<String, f64> = read_json(&dir.join(PARSED_FILES[0]))?;
        let bigrams_raw: HashMap<String, f64> = read_json(&dir.join(PARSED_FILES[1]))?;
        let trigrams_raw: HashMap<String, f64> = read_json(&dir.join(PARSED_FILES[2]))?;
        let mut letters = HashMap::new();
        for (k, v) in letters_raw {
            if let Some(ch) = single_char(&k) {
                letters.insert(ch, v);
            }
        }
        let mut bigrams = HashMap::new();
        for (k, v) in bigrams_raw {
            let chars: Vec<char> = k.chars().collect();
            if chars.len() == 2 {
                bigrams.insert((chars[0], chars[1]), v);
            }
        }
        let mut trigrams = HashMap::new();
        for (k, v) in trigrams_raw {
            let chars: Vec<char> = k.chars().collect();
            if chars.len() == 3 {
                trigrams.insert((chars[0], chars[1], chars[2]), v);
            }
        }
        Ok(Self::new(letters, bigrams, trigrams))
    }

    fn count_text(text: &str) -> Self {
        let mut letters = HashMap::new();
        let mut bigrams = HashMap::new();
        let mut trigrams = HashMap::new();
        let mut run: Vec<char> = Vec::new();
        let mut flush = |run: &mut Vec<char>| {
            for ch in run.iter() {
                *letters.entry(*ch).or_insert(0.0) += 1.0;
            }
            for i in 0..run.len().saturating_sub(1) {
                *bigrams.entry((run[i], run[i + 1])).or_insert(0.0) += 1.0;
            }
            for i in 0..run.len().saturating_sub(2) {
                *trigrams
                    .entry((run[i], run[i + 1], run[i + 2]))
                    .or_insert(0.0) += 1.0;
            }
            run.clear();
        };
        for ch in text.chars() {
            if ch.is_ascii_lowercase() {
                run.push(ch);
            } else {
                flush(&mut run);
            }
        }
        flush(&mut run);
        Self::new(letters, bigrams, trigrams)
    }
}

fn single_char(s: &str) -> Option<char> {
    let mut it = s.chars();
    let ch = it.next()?;
    if it.next().is_some() {
        None
    } else {
        Some(ch)
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("invalid JSON in {}", path.display()))
}

fn parsed_dir(path: &Path) -> Option<PathBuf> {
    let path = expand_user(path);
    if path.is_file() {
        let name = path.file_name()?.to_string_lossy();
        if PARSED_FILES.contains(&name.as_ref()) {
            return path.parent().map(Path::to_path_buf);
        }
        return None;
    }
    if path.is_dir() {
        if PARSED_FILES.iter().all(|n| path.join(n).is_file()) {
            return Some(path);
        }
        let parsed = path.join("parsed");
        if PARSED_FILES.iter().all(|n| parsed.join(n).is_file()) {
            return Some(parsed);
        }
    }
    None
}

fn source_file(path: &Path) -> Option<PathBuf> {
    let path = expand_user(path);
    if path.is_file() {
        return Some(path);
    }
    if path.is_dir() {
        let candidates: Vec<PathBuf> = std::fs::read_dir(&path)
            .ok()?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.is_file()
                    && p.extension()
                        .and_then(|e| e.to_str())
                        .map(|e| matches!(e.to_lowercase().as_str(), "txt" | "xz" | "zip"))
                        .unwrap_or(false)
            })
            .collect();
        if candidates.len() == 1 {
            return Some(candidates[0].clone());
        }
    }
    None
}

fn expand_user(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(format!("{}/{}", home, rest));
        }
    }
    path.to_path_buf()
}

fn read_source(path: &Path) -> Result<String> {
    let suffix = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match suffix.as_str() {
        "xz" => {
            let mut input = Vec::new();
            File::open(path)
                .with_context(|| format!("failed to open {}", path.display()))?
                .read_to_end(&mut input)?;
            let mut output = Vec::new();
            lzma_rs::xz_decompress(&mut &input[..], &mut output)
                .with_context(|| format!("failed to decompress {}", path.display()))?;
            Ok(String::from_utf8_lossy(&output).into_owned())
        }
        "zip" => {
            let file = File::open(path)
                .with_context(|| format!("failed to open {}", path.display()))?;
            let mut archive = zip::ZipArchive::new(file)
                .with_context(|| format!("invalid zip {}", path.display()))?;
            let name = (0..archive.len())
                .map(|i| archive.by_index(i).map(|f| f.name().to_string()))
                .collect::<std::result::Result<Vec<_>, _>>()?
                .into_iter()
                .find(|n| n.to_lowercase().ends_with(".txt"))
                .ok_or_else(|| anyhow!("no .txt member in {}", path.display()))?;
            let mut member = archive.by_name(&name)?;
            let mut output = Vec::new();
            member.read_to_end(&mut output)?;
            Ok(String::from_utf8_lossy(&output).into_owned())
        }
        _ => Ok(std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?),
    }
}

pub fn load_many(paths: &[PathBuf]) -> Result<Corpus> {
    let corpora = paths
        .iter()
        .map(|p| Corpus::load(p))
        .collect::<Result<Vec<_>>>()?;
    Corpus::merge(&corpora)
}
