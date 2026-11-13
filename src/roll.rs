//! Roll tier preferences.
//!
//! A roll is two consecutive keys typed by adjacent fingers of one hand.  The
//! config names directed finger pairs as `"from-to"` per-hand ranks, where
//! `1 = index`, `2 = middle`, `3 = ring`, and `4 = pinky` (the same notation
//! as `redirect_tiers`).  `"1-2"` is index then middle, `"2-1"` is middle then
//! index.  Tiers run from most preferred (tier 1) to least preferred.

use crate::geometry::finger_hand;
use crate::model::W_ROLL_TIER_BEST;
use crate::redirect::rank;

/// Whether two ranks are neighbouring fingers.
#[inline]
pub fn is_adjacent(pair: [u8; 2]) -> bool {
    pair[0].abs_diff(pair[1]) == 1
}

/// Classify two eight-state fingers as a directed same-hand adjacent pair.
pub fn pair_for(fs: [usize; 2]) -> Option<[u8; 2]> {
    if finger_hand(fs[0]) != finger_hand(fs[1]) {
        return None;
    }
    let pair = [rank(fs[0]), rank(fs[1])];
    is_adjacent(pair).then_some(pair)
}

/// Render a configured pair the way it appears in the YAML file.
pub fn pair_label(pair: [u8; 2]) -> String {
    format!("{}-{}", pair[0], pair[1])
}

/// Flat key for per-pair analyzer output.
pub fn pair_key(pair: [u8; 2]) -> String {
    format!("roll_{}_{}", pair[0], pair[1])
}

/// Roll credit for a zero-based tier in a table with `tier_count` tiers.
/// Tier 1 earns the full `W_ROLL_TIER_BEST`; each later tier earns a smaller
/// equal share, so the last of N tiers still earns `W_ROLL_TIER_BEST / N`.
pub fn tier_credit(tier: usize, tier_count: usize) -> f64 {
    let count = tier_count.max(1);
    W_ROLL_TIER_BEST * (count - tier.min(count - 1)) as f64 / count as f64
}

/// Ordered roll tiers, most preferred first.
#[derive(Clone, Debug, Default)]
pub struct RollTiers {
    tiers: Vec<Vec<[u8; 2]>>,
}

impl RollTiers {
    pub fn new(tiers: Vec<Vec<[u8; 2]>>) -> Self {
        Self { tiers }
    }

    pub fn tiers(&self) -> &[Vec<[u8; 2]>] {
        &self.tiers
    }

    pub fn len(&self) -> usize {
        self.tiers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tiers.is_empty()
    }

    /// Zero-based tier index for a configured pair.
    pub fn tier_of(&self, pair: [u8; 2]) -> Option<usize> {
        self.tiers.iter().position(|tier| tier.contains(&pair))
    }

    pub fn base_credit(&self, tier: usize) -> f64 {
        tier_credit(tier, self.tiers.len())
    }

    pub fn credit(&self, pair: [u8; 2]) -> Option<f64> {
        self.tier_of(pair).map(|tier| self.base_credit(tier))
    }

    pub fn credit_table(&self) -> RollCreditTable {
        let mut credits = [[None; 4]; 4];
        for (tier, pairs) in self.tiers.iter().enumerate() {
            let credit = self.base_credit(tier);
            for &pair in pairs {
                credits[(pair[0] - 1) as usize][(pair[1] - 1) as usize] = Some(credit);
            }
        }
        RollCreditTable { credits }
    }
}

/// Copyable lookup table installed in [`crate::model::Scales`].
#[derive(Clone, Copy, Debug, Default)]
pub struct RollCreditTable {
    credits: [[Option<f64>; 4]; 4],
}

impl RollCreditTable {
    #[inline]
    pub fn credit(&self, pair: [u8; 2]) -> Option<f64> {
        self.credits[(pair[0] - 1) as usize][(pair[1] - 1) as usize]
    }

    #[inline]
    pub fn credit_for_fingers(&self, fs: [usize; 2]) -> Option<f64> {
        pair_for(fs).and_then(|pair| self.credit(pair))
    }
}

/// Directed adjacent pair for two finger kinds (`P=0 .. I=3`), for callers that
/// already know the fingers share a hand.
#[inline]
pub fn pair_for_kinds(from: usize, to: usize) -> Option<[u8; 2]> {
    let pair = [(4 - from) as u8, (4 - to) as u8];
    is_adjacent(pair).then_some(pair)
}

/// Every directed adjacent pair, in the order analyzers list them.
pub const ALL_PAIRS: [[u8; 2]; 6] = [[1, 2], [2, 1], [2, 3], [3, 2], [3, 4], [4, 3]];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::parse_finger8;

    fn fingers(pair: [u8; 2], hand: usize) -> [usize; 2] {
        pair.map(|rank| hand * 4 + (4 - rank) as usize)
    }

    #[test]
    fn pairs_are_same_hand_and_adjacent_only() {
        assert_eq!(pair_for(fingers([1, 2], 0)), Some([1, 2]));
        assert_eq!(pair_for(fingers([2, 1], 1)), Some([2, 1]));
        assert_eq!(pair_for(fingers([1, 3], 0)), None);
        assert_eq!(pair_for(fingers([2, 2], 0)), None);
        assert_eq!(
            pair_for([parse_finger8("LI").unwrap(), parse_finger8("RM").unwrap()]),
            None
        );
    }

    #[test]
    fn credits_shrink_with_the_tier() {
        let tiers = RollTiers::new(vec![vec![[1, 2]], vec![[2, 1]], vec![[3, 2]]]);
        let credits: Vec<f64> = (0..tiers.len()).map(|t| tiers.base_credit(t)).collect();
        assert!((credits[0] - W_ROLL_TIER_BEST).abs() < 1e-12);
        assert!((credits[2] - W_ROLL_TIER_BEST / 3.0).abs() < 1e-12);
        assert!(credits.windows(2).all(|w| w[0] < w[1] && w[1] < 0.0));
        let one = RollTiers::new(vec![vec![[1, 2], [2, 1]]]);
        assert_eq!(one.base_credit(0), W_ROLL_TIER_BEST);
    }

    #[test]
    fn table_prices_only_configured_pairs_and_keeps_direction() {
        let tiers = RollTiers::new(vec![vec![[1, 2]], vec![[3, 2]]]);
        let table = tiers.credit_table();
        assert_eq!(table.credit([1, 2]), Some(W_ROLL_TIER_BEST));
        assert_eq!(table.credit([3, 2]), Some(W_ROLL_TIER_BEST / 2.0));
        assert_eq!(table.credit([2, 1]), None);
        assert_eq!(table.credit([2, 3]), None);
        let left = fingers([1, 2], 0);
        let right = fingers([1, 2], 1);
        assert_eq!(table.credit_for_fingers(left), Some(W_ROLL_TIER_BEST));
        assert_eq!(table.credit_for_fingers(right), Some(W_ROLL_TIER_BEST));
    }

    #[test]
    fn kind_conversion_matches_rank_notation() {
        // finger kinds: P=0, R=1, M=2, I=3
        assert_eq!(pair_for_kinds(3, 2), Some([1, 2]));
        assert_eq!(pair_for_kinds(2, 3), Some([2, 1]));
        assert_eq!(pair_for_kinds(1, 0), Some([3, 4]));
        assert_eq!(pair_for_kinds(3, 1), None);
        assert_eq!(ALL_PAIRS.len(), 6);
        assert!(ALL_PAIRS.iter().all(|&p| is_adjacent(p)));
    }
}
