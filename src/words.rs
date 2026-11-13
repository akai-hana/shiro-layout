//! `words`: score a layout against a common-word frequency list.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use serde_json::json;

use crate::analyze::{classify_trigram, trigram_pattern};
use crate::context;
use crate::paths;
use crate::redirect::RedirectTiers;
use crate::scorer::{big_idx, tri_idx, Tables};

#[derive(Clone, Debug)]
struct WordEntry {
    word: String,
    freq: f64,
}

#[derive(Clone, Debug, Default)]
struct WordScore {
    word: String,
    freq: f64,
    bigrams: f64,
    trigrams: f64,
    bigram_total: f64,
    trigram_total: f64,
    bigram_max: f64,
    trigram_max: f64,
    sfb: f64,
    sft: f64,
    sfs: f64,
    onehand: f64,
    redirect: f64,
    repeat: f64,
}

impl WordScore {
    fn bigram_avg(&self) -> f64 {
        if self.bigrams > 0.0 {
            self.bigram_total / self.bigrams
        } else {
            0.0
        }
    }

    fn trigram_avg(&self) -> f64 {
        if self.trigrams > 0.0 {
            self.trigram_total / self.trigrams
        } else {
            0.0
        }
    }
}

#[derive(Clone, Debug, Default)]
struct Aggregate {
    words: usize,
    freq: f64,
    bigrams: f64,
    trigrams: f64,
    bigram_total: f64,
    trigram_total: f64,
    sfb: f64,
    sft: f64,
    sfs: f64,
    onehand: f64,
    redirect: f64,
    repeat: f64,
    sfs_tokens: f64,
    onehand_tokens: f64,
    redirect_tokens: f64,
    repeat_tokens: f64,
}

impl Aggregate {
    fn add(&mut self, score: &WordScore) {
        self.words += 1;
        self.freq += score.freq;
        self.bigrams += score.bigrams;
        self.trigrams += score.trigrams;
        self.bigram_total += score.bigram_total;
        self.trigram_total += score.trigram_total;
        self.sfb += score.sfb;
        self.sft += score.sft;
        self.sfs += score.sfs;
        self.onehand += score.onehand;
        self.redirect += score.redirect;
        self.repeat += score.repeat;
        if score.sfs > 0.0 {
            self.sfs_tokens += score.freq;
        }
        if score.onehand > 0.0 {
            self.onehand_tokens += score.freq;
        }
        if score.redirect > 0.0 {
            self.redirect_tokens += score.freq;
        }
        if score.repeat > 0.0 {
            self.repeat_tokens += score.freq;
        }
    }

    fn bigram_avg(&self) -> f64 {
        if self.bigrams > 0.0 {
            self.bigram_total / self.bigrams
        } else {
            0.0
        }
    }

    fn trigram_avg(&self) -> f64 {
        if self.trigrams > 0.0 {
            self.trigram_total / self.trigrams
        } else {
            0.0
        }
    }

    fn trigram_rate(&self, value: f64) -> f64 {
        if self.trigrams > 0.0 {
            value / self.trigrams * 100.0
        } else {
            0.0
        }
    }

    fn token_rate(&self, value: f64) -> f64 {
        if self.freq > 0.0 {
            value / self.freq * 100.0
        } else {
            0.0
        }
    }
}

struct KeyMap {
    letters: [Option<usize>; 26],
    apostrophe: Option<usize>,
}

impl KeyMap {
    fn new(layout: &str) -> Self {
        let mut letters = [None; 26];
        let mut apostrophe = None;
        for (key, ch) in layout.chars().enumerate() {
            if ch.is_ascii_alphabetic() {
                letters[(ch.to_ascii_lowercase() as u8 - b'a') as usize] = Some(key);
            } else if ch == '\'' {
                apostrophe = Some(key);
            }
        }
        Self {
            letters,
            apostrophe,
        }
    }

    fn key(&self, ch: char) -> Option<usize> {
        match ch {
            'a'..='z' => self.letters[(ch as u8 - b'a') as usize],
            '\'' => self.apostrophe,
            _ => None,
        }
    }
}

fn normalize_word(raw: &str) -> Option<String> {
    let mut word = String::new();
    for ch in raw.chars().flat_map(|ch| ch.to_lowercase()) {
        if ch.is_ascii_lowercase() || ch == '\'' {
            word.push(ch);
        }
    }
    let word = word.trim_matches('\'');
    if word.chars().count() < 2 {
        None
    } else {
        Some(word.to_string())
    }
}

