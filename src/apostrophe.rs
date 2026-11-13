//! Apostrophe n-gram scoring, punctuation placement, and letter-swap search.

use std::path::Path;

use anyhow::{anyhow, Result};
use serde_json::json;

use crate::config::ApostropheConfig;
use crate::context;
use crate::evaluate::{flat_share_from_config, perm_of, Evaluator, LETTERS, N_ITEMS, N_LETTERS};
use crate::geometry::KEYS;
use crate::paths;
use crate::scorer::{big_idx, tri_idx, Tables};

#[derive(Clone, Debug)]
struct ResolvedNgram {
    items: Vec<usize>,
    weight: f64,
}

#[derive(Clone, Debug)]
pub struct ApostropheTerm {
    weight: f64,
    ngrams: Vec<ResolvedNgram>,
    apostrophe_item: usize,
}

impl ApostropheTerm {
    pub fn from_config(cfg: &ApostropheConfig, punctuation: &[char]) -> Result<Self, String> {
        let apostrophe_index = punctuation
            .iter()
            .position(|&ch| ch == '\'')
            .ok_or_else(|| "apostrophe n-grams require `'` in general.punctuation".to_string())?;
        let apostrophe_item = N_LETTERS + apostrophe_index;
        let mut ngrams = Vec::with_capacity(cfg.ngrams.len());
        for ngram in &cfg.ngrams {
            let mut items = Vec::with_capacity(ngram.text.chars().count());
            for ch in ngram.text.chars() {
                if ch == '\'' {
                    items.push(apostrophe_item);
                } else if ch.is_ascii_lowercase() {
                    items.push((ch as u8 - b'a') as usize);
                } else {
                    return Err(format!("invalid apostrophe n-gram {:?}", ngram.text));
                }
            }
            if !(2..=3).contains(&items.len()) {
                return Err(format!(
                    "apostrophe n-gram {:?} must have 2 or 3 characters",
                    ngram.text
                ));
            }
            ngrams.push(ResolvedNgram {
                items,
                weight: ngram.weight,
            });
        }
        if ngrams.is_empty() {
            return Err("apostrophe n-grams are empty".to_string());
        }
        Ok(Self {
            weight: cfg.weight,
            ngrams,
            apostrophe_item,
        })
    }

    pub fn weight(&self) -> f64 {
        self.weight
    }

    pub fn apostrophe_item(&self) -> usize {
        self.apostrophe_item
    }

    /// Average n-gram cost, before applying the configured overall weight.
    pub fn raw_cost(&self, perm: &[usize], tables: &Tables) -> f64 {
        let mut total = 0.0;
        let mut norm = 0.0;
        for ngram in &self.ngrams {
            let cost = match ngram.items.as_slice() {
                [a, b] => tables.big[big_idx(perm[*a], perm[*b])],
                [a, b, c] => tables.tri[tri_idx(perm[*a], perm[*b], perm[*c])],
                _ => continue,
            };
            total += cost * ngram.weight;
            norm += ngram.weight;
        }
        if norm > 0.0 {
            total / norm
        } else {
            0.0
        }
    }

    /// Weighted apostrophe term, suitable for adding to another score.
    pub fn cost(&self, perm: &[usize], tables: &Tables) -> f64 {
        self.weight * self.raw_cost(perm, tables)
    }

    /// All assignments of the punctuation items to the current punctuation
    /// slots, paired with their raw apostrophe cost.  The first option is the
    /// canonical left-to-right assignment.
    pub fn punctuation_options(&self, perm: &[usize], tables: &Tables) -> Vec<(Vec<usize>, f64)> {
        let mut slots: Vec<usize> = perm[N_LETTERS..].to_vec();
        slots.sort_unstable();
        let mut items: Vec<usize> = (N_LETTERS..N_ITEMS).collect();
        let mut options = Vec::new();
        permute(&mut items, 0, &mut |assignment| {
            let mut candidate = perm.to_vec();
            for (i, &item) in assignment.iter().enumerate() {
                candidate[item] = slots[i];
            }
            let cost = self.raw_cost(&candidate, tables);
            options.push((candidate, cost));
        });
        options
    }

    /// Put the punctuation items on their cheapest slots.  Letters are not
    /// moved.  Returns the raw apostrophe cost of the chosen placement.
    pub fn optimize_punctuation(&self, perm: &mut [usize], tables: &Tables) -> f64 {
        let options = self.punctuation_options(perm, tables);
        let (best, cost) = options
            .into_iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .expect("punctuation options are never empty");
        perm.copy_from_slice(&best);
        cost
    }

