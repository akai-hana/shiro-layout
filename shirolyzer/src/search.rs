//! Local-search primitives (quench, anneal, perturb), ported from
//! `shirolyzer/utils/search.py`.
//!
//! `quench` and `anneal` for the corpus evaluator use incremental scoring:
//! swapping or cycling letters only changes the trigrams containing those
//! letters, so the weighted trigram sum is updated from a per-letter incidence
//! list instead of rescanning the whole corpus.

use std::sync::OnceLock;

use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rayon::prelude::*;

use crate::evaluate::{Evaluator, N_ITEMS};
use crate::scorer::tri_idx_u8;

pub trait ScoreFn {
    fn score(&self, perm: &[usize]) -> f64;
}

impl ScoreFn for Evaluator<'_> {
    fn score(&self, perm: &[usize]) -> f64 {
        Evaluator::score(self, perm)
    }
}

impl<F: Fn(&[usize]) -> f64> ScoreFn for F {
    fn score(&self, perm: &[usize]) -> f64 {
        self(perm)
    }
}

struct Moves {
    pairs: Vec<[usize; 2]>,
    two: Vec<[usize; 4]>,
    two_pairs: Vec<[usize; 2]>,
    three: Vec<[usize; 3]>,
}

fn moves() -> &'static Moves {
    static MOVES: OnceLock<Moves> = OnceLock::new();
    MOVES.get_or_init(|| {
        let mut pairs = Vec::with_capacity(N_ITEMS * (N_ITEMS - 1) / 2);
        for i in 0..N_ITEMS {
            for j in i + 1..N_ITEMS {
                pairs.push([i, j]);
            }
        }
        let mut two = Vec::new();
        let mut two_pairs = Vec::new();
        for (x, a) in pairs.iter().enumerate() {
            for (y, b) in pairs.iter().enumerate().skip(x + 1) {
                let mut all = [a[0], a[1], b[0], b[1]];
                all.sort_unstable();
                if all.windows(2).all(|w| w[0] != w[1]) {
                    two.push([a[0], a[1], b[0], b[1]]);
                    two_pairs.push([x, y]);
                }
            }
        }
        let mut three = Vec::with_capacity(4060);
        for i in 0..N_ITEMS {
            for j in i + 1..N_ITEMS {
                for k in j + 1..N_ITEMS {
                    three.push([i, j, k]);
                }
            }
        }
        Moves {
            pairs,
            two,
            two_pairs,
            three,
        }
    })
}

/// Precomputed trigram intersections for disjoint two-swap moves.
///
/// For a move that swaps `(i,j)` and `(k,l)`, a trigram changes because of
/// both swaps only when it contains a letter from each pair.  Keeping that
/// intersection lets a two-pair candidate be scored from the cached
/// single-swap deltas plus a few hundred cross terms.  The table is indexed by
/// the same move index as [`Moves::two_pairs`], so the hot path has no hashing.
pub struct DeltaTables {
    cross: Vec<Vec<u16>>,
}

fn intersection(
    inc: &[Vec<u32>],
    a: usize,
    b: usize,
    c: usize,
    d: usize,
    seen: &mut [u32],
    stamp: &mut u32,
) -> Vec<u16> {
    *stamp = stamp.wrapping_add(2);
    if *stamp < 2 {
        seen.fill(0);
        *stamp = 2;
    }
    let mark = *stamp;
    let pushed = *stamp + 1;
    for &letter in &[a, b] {
        for &t in &inc[letter] {
            seen[t as usize] = mark;
        }
    }
    let mut out = Vec::new();
    for &letter in &[c, d] {
        for &t in &inc[letter] {
            let t = t as usize;
            if seen[t] == mark {
                out.push(t as u16);
                seen[t] = pushed;
            }
        }
    }
    out
}