fn parse_frequency(token: &str) -> Option<f64> {
    let cleaned: String = token
        .chars()
        .filter(|ch| *ch != ',' && *ch != '_')
        .collect();
    cleaned.parse::<f64>().ok().filter(|value| *value > 0.0)
}

fn load_wordlist(path: &Path, limit: usize, min_frequency: f64) -> Result<Vec<WordEntry>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read word list {}", path.display()))?;
    let mut counts: HashMap<String, f64> = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(raw) = parts.next() else {
            continue;
        };
        let frequency = parts.next().and_then(parse_frequency).unwrap_or(1.0);
        if frequency < min_frequency {
            continue;
        }
        if let Some(word) = normalize_word(raw) {
            *counts.entry(word).or_insert(0.0) += frequency;
        }
    }

    let mut entries: Vec<WordEntry> = counts
        .into_iter()
        .map(|(word, freq)| WordEntry { word, freq })
        .collect();
    entries.sort_by(|a, b| b.freq.total_cmp(&a.freq).then_with(|| a.word.cmp(&b.word)));
    if limit > 0 {
        entries.truncate(limit);
    }
    if entries.is_empty() {
        return Err(anyhow!("word list {} has no usable words", path.display()));
    }
    Ok(entries)
}

fn score_word(
    entry: &WordEntry,
    keys: &KeyMap,
    tables: &Tables,
    tiers: Option<&RedirectTiers>,
) -> Option<WordScore> {
    let sequence: Vec<usize> = entry
        .word
        .chars()
        .map(|ch| keys.key(ch))
        .collect::<Option<Vec<_>>>()?;
    if sequence.len() < 2 {
        return None;
    }

    let frequency = entry.freq;
    let mut score = WordScore {
        word: entry.word.clone(),
        freq: frequency,
        ..WordScore::default()
    };

    for pair in sequence.windows(2) {
        let idx = big_idx(pair[0], pair[1]);
        let cost = tables.big[idx];
        if !cost.is_finite() {
            continue;
        }
        score.bigrams += frequency;
        score.bigram_total += cost * frequency;
        score.bigram_max = score.bigram_max.max(cost);
    }

    for tri in sequence.windows(3) {
        let idx = tri_idx(tri[0], tri[1], tri[2]);
        let cost = tables.tri[idx];
        if !cost.is_finite() {
            continue;
        }
        let fs = [
            tables.tri_f[idx][0] as usize,
            tables.tri_f[idx][1] as usize,
            tables.tri_f[idx][2] as usize,
        ];
        score.trigrams += frequency;
        score.trigram_total += cost * frequency;
        score.trigram_max = score.trigram_max.max(cost);

        let class = classify_trigram([tri[0], tri[1], tri[2]], fs);
        if class.sfb_true {
            score.sfb += frequency;
        }
        if class.sft_true {
            score.sft += frequency;
        }
        if class.sfs_true {
            score.sfs += frequency;
        }
        if class.repeat_adjacent {
            score.repeat += frequency;
        }

        match trigram_pattern(fs, tiers).category {
            "onehand" => score.onehand += frequency,
            "redirect" | "redirect_weak" => score.redirect += frequency,
            _ => {}
        }
    }

    Some(score)
}

fn format_count(value: f64) -> String {
    let n = value.round() as i64;
    let digits = n.abs().to_string();
    let mut grouped = String::new();
    for (i, ch) in digits.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let grouped: String = grouped.chars().rev().collect();
    if n < 0 {
        format!("-{grouped}")
    } else {
        grouped
    }
}

fn truncate(text: &str, width: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= width {
        text.to_string()
    } else {
        chars[..width].iter().collect()
    }
}

fn top_by<F>(scores: &[WordScore], top: usize, value: F) -> Vec<&WordScore>
where
    F: Fn(&WordScore) -> f64,
{
    let mut rows: Vec<&WordScore> = scores.iter().collect();
    rows.sort_by(|a, b| {
        value(b)
            .total_cmp(&value(a))
            .then_with(|| b.freq.total_cmp(&a.freq))
            .then_with(|| a.word.cmp(&b.word))
    });
    rows.truncate(top);
    rows
}

