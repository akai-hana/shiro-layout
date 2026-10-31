//! Exact alt-finger ngram rules.
//!
//! Rules are keyed by ordered QWERTY physical-key sequences.  A longer rule
//! overrides shorter rules that overlap it, so a trigram rule wins over a
//! bigram rule contained in it.  `allow` rules force the listed fingering;
//! repeated definitions of the same ngram are tried in order, with later
//! definitions used only when the earlier fingering is impossible.  `block`
//! rules forbid exactly the listed fingering.

use crate::conditions::Conditions;
use crate::config::{AltMode, AltRule, Config};
use crate::geometry::{finger_hand, NF};

pub const XPEN: f64 = 12.0;
pub const FALLBACK_PENALTY: f64 = XPEN;

#[derive(Clone, Debug)]
pub struct Gates {
    pub rules: Vec<AltRule>,
}

impl Gates {
    pub fn from_config(cfg: &Config) -> Self {
        Self {
            rules: cfg.alt_fingers.clone(),
        }
    }

    pub fn matching_len(&self, keys: &[char]) -> usize {
        let mut best = 0;
        for rule in &self.rules {
            let len = rule.keys.len();
            if len <= keys.len() && len > best && windows(keys, &rule.keys).next().is_some() {
                best = len;
            }
        }
        best
    }

    /// Penalty array of shape (NF,)*keys.len(), C-order.
    pub fn penalty_mask(&self, keys: &[char], conditions: &Conditions) -> Vec<f64> {
        let n = keys.len();
        let mut size = 1usize;
        for _ in 0..n {
            size *= NF;
        }
        let mut out = vec![0.0; size];
        self.penalty_mask_into(keys, conditions, &mut out);
        out
    }

    pub fn penalty_mask_into(&self, keys: &[char], conditions: &Conditions, out: &mut [f64]) {
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
        let mut allow_len = 0;
        for rule in &self.rules {
            let len = rule.keys.len();
            if rule.mode == AltMode::Allow
                && len <= keys.len()
                && len > allow_len
                && windows(keys, &rule.keys).next().is_some()
            {
                allow_len = len;
            }
        }
        let mut allow_groups: Vec<(usize, Vec<&AltRule>)> = Vec::new();
        let mut block_rules: Vec<(usize, &AltRule)> = Vec::new();
        for rule in &self.rules {
            let len = rule.keys.len();
            if len < allow_len {
                continue;
            }
            for start in windows(keys, &rule.keys) {
                match rule.mode {
                    AltMode::Allow if len == allow_len => {
                        if let Some(group) = allow_groups.iter_mut().find(|(s, _)| *s == start) {
                            group.1.push(rule);
                        } else {
                            allow_groups.push((start, vec![rule]));
                        }
                    }
                    AltMode::Block => block_rules.push((start, rule)),
                    _ => {}
                }
            }
        }
        let key_hands: Vec<usize> = keys
            .iter()
            .map(|key| crate::geometry::hand_of(&key.to_string()))
            .collect();
        for (idx, value) in out.iter_mut().enumerate() {
            let fingers = decode(idx, n);
            if block_rules.iter().any(|(start, rule)| {
                let len = rule.keys.len();
                fingers[*start..*start + len] == rule.fingers[..]
            }) {
                *value = f64::INFINITY;
                continue;
            }
            let mut allowed = true;
            let mut fallback = 0.0;
            for (start, group) in &allow_groups {
                match group
                    .iter()
                    .position(|rule| fingers[*start..*start + allow_len] == rule.fingers[..])
                {
                    Some(pos) => fallback += pos as f64 * FALLBACK_PENALTY,
                    None => {
                        allowed = false;
                        break;
                    }
                }
            }
            if allowed {
                *value += fallback;
            } else {
                *value = f64::INFINITY;
                continue;
            }
            // Cross-hand fingerings are impossible by default.  An allow rule
            // must name the exact cross-hand state at that sequence position.
            for (pos, &finger) in fingers.iter().enumerate() {
                if finger_hand(finger) == key_hands[pos] {
                    continue;
                }
                let licensed = allow_groups.iter().any(|(start, group)| {
                    pos >= *start
                        && pos < *start + allow_len
                        && group
                            .iter()
                            .any(|rule| rule.fingers[pos - *start] == finger)
                });
                if !licensed {
                    *value = f64::INFINITY;
                    break;
                }
            }
        }
    }
}

pub fn windows<'a>(keys: &'a [char], pattern: &'a [char]) -> impl Iterator<Item = usize> + 'a {
    let n = keys.len();
    let m = pattern.len();
    (0..=n.saturating_sub(m)).filter(move |&start| m <= n && keys[start..start + m] == *pattern)
}

pub fn decode(mut idx: usize, n: usize) -> Vec<usize> {
    let mut out = vec![0usize; n];
    for pos in (0..n).rev() {
        out[pos] = idx % NF;
        idx /= NF;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conditions::Conditions;
    use crate::config::{AltMode, AltRule};
    use crate::geometry::parse_finger8;

    fn conditions_off() -> Conditions {
        Conditions {
            enabled: false,
            rules: Vec::new(),
            pair_denied: None,
        }
    }

    #[test]
    fn cross_hand_states_need_an_explicit_allow_rule() {
        let conditions = conditions_off();
        let li = parse_finger8("LI").unwrap();
        let ri = parse_finger8("RI").unwrap();
        let lm = parse_finger8("LM").unwrap();
        let rm = parse_finger8("RM").unwrap();

        let plain = Gates { rules: Vec::new() };
        let h = plain.penalty_mask(&['H'], &conditions);
        assert!(h[li].is_infinite());
        assert!(h[ri].is_finite());
        let y = plain.penalty_mask(&['Y'], &conditions);
        assert!(y[lm].is_infinite());
        assert!(y[rm].is_finite());

        let licensed = Gates {
            rules: vec![
                AltRule {
                    keys: vec!['H'],
                    fingers: vec![li],
                    mode: AltMode::Allow,
                },
                AltRule {
                    keys: vec!['Y'],
                    fingers: vec![lm],
                    mode: AltMode::Allow,
                },
            ],
        };
        let h = licensed.penalty_mask(&['H'], &conditions);
        assert!(h[li].is_finite());
        assert!(h[ri].is_infinite());
        let y = licensed.penalty_mask(&['Y'], &conditions);
        assert!(y[lm].is_finite());
        assert!(y[rm].is_infinite());
    }
}
