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
            all.insert(
                (gate.key, gate.finger),
                (gate.mode, gate.context.clone()),
            );
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
                    s.fingers.iter().map(|f| finger_index(*f).unwrap()).collect(),
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
            let neighbours: Vec<char> = keys
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != pos)
                .map(|(_, c)| *c)
                .collect();
            for ((gk, gf), (mode, context)) in gates {
                if gk != key {
                    continue;
                }
                let matched = match mode {
                    GateMode::Never => true,
                    GateMode::Block => context
                        .as_ref()
                        .map(|s| neighbours.iter().any(|n| s.contains(n)))
                        .unwrap_or(false),
                    GateMode::Only => context
                        .as_ref()
                        .map(|s| !neighbours.iter().any(|n| s.contains(n)))
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
    pub fn penalty_mask(&self, keys: &[char], conditions: &crate::conditions::Conditions) -> Vec<f64> {
        let n = keys.len();
        let mut size = 1usize;
        for _ in 0..n {
            size *= NF;
        }
        let mut out = vec![0.0; size];
        if n == 0 {
            return out;
        }
        let forb = self.forbidden_pairs(keys, &self.all);
        let mut hit = vec![false; size];
        for (pos, fi) in forb {
            for idx in 0..size {
                let f = (idx / NF.pow((n - 1 - pos) as u32)) % NF;
                if f == fi {
                    hit[idx] = true;
                }
            }
        }
        for (seq, fingers) in &self.sequences {
            if seq.as_slice() == keys {
                for (pos, finger) in fingers.iter().enumerate() {
                    for idx in 0..size {
                        let f = (idx / NF.pow((n - 1 - pos) as u32)) % NF;
                        if f != *finger {
                            hit[idx] = true;
                        }
                    }
                }
            }
        }
        let cond = conditions.penalty_mask(keys);
        for i in 0..size {
            out[i] = if hit[i] { XPEN } else { 0.0 } + cond[i];
        }
        out
    }
}

pub fn finger_name(index: usize) -> char {
    FINGERS[index]
}
