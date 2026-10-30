//! Alt-finger context gates as data, plus the penalty mask used by the scorer.
//!
//! Port of `shirolyzer/utils/gates.py`.  Only explicitly-gated entries are
//! injected into the scoring tables; the remaining gates are model targets and
//! are exercised by the audit.

use std::collections::{HashMap, HashSet};

use crate::config::{Config, GateMode};
use crate::geometry::{finger_index, FINGERS, NF};

pub const XPEN: f64 = 12.0;

#[derive(Clone, Debug)]
pub struct Gates {
    pub all: HashMap<(char, char), (GateMode, Option<HashSet<char>>)>,
    pub explicit: HashSet<(char, char)>,
    pub sequences: Vec<(Vec<char>, Vec<usize>)>,
}

impl Gates {
    pub fn from_config(cfg: &Config) -> Self {
        let mut all = HashMap::new();
        let mut explicit = HashSet::new();
        for gate in cfg.gate_entries() {
            all.insert((gate.key, gate.finger), (gate.mode, gate.context.clone()));
            if gate.explicit {
                explicit.insert((gate.key, gate.finger));
            }
        }
        let sequences = cfg
            .sequence_entries()
            .into_iter()
            .map(|s| {
                (
                    s.sequence.clone(),
                    s.fingers
                        .iter()
                        .map(|f| finger_index(*f).unwrap())
                        .collect(),
                )
            })
            .collect();
        Self {
            all,
            explicit,
            sequences,
        }
    }

    pub fn gate(&self, key: char, finger: char) -> Option<&(GateMode, Option<HashSet<char>>)> {
        self.all.get(&(key, finger))
    }

    pub fn explicit_subset(&self) -> HashMap<(char, char), (GateMode, Option<HashSet<char>>)> {
        self.all
            .iter()
            .filter(|(k, _)| self.explicit.contains(*k))
            .map(|(k, v)| (*k, v.clone()))
            .collect()
    }

    pub fn forbidden_pairs(
        &self,
        keys: &[char],
        gates: &HashMap<(char, char), (GateMode, Option<HashSet<char>>)>,
    ) -> HashSet<(usize, usize)> {
        let mut out = HashSet::new();
        for (pos, key) in keys.iter().enumerate() {
            let contains_neighbour = |context: &HashSet<char>| {
                keys.iter()
                    .enumerate()
                    .any(|(i, c)| i != pos && context.contains(c))
            };
            for ((gk, gf), (mode, context)) in gates {
                if gk != key {
                    continue;
                }
                let matched = match mode {
                    GateMode::Never => true,
                    GateMode::Block => context.as_ref().map(contains_neighbour).unwrap_or(false),
                    GateMode::Only => context
                        .as_ref()
                        .map(|s| !contains_neighbour(s))
                        .unwrap_or(true),
                };
                if matched {
                    if let Some(fi) = finger_index(*gf) {
                        out.insert((pos, fi));
                    }
                }
            }
        }
        out
    }

    /// Penalty array of shape (NF,)*keys.len(), C-order.
    pub fn penalty_mask(
        &self,
        keys: &[char],
        conditions: &crate::conditions::Conditions,
    ) -> Vec<f64> {
        let n = keys.len();
        let mut size = 1usize;
        for _ in 0..n {
            size *= NF;
        }
        let mut out = vec![0.0; size];
        self.penalty_mask_into(keys, conditions, &mut out);
        out
    }

    pub fn penalty_mask_into(
        &self,
        keys: &[char],
        conditions: &crate::conditions::Conditions,
        out: &mut [f64],
    ) {
        let n = keys.len();
        let mut size = 1usize;
        for _ in 0..n {
            size *= NF;
        }
        let out = &mut out[..size];
        conditions.penalty_mask_into(keys, out);
        if n == 0 {
            return;
        }
        let mut forb = vec![false; n * NF];
        for (pos, key) in keys.iter().enumerate() {
            let contains_neighbour = |context: &HashSet<char>| {
                keys.iter()
                    .enumerate()
                    .any(|(i, c)| i != pos && context.contains(c))
            };
            for ((gk, gf), (mode, context)) in &self.all {
                if gk != key {
                    continue;
                }
                let matched = match mode {
                    GateMode::Never => true,
                    GateMode::Block => context.as_ref().map(contains_neighbour).unwrap_or(false),
                    GateMode::Only => context
                        .as_ref()
                        .map(|s| !contains_neighbour(s))
                        .unwrap_or(true),
                };
                if matched {
                    if let Some(fi) = finger_index(*gf) {
                        forb[pos * NF + fi] = true;
                    }
                }
            }
        }
        for (seq, fingers) in &self.sequences {
            if seq.as_slice() == keys {
                for (pos, finger) in fingers.iter().enumerate() {
                    for f in 0..NF {
                        if f != *finger {
                            forb[pos * NF + f] = true;
                        }
                    }
                }
            }
        }
        for (idx, value) in out.iter_mut().enumerate() {
            let mut div = 1usize;
            for pos in (0..n).rev() {
                let f = (idx / div) % NF;
                if forb[pos * NF + f] {
                    *value += XPEN;
                    break;
                }
                div *= NF;
            }
        }
    }
}

pub fn finger_name(index: usize) -> char {
    FINGERS[index]
}
