//! Layout scoring and the permutation evaluator used by generation.

use rand::seq::SliceRandom;
use rand::Rng;

use crate::corpus::{Corpus, SPACE_INDEX};
use crate::geometry::{finger_index, key_index, KEYS, NK};
use crate::model::Model;
use crate::scorer::{tri_idx, tri_idx_u8, Tables, SPACE_ID};

pub const LETTERS: &str = "abcdefghijklmnopqrstuvwxyz";
pub const FORDER: [char; 4] = ['P', 'R', 'M', 'I'];
pub const N_LETTERS: usize = 26;
pub const N_ITEMS: usize = NK;

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

pub struct Evaluator<'a> {
    pub corpus: &'a Corpus,
    pub tables: &'a Tables,
    pub model: &'a Model,
    pub punctuation: Vec<char>,
    pub slots: Vec<usize>,
    pub slot_key: Vec<u8>,
    pub default_finger: Vec<usize>,
    pub hand: Vec<usize>,
    pub lfreq: Vec<f64>,
    pub tri_idx_slot: Vec<[u8; 3]>,
    pub tri_counts: Vec<f64>,
    pub inc: Vec<Vec<u32>>,
    pub n_tri: f64,
    pub w_balance: f64,
    pub w_hand: f64,
    pub flat_share: [f64; 4],
    pub hand_restrictions: [Option<usize>; 26],
}

impl<'a> Evaluator<'a> {
    pub fn new(
        corpus: &'a Corpus,
        tables: &'a Tables,
        model: &'a Model,
        punctuation: &[char],
        flat_share: [f64; 4],
        w_balance: f64,
        w_hand: f64,
    ) -> Result<Self, String> {
        if punctuation.len() != N_ITEMS - N_LETTERS {
            return Err(format!(
                "need {} punctuation characters, got {}",
                N_ITEMS - N_LETTERS,
                punctuation.len()
            ));
        }
        let slots: Vec<usize> = (0..N_ITEMS).collect();
        let slot_key: Vec<u8> = slots.iter().map(|&slot| slot as u8).collect();
        let default_finger: Vec<usize> = (0..N_ITEMS)
            .map(|i| finger_index(model.default_finger[i]).unwrap())
            .collect();
        let hand: Vec<usize> = (0..N_ITEMS)
            .map(|i| crate::geometry::hand_of(KEYS[i]))
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
        let mut inc: Vec<Vec<u32>> = vec![Vec::new(); N_ITEMS];
        for (ids, &count) in corpus.tri_ids.iter().zip(corpus.tri_counts.iter()) {
            if ids.iter().all(|&c| c < 26) {
                let k = tri_idx_slot.len() as u32;
                tri_idx_slot.push([ids[0] as u8, ids[1] as u8, ids[2] as u8]);
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
            punctuation: punctuation.to_vec(),
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
            hand_restrictions: [None; 26],
        })
    }

    pub fn set_hand_restrictions(&mut self, restrictions: [Option<usize>; 26]) {
        self.hand_restrictions = restrictions;
    }

    #[inline]
    fn slot_hand_allowed(&self, letter: usize, slot: usize) -> bool {
        self.hand_restrictions[letter].is_none_or(|hand| self.hand[slot] == hand)
    }

    #[inline]
    fn item_hand_allowed(&self, item: usize, slot: usize) -> bool {
        item >= N_LETTERS || self.slot_hand_allowed(item, slot)
    }

    pub fn perm_allowed(&self, perm: &[usize]) -> bool {
        (0..N_ITEMS).all(|item| self.item_hand_allowed(item, perm[item]))
    }

    pub fn swap_allowed(&self, perm: &[usize], a: usize, b: usize) -> bool {
        self.item_hand_allowed(a, perm[b]) && self.item_hand_allowed(b, perm[a])
    }

    pub fn cycle3_allowed(&self, perm: &[usize], a: usize, b: usize, c: usize) -> bool {
        self.item_hand_allowed(a, perm[b])
            && self.item_hand_allowed(b, perm[c])
            && self.item_hand_allowed(c, perm[a])
    }