impl DeltaTables {
    pub fn build(ev: &Evaluator) -> Self {
        let m = moves();
        let n = ev.tri_idx_slot.len();
        let cross: Vec<Vec<u16>> = m
            .two_pairs
            .par_iter()
            .map_with((vec![0u32; n], 0u32), |(seen, stamp), &[p1, p2]| {
                let [a, b] = m.pairs[p1];
                let [c, d] = m.pairs[p2];
                intersection(&ev.inc, a, b, c, d, seen, stamp)
            })
            .collect();
        Self { cross }
    }

    #[inline]
    fn get(&self, move_index: usize) -> &[u16] {
        &self.cross[move_index]
    }

    #[cfg(test)]
    fn get_pairs(&self, i1: usize, j1: usize, i2: usize, j2: usize) -> &[u16] {
        let m = moves();
        let pair_index = |i: usize, j: usize| {
            m.pairs
                .iter()
                .position(|p| *p == [i, j])
                .expect("pair exists")
        };
        let p1 = pair_index(i1, j1);
        let p2 = pair_index(i2, j2);
        let key = if p1 < p2 { [p1, p2] } else { [p2, p1] };
        let mi = m
            .two_pairs
            .iter()
            .position(|p| *p == key)
            .expect("pair of pairs exists");
        self.get(mi)
    }
}

/// Incrementally scored permutation.
struct Local<'a> {
    ev: &'a Evaluator<'a>,
    perm: Vec<usize>,
    kid: [u8; N_ITEMS],
    weighted: f64,
    pen: f64,
    seen: Vec<u32>,
    stamp: u32,
}

impl<'a> Local<'a> {
    fn new(ev: &'a Evaluator<'a>, perm: &[usize]) -> Self {
        let mut kid = [0u8; N_ITEMS];
        for (li, &slot) in perm.iter().enumerate() {
            kid[li] = ev.slot_key[slot];
        }
        let mut weighted = 0.0;
        for (ids, &count) in ev.tri_idx_slot.iter().zip(ev.tri_counts.iter()) {
            let idx = tri_idx_u8(
                kid[ids[0] as usize],
                kid[ids[1] as usize],
                kid[ids[2] as usize],
            );
            weighted += ev.tables.tri[idx] * count;
        }
        Self {
            ev,
            perm: perm.to_vec(),
            kid,
            weighted,
            pen: ev.penalty(perm),
            seen: vec![0; ev.tri_idx_slot.len()],
            stamp: 0,
        }
    }

    #[inline]
    fn score(&self) -> f64 {
        self.weighted / self.ev.n_tri + self.pen
    }

    fn delta(&mut self, kid2: &[u8; N_ITEMS], changed: &[usize]) -> f64 {
        self.stamp = self.stamp.wrapping_add(1);
        if self.stamp == 0 {
            self.seen.fill(0);
            self.stamp = 1;
        }
        let stamp = self.stamp;
        let mut dw = 0.0;
        for &letter in changed {
            let list = &self.ev.inc[letter];
            for &t in list {
                let t = t as usize;
                if self.seen[t] == stamp {
                    continue;
                }
                self.seen[t] = stamp;
                let ids = self.ev.tri_idx_slot[t];
                let a = ids[0] as usize;
                let b = ids[1] as usize;
                let c = ids[2] as usize;
                let old = self.ev.tables.tri[tri_idx_u8(self.kid[a], self.kid[b], self.kid[c])];
                let new = self.ev.tables.tri[tri_idx_u8(kid2[a], kid2[b], kid2[c])];
                dw += (new - old) * self.ev.tri_counts[t];
            }
        }
        dw
    }

    fn delta_swap(&mut self, i: usize, j: usize) -> f64 {
        let mut kid2 = self.kid;
        kid2[i] = self.kid[j];
        kid2[j] = self.kid[i];
        self.delta(&kid2, &[i, j])
    }

