//! Scorer tables over the 30-key ANSI layout plus the space "key".
//!
//! Port of `shirolyzer/utils/scorer.py`.

use anyhow::{bail, Result};
use rayon::prelude::*;

use crate::conditions::Conditions;
use crate::config::{AltMode, Config, FingeringMode};
use crate::gates::Gates;
use crate::geometry::{finger_hand, key_index, Geometry, KEYS, NF, NK};
use crate::model::{Model, ModelTables, W_ALT};

pub const N: usize = 31;
pub const SPACE_ID: usize = 30;
pub const SPACE: &str = "@space";
pub const SPACE_EFFORT: f64 = 0.6;
pub const PRESS: f64 = 1.0;
/// Pseudo finger slot used for the space key in the compact scorer encoding.
pub const SPACE_FINGER: usize = NF;
pub const FINGER_SLOTS: usize = NF + 1;

#[derive(Clone, Debug)]
pub struct Tables {
    pub big: Vec<f64>,
    pub tri: Vec<f64>,
    pub big_f: Vec<u8>,
    pub tri_f: Vec<[u8; 3]>,
    pub model: ModelTables,
    pub unrestricted: Option<ModelTables>,
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
        cfg: &Config,
        gates: &Gates,
        conditions: &Conditions,
    ) -> Result<Self> {
        let mt = model.tables(geom);
        let e_with_press: Vec<f64> = mt.e.iter().map(|v| v + PRESS).collect();
        let chars: Vec<char> = (0..NK).map(key_char).collect();
        let fixed = cfg.fingering == FingeringMode::Fixed;
        let strength_cost = |f: usize| {
            if f < NF {
                model.finger_strength[f]
            } else {
                0.0
            }
        };

        // Explicit allow rules may name fingers that geometry marks as
        // unreachable.  In that case we need a second table with reachability
        // removed; the normal travel/effort weights still decide the cost.
        let mt_unrestricted = if fixed {
            None
        } else if gates.rules.iter().any(|rule| {
            matches!(rule.mode, AltMode::Allow | AltMode::Permit)
                && rule.keys.iter().zip(&rule.fingers).any(|(&key, &finger)| {
                    let k = key_index(&key.to_string()).unwrap_or(0);
                    !geom.is_reachable(k, finger)
                })
        }) {
            Some(model.tables_unrestricted(geom))
        } else {
            None
        };

        let mut big = vec![f64::INFINITY; N * N];
        let mut big_f = vec![0u8; N * N];
        big.par_chunks_mut(N)
            .zip(big_f.par_chunks_mut(N))
            .enumerate()
            .take(NK)
            .for_each(|(a, (big_row, big_f_row))| {
                let mut pm = vec![0.0; NF * NF];
                let ca = chars[a];
                let fa_range = if fixed {
                    cfg.fingermap[a]..cfg.fingermap[a] + 1
                } else {
                    0..NF
                };
                for b in 0..NK {
                    if fixed {
                        conditions.penalty_mask_into(&[ca, chars[b]], &mut pm);
                    } else {
                        gates.penalty_mask_into(&[ca, chars[b]], conditions, &mut pm);
                    }
                    let explicit = if mt_unrestricted.is_some() && !fixed {
                        Some(gates.explicit_fingers(&[ca, chars[b]]))
                    } else {
                        None
                    };
                    let fb_range = if fixed {
                        cfg.fingermap[b]..cfg.fingermap[b] + 1
                    } else {
                        0..NF
                    };
                    let mut best = f64::INFINITY;
                    let mut best_f = 0usize;
                    for fa in fa_range.clone() {
                        for fb in fb_range.clone() {
                            let penalty = pm[fa * NF + fb];
                            let mut c = mt.t2[crate::model::pt_idx(a, fa, b, fb)]
                                + 2.0 * PRESS
                                + penalty
                                + strength_cost(fa)
                                + strength_cost(fb);
                            if let (Some(mtu), Some(explicit)) = (&mt_unrestricted, &explicit) {
                                if c.is_infinite() && penalty.is_finite() {
                                    let unreachable =
                                        !geom.is_reachable(a, fa) || !geom.is_reachable(b, fb);
                                    let allowed = (geom.is_reachable(a, fa) || explicit[0][fa])
                                        && (geom.is_reachable(b, fb) || explicit[1][fb]);
                                    if unreachable && allowed {
                                        c = mtu.t2[crate::model::pt_idx(a, fa, b, fb)]
                                            + 2.0 * PRESS
                                            + penalty
                                            + strength_cost(fa)
                                            + strength_cost(fb);
                                    }
                                }
                            }
                            if a == b && fa != fb {
                                c = f64::INFINITY;
                            }
                            if c < best {
                                best = c;
                                best_f = fa * FINGER_SLOTS + fb;
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
                let f1_range = if fixed {
                    cfg.fingermap[a]..cfg.fingermap[a] + 1
                } else {
                    0..NF
                };
                for b in 0..NK {
                    let cb = chars[b];
                    let f2_range = if fixed {
                        cfg.fingermap[b]..cfg.fingermap[b] + 1
                    } else {
                        0..NF
                    };
                    for c in 0..NK {
                        if fixed {
                            conditions.penalty_mask_into(&[ca, cb, chars[c]], &mut pm);
                        } else {
                            gates.penalty_mask_into(&[ca, cb, chars[c]], conditions, &mut pm);
                        }
                        let explicit = if mt_unrestricted.is_some() && !fixed {
                            Some(gates.explicit_fingers(&[ca, cb, chars[c]]))
                        } else {
                            None
                        };
                        let needs_unrestricted = explicit.as_ref().is_some_and(|ex| {
                            ex.iter().enumerate().any(|(pos, fingers)| {
                                let key = [a, b, c][pos];
                                fingers
                                    .iter()
                                    .enumerate()
                                    .any(|(f, &allowed)| allowed && !geom.is_reachable(key, f))
                            })
                        });
                        let mut costs_unrestricted = if needs_unrestricted {
                            Some(vec![0.0; NF * NF * NF])
                        } else {
                            None
                        };
                        if let (Some(mtu), Some(cu)) = (&mt_unrestricted, &mut costs_unrestricted) {
                            model.tri_many_into(geom, mtu, &[a], &[b], &[c], cu);
                        }
                        let f3_range = if fixed {
                            cfg.fingermap[c]..cfg.fingermap[c] + 1
                        } else {
                            0..NF
                        };
                        model.tri_many_into(geom, &mt, &[a], &[b], &[c], &mut costs);
                        let mut best = f64::INFINITY;
                        let mut best_f = [0u8; 3];
                        for f1 in f1_range.clone() {
                            for f2 in f2_range.clone() {
                                for f3 in f3_range.clone() {
                                    let idx = (f1 * NF + f2) * NF + f3;
                                    let penalty = pm[idx];
                                    let mut value = costs[idx]
                                        + 3.0 * PRESS
                                        + penalty
                                        + strength_cost(f1)
                                        + strength_cost(f2)
                                        + strength_cost(f3);
                                    if let (Some(cu), Some(explicit)) =
                                        (&costs_unrestricted, &explicit)
                                    {
                                        if value.is_infinite() && penalty.is_finite() {
                                            let fs = [f1, f2, f3];
                                            let keys = [a, b, c];
                                            let unreachable = fs
                                                .iter()
                                                .enumerate()
                                                .any(|(pos, &f)| !geom.is_reachable(keys[pos], f));
                                            let allowed = fs.iter().enumerate().all(|(pos, &f)| {
                                                geom.is_reachable(keys[pos], f) || explicit[pos][f]
                                            });
                                            if unreachable && allowed {
                                                value = cu[idx]
                                                    + 3.0 * PRESS
                                                    + penalty
                                                    + strength_cost(f1)
                                                    + strength_cost(f2)
                                                    + strength_cost(f3);
                                            }
                                        }
                                    }
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
            let (f, _) = if fixed {
                (cfg.fingermap[a], f64::INFINITY)
            } else {
                argmin_same_hand_finger(geom, a, &e_with_press[a * NF..(a + 1) * NF])
            };
            let ea = mt.e[crate::model::e_idx(a, f)];
            let value = ea + PRESS + SPACE_EFFORT + W_ALT + strength_cost(f);
            big[big_idx(a, SPACE_ID)] = value;
            big_f[big_idx(a, SPACE_ID)] = (f * FINGER_SLOTS + SPACE_FINGER) as u8;
            big[big_idx(SPACE_ID, a)] = value;
            big_f[big_idx(SPACE_ID, a)] = (SPACE_FINGER * FINGER_SLOTS + f) as u8;
        }
        big[big_idx(SPACE_ID, SPACE_ID)] = 0.0;
        big_f[big_idx(SPACE_ID, SPACE_ID)] = (SPACE_FINGER * FINGER_SLOTS + SPACE_FINGER) as u8;

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
                                vec![SPACE_FINGER]
                            } else if fixed {
                                vec![cfg.fingermap[x]]
                            } else {
                                (0..NF)
                                    .filter(|&f| {
                                        finger_hand(f) == geom.hand[x]
                                            && mt.e[crate::model::e_idx(x, f)].is_finite()
                                    })
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
                                        cost += mt.e[crate::model::e_idx(x, fs[pos])]
                                            + PRESS
                                            + strength_cost(fs[pos]);
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
                        "no feasible fingering for bigram {}->{} (mode: {})",
                        KEYS[a],
                        KEYS[b],
                        cfg.fingering.label()
                    );
                }
            }
        }
        for a in 0..NK {
            for b in 0..NK {
                for c in 0..NK {
                    if !tri[tri_idx(a, b, c)].is_finite() {
                        bail!(
                            "no feasible fingering for trigram {}->{}->{} (mode: {})",
                            KEYS[a],
                            KEYS[b],
                            KEYS[c],
                            cfg.fingering.label()
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
            unrestricted: mt_unrestricted,
        })
    }

    pub fn bigram_overhead(&self, a: usize, b: usize) -> f64 {
        self.big[big_idx(a, b)]
    }
}

fn argmin_same_hand_finger(geom: &Geometry, key: usize, values: &[f64]) -> (usize, f64) {
    let mut best = f64::INFINITY;
    let mut idx = geom.hand[key] * crate::geometry::NFINGER;
    for (i, v) in values.iter().enumerate() {
        if finger_hand(i) == geom.hand[key] && *v < best {
            best = *v;
            idx = i;
        }
    }
    (idx, best)
}