    pub fn apostrophe_key(&self, perm: &[usize]) -> &'static str {
        KEYS[perm[self.apostrophe_item]]
    }
}

fn permute<T>(items: &mut [T], start: usize, f: &mut impl FnMut(&[T])) {
    if start == items.len() {
        f(items);
        return;
    }
    for i in start..items.len() {
        items.swap(start, i);
        permute(items, start + 1, f);
        items.swap(start, i);
    }
}

#[derive(Clone, Debug)]
pub struct SwapSuggestion {
    pub a: usize,
    pub b: usize,
    pub a_char: char,
    pub b_char: char,
    pub layout: String,
    pub primary: f64,
    pub primary_delta: f64,
    pub apostrophe_avg: f64,
    pub apostrophe_delta: f64,
}

#[allow(clippy::too_many_arguments)]
fn suggestion(
    a: usize,
    b: usize,
    a_char: char,
    b_char: char,
    candidate: &[usize],
    ev: &Evaluator<'_>,
    primary: f64,
    base_primary: f64,
    apostrophe_avg: f64,
    previous_apostrophe: f64,
) -> SwapSuggestion {
    SwapSuggestion {
        a,
        b,
        a_char,
        b_char,
        layout: ev.layout_str(candidate),
        primary,
        primary_delta: primary - base_primary,
        apostrophe_avg,
        apostrophe_delta: apostrophe_avg - previous_apostrophe,
    }
}