    fn delta_cross(&mut self, i1: usize, j1: usize, i2: usize, j2: usize, list: &[u16]) -> f64 {
        if list.is_empty() {
            return 0.0;
        }
        let mut kid1 = self.kid;
        kid1[i1] = self.kid[j1];
        kid1[j1] = self.kid[i1];
        let mut kid2 = self.kid;
        kid2[i2] = self.kid[j2];
        kid2[j2] = self.kid[i2];
        let mut kid12 = kid1;
        kid12[i2] = kid1[j2];
        kid12[j2] = kid1[i2];
        let mut dw = 0.0;
        for &t in list {
            let t = t as usize;
            let ids = self.ev.tri_idx_slot[t];
            let a = ids[0] as usize;
            let b = ids[1] as usize;
            let c = ids[2] as usize;
            let old = self.ev.tables.tri[tri_idx_u8(self.kid[a], self.kid[b], self.kid[c])];
            let only1 = self.ev.tables.tri[tri_idx_u8(kid1[a], kid1[b], kid1[c])];
            let only2 = self.ev.tables.tri[tri_idx_u8(kid2[a], kid2[b], kid2[c])];
            let both = self.ev.tables.tri[tri_idx_u8(kid12[a], kid12[b], kid12[c])];
            dw += (both - only1 - only2 + old) * self.ev.tri_counts[t];
        }
        dw
    }

    fn swap(&mut self, i: usize, j: usize) -> f64 {
        let mut kid2 = self.kid;
        kid2[i] = self.kid[j];
        kid2[j] = self.kid[i];
        let dw = self.delta(&kid2, &[i, j]);
        self.kid = kid2;
        self.perm.swap(i, j);
        self.weighted += dw;
        self.score()
    }

    fn undo_swap(&mut self, i: usize, j: usize, w: f64, kid: [u8; N_ITEMS]) {
        self.perm.swap(i, j);
        self.weighted = w;
        self.kid = kid;
    }

    fn cycle3(&mut self, i: usize, j: usize, k: usize) -> f64 {
        let mut kid2 = self.kid;
        kid2[i] = self.kid[j];
        kid2[j] = self.kid[k];
        kid2[k] = self.kid[i];
        let dw = self.delta(&kid2, &[i, j, k]);
        let state = [self.perm[i], self.perm[j], self.perm[k]];
        self.perm[i] = state[1];
        self.perm[j] = state[2];
        self.perm[k] = state[0];
        self.kid = kid2;
        self.weighted += dw;
        self.score()
    }

    fn undo_cycle3(
        &mut self,
        i: usize,
        j: usize,
        k: usize,
        w: f64,
        kid: [u8; N_ITEMS],
        p: [usize; 3],
    ) {
        self.perm[i] = p[0];
        self.perm[j] = p[1];
        self.perm[k] = p[2];
        self.weighted = w;
        self.kid = kid;
    }
}

pub fn anneal(
    ev: &Evaluator,
    seed_perm: Option<&[usize]>,
    iters: usize,
    t0: f64,
    t1: f64,
    seed: u64,
) -> (Vec<usize>, f64) {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let perm = match seed_perm {
        Some(p) => p.to_vec(),
        None => ev.constrained_shuffle(&mut rng).unwrap_or_else(|_| {
            let mut p: Vec<usize> = (0..N_ITEMS).collect();
            p.shuffle(&mut rng);
            p
        }),
    };
    let mut st = Local::new(ev, &perm);
    let mut cur = st.score();
    let mut best = st.perm.clone();
    let mut best_s = cur;
    let iters = iters.max(1);
    for it in 0..iters {
        let t = t0 * (t1 / t0).powf(it as f64 / iters as f64);
        let mut pair = None;
        for _ in 0..100 {
            let i = rng.gen_range(0..N_ITEMS);
            let mut j = rng.gen_range(0..N_ITEMS - 1);
            if j >= i {
                j += 1;
            }
            if ev.swap_allowed(&st.perm, i, j) {
                pair = Some((i, j));
                break;
            }
        }
        let Some((i, j)) = pair else {
            continue;
        };
        let w0 = st.weighted;
        let kid0 = st.kid;
        let s = st.swap(i, j);
        if s < cur || rng.gen::<f64>() < ((cur - s) / t).exp() {
            cur = s;
            if s < best_s {
                best_s = s;
                best = st.perm.clone();
            }
        } else {
            st.undo_swap(i, j, w0, kid0);
        }
    }
    (best, best_s)
}

