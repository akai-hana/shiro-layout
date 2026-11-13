//! Shared redirect classification and tier preferences.
//!
//! Redirect patterns are expressed as per-hand finger ranks, where
//! `1 = index`, `2 = middle`, `3 = ring`, and `4 = pinky`.  The ranks convert
//! to the model's finger kinds with `rank = 4 - finger_kind`.

use crate::geometry::{finger_hand, finger_kind};
use crate::model::{W_REDIRECT, W_REDIRECT_WEAK};

/// Convert an eight-state finger index to the user-facing per-hand rank.
#[inline]
pub fn rank(finger: usize) -> u8 {
    (4 - finger_kind(finger)) as u8
}

/// Whether a rank triple changes direction at the middle finger.
#[inline]
pub fn is_turn(pattern: [u8; 3]) -> bool {
    let d1 = pattern[1] as i16 - pattern[0] as i16;
    let d2 = pattern[2] as i16 - pattern[1] as i16;
    d1 * d2 < 0
}

/// Classify three eight-state fingers as a same-hand redirect pattern.
pub fn pattern_for(fs: [usize; 3]) -> Option<[u8; 3]> {
    if finger_hand(fs[0]) != finger_hand(fs[1]) || finger_hand(fs[1]) != finger_hand(fs[2]) {
        return None;
    }
    let pattern = [rank(fs[0]), rank(fs[1]), rank(fs[2])];
    is_turn(pattern).then_some(pattern)
}

/// Render a configured pattern the way it appears in the YAML file.
pub fn pattern_label(pattern: [u8; 3]) -> String {
    format!("{}-{}-{}", pattern[0], pattern[1], pattern[2])
}

/// Flat key for per-pattern analyzer output.
pub fn pattern_key(pattern: [u8; 3]) -> String {
    format!("redirect_{}_{}_{}", pattern[0], pattern[1], pattern[2])
}

/// Legacy fallback cost for an unclassified same-hand turn.  Pinky involvement
/// keeps the old `W_REDIRECT_WEAK` rule; all other turns stay at `W_REDIRECT`.
pub fn legacy_cost_for_fingers(fs: [usize; 3]) -> f64 {
    if fs.iter().any(|&f| finger_kind(f) == 0) {
        W_REDIRECT_WEAK
    } else {
        W_REDIRECT
    }
}

/// Legacy fallback cost for a rank pattern.
pub fn legacy_cost_for_pattern(pattern: [u8; 3]) -> f64 {
    if pattern.contains(&4) {
        W_REDIRECT_WEAK
    } else {
        W_REDIRECT
    }
}

/// Base cost for a zero-based tier in a table with `tier_count` tiers.
pub fn tier_cost(tier: usize, tier_count: usize) -> f64 {
    let span = tier_count.saturating_sub(1).max(1) as f64;
    W_REDIRECT + (W_REDIRECT_WEAK - W_REDIRECT) * tier as f64 / span
}

/// Base cost for a same-hand turn, using configured tiers when available and
/// the legacy rule otherwise.
pub fn redirect_cost(table: Option<&RedirectCostTable>, fs: [usize; 3]) -> f64 {
    match table {
        Some(table) => table
            .cost_for_fingers(fs)
            .or(table.fallback())
            .unwrap_or_else(|| legacy_cost_for_fingers(fs)),
        None => legacy_cost_for_fingers(fs),
    }
}

/// Ordered redirect tiers, most preferred first.
#[derive(Clone, Debug, Default)]
pub struct RedirectTiers {
    tiers: Vec<Vec<[u8; 3]>>,
    weights: Option<Vec<f64>>,
    fallback: Option<f64>,
}

impl RedirectTiers {
    pub fn new(tiers: Vec<Vec<[u8; 3]>>) -> Self {
        Self {
            tiers,
            weights: None,
            fallback: None,
        }
    }

    pub fn with_weights(tiers: Vec<Vec<[u8; 3]>>, weights: Vec<f64>) -> Self {
        let fallback = weights.last().copied();
        Self {
            tiers,
            weights: Some(weights),
            fallback,
        }
    }

    pub fn fallback(&self) -> Option<f64> {
        self.fallback
    }

    pub fn tiers(&self) -> &[Vec<[u8; 3]>] {
        &self.tiers
    }

    pub fn weights(&self) -> Option<&[f64]> {
        self.weights.as_deref()
    }

    pub fn has_weights(&self) -> bool {
        self.weights.is_some()
    }

    pub fn len(&self) -> usize {
        self.tiers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tiers.is_empty()
    }

    /// Zero-based tier index for a configured pattern.
    pub fn tier_of(&self, pattern: [u8; 3]) -> Option<usize> {
        self.tiers.iter().position(|tier| tier.contains(&pattern))
    }

    pub fn base_cost(&self, tier: usize) -> f64 {
        self.weights
            .as_ref()
            .and_then(|weights| weights.get(tier).copied())
            .unwrap_or_else(|| tier_cost(tier, self.tiers.len()))
    }

    pub fn cost(&self, pattern: [u8; 3]) -> Option<f64> {
        self.tier_of(pattern).map(|tier| self.base_cost(tier))
    }

    pub fn cost_table(&self) -> RedirectCostTable {
        let mut costs = [[[None; 4]; 4]; 4];
        for (tier, patterns) in self.tiers.iter().enumerate() {
            let cost = self.base_cost(tier);
            for &pattern in patterns {
                let index = [
                    (pattern[0] - 1) as usize,
                    (pattern[1] - 1) as usize,
                    (pattern[2] - 1) as usize,
                ];
                costs[index[0]][index[1]][index[2]] = Some(cost);
            }
        }
        RedirectCostTable {
            costs,
            fallback: self.fallback,
        }
    }
}

