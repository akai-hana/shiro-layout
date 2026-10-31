//! Scorer tables over the 30-key ANSI layout plus the space "key".
//!
//! Port of `shirolyzer/utils/scorer.py`.

use anyhow::{bail, Result};
use rayon::prelude::*;

use crate::conditions::Conditions;
use crate::config::Config;
use crate::gates::Gates;
use crate::geometry::{Geometry, KEYS, NF, NK};
use crate::model::{Model, ModelTables, W_ALT};

pub const N: usize = 31;
pub const SPACE_ID: usize = 30;
pub const SPACE: &str = "@space";
pub const SPACE_EFFORT: f64 = 0.6;
pub const PRESS: f64 = 1.0;

#[derive(Clone, Debug)]
pub struct Tables {
    pub big: Vec<f64>,
    pub tri: Vec<f64>,
    pub big_f: Vec<u8>,
    pub tri_f: Vec<[u8; 3]>,
    pub model: ModelTables,
}

#[inline]
pub fn big_idx(a: usize, b: usize) -> usize {
    a * N + b
}

#[inline]
pub fn tri_idx(a: usize, b: usize, c: usize) -> usize {
    (a * N + b) * N + c
}

#[inline]
pub fn tri_idx_u8(a: u8, b: u8, c: u8) -> usize {
    ((a as usize * N + b as usize) * N) + c as usize
}

fn key_char(k: usize) -> char {
    KEYS[k].chars().next().unwrap()
}