pub fn quench(
    ev: &Evaluator,
    tables: &DeltaTables,
    perm: &[usize],
    cur_s: f64,
    rng: &mut impl Rng,
) -> (Vec<usize>, f64) {
    let m = moves();
    let mut st = Local::new(ev, perm);
    let mut cur = cur_s;
    {
        let mut order: Vec<u32> = (0..m.pairs.len() as u32).collect();
        let mut changed = true;
        while changed {
            changed = false;
            order.shuffle(rng);
            for &mi in &order {
                let [i, j] = m.pairs[mi as usize];
                if !ev.swap_allowed(&st.perm, i, j) {
                    continue;
                }
                let w0 = st.weighted;
                let kid0 = st.kid;
                let s = st.swap(i, j);
                if s < cur - 1e-12 {
                    cur = s;
                    changed = true;
                } else {
                    st.undo_swap(i, j, w0, kid0);
                }
            }
        }
    }
    {
        let mut order: Vec<u32> = (0..m.two.len() as u32).collect();
        let mut cache: Vec<Option<f64>> = vec![None; m.pairs.len()];
        let mut changed = true;
        while changed {
            changed = false;
            order.shuffle(rng);
            for &mi in &order {
                let mi = mi as usize;
                let [p1, p2] = m.two_pairs[mi];
                let [i1, j1] = m.pairs[p1];
                let [i2, j2] = m.pairs[p2];
                if !ev.swap_allowed(&st.perm, i1, j1) || !ev.swap_allowed(&st.perm, i2, j2) {
                    continue;
                }
                let d1 = match cache[p1] {
                    Some(v) => v,
                    None => {
                        let v = st.delta_swap(i1, j1);
                        cache[p1] = Some(v);
                        v
                    }
                };
                let d2 = match cache[p2] {
                    Some(v) => v,
                    None => {
                        let v = st.delta_swap(i2, j2);
                        cache[p2] = Some(v);
                        v
                    }
                };
                let dc = st.delta_cross(i1, j1, i2, j2, tables.get(mi));
                let s = cur + (d1 + d2 + dc) / ev.n_tri;
                if s < cur - 1e-12 {
                    st.swap(i1, j1);
                    st.swap(i2, j2);
                    cur = s;
                    changed = true;
                    cache.fill(None);
                }
            }
        }
    }
    {
        let mut order: Vec<u32> = (0..m.three.len() as u32).collect();
        let mut changed = true;
        while changed {
            changed = false;
            order.shuffle(rng);
            for &mi in &order {
                let [i, j, k] = m.three[mi as usize];
                if !ev.cycle3_allowed(&st.perm, i, j, k) {
                    continue;
                }
                let w0 = st.weighted;
                let kid0 = st.kid;
                let p0 = [st.perm[i], st.perm[j], st.perm[k]];
                let s = st.cycle3(i, j, k);
                if s < cur - 1e-12 {
                    cur = s;
                    changed = true;
                } else {
                    st.undo_cycle3(i, j, k, w0, kid0, p0);
                }
            }
        }
    }
    (st.perm, cur)
}

pub fn delta_swap_cost(ev: &Evaluator, perm: &[usize], reps: usize) -> (f64, usize) {
    let mut st = Local::new(ev, perm);
    let mut sum = 0.0;
    for _ in 0..reps {
        for i in 0..N_ITEMS {
            for j in i + 1..N_ITEMS {
                sum += st.delta_swap(i, j);
            }
        }
    }
    (sum, reps * N_ITEMS * (N_ITEMS - 1) / 2)
}

fn improve<F, U>(
    ev: &impl ScoreFn,
    perm: &mut Vec<usize>,
    cur_s: &mut f64,
    rng: &mut impl Rng,
    moves: &mut [Vec<usize>],
    mut apply: F,
    mut undo: U,
) where
    F: FnMut(&mut Vec<usize>, &[usize]) -> Option<Vec<usize>>,
    U: FnMut(&mut Vec<usize>, &[usize], Option<Vec<usize>>),
{
    let mut changed = true;
    while changed {
        changed = false;
        moves.shuffle(rng);
        for mv in moves.iter() {
            let state = apply(perm, mv);
            let s = ev.score(perm);
            if s < *cur_s - 1e-12 {
                *cur_s = s;
                changed = true;
            } else {
                undo(perm, mv, state);
            }
        }
    }
}

