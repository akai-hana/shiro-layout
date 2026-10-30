//! `compute`: generate a layout with the calibrated pattern model.
//!
//! Port of `shirolyzer/commands/compute.py`.

use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;
use serde_json::json;

use crate::context::{self, Context};
use crate::diversity::{self, Row};
use crate::evaluate::{dead_of, flat_share_from_config, perm_of, Evaluator};
use crate::paths;
use crate::references;
use crate::search;
use crate::xkb;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Device {
    Cpu,
    Gpu,
}

pub struct ComputeArgs<'a> {
    pub config: Option<&'a Path>,
    pub seed_layout: Vec<String>,
    pub name: Option<String>,
    pub primary_corpus: Vec<String>,
    pub secondary_corpus: Vec<String>,
    pub references: Option<PathBuf>,
    pub primary_sacrifice: Option<f64>,
    pub runs: usize,
    pub iters: usize,
    pub ils: usize,
    pub valid_tolerance: f64,
    pub secondary_tolerance: f64,
    pub min_layout_distance: usize,
    pub max_valid: usize,
    pub polish_rounds: usize,
    pub polish_candidates: usize,
    pub append_references: Option<PathBuf>,
    pub seed: Option<u64>,
    pub time_budget: Option<f64>,
    pub device: Device,
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
) -> Result<IlsOutcome> {
    let mut best = perm_of(&row.layout, &[]).map_err(anyhow::Error::msg)?;
    let mut best_s = row.final_score;
    let mut rounds = 0usize;
    let mut iterations = 0usize;
    for it in 0..ils.max(1) {
        if deadline.is_some_and(|d| Instant::now() >= d) {
            break;
        }
        let k = choose3(rng, &[4, 6, 8]);
        let cand = search::perturb(&best, rng, k);
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
        if cs < best_s - 1e-12 {
            best = cand;
            best_s = cs;
        }
    }
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
) -> Result<PolishOutcome> {
    let mut best = perm_of(&row.layout, &[]).map_err(anyhow::Error::msg)?;
    let mut best_s = ev.score(&best);
    let start_s = best_s;
    let mut passes = 0usize;
    let mut iterations = 0usize;
    for round_idx in 0..polish_rounds.max(1) {
        if deadline.is_some_and(|d| Instant::now() >= d) {
            break;
        }
        let k = choose3(rng, &[4, 6, 8]);
        let mut cand = search::perturb(&best, rng, k);
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
        if cs < best_s - 1e-12 {
            best = cand;
            best_s = cs;
        } else {
            break;
        }
    }
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

pub fn run(args: ComputeArgs) -> Result<()> {
    let device = match args.device {
        Device::Cpu => "cpu",
        Device::Gpu => {
            eprintln!("warning: GPU mode is not enabled in this build; falling back to CPU");
            "cpu"
        }
    };
    let ctx = Context::load(args.config)?;
    let out_path = args
        .out
        .clone()
        .unwrap_or_else(|| paths::result_path("compute", args.name.as_deref(), ".txt"));
    let out_json = args
        .out_json
        .clone()
        .unwrap_or_else(|| paths::result_path("compute", args.name.as_deref(), ".json"));
    let primary_sacrifice = args.primary_sacrifice.unwrap_or(ctx.cfg.primary_sacrifice);
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
    println!(
        "seed={}{}",
        seed,
        if seed_generated { " (generated)" } else { "" }
    );
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

    let base_layout = args
        .seed_layout
        .first()
        .cloned()
        .unwrap_or_else(|| ctx.cfg.current_layout.clone());
    if base_layout.chars().count() != 30 {
        return Err(anyhow!("layout must be 30 characters"));
    }
    let dead = dead_of(&base_layout);
    let flat_share = flat_share_from_config(&ctx.cfg);
    let (w_balance, w_hand) = ctx.cfg.balance_weights();
    let primary_ev = Evaluator::new(
        &primary_corpus,
        &ctx.tables,
        &ctx.model,
        &dead,
        flat_share,
        w_balance,
        w_hand,
    )
    .map_err(anyhow::Error::msg)?;
    let delta_tables = search::DeltaTables::build(&primary_ev);
    let secondary_ev = match secondary_corpus.as_ref() {
        Some(corpus) => Some(
            Evaluator::new(
                corpus,
                &ctx.tables,
                &ctx.model,
                &dead,
                flat_share,
                w_balance,
                w_hand,
            )
            .map_err(anyhow::Error::msg)?,
        ),
        None => None,
    };

    let refs_path = args
        .references
        .clone()
        .unwrap_or_else(paths::references_file);
    let mut seeds: Vec<(String, Vec<usize>)> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for entry in references::load_references(Some(&refs_path))? {
        if seen.insert(entry.layout.clone()) {
            let perm = perm_of(&entry.layout, &dead).map_err(anyhow::Error::msg)?;
            seeds.push((entry.name, perm));
        }
    }
    for (i, layout) in args.seed_layout.iter().enumerate() {
        if layout.chars().count() == 30 && seen.insert(layout.clone()) {
            let perm = perm_of(layout, &dead).map_err(anyhow::Error::msg)?;
            seeds.push((format!("user{}", i), perm));
        }
    }
    for s in 0..args.runs {
        let mut perm: Vec<usize> = (0..26).collect();
        let mut start_rng = ChaCha8Rng::seed_from_u64(seed + 1000 + s as u64);
        perm.shuffle(&mut start_rng);
        seeds.push((format!("rand{}", seed + 1000 + s as u64), perm));
    }

    let mut results: Vec<Row> = Vec::new();
    let mut best_primary: Option<Row> = None;
    println!("{:<18} {:>9} {:>9}  layout", "start", "final", "primary");
    let parallel = args.time_budget.is_none() && seeds.len() > 1;
    let mut seed_rows: Vec<Row> = if parallel {
        seeds
            .par_iter()
            .enumerate()
            .map(|(index, (name, start))| {
                let mut rng = seed_rng(seed, index);
                optimize_seed(
                    &primary_ev,
                    &delta_tables,
                    &mut rng,
                    name,
                    start,
                    args.iters,
                    seed_anneal(seed, index),
                )
            })
            .collect()
    } else {
        let mut rows = Vec::with_capacity(seeds.len());
        for (index, (name, start)) in seeds.iter().enumerate() {
            if let Some(budget) = args.time_budget {
                if !rows.is_empty() && started.elapsed().as_secs_f64() >= budget {
                    println!("time budget reached; saving best so far");
                    break;
                }
            }
            let mut rng = seed_rng(seed, index);
            rows.push(optimize_seed(
                &primary_ev,
                &delta_tables,
                &mut rng,
                name,
                start,
                args.iters,
                seed_anneal(seed, index),
            ));
        }
        rows
    };
    for row in seed_rows.drain(..) {
        completed_runs += 1;
        completed_iterations += args.iters;
        println!(
            "{:<18} {:>9.4} {:>9.4}  {}",
            row.start, row.final_score, row.primary, row.layout
        );
        if best_primary
            .as_ref()
            .map(|b| row.primary < b.primary)
            .unwrap_or(true)
        {
            best_primary = Some(row.clone());
        }
        results.push(row);
    }
    if results.is_empty() {
        return Err(anyhow!("no completed runs"));
    }

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
    top.truncate(4);
    println!("\nILS (primary):");
    let deadline = args
        .time_budget
        .map(|budget| started + std::time::Duration::from_secs_f64(budget));
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
                )
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        let mut outcomes = Vec::new();
        for (rank, r) in top.iter().enumerate() {
            if deadline.is_some_and(|d| Instant::now() >= d) {
                println!("time budget reached during ILS; saving best so far");
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
            )?);
        }
        outcomes
    };
    for (rank, outcome) in ils_outcomes.into_iter().enumerate() {
        completed_ils_rounds += outcome.rounds;
        completed_iterations += outcome.iterations;
        let row = outcome.row;
        let ilay = row.layout.clone();
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
        println!(
            "  ILS{} {:<16} {:.4} -> {:.4} primary={:.4}  {}",
            rank + 1,
            outcome.source_start,
            outcome.start_score,
            final_score,
            ipri,
            ilay
        );
    }

    if args.polish_rounds > 0 {
        println!("\npolish (primary):");
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
                )?);
            }
            outcomes
        };
        let mut polished = Vec::new();
        for (i, outcome) in polish_outcomes.into_iter().enumerate() {
            completed_polish_rounds += outcome.passes;
            completed_iterations += outcome.iterations;
            let row = outcome.row;
            println!(
                "  polish{} {:<16} {:.4} -> {:.4} primary={:.4} rounds={}  {}",
                i + 1,
                row.start.trim_start_matches("polish:"),
                outcome.start_s,
                row.final_score,
                row.primary,
                outcome.passes,
                row.layout
            );
            polished.push(row);
        }
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
        let budget = best_primary.primary.abs() * primary_sacrifice / 100.0;
        let limit = best_primary.primary + budget + 1e-12;
        let constrained = |perm: &[usize]| -> f64 {
            if primary_ev.score(perm) > limit {
                f64::INFINITY
            } else {
                secondary_ev.score(perm)
            }
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
        println!(
            "\nsecondary phase (sacrifice {}% = {}):",
            fmt_sig(primary_sacrifice, 6),
            fmt_sig(budget, 3)
        );
        for r in primary_results {
            if let Some(t) = args.time_budget {
                if started.elapsed().as_secs_f64() >= t {
                    println!("time budget reached before secondary phase");
                    break;
                }
            }
            let perm = perm_of(&r.layout, &dead).map_err(anyhow::Error::msg)?;
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
                let cand = search::perturb(&best_perm, &mut rng, k);
                let cand_s = constrained(&cand);
                if !cand_s.is_finite() {
                    continue;
                }
                let (cand, cand_s) = search::quench_constrained(
                    &primary_ev,
                    secondary_ev,
                    &cand,
                    limit,
                    cand_s,
                    &mut rng,
                );
                if cand_s < best_sec - 1e-12 {
                    best_perm = cand;
                    best_sec = cand_s;
                }
            }
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
                "  {:<16} primary={:.6} secondary={:.6}  {}",
                r.start, pri, sec, lay
            );
        }
        if let Some(ref b) = best_secondary {
            final_row = b.clone();
        }
        let budget = best_primary.primary.abs() * primary_sacrifice / 100.0;
        let limit = best_primary.primary + budget + 1e-12;
        let eligible: Vec<Row> = results
            .iter()
            .filter(|r| r.secondary.is_some() && r.primary <= limit)
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
            println!(
                "appended reference {:?} to {}",
                added,
                append_path.display()
            );
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
        println!("wrote {}", path.display());
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
        format!("primary_sacrifice_pct={}", fmt_sig(primary_sacrifice, 6)),
        format!(
            "selection={} valid_layouts_truncated={}",
            selection_status, valid_truncated
        ),
        format!(
            "completed_runs={} completed_iterations={} completed_ils_rounds={} completed_secondary_rounds={}",
            completed_runs, completed_iterations, completed_ils_rounds, completed_secondary_rounds
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
        "device": device,
        "seed": seed,
        "seed_generated": seed_generated,
        "primary_corpora": primary_paths,
        "secondary_corpora": secondary_paths,
        "references": refs_path,
        "primary_sacrifice_pct": primary_sacrifice,
        "elapsed_seconds": started.elapsed().as_secs_f64(),
        "time_budget_seconds": args.time_budget,
        "completed_runs": completed_runs,
        "completed_iterations": completed_iterations,
        "completed_ils_rounds": completed_ils_rounds,
        "completed_secondary_rounds": completed_secondary_rounds,
        "completed_polish_rounds": completed_polish_rounds,
        "runs": args.runs, "iters": args.iters, "ils": args.ils,
        "polish_rounds": args.polish_rounds,
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
    println!(
        "\nbest_primary: {} ({:.6})",
        best_primary.layout, best_primary.primary
    );
    println!("selected: {} ({})", final_row.layout, selection_status);
    for line in diversity::format_valid_layouts(&valid_rows, selection_status, valid_truncated) {
        println!("{}", line);
    }
    println!(
        "wrote {} and {} ({:.0}s)",
        out_path.display(),
        out_json.display(),
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
