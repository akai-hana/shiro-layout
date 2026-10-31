//! Layout scoring and the permutation evaluator used by generation.

use crate::corpus::{Corpus, SPACE_INDEX};
use crate::geometry::{finger_index, key_index, KEYS, NK};
use crate::model::Model;
use crate::scorer::{tri_idx, Tables, SPACE_ID};

pub const LETTERS: &str = "abcdefghijklmnopqrstuvwxyz";
pub const FORDER: [char; 4] = ['P', 'R', 'M', 'I'];

pub fn keymap(layout: &str) -> Vec<(char, usize)> {
    let chars: Vec<char> = layout.chars().collect();
    let mut out = Vec::new();
    for (i, ch) in chars.iter().enumerate() {
        if ch.is_ascii_alphabetic() && i < NK {
            if let Some(k) = key_index(KEYS[i]) {
                out.push((*ch, k));
            }
        }
    }
    out
}

pub fn remap_for_layout(layout: &str) -> [i32; 27] {
    let mut remap = [-1i32; 27];
    for (ch, key) in keymap(layout) {
        if ch.is_ascii_lowercase() {
            remap[(ch as u8 - b'a') as usize] = key as i32;
        }
    }
    remap[SPACE_INDEX] = SPACE_ID as i32;
    remap
}

pub fn score_layout(layout: &str, corpus: &Corpus, tables: &Tables) -> f64 {
    let remap = remap_for_layout(layout);
    let mut total = 0.0;
    let mut weighted = 0.0;
    for (ids, &count) in corpus.tri_ids.iter().zip(corpus.tri_counts.iter()) {
        let a = remap[ids[0]];
        let b = remap[ids[1]];
        let c = remap[ids[2]];
        if a < 0 || b < 0 || c < 0 {
            continue;
        }
        total += count;
        let idx = tri_idx(a as usize, b as usize, c as usize);
        weighted += tables.tri[idx] * count;
    }
    if total == 0.0 {
        0.0
    } else {
        weighted / total
    }
}

/// `slots_of`: letter -> slot index among the 26 non-dead keys.
pub fn slots_of(layout: &str) -> Result<(Vec<usize>, Vec<(char, usize)>), String> {
    let chars: Vec<char> = layout.chars().collect();
    if chars.len() != 30 {
        return Err(format!("layout must be 30 characters, got {}", chars.len()));
    }
    let mut dead = Vec::new();
    let mut slots = Vec::new();
    for (i, ch) in chars.iter().enumerate() {
        if ch.is_ascii_lowercase() {
            slots.push(i);
        } else {
            dead.push((*ch, i));
        }
    }
    if slots.len() != 26 {
        return Err(format!(
            "layout must leave 26 letter slots, got {}",
            slots.len()
        ));
    }
    let mut seen = [false; 26];
    let mut perm = vec![0usize; 26];
    for (li, ch) in chars.iter().enumerate() {
        if !ch.is_ascii_lowercase() {
            continue;
        }
        let letter = (*ch as u8 - b'a') as usize;
        if seen[letter] {
            return Err(format!("layout repeats letter {ch:?}"));
        }
        seen[letter] = true;
        let slot_idx = slots.iter().position(|&s| s == li).unwrap();
        perm[letter] = slot_idx;
    }
    Ok((perm, dead))
}

pub fn layout_str(perm: &[usize], dead: &[(char, usize)]) -> String {
    let mut arr = vec![' '; 30];
    for (ch, pos) in dead {
        arr[*pos] = *ch;
    }
    let mut letters = perm.iter().enumerate().collect::<Vec<_>>();
    letters.sort_by_key(|(_, slot)| **slot);
    for (li, slot_idx) in letters {
        let pos = slot_positions(dead)[*slot_idx];
        arr[pos] = LETTERS.as_bytes()[li] as char;
    }
    arr.into_iter().collect()
}

fn slot_positions(dead: &[(char, usize)]) -> Vec<usize> {
    (0..30)
        .filter(|i| !dead.iter().any(|(_, p)| p == i))
        .collect()
}

pub struct Evaluator<'a> {
    pub corpus: &'a Corpus,
    pub tables: &'a Tables,
    pub model: &'a Model,
    pub dead: Vec<(char, usize)>,
    pub dead_count: usize,
    pub slots: Vec<usize>,
    pub slot_key: Vec<usize>,
    pub default_finger: Vec<usize>,
    pub hand: Vec<usize>,
    pub lfreq: Vec<f64>,
    pub tri_idx_slot: Vec<[usize; 3]>,
    pub tri_counts: Vec<f64>,
    pub inc: Vec<Vec<u32>>,
    pub n_tri: f64,
    pub w_balance: f64,
    pub w_hand: f64,
    pub flat_share: [f64; 4],
}