/// Generic full-score quench, used when the objective is a closure (for
/// example the constrained secondary objective).
pub fn quench_generic(
    ev: &impl ScoreFn,
    perm: &[usize],
    cur_s: f64,
    rng: &mut impl Rng,
) -> (Vec<usize>, f64) {
    let mut perm = perm.to_vec();
    let mut cur_s = cur_s;

    let mut pairs: Vec<Vec<usize>> = Vec::new();
    for i in 0..N_ITEMS {
        for j in i + 1..N_ITEMS {
            pairs.push(vec![i, j]);
        }
    }

    let mut two: Vec<Vec<usize>> = Vec::new();
    for x in 0..pairs.len() {
        let (i1, j1) = (pairs[x][0], pairs[x][1]);
        for y in x + 1..pairs.len() {
            let (i2, j2) = (pairs[y][0], pairs[y][1]);
            if i1 != i2 && i1 != j2 && j1 != i2 && j1 != j2 {
                two.push(vec![i1, j1, i2, j2]);
            }
        }
    }

    let mut three: Vec<Vec<usize>> = Vec::new();
    for i in 0..N_ITEMS {
        for j in i + 1..N_ITEMS {
            for k in j + 1..N_ITEMS {
                three.push(vec![i, j, k]);
            }
        }
    }

    improve(
        ev,
        &mut perm,
        &mut cur_s,
        rng,
        &mut pairs,
        |p, m| {
            p.swap(m[0], m[1]);
            None
        },
        |p, m, _| p.swap(m[0], m[1]),
    );

    improve(
        ev,
        &mut perm,
        &mut cur_s,
        rng,
        &mut two,
        |p, m| {
            p.swap(m[0], m[1]);
            p.swap(m[2], m[3]);
            None
        },
        |p, m, _| {
            p.swap(m[0], m[1]);
            p.swap(m[2], m[3]);
        },
    );

    improve(
        ev,
        &mut perm,
        &mut cur_s,
        rng,
        &mut three,
        |p, m| {
            let state = vec![p[m[0]], p[m[1]], p[m[2]]];
            p[m[0]] = state[1];
            p[m[1]] = state[2];
            p[m[2]] = state[0];
            Some(state)
        },
        |p, m, state| {
            let state = state.unwrap();
            p[m[0]] = state[0];
            p[m[1]] = state[1];
            p[m[2]] = state[2];
        },
    );
    (perm, cur_s)
}

