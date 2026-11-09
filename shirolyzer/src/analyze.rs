//! `analyze`: detailed layout report.
//!
//! Port of `shirolyzer/commands/analyze.py`.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{anyhow, Result};
use serde_json::json;

use crate::context;
use crate::evaluate::score_layout;
use crate::geometry::{finger_hand, finger_kind, finger_token, key_index, KEYS, NF};
use crate::model::{
    e_idx, pair_feature, pt_idx, sfb_pair_cost, PAIR_CAP, W_ALT, W_CONTINUE, W_REPEAT,
    W_REPEAT_SWAP, W_SFS_1U, W_SFS_FAR,
};
use crate::paths;
use crate::preferences;
use crate::provenance;
use crate::redirect::{pattern_for, pattern_key, pattern_label, redirect_cost, RedirectTiers};
use crate::roll::{self, pair_key, pair_label};
use crate::scorer::{Tables, FINGER_SLOTS, SPACE_EFFORT, SPACE_FINGER, SPACE_ID};

const LETTERS: &str = "abcdefghijklmnopqrstuvwxyz";
const FORDER: [usize; 4] = [0, 1, 2, 3];

type KeyMap = [Option<usize>; 27];

fn letter_id(ch: char) -> Option<usize> {
    match ch {
        'a'..='z' => Some(ch as usize - 'a' as usize),
        ' ' => Some(crate::corpus::SPACE_INDEX),
        _ => None,
    }
}

fn id_char(id: usize) -> char {
    if id == crate::corpus::SPACE_INDEX {
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
    if finger == SPACE_FINGER {
        "T".to_string()
    } else {
        finger_token(finger).to_string()
    }
}

fn decode_finger(value: u8) -> usize {
    (value as usize) / FINGER_SLOTS
}

fn decode_finger_b(value: u8) -> usize {
    (value as usize) % FINGER_SLOTS
}

fn decode_finger_at(value: [u8; 3], pos: usize) -> usize {
    value[pos] as usize
}

#[derive(Clone, Debug, Default)]
pub struct ScoreBreakdown {
    pub key_effort: f64,
    pub key_press: f64,
    pub pair_vertical: f64,
    pub pair_cross: f64,
    pub pair_spread: f64,
    pub pair_roll: f64,
    pub pair_leap: f64,
    pub pair_shift: f64,
    pub pair_cap: f64,
    pub pair_sfb: f64,
    pub pair_alt: f64,
    pub pair_repeat: f64,
    pub trigram_redirect: f64,
    pub trigram_continue: f64,
    pub trigram_scissor: f64,
    pub trigram_sfs: f64,
    pub finger_strength: f64,
    pub gate_penalty: f64,
    pub space: f64,
    pub total: f64,
}

impl ScoreBreakdown {
    pub fn sum(&self) -> f64 {
        self.key_effort
            + self.key_press
            + self.pair_vertical
            + self.pair_cross
            + self.pair_spread
            + self.pair_roll
            + self.pair_leap
            + self.pair_shift
            + self.pair_cap
            + self.pair_sfb
            + self.pair_alt
            + self.pair_repeat
            + self.trigram_redirect
            + self.trigram_continue
            + self.trigram_scissor
            + self.trigram_sfs
            + self.finger_strength
            + self.gate_penalty
            + self.space
    }

    fn scale(&mut self, denom: f64) {
        for value in [
            &mut self.key_effort,
            &mut self.key_press,
            &mut self.pair_vertical,
            &mut self.pair_cross,
            &mut self.pair_spread,
            &mut self.pair_roll,
            &mut self.pair_leap,
            &mut self.pair_shift,
            &mut self.pair_cap,
            &mut self.pair_sfb,
            &mut self.pair_alt,
            &mut self.pair_repeat,
            &mut self.trigram_redirect,
            &mut self.trigram_continue,
            &mut self.trigram_scissor,
            &mut self.trigram_sfs,
            &mut self.finger_strength,
            &mut self.gate_penalty,
            &mut self.space,
            &mut self.total,
        ] {
            *value /= denom;
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "key_effort": self.key_effort,
            "key_press": self.key_press,
            "pair_vertical": self.pair_vertical,
            "pair_cross": self.pair_cross,
            "pair_spread": self.pair_spread,
            "pair_roll": self.pair_roll,
            "pair_leap": self.pair_leap,
            "pair_shift": self.pair_shift,
            "pair_cap": self.pair_cap,
            "pair_sfb": self.pair_sfb,
            "pair_alt": self.pair_alt,
            "pair_repeat": self.pair_repeat,
            "trigram_redirect": self.trigram_redirect,
            "trigram_continue": self.trigram_continue,
            "trigram_scissor": self.trigram_scissor,
            "trigram_sfs": self.trigram_sfs,
            "finger_strength": self.finger_strength,
            "gate_penalty": self.gate_penalty,
            "space": self.space,
            "total": self.total,
            "reconstructed": self.sum(),
        })
    }
}

fn key_char(k: usize) -> char {
    KEYS[k].chars().next().unwrap()
}

#[allow(clippy::too_many_arguments)]
fn add_pair_terms(
    out: &mut ScoreBreakdown,
    model: &crate::model::Model,
    geom: &crate::geometry::Geometry,
    mt: &crate::model::ModelTables,
    a: usize,
    fa: usize,
    b: usize,
    fb: usize,
    count: f64,
) {
    let pf = pair_feature(geom, a, fa, b, fb);
    if a == b {
        let repeat = W_REPEAT + if fa != fb { W_REPEAT_SWAP } else { 0.0 };
        out.pair_repeat += repeat * count;
        return;
    }
    if pf.same_hand && fa == fb {
        out.pair_sfb +=
            sfb_pair_cost(geom, a, b, model.scales.sfb, model.scales.sfb_impossible) * count;
        return;
    }
    if !pf.same_hand {
        out.pair_alt += W_ALT * model.scales.alt * count;
        return;
    }
    let i = pt_idx(a, fa, b, fb);
    let parts = &mt.parts;
    out.pair_vertical += parts.vert[i] * count;
    out.pair_cross += parts.cross[i] * count;
    out.pair_spread += parts.spread[i] * count;
    out.pair_roll += parts.roll[i] * count;
    out.pair_leap += parts.leap[i] * count;
    out.pair_shift += parts.shift[i] * count;
    let raw = parts.vert[i]
        + parts.cross[i]
        + parts.spread[i]
        + parts.roll[i]
        + parts.leap[i]
        + parts.shift[i];
    if raw > PAIR_CAP {
        out.pair_cap += (PAIR_CAP - raw) * count;
    }
}

#[allow(clippy::too_many_arguments)]
fn add_trigram_terms(
    out: &mut ScoreBreakdown,
    model: &crate::model::Model,
    geom: &crate::geometry::Geometry,
    tables: &Tables,
    a: usize,
    fa: usize,
    _b: usize,
    fb: usize,
    c: usize,
    fc: usize,
    count: f64,
) {
    let d1 = fb as i32 - fa as i32;
    let d2 = fc as i32 - fb as i32;
    let same = finger_hand(fa) == finger_hand(fb) && finger_hand(fb) == finger_hand(fc);
    let strain = |x: usize, fx: usize, z: usize, fz: usize| {
        let s = tables.model.strain[pt_idx(x, fx, z, fz)];
        if s.is_finite() {
            s
        } else {
            0.0
        }
    };
    if same {
        if d1 * d2 < 0 {
            out.trigram_redirect +=
                redirect_cost(model.scales.redirect_costs.as_ref(), [fa, fb, fc])
                    * model.scales.redirect
                    * count;
        }
        if d1 * d2 > 0 {
            out.trigram_continue += W_CONTINUE * model.scales.redirect * count;
        }
        if fa != fc && a != c {
            out.trigram_scissor +=
                model.p("gamma13") * model.scales.scissor * strain(a, fa, c, fc) * count;
        }
    } else {
        let alt13 =
            finger_hand(fa) == finger_hand(fc) && finger_hand(fa) != finger_hand(fb) && a != c;
        if alt13 && fa != fc {
            out.trigram_scissor +=
                model.p("gamma13_alt") * model.scales.scissor * strain(a, fa, c, fc) * count;
        }
    }
    let m13 = fa == fc
        && !(finger_hand(fa) == finger_hand(fb) && fa == fb)
        && finger_hand(fa) == finger_hand(fc)
        && a != c;
    if m13 {
        let sfs = if geom.distance(a, c) <= 1.3 {
            W_SFS_1U
        } else {
            W_SFS_FAR
        } * model.scales.sfb;
        out.trigram_sfs += sfs * count;
    }
}