/// Find single letter swaps that improve the apostrophe term while keeping
/// the primary score within `tolerance` of the base layout.
pub fn search_swaps(
    base_perm: &[usize],
    ev: &Evaluator<'_>,
    term: &ApostropheTerm,
    tables: &Tables,
    tolerance: f64,
    include_apostrophe_to_letter: bool,
) -> Vec<SwapSuggestion> {
    let base_primary = ev.score(base_perm);
    let base_cost = term.raw_cost(base_perm, tables);
    let mut out = Vec::new();
    for a in 0..N_LETTERS {
        for b in (a + 1)..N_LETTERS {
            if !ev.swap_allowed(base_perm, a, b) {
                continue;
            }
            let mut candidate = base_perm.to_vec();
            candidate.swap(a, b);
            let primary = ev.score(&candidate);
            if primary > base_primary + tolerance {
                continue;
            }
            let cost = term.optimize_punctuation(&mut candidate, tables);
            if cost < base_cost - 1e-12 {
                out.push(suggestion(
                    a,
                    b,
                    LETTERS.as_bytes()[a] as char,
                    LETTERS.as_bytes()[b] as char,
                    &candidate,
                    ev,
                    primary,
                    base_primary,
                    cost,
                    base_cost,
                ));
            }
        }
    }
    if include_apostrophe_to_letter {
        let apostrophe = term.apostrophe_item();
        for a in 0..N_LETTERS {
            if !ev.swap_allowed(base_perm, a, apostrophe) {
                continue;
            }
            let mut candidate = base_perm.to_vec();
            candidate.swap(a, apostrophe);
            let primary = ev.score(&candidate);
            if primary > base_primary + tolerance {
                continue;
            }
            let cost = term.optimize_punctuation(&mut candidate, tables);
            if cost < base_cost - 1e-12 {
                out.push(suggestion(
                    a,
                    apostrophe,
                    LETTERS.as_bytes()[a] as char,
                    '\'',
                    &candidate,
                    ev,
                    primary,
                    base_primary,
                    cost,
                    base_cost,
                ));
            }
        }
    }
    out.sort_by(|a, b| {
        a.apostrophe_avg
            .partial_cmp(&b.apostrophe_avg)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                a.primary_delta
                    .partial_cmp(&b.primary_delta)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    out
}

#[derive(Clone, Debug)]
pub struct GreedyResult {
    pub perm: Vec<usize>,
    pub primary: f64,
    pub apostrophe_avg: f64,
    pub swaps: Vec<SwapSuggestion>,
}

/// Repeatedly apply the best single letter swap until no swap improves the
/// apostrophe term.  Every step must stay within `tolerance` of the original
/// primary score.
pub fn greedy_refine(
    base_perm: &[usize],
    ev: &Evaluator<'_>,
    term: &ApostropheTerm,
    tables: &Tables,
    tolerance: f64,
    max_steps: usize,
) -> GreedyResult {
    let base_primary = ev.score(base_perm);
    let mut current = base_perm.to_vec();
    let mut current_primary = base_primary;
    let mut current_cost = term.raw_cost(&current, tables);
    let mut swaps = Vec::new();
    for _ in 0..max_steps {
        let mut best: Option<(usize, usize, f64, f64, Vec<usize>)> = None;
        for a in 0..N_LETTERS {
            for b in (a + 1)..N_LETTERS {
                if !ev.swap_allowed(&current, a, b) {
                    continue;
                }
                let mut candidate = current.clone();
                candidate.swap(a, b);
                let primary = ev.score(&candidate);
                if primary > base_primary + tolerance {
                    continue;
                }
                let cost = term.optimize_punctuation(&mut candidate, tables);
                if cost < current_cost - 1e-12
                    && best
                        .as_ref()
                        .is_none_or(|(_, _, _, best_cost, _)| cost < *best_cost)
                {
                    best = Some((a, b, primary, cost, candidate));
                }
            }
        }
        let Some((a, b, primary, cost, candidate)) = best else {
            break;
        };
        swaps.push(suggestion(
            a,
            b,
            LETTERS.as_bytes()[a] as char,
            LETTERS.as_bytes()[b] as char,
            &candidate,
            ev,
            primary,
            base_primary,
            cost,
            current_cost,
        ));
        current = candidate;
        current_primary = primary;
        current_cost = cost;
    }
    GreedyResult {
        perm: current,
        primary: current_primary,
        apostrophe_avg: current_cost,
        swaps,
    }
}

pub struct ApostropheArgs<'a> {
    pub layout: Option<&'a str>,
    pub config: Option<&'a Path>,
    pub primary_tolerance: f64,
    pub top: usize,
    pub apostrophe_to_letter: bool,
    pub out: Option<&'a Path>,
    pub out_json: Option<&'a Path>,
}

pub fn run(args: ApostropheArgs<'_>) -> Result<()> {
    let ctx = context::Context::load(args.config)?;
    let latest;
    let layout = match args.layout {
        Some(layout) => layout,
        None => {
            latest = paths::read_latest_layout()?;
            &latest
        }
    };
    let apostrophe_config = ctx
        .cfg
        .apostrophe
        .as_ref()
        .ok_or_else(|| anyhow!("config has no `apostrophe` section"))?;
    let term = ApostropheTerm::from_config(apostrophe_config, &ctx.cfg.punctuation)
        .map_err(anyhow::Error::msg)?;
    let (corpus, labels) = context::load_role(&ctx.cfg, "primary")?;
    let flat_share = flat_share_from_config(&ctx.cfg);
    let (w_balance, w_hand) = ctx.cfg.balance_weights();
    let mut ev = Evaluator::new(
        &corpus,
        &ctx.tables,
        &ctx.model,
        &ctx.cfg.punctuation,
        flat_share,
        w_balance,
        w_hand,
    )
    .map_err(anyhow::Error::msg)?;
    ev.set_hand_restrictions(ctx.cfg.hand_restrictions);
    ev.set_balance_source(ctx.cfg.balance_source);

    let perm = perm_of(layout, &ctx.cfg.punctuation).map_err(anyhow::Error::msg)?;
    let original_primary = ev.score(&perm);
    let original_cost = term.raw_cost(&perm, &ctx.tables);

    let mut options = term.punctuation_options(&perm, &ctx.tables);
    options.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut base_perm = options
        .first()
        .map(|(candidate, _)| candidate.clone())
        .unwrap_or_else(|| perm.clone());
    let base_cost = term.optimize_punctuation(&mut base_perm, &ctx.tables);
    let base_primary = ev.score(&base_perm);

    let swaps = search_swaps(
        &base_perm,
        &ev,
        &term,
        &ctx.tables,
        args.primary_tolerance,
        args.apostrophe_to_letter,
    );
    let greedy = greedy_refine(
        &base_perm,
        &ev,
        &term,
        &ctx.tables,
        args.primary_tolerance,
        32,
    );

    let mut lines = vec![
        format!("layout: {}", layout),
        format!("corpus: {}", labels.join(",")),
        format!(
            "apostrophe: weight {:.4}, {} n-grams, punctuation {:?}",
            term.weight(),
            apostrophe_config.ngrams.len(),
            ctx.cfg.punctuation.iter().collect::<String>()
        ),
        format!(
            "original: primary {:.6}, apostrophe avg {:.6}, apostrophe on {}",
            original_primary,
            original_cost,
            term.apostrophe_key(&perm)
        ),
        format!(
            "best punctuation: primary {:.6}, apostrophe avg {:.6}, apostrophe on {}",
            base_primary,
            base_cost,
            term.apostrophe_key(&base_perm)
        ),
        format!("best layout: {}", ev.layout_str(&base_perm)),
        String::new(),
        "== punctuation placements (best first) ==".to_string(),
    ];
    for (i, (candidate, cost)) in options.iter().enumerate() {
        lines.push(format!(
            "{:>2}. avg {:.6}  key {:<2}  {}",
            i + 1,
            cost,
            term.apostrophe_key(candidate),
            ev.layout_str(candidate)
        ));
    }
    lines.push(String::new());
    lines.push(format!(
        "== letter swaps within primary +{:.6} ==",
        args.primary_tolerance
    ));
    if swaps.is_empty() {
        lines.push("none".to_string());
    } else {
        for swap in swaps.iter().take(args.top) {
            lines.push(format!(
                "{} <-> {}  primary {:+.6}  apostrophe {:+.6}  {}",
                swap.a_char, swap.b_char, swap.primary_delta, swap.apostrophe_delta, swap.layout
            ));
        }
    }
    lines.push(String::new());
    lines.push(format!("== greedy chain ({} swaps) ==", greedy.swaps.len()));
    for swap in &greedy.swaps {
        lines.push(format!(
            "{} <-> {}  primary {:+.6}  apostrophe {:+.6}  {}",
            swap.a_char, swap.b_char, swap.primary_delta, swap.apostrophe_delta, swap.layout
        ));
    }
    if greedy.swaps.is_empty() {
        lines.push("none".to_string());
    }
    lines.push(format!(
        "final: primary {:.6}, apostrophe avg {:.6}, apostrophe on {}",
        greedy.primary,
        greedy.apostrophe_avg,
        term.apostrophe_key(&greedy.perm)
    ));
    lines.push(format!("final layout: {}", ev.layout_str(&greedy.perm)));

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
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let placements: Vec<_> = options
            .iter()
            .map(|(candidate, cost)| {
                json!({
                    "layout": ev.layout_str(candidate),
                    "apostrophe_key": term.apostrophe_key(candidate),
                    "apostrophe_avg": cost,
                    "apostrophe_weighted": term.weight() * cost,
                })
            })
            .collect();
        let swaps_json: Vec<_> = swaps
            .iter()
            .map(|swap| {
                json!({
                    "a": swap.a_char.to_string(),
                    "b": swap.b_char.to_string(),
                    "layout": swap.layout,
                    "primary": swap.primary,
                    "primary_delta": swap.primary_delta,
                    "apostrophe_avg": swap.apostrophe_avg,
                    "apostrophe_delta": swap.apostrophe_delta,
                })
            })
            .collect();
        let greedy_json: Vec<_> = greedy
            .swaps
            .iter()
            .map(|swap| {
                json!({
                    "a": swap.a_char.to_string(),
                    "b": swap.b_char.to_string(),
                    "layout": swap.layout,
                    "primary": swap.primary,
                    "primary_delta": swap.primary_delta,
                    "apostrophe_avg": swap.apostrophe_avg,
                    "apostrophe_delta": swap.apostrophe_delta,
                })
            })
            .collect();
        let value = json!({
            "layout": layout,
            "corpus": labels,
            "apostrophe_weight": term.weight(),
            "ngrams": apostrophe_config.ngrams.iter().map(|ngram| json!({
                "text": ngram.text,
                "weight": ngram.weight,
            })).collect::<Vec<_>>(),
            "original": {
                "primary": original_primary,
                "apostrophe_avg": original_cost,
                "apostrophe_key": term.apostrophe_key(&perm),
            },
            "base": {
                "layout": ev.layout_str(&base_perm),
                "primary": base_primary,
                "apostrophe_avg": base_cost,
                "apostrophe_key": term.apostrophe_key(&base_perm),
            },
            "punctuation_placements": placements,
            "letter_swaps": swaps_json,
            "greedy": {
                "layout": ev.layout_str(&greedy.perm),
                "primary": greedy.primary,
                "apostrophe_avg": greedy.apostrophe_avg,
                "apostrophe_key": term.apostrophe_key(&greedy.perm),
                "swaps": greedy_json,
            },
        });
        std::fs::write(path, serde_json::to_string_pretty(&value)? + "\n")?;
        println!("wrote {}", path.display());
    }
    Ok(())
}