/// Incrementally scored constrained quench.
///
/// This mirrors [`quench_generic`] move for move, but keeps two [`Local`]
/// states: the primary objective supplies the sacrifice constraint and the
/// secondary objective supplies the score being minimized.  The move order and
/// RNG consumption are identical to the generic path, so the constrained
/// optimum matches full rescoring without touching the full-score fallback.
pub fn quench_constrained(
    primary: &Evaluator,
    secondary: &Evaluator,
    perm: &[usize],
    primary_limit: f64,
    cur_s: f64,
    rng: &mut impl Rng,
) -> (Vec<usize>, f64) {
    let m = moves();
    let mut p = Local::new(primary, perm);
    let mut s = Local::new(secondary, perm);
    let mut cur = cur_s;

    let mut order: Vec<u32> = (0..m.pairs.len() as u32).collect();
    let mut changed = true;
    while changed {
        changed = false;
        order.shuffle(rng);
        for &mi in &order {
            let [i, j] = m.pairs[mi as usize];
            if !primary.swap_allowed(&p.perm, i, j) {
                continue;
            }
            let pw0 = p.weighted;
            let pk0 = p.kid;
            let sw0 = s.weighted;
            let sk0 = s.kid;
            let ps = p.swap(i, j);
            let ss = s.swap(i, j);
            if ps > primary_limit || ss >= cur - 1e-12 {
                p.undo_swap(i, j, pw0, pk0);
                s.undo_swap(i, j, sw0, sk0);
            } else {
                cur = ss;
                changed = true;
            }
        }
    }

    let mut order: Vec<u32> = (0..m.two.len() as u32).collect();
    let mut changed = true;
    while changed {
        changed = false;
        order.shuffle(rng);
        for &mi in &order {
            let mi = mi as usize;
            let [p1, p2] = m.two_pairs[mi];
            let [i1, j1] = m.pairs[p1];
            let [i2, j2] = m.pairs[p2];
            if !primary.swap_allowed(&p.perm, i1, j1) || !primary.swap_allowed(&p.perm, i2, j2) {
                continue;
            }
            let pw0 = p.weighted;
            let pk0 = p.kid;
            let sw0 = s.weighted;
            let sk0 = s.kid;
            p.swap(i1, j1);
            p.swap(i2, j2);
            s.swap(i1, j1);
            s.swap(i2, j2);
            let ps = p.score();
            let ss = s.score();
            if ps > primary_limit || ss >= cur - 1e-12 {
                p.undo_swap(i1, j1, pw0, pk0);
                p.undo_swap(i2, j2, pw0, pk0);
                s.undo_swap(i1, j1, sw0, sk0);
                s.undo_swap(i2, j2, sw0, sk0);
            } else {
                cur = ss;
                changed = true;
            }
        }
    }

    let mut order: Vec<u32> = (0..m.three.len() as u32).collect();
    let mut changed = true;
    while changed {
        changed = false;
        order.shuffle(rng);
        for &mi in &order {
            let [i, j, k] = m.three[mi as usize];
            if !primary.cycle3_allowed(&p.perm, i, j, k) {
                continue;
            }
            let pw0 = p.weighted;
            let pk0 = p.kid;
            let pp0 = [p.perm[i], p.perm[j], p.perm[k]];
            let sw0 = s.weighted;
            let sk0 = s.kid;
            let sp0 = [s.perm[i], s.perm[j], s.perm[k]];
            let ps = p.cycle3(i, j, k);
            let ss = s.cycle3(i, j, k);
            if ps > primary_limit || ss >= cur - 1e-12 {
                p.undo_cycle3(i, j, k, pw0, pk0, pp0);
                s.undo_cycle3(i, j, k, sw0, sk0, sp0);
            } else {
                cur = ss;
                changed = true;
            }
        }
    }
    (s.perm, cur)
}

