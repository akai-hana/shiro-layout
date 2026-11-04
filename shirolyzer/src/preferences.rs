//! Preference metrics used to compare layout and model variants.
//!
//! This report is observational: it never feeds back into scoring.  It exposes
//! the frequency-weighted details behind the aggregate analyzer numbers so
//! calibration work can compare variants without ad hoc scripts.

use std::collections::HashMap;

use serde_json::{json, Value};

use crate::analyze::trigram_pattern;
use crate::corpus::{Corpus, SPACE_INDEX};
use crate::geometry::{finger_hand, finger_kind, finger_token, key_index, KEYS, NF};
use crate::model::{pair_feature, pt_idx, Model};
use crate::redirect::{pattern_label, redirect_cost, RedirectTiers};
use crate::roll::{pair_for, pair_key, pair_label, ALL_PAIRS};
use crate::scorer::{big_idx, tri_idx, Tables, FINGER_SLOTS};

pub const TOP_N: usize = 25;
pub const SCISSOR_BANDS: [&str; 4] = ["mild", "moderate", "severe", "extreme"];
pub const SCISSOR_BAND_THRESHOLDS: [f64; 3] = [3.0, 5.0, 8.0];
const FORDER: [usize; 4] = [0, 1, 2, 3];

type KeyMap = [Option<usize>; 27];

#[derive(Clone, Copy, Debug, Default)]
pub struct Metric {
    pub count: f64,
    pub cost: f64,
}

impl Metric {
    fn add(&mut self, count: f64, cost: f64) {
        self.count += count;
        self.cost += cost;
    }

    fn share(&self, total: f64) -> f64 {
        if total > 0.0 {
            self.count / total * 100.0
        } else {
            0.0
        }
    }

    fn weighted_cost(&self, total: f64) -> f64 {
        if total > 0.0 {
            self.cost / total
        } else {
            0.0
        }
    }