    pub fn constrained_shuffle(&self, rng: &mut impl Rng) -> Result<Vec<usize>, String> {
        let mut slots_by_hand: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
        for slot in 0..N_ITEMS {
            slots_by_hand[self.hand[slot]].push(slot);
        }
        let mut letters_by_hand: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
        let mut free = Vec::new();
        for letter in 0..N_LETTERS {
            match self.hand_restrictions[letter] {
                Some(hand) => letters_by_hand[hand].push(letter),
                None => free.push(letter),
            }
        }
        for item in N_LETTERS..N_ITEMS {
            free.push(item);
        }
        let mut perm = vec![usize::MAX; N_ITEMS];
        let mut used = [false; N_ITEMS];
        for hand in 0..2 {
            if letters_by_hand[hand].len() > slots_by_hand[hand].len() {
                return Err(format!(
                    "hand restrictions require {} letters on the {} hand, but only {} letter keys are available",
                    letters_by_hand[hand].len(),
                    if hand == 0 { "left" } else { "right" },
                    slots_by_hand[hand].len()
                ));
            }
            slots_by_hand[hand].shuffle(rng);
            for (i, &letter) in letters_by_hand[hand].iter().enumerate() {
                let slot = slots_by_hand[hand][i];
                perm[letter] = slot;
                used[slot] = true;
            }
        }
        let mut remaining: Vec<usize> = (0..N_ITEMS).filter(|slot| !used[*slot]).collect();
        remaining.shuffle(rng);
        free.shuffle(rng);
        for (item, slot) in free.into_iter().zip(remaining) {
            perm[item] = slot;
        }
        if perm.contains(&usize::MAX) {
            return Err("could not assign every item to a slot".to_string());
        }
        Ok(perm)
    }

    pub fn perturb(&self, perm: &[usize], rng: &mut impl Rng, k: usize) -> Vec<usize> {
        let mut cand = perm.to_vec();
        let mut idx: Vec<usize> = (0..N_ITEMS).collect();
        idx.shuffle(rng);
        let moved = &idx[..(2 * k).min(idx.len())];
        let mut a = 0;
        while a + 1 < moved.len() {
            let i = moved[a];
            let j = moved[a + 1];
            if self.swap_allowed(&cand, i, j) {
                cand.swap(i, j);
            }
            a += 2;
        }
        cand
    }

    pub fn key_ids(&self, perm: &[usize]) -> [u8; N_ITEMS] {
        let mut kid = [0u8; N_ITEMS];
        for (li, &slot) in perm.iter().enumerate() {
            kid[li] = self.slot_key[slot];
        }
        kid
    }

    pub fn weighted(&self, perm: &[usize]) -> f64 {
        let kid = self.key_ids(perm);
        let mut weighted = 0.0;
        for (ids, &count) in self.tri_idx_slot.iter().zip(self.tri_counts.iter()) {
            let idx = tri_idx_u8(
                kid[ids[0] as usize],
                kid[ids[1] as usize],
                kid[ids[2] as usize],
            );
            weighted += self.tables.tri[idx] * count;
        }
        weighted
    }

    pub fn penalty(&self, perm: &[usize]) -> f64 {
        let mut usage = [[0.0f64; 4]; 2];
        for li in 0..N_LETTERS {
            let slot = perm[li];
            usage[self.hand[slot]][self.default_finger[slot]] += self.lfreq[li];
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
        let mut arr = vec![' '; N_ITEMS];
        for (li, &pos) in perm.iter().take(N_LETTERS).enumerate() {
            arr[pos] = LETTERS.as_bytes()[li] as char;
        }
        for (pi, &ch) in self.punctuation.iter().enumerate() {
            arr[perm[N_LETTERS + pi]] = ch;
        }
        arr.into_iter().collect()
    }
}

/// Item-to-slot permutation in layout order.  Items 0..26 are letters a-z and
/// items 26..30 are non-letter positions, assigned in order of appearance.
/// The actual non-letter characters do not affect scoring; `layout_str` emits
/// the configured punctuation instead.
pub fn perm_of(layout: &str, punctuation: &[char]) -> Result<Vec<usize>, String> {
    let chars: Vec<char> = layout.chars().collect();
    if chars.len() != N_ITEMS {
        return Err(format!(
            "layout must be {} characters, got {}",
            N_ITEMS,
            chars.len()
        ));
    }
    let mut perm = vec![usize::MAX; N_ITEMS];
    let mut seen_letters = [false; N_LETTERS];
    let mut non_letters = Vec::new();
    for (slot, ch) in chars.into_iter().enumerate() {
        if ch.is_ascii_lowercase() {
            let letter = (ch as u8 - b'a') as usize;
            if letter >= N_LETTERS || seen_letters[letter] {
                return Err(format!("layout repeats or has an invalid letter {ch:?}"));
            }
            seen_letters[letter] = true;
            perm[letter] = slot;
        } else {
            non_letters.push(slot);
        }
    }
    if seen_letters.iter().any(|seen| !seen) {
        return Err("layout must contain all 26 letters".to_string());
    }
    if non_letters.len() != punctuation.len() {
        return Err(format!(
            "layout must contain {} non-letter characters, got {}",
            punctuation.len(),
            non_letters.len()
        ));
    }
    for (pi, slot) in non_letters.into_iter().enumerate() {
        perm[N_LETTERS + pi] = slot;
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