pub fn perturb(perm: &[usize], rng: &mut impl Rng, k: usize) -> Vec<usize> {
    let mut cand = perm.to_vec();
    let mut idx: Vec<usize> = (0..N_ITEMS).collect();
    idx.shuffle(rng);
    let moved = &idx[..(2 * k).min(idx.len())];
    let mut a = 0;
    while a + 1 < moved.len() {
        let i = moved[a];
        let j = moved[a + 1];
        cand.swap(i, j);
        a += 2;
    }
    cand
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluate::flat_share_from_config;

    pub(super) fn setup() -> (crate::config::Config, crate::evaluate::Evaluator<'static>) {
        let cfg = crate::config::load(None).expect("config");
        let geom = crate::geometry::Geometry::new();
        let model = crate::config::build_model(&cfg).expect("model");
        let gates = crate::gates::Gates::from_config(&cfg);
        let conds = crate::conditions::Conditions::from_config(&cfg, &geom);
        let tables =
            crate::scorer::Tables::build(&model, &geom, &cfg, &gates, &conds).expect("tables");
        let paths: Vec<std::path::PathBuf> = cfg
            .primary_corpus
            .iter()
            .map(|e| cfg.corpus_path(e))
            .collect();
        let corpus = crate::corpus::load_many(&paths).expect("corpus");
        let flat = flat_share_from_config(&cfg);
        let (w_balance, w_hand) = cfg.balance_weights();
        let ev = crate::evaluate::Evaluator::new(
            Box::leak(Box::new(corpus)),
            Box::leak(Box::new(tables)),
            Box::leak(Box::new(model)),
            &cfg.punctuation,
            flat,
            w_balance,
            w_hand,
        )
        .expect("evaluator");
        (cfg, ev)
    }

    const TEST_LAYOUT: &str = ";ntgfjoehkrpmscuaydilbqvwxz,./";

    #[test]
    fn hand_restrictions_are_preserved_by_search() {
        let (_cfg, mut ev) = setup();
        let mut required = [None; 26];
        required[(b'z' - b'a') as usize] = Some(0);
        required[(b'x' - b'a') as usize] = Some(1);
        ev.set_hand_restrictions(required);

        let mut rng = rand::rngs::StdRng::seed_from_u64(3);
        let perm = ev.constrained_shuffle(&mut rng).expect("shuffle");
        assert!(ev.perm_allowed(&perm));

        for i in 0..N_ITEMS {
            for j in i + 1..N_ITEMS {
                let mut swapped = perm.clone();
                swapped.swap(i, j);
                if !ev.swap_allowed(&perm, i, j) {
                    assert!(!ev.perm_allowed(&swapped));
                }
            }
        }

        let moved = ev.perturb(&perm, &mut rng, 8);
        assert!(ev.perm_allowed(&moved));

        let (annealed, _) = anneal(&ev, Some(&perm), 500, 0.35, 0.002, 9);
        assert!(ev.perm_allowed(&annealed));

        let tables = DeltaTables::build(&ev);
        let (quenched, _) = quench(&ev, &tables, &perm, ev.score(&perm), &mut rng);
        assert!(ev.perm_allowed(&quenched));
    }

    #[test]
    fn hand_restrictions_reject_impossible_shuffles() {
        let (_cfg, mut ev) = setup();
        ev.set_hand_restrictions([Some(0); 26]);
        let mut rng = rand::rngs::StdRng::seed_from_u64(4);
        assert!(ev.constrained_shuffle(&mut rng).is_err());
    }

    #[test]
    fn constrained_delta_matches_full_score() {
        let (cfg, primary) = setup();
        let paths: Vec<std::path::PathBuf> = cfg
            .secondary_corpus
            .iter()
            .map(|e| cfg.corpus_path(e))
            .collect();
        let corpus = crate::corpus::load_many(&paths).expect("secondary corpus");
        let flat = flat_share_from_config(&cfg);
        let (w_balance, w_hand) = cfg.balance_weights();
        let secondary = crate::evaluate::Evaluator::new(
            Box::leak(Box::new(corpus)),
            primary.tables,
            primary.model,
            &cfg.punctuation,
            flat,
            w_balance,
            w_hand,
        )
        .expect("secondary evaluator");
        let perm = crate::evaluate::perm_of(TEST_LAYOUT, &cfg.punctuation).expect("perm");
        let limit = primary.score(&perm) + 0.001;
        let constrained = |p: &[usize]| -> f64 {
            if primary.score(p) > limit {
                f64::INFINITY
            } else {
                secondary.score(p)
            }
        };
        let mut generic_rng = rand::rngs::StdRng::seed_from_u64(11);
        let (generic_perm, generic_score) =
            quench_generic(&constrained, &perm, constrained(&perm), &mut generic_rng);
        let mut delta_rng = rand::rngs::StdRng::seed_from_u64(11);
        let (delta_perm, delta_score) = quench_constrained(
            &primary,
            &secondary,
            &perm,
            limit,
            constrained(&perm),
            &mut delta_rng,
        );
        assert_eq!(delta_perm, generic_perm);
        assert!((delta_score - generic_score).abs() < 1e-12);
    }

    #[test]
    fn incremental_scores_match_full_score() {
        let (cfg, ev) = setup();
        let perm = crate::evaluate::perm_of(TEST_LAYOUT, &cfg.punctuation).expect("perm");
        let tables = DeltaTables::build(&ev);
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);

        let mut st = Local::new(&ev, &perm);
        assert!((st.score() - ev.score(&perm)).abs() < 1e-12);
        for _ in 0..40 {
            let i = rng.gen_range(0..N_ITEMS);
            let mut j = rng.gen_range(0..N_ITEMS - 1);
            if j >= i {
                j += 1;
            }
            let mut full = st.perm.clone();
            full.swap(i, j);
            let before = st.score();
            let w = st.weighted;
            let kid = st.kid;
            let s = st.swap(i, j);
            assert!((s - ev.score(&full)).abs() < 1e-12);
            st.undo_swap(i, j, w, kid);
            assert!((st.score() - before).abs() < 1e-12);
            assert_eq!(st.perm, perm);
        }

        let mut st = Local::new(&ev, &perm);
        for _ in 0..40 {
            let mut idx = [0usize; 4];
            for x in idx.iter_mut() {
                *x = rng.gen_range(0..N_ITEMS);
            }
            idx.sort_unstable();
            if idx.windows(2).any(|w| w[0] == w[1]) {
                continue;
            }
            let [i1, j1, i2, j2] = idx;
            let d1 = st.delta_swap(i1, j1);
            let d2 = st.delta_swap(i2, j2);
            let dc = st.delta_cross(i1, j1, i2, j2, tables.get_pairs(i1, j1, i2, j2));
            let predicted = st.score() + (d1 + d2 + dc) / ev.n_tri;
            let mut full = st.perm.clone();
            full.swap(i1, j1);
            full.swap(i2, j2);
            assert!((predicted - ev.score(&full)).abs() < 1e-12);
        }

        let mut st = Local::new(&ev, &perm);
        for _ in 0..40 {
            let i = rng.gen_range(0..N_ITEMS);
            let mut j = rng.gen_range(0..N_ITEMS - 1);
            if j >= i {
                j += 1;
            }
            let mut k = rng.gen_range(0..N_ITEMS - 2);
            if k >= i {
                k += 1;
            }
            if k >= j {
                k += 1;
            }
            let state = [st.perm[i], st.perm[j], st.perm[k]];
            let mut full = st.perm.clone();
            full[i] = state[1];
            full[j] = state[2];
            full[k] = state[0];
            let before = st.score();
            let w = st.weighted;
            let kid = st.kid;
            let old = [st.perm[i], st.perm[j], st.perm[k]];
            let s = st.cycle3(i, j, k);
            assert!((s - ev.score(&full)).abs() < 1e-12);
            st.undo_cycle3(i, j, k, w, kid, old);
            assert!((st.score() - before).abs() < 1e-12);
            assert_eq!(st.perm, perm);
        }
    }

    #[test]
    fn delta_quench_matches_full_quench() {
        let (cfg, ev) = setup();
        let perm = crate::evaluate::perm_of(TEST_LAYOUT, &cfg.punctuation).expect("perm");
        let tables = DeltaTables::build(&ev);
        let mut delta_rng = rand::rngs::StdRng::seed_from_u64(99);
        let (delta_perm, delta_score) =
            quench(&ev, &tables, &perm, ev.score(&perm), &mut delta_rng);
        let mut full_rng = rand::rngs::StdRng::seed_from_u64(99);
        let (full_perm, full_score) = quench_generic(&ev, &perm, ev.score(&perm), &mut full_rng);
        assert_eq!(delta_perm, full_perm);
        assert!((delta_score - full_score).abs() < 1e-12);
    }
}

#[cfg(test)]
mod delta_tables_tests {
    #[test]
    fn build_is_thread_invariant() {
        let (cfg, ev) = super::tests::setup();
        let _ = cfg;
        let one = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap()
            .install(|| super::DeltaTables::build(&ev));
        let many = rayon::ThreadPoolBuilder::new()
            .num_threads(8)
            .build()
            .unwrap()
            .install(|| super::DeltaTables::build(&ev));
        assert_eq!(one.cross.len(), many.cross.len());
        for (list, other) in one.cross.iter().zip(many.cross.iter()) {
            assert_eq!(list, other);
        }
    }
}
