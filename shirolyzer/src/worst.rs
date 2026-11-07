//! `worst`: list the highest-cost bigrams and trigrams for a layout.

use std::path::Path;

use anyhow::{anyhow, Result};

use crate::analyze::{trigram_pattern, true_same_finger_metrics};
use crate::context;
use crate::corpus::SPACE_INDEX;
use crate::geometry::{finger_token, key_index, KEYS};
use crate::paths;
use crate::scorer::{big_idx, tri_idx, FINGER_SLOTS, SPACE_FINGER};

const LETTERS: usize = 26;

fn letter_id(ch: char) -> Option<usize> {
    match ch {
        'a'..='z' => Some(ch as usize - 'a' as usize),
        ' ' => Some(SPACE_INDEX),
        _ => None,
    }
}

fn id_char(id: usize) -> char {
    if id == SPACE_INDEX {
        ' '
    } else {
        (b'a' + id as u8) as char
    }
}

fn keymap(layout: &str) -> [Option<usize>; LETTERS + 1] {
    let mut map = [None; LETTERS + 1];
    for (i, ch) in layout.chars().enumerate() {
        if let (Some(id), Some(key)) = (letter_id(ch), key_index(KEYS[i])) {
            map[id] = Some(key);
        }
    }
    map
}

fn finger_label(finger: usize) -> String {
    if finger == SPACE_FINGER {
        "T".to_string()
    } else {
        finger_token(finger).to_string()
    }
}

struct Row {
    cost: f64,
    ngram: String,
    qwerty: String,
    fingers: String,
    pattern: String,
    count: f64,
}