fn append_cost_rows(lines: &mut Vec<String>, title: &str, rows: &[&WordScore], trigram: bool) {
    lines.push(format!("== {title} =="));
    if rows.is_empty() {
        lines.push("  (none)".to_string());
        lines.push(String::new());
        return;
    }
    lines.push(format!(
        "{:>4}  {:<18} {:>10} {:>12} {:>10}",
        "rank", "word", "freq", "total", "average"
    ));
    for (i, row) in rows.iter().enumerate() {
        let (total, average) = if trigram {
            (row.trigram_total, row.trigram_avg())
        } else {
            (row.bigram_total, row.bigram_avg())
        };
        lines.push(format!(
            "{:>4}  {:<18} {:>10} {:>12.3} {:>10.3}",
            i + 1,
            truncate(&row.word, 18),
            format_count(row.freq),
            total,
            average
        ));
    }
    lines.push(String::new());
}

fn append_rows(lines: &mut Vec<String>, title: &str, rows: &[&WordScore], show_features: bool) {
    lines.push(format!("== {title} =="));
    if rows.is_empty() {
        lines.push("  (none)".to_string());
        lines.push(String::new());
        return;
    }
    if show_features {
        lines.push(format!(
            "{:>4}  {:<18} {:>10} {:>10} {:>10} {:>8} {:>8} {:>8} {:>8}",
            "rank", "word", "freq", "bigram", "trigram", "sfs", "onehand", "redirect", "repeat"
        ));
        for (i, row) in rows.iter().enumerate() {
            lines.push(format!(
                "{:>4}  {:<18} {:>10} {:>10.3} {:>10.3} {:>8.0} {:>8.0} {:>8.0} {:>8.0}",
                i + 1,
                truncate(&row.word, 18),
                format_count(row.freq),
                row.bigram_avg(),
                row.trigram_avg(),
                row.sfs,
                row.onehand,
                row.redirect,
                row.repeat
            ));
        }
    } else {
        lines.push(format!(
            "{:>4}  {:<18} {:>10} {:>10} {:>10}",
            "rank", "word", "freq", "bigram", "trigram"
        ));
        for (i, row) in rows.iter().enumerate() {
            lines.push(format!(
                "{:>4}  {:<18} {:>10} {:>10.3} {:>10.3}",
                i + 1,
                truncate(&row.word, 18),
                format_count(row.freq),
                row.bigram_avg(),
                row.trigram_avg()
            ));
        }
    }
    lines.push(String::new());
}

fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(value)? + "\n")
        .with_context(|| format!("failed to write {}", path.display()))
}

fn write_corpus(dir: &Path, entries: &[WordEntry]) -> Result<()> {
    let mut letters: HashMap<String, f64> = HashMap::new();
    let mut bigrams: HashMap<String, f64> = HashMap::new();
    let mut trigrams: HashMap<String, f64> = HashMap::new();

    for entry in entries {
        for run in entry.word.split(|ch: char| !ch.is_ascii_alphabetic()) {
            let chars: Vec<char> = run.chars().collect();
            if chars.is_empty() {
                continue;
            }
            for ch in &chars {
                *letters.entry(ch.to_string()).or_insert(0.0) += entry.freq;
            }
            for pair in chars.windows(2) {
                *bigrams
                    .entry(pair.iter().collect::<String>())
                    .or_insert(0.0) += entry.freq;
            }
            for tri in chars.windows(3) {
                *trigrams
                    .entry(tri.iter().collect::<String>())
                    .or_insert(0.0) += entry.freq;
            }
        }
    }

    std::fs::create_dir_all(dir)
        .with_context(|| format!("failed to create corpus directory {}", dir.display()))?;
    write_json(&dir.join("letter_freq.json"), &letters)?;
    write_json(&dir.join("bigram_freq.json"), &bigrams)?;
    write_json(&dir.join("trigram_freq.json"), &trigrams)?;
    Ok(())
}

pub struct WordsArgs<'a> {
    pub layout: Option<&'a str>,
    pub config: Option<&'a Path>,
    pub wordlist: &'a Path,
    pub limit: usize,
    pub top: usize,
    pub min_frequency: f64,
    pub out: Option<&'a Path>,
    pub out_json: Option<&'a Path>,
    pub out_corpus: Option<&'a Path>,
}