fn add_space_terms(
    out: &mut ScoreBreakdown,
    mt: &crate::model::ModelTables,
    ids: [usize; 3],
    fs: [usize; 3],
    count: f64,
) {
    let mut cost = 0.0;
    for (pos, &id) in ids.iter().enumerate() {
        if id == SPACE_ID {
            cost += SPACE_EFFORT;
        } else {
            cost += mt.e[e_idx(id, fs[pos])] + crate::scorer::PRESS;
        }
    }
    for (p, q) in [(0usize, 1usize), (1, 2)] {
        if ids[p] == SPACE_ID || ids[q] == SPACE_ID {
            cost += W_ALT;
        } else {
            cost += mt.pt[pt_idx(ids[p], fs[p], ids[q], fs[q])];
        }
    }
    out.space += cost * count;
}

pub fn score_breakdown(
    layout: &str,
    corpus: &crate::corpus::Corpus,
    tables: &Tables,
    model: &crate::model::Model,
    geom: &crate::geometry::Geometry,
    gates: &crate::gates::Gates,
    conditions: &crate::conditions::Conditions,
) -> ScoreBreakdown {
    let m = keymap(layout);
    let mt = model.tables_unrestricted(geom);
    let mut out = ScoreBreakdown::default();
    let mut denom = 0.0;
    let mut pm = vec![0.0; NF * NF * NF];
    for (ids, &count) in corpus.tri_ids.iter().zip(&corpus.tri_counts) {
        let (Some(k0), Some(k1), Some(k2)) = (m[ids[0]], m[ids[1]], m[ids[2]]) else {
            continue;
        };
        let keys = [k0, k1, k2];
        let enc = tables.tri_f[crate::scorer::tri_idx(k0, k1, k2)];
        let fs = [enc[0] as usize, enc[1] as usize, enc[2] as usize];
        denom += count;
        out.total += tables.tri[crate::scorer::tri_idx(k0, k1, k2)] * count;
        out.finger_strength += keys
            .iter()
            .zip(fs)
            .filter(|(key, _)| **key != SPACE_ID)
            .map(|(_, finger)| model.finger_strength[finger])
            .sum::<f64>()
            * count;
        if keys.contains(&SPACE_ID) {
            add_space_terms(&mut out, &mt, keys, fs, count);
            continue;
        }
        out.key_effort +=
            (mt.e[e_idx(k0, fs[0])] + mt.e[e_idx(k1, fs[1])] + mt.e[e_idx(k2, fs[2])]) * count;
        out.key_press += 3.0 * crate::scorer::PRESS * count;
        add_pair_terms(&mut out, model, geom, &mt, k0, fs[0], k1, fs[1], count);
        add_pair_terms(&mut out, model, geom, &mt, k1, fs[1], k2, fs[2], count);
        add_trigram_terms(
            &mut out, model, geom, tables, k0, fs[0], k1, fs[1], k2, fs[2], count,
        );
        let chars = [key_char(k0), key_char(k1), key_char(k2)];
        gates.penalty_mask_into(&chars, conditions, &mut pm);
        let pi = (fs[0] * NF + fs[1]) * NF + fs[2];
        out.gate_penalty += pm[pi] * count;
    }
    if denom > 0.0 {
        out.scale(denom);
    }
    out
}

pub struct CategoryReport {
    pub cats: HashMap<String, f64>,
    pub worst: Vec<(f64, String, String, String, f64)>,
    pub scissors: Vec<(f64, f64, String, String, String, f64, String)>,
}

pub fn category_report(
    layout: &str,
    corpus: &crate::corpus::Corpus,
    tables: &Tables,
    geom: &crate::geometry::Geometry,
) -> CategoryReport {
    let m = keymap(layout);
    let mut stats: HashMap<String, f64> = HashMap::new();
    let mut total = 0.0;
    let mut worst = Vec::new();
    let mut scissors = Vec::new();
    for (ids, &cnt) in corpus.bi_ids.iter().zip(&corpus.bi_counts) {
        let (Some(i), Some(j)) = (m[ids[0]], m[ids[1]]) else {
            continue;
        };
        let (a, b) = (id_char(ids[0]), id_char(ids[1]));
        total += cnt;
        let cost = tables.big[crate::scorer::big_idx(i, j)];
        let enc = tables.big_f[crate::scorer::big_idx(i, j)];
        let f1 = decode_finger(enc);
        let f2 = decode_finger_b(enc);
        let l1 = finger_label(f1);
        let l2 = finger_label(f2);
        worst.push((cost, format!("{}{}", a, b), l1.clone(), l2.clone(), cnt));
        if i == j {
            *stats.entry("repeat".into()).or_insert(0.0) += cnt;
        } else if finger_hand(f1) != finger_hand(f2) {
            *stats.entry("alt".into()).or_insert(0.0) += cnt;
        } else if f1 == f2 {
            *stats.entry("sfb".into()).or_insert(0.0) += cnt;
        } else {
            let d1 = FORDER[finger_kind(f1)];
            let d2 = FORDER[finger_kind(f2)];
            let dcol = (geom.cx[i] - geom.cx[j]).abs();
            if d2.abs_diff(d1) >= 2 || dcol >= 3.0 {
                *stats.entry("leap".into()).or_insert(0.0) += cnt;
            } else if d2 > d1 {
                *stats.entry("roll_in".into()).or_insert(0.0) += cnt;
            } else {
                *stats.entry("roll_out".into()).or_insert(0.0) += cnt;
            }
            if d2.abs_diff(d1) == 1 && dcol < 3.0 {
                if let Some(pair) = roll::pair_for_kinds(finger_kind(f1), finger_kind(f2)) {
                    *stats.entry(pair_key(pair)).or_insert(0.0) += cnt;
                }
            }
            let pf = pair_feature(geom, i, f1, j, f2);
            if pf.g > 0.25 {
                let (outer, inner) = if d1 < d2 {
                    (l1.clone(), l2.clone())
                } else {
                    (l2.clone(), l1.clone())
                };
                let kind = match finger_kind(if d1 < d2 { f2 } else { f1 }) {
                    2 => Some("scissor_IM"),
                    1 => Some("scissor_MR"),
                    0 => Some("scissor_RP"),
                    _ => None,
                };
                if let Some(kind) = kind {
                    let suffix = if geom.row[i].abs_diff(geom.row[j]) >= 2 {
                        "_fsb"
                    } else {
                        "_hsb"
                    };
                    *stats.entry(format!("{}{}", kind, suffix)).or_insert(0.0) += cnt;
                    scissors.push((
                        cost * cnt,
                        cost,
                        format!("{}{}", a, b),
                        l1.clone(),
                        l2.clone(),
                        cnt,
                        format!("{}{}", outer, inner),
                    ));
                }
            }
        }
    }
    if total == 0.0 {
        total = 1.0;
    }
    let mut cats = HashMap::new();
    for key in [
        "sfb",
        "scissor_IM_hsb",
        "scissor_IM_fsb",
        "scissor_MR",
        "scissor_RP",
        "roll_in",
        "roll_out",
        "leap",
        "alt",
        "repeat",
    ] {
        let value = stats.get(key).copied().unwrap_or(0.0);
        cats.insert(key.to_string(), value / total * 100.0);
    }
    for pair in roll::ALL_PAIRS {
        let key = pair_key(pair);
        let value = stats.get(&key).copied().unwrap_or(0.0);
        cats.insert(key, value / total * 100.0);
    }
    cats.insert(
        "hand_alternation".to_string(),
        *cats.get("alt").unwrap_or(&0.0),
    );
    cats.insert("total_tokens".to_string(), total);
    worst.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.1.cmp(&a.1))
    });
    scissors.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal))
    });
    worst.truncate(30);
    scissors.truncate(20);
    CategoryReport {
        cats,
        worst,
        scissors,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrigramPattern {
    pub category: &'static str,
    pub redirect: Option<[u8; 3]>,
    pub tier: Option<usize>,
}

impl TrigramPattern {
    fn plain(category: &'static str) -> Self {
        Self {
            category,
            redirect: None,
            tier: None,
        }
    }

    pub fn label(&self, tiers: Option<&RedirectTiers>) -> String {
        match (self.redirect, self.tier) {
            (Some(pattern), Some(tier)) => {
                format!("redirect_tier_{}:{}", tier + 1, pattern_label(pattern))
            }
            (Some(pattern), None) if tiers.is_some() => {
                format!("redirect_unclassified:{}", pattern_label(pattern))
            }
            _ => self.category.to_string(),
        }
    }
}

pub fn trigram_pattern(fs: [usize; 3], tiers: Option<&RedirectTiers>) -> TrigramPattern {
    let hs = [finger_hand(fs[0]), finger_hand(fs[1]), finger_hand(fs[2])];
    let os = [
        FORDER[finger_kind(fs[0])],
        FORDER[finger_kind(fs[1])],
        FORDER[finger_kind(fs[2])],
    ];
    if (hs == [0, 1, 0]) || (hs == [1, 0, 1]) {
        return TrigramPattern::plain("alternate");
    }
    if hs[0] == hs[1] && hs[1] == hs[2] {
        if fs[0] == fs[1] && fs[1] == fs[2] {
            TrigramPattern::plain("sft")
        } else if fs[0] == fs[1] || fs[1] == fs[2] {
            TrigramPattern::plain("sfb_tri")
        } else if let Some(pattern) = pattern_for(fs) {
            TrigramPattern {
                category: if fs.iter().any(|&f| finger_kind(f) == 3) {
                    "redirect"
                } else {
                    "redirect_weak"
                },
                redirect: Some(pattern),
                tier: tiers.and_then(|tiers| tiers.tier_of(pattern)),
            }
        } else {
            TrigramPattern::plain("onehand")
        }
    } else if hs[0] == hs[1] {
        if fs[0] == fs[1] {
            TrigramPattern::plain("sfb_tri")
        } else if os[0] < os[1] {
            TrigramPattern::plain("inroll")
        } else {
            TrigramPattern::plain("outroll")
        }
    } else if hs[1] == hs[2] {
        if fs[1] == fs[2] {
            TrigramPattern::plain("sfb_tri")
        } else if os[1] < os[2] {
            TrigramPattern::plain("inroll")
        } else {
            TrigramPattern::plain("outroll")
        }
    } else {
        TrigramPattern::plain("other")
    }
}

fn is_same_finger_skipgram(fs: [usize; 3]) -> bool {
    fs[0] == fs[2] && fs[0] != fs[1]
}

/// Key-aware classification of a trigram for the repeat-free diagnostics.
///
/// A repeated physical key always uses one finger, so the true same-finger
/// metrics are restricted to trigrams with three distinct physical keys.
/// Repeats are reported separately and never inflate the true counts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrigramClass {
    pub sfb_true: bool,
    pub sft_true: bool,
    pub sfs_true: bool,
    pub repeat_adjacent: bool,
    pub repeat_skip: bool,
    pub repeat_all: bool,
}

