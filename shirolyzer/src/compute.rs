//! `compute`: generate a layout with the calibrated pattern model.
//!
//! Port of `shirolyzer/commands/compute.py`.

use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;
use serde_json::json;

use crate::analyze::{category_report, finger_usage, trigram_patterns};
use crate::config::Sacrifice;
use crate::context::{self, Context};
use crate::corpus::Corpus;
use crate::diversity::{self, Row};
use crate::evaluate::{canonical_punctuation, flat_share_from_config, perm_of, Evaluator};
use crate::geometry::Geometry;
use crate::paths;
use crate::redirect::RedirectTiers;
use crate::references;
use crate::scorer::Tables;
use crate::search;
use crate::ui;
use crate::xkb;

const ILS_MIN_LETTER_DISTANCE: usize = 4;

pub fn parse_budget(value: &str) -> Result<f64> {
    let value = value.trim().to_lowercase();
    let number = |text: &str| -> Result<f64> {
        text.parse::<f64>()
            .map_err(|_| anyhow!("invalid time budget: {}", value))
    };
    if let Some(v) = value.strip_suffix("ms") {
        Ok(number(v)? / 1000.0)
    } else if let Some(v) = value.strip_suffix('s') {
        number(v)
    } else if let Some(v) = value.strip_suffix('m') {
        Ok(number(v)? * 60.0)
    } else if let Some(v) = value.strip_suffix('h') {
        Ok(number(v)? * 3600.0)
    } else {
        number(&value)
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Metrics {
    sfb_sft: f64,
    scissors: f64,
    redirects: f64,
    rolls: f64,
    alternation: f64,
    finger_balance: f64,
}

impl Metrics {
    fn evaluate(
        layout: &str,
        corpus: &Corpus,
        tables: &Tables,
        geom: &Geometry,
        targets: &std::collections::HashMap<char, f64>,
        redirect_tiers: Option<&RedirectTiers>,
    ) -> Self {
        let cat = category_report(layout, corpus, tables, geom);
        let pats = trigram_patterns(layout, corpus, tables, geom, redirect_tiers);
        let usage = finger_usage(layout, corpus, tables, geom);
        let sfb = cat.cats.get("sfb").copied().unwrap_or(0.0);
        let sfb_tri = pats.get("sfb_tri").copied().unwrap_or(0.0);
        let sft = pats.get("sft").copied().unwrap_or(0.0);
        let scissors = [
            "scissor_IM_hsb",
            "scissor_IM_fsb",
            "scissor_MR",
            "scissor_RP",
        ]
        .iter()
        .map(|k| cat.cats.get(*k).copied().unwrap_or(0.0))
        .sum();
        let mut finger_balance = 0.0;
        for hand in ['L', 'R'] {
            for finger in ['P', 'R', 'M', 'I'] {
                let token = format!("{hand}{finger}");
                let target = targets.get(&finger).copied().unwrap_or(0.0);
                finger_balance += (usage.get(&token).copied().unwrap_or(0.0) - target).abs();
            }
        }
        Self {
            sfb_sft: sfb + sfb_tri + 2.0 * sft,
            scissors,
            redirects: pats.get("total_redirects").copied().unwrap_or(0.0),
            rolls: pats.get("total_rolls").copied().unwrap_or(0.0),
            alternation: pats.get("hand_alternation").copied().unwrap_or(0.0),
            finger_balance,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct MetricLimits {
    sfb_sft: Option<f64>,
    scissors: Option<f64>,
    redirects: Option<f64>,
    rolls: Option<f64>,
    alternation: Option<f64>,
    finger_balance: Option<f64>,
}

impl MetricLimits {
    fn new(base: &Metrics, sacrifice: &Sacrifice) -> Self {
        let lower =
            |percent: Option<f64>, value: f64| percent.map(|p| value + value.abs() * p / 100.0);
        let higher = |percent: Option<f64>, value: f64| {
            percent.map(|p| (value - value.abs() * p / 100.0).max(0.0))
        };
        Self {
            sfb_sft: lower(sacrifice.sfb_sft, base.sfb_sft),
            scissors: lower(sacrifice.scissors, base.scissors),
            redirects: lower(sacrifice.redirects, base.redirects),
            rolls: higher(sacrifice.rolls, base.rolls),
            alternation: higher(sacrifice.alternation, base.alternation),
            finger_balance: lower(sacrifice.finger_balance, base.finger_balance),
        }
    }

    fn allows(&self, metrics: &Metrics) -> bool {
        let below =
            |limit: Option<f64>, value: f64| limit.map(|l| value <= l + 1e-12).unwrap_or(true);
        let above =
            |limit: Option<f64>, value: f64| limit.map(|l| value + 1e-12 >= l).unwrap_or(true);
        below(self.sfb_sft, metrics.sfb_sft)
            && below(self.scissors, metrics.scissors)
            && below(self.redirects, metrics.redirects)
            && above(self.rolls, metrics.rolls)
            && above(self.alternation, metrics.alternation)
            && below(self.finger_balance, metrics.finger_balance)
    }
}

pub struct ComputeArgs<'a> {
    pub config: Option<&'a Path>,
    pub seed_layout: Vec<String>,
    pub seed_reference: Vec<String>,
    pub seed_all_references: bool,
    pub name: Option<String>,
    pub primary_corpus: Vec<String>,
    pub secondary_corpus: Vec<String>,
    pub references: Option<PathBuf>,
    pub primary_sacrifice: Option<f64>,
    pub runs: usize,
    pub iters: usize,
    pub ils: usize,
    pub ils_candidates: usize,
    pub batch_size: usize,
    pub stop_hits: usize,
    pub stop_stale_batches: usize,
    pub valid_tolerance: f64,
    pub secondary_tolerance: f64,
    pub min_layout_distance: usize,
    pub max_valid: usize,
    pub polish_rounds: usize,
    pub polish_candidates: usize,
    pub append_references: Option<PathBuf>,
    pub seed: Option<u64>,
    pub time_budget: Option<f64>,
    pub verbose: bool,
    pub xkb: bool,
    pub xkb_name: Option<String>,
    pub out: Option<PathBuf>,
    pub out_json: Option<PathBuf>,
}

fn fmt_sig(value: f64, sig: usize) -> String {
    if value == 0.0 || !value.is_finite() {
        return format!("{}", value);
    }
    let exp = value.abs().log10().floor() as i32;
    if exp < -4 || exp >= sig as i32 {
        format!("{:.*e}", sig.saturating_sub(1), value)
    } else {
        let decimals = (sig as i32 - 1 - exp).max(0) as usize;
        format!("{:.*}", decimals, value)
    }
}

fn choose3(rng: &mut impl rand::Rng, values: &[usize]) -> usize {
    values[rng.gen_range(0..values.len())]
}

fn seed_rng(seed: u64, index: usize) -> ChaCha8Rng {
    const GOLDEN: u64 = 0x9e37_79b9_7f4a_7c15;
    ChaCha8Rng::seed_from_u64(seed ^ GOLDEN.wrapping_mul(index as u64 + 1))
}

fn seed_anneal(seed: u64, index: usize) -> u64 {
    seed.wrapping_add(17).wrapping_add((index as u64) << 32)
}

fn optimize_seed(
    ev: &Evaluator<'_>,
    delta_tables: &search::DeltaTables,
    rng: &mut impl Rng,
    name: &str,
    start: &[usize],
    iters: usize,
    anneal_seed: u64,
) -> Row {
    let mut perm = start.to_vec();
    let cs = ev.score(&perm);
    let (p, _s) = search::quench(ev, delta_tables, &perm, cs, rng);
    perm = p;
    let (p, s) = search::anneal(ev, Some(&perm), iters, 0.35, 0.002, anneal_seed);
    perm = p;
    let (p, s) = search::quench(ev, delta_tables, &perm, s, rng);
    perm = p;
    canonical_punctuation(&mut perm);
    let layout = ev.layout_str(&perm);
    let primary = ev.score(&perm);
    Row {
        start: name.to_string(),
        final_score: s,
        primary,
        secondary: None,
        layout,
        reference_name: None,
    }
}

struct IlsOutcome {
    row: Row,
    source_start: String,
    start_score: f64,
    rounds: usize,
    iterations: usize,
}

fn ils_rng(seed: u64, rank: usize) -> ChaCha8Rng {
    seed_rng(seed, 0x1000 + rank)
}

#[allow(clippy::too_many_arguments)]
fn optimize_ils_row(
    ev: &Evaluator<'_>,
    delta_tables: &search::DeltaTables,
    row: &Row,
    rank: usize,
    iters: usize,
    ils: usize,
    anneal_base: u64,
    rng: &mut impl Rng,
    deadline: Option<Instant>,
    on_round: &(dyn Fn() + Sync),
) -> Result<IlsOutcome> {
    let mut best = perm_of(&row.layout, &ev.punctuation).map_err(anyhow::Error::msg)?;
    let mut best_s = row.final_score;
    let mut rounds = 0usize;
    let mut iterations = 0usize;
    for it in 0..ils.max(1) {
        if deadline.is_some_and(|d| Instant::now() >= d) {
            break;
        }
        let k = choose3(rng, &[4, 6, 8]);
        let cand = ev.perturb(&best, rng, k);
        let (cand, _cs) = search::quench(ev, delta_tables, &cand, ev.score(&cand), rng);
        let round_iters = (iters / 2).max(1);
        let (cand, cs) = search::anneal(
            ev,
            Some(&cand),
            round_iters,
            0.35,
            0.002,
            anneal_base.wrapping_add(it as u64),
        );
        let (cand, cs) = search::quench(ev, delta_tables, &cand, cs, rng);
        rounds += 1;
        iterations += round_iters;
        on_round();
        if cs < best_s - 1e-12 {
            best = cand;
            best_s = cs;
        }
    }
    canonical_punctuation(&mut best);
    let layout = ev.layout_str(&best);
    let primary = ev.score(&best);
    Ok(IlsOutcome {
        row: Row {
            start: format!("ILS{}:{}", rank + 1, row.start),
            final_score: best_s,
            primary,
            secondary: None,
            layout,
            reference_name: None,
        },
        source_start: row.start.clone(),
        start_score: row.final_score,
        rounds,
        iterations,
    })
}

struct PolishOutcome {
    row: Row,
    passes: usize,
    iterations: usize,
    start_s: f64,
}

fn polish_rng(seed: u64, index: usize) -> ChaCha8Rng {
    seed_rng(seed, 0x2000 + index)
}

#[allow(clippy::too_many_arguments)]
fn optimize_polish_row(
    ev: &Evaluator<'_>,
    delta_tables: &search::DeltaTables,
    row: &Row,
    index: usize,
    polish_rounds: usize,
    iters: usize,
    seed: u64,
    rng: &mut impl Rng,
    secondary_ev: Option<&Evaluator<'_>>,
    deadline: Option<Instant>,
    on_round: &(dyn Fn() + Sync),
) -> Result<PolishOutcome> {
    let mut best = perm_of(&row.layout, &ev.punctuation).map_err(anyhow::Error::msg)?;
    let mut best_s = ev.score(&best);
    let start_s = best_s;
    let mut passes = 0usize;
    let mut iterations = 0usize;
    // polish_rounds == usize::MAX means unlimited: keep going until a round
    // fails to improve the best layout.
    for round_idx in 0..polish_rounds.max(1) {
        if deadline.is_some_and(|d| Instant::now() >= d) {
            break;
        }
        let k = choose3(rng, &[4, 6, 8]);
        let mut cand = ev.perturb(&best, rng, k);
        let (p, _cs) = search::quench(ev, delta_tables, &cand, ev.score(&cand), rng);
        cand = p;
        let round_iters = iters.max(1);
        let (p, cs) = search::anneal(
            ev,
            Some(&cand),
            round_iters,
            0.35,
            0.002,
            seed.wrapping_add(100_003)
                .wrapping_add(17 * round_idx as u64)
                .wrapping_add(index as u64)
                .wrapping_add(1),
        );
        cand = p;
        let (cand, cs) = search::quench(ev, delta_tables, &cand, cs, rng);
        passes += 1;
        iterations += round_iters;
        on_round();
        if cs < best_s - 1e-12 {
            best = cand;
            best_s = cs;
        } else {
            break;
        }
    }
    canonical_punctuation(&mut best);
    let layout = ev.layout_str(&best);
    let primary = ev.score(&best);
    let secondary = secondary_ev.map(|s| s.score(&best));
    Ok(PolishOutcome {
        row: Row {
            start: format!("polish:{}", row.start),
            final_score: best_s,
            primary,
            secondary,
            layout,
            reference_name: None,
        },
        passes,
        iterations,
        start_s,
    })
}

/// Draw a layout with the same staggered geometry as `render --plain`.
fn layout_lines(layout: &str) -> Vec<String> {
    match crate::render::render(layout, None) {
        Ok(text) => text.lines().map(str::to_string).collect(),
        Err(_) => vec![layout.to_string()],
    }
}

fn score_text(row: &Row) -> String {
    match row.secondary {
        Some(sec) => format!("primary {:.4} · secondary {:.4}", row.primary, sec),
        None => format!("primary {:.4}", row.primary),
    }
}

fn print_summary(
    ui: &ui::Style,
    selected: &Row,
    best_primary: &Row,
    valid_rows: &[Row],
    valid_truncated: bool,
    status: &str,
) {
    let status_text = if status == "clear_winner" {
        "clear winner"
    } else {
        "multiple valid layouts"
    };
    let mut body = layout_lines(&selected.layout);
    body.push(String::new());
    let mut score = score_text(selected);
    if selected.layout == best_primary.layout {
        score.push_str(" · best primary");
    }
    body.push(score);
    for line in ui.boxed(&format!("Best layout · {status_text}"), &body) {
        println!("{line}");
    }
    let others: Vec<&Row> = valid_rows
        .iter()
        .filter(|r| r.layout != selected.layout)
        .collect();
    if !others.is_empty() {
        println!();
        println!("{}", ui.bold("Other valid layouts"));
        let cards: Vec<Vec<String>> = others
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let mut head = format!("#{}  {}", i + 2, score_text(r));
                if r.layout == best_primary.layout {
                    head.push_str(" · best primary");
                }
                let mut card = vec![head];
                card.extend(
                    layout_lines(&r.layout)
                        .into_iter()
                        .map(|l| format!("  {l}")),
                );
                card
            })
            .collect();
        for line in ui::columns(&cards, 4, ui::term_width()) {
            println!("{line}");
        }
    }
    if valid_truncated {
        println!("{}", ui.dim("additional comparable layouts omitted"));
    }
}

#[allow(clippy::too_many_arguments)]
fn count_rediscoveries(results: &[Row], layout: &str) -> usize {
    let signature = diversity::letter_signature(layout);
    results
        .iter()
        .filter(|r| {
            r.start.starts_with("rand") && diversity::letter_signature(&r.layout) == signature
        })
        .count()
}

fn bump_stale(previous: Option<f64>, current: f64, stale: usize) -> usize {
    match previous {
        Some(prev) if current < prev - 1e-12 => 0,
        Some(_) => stale + 1,
        None => 0,
    }
}

#[allow(clippy::too_many_arguments)]
fn run_seed_batch(
    ev: &Evaluator<'_>,
    delta_tables: &search::DeltaTables,
    seeds: &[(String, Vec<usize>)],
    index_offset: usize,
    seed: u64,
    iters: usize,
    time_budget: Option<f64>,
    started: Instant,
    progress: &ui::Progress,
) -> Vec<Row> {
    let parallel = time_budget.is_none() && seeds.len() > 1;
    if parallel {
        seeds
            .par_iter()
            .enumerate()
            .map(|(i, (name, start))| {
                let index = index_offset + i;
                let mut rng = seed_rng(seed, index);
                let row = optimize_seed(
                    ev,
                    delta_tables,
                    &mut rng,
                    name,
                    start,
                    iters,
                    seed_anneal(seed, index),
                );
                progress.tick();
                row
            })
            .collect()
    } else {
        let mut rows = Vec::with_capacity(seeds.len());
        for (i, (name, start)) in seeds.iter().enumerate() {
            if !rows.is_empty() && time_budget.is_some_and(|b| started.elapsed().as_secs_f64() >= b)
            {
                break;
            }
            let index = index_offset + i;
            let mut rng = seed_rng(seed, index);
            rows.push(optimize_seed(
                ev,
                delta_tables,
                &mut rng,
                name,
                start,
                iters,
                seed_anneal(seed, index),
            ));
            progress.tick();
        }
        rows
    }
}

pub fn run(args: ComputeArgs) -> Result<()> {
    let ctx = Context::load(args.config)?;
    let timestamp = paths::timestamp();
    let stem = match args.name.as_deref() {
        Some(name) => format!("{}_{}", name, timestamp),
        None => timestamp.clone(),
    };
    let out_path = args
        .out
        .clone()
        .unwrap_or_else(|| paths::result_path("compute", Some(stem.as_str()), ".txt"));
    let out_json = args
        .out_json
        .clone()
        .unwrap_or_else(|| paths::result_path("compute", Some(stem.as_str()), ".json"));
    let sacrifice = args
        .primary_sacrifice
        .map(Sacrifice::score_only)
        .unwrap_or_else(|| ctx.cfg.primary_sacrifice.clone());
    let started = Instant::now();
    let mut completed_runs = 0usize;
    let mut completed_iterations = 0usize;
    let mut completed_ils_rounds = 0usize;
    let mut completed_secondary_rounds = 0usize;
    let mut completed_polish_rounds = 0usize;

    let seed_generated = args.seed.is_none();
    let seed = args.seed.unwrap_or_else(|| {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64)
            .unwrap_or(0);
        (nanos ^ (std::process::id() as u64)) & 0x7fff_ffff
    });
    let ui = ui::Style::detect();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let (primary_corpus, primary_paths) = if !args.primary_corpus.is_empty() {
        (
            context::load_paths(&args.primary_corpus)?,
            args.primary_corpus.clone(),
        )
    } else {
        context::load_role(&ctx.cfg, "primary")?
    };
    let (secondary_corpus, secondary_paths) = if !args.secondary_corpus.is_empty() {
        (
            Some(context::load_paths(&args.secondary_corpus)?),
            args.secondary_corpus.clone(),
        )
    } else {
        match context::load_role(&ctx.cfg, "secondary") {
            Ok((corpus, labels)) => (Some(corpus), labels),
            Err(_) => (None, Vec::new()),
        }
    };

    let flat_share = flat_share_from_config(&ctx.cfg);
    let (w_balance, w_hand) = ctx.cfg.balance_weights();
    let mut primary_ev = Evaluator::new(
        &primary_corpus,
        &ctx.tables,
        &ctx.model,
        &ctx.cfg.punctuation,
        flat_share,
        w_balance,
        w_hand,
    )
    .map_err(anyhow::Error::msg)?;
    primary_ev.set_hand_restrictions(ctx.cfg.hand_restrictions);
    let delta_tables = search::DeltaTables::build(&primary_ev);
    let secondary_ev = match secondary_corpus.as_ref() {
        Some(corpus) => {
            let mut ev = Evaluator::new(
                corpus,
                &ctx.tables,
                &ctx.model,
                &ctx.cfg.punctuation,
                flat_share,
                w_balance,
                w_hand,
            )
            .map_err(anyhow::Error::msg)?;
            ev.set_hand_restrictions(ctx.cfg.hand_restrictions);
            Some(ev)
        }
        None => None,
    };

    let refs_path = args
        .references
        .clone()
        .unwrap_or_else(paths::references_file);
    let mut seeds: Vec<(String, Vec<usize>)> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    // Reference layouts are NOT used as starting points unless explicitly
    // requested (--seed-reference NAME or --seed-all-references).  The
    // references file is still used below for naming and de-duplicating results.
    if args.seed_all_references || !args.seed_reference.is_empty() {
        let refs = references::load_references(Some(&refs_path))?;
        for wanted in &args.seed_reference {
            if !refs.iter().any(|e| &e.name == wanted) {
                anyhow::bail!(
                    "--seed-reference {:?}: no such layout in {}",
                    wanted,
                    refs_path.display()
                );
            }
        }
        for entry in refs {
            if !args.seed_all_references && !args.seed_reference.contains(&entry.name) {
                continue;
            }
            if seen.insert(entry.layout.clone()) {
                let perm =
                    perm_of(&entry.layout, &ctx.cfg.punctuation).map_err(anyhow::Error::msg)?;
                if !primary_ev.perm_allowed(&perm) {
                    eprintln!(
                        "warning: skipping seed {:?}: violates the configured hand restrictions",
                        entry.name
                    );
                    continue;
                }
                seeds.push((entry.name, perm));
            }
        }
    }
    for (i, layout) in args.seed_layout.iter().enumerate() {
        if layout.chars().count() == 30 && seen.insert(layout.clone()) {
            let perm = perm_of(layout, &ctx.cfg.punctuation).map_err(anyhow::Error::msg)?;
            if !primary_ev.perm_allowed(&perm) {
                eprintln!(
                    "warning: skipping user seed {}: violates the configured hand restrictions",
                    i
                );
                continue;
            }
            seeds.push((format!("user{}", i), perm));
        }
    }

    let planned_starts = seeds.len() + args.runs;
    let mut results: Vec<Row> = Vec::new();
    let mut best_primary: Option<Row> = None;
    println!(
        "{}",
        ui.bold(&format!(
            "shirolyzer compute · seed {} · {} starts · {} iters · {} ILS · polish {}",
            seed,
            planned_starts,
            ui::compact(args.iters),
            args.ils,
            match args.polish_rounds {
                0 => "off".to_string(),
                usize::MAX => "∞".to_string(),
                n => n.to_string(),
            }
        ))
    );
    println!();

    let anneal_progress = ui::Progress::new("Anneal", planned_starts);
    let batch_size = args.batch_size.max(1);
    let mut batches: Vec<Vec<(String, Vec<usize>)>> = Vec::new();
    let fixed_batches = usize::from(!seeds.is_empty());
    if !seeds.is_empty() {
        batches.push(std::mem::take(&mut seeds));
    }
    let mut random_index = 0usize;
    while random_index < args.runs {
        let end = (random_index + batch_size).min(args.runs);
        let mut batch = Vec::with_capacity(end - random_index);
        for s in random_index..end {
            let mut start_rng = ChaCha8Rng::seed_from_u64(seed + 1000 + s as u64);
            let perm = primary_ev
                .constrained_shuffle(&mut start_rng)
                .map_err(anyhow::Error::msg)?;
            batch.push((format!("rand{}", seed + 1000 + s as u64), perm));
        }
        batches.push(batch);
        random_index = end;
    }

    let mut completed_batches = 0usize;
    let mut stop_reason: Option<String> = None;
    let mut stale_batches = 0usize;
    let mut last_best: Option<f64> = None;
    let mut index_offset = 0usize;
    for (batch_index, batch) in batches.iter().enumerate() {
        if !results.is_empty()
            && args
                .time_budget
                .is_some_and(|b| started.elapsed().as_secs_f64() >= b)
        {
            stop_reason = Some("time budget".to_string());
            println!("{}", ui.dim("time budget reached; saving best so far"));
            break;
        }
        let rows = run_seed_batch(
            &primary_ev,
            &delta_tables,
            batch,
            index_offset,
            seed,
            args.iters,
            args.time_budget,
            started,
            &anneal_progress,
        );
        let got = rows.len();
        index_offset += got;
        completed_batches += 1;
        for row in rows {
            completed_runs += 1;
            completed_iterations += args.iters;
            if best_primary
                .as_ref()
                .map(|b| row.primary < b.primary)
                .unwrap_or(true)
            {
                best_primary = Some(row.clone());
            }
            results.push(row);
        }
        if got < batch.len() {
            stop_reason = Some("time budget".to_string());
            println!("{}", ui.dim("time budget reached; saving best so far"));
            break;
        }
        if batch_index + 1 < batches.len() && batch_index >= fixed_batches {
            if let Some(best) = best_primary.as_ref() {
                let rediscoveries = count_rediscoveries(&results, &best.layout);
                if args.stop_hits > 0 && rediscoveries >= args.stop_hits {
                    stop_reason = Some(format!("rediscovered by {} starts", rediscoveries));
                    break;
                }
                let current = best.primary;
                stale_batches = bump_stale(last_best, current, stale_batches);
                last_best = Some(current);
                if args.stop_stale_batches > 0 && stale_batches >= args.stop_stale_batches {
                    stop_reason = Some(format!(
                        "no primary improvement for {} batches",
                        stale_batches
                    ));
                    break;
                }
            }
        }
    }
    anneal_progress.finish();
    if results.is_empty() {
        return Err(anyhow!("no completed runs"));
    }
    let stop_reason = stop_reason.unwrap_or_else(|| "completed planned starts".to_string());

    let mut uniq: std::collections::HashMap<String, Row> = std::collections::HashMap::new();
    for row in &results {
        match uniq.get(&row.layout) {
            None => {
                uniq.insert(row.layout.clone(), row.clone());
            }
            Some(cur) => {
                if row.final_score < cur.final_score {
                    uniq.insert(row.layout.clone(), row.clone());
                }
            }
        }
    }
    let mut top: Vec<Row> = uniq.into_values().collect();
    top.sort_by(|a, b| {
        a.primary
            .partial_cmp(&b.primary)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                a.final_score
                    .partial_cmp(&b.final_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    let ranked = top.clone();
    top = diversity::select_diverse_layouts(&ranked, args.ils_candidates, ILS_MIN_LETTER_DISTANCE);

    // ---- Anneal summary -------------------------------------------------
    let mut hits: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for row in &results {
        *hits
            .entry(diversity::letter_signature(&row.layout))
            .or_insert(0) += 1;
    }
    let total_starts = results.len();
    println!(
        "{}",
        ui.section(
            "Anneal",
            Some((total_starts, total_starts)),
            &format!(
                "{} distinct layouts · best primary {}",
                ranked.len(),
                ui.bold_green(&format!("{:.4}", ranked[0].primary))
            )
        )
    );
    let shown = if args.verbose {
        ranked.len()
    } else {
        ranked.len().min(8)
    };
    let name_w = ranked
        .iter()
        .take(shown)
        .map(|r| ui::short_name(&r.start).chars().count())
        .max()
        .unwrap_or(5)
        .max(5);
    println!(
        "{}",
        ui.dim(&format!(
            "   {:>2}  {:<name_w$}  {:>7}  {:>7}  {:>4}{}",
            "#",
            "start",
            "final",
            "primary",
            "hits",
            if args.verbose { "  layout" } else { "" }
        ))
    );
    for (i, r) in ranked.iter().take(shown).enumerate() {
        let line = format!(
            "   {:>2}  {:<name_w$}  {:>7.4}  {:>7.4}  {:>4}{}",
            i + 1,
            ui::short_name(&r.start),
            r.final_score,
            r.primary,
            hits.get(&diversity::letter_signature(&r.layout))
                .copied()
                .unwrap_or(1),
            if args.verbose {
                format!("  {}", r.layout)
            } else {
                String::new()
            }
        );
        println!("{}", if i == 0 { ui.bold(&line) } else { line });
    }
    if ranked.len() > shown {
        println!(
            "{}",
            ui.dim(&format!(
                "      … {} more distinct layouts (--verbose lists them all)",
                ranked.len() - shown
            ))
        );
    }
    println!(
        "{}",
        ui.dim("   final = search's running score · primary = score recomputed from scratch · hits = starts reaching this layout")
    );
    println!();
    let deadline = args
        .time_budget
        .map(|budget| started + std::time::Duration::from_secs_f64(budget));
    let ils_planned = top.len() * args.ils.max(1);
    let ils_progress = ui::Progress::new("ILS", ils_planned);
    let ils_tick = || ils_progress.tick();
    let ils_outcomes: Vec<IlsOutcome> = if deadline.is_none() {
        top.par_iter()
            .enumerate()
            .map(|(rank, r)| {
                let mut rng = ils_rng(seed, rank);
                optimize_ils_row(
                    &primary_ev,
                    &delta_tables,
                    r,
                    rank,
                    args.iters,
                    args.ils,
                    seed_anneal(seed, 0x1000 + rank),
                    &mut rng,
                    None,
                    &ils_tick,
                )
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        let mut outcomes = Vec::new();
        for (rank, r) in top.iter().enumerate() {
            if deadline.is_some_and(|d| Instant::now() >= d) {
                ils_progress.finish();
                println!(
                    "{}",
                    ui.dim("time budget reached during ILS; saving best so far")
                );
                break;
            }
            let mut rng = ils_rng(seed, rank);
            outcomes.push(optimize_ils_row(
                &primary_ev,
                &delta_tables,
                r,
                rank,
                args.iters,
                args.ils,
                seed_anneal(seed, 0x1000 + rank),
                &mut rng,
                deadline,
                &ils_tick,
            )?);
        }
        outcomes
    };
    ils_progress.finish();
    let ils_done: usize = ils_outcomes.iter().map(|o| o.rounds).sum();
    println!(
        "{}",
        ui.section(
            "ILS",
            Some((ils_done, ils_planned)),
            &format!("{} starts × {} rounds", ils_outcomes.len(), args.ils.max(1))
        )
    );
    let ils_name_w = ils_outcomes
        .iter()
        .map(|o| ui::short_name(&o.source_start).chars().count())
        .max()
        .unwrap_or(5)
        .max(5);
    println!(
        "{}",
        ui.dim(&format!(
            "   {:<ils_name_w$}  {:>7}     {:>7}  {:>7}  {:>8}",
            "start", "before", "after", "primary", "gain"
        ))
    );
    for outcome in ils_outcomes.into_iter() {
        completed_ils_rounds += outcome.rounds;
        completed_iterations += outcome.iterations;
        let row = outcome.row;
        let ipri = row.primary;
        let final_score = row.final_score;
        results.push(row.clone());
        if best_primary
            .as_ref()
            .map(|b| ipri < b.primary)
            .unwrap_or(true)
        {
            best_primary = Some(row);
        }
        let gain = outcome.start_score - final_score;
        let gain_text = if gain > 1e-9 {
            ui.green(&format!("▼{:.4}", gain))
        } else {
            ui.dim("—")
        };
        println!(
            "   {:<ils_name_w$}  {:>7.4}  →  {:>7.4}  {:>7.4}  {}",
            ui::short_name(&outcome.source_start),
            outcome.start_score,
            final_score,
            ipri,
            gain_text
        );
    }
    println!();

    if args.polish_rounds > 0 {
        let rows: Vec<Row> = results
            .iter()
            .filter(|r| !r.start.starts_with("secondary:"))
            .cloned()
            .collect();
        let ref_row = rows
            .iter()
            .min_by(|a, b| {
                a.primary
                    .partial_cmp(&b.primary)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| {
                        a.final_score
                            .partial_cmp(&b.final_score)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
            })
            .cloned()
            .unwrap();
        let (candidates, _) = diversity::select_valid_layouts(
            &rows,
            &ref_row,
            args.valid_tolerance,
            args.secondary_tolerance,
            args.min_layout_distance,
            args.polish_candidates,
        );
        let polish_progress = ui::Progress::new("Polish", 0);
        let polish_tick = || polish_progress.tick();
        let polish_outcomes: Vec<PolishOutcome> = if deadline.is_none() {
            candidates
                .par_iter()
                .enumerate()
                .map(|(i, row)| {
                    let mut rng = polish_rng(seed, i);
                    optimize_polish_row(
                        &primary_ev,
                        &delta_tables,
                        row,
                        i,
                        args.polish_rounds,
                        args.iters,
                        seed,
                        &mut rng,
                        secondary_ev.as_ref(),
                        None,
                        &polish_tick,
                    )
                })
                .collect::<Result<Vec<_>>>()?
        } else {
            let mut outcomes = Vec::new();
            for (i, row) in candidates.iter().enumerate() {
                let mut rng = polish_rng(seed, i);
                outcomes.push(optimize_polish_row(
                    &primary_ev,
                    &delta_tables,
                    row,
                    i,
                    args.polish_rounds,
                    args.iters,
                    seed,
                    &mut rng,
                    secondary_ev.as_ref(),
                    deadline,
                    &polish_tick,
                )?);
            }
            outcomes
        };
        polish_progress.finish();
        let polish_cap = if args.polish_rounds == usize::MAX {
            "unlimited rounds".to_string()
        } else {
            format!("up to {} rounds", args.polish_rounds)
        };
        println!(
            "{}",
            ui.section(
                "Polish",
                None,
                &format!(
                    "{} candidate{} · {}",
                    polish_outcomes.len(),
                    if polish_outcomes.len() == 1 { "" } else { "s" },
                    polish_cap
                )
            )
        );
        let pol_name_w = polish_outcomes
            .iter()
            .map(|o| {
                ui::short_name(o.row.start.trim_start_matches("polish:"))
                    .chars()
                    .count()
            })
            .max()
            .unwrap_or(9)
            .max(9);
        if !polish_outcomes.is_empty() {
            println!(
                "{}",
                ui.dim(&format!(
                    "   {:<pol_name_w$}  {:>7}     {:>7}  {:>7}  rounds",
                    "candidate", "before", "after", "primary"
                ))
            );
        }
        let mut polished = Vec::new();
        for outcome in polish_outcomes.into_iter() {
            completed_polish_rounds += outcome.passes;
            completed_iterations += outcome.iterations;
            let row = outcome.row;
            let status = if outcome.passes >= args.polish_rounds {
                "round limit"
            } else if outcome.start_s - row.final_score > 1e-9 {
                "converged"
            } else {
                "no improvement"
            };
            println!(
                "   {:<pol_name_w$}  {:>7.4}  →  {:>7.4}  {:>7.4}  {} {}",
                ui::short_name(row.start.trim_start_matches("polish:")),
                outcome.start_s,
                row.final_score,
                row.primary,
                outcome.passes,
                ui.dim(&format!("({status})"))
            );
            polished.push(row);
        }
        println!();
        results.extend(polished.iter().cloned());
        for row in &polished {
            if best_primary
                .as_ref()
                .map(|b| row.primary < b.primary)
                .unwrap_or(true)
            {
                best_primary = Some(row.clone());
            }
        }
    }

    let best_primary = best_primary.unwrap();
    let mut final_row = best_primary.clone();
    let mut best_secondary: Option<Row> = None;
    if let Some(secondary_ev) = secondary_ev.as_ref() {
        let score_limit = sacrifice
            .score
            .map(|pct| best_primary.primary + best_primary.primary.abs() * pct / 100.0 + 1e-12);
        let base_metrics = sacrifice.has_metrics().then(|| {
            Metrics::evaluate(
                &best_primary.layout,
                &primary_corpus,
                &ctx.tables,
                &ctx.geom,
                &ctx.cfg.finger_balance,
                ctx.cfg.redirect_tiers.as_ref(),
            )
        });
        let metric_limits = base_metrics
            .as_ref()
            .map(|base| MetricLimits::new(base, &sacrifice));
        let metric_allows = |layout: &str| -> bool {
            metric_limits
                .as_ref()
                .map(|limits| {
                    limits.allows(&Metrics::evaluate(
                        layout,
                        &primary_corpus,
                        &ctx.tables,
                        &ctx.geom,
                        &ctx.cfg.finger_balance,
                        ctx.cfg.redirect_tiers.as_ref(),
                    ))
                })
                .unwrap_or(true)
        };
        let constrained = |perm: &[usize]| -> f64 {
            if let Some(limit) = score_limit {
                if primary_ev.score(perm) > limit {
                    return f64::INFINITY;
                }
            }
            if metric_limits.is_some() && !metric_allows(&primary_ev.layout_str(perm)) {
                return f64::INFINITY;
            }
            secondary_ev.score(perm)
        };
        let mut primary_results: Vec<Row> = results
            .iter()
            .filter(|r| !r.start.starts_with("secondary:"))
            .cloned()
            .collect();
        primary_results.sort_by(|a, b| {
            a.primary
                .partial_cmp(&b.primary)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        primary_results.truncate(4);
        let score_text = score_limit
            .map(|limit| format!("score ≤ {}", fmt_sig(limit, 6)))
            .unwrap_or_else(|| "score unconstrained".into());
        println!(
            "{}",
            ui.section(
                "Secondary",
                None,
                &format!(
                    "{} · per-metric budgets {}",
                    score_text,
                    if sacrifice.has_metrics() { "on" } else { "off" }
                )
            )
        );
        let sec_name_w = primary_results
            .iter()
            .map(|r| ui::short_name(&r.start).chars().count())
            .max()
            .unwrap_or(9)
            .max(9);
        if !primary_results.is_empty() {
            println!(
                "{}",
                ui.dim(&format!(
                    "   {:<sec_name_w$}  {:>7}  {:>9}",
                    "candidate", "primary", "secondary"
                ))
            );
        }
        for r in primary_results {
            if let Some(t) = args.time_budget {
                if started.elapsed().as_secs_f64() >= t {
                    println!("{}", ui.dim("time budget reached before secondary phase"));
                    break;
                }
            }
            let perm = perm_of(&r.layout, &ctx.cfg.punctuation).map_err(anyhow::Error::msg)?;
            let cur = constrained(&perm);
            if !cur.is_finite() {
                continue;
            }
            let mut best_perm = perm.clone();
            let mut best_sec = cur;
            for _ in 0..args.ils.max(1) {
                if let Some(t) = args.time_budget {
                    if started.elapsed().as_secs_f64() >= t {
                        break;
                    }
                }
                let k = choose3(&mut rng, &[2, 4, 6]);
                let cand = primary_ev.perturb(&best_perm, &mut rng, k);
                let cand_s = constrained(&cand);
                if !cand_s.is_finite() {
                    continue;
                }
                let (cand, _) = search::quench_constrained(
                    &primary_ev,
                    secondary_ev,
                    &cand,
                    score_limit.unwrap_or(f64::INFINITY),
                    cand_s,
                    &mut rng,
                );
                let cand_s = constrained(&cand);
                if !cand_s.is_finite() {
                    continue;
                }
                if cand_s < best_sec - 1e-12 {
                    best_perm = cand;
                    best_sec = cand_s;
                }
            }
            canonical_punctuation(&mut best_perm);
            let lay = primary_ev.layout_str(&best_perm);
            let pri = primary_ev.score(&best_perm);
            let sec = secondary_ev.score(&best_perm);
            let row = Row {
                start: format!("secondary:{}", r.start),
                final_score: sec,
                primary: pri,
                secondary: Some(sec),
                layout: lay.clone(),
                reference_name: None,
            };
            results.push(row.clone());
            completed_secondary_rounds += 1;
            if best_secondary
                .as_ref()
                .map(|b| sec < b.secondary.unwrap())
                .unwrap_or(true)
            {
                best_secondary = Some(row);
            }
            println!(
                "   {:<sec_name_w$}  {:>7.4}  {:>9.4}",
                ui::short_name(&r.start),
                pri,
                sec
            );
        }
        println!();
        if let Some(ref b) = best_secondary {
            final_row = b.clone();
        }
        let eligible: Vec<Row> = results
            .iter()
            .filter(|r| {
                r.secondary.is_some()
                    && score_limit.map(|limit| r.primary <= limit).unwrap_or(true)
                    && metric_allows(&r.layout)
            })
            .cloned()
            .collect();
        if let Some(best) = eligible.into_iter().min_by(|a, b| {
            a.secondary
                .unwrap()
                .partial_cmp(&b.secondary.unwrap())
                .unwrap_or(std::cmp::Ordering::Equal)
        }) {
            final_row = best;
            best_secondary = Some(final_row.clone());
        }
    }

    let ref_row = best_primary.clone();
    let (mut valid_rows, valid_truncated) = diversity::select_valid_layouts(
        &results,
        &ref_row,
        args.valid_tolerance,
        args.secondary_tolerance,
        args.min_layout_distance,
        args.max_valid,
    );
    if !valid_rows.iter().any(|r| r.layout == final_row.layout) {
        let merged = diversity::merge_results(&results)
            .into_iter()
            .find(|r| r.layout == final_row.layout)
            .unwrap_or_else(|| final_row.clone());
        valid_rows.push(merged);
    }
    if valid_rows.is_empty() {
        valid_rows.push(final_row.clone());
    }
    valid_rows.sort_by(|a, b| {
        (a.layout != final_row.layout)
            .cmp(&(b.layout != final_row.layout))
            .then_with(|| {
                a.primary
                    .partial_cmp(&b.primary)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                a.secondary
                    .unwrap_or(f64::INFINITY)
                    .partial_cmp(&b.secondary.unwrap_or(f64::INFINITY))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    let selection_status = if valid_rows.len() == 1 {
        "clear_winner"
    } else {
        "multiple_valid"
    };
    final_row = valid_rows[0].clone();
    let latest_path = paths::write_latest_layout(&final_row.layout, &timestamp)?;

    let append_path = args
        .append_references
        .clone()
        .unwrap_or_else(|| refs_path.clone());
    let existing_refs = references::load_references(Some(&append_path)).unwrap_or_default();
    let mut existing_names: std::collections::HashMap<String, String> = existing_refs
        .iter()
        .map(|r| (r.layout.clone(), r.name.clone()))
        .collect();
    let base_name = args
        .name
        .clone()
        .unwrap_or_else(|| format!("shiro-{}", seed));
    let mut added_names = Vec::new();
    for (i, row) in valid_rows.iter_mut().enumerate() {
        let requested = if i == 0 {
            base_name.clone()
        } else {
            format!("{}-{}", base_name, i + 1)
        };
        if let Some(added) = references::append_reference(
            &requested,
            &row.layout,
            Some(row.primary),
            row.secondary,
            Some(seed as i64),
            Some(&append_path),
        )? {
            existing_names.insert(row.layout.clone(), added.clone());
            added_names.push(added.clone());
        }
        row.reference_name = existing_names.get(&row.layout).cloned();
    }
    let added_name = added_names.first().cloned();

    let mut xkb_path: Option<PathBuf> = None;
    if args.xkb {
        let name = args
            .xkb_name
            .clone()
            .unwrap_or(xkb::safe_name(&final_row.layout)?);
        let path = paths::xkb_dir().join(&name);
        xkb::write(&path, &final_row.layout, &name)?;
        xkb_path = Some(path);
    }

    results.sort_by(|a, b| {
        a.primary
            .partial_cmp(&b.primary)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                a.final_score
                    .partial_cmp(&b.final_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    let mut out_lines = vec![
        format!("seed={} generated={}", seed, seed_generated),
        format!(
            "primary={} secondary={}",
            primary_paths.join(","),
            secondary_paths.join(",")
        ),
        format!("references={}", refs_path.display()),
        format!("append_references={}", append_path.display()),
        format!("reference_added={}", added_name.clone().unwrap_or_default()),
        format!("references_added={}", added_names.join(",")),
        format!(
            "primary_sacrifice_pct={}",
            sacrifice
                .score
                .map(|pct| fmt_sig(pct, 6))
                .unwrap_or_else(|| "none".into())
        ),
        format!(
            "selection={} valid_layouts_truncated={}",
            selection_status, valid_truncated
        ),
        format!(
            "completed_runs={} completed_iterations={} completed_ils_rounds={} completed_secondary_rounds={}",
            completed_runs, completed_iterations, completed_ils_rounds, completed_secondary_rounds
        ),
        format!(
            "completed_batches={} stop_reason={}",
            completed_batches, stop_reason
        ),
        format!(
            "best_primary: primary={:.9} {}",
            best_primary.primary, best_primary.layout
        ),
        match best_secondary.as_ref() {
            Some(b) => format!(
                "best_secondary: primary={:.9} secondary={:.9} {}",
                b.primary,
                b.secondary.unwrap(),
                b.layout
            ),
            None => "best_secondary: none".to_string(),
        },
        format!(
            "selected: primary={:.9} secondary={:.9} {}",
            final_row.primary,
            final_row.secondary.unwrap_or(f64::NAN),
            final_row.layout
        ),
    ];
    out_lines.extend(diversity::format_valid_layouts(
        &valid_rows,
        selection_status,
        valid_truncated,
    ));
    out_lines.push(String::new());
    out_lines.push(format!(
        "{:<24} {:>10} {:>10}  layout",
        "start", "primary", "secondary"
    ));
    for r in &results {
        out_lines.push(format!(
            "{:<24} {:>10.6} {:>10.6}  {}",
            r.start,
            r.primary,
            r.secondary.unwrap_or(f64::NAN),
            r.layout
        ));
    }
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&out_path, out_lines.join("\n") + "\n")?;
    if let Some(parent) = out_json.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let payload = json!({
        "seed": seed,
        "seed_generated": seed_generated,
        "primary_corpora": primary_paths,
        "secondary_corpora": secondary_paths,
        "references": refs_path,
        "primary_sacrifice_pct": sacrifice.score,
        "primary_sacrifice_metrics": {
            "sfb_sft": sacrifice.sfb_sft,
            "scissors": sacrifice.scissors,
            "redirects": sacrifice.redirects,
            "rolls": sacrifice.rolls,
            "alternation": sacrifice.alternation,
            "finger_balance": sacrifice.finger_balance,
        },
        "elapsed_seconds": started.elapsed().as_secs_f64(),
        "time_budget_seconds": args.time_budget,
        "completed_runs": completed_runs,
        "completed_batches": completed_batches,
        "stop_reason": stop_reason,
        "completed_iterations": completed_iterations,
        "completed_ils_rounds": completed_ils_rounds,
        "completed_secondary_rounds": completed_secondary_rounds,
        "completed_polish_rounds": completed_polish_rounds,
        "runs": args.runs, "iters": args.iters, "ils": args.ils,
        "ils_candidates": args.ils_candidates,
        "batch_size": args.batch_size,
        "stop_hits": args.stop_hits,
        "stop_stale_batches": args.stop_stale_batches,
        "polish_rounds": if args.polish_rounds == usize::MAX { serde_json::Value::Null } else { serde_json::json!(args.polish_rounds) },
        "polish_candidates": args.polish_candidates,
        "valid_tolerance": args.valid_tolerance,
        "secondary_tolerance": args.secondary_tolerance,
        "min_layout_distance": args.min_layout_distance,
        "max_valid": args.max_valid,
        "results": results.iter().map(|r| json!({
            "start": r.start, "final": r.final_score, "primary": r.primary,
            "secondary": r.secondary, "layout": r.layout,
            "reference_name": r.reference_name,
        })).collect::<Vec<_>>(),
        "best_primary": {"primary": best_primary.primary, "layout": best_primary.layout},
        "best_secondary": best_secondary.as_ref().map(|b| json!({
            "primary": b.primary, "secondary": b.secondary, "layout": b.layout
        })),
        "selected": {"primary": final_row.primary, "secondary": final_row.secondary, "layout": final_row.layout},
        "selection_status": selection_status,
        "valid_layouts": valid_rows.iter().map(|r| json!({
            "name": r.reference_name, "start": r.start, "layout": r.layout,
            "primary": r.primary, "secondary": r.secondary
        })).collect::<Vec<_>>(),
        "valid_layouts_truncated": valid_truncated,
        "reference_added": added_name,
        "references_added": added_names,
        "append_references": append_path,
        "xkb": xkb_path,
    });
    std::fs::write(&out_json, serde_json::to_string_pretty(&payload)? + "\n")?;
    print_summary(
        &ui,
        &final_row,
        &best_primary,
        &valid_rows,
        valid_truncated,
        selection_status,
    );
    let rel = |p: &Path| -> String {
        p.strip_prefix(paths::repo_root())
            .unwrap_or(p)
            .display()
            .to_string()
    };
    let label = |name: &str| ui.dim(&format!(" {:<7}", name));
    let saved = if out_path.with_extension("txt") == out_path
        && out_json.with_extension("json") == out_json
        && out_path.with_extension("") == out_json.with_extension("")
    {
        format!("{}.{{txt,json}}", rel(&out_path.with_extension("")))
    } else {
        format!("{}, {}", rel(&out_path), rel(&out_json))
    };
    println!();
    println!("{} {}", label("saved"), saved);
    println!("{} {}", label("latest"), rel(&latest_path));
    if !added_names.is_empty() {
        println!(
            "{} +{} → {}",
            label("refs"),
            added_names.join(", +"),
            rel(&append_path)
        );
    }
    if let Some(path) = xkb_path.as_ref() {
        println!("{} {}", label("xkb"), rel(path));
    }
    println!(
        "{} {}",
        label("done"),
        ui.bold(&format!("{:.0}s", started.elapsed().as_secs_f64()))
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(start: &str, layout: &str) -> Row {
        Row::new(start, 1.0, None, layout)
    }

    #[test]
    fn rediscovery_counts_random_letter_matches_only() {
        let results = vec![
            row("rand1", "abcdefghijklmnopqrstuvwxyz;,./"),
            row("rand2", "abcdefghijklmnopqrstuvwxyz,./;"),
            row("user0", "abcdefghijklmnopqrstuvwxyz./,;"),
            row("rand3", "bacdefghijklmnopqrstuvwxyz;,./"),
        ];
        assert_eq!(
            count_rediscoveries(&results, "abcdefghijklmnopqrstuvwxyz;,./"),
            2
        );
    }

    #[test]
    fn stale_batches_reset_on_improvement() {
        assert_eq!(bump_stale(None, 3.5, 0), 0);
        assert_eq!(bump_stale(Some(3.5), 3.5, 0), 1);
        assert_eq!(bump_stale(Some(3.5), 3.5, 2), 3);
        assert_eq!(bump_stale(Some(3.5), 3.4, 2), 0);
    }
}
