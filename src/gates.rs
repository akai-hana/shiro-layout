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

#[derive(Clone, Copy)]
struct RuleMatch {
    start: usize,
    len: usize,
    rule_idx: usize,
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

    fn active_matches(&self, keys: &[char]) -> Vec<RuleMatch> {
        let n = keys.len();
        let matches: Vec<RuleMatch> = self
            .rules
            .iter()
            .enumerate()
            .flat_map(|(rule_idx, rule)| {
                let len = rule.keys.len();
                if len == 0 || len > n {
                    return Vec::new();
                }
                windows(keys, &rule.keys)
                    .map(|start| RuleMatch {
                        start,
                        len,
                        rule_idx,
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        // A longer allow rule overrides any shorter rule whose range it
        // overlaps.  A longer block rule adds a restriction but does not
        // cancel a shorter block, so overlapping block rules all stay active.
        matches
            .iter()
            .copied()
            .filter(|m| {
                !matches.iter().any(|other| {
                    other.len > m.len
                        && self.rules[other.rule_idx].mode == AltMode::Allow
                        && m.start < other.start + other.len
                        && other.start < m.start + m.len
                })
            })
            .collect()
    }

    /// Fingers explicitly named by active `allow` rules, per sequence position.
    pub fn explicit_fingers(&self, keys: &[char]) -> Vec<[bool; NF]> {
        let mut out = vec![[false; NF]; keys.len()];
        for m in self.active_matches(keys) {
            let rule = &self.rules[m.rule_idx];
            if !matches!(rule.mode, AltMode::Allow | AltMode::Permit) {
                continue;
            }
            for (offset, &finger) in rule.fingers.iter().enumerate() {
                out[m.start + offset][finger] = true;
            }
        }
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

        let matches = self.active_matches(keys);

        let mut allow_groups: Vec<(usize, usize, Vec<usize>)> = Vec::new();
        let mut permit_rules: Vec<(usize, usize, usize)> = Vec::new();
        let mut block_rules: Vec<(usize, usize, usize)> = Vec::new();
        for m in matches {
            match self.rules[m.rule_idx].mode {
                AltMode::Allow => {
                    if let Some(group) = allow_groups
                        .iter_mut()
                        .find(|(start, len, _)| *start == m.start && *len == m.len)
                    {
                        group.2.push(m.rule_idx);
                    } else {
                        allow_groups.push((m.start, m.len, vec![m.rule_idx]));
                    }
                }
                AltMode::Permit => permit_rules.push((m.start, m.len, m.rule_idx)),
                AltMode::Block => block_rules.push((m.start, m.len, m.rule_idx)),
            }
        }

        let key_hands: Vec<usize> = keys
            .iter()
            .map(|key| crate::geometry::hand_of(&key.to_string()))
            .collect();
        'tuple: for (idx, value) in out.iter_mut().enumerate() {
            let fingers = decode(idx, n);
            for &(start, len, rule_idx) in &block_rules {
                if fingers[start..start + len] == self.rules[rule_idx].fingers[..] {
                    *value = f64::INFINITY;
                    continue 'tuple;
                }
            }
            let mut fallback = 0.0;
            for (start, len, group) in &allow_groups {
                match group.iter().position(|&rule_idx| {
                    fingers[*start..*start + *len] == self.rules[rule_idx].fingers[..]
                }) {
                    Some(pos) => fallback += pos as f64 * FALLBACK_PENALTY,
                    None => {
                        *value = f64::INFINITY;
                        continue 'tuple;
                    }
                }
            }
            // Cross-hand fingerings are impossible by default.  An active allow
            // rule must name the exact cross-hand state at that sequence position.
            for (pos, &finger) in fingers.iter().enumerate() {
                if finger_hand(finger) == key_hands[pos] {
                    continue;
                }
                let licensed = allow_groups.iter().any(|(start, len, group)| {
                    pos >= *start
                        && pos < *start + *len
                        && group
                            .iter()
                            .any(|&rule_idx| self.rules[rule_idx].fingers[pos - *start] == finger)
                }) || permit_rules.iter().any(|(start, len, rule_idx)| {
                    pos >= *start
                        && pos < *start + *len
                        && self.rules[*rule_idx].fingers[pos - *start] == finger
                });
                if !licensed {
                    *value = f64::INFINITY;
                    continue 'tuple;
                }
            }
            *value += fallback;
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
    fn index(fingers: &[usize]) -> usize {
        fingers.iter().fold(0, |acc, &f| acc * NF + f)
    }

    #[test]
    fn explicit_allow_rules_license_requested_unreachable_fingers() {
        let conditions = conditions_off();
        let lr = parse_finger8("LR").unwrap();
        let li = parse_finger8("LI").unwrap();
        let lp = parse_finger8("LP").unwrap();
        let gates = Gates {
            rules: vec![
                AltRule {
                    keys: vec!['C', 'H', 'X'],
                    fingers: vec![lr, li, lp],
                    mode: AltMode::Allow,
                },
                AltRule {
                    keys: vec!['H', 'X'],
                    fingers: vec![li, lp],
                    mode: AltMode::Allow,
                },
            ],
        };
        let chx = gates.penalty_mask(&['C', 'H', 'X'], &conditions);
        assert!(chx[index(&[lr, li, lp])].is_finite());
        assert!(chx[index(&[li, li, lp])].is_infinite());
        let hx = gates.penalty_mask(&['H', 'X'], &conditions);
        assert!(hx[index(&[li, lp])].is_finite());
        assert!(hx[index(&[li, lr])].is_infinite());
    }

    #[test]
    fn non_overlapping_shorter_allow_rules_still_apply() {
        let conditions = conditions_off();
        let li = parse_finger8("LI").unwrap();
        let lm = parse_finger8("LM").unwrap();
        let ri = parse_finger8("RI").unwrap();
        let gates = Gates {
            rules: vec![
                AltRule {
                    keys: vec!['C', 'X'],
                    fingers: vec![li, lm],
                    mode: AltMode::Allow,
                },
                AltRule {
                    keys: vec!['H'],
                    fingers: vec![li],
                    mode: AltMode::Allow,
                },
            ],
        };
        let mask = gates.penalty_mask(&['C', 'X', 'H'], &conditions);
        assert!(mask[index(&[li, lm, li])].is_finite());
        assert!(mask[index(&[li, lm, ri])].is_infinite());
    }

    #[test]
    fn longer_allow_rule_overrides_overlapping_block() {
        let conditions = conditions_off();
        let lr = parse_finger8("LR").unwrap();
        let lm = parse_finger8("LM").unwrap();
        let gates = Gates {
            rules: vec![
                AltRule {
                    keys: vec!['W'],
                    fingers: vec![lm],
                    mode: AltMode::Block,
                },
                AltRule {
                    keys: vec!['Q', 'W'],
                    fingers: vec![lr, lm],
                    mode: AltMode::Allow,
                },
            ],
        };
        let mask = gates.penalty_mask(&['Q', 'W'], &conditions);
        assert!(mask[index(&[lr, lm])].is_finite());
        assert!(mask[index(&[lr, lr])].is_infinite());
    }

    #[test]
    fn longer_block_rule_does_not_cancel_shorter_block() {
        let conditions = conditions_off();
        let lp = parse_finger8("LP").unwrap();
        let li = parse_finger8("LI").unwrap();
        let lm = parse_finger8("LM").unwrap();
        let gates = Gates {
            rules: vec![
                AltRule {
                    keys: vec!['G'],
                    fingers: vec![lm],
                    mode: AltMode::Block,
                },
                AltRule {
                    keys: vec!['V', 'G'],
                    fingers: vec![lp, lm],
                    mode: AltMode::Block,
                },
            ],
        };
        let mask = gates.penalty_mask(&['V', 'G'], &conditions);
        assert!(mask[index(&[lm, lm])].is_infinite());
        assert!(mask[index(&[lp, lm])].is_infinite());
        assert!(mask[index(&[lp, li])].is_finite());
    }

    #[test]
    fn repeated_key_context_rule_can_lift_block_at_other_occurrence() {
        let conditions = conditions_off();
        let lr = parse_finger8("LR").unwrap();
        let lm = parse_finger8("LM").unwrap();
        let gates = Gates {
            rules: vec![
                AltRule {
                    keys: vec!['W'],
                    fingers: vec![lm],
                    mode: AltMode::Block,
                },
                AltRule {
                    keys: vec!['Q', 'W'],
                    fingers: vec![lr, lm],
                    mode: AltMode::Allow,
                },
                AltRule {
                    keys: vec!['Q', 'W', 'W'],
                    fingers: vec![lr, lm, lm],
                    mode: AltMode::Allow,
                },
            ],
        };
        let mask = gates.penalty_mask(&['Q', 'W', 'W'], &conditions);
        assert!(mask[index(&[lr, lm, lm])].is_finite());
    }
}