pub fn classify_trigram(keys: [usize; 3], fingers: [usize; 3]) -> TrigramClass {
    let repeat_adjacent = keys[0] == keys[1] || keys[1] == keys[2];
    let repeat_skip = keys[0] == keys[2] && keys[0] != keys[1];
    let repeat_all = repeat_adjacent || keys[0] == keys[2];
    let distinct = keys[0] != keys[1] && keys[1] != keys[2] && keys[0] != keys[2];
    let all_same_finger = fingers[0] == fingers[1] && fingers[1] == fingers[2];
    let sft_true = distinct && all_same_finger;
    let sfb_true = distinct && !sft_true && (fingers[0] == fingers[1] || fingers[1] == fingers[2]);
    let sfs_true = distinct && fingers[0] == fingers[2] && fingers[0] != fingers[1];
    TrigramClass {
        sfb_true,
        sft_true,
        sfs_true,
        repeat_adjacent,
        repeat_skip,
        repeat_all,
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct TrueCounts {
    sfb_true: f64,
    sft_true: f64,
    sfs_true: f64,
    repeat_adjacent: f64,
    repeat_skip: f64,
    repeat_all: f64,
}

impl TrueCounts {
    fn add(&mut self, class: TrigramClass, count: f64) {
        if class.sfb_true {
            self.sfb_true += count;
        }
        if class.sft_true {
            self.sft_true += count;
        }
        if class.sfs_true {
            self.sfs_true += count;
        }
        if class.repeat_adjacent {
            self.repeat_adjacent += count;
        }
        if class.repeat_skip {
            self.repeat_skip += count;
        }
        if class.repeat_all {
            self.repeat_all += count;
        }
    }
}

fn count_true_trigrams(
    layout: &str,
    corpus: &crate::corpus::Corpus,
    tables: &Tables,
    default_fingers: Option<&[usize; crate::geometry::NK]>,
) -> (TrueCounts, f64) {
    let m = keymap(layout);
    let mut acc = TrueCounts::default();
    let mut total = 0.0;
    for (letter_ids, &cnt) in corpus.tri_ids.iter().zip(&corpus.tri_counts) {
        let (Some(k0), Some(k1), Some(k2)) = (m[letter_ids[0]], m[letter_ids[1]], m[letter_ids[2]])
        else {
            continue;
        };
        let keys = [k0, k1, k2];
        let fs = match default_fingers {
            Some(dflt) => [dflt[k0], dflt[k1], dflt[k2]],
            None => {
                let enc = tables.tri_f[crate::scorer::tri_idx(k0, k1, k2)];
                [
                    decode_finger_at(enc, 0),
                    decode_finger_at(enc, 1),
                    decode_finger_at(enc, 2),
                ]
            }
        };
        total += cnt;
        acc.add(classify_trigram(keys, fs), cnt);
    }
    (acc, total)
}

/// Repeat-free same-finger metrics under the chosen DP fingers and the default
/// fingermap.  `*_delta` is chosen minus default, so a negative value means
/// alt fingering reduced the metric.
pub fn true_same_finger_metrics(
    layout: &str,
    corpus: &crate::corpus::Corpus,
    tables: &Tables,
    default_fingers: &[usize; crate::geometry::NK],
) -> HashMap<String, f64> {
    let (chosen, total) = count_true_trigrams(layout, corpus, tables, None);
    let (default, default_total) =
        count_true_trigrams(layout, corpus, tables, Some(default_fingers));
    let pct = |value: f64, denom: f64| {
        if denom > 0.0 {
            value / denom * 100.0
        } else {
            0.0
        }
    };
    let rows = [
        ("sfb_true", chosen.sfb_true, default.sfb_true),
        ("sft_true", chosen.sft_true, default.sft_true),
        (
            "sfb_sft_true",
            chosen.sfb_true + chosen.sft_true,
            default.sfb_true + default.sft_true,
        ),
        ("sfs_true", chosen.sfs_true, default.sfs_true),
        (
            "repeat_adjacent",
            chosen.repeat_adjacent,
            default.repeat_adjacent,
        ),
        ("repeat_skip", chosen.repeat_skip, default.repeat_skip),
        ("repeat_all", chosen.repeat_all, default.repeat_all),
    ];
    let mut out = HashMap::new();
    for (name, chosen_value, default_value) in rows {
        let chosen_pct = pct(chosen_value, total);
        let default_pct = pct(default_value, default_total);
        out.insert(name.to_string(), chosen_pct);
        out.insert(format!("{name}_default"), default_pct);
        out.insert(format!("{name}_delta"), chosen_pct - default_pct);
    }
    out
}

pub fn trigram_patterns(
    layout: &str,
    corpus: &crate::corpus::Corpus,
    tables: &Tables,
    _geom: &crate::geometry::Geometry,
    tiers: Option<&RedirectTiers>,
) -> HashMap<String, f64> {
    let m = keymap(layout);
    let mut acc: HashMap<String, f64> = HashMap::new();
    let mut total = 0.0;
    for (letter_ids, &cnt) in corpus.tri_ids.iter().zip(&corpus.tri_counts) {
        let (Some(k0), Some(k1), Some(k2)) = (m[letter_ids[0]], m[letter_ids[1]], m[letter_ids[2]])
        else {
            continue;
        };
        let ids = [k0, k1, k2];
        let enc = tables.tri_f[crate::scorer::tri_idx(ids[0], ids[1], ids[2])];
        let fs = [
            decode_finger_at(enc, 0),
            decode_finger_at(enc, 1),
            decode_finger_at(enc, 2),
        ];
        total += cnt;
        let add = |acc: &mut HashMap<String, f64>, key: &str| {
            *acc.entry(key.to_string()).or_insert(0.0) += cnt;
        };
        let pat = trigram_pattern(fs, tiers);
        add(&mut acc, pat.category);
        if is_same_finger_skipgram(fs) {
            add(&mut acc, "sfs");
        }
        if let Some(redirect) = pat.redirect {
            match pat.tier {
                Some(tier) => {
                    add(&mut acc, &format!("redirect_tier_{}", tier + 1));
                    add(&mut acc, &pattern_key(redirect));
                }
                None if tiers.is_some() => add(&mut acc, "redirect_unclassified"),
                None => {}
            }
        }
        if ids[0] == ids[1] || ids[1] == ids[2] || ids[0] == ids[2] {
            add(&mut acc, "repeat");
        }
    }
    if total == 0.0 {
        total = 1.0;
    }
    let mut out: HashMap<String, f64> = acc
        .into_iter()
        .map(|(k, v)| (k, v / total * 100.0))
        .collect();
    let rolls =
        out.get("inroll").copied().unwrap_or(0.0) + out.get("outroll").copied().unwrap_or(0.0);
    let redirects = out.get("redirect").copied().unwrap_or(0.0)
        + out.get("redirect_weak").copied().unwrap_or(0.0);
    let alternation = out.get("alternate").copied().unwrap_or(0.0);
    let sft = out.get("sft").copied().unwrap_or(0.0);
    let sfb_tri = out.get("sfb_tri").copied().unwrap_or(0.0);
    let sfs = out.get("sfs").copied().unwrap_or(0.0);
    out.insert("total_rolls".into(), rolls);
    out.insert("total_redirects".into(), redirects);
    out.insert("hand_alternation".into(), alternation);
    out.insert("sfb_sft".into(), sfb_tri + sft);
    out.insert("same_finger_total".into(), sfb_tri + sft + sfs);
    out
}

pub fn finger_usage(
    layout: &str,
    corpus: &crate::corpus::Corpus,
    tables: &Tables,
    _geom: &crate::geometry::Geometry,
) -> HashMap<String, f64> {
    let m = keymap(layout);
    let mut usage = [[0.0f64; 4]; 2];
    for (letter_ids, &cnt) in corpus.tri_ids.iter().zip(&corpus.tri_counts) {
        let (Some(k0), Some(k1), Some(k2)) = (m[letter_ids[0]], m[letter_ids[1]], m[letter_ids[2]])
        else {
            continue;
        };
        let ids = [k0, k1, k2];
        let enc = tables.tri_f[crate::scorer::tri_idx(ids[0], ids[1], ids[2])];
        for (pos, _k) in ids.iter().enumerate() {
            let f = decode_finger_at(enc, pos);
            usage[finger_hand(f)][finger_kind(f)] += cnt;
        }
    }
    let total: f64 = usage.iter().flat_map(|h| h.iter()).sum::<f64>().max(1.0);
    let mut out = HashMap::new();
    for (h, label) in [(0, 'L'), (1, 'R')] {
        for (fi, f) in ['P', 'R', 'M', 'I'].iter().enumerate() {
            out.insert(format!("{}{}", label, f), usage[h][fi] / total * 100.0);
        }
    }
    out.insert("handL".into(), usage[0].iter().sum::<f64>() / total * 100.0);
    out.insert("handR".into(), usage[1].iter().sum::<f64>() / total * 100.0);
    out
}

pub struct GuideEntry {
    pub key: String,
    pub default: usize,
    pub share: Vec<(usize, f64)>,
    pub alt_contexts: Vec<(usize, Vec<(String, f64)>)>,
}

#[derive(Debug)]
pub struct CrossHandRow {
    pub letter: char,
    pub key: String,
    pub default: usize,
    pub default_share: f64,
    pub cross_share: f64,
    pub contexts: Vec<(String, usize, f64)>,
}

#[derive(Debug)]
pub struct CrossHandReport {
    pub total_share: f64,
    pub rows: Vec<CrossHandRow>,
}

/// Report-only cross-hand alt usage, weighted by the primary corpus letters.
pub fn cross_hand_report(
    layout: &str,
    corpus: &crate::corpus::Corpus,
    guide: &HashMap<char, GuideEntry>,
) -> CrossHandReport {
    let m = keymap(layout);
    let total_letters: f64 = corpus.letters.values().sum();
    let mut weighted_cross = 0.0;
    let mut rows = Vec::new();
    for ch in LETTERS.chars() {
        let Some(key) = m[letter_id(ch).unwrap()] else {
            continue;
        };
        let Some(entry) = guide.get(&ch) else {
            continue;
        };
        let default_hand = finger_hand(entry.default);
        let mut default_share = 0.0;
        let mut cross_share = 0.0;
        for &(finger, share) in &entry.share {
            if finger == entry.default {
                default_share += share;
            }
            if finger_hand(finger) != default_hand {
                cross_share += share;
            }
        }
        if cross_share <= 1e-9 {
            continue;
        }
        let mut contexts: Vec<(String, usize, f64)> = Vec::new();
        for (finger, items) in &entry.alt_contexts {
            if finger_hand(*finger) == default_hand {
                continue;
            }
            for (trigram, count) in items {
                contexts.push((trigram.clone(), *finger, *count));
            }
        }
        contexts.sort_by(|a, b| {
            b.2.partial_cmp(&a.2)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        contexts.truncate(8);
        weighted_cross += cross_share / 100.0 * corpus.letters.get(&ch).copied().unwrap_or(0.0);
        rows.push(CrossHandRow {
            letter: ch,
            key: KEYS[key].to_string(),
            default: entry.default,
            default_share,
            cross_share,
            contexts,
        });
    }
    rows.sort_by(|a, b| {
        b.cross_share
            .partial_cmp(&a.cross_share)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    CrossHandReport {
        total_share: if total_letters > 0.0 {
            weighted_cross / total_letters * 100.0
        } else {
            0.0
        },
        rows,
    }
}

#[derive(Debug)]
pub struct DeadKeyRow {
    pub key: String,
    pub symbol: char,
    pub shifted: Option<char>,
}

#[derive(Debug)]
pub struct HumanFactorBigram {
    pub bigram: String,
    pub kind: &'static str,
    pub keys: Vec<String>,
}

#[derive(Debug)]
pub struct PunctuationReport {
    pub dead_keys: Vec<DeadKeyRow>,
    pub apostrophe_symbol: char,
    pub apostrophe_shifted: char,
    pub apostrophe_keys: Vec<String>,
    pub apostrophe_modeled: bool,
    pub bigrams: Vec<HumanFactorBigram>,
}

const APOSTROPHE_KEY: &str = "AC11/LSGT";

const HUMAN_FACTOR_BIGRAMS: &[(&str, &str)] = &[
    ("; ", "prose"),
    (", ", "prose"),
    (". ", "prose"),
    ("? ", "prose"),
    (": ", "prose"),
    ("'s", "prose"),
    ("n't", "prose"),
    (");", "programming"),
    ("};", "programming"),
    ("//", "programming"),
    ("/*", "programming"),
    ("*/", "programming"),
    ("<=", "programming"),
    ("=>", "programming"),
];

fn shifted_symbol(ch: char) -> Option<char> {
    match ch {
        ';' => Some(':'),
        ',' => Some('<'),
        '.' => Some('>'),
        '/' => Some('?'),
        '\'' => Some('"'),
        _ => None,
    }
}

fn punctuation_base(ch: char) -> Option<char> {
    match ch {
        ':' => Some(';'),
        '<' => Some(','),
        '>' => Some('.'),
        '?' => Some('/'),
        '"' => Some('\''),
        _ => None,
    }
}

fn key_for_char(layout: &str, ch: char) -> Option<String> {
    if ch == ' ' {
        return Some("SPACE".to_string());
    }
    if ch == '\'' {
        return Some(APOSTROPHE_KEY.to_string());
    }
    if ch.is_ascii_alphabetic() {
        let m = keymap(layout);
        return m[letter_id(ch)?].map(|key| KEYS[key].to_string());
    }
    let base = punctuation_base(ch).unwrap_or(ch);
    layout
        .chars()
        .position(|c| c == base)
        .map(|slot| KEYS[slot].to_string())
}

/// Report-only punctuation and shift-layer coverage.
pub fn punctuation_report(layout: &str) -> PunctuationReport {
    let mut dead_keys = Vec::new();
    for (slot, ch) in layout.chars().enumerate() {
        if !ch.is_ascii_alphabetic() {
            dead_keys.push(DeadKeyRow {
                key: KEYS[slot].to_string(),
                symbol: ch,
                shifted: shifted_symbol(ch),
            });
        }
    }
    let apostrophe_slot = layout.chars().position(|ch| ch == '\'');
    let apostrophe_modeled = apostrophe_slot.is_some();
    let apostrophe_keys = apostrophe_slot
        .map(|slot| vec![KEYS[slot].to_string()])
        .unwrap_or_else(|| vec![APOSTROPHE_KEY.to_string()]);
    let mut bigrams = Vec::new();
    for &(bigram, kind) in HUMAN_FACTOR_BIGRAMS {
        let mut keys = Vec::new();
        for ch in bigram.chars() {
            let Some(key) = key_for_char(layout, ch) else {
                continue;
            };
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
        if keys.iter().any(|key| {
            dead_keys.iter().any(|dead| &dead.key == key) || apostrophe_keys.contains(key)
        }) {
            bigrams.push(HumanFactorBigram {
                bigram: bigram.to_string(),
                kind,
                keys,
            });
        }
    }
    PunctuationReport {
        dead_keys,
        apostrophe_symbol: '\'',
        apostrophe_shifted: '"',
        apostrophe_keys,
        apostrophe_modeled,
        bigrams,
    }
}

pub struct ShortcutRow {
    pub letter: char,
    pub key: String,
    pub hand: &'static str,
    pub chosen: usize,
    pub chosen_share: f64,
    pub default: usize,
    pub ctrl_same_hand: bool,
    pub ctrl_same_finger: bool,
}

/// Report-only check for one-handed left-Ctrl shortcuts.
///
/// Left Ctrl is treated as left-pinky (`LP`). The chosen finger is the most
/// frequent DP finger for the letter; the default finger comes from the
/// configured fingermap.
pub fn shortcut_report(
    layout: &str,
    guide: &HashMap<char, GuideEntry>,
    model: &crate::model::Model,
) -> Vec<ShortcutRow> {
    let m = keymap(layout);
    ['z', 'x', 'c', 'v', 't', 'y']
        .iter()
        .filter_map(|&letter| {
            let key = m[letter_id(letter)?]?;
            let default = model.default_finger[key];
            let (chosen, chosen_share) = guide
                .get(&letter)
                .and_then(|entry| entry.share.first().copied())
                .unwrap_or((default, 0.0));
            Some(ShortcutRow {
                letter,
                key: KEYS[key].to_string(),
                hand: if finger_hand(default) == 0 {
                    "left"
                } else {
                    "right"
                },
                chosen,
                chosen_share,
                default,
                ctrl_same_hand: finger_hand(chosen) == 0,
                ctrl_same_finger: chosen == 0,
            })
        })
        .collect()
}

pub fn fingering_guide(
    layout: &str,
    corpus: &crate::corpus::Corpus,
    tables: &Tables,
    model: &crate::model::Model,
) -> HashMap<char, GuideEntry> {
    let m = keymap(layout);
    let mut agg: HashMap<char, HashMap<usize, f64>> = HashMap::new();
    let mut ctx: HashMap<char, HashMap<usize, HashMap<[char; 3], f64>>> = HashMap::new();
    for (letter_ids, &cnt) in corpus.tri_ids.iter().zip(&corpus.tri_counts) {
        let (Some(k0), Some(k1), Some(k2)) = (m[letter_ids[0]], m[letter_ids[1]], m[letter_ids[2]])
        else {
            continue;
        };
        let ids = [k0, k1, k2];
        let enc = tables.tri_f[crate::scorer::tri_idx(ids[0], ids[1], ids[2])];
        let tg = [
            id_char(letter_ids[0]),
            id_char(letter_ids[1]),
            id_char(letter_ids[2]),
        ];
        for (pos, &ch) in [tg[0], tg[1], tg[2]].iter().enumerate() {
            let f = decode_finger_at(enc, pos);
            *agg.entry(ch).or_default().entry(f).or_insert(0.0) += cnt;
            *ctx.entry(ch)
                .or_default()
                .entry(f)
                .or_default()
                .entry(tg)
                .or_insert(0.0) += cnt;
        }
    }
    let mut guide = HashMap::new();
    for ch in LETTERS.chars() {
        let Some(key) = m[letter_id(ch).unwrap()] else {
            continue;
        };
        let default = model.default_finger[key];
        let entry = agg.get(&ch).cloned().unwrap_or_default();
        let total: f64 = entry.values().sum::<f64>().max(1.0);
        let mut share: Vec<(usize, f64)> =
            entry.iter().map(|(f, c)| (*f, c / total * 100.0)).collect();
        share.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| finger_label(a.0).cmp(&finger_label(b.0)))
        });
        let mut alt_contexts = Vec::new();
        for &(f, _) in &share {
            if f == default {
                continue;
            }
            let mut items: Vec<([char; 3], f64)> = ctx
                .get(&ch)
                .and_then(|m| m.get(&f))
                .map(|m| m.iter().map(|(k, v)| (*k, *v)).collect())
                .unwrap_or_default();
            items.sort_by(|a, b| {
                b.1.partial_cmp(&a.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.0.cmp(&b.0))
            });
            items.truncate(8);
            alt_contexts.push((
                f,
                items
                    .into_iter()
                    .map(|(tg, n)| (tg.iter().collect(), n))
                    .collect(),
            ));
        }
        guide.insert(
            ch,
            GuideEntry {
                key: KEYS[key].to_string(),
                default,
                share,
                alt_contexts,
            },
        );
    }
    guide
}

pub struct AnalyzeArgs<'a> {
    pub layout: Option<&'a str>,
    pub config: Option<&'a Path>,
    pub name: Option<&'a str>,
    pub primary_corpus: &'a [String],
    pub secondary_corpus: &'a [String],
    pub out: Option<&'a Path>,
    pub out_json: Option<&'a Path>,
}

pub fn run(args: AnalyzeArgs) -> Result<()> {
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
    let out_path = args
        .out
        .map(Path::to_path_buf)
        .unwrap_or_else(|| paths::result_path("analysis", args.name, ".txt"));
    let out_json = args
        .out_json
        .map(Path::to_path_buf)
        .unwrap_or_else(|| paths::result_path("analysis", args.name, ".json"));

    let (corpus, primary_paths) = if !args.primary_corpus.is_empty() {
        (
            context::load_paths(args.primary_corpus)?,
            args.primary_corpus.to_vec(),
        )
    } else {
        context::load_role(&ctx.cfg, "primary")?
    };
    let (secondary_corpus, secondary_paths) = if !args.secondary_corpus.is_empty() {
        (
            Some(context::load_paths(args.secondary_corpus)?),
            args.secondary_corpus.to_vec(),
        )
    } else {
        match context::load_role(&ctx.cfg, "secondary") {
            Ok((corpus, labels)) => (Some(corpus), labels),
            Err(_) => (None, Vec::new()),
        }
    };

    let (mut report, (mut pats, (use_, guide))) = rayon::join(
        || category_report(layout, &corpus, &ctx.tables, &ctx.geom),
        || {
            rayon::join(
                || {
                    trigram_patterns(
                        layout,
                        &corpus,
                        &ctx.tables,
                        &ctx.geom,
                        ctx.cfg.redirect_tiers.as_ref(),
                    )
                },
                || {
                    rayon::join(
                        || finger_usage(layout, &corpus, &ctx.tables, &ctx.geom),
                        || fingering_guide(layout, &corpus, &ctx.tables, &ctx.model),
                    )
                },
            )
        },
    );
    pats.extend(true_same_finger_metrics(
        layout,
        &corpus,
        &ctx.tables,
        &ctx.model.default_finger,
    ));
    if let Some(tiers) = ctx.cfg.roll_tiers.as_ref() {
        for (i, tier) in tiers.tiers().iter().enumerate() {
            let share: f64 = tier
                .iter()
                .map(|&pair| report.cats.get(&pair_key(pair)).copied().unwrap_or(0.0))
                .sum();
            report.cats.insert(format!("roll_tier_{}", i + 1), share);
        }
    }
    let primary_provenance = if !args.primary_corpus.is_empty() {
        provenance::describe_paths(args.primary_corpus)?
    } else {
        provenance::describe_entries(&ctx.cfg, "primary")?
    };
    let secondary_provenance = if !args.secondary_corpus.is_empty() {
        provenance::describe_paths(args.secondary_corpus)?
    } else if !secondary_paths.is_empty() {
        provenance::describe_entries(&ctx.cfg, "secondary")?
    } else {
        Vec::new()
    };

    let primary_score = score_layout(layout, &corpus, &ctx.tables);
    let secondary_score = secondary_corpus
        .as_ref()
        .map(|c| score_layout(layout, c, &ctx.tables));
    let breakdown = score_breakdown(
        layout,
        &corpus,
        &ctx.tables,
        &ctx.model,
        &ctx.geom,
        &ctx.gates,
        &ctx.conditions,
    );
    let preference_primary = preferences::report(
        layout,
        &corpus,
        &ctx.tables,
        &ctx.model,
        &ctx.geom,
        ctx.cfg.redirect_tiers.as_ref(),
        &ctx.model.finger_strength,
    );
    let preference_secondary = secondary_corpus.as_ref().map(|corpus| {
        preferences::report(
            layout,
            corpus,
            &ctx.tables,
            &ctx.model,
            &ctx.geom,
            ctx.cfg.redirect_tiers.as_ref(),
            &ctx.model.finger_strength,
        )
    });

    let mut lines = Vec::new();
    let mut out = |s: String| {
        println!("{}", s);
        lines.push(s);
    };
    out(format!("layout: {}", layout));
    out(format!("fingering: {}", ctx.cfg.fingering.label()));
    out("grid (QWERTY order, left/right split 5+5):".to_string());
    let chars: Vec<char> = layout.chars().collect();
    for r in 0..3 {
        let row: String = chars[r * 10..(r + 1) * 10].iter().collect();
        out(format!("  {} | {}", &row[..5], &row[5..]));
    }
    out(format!(
        "primary={:.9}  corpus={}",
        primary_score,
        primary_paths.join(",")
    ));
    if let Some(score) = secondary_score {
        out(format!(
            "secondary={:.9}  corpus={}",
            score,
            secondary_paths.join(",")
        ));
    }
    out(String::new());
    out("== primary score decomposition (per trigram) ==".to_string());
    for (name, value) in [
        ("key_effort", breakdown.key_effort),
        ("key_press", breakdown.key_press),
        ("pair_vertical", breakdown.pair_vertical),
        ("pair_cross", breakdown.pair_cross),
        ("pair_spread", breakdown.pair_spread),
        ("pair_roll", breakdown.pair_roll),
        ("pair_leap", breakdown.pair_leap),
        ("pair_shift", breakdown.pair_shift),
        ("pair_cap", breakdown.pair_cap),
        ("pair_sfb", breakdown.pair_sfb),
        ("pair_alt", breakdown.pair_alt),
        ("pair_repeat", breakdown.pair_repeat),
        ("trigram_redirect", breakdown.trigram_redirect),
        ("trigram_continue", breakdown.trigram_continue),
        ("trigram_scissor", breakdown.trigram_scissor),
        ("trigram_sfs", breakdown.trigram_sfs),
        ("finger_strength", breakdown.finger_strength),
        ("gate_penalty", breakdown.gate_penalty),
        ("space", breakdown.space),
        ("sum", breakdown.sum()),
        ("total", breakdown.total),
    ] {
        out(format!("  {:<18} {:9.6}", name, value));
    }
    out(String::new());
    out("== corpus provenance ==".to_string());
    for (role, entries) in [
        ("primary", &primary_provenance),
        ("secondary", &secondary_provenance),
    ] {
        for prov in entries {
            out(format!(
                "  {:<9} {:<12} {} ({}) sha1={} parser={}",
                role,
                prov.label,
                prov.path.display(),
                prov.kind,
                &prov.combined_sha1[..12],
                prov.parser
            ));
        }
    }
    out("== bigram categories (% of letter bigrams, chosen fingering) ==".to_string());
    for k in [
        "sfb",
        "scissor_IM_hsb",
        "scissor_IM_fsb",
        "scissor_MR",
        "scissor_RP",
        "roll_in",
        "roll_out",
        "leap",
        "alt",
        "hand_alternation",
        "repeat",
    ] {
        out(format!(
            "  {:<16} {:6.2}",
            k,
            report.cats.get(k).copied().unwrap_or(0.0)
        ));
    }
    if let Some(tiers) = ctx.cfg.roll_tiers.as_ref() {
        out(String::new());
        out("== roll tiers (configured, % of letter bigrams) ==".to_string());
        for (i, tier) in tiers.tiers().iter().enumerate() {
            out(format!(
                "  {:<16} {:6.2}",
                format!("tier {}", i + 1),
                report
                    .cats
                    .get(&format!("roll_tier_{}", i + 1))
                    .copied()
                    .unwrap_or(0.0)
            ));
            for &pair in tier {
                out(format!(
                    "    {:<14} {:6.2}",
                    pair_label(pair),
                    report.cats.get(&pair_key(pair)).copied().unwrap_or(0.0)
                ));
            }
        }
    }
    out(String::new());
    out("== worst bigrams (chosen cost, fingering, count) ==".to_string());
    for (cost, bg, f1, f2, cnt) in report.worst.iter().take(20) {
        out(format!(
            "  {}  n={:8.0}  {:6.2}  {}{}",
            bg, cnt, cost, f1, f2
        ));
    }
    out(String::new());
    out("== scissors (weight=cost*n, kind, fingering) ==".to_string());
    for (_w, c, bg, f1, f2, cnt, pair) in report.scissors.iter().take(15) {
        out(format!(
            "  {}  n={:8.0}  cost={:5.2}  {:3}  {}{}",
            bg, cnt, c, pair, f1, f2
        ));
    }
    out(String::new());
    out("== trigram patterns (chosen fingering, % of trigrams) ==".to_string());
    for (label, key) in [
        ("alternate", "alternate"),
        ("hand_alternation", "hand_alternation"),
        ("inroll", "inroll"),
        ("outroll", "outroll"),
        ("total_rolls", "total_rolls"),
        ("onehand", "onehand"),
        ("redirect", "redirect"),
        ("redirect_weak", "redirect_weak"),
        ("total_redirects", "total_redirects"),
        ("legacy_sft", "sft"),
        ("legacy_sfb_tri", "sfb_tri"),
        ("legacy_sfb_sft", "sfb_sft"),
        ("legacy_sfs", "sfs"),
        ("legacy_same_finger_total", "same_finger_total"),
        ("repeat", "repeat"),
        ("other", "other"),
    ] {
        out(format!(
            "  {:<24} {:6.2}",
            label,
            pats.get(key).copied().unwrap_or(0.0)
        ));
    }
    out(String::new());
    out("== true same-finger metrics (chosen vs default fingering, % of trigrams) ==".to_string());
    out(format!(
        "  {:<20} {:>8} {:>8} {:>8}",
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
        out(format!(
            "  {:<20} {:8.2} {:8.2} {:8.2}",
            name,
            pats.get(name).copied().unwrap_or(0.0),
            pats.get(&format!("{name}_default")).copied().unwrap_or(0.0),
            pats.get(&format!("{name}_delta")).copied().unwrap_or(0.0)
        ));
    }
    out(
        "  note: legacy_* values include repeated physical keys and retain their budget meaning; delta = chosen - default"
            .to_string(),
    );
    if let Some(tiers) = ctx.cfg.redirect_tiers.as_ref() {
        out(String::new());
        out("== redirect tiers (configured, % of trigrams) ==".to_string());
        for (i, tier) in tiers.tiers().iter().enumerate() {
            out(format!(
                "  {:<16} {:6.2}  weight={:.3}",
                format!("tier {}", i + 1),
                pats.get(&format!("redirect_tier_{}", i + 1))
                    .copied()
                    .unwrap_or(0.0),
                tiers.base_cost(i)
            ));
            for &pattern in tier {
                out(format!(
                    "    {:<14} {:6.2}",
                    pattern_label(pattern),
                    pats.get(&pattern_key(pattern)).copied().unwrap_or(0.0)
                ));
            }
        }
        out(format!(
            "  {:<16} {:6.2}",
            "unclassified",
            pats.get("redirect_unclassified").copied().unwrap_or(0.0)
        ));
        out("  note: redirect/redirect_weak are deprecated legacy labels".to_string());
    }
    out(String::new());
    out("== finger load (% of all strokes) ==".to_string());
    for h in ['L', 'R'] {
        let parts: Vec<String> = ['P', 'R', 'M', 'I']
            .iter()
            .map(|f| {
                format!(
                    "{}={:5.2}%",
                    f,
                    use_.get(&format!("{}{}", h, f)).copied().unwrap_or(0.0)
                )
            })
            .collect();
        out(format!("  {}: {}", h, parts.join("  ")));
    }
    let shortcuts = shortcut_report(layout, &guide, &ctx.model);
    let cross_hand = cross_hand_report(layout, &corpus, &guide);
    let punctuation = punctuation_report(layout);
    out(String::new());
    out("== shortcut constraints (left Ctrl + key) ==".to_string());
    for row in &shortcuts {
        out(format!(
            "  {}  key={:<2} hand={:<5} chosen={:<2} ({:5.1}%)  default={:<2}  same_hand={:<3}  same_finger={}",
            row.letter,
            row.key,
            row.hand,
            finger_label(row.chosen),
            row.chosen_share,
            finger_label(row.default),
            if row.ctrl_same_hand { "yes" } else { "no" },
            if row.ctrl_same_finger { "yes" } else { "no" },
        ));
    }
    out("  note: left Ctrl is treated as LP; this section is report-only".to_string());
    out(String::new());
    out("== human factors (report-only) ==".to_string());
    out(format!(
        "  cross-hand alt usage: {:.2}% of letter strokes",
        cross_hand.total_share
    ));
    for row in &cross_hand.rows {
        let contexts: Vec<String> = row
            .contexts
            .iter()
            .take(5)
            .map(|(trigram, finger, count)| {
                format!("{}:{}:{:.0}", trigram, finger_label(*finger), count)
            })
            .collect();
        out(format!(
            "  {}@{} default {} {:.2}%  cross {:.2}%  {}",
            row.letter,
            row.key,
            finger_label(row.default),
            row.default_share,
            row.cross_share,
            contexts.join(", ")
        ));
    }
    let dead: Vec<String> = punctuation
        .dead_keys
        .iter()
        .map(|key| match key.shifted {
            Some(shifted) => format!("{}='{}' shift='{}'", key.key, key.symbol, shifted),
            None => format!("{}='{}'", key.key, key.symbol),
        })
        .collect();
    out("  punctuation and shift layer are unscored".to_string());
    out(format!("  dead keys: {}", dead.join("  ")));
    out(format!(
        "  apostrophe {:?} (shift {:?}) modeled={} keys={}",
        punctuation.apostrophe_symbol,
        punctuation.apostrophe_shifted,
        punctuation.apostrophe_modeled,
        punctuation.apostrophe_keys.join("/")
    ));
    for bigram in &punctuation.bigrams {
        out(format!(
            "  bigram {:?} ({}) crosses {}",
            bigram.bigram,
            bigram.kind,
            bigram.keys.join("+")
        ));
    }
    out("  note: alt fingering is not penalised; punctuation is not scored".to_string());
    out(String::new());
    out("== preference metrics (primary) ==".to_string());
    for line in preferences::top_summary(&preference_primary) {
        out(format!("  {}", line));
    }
    out("  (full top-25 lists and per-pattern costs are in the JSON report)".to_string());
    if let Some(report) = preference_secondary.as_ref() {
        out(String::new());
        out("== preference metrics (secondary) ==".to_string());
        for line in preferences::top_summary(report) {
            out(format!("  {}", line));
        }
        out("  (full top-25 lists and per-pattern costs are in the JSON report)".to_string());
    }
    out(String::new());
    out("== alpha fingering guide (trigram DP) ==".to_string());
    for ch in LETTERS.chars() {
        let Some(g) = guide.get(&ch) else { continue };
        let shares: Vec<String> = g
            .share
            .iter()
            .map(|(f, v)| format!("{}={:.1}%", finger_label(*f), v))
            .collect();
        out(format!(
            "  {}@{:<2} default {}  {}",
            ch,
            g.key,
            finger_label(g.default),
            shares.join(" ")
        ));
        for (f, tops) in &g.alt_contexts {
            let text: Vec<String> = tops
                .iter()
                .take(5)
                .map(|(tg, n)| format!("{}:{:.0}", tg, n))
                .collect();
            out(format!(
                "      alt {}: {}",
                finger_label(*f),
                text.join(", ")
            ));
        }
    }
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&out_path, lines.join("\n") + "\n")?;
    if let Some(parent) = out_json.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let guide_json: serde_json::Value = guide
        .iter()
        .map(|(ch, g)| {
            let share: serde_json::Map<String, serde_json::Value> = g
                .share
                .iter()
                .map(|(f, v)| (finger_label(*f), json!(v)))
                .collect();
            let alts: serde_json::Map<String, serde_json::Value> = g
                .alt_contexts
                .iter()
                .map(|(f, items)| {
                    (
                        finger_label(*f),
                        json!(items
                            .iter()
                            .map(|(tg, n)| json!([tg, n.round_ties_even() as i64]))
                            .collect::<Vec<_>>()),
                    )
                })
                .collect();
            (
                ch.to_string(),
                json!({
                    "key": g.key,
                    "default": finger_label(g.default),
                    "share": share,
                    "alt_contexts": alts,
                }),
            )
        })
        .collect();
    let payload = json!({
        "layout": layout,
        "fingering": ctx.cfg.fingering.label(),
        "primary_score": primary_score,
        "score_breakdown": breakdown.to_json(),
        "primary_corpora": primary_paths,
        "secondary_score": secondary_score,
        "secondary_corpora": secondary_paths,
        "corpus_provenance": {
            "primary": primary_provenance.iter().map(provenance::Provenance::to_json).collect::<Vec<_>>(),
            "secondary": secondary_provenance.iter().map(provenance::Provenance::to_json).collect::<Vec<_>>(),
        },
        "categories": report.cats,
        "worst_bigrams": report.worst.iter().map(|(c,b,f1,f2,n)| json!([c,b,f1.to_string(),f2.to_string(),n])).collect::<Vec<_>>(),
        "scissors": report.scissors.iter().map(|(w,c,b,f1,f2,n,p)| json!([w,c,b,f1.to_string(),f2.to_string(),n,p])).collect::<Vec<_>>(),
        "trigram_patterns": pats,
        "usage": use_,
        "guide": guide_json,
        "shortcuts": shortcuts.iter().map(|row| {
            (
                row.letter.to_string(),
                json!({
                    "key": row.key,
                    "hand": row.hand,
                    "chosen": finger_label(row.chosen),
                    "chosen_share": row.chosen_share,
                    "default": finger_label(row.default),
                    "ctrl_same_hand": row.ctrl_same_hand,
                    "ctrl_same_finger": row.ctrl_same_finger,
                }),
            )
        }).collect::<serde_json::Map<_, _>>(),
        "human_factors": {
            "cross_hand": {
                "total_share": cross_hand.total_share,
                "letters": cross_hand.rows.iter().map(|row| {
                    (
                        row.letter.to_string(),
                        json!({
                            "key": row.key,
                            "default": finger_label(row.default),
                            "default_share": row.default_share,
                            "cross_share": row.cross_share,
                            "contexts": row.contexts.iter().map(|(trigram, finger, count)| {
                                json!([trigram, finger_label(*finger), count])
                            }).collect::<Vec<_>>(),
                        }),
                    )
                }).collect::<serde_json::Map<_, _>>(),
            },
            "punctuation": {
                "dead_keys": punctuation.dead_keys.iter().map(|key| {
                    json!({
                        "key": key.key,
                        "symbol": key.symbol.to_string(),
                        "shifted": key.shifted.map(|ch| ch.to_string()),
                    })
                }).collect::<Vec<_>>(),
                "apostrophe": {
                    "symbol": punctuation.apostrophe_symbol.to_string(),
                    "shifted": punctuation.apostrophe_shifted.to_string(),
                    "modeled": punctuation.apostrophe_modeled,
                    "keys": &punctuation.apostrophe_keys,
                },
                "shift_layer_scored": false,
                "bigrams": punctuation.bigrams.iter().map(|bigram| {
                    json!({
                        "bigram": bigram.bigram,
                        "kind": bigram.kind,
                        "keys": &bigram.keys,
                    })
                }).collect::<Vec<_>>(),
            },
        },
        "preference": {
            "primary": preference_primary.to_json(),
            "secondary": preference_secondary.as_ref().map(|report| report.to_json()),
        },
    });
    std::fs::write(&out_json, serde_json::to_string_pretty(&payload)? + "\n")?;
    println!("\nwrote {} and {}", out_path.display(), out_json.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        classify_trigram, is_same_finger_skipgram, trigram_pattern, true_same_finger_metrics,
    };
    use crate::redirect::RedirectTiers;

    fn fingers(pattern: [u8; 3], hand: usize) -> [usize; 3] {
        pattern.map(|rank| hand * 4 + (4 - rank) as usize)
    }

    fn tiers() -> RedirectTiers {
        RedirectTiers::new(vec![vec![[1, 2, 1]], vec![[3, 2, 3]], vec![[1, 4, 1]]])
    }

    #[test]
    fn score_breakdown_reconstructs_the_primary_score() {
        let ctx = crate::context::Context::load(None).expect("context");
        let (corpus, _) = crate::context::load_role(&ctx.cfg, "primary").expect("corpus");
        let layout = "bthcrmdisjnply;eougavkxz,q.wf/";
        let score = crate::evaluate::score_layout(layout, &corpus, &ctx.tables);
        let breakdown = super::score_breakdown(
            layout,
            &corpus,
            &ctx.tables,
            &ctx.model,
            &ctx.geom,
            &ctx.gates,
            &ctx.conditions,
        );
        assert!((breakdown.total - score).abs() < 1e-9);
        assert!(
            (breakdown.sum() - score).abs() < 1e-5,
            "sum={} score={} diff={} breakdown={:?}",
            breakdown.sum(),
            score,
            breakdown.sum() - score,
            breakdown
        );
    }

    #[test]
    fn finger_strength_term_is_corpus_weighted_and_geometry_independent() {
        let ctx = crate::context::Context::load(None).expect("context");
        let (corpus, _) = crate::context::load_role(&ctx.cfg, "primary").expect("corpus");
        let layout = "bthcrmdisjnply;eougavkxz,q.wf/";
        let breakdown = super::score_breakdown(
            layout,
            &corpus,
            &ctx.tables,
            &ctx.model,
            &ctx.geom,
            &ctx.gates,
            &ctx.conditions,
        );
        let m = super::keymap(layout);
        let mut expected = 0.0;
        let mut total = 0.0;
        for (ids, &count) in corpus.tri_ids.iter().zip(&corpus.tri_counts) {
            let (Some(a), Some(b), Some(c)) = (m[ids[0]], m[ids[1]], m[ids[2]]) else {
                continue;
            };
            let keys = [a, b, c];
            let fingers = ctx.tables.tri_f[crate::scorer::tri_idx(a, b, c)];
            for (key, finger) in keys.into_iter().zip(fingers) {
                if key != crate::scorer::SPACE_ID {
                    expected += ctx.model.finger_strength[finger as usize] * count;
                }
            }
            total += count;
        }
        expected /= total;
        assert!(breakdown.finger_strength > 0.0);
        assert!((breakdown.finger_strength - expected).abs() < 1e-12);
    }

    #[test]
    fn same_finger_skipgram_detects_non_adjacent_repeats() {
        assert!(is_same_finger_skipgram([3, 6, 3]));
        assert!(!is_same_finger_skipgram([3, 3, 6]));
        assert!(!is_same_finger_skipgram([3, 3, 3]));
        assert!(!is_same_finger_skipgram([2, 3, 4]));
    }

    #[test]
    fn key_aware_classifier_separates_repeats_from_true_same_finger() {
        let same = [3, 3, 3];

        let aaa = classify_trigram([0, 0, 0], same);
        assert!(aaa.repeat_adjacent && aaa.repeat_all && !aaa.repeat_skip);
        assert!(!aaa.sfb_true && !aaa.sft_true && !aaa.sfs_true);

        let aba = classify_trigram([0, 1, 0], [3, 2, 3]);
        assert!(aba.repeat_skip && aba.repeat_all && !aba.repeat_adjacent);
        assert!(!aba.sfb_true && !aba.sft_true && !aba.sfs_true);

        let abb = classify_trigram([0, 1, 1], [3, 2, 2]);
        assert!(abb.repeat_adjacent && abb.repeat_all && !abb.repeat_skip);
        assert!(!abb.sfb_true && !abb.sft_true && !abb.sfs_true);

        let abc_sft = classify_trigram([0, 1, 2], same);
        assert!(abc_sft.sft_true);
        assert!(!abc_sft.sfb_true && !abc_sft.sfs_true && !abc_sft.repeat_all);

        let abc_sfb = classify_trigram([0, 1, 2], [3, 3, 6]);
        assert!(abc_sfb.sfb_true);
        assert!(!abc_sfb.sft_true && !abc_sfb.sfs_true);

        let abc_sfs = classify_trigram([0, 1, 2], [3, 2, 3]);
        assert!(abc_sfs.sfs_true);
        assert!(!abc_sfs.sfb_true && !abc_sfs.sft_true);

        let abc_clean = classify_trigram([0, 1, 2], [3, 2, 6]);
        assert!(
            !abc_clean.sfb_true
                && !abc_clean.sft_true
                && !abc_clean.sfs_true
                && !abc_clean.repeat_all
        );
    }

    #[test]
    fn default_sfb_can_disappear_with_chosen_alt_finger() {
        let keys = [0, 1, 2];
        let default_fingers = [3, 3, 6];
        let chosen_alt_fingers = [3, 2, 6];
        assert!(classify_trigram(keys, default_fingers).sfb_true);
        assert!(!classify_trigram(keys, chosen_alt_fingers).sfb_true);
    }

    #[test]
    fn true_metrics_are_repeat_free_on_the_shipped_layout() {
        let ctx = crate::context::Context::load(None).expect("context");
        let (corpus, _) = crate::context::load_role(&ctx.cfg, "primary").expect("corpus");
        let layout = "bthcrmdisjnply;eougavkxz,q.wf/";
        let pats = super::trigram_patterns(
            layout,
            &corpus,
            &ctx.tables,
            &ctx.geom,
            ctx.cfg.redirect_tiers.as_ref(),
        );
        let metrics =
            true_same_finger_metrics(layout, &corpus, &ctx.tables, &ctx.model.default_finger);
        assert!(metrics["sfb_true"] < 1e-9, "{metrics:?}");
        assert!(metrics["sft_true"] < 1e-9, "{metrics:?}");
        assert!(metrics["sfb_sft_true"] < 1e-9, "{metrics:?}");
        let legacy = pats["sfb_sft"];
        assert!(
            (metrics["repeat_adjacent"] - legacy).abs() < 0.01,
            "repeat_adjacent={} legacy={legacy}",
            metrics["repeat_adjacent"]
        );
    }

    #[test]
    fn trigram_categories_separate_sft_from_sfb() {
        assert_eq!(trigram_pattern([3, 3, 3], None).category, "sft");
        assert_eq!(trigram_pattern([2, 2, 6], None).category, "sfb_tri");
        assert_eq!(trigram_pattern([1, 1, 2], None).category, "sfb_tri");
        assert_eq!(trigram_pattern([2, 2, 7], None).category, "sfb_tri");
        assert_eq!(trigram_pattern([3, 6, 3], None).category, "alternate");
    }

    #[test]
    fn redirect_tiers_are_shared_by_both_hands() {
        let tiers = tiers();
        let left = trigram_pattern(fingers([1, 2, 1], 0), Some(&tiers));
        let right = trigram_pattern(fingers([1, 2, 1], 1), Some(&tiers));
        assert_eq!(left.category, "redirect");
        assert_eq!(left.redirect, Some([1, 2, 1]));
        assert_eq!(left.tier, Some(0));
        assert_eq!(right.tier, Some(0));
        assert_eq!(left.label(Some(&tiers)), "redirect_tier_1:1-2-1");

        let ring = trigram_pattern(fingers([3, 2, 3], 0), Some(&tiers));
        assert_eq!(ring.tier, Some(1));
        assert_eq!(ring.label(Some(&tiers)), "redirect_tier_2:3-2-3");

        let pinky = trigram_pattern(fingers([1, 4, 1], 0), Some(&tiers));
        assert_eq!(pinky.tier, Some(2));
    }

    #[test]
    fn unlisted_redirects_use_the_legacy_fallback() {
        let tiers = tiers();
        let pattern = trigram_pattern(fingers([1, 3, 2], 0), Some(&tiers));
        assert_eq!(pattern.category, "redirect");
        assert_eq!(pattern.redirect, Some([1, 3, 2]));
        assert_eq!(pattern.tier, None);
        assert_eq!(pattern.label(Some(&tiers)), "redirect_unclassified:1-3-2");
        assert_eq!(pattern.label(None), "redirect");
    }

    #[test]
    fn human_factor_reports_pin_cross_hand_and_punctuation() {
        let ctx = crate::context::Context::load(None).expect("context");
        let (corpus, _) = crate::context::load_role(&ctx.cfg, "primary").expect("corpus");
        let layout = "bthcrmdisjnply;eougavkxz,q.wf/";
        let guide = super::fingering_guide(layout, &corpus, &ctx.tables, &ctx.model);
        let cross = super::cross_hand_report(layout, &corpus, &guide);
        let m = cross.rows.iter().find(|row| row.letter == 'm').unwrap();
        assert_eq!(m.key, "Y");
        assert!((m.cross_share - 99.48).abs() < 0.1, "{m:?}");
        let e = cross.rows.iter().find(|row| row.letter == 'e').unwrap();
        assert_eq!(e.key, "H");
        assert!((e.cross_share - 30.83).abs() < 0.1, "{e:?}");
        assert!((cross.total_share - 6.20).abs() < 0.05, "{cross:?}");

        let punctuation = super::punctuation_report(layout);
        let dead: Vec<&str> = punctuation
            .dead_keys
            .iter()
            .map(|row| row.key.as_str())
            .collect();
        assert_eq!(dead, ["G", "B", "M", "/"]);
        assert!(!punctuation.apostrophe_modeled);
        assert_eq!(punctuation.apostrophe_keys, ["AC11/LSGT"]);
        assert!(punctuation
            .bigrams
            .iter()
            .any(|bigram| bigram.bigram == "; "));
        assert!(punctuation
            .bigrams
            .iter()
            .any(|bigram| bigram.bigram == "'s"));
    }
}