fn qwerty(keys: &[usize]) -> String {
    keys.iter()
        .map(|&key| KEYS.get(key).copied().unwrap_or("SPACE"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn ngram(seq: &[usize]) -> String {
    seq.iter().map(|&id| id_char(id)).collect()
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

fn sort_rows(rows: &mut [Row]) {
    rows.sort_by(|a, b| {
        b.cost
            .total_cmp(&a.cost)
            .then_with(|| b.count.total_cmp(&a.count))
            .then_with(|| a.ngram.cmp(&b.ngram))
    });
}

fn append_section(
    lines: &mut Vec<String>,
    title: &str,
    rows: &[Row],
    limit: usize,
    show_pattern: bool,
) {
    lines.push(format!("== {title} =="));
    if rows.is_empty() {
        lines.push("  (none)".to_string());
        lines.push(String::new());
        return;
    }
    let shown = if limit == 0 {
        rows.len()
    } else {
        rows.len().min(limit)
    };
    lines.push(format!("showing {shown} of {} ngrams", rows.len()));
    if show_pattern {
        lines.push(format!(
            "{:>4}  {:<6}  {:<12}  {:<10}  {:>8}  {:<14}  {:>14}",
            "rank", "ngram", "qwerty", "fingers", "cost", "pattern", "count"
        ));
    } else {
        lines.push(format!(
            "{:>4}  {:<6}  {:<12}  {:<10}  {:>8}  {:>14}",
            "rank", "ngram", "qwerty", "fingers", "cost", "count"
        ));
    }
    for (i, row) in rows.iter().take(shown).enumerate() {
        if show_pattern {
            lines.push(format!(
                "{:>4}  {:<6}  {:<12}  {:<10}  {:>8.3}  {:<14}  {:>14}",
                i + 1,
                row.ngram,
                row.qwerty,
                row.fingers,
                row.cost,
                row.pattern,
                format_count(row.count)
            ));
        } else {
            lines.push(format!(
                "{:>4}  {:<6}  {:<12}  {:<10}  {:>8.3}  {:>14}",
                i + 1,
                row.ngram,
                row.qwerty,
                row.fingers,
                row.cost,
                format_count(row.count)
            ));
        }
    }
    lines.push(String::new());
}

pub struct WorstArgs<'a> {
    pub layout: Option<&'a str>,
    pub config: Option<&'a Path>,
    pub primary_corpus: &'a [String],
    pub limit: usize,
    pub out: Option<&'a Path>,
}

pub fn run(args: WorstArgs) -> Result<()> {
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

    let (corpus, labels) = if !args.primary_corpus.is_empty() {
        (
            context::load_paths(args.primary_corpus)?,
            args.primary_corpus.to_vec(),
        )
    } else {
        context::load_role(&ctx.cfg, "primary")?
    };

    let m = keymap(layout);

    let mut bigrams = Vec::new();
    for (ids, &count) in corpus.bi_ids.iter().zip(&corpus.bi_counts) {
        let (Some(a), Some(b)) = (m[ids[0]], m[ids[1]]) else {
            continue;
        };
        let idx = big_idx(a, b);
        let enc = ctx.tables.big_f[idx] as usize;
        let f1 = enc / FINGER_SLOTS;
        let f2 = enc % FINGER_SLOTS;
        bigrams.push(Row {
            cost: ctx.tables.big[idx],
            ngram: ngram(&[ids[0], ids[1]]),
            qwerty: qwerty(&[a, b]),
            fingers: format!("{}-{}", finger_label(f1), finger_label(f2)),
            pattern: String::new(),
            count,
        });
    }

    let mut trigrams = Vec::new();
    for (ids, &count) in corpus.tri_ids.iter().zip(&corpus.tri_counts) {
        let (Some(a), Some(b), Some(c)) = (m[ids[0]], m[ids[1]], m[ids[2]]) else {
            continue;
        };
        let idx = tri_idx(a, b, c);
        let fs = ctx.tables.tri_f[idx];
        let fs = [fs[0] as usize, fs[1] as usize, fs[2] as usize];
        let tiers = ctx.cfg.redirect_tiers.as_ref();
        trigrams.push(Row {
            cost: ctx.tables.tri[idx],
            ngram: ngram(&[ids[0], ids[1], ids[2]]),
            qwerty: qwerty(&[a, b, c]),
            fingers: format!(
                "{}-{}-{}",
                finger_label(fs[0]),
                finger_label(fs[1]),
                finger_label(fs[2])
            ),
            pattern: trigram_pattern(fs, tiers).label(tiers),
            count,
        });
    }

    sort_rows(&mut bigrams);
    sort_rows(&mut trigrams);

    let mut lines = vec![
        format!("layout: {}", layout),
        format!("corpus: {}", labels.join(",")),
        String::new(),
    ];
    append_section(
        &mut lines,
        "worst bigrams (highest cost first)",
        &bigrams,
        args.limit,
        false,
    );
    append_section(
        &mut lines,
        "worst trigrams (highest cost first)",
        &trigrams,
        args.limit,
        true,
    );
    let true_metrics =
        true_same_finger_metrics(layout, &corpus, &ctx.tables, &ctx.model.default_finger);
    lines.push("== true same-finger metrics (% of trigrams) ==".to_string());
    lines.push(format!(
        "{:<20} {:>8} {:>8} {:>8}",
        "metric", "chosen", "default", "delta"
    ));
    for name in [
        "sfb_true",
        "sft_true",
        "sfb_sft_true",
        "sfs_true",
        "repeat_adjacent",
        "repeat_skip",
        "repeat_all",
    ] {
        lines.push(format!(
            "{:<20} {:8.2} {:8.2} {:8.2}",
            name,
            true_metrics.get(name).copied().unwrap_or(0.0),
            true_metrics
                .get(&format!("{name}_default"))
                .copied()
                .unwrap_or(0.0),
            true_metrics
                .get(&format!("{name}_delta"))
                .copied()
                .unwrap_or(0.0)
        ));
    }
    lines.push(String::new());
    lines.push(
        "note: bigram and trigram costs are not directly comparable across lengths.".to_string(),
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
    Ok(())
}
