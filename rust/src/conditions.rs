//! Forbidden-condition evaluator used when `conditions` has a non-default set.
//!
//! Port of `shirolyzer/utils/conditions.py`.

use crate::config::{Condition, Config, FingerSet};
use crate::geometry::{finger_index, Geometry, NF, NK};
use crate::model::{pair_feature, pt_idx};

pub const PENALTY: f64 = 12.0;

#[derive(Clone, Debug)]
pub struct Conditions {
    pub enabled: bool,
    pub rules: Vec<Condition>,
    pub pair_denied: Option<Vec<bool>>,
    pub explicit_fingers: Vec<Vec<FingerSet>>,
}

impl Conditions {
    pub fn from_config(cfg: &Config, geom: &Geometry) -> Self {
        let enabled = !cfg.conditions_are_default;
        let rules = if enabled { cfg.conditions.clone() } else { Vec::new() };
        let mut pair_denied = None;
        if enabled {
            let mut denied = vec![false; NK * NF * NK * NF];
            for a in 0..NK {
                for fa in 0..NF {
                    for b in 0..NK {
                        for fb in 0..NF {
                            denied[pt_idx(a, fa, b, fb)] =
                                !allowed_pair(geom, &rules, a, fa, b, fb);
                        }
                    }
                }
            }
            pair_denied = Some(denied);
        }
        let mut explicit_fingers = vec![vec![FingerSet::new(); NF]; NK];
        for gate in cfg.gate_entries() {
            if !gate.explicit {
                continue;
            }
            if let (Some(k), Some(f)) = (
                crate::geometry::key_index(&gate.key.to_string()),
                finger_index(gate.finger),
            ) {
                explicit_fingers[k][f] = gate.context.clone().unwrap_or_default();
            }
        }
        Self {
            enabled,
            rules,
            pair_denied,
            explicit_fingers,
        }
    }

    pub fn matches_fingers(&self, cond: &Condition, hand_a: usize, fa: usize, hand_b: usize, fb: usize) -> bool {
        let Some(fingers) = &cond.fingers else {
            return true;
        };
        fingers.contains(&(hand_a, fa)) || fingers.contains(&(hand_b, fb))
    }

    pub fn condition_matches(
        &self,
        cond: &Condition,
        hand_a: usize,
        fa: usize,
        hand_b: usize,
        fb: usize,
        pf: &crate::model::PairFeature,
    ) -> bool {
        if !self.matches_fingers(cond, hand_a, fa, hand_b, fb) {
            return false;
        }
        let same_hand = hand_a == hand_b;
        match cond.kind.as_str() {
            "scissors" => {
                let drow = pf.drow;
                let matched = match cond.distance.as_deref() {
                    Some("full") => drow.abs() >= 2.0,
                    Some("half") => drow.abs() == 1.0,
                    _ => false,
                };
                matched && (!cond.reverse || pf.g < 0.0)
            }
            "spread" => !same_hand && pf.nadj == 1 && pf.s > pf.rest_sep + cond.max_extra,
            "separation" => !same_hand && pf.s < cond.min_width,
            "crossing" => !same_hand && pf.nadj >= 2 && pf.s < cond.deficit + 1.0,
            _ => false,
        }
    }

    pub fn penalty_mask(&self, keys: &[char]) -> Vec<f64> {
        let n = keys.len();
        let mut size = 1usize;
        for _ in 0..n {
            size *= NF;
        }
        let mut out = vec![0.0; size];
        if !self.enabled || n < 2 {
            return out;
        }
        let Some(denied) = &self.pair_denied else {
            return out;
        };
        let ids: Vec<usize> = keys
            .iter()
            .map(|c| crate::geometry::key_index(&c.to_string()).unwrap())
            .collect();
        for idx in 0..size {
            let mut fingers = [0usize; 3];
            for pos in 0..n {
                fingers[pos] = (idx / NF.pow((n - 1 - pos) as u32)) % NF;
            }
            let mut penalty = 0.0;
            for pos in 0..n - 1 {
                let a = ids[pos];
                let fa = fingers[pos];
                let b = ids[pos + 1];
                let fb = fingers[pos + 1];
                if denied[pt_idx(a, fa, b, fb)] {
                    penalty += PENALTY;
                }
            }
            out[idx] = penalty;
        }
        out
    }
}

pub fn allowed_pair(geom: &Geometry, rules: &[Condition], a: usize, fa: usize, b: usize, fb: usize) -> bool {
    if geom.hand[a] != geom.hand[b] || fa == fb {
        return true;
    }
    let pf = pair_feature(geom, a, fa, b, fb);
    let conditions = Conditions {
        enabled: true,
        rules: rules.to_vec(),
        pair_denied: None,
        explicit_fingers: Vec::new(),
    };
    for cond in rules {
        if conditions.condition_matches(cond, geom.hand[a], fa, geom.hand[b], fb, &pf) {
            return cond.allow;
        }
    }
    true
}