impl Tables {
    pub fn build(
        model: &Model,
        geom: &Geometry,
        _cfg: &Config,
        gates: &Gates,
        conditions: &Conditions,
    ) -> Result<Self> {
        let mt = model.tables(geom);
        let e_with_press: Vec<f64> = mt.e.iter().map(|v| v + PRESS).collect();
        let chars: Vec<char> = (0..NK).map(key_char).collect();

        let mut big = vec![f64::INFINITY; N * N];
        let mut big_f = vec![0u8; N * N];
        big.par_chunks_mut(N)
            .zip(big_f.par_chunks_mut(N))
            .enumerate()
            .take(NK)
            .for_each(|(a, (big_row, big_f_row))| {
                let mut pm = vec![0.0; NF * NF];
                let ca = chars[a];
                for b in 0..NK {
                    gates.penalty_mask_into(&[ca, chars[b]], conditions, &mut pm);
                    let mut best = f64::INFINITY;
                    let mut best_f = 0usize;
                    for fa in 0..NF {
                        for fb in 0..NF {
                            let mut c = mt.t2[crate::model::pt_idx(a, fa, b, fb)]
                                + 2.0 * PRESS
                                + pm[fa * NF + fb];
                            if a == b && fa != fb {
                                c = f64::INFINITY;
                            }
                            if c < best {
                                best = c;
                                best_f = fa * 5 + fb;
                            }
                        }
                    }
                    big_row[b] = best;
                    big_f_row[b] = best_f as u8;
                }
            });

        let mut tri = vec![f64::INFINITY; N * N * N];
        let mut tri_f = vec![[0u8; 3]; N * N * N];
        tri.par_chunks_mut(N * N)
            .zip(tri_f.par_chunks_mut(N * N))
            .enumerate()
            .take(NK)
            .for_each(|(a, (tri_row, tri_f_row))| {
                let mut pm = vec![0.0; NF * NF * NF];
                let mut costs = vec![0.0; NF * NF * NF];
                let ca = chars[a];
                for b in 0..NK {
                    let cb = chars[b];
                    for c in 0..NK {
                        gates.penalty_mask_into(&[ca, cb, chars[c]], conditions, &mut pm);
                        model.tri_many_into(geom, &mt, &[a], &[b], &[c], &mut costs);
                        let mut best = f64::INFINITY;
                        let mut best_f = [0u8; 3];
                        for f1 in 0..NF {
                            for f2 in 0..NF {
                                for f3 in 0..NF {
                                    let idx = (f1 * NF + f2) * NF + f3;
                                    let value = costs[idx] + 3.0 * PRESS + pm[idx];
                                    if value < best {
                                        best = value;
                                        best_f = [f1 as u8, f2 as u8, f3 as u8];
                                    }
                                }
                            }
                        }
                        tri_row[b * N + c] = best;
                        tri_f_row[b * N + c] = best_f;
                    }
                }
            });

        for a in 0..NK {
            let (f, _) = argmin_finger(&e_with_press[a * NF..(a + 1) * NF]);
            let ea = mt.e[crate::model::e_idx(a, f)];
            let value = ea + PRESS + SPACE_EFFORT + W_ALT;
            big[big_idx(a, SPACE_ID)] = value;
            big_f[big_idx(a, SPACE_ID)] = (f * 5 + 4) as u8;
            big[big_idx(SPACE_ID, a)] = value;
            big_f[big_idx(SPACE_ID, a)] = (4 * 5 + f) as u8;
        }
        big[big_idx(SPACE_ID, SPACE_ID)] = 0.0;
        big_f[big_idx(SPACE_ID, SPACE_ID)] = 4 * 5 + 4;

        for i in 0..N {
            for j in 0..N {
                for k in 0..N {
                    if i != SPACE_ID && j != SPACE_ID && k != SPACE_ID {
                        continue;
                    }
                    let ids = [i, j, k];
                    let mut best = f64::INFINITY;
                    let mut best_fs = [0u8; 3];
                    let opts: Vec<Vec<usize>> = ids
                        .iter()
                        .map(|&x| {
                            if x == SPACE_ID {
                                vec![4]
                            } else {
                                (0..NF)
                                    .filter(|&f| mt.e[crate::model::e_idx(x, f)].is_finite())
                                    .collect()
                            }
                        })
                        .collect();
                    for &f0 in &opts[0] {
                        for &f1 in &opts[1] {
                            for &f2 in &opts[2] {
                                let fs = [f0, f1, f2];
                                if (i == j && fs[0] != fs[1])
                                    || (j == k && fs[1] != fs[2])
                                    || (i == k && fs[0] != fs[2])
                                {
                                    continue;
                                }
                                let mut cost = 0.0;
                                for (pos, &x) in ids.iter().enumerate() {
                                    if x == SPACE_ID {
                                        cost += SPACE_EFFORT;
                                    } else {
                                        cost += mt.e[crate::model::e_idx(x, fs[pos])] + PRESS;
                                    }
                                }
                                for (p, q) in [(0usize, 1usize), (1, 2)] {
                                    if ids[p] == SPACE_ID || ids[q] == SPACE_ID {
                                        cost += W_ALT;
                                    } else {
                                        cost += mt.pt
                                            [crate::model::pt_idx(ids[p], fs[p], ids[q], fs[q])];
                                    }
                                }
                                if cost < best {
                                    best = cost;
                                    best_fs = [fs[0] as u8, fs[1] as u8, fs[2] as u8];
                                }
                            }
                        }
                    }
                    tri[tri_idx(i, j, k)] = best;
                    tri_f[tri_idx(i, j, k)] = best_fs;
                }
            }
        }

        for a in 0..NK {
            for b in 0..NK {
                if !big[big_idx(a, b)].is_finite() {
                    bail!(
                        "alt_fingers: no feasible fingering for bigram {}->{}",
                        KEYS[a],
                        KEYS[b]
                    );
                }
            }
        }
        for a in 0..NK {
            for b in 0..NK {
                for c in 0..NK {
                    if !tri[tri_idx(a, b, c)].is_finite() {
                        bail!(
                            "alt_fingers: no feasible fingering for trigram {}->{}->{}",
                            KEYS[a],
                            KEYS[b],
                            KEYS[c]
                        );
                    }
                }
            }
        }

        Ok(Self {
            big,
            tri,
            big_f,
            tri_f,
            model: mt,
        })
    }

    pub fn bigram_overhead(&self, a: usize, b: usize) -> f64 {
        self.big[big_idx(a, b)]
    }
}

fn argmin_finger(values: &[f64]) -> (usize, f64) {
    let mut best = f64::INFINITY;
    let mut idx = 0;
    for (i, v) in values.iter().enumerate() {
        if *v < best {
            best = *v;
            idx = i;
        }
    }
    (idx, best)
}