/// Copyable lookup table installed in [`crate::model::Scales`].
#[derive(Clone, Copy, Debug, Default)]
pub struct RedirectCostTable {
    costs: [[[Option<f64>; 4]; 4]; 4],
    fallback: Option<f64>,
}

impl RedirectCostTable {
    pub fn from_tiers(tiers: &RedirectTiers) -> Self {
        tiers.cost_table()
    }

    #[inline]
    pub fn fallback(&self) -> Option<f64> {
        self.fallback
    }

    #[inline]
    pub fn cost(&self, pattern: [u8; 3]) -> Option<f64> {
        self.costs[(pattern[0] - 1) as usize][(pattern[1] - 1) as usize][(pattern[2] - 1) as usize]
    }

    #[inline]
    pub fn cost_for_fingers(&self, fs: [usize; 3]) -> Option<f64> {
        pattern_for(fs).and_then(|pattern| self.cost(pattern))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::parse_finger8;

    fn fingers(pattern: [u8; 3], hand: usize) -> [usize; 3] {
        pattern.map(|rank| hand * 4 + (4 - rank) as usize)
    }

    #[test]
    fn rank_conversion_matches_user_notation() {
        assert_eq!(rank(parse_finger8("LI").unwrap()), 1);
        assert_eq!(rank(parse_finger8("LM").unwrap()), 2);
        assert_eq!(rank(parse_finger8("LR").unwrap()), 3);
        assert_eq!(rank(parse_finger8("LP").unwrap()), 4);
        assert_eq!(rank(parse_finger8("RI").unwrap()), 1);
    }

    #[test]
    fn patterns_are_same_hand_and_turns_only() {
        assert_eq!(pattern_for(fingers([1, 2, 1], 0)), Some([1, 2, 1]));
        assert_eq!(pattern_for(fingers([1, 2, 1], 1)), Some([1, 2, 1]));
        assert_eq!(pattern_for(fingers([1, 2, 3], 0)), None);
        assert_eq!(pattern_for(fingers([2, 2, 2], 0)), None);
        assert_eq!(
            pattern_for([
                parse_finger8("LI").unwrap(),
                parse_finger8("RI").unwrap(),
                parse_finger8("LI").unwrap(),
            ]),
            None
        );
    }

    #[test]
    fn tier_costs_are_strictly_ordered() {
        let patterns = [
            vec![[1, 2, 1]],
            vec![[3, 2, 3]],
            vec![[2, 1, 3]],
            vec![[4, 1, 4]],
            vec![[3, 4, 3]],
        ];
        let tiers = RedirectTiers::new(patterns.to_vec());
        let costs: Vec<f64> = (0..tiers.len()).map(|t| tiers.base_cost(t)).collect();
        assert_eq!(costs[0], W_REDIRECT);
        assert_eq!(costs[4], W_REDIRECT_WEAK);
        assert!(costs.windows(2).all(|w| w[0] < w[1]), "{costs:?}");
        let table = tiers.cost_table();
        assert_eq!(table.cost([1, 2, 1]), Some(W_REDIRECT));
        assert_eq!(table.cost([3, 4, 3]), Some(W_REDIRECT_WEAK));
        assert_eq!(table.cost([1, 3, 2]), None);
    }

    #[test]
    fn configured_and_fallback_costs_route_through_the_table() {
        let tiers = RedirectTiers::new(vec![vec![[1, 2, 1]], vec![[3, 2, 3]]]);
        let table = tiers.cost_table();
        let ring_middle_ring = fingers([3, 2, 3], 0);
        assert_eq!(
            redirect_cost(Some(&table), ring_middle_ring),
            tiers.base_cost(1)
        );
        assert_eq!(
            redirect_cost(Some(&table), fingers([1, 3, 2], 0)),
            W_REDIRECT
        );
        assert_eq!(
            redirect_cost(Some(&table), fingers([1, 4, 2], 0)),
            W_REDIRECT_WEAK
        );
        assert_eq!(redirect_cost(None, ring_middle_ring), W_REDIRECT);
    }

    #[test]
    fn weighted_tiers_use_the_last_weight_as_the_unlisted_floor() {
        let tiers =
            RedirectTiers::with_weights(vec![vec![[1, 2, 1]], vec![[3, 2, 3]]], vec![0.0, 2.5]);
        let table = tiers.cost_table();
        assert_eq!(table.cost([1, 2, 1]), Some(0.0));
        assert_eq!(table.cost([3, 2, 3]), Some(2.5));
        assert_eq!(redirect_cost(Some(&table), fingers([1, 3, 2], 0)), 2.5);
        assert_eq!(redirect_cost(Some(&table), fingers([1, 4, 2], 0)), 2.5);
    }

    #[test]
    fn legacy_fallback_only_weighs_pinky_patterns_as_weak() {
        assert_eq!(legacy_cost_for_pattern([1, 3, 2]), W_REDIRECT);
        assert_eq!(legacy_cost_for_pattern([1, 4, 2]), W_REDIRECT_WEAK);
        assert_eq!(
            legacy_cost_for_fingers(fingers([2, 4, 2], 0)),
            W_REDIRECT_WEAK
        );
    }
}