    fn to_json(self, total: f64) -> Value {
        json!({
            "count": self.count,
            "share": self.share(total),
            "weighted_cost": self.weighted_cost(total),
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct TierMetric {
    pub tier: usize,
    pub total: Metric,
    pub patterns: Vec<([u8; 3], Metric)>,
}

#[derive(Clone, Debug, Default)]
pub struct LetterEntry {
    pub letter: char,
    pub key: String,
    pub count: f64,
    pub default_finger: usize,
    pub fingers: Vec<(usize, f64)>,
    pub weighted_strength: f64,
}

#[derive(Clone, Debug)]
pub struct BigramEntry {
    pub ngram: String,
    pub count: f64,
    pub fingers: [usize; 2],
    pub cost: f64,
    pub category: &'static str,
}

#[derive(Clone, Debug)]
pub struct TrigramEntry {
    pub ngram: String,
    pub count: f64,
    pub fingers: [usize; 3],
    pub cost: f64,
    pub category: String,
}

#[derive(Clone, Debug, Default)]
pub struct StrengthMetric {
    pub mean_penalty: f64,
    pub per_finger: [f64; NF],
}

#[derive(Clone, Debug, Default)]
pub struct PreferenceReport {
    pub total_letters: f64,
    pub total_bigrams: f64,
    pub total_trigrams: f64,
    pub scissor_bands: [Metric; 4],
    pub scissor_categories: HashMap<&'static str, Metric>,
    pub redirects: Metric,
    pub redirect_unclassified: Metric,
    pub redirect_tiers: Vec<TierMetric>,
    pub rolls: HashMap<[u8; 2], Metric>,
    pub top_letters: Vec<LetterEntry>,
    pub top_bigrams: Vec<BigramEntry>,
    pub top_trigrams: Vec<TrigramEntry>,
    pub strength_letters: StrengthMetric,
    pub strength_bigrams: StrengthMetric,
    pub strength_trigrams: StrengthMetric,
}

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

fn keymap(layout: &str) -> KeyMap {
    let mut map = [None; 27];
    for (i, ch) in layout.chars().enumerate() {
        if let (Some(id), Some(key)) = (letter_id(ch), key_index(KEYS[i])) {
            map[id] = Some(key);
        }
    }
    map
}

fn finger_label(finger: usize) -> String {
    finger_token(finger).to_string()
}

fn decode_finger(value: u8) -> usize {
    (value as usize) / FINGER_SLOTS
}

fn decode_finger_b(value: u8) -> usize {
    (value as usize) % FINGER_SLOTS
}

pub fn scissor_band(cost: f64) -> usize {
    if cost < SCISSOR_BAND_THRESHOLDS[0] {
        0
    } else if cost < SCISSOR_BAND_THRESHOLDS[1] {
        1
    } else if cost < SCISSOR_BAND_THRESHOLDS[2] {
        2
    } else {
        3
    }
}

fn scissor_category(
    geom: &crate::geometry::Geometry,
    i: usize,
    f1: usize,
    j: usize,
    f2: usize,
) -> Option<&'static str> {
    let pf = pair_feature(geom, i, f1, j, f2);
    if pf.g <= 0.25 {
        return None;
    }
    let d1 = FORDER[finger_kind(f1)];
    let d2 = FORDER[finger_kind(f2)];
    let kind = match finger_kind(if d1 < d2 { f2 } else { f1 }) {
        2 => "scissor_IM",
        1 => "scissor_MR",
        0 => "scissor_RP",
        _ => return None,
    };
    let suffix = if geom.row[i].abs_diff(geom.row[j]) >= 2 {
        "_fsb"
    } else {
        "_hsb"
    };
    Some(match kind {
        "scissor_IM" if suffix == "_fsb" => "scissor_IM_fsb",
        "scissor_IM" => "scissor_IM_hsb",
        "scissor_MR" => "scissor_MR",
        _ => "scissor_RP",
    })
}

fn strength_metric(presses: &[f64; NF], strength: &[f64; NF]) -> StrengthMetric {
    let total: f64 = presses.iter().sum();
    if total <= 0.0 {
        return StrengthMetric::default();
    }
    let cost: f64 = presses
        .iter()
        .zip(strength.iter())
        .map(|(count, value)| count * value)
        .sum();
    let mut per_finger = [0.0; NF];
    for (out, count) in per_finger.iter_mut().zip(presses.iter()) {
        *out = count / total * 100.0;
    }
    StrengthMetric {
        mean_penalty: cost / total,
        per_finger,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn report(
    layout: &str,
    corpus: &Corpus,
    tables: &Tables,
    model: &Model,
    geom: &crate::geometry::Geometry,
    tiers: Option<&RedirectTiers>,
    strength: &[f64; NF],
) -> PreferenceReport {
    let m = keymap(layout);
    let mut out = PreferenceReport {
        rolls: ALL_PAIRS
            .iter()
            .map(|&pair| (pair, Metric::default()))
            .collect(),
        redirect_tiers: tiers
            .map(|tiers| {
                tiers
                    .tiers()
                    .iter()
                    .enumerate()
                    .map(|(tier, patterns)| TierMetric {
                        tier,
                        total: Metric::default(),
                        patterns: patterns
                            .iter()
                            .map(|&pattern| (pattern, Metric::default()))
                            .collect(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        ..Default::default()
    };

    let mut bigram_presses = [0.0; NF];
    let mut trigram_presses = [0.0; NF];
    let mut letter_presses = [0.0; NF];
    let mut letter_fingers: HashMap<char, [f64; NF]> = HashMap::new();
    let mut top_bigrams = Vec::new();
    let mut top_trigrams = Vec::new();

    for (ids, &cnt) in corpus.bi_ids.iter().zip(&corpus.bi_counts) {
        let (Some(i), Some(j)) = (m[ids[0]], m[ids[1]]) else {
            continue;
        };
        out.total_bigrams += cnt;
        let cost = tables.big[big_idx(i, j)];
        let enc = tables.big_f[big_idx(i, j)];
        let f1 = decode_finger(enc);
        let f2 = decode_finger_b(enc);

        if let Some(category) = scissor_category(geom, i, f1, j, f2) {
            let band = scissor_band(cost);
            out.scissor_bands[band].add(cnt, cost * cnt);
            out.scissor_categories
                .entry(category)
                .or_default()
                .add(cnt, cost * cnt);
        }

        if let Some(pair) = pair_for([f1, f2]) {
            let roll_cost = tables.model.parts.roll[pt_idx(i, f1, j, f2)];
            out.rolls.entry(pair).or_default().add(cnt, roll_cost * cnt);
        }

        bigram_presses[f1] += cnt;
        bigram_presses[f2] += cnt;

        let category = if i == j {
            "repeat"
        } else if finger_hand(f1) != finger_hand(f2) {
            "alt"
        } else if f1 == f2 {
            "sfb"
        } else {
            let d1 = FORDER[finger_kind(f1)];
            let d2 = FORDER[finger_kind(f2)];
            let dcol = (geom.cx[i] - geom.cx[j]).abs();
            if d2.abs_diff(d1) >= 2 || dcol >= 3.0 {
                "leap"
            } else if d2 > d1 {
                "roll_in"
            } else {
                "roll_out"
            }
        };

        top_bigrams.push(BigramEntry {
            ngram: format!("{}{}", id_char(ids[0]), id_char(ids[1])),
            count: cnt,
            fingers: [f1, f2],
            cost,
            category,
        });
    }

    for (ids, &cnt) in corpus.tri_ids.iter().zip(&corpus.tri_counts) {
        let (Some(k0), Some(k1), Some(k2)) = (m[ids[0]], m[ids[1]], m[ids[2]]) else {
            continue;
        };
        out.total_trigrams += cnt;
        let keys = [k0, k1, k2];
        let enc = tables.tri_f[tri_idx(keys[0], keys[1], keys[2])];
        let fs = [enc[0] as usize, enc[1] as usize, enc[2] as usize];
        let pattern = trigram_pattern(fs, tiers);

        if let Some(redirect) = pattern.redirect {
            let cost =
                redirect_cost(model.scales.redirect_costs.as_ref(), fs) * model.scales.redirect;
            out.redirects.add(cnt, cost * cnt);
            match pattern.tier {
                Some(tier) if tier < out.redirect_tiers.len() => {
                    out.redirect_tiers[tier].total.add(cnt, cost * cnt);
                    if let Some((_, metric)) = out.redirect_tiers[tier]
                        .patterns
                        .iter_mut()
                        .find(|(candidate, _)| *candidate == redirect)
                    {
                        metric.add(cnt, cost * cnt);
                    }
                }
                _ => out.redirect_unclassified.add(cnt, cost * cnt),
            }
        }

        for (pos, &f) in fs.iter().enumerate() {
            trigram_presses[f] += cnt;
            letter_presses[f] += cnt;
            let ch = id_char(ids[pos]);
            letter_fingers.entry(ch).or_insert([0.0; NF])[f] += cnt;
        }

        top_trigrams.push(TrigramEntry {
            ngram: format!("{}{}{}", id_char(ids[0]), id_char(ids[1]), id_char(ids[2])),
            count: cnt,
            fingers: fs,
            cost: tables.tri[tri_idx(keys[0], keys[1], keys[2])],
            category: pattern.label(tiers),
        });
    }

    out.strength_letters = strength_metric(&letter_presses, strength);
    out.strength_bigrams = strength_metric(&bigram_presses, strength);
    out.strength_trigrams = strength_metric(&trigram_presses, strength);

    let total_letters: f64 = ('a'..='z')
        .map(|ch| corpus.letters.get(&ch).copied().unwrap_or(0.0))
        .sum();
    out.total_letters = total_letters;
    for ch in 'a'..='z' {
        let Some(key) = m[letter_id(ch).unwrap()] else {
            continue;
        };
        let count = corpus.letters.get(&ch).copied().unwrap_or(0.0);
        let counts = letter_fingers.get(&ch).copied().unwrap_or([0.0; NF]);
        let total: f64 = counts.iter().sum();
        let default_finger = model.default_finger[key];
        let mut fingers: Vec<(usize, f64)> = if total > 0.0 {
            counts
                .iter()
                .enumerate()
                .filter(|(_, &value)| value > 0.0)
                .map(|(finger, &value)| (finger, value / total * 100.0))
                .collect()
        } else {
            vec![(default_finger, 100.0)]
        };
        fingers.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| finger_label(a.0).cmp(&finger_label(b.0)))
        });
        let weighted_strength = fingers
            .iter()
            .map(|(finger, share)| strength[*finger] * share / 100.0)
            .sum();
        out.top_letters.push(LetterEntry {
            letter: ch,
            key: KEYS[key].to_string(),
            count,
            default_finger,
            fingers,
            weighted_strength,
        });
    }

    top_bigrams.sort_by(|a, b| {
        b.count
            .total_cmp(&a.count)
            .then_with(|| a.ngram.cmp(&b.ngram))
    });
    top_bigrams.truncate(TOP_N);
    out.top_bigrams = top_bigrams;

    top_trigrams.sort_by(|a, b| {
        b.count
            .total_cmp(&a.count)
            .then_with(|| a.ngram.cmp(&b.ngram))
    });
    top_trigrams.truncate(TOP_N);
    out.top_trigrams = top_trigrams;

    out.top_letters.sort_by(|a, b| {
        b.count
            .total_cmp(&a.count)
            .then_with(|| a.letter.cmp(&b.letter))
    });
    out.top_letters.truncate(TOP_N);
    out
}

fn strength_json(metric: &StrengthMetric) -> Value {
    json!({
        "mean_penalty": metric.mean_penalty,
        "per_finger": metric
            .per_finger
            .iter()
            .enumerate()
            .map(|(finger, share)| (finger_label(finger), json!(*share)))
            .collect::<serde_json::Map<_, _>>(),
    })
}

impl PreferenceReport {
    pub fn to_json(&self) -> Value {
        let scissor_bands: Vec<Value> = self
            .scissor_bands
            .iter()
            .enumerate()
            .map(|(band, metric)| {
                let mut value = metric.to_json(self.total_bigrams);
                value["band"] = json!(SCISSOR_BANDS[band]);
                value
            })
            .collect();
        let scissor_categories: serde_json::Map<String, Value> = self
            .scissor_categories
            .iter()
            .map(|(name, metric)| ((*name).to_string(), metric.to_json(self.total_bigrams)))
            .collect();
        let redirect_tiers: Vec<Value> = self
            .redirect_tiers
            .iter()
            .map(|tier| {
                let patterns: serde_json::Map<String, Value> = tier
                    .patterns
                    .iter()
                    .map(|(pattern, metric)| {
                        (pattern_label(*pattern), metric.to_json(self.total_trigrams))
                    })
                    .collect();
                json!({
                    "tier": tier.tier + 1,
                    "count": tier.total.count,
                    "share": tier.total.share(self.total_trigrams),
                    "weighted_cost": tier.total.weighted_cost(self.total_trigrams),
                    "patterns": patterns,
                })
            })
            .collect();
        let rolls: serde_json::Map<String, Value> = ALL_PAIRS
            .iter()
            .map(|&pair| {
                let metric = self.rolls.get(&pair).copied().unwrap_or_default();
                (pair_key(pair), metric.to_json(self.total_bigrams))
            })
            .collect();
        let top_letters: Vec<Value> = self
            .top_letters
            .iter()
            .map(|entry| {
                json!({
                    "letter": entry.letter.to_string(),
                    "key": entry.key,
                    "count": entry.count,
                    "share": if self.total_letters > 0.0 {
                        entry.count / self.total_letters * 100.0
                    } else {
                        0.0
                    },
                    "default": finger_label(entry.default_finger),
                    "fingers": entry
                        .fingers
                        .iter()
                        .map(|(finger, share)| (finger_label(*finger), json!(*share)))
                        .collect::<serde_json::Map<_, _>>(),
                    "weighted_strength": entry.weighted_strength,
                })
            })
            .collect();
        let top_bigrams: Vec<Value> = self
            .top_bigrams
            .iter()
            .map(|entry| {
                json!({
                    "ngram": entry.ngram,
                    "count": entry.count,
                    "share": if self.total_bigrams > 0.0 {
                        entry.count / self.total_bigrams * 100.0
                    } else {
                        0.0
                    },
                    "fingers": [
                        finger_label(entry.fingers[0]),
                        finger_label(entry.fingers[1]),
                    ],
                    "cost": entry.cost,
                    "category": entry.category,
                })
            })
            .collect();
        let top_trigrams: Vec<Value> = self
            .top_trigrams
            .iter()
            .map(|entry| {
                json!({
                    "ngram": entry.ngram,
                    "count": entry.count,
                    "share": if self.total_trigrams > 0.0 {
                        entry.count / self.total_trigrams * 100.0
                    } else {
                        0.0
                    },
                    "fingers": [
                        finger_label(entry.fingers[0]),
                        finger_label(entry.fingers[1]),
                        finger_label(entry.fingers[2]),
                    ],
                    "cost": entry.cost,
                    "category": entry.category,
                })
            })
            .collect();

        json!({
            "total_letters": self.total_letters,
            "total_bigrams": self.total_bigrams,
            "total_trigrams": self.total_trigrams,
            "scissor_bands": scissor_bands,
            "scissor_categories": scissor_categories,
            "redirects": self.redirects.to_json(self.total_trigrams),
            "redirect_unclassified": self.redirect_unclassified.to_json(self.total_trigrams),
            "redirect_tiers": redirect_tiers,
            "rolls": rolls,
            "top_letters": top_letters,
            "top_bigrams": top_bigrams,
            "top_trigrams": top_trigrams,
            "finger_strength": {
                "letters": strength_json(&self.strength_letters),
                "bigrams": strength_json(&self.strength_bigrams),
                "trigrams": strength_json(&self.strength_trigrams),
            },
        })
    }
}

pub fn top_summary(report: &PreferenceReport) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push("scissor bands:".to_string());
    for (band, metric) in report.scissor_bands.iter().enumerate() {
        lines.push(format!(
            "  {:<9} count={:<12.0} share={:5.2}%  weighted_cost={:.6}",
            SCISSOR_BANDS[band],
            metric.count,
            metric.share(report.total_bigrams),
            metric.weighted_cost(report.total_bigrams)
        ));
    }
    if !report.redirect_tiers.is_empty() {
        lines.push("redirect tiers:".to_string());
        for tier in &report.redirect_tiers {
            lines.push(format!(
                "  tier {:<2} count={:<12.0} share={:5.2}%  weighted_cost={:.6}",
                tier.tier + 1,
                tier.total.count,
                tier.total.share(report.total_trigrams),
                tier.total.weighted_cost(report.total_trigrams)
            ));
        }
        lines.push(format!(
            "  unclassified count={:<12.0} share={:5.2}%",
            report.redirect_unclassified.count,
            report.redirect_unclassified.share(report.total_trigrams)
        ));
    }
    lines.push("rolls:".to_string());
    for pair in ALL_PAIRS {
        let metric = report.rolls.get(&pair).copied().unwrap_or_default();
        lines.push(format!(
            "  {:<4} count={:<12.0} share={:5.2}%  weighted_cost={:.6}",
            pair_label(pair),
            metric.count,
            metric.share(report.total_bigrams),
            metric.weighted_cost(report.total_bigrams)
        ));
    }
    lines.push("finger strength:".to_string());
    for (label, metric) in [
        ("letters", &report.strength_letters),
        ("bigrams", &report.strength_bigrams),
        ("trigrams", &report.strength_trigrams),
    ] {
        lines.push(format!(
            "  {:<9} mean_penalty={:.6}",
            label, metric.mean_penalty
        ));
    }
    lines.push("top letters:".to_string());
    for entry in report.top_letters.iter().take(10) {
        let fingers: Vec<String> = entry
            .fingers
            .iter()
            .map(|(finger, share)| format!("{}={:.1}%", finger_label(*finger), share))
            .collect();
        lines.push(format!(
            "  {}@{} count={:<12.0} {}",
            entry.letter,
            entry.key,
            entry.count,
            fingers.join(" ")
        ));
    }
    lines.push("top bigrams:".to_string());
    for entry in report.top_bigrams.iter().take(10) {
        lines.push(format!(
            "  {:<3} {}-{:<3} cost={:.3}  {}",
            entry.ngram,
            finger_label(entry.fingers[0]),
            finger_label(entry.fingers[1]),
            entry.cost,
            entry.category
        ));
    }
    lines.push("top trigrams:".to_string());
    for entry in report.top_trigrams.iter().take(10) {
        lines.push(format!(
            "  {:<4} {}-{}-{:<3} cost={:.3}  {}",
            entry.ngram,
            finger_label(entry.fingers[0]),
            finger_label(entry.fingers[1]),
            finger_label(entry.fingers[2]),
            entry.cost,
            entry.category
        ));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn presses(values: &[(usize, f64)]) -> [f64; NF] {
        let mut out = [0.0; NF];
        for &(finger, count) in values {
            out[finger] += count;
        }
        out
    }

    #[test]
    fn scissor_bands_follow_the_configured_thresholds() {
        assert_eq!(scissor_band(SCISSOR_BAND_THRESHOLDS[0] - 0.001), 0);
        assert_eq!(scissor_band(SCISSOR_BAND_THRESHOLDS[0]), 1);
        assert_eq!(scissor_band(SCISSOR_BAND_THRESHOLDS[1]), 2);
        assert_eq!(scissor_band(SCISSOR_BAND_THRESHOLDS[2]), 3);
        assert_eq!(scissor_band(100.0), 3);
    }

    #[test]
    fn metric_shares_and_weighted_cost_use_the_corpus_total() {
        let mut metric = Metric::default();
        metric.add(20.0, 10.0);
        assert!((metric.share(80.0) - 25.0).abs() < 1e-12);
        assert!((metric.weighted_cost(80.0) - 0.125).abs() < 1e-12);
        assert_eq!(metric.share(0.0), 0.0);
        assert_eq!(metric.weighted_cost(0.0), 0.0);
    }

    #[test]
    fn strength_metric_is_a_press_weighted_average() {
        let mut strength = [0.0; NF];
        strength[0] = 1.0;
        strength[1] = 3.0;
        let metric = strength_metric(&presses(&[(0, 3.0), (1, 1.0)]), &strength);
        assert!((metric.mean_penalty - 1.5).abs() < 1e-12);
        assert!((metric.per_finger[0] - 75.0).abs() < 1e-12);
        assert!((metric.per_finger[1] - 25.0).abs() < 1e-12);
        let empty = strength_metric(&[0.0; NF], &strength);
        assert_eq!(empty.mean_penalty, 0.0);
    }

    #[test]
    fn metric_json_round_trips_the_core_numbers() {
        let mut metric = Metric::default();
        metric.add(25.0, 5.0);
        let value = metric.to_json(100.0);
        assert_eq!(value["count"], 25.0);
        assert_eq!(value["share"], 25.0);
        assert_eq!(value["weighted_cost"], 0.05);
    }
}