impl<'a> Evaluator<'a> {
    pub fn new(
        corpus: &'a Corpus,
        tables: &'a Tables,
        model: &'a Model,
        dead: &[(char, usize)],
        flat_share: [f64; 4],
        w_balance: f64,
        w_hand: f64,
    ) -> Result<Self, String> {
        let mut slots: Vec<usize> = Vec::new();
        for i in 0..30 {
            if !dead.iter().any(|(_, p)| *p == i) {
                slots.push(i);
            }
        }
        if slots.len() != 26 {
            return Err(format!("need 26 letter slots, got {}", slots.len()));
        }
        let slot_key: Vec<usize> = slots.to_vec();
        let default_finger: Vec<usize> = slots
            .iter()
            .map(|&i| finger_index(model.default_finger[i]).unwrap())
            .collect();
        let hand: Vec<usize> = slots
            .iter()
            .map(|&i| crate::geometry::hand_of(KEYS[i]))
            .collect();
        let mut lfreq = vec![0.0; 26];
        for (i, ch) in LETTERS.chars().enumerate() {
            lfreq[i] = *corpus.letters.get(&ch).unwrap_or(&0.0);
        }
        let sum: f64 = lfreq.iter().sum();
        if sum > 0.0 {
            for v in &mut lfreq {
                *v /= sum;
            }
        }
        let mut tri_idx_slot = Vec::new();
        let mut tri_counts = Vec::new();
        let mut inc: Vec<Vec<u32>> = vec![Vec::new(); 26];
        for (ids, &count) in corpus.tri_ids.iter().zip(corpus.tri_counts.iter()) {
            if ids.iter().all(|&c| c < 26) {
                let k = tri_idx_slot.len() as u32;
                tri_idx_slot.push([ids[0], ids[1], ids[2]]);
                tri_counts.push(count);
                for &id in ids {
                    inc[id].push(k);
                }
            }
        }
        let n_tri = tri_counts.iter().sum::<f64>();
        Ok(Self {
            corpus,
            tables,
            model,
            dead: dead.to_vec(),
            dead_count: dead.len(),
            slots,
            slot_key,
            default_finger,
            hand,
            lfreq,
            tri_idx_slot,
            tri_counts,
            inc,
            n_tri: if n_tri == 0.0 { 1.0 } else { n_tri },
            w_balance,
            w_hand,
            flat_share,
        })
    }

    pub fn key_ids(&self, perm: &[usize]) -> Vec<usize> {
        perm.iter().map(|&slot| self.slot_key[slot]).collect()
    }

    pub fn weighted(&self, perm: &[usize]) -> f64 {
        let kid = self.key_ids(perm);
        let mut weighted = 0.0;
        for (ids, &count) in self.tri_idx_slot.iter().zip(self.tri_counts.iter()) {
            let idx = tri_idx(kid[ids[0]], kid[ids[1]], kid[ids[2]]);
            weighted += self.tables.tri[idx] * count;
        }
        weighted
    }

    pub fn penalty(&self, _perm: &[usize]) -> f64 {
        let mut usage = [[0.0f64; 4]; 2];
        for li in 0..26 {
            usage[self.hand[li]][self.default_finger[li]] += self.lfreq[li];
        }
        let mut pen = 0.0;
        for h in 0..2 {
            let total: f64 = usage[h].iter().sum();
            if total <= 0.0 {
                continue;
            }
            for fi in 0..4 {
                let share = usage[h][fi] / total;
                pen += ((share - self.flat_share[fi]) / self.flat_share[fi]).powi(2);
            }
        }
        pen *= self.w_balance;
        let hl = usage[0].iter().sum::<f64>();
        let hr = usage[1].iter().sum::<f64>();
        pen += ((hl - hr) / (hl + hr).max(1e-9)).powi(2) * self.w_hand;
        pen
    }

    pub fn score(&self, perm: &[usize]) -> f64 {
        self.weighted(perm) / self.n_tri + self.penalty(perm)
    }

    pub fn layout_str(&self, perm: &[usize]) -> String {
        let mut arr = vec![' '; 30];
        for (ch, pos) in &self.dead {
            arr[*pos] = *ch;
        }
        for (li, slot_idx) in perm.iter().enumerate() {
            let pos = self.slots[*slot_idx];
            arr[pos] = LETTERS.as_bytes()[li] as char;
        }
        arr.into_iter().collect()
    }
}

/// Non-letter positions of a layout, preserving their characters.
pub fn dead_of(layout: &str) -> Vec<(char, usize)> {
    layout
        .chars()
        .enumerate()
        .filter(|(_, ch)| !ch.is_ascii_lowercase())
        .map(|(i, ch)| (ch, i))
        .collect()
}

/// Letter-to-slot permutation in layout order (Python `search.slots_of`).
pub fn perm_of(layout: &str, _dead: &[(char, usize)]) -> Result<Vec<usize>, String> {
    let chars: Vec<char> = layout.chars().collect();
    if chars.len() != 30 {
        return Err(format!("layout must be 30 characters, got {}", chars.len()));
    }
    let letters: Vec<char> = chars
        .into_iter()
        .filter(|c| c.is_ascii_lowercase())
        .collect();
    if letters.len() != 26 {
        return Err(format!(
            "layout must contain 26 letters, got {}",
            letters.len()
        ));
    }
    let distinct: std::collections::HashSet<char> = letters.iter().copied().collect();
    if distinct.len() != 26 {
        return Err("layout does not contain 26 distinct letters".to_string());
    }
    let mut perm = vec![0usize; 26];
    for (slot, ch) in letters.into_iter().enumerate() {
        perm[(ch as u8 - b'a') as usize] = slot;
    }
    Ok(perm)
}

pub fn flat_share_from_config(cfg: &crate::config::Config) -> [f64; 4] {
    [
        *cfg.flat_share.get(&'P').unwrap_or(&0.18),
        *cfg.flat_share.get(&'R').unwrap_or(&0.25),
        *cfg.flat_share.get(&'M').unwrap_or(&0.28),
        *cfg.flat_share.get(&'I').unwrap_or(&0.29),
    ]
}