pub fn run(args: WordsArgs) -> Result<()> {
    let ctx = context::Context::load(args.config)?;
    let latest;
    let layout = match args.layout {
        Some(layout) => layout,
        None => {
            latest = paths::read_latest_layout()?;
            &latest
        }
    };
    if layout.chars().count() != 30 {
        return Err(anyhow!("layout must be 30 characters"));
    }

    let entries = load_wordlist(args.wordlist, args.limit, args.min_frequency)?;
    let keys = KeyMap::new(layout);
    let tiers = ctx.cfg.redirect_tiers.as_ref();

    let mut scores = Vec::new();
    for entry in &entries {
        if let Some(score) = score_word(entry, &keys, &ctx.tables, tiers) {
            scores.push(score);
        }
    }
    if scores.is_empty() {
        return Err(anyhow!(
            "no words from {} fit layout {}",
            args.wordlist.display(),
            layout
        ));
    }

    let mut aggregate = Aggregate::default();
    for score in &scores {
        aggregate.add(score);
    }

    let top_bigram = top_by(&scores, args.top, |score| score.bigram_total);
    let top_trigram = top_by(&scores, args.top, |score| score.trigram_total);
    let top_sfs = top_by(&scores, args.top, |score| score.sfs);
    let top_onehand = top_by(&scores, args.top, |score| score.onehand);
    let top_redirect = top_by(&scores, args.top, |score| score.redirect);
    let top_repeat = top_by(&scores, args.top, |score| score.repeat);

    let mut lines = vec![
        format!("layout: {}", layout),
        format!(
            "wordlist: {} ({} words, {} weighted tokens)",
            args.wordlist.display(),
            entries.len(),
            format_count(aggregate.freq)
        ),
        format!(
            "scored: {} words, {} weighted tokens",
            scores.len(),
            format_count(aggregate.freq)
        ),
        String::new(),
        "== aggregate ==".to_string(),
        format!("weighted avg bigram cost: {:.4}", aggregate.bigram_avg()),
        format!("weighted avg trigram cost: {:.4}", aggregate.trigram_avg()),
        format!(
            "true sfb: {:.2}% of trigrams",
            aggregate.trigram_rate(aggregate.sfb)
        ),
        format!(
            "true sft: {:.2}% of trigrams",
            aggregate.trigram_rate(aggregate.sft)
        ),
        format!(
            "true sfs: {:.2}% of trigrams",
            aggregate.trigram_rate(aggregate.sfs)
        ),
        format!(
            "onehand: {:.2}% of trigrams",
            aggregate.trigram_rate(aggregate.onehand)
        ),
        format!(
            "redirect: {:.2}% of trigrams",
            aggregate.trigram_rate(aggregate.redirect)
        ),
        format!(
            "repeat: {:.2}% of trigrams",
            aggregate.trigram_rate(aggregate.repeat)
        ),
        format!(
            "words with sfs: {:.2}% of tokens",
            aggregate.token_rate(aggregate.sfs_tokens)
        ),
        format!(
            "words with onehand: {:.2}% of tokens",
            aggregate.token_rate(aggregate.onehand_tokens)
        ),
        format!(
            "words with redirect: {:.2}% of tokens",
            aggregate.token_rate(aggregate.redirect_tokens)
        ),
        format!(
            "words with repeat: {:.2}% of tokens",
            aggregate.token_rate(aggregate.repeat_tokens)
        ),
        String::new(),
    ];

    append_cost_rows(
        &mut lines,
        "top words by weighted bigram cost",
        &top_bigram,
        false,
    );
    append_cost_rows(
        &mut lines,
        "top words by weighted trigram cost",
        &top_trigram,
        true,
    );
    append_rows(&mut lines, "top words by sfs count", &top_sfs, true);
    append_rows(&mut lines, "top words by onehand count", &top_onehand, true);
    append_rows(
        &mut lines,
        "top words by redirect count",
        &top_redirect,
        true,
    );
    append_rows(&mut lines, "top words by repeat count", &top_repeat, true);
    lines.push(
        "note: costs are the calibrated per-ngram model costs; feature counts are frequency-weighted."
            .to_string(),
    );

    for line in &lines {
        println!("{line}");
    }

    if let Some(path) = args.out {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, lines.join("\n") + "\n")?;
        println!("\nwrote {}", path.display());
    }

    if let Some(path) = args.out_json {
        let rows = |rows: &[&WordScore]| {
            rows.iter()
                .map(|score| {
                    json!({
                        "word": score.word,
                        "frequency": score.freq,
                        "bigram_avg": score.bigram_avg(),
                        "trigram_avg": score.trigram_avg(),
                        "bigram_max": score.bigram_max,
                        "trigram_max": score.trigram_max,
                        "sfb": score.sfb,
                        "sft": score.sft,
                        "sfs": score.sfs,
                        "onehand": score.onehand,
                        "redirect": score.redirect,
                        "repeat": score.repeat,
                    })
                })
                .collect::<Vec<_>>()
        };
        let payload = json!({
            "layout": layout,
            "wordlist": args.wordlist.display().to_string(),
            "words": entries.len(),
            "scored_words": scores.len(),
            "weighted_tokens": aggregate.freq,
            "aggregate": {
                "bigrams": aggregate.bigrams,
                "trigrams": aggregate.trigrams,
                "bigram_avg": aggregate.bigram_avg(),
                "trigram_avg": aggregate.trigram_avg(),
                "sfb": aggregate.sfb,
                "sft": aggregate.sft,
                "sfs": aggregate.sfs,
                "onehand": aggregate.onehand,
                "redirect": aggregate.redirect,
                "repeat": aggregate.repeat,
                "sfb_pct": aggregate.trigram_rate(aggregate.sfb),
                "sft_pct": aggregate.trigram_rate(aggregate.sft),
                "sfs_pct": aggregate.trigram_rate(aggregate.sfs),
                "onehand_pct": aggregate.trigram_rate(aggregate.onehand),
                "redirect_pct": aggregate.trigram_rate(aggregate.redirect),
                "repeat_pct": aggregate.trigram_rate(aggregate.repeat),
                "sfs_tokens_pct": aggregate.token_rate(aggregate.sfs_tokens),
                "onehand_tokens_pct": aggregate.token_rate(aggregate.onehand_tokens),
                "redirect_tokens_pct": aggregate.token_rate(aggregate.redirect_tokens),
                "repeat_tokens_pct": aggregate.token_rate(aggregate.repeat_tokens),
            },
            "top_bigram_cost": rows(&top_bigram),
            "top_trigram_cost": rows(&top_trigram),
            "top_sfs": rows(&top_sfs),
            "top_onehand": rows(&top_onehand),
            "top_redirect": rows(&top_redirect),
            "top_repeat": rows(&top_repeat),
        });
        write_json(path, &payload)?;
        println!("wrote {}", path.display());
    }

    if let Some(dir) = args.out_corpus {
        write_corpus(dir, &entries)?;
        println!(
            "wrote corpus {} (letter_freq.json, bigram_freq.json, trigram_freq.json)",
            dir.display()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_weighted_and_plain_wordlists() {
        let path =
            std::env::temp_dir().join(format!("shirolyzer-words-{}-parse.txt", std::process::id()));
        std::fs::write(&path, "the 100\nwe 50\n# comment\nty\n").unwrap();
        let entries = load_wordlist(&path, 0, 0.0).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].word, "the");
        assert_eq!(entries[0].freq, 100.0);
        assert_eq!(entries[1].word, "we");
        assert_eq!(entries[1].freq, 50.0);
        assert_eq!(entries[2].word, "ty");
        assert_eq!(entries[2].freq, 1.0);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn keymap_maps_letters_and_apostrophe_by_physical_position() {
        let keys = KeyMap::new("ftrhcguenjsvld'oawpimzxky,.bq/");
        assert_eq!(keys.key('f'), Some(0));
        assert_eq!(keys.key('a'), Some(16));
        assert_eq!(keys.key('\''), Some(14));
        assert_eq!(keys.key(' '), None);
    }

    #[test]
    fn normalizes_contractions_and_filters_short_words() {
        assert_eq!(normalize_word("Don't"), Some("don't".to_string()));
        assert_eq!(normalize_word("'tis"), Some("tis".to_string()));
        assert_eq!(normalize_word("a"), None);
        assert_eq!(normalize_word("123"), None);
    }

    #[test]
    fn corpus_round_trips_through_loader() {
        let dir =
            std::env::temp_dir().join(format!("shirolyzer-words-{}-corpus", std::process::id()));
        let entries = vec![
            WordEntry {
                word: "we".to_string(),
                freq: 10.0,
            },
            WordEntry {
                word: "the".to_string(),
                freq: 20.0,
            },
        ];
        write_corpus(&dir, &entries).unwrap();
        let corpus = crate::corpus::Corpus::load(&dir).unwrap();
        assert_eq!(corpus.bigrams.get(&('t', 'h')), Some(&20.0));
        assert_eq!(corpus.trigrams.get(&('t', 'h', 'e')), Some(&20.0));
        std::fs::remove_dir_all(dir).ok();
    }
}
