//! Scissor severity bands and configurable tier multipliers.
//!
//! Scissor cost is continuous in the geometry model.  The bands below turn the
//! base pair strain into four named severity levels so the config can weight
//! the worst forms like SFB/SFT while keeping mild stretches cheap.

/// Named severity bands from mild to extreme.
pub const SCISSOR_BANDS: [&str; 4] = ["mild", "moderate", "severe", "extreme"];

/// Upper bounds for the first three bands; the fourth band is the catch-all.
pub const SCISSOR_BAND_THRESHOLDS: [f64; 3] = [3.0, 5.0, 8.0];

/// Approximate key-press baseline added to the intrinsic pair strain so the
/// band thresholds line up with the analyzer's bigram cost.
pub const SCISSOR_STRAIN_OFFSET: f64 = 2.0;

/// Zero-based severity band for a base pair strain.
pub fn band(cost: f64) -> usize {
    if cost < SCISSOR_BAND_THRESHOLDS[0] {
        0
    } else if cost < SCISSOR_BAND_THRESHOLDS[1] {
        1
    } else if cost < SCISSOR_BAND_THRESHOLDS[2] {
        2
    } else {
        3
    }
}

/// Configured multiplier for every severity band.
#[derive(Clone, Debug)]
pub struct ScissorTiers {
    multipliers: [f64; 4],
}

impl ScissorTiers {
    pub fn new(multipliers: [f64; 4]) -> Self {
        Self { multipliers }
    }

    pub fn multipliers(&self) -> [f64; 4] {
        self.multipliers
    }

    pub fn table(&self) -> ScissorTierTable {
        ScissorTierTable {
            multipliers: self.multipliers,
        }
    }
}

/// Copyable lookup table installed in [`crate::model::Scales`].
#[derive(Clone, Copy, Debug, Default)]
pub struct ScissorTierTable {
    multipliers: [f64; 4],
}

impl ScissorTierTable {
    pub fn new(multipliers: [f64; 4]) -> Self {
        Self { multipliers }
    }

    pub fn multipliers(&self) -> [f64; 4] {
        self.multipliers
    }

    #[inline]
    pub fn multiplier_for_band(&self, band: usize) -> f64 {
        self.multipliers[band.min(3)]
    }

    #[inline]
    pub fn multiplier(&self, cost: f64) -> f64 {
        self.multiplier_for_band(band(cost))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bands_follow_the_documented_thresholds() {
        assert_eq!(band(SCISSOR_BAND_THRESHOLDS[0] - 0.001), 0);
        assert_eq!(band(SCISSOR_BAND_THRESHOLDS[0]), 1);
        assert_eq!(band(SCISSOR_BAND_THRESHOLDS[1]), 2);
        assert_eq!(band(SCISSOR_BAND_THRESHOLDS[2]), 3);
        assert_eq!(band(100.0), 3);
    }

    #[test]
    fn tier_table_selects_the_named_band_multiplier() {
        let table = ScissorTierTable::new([0.5, 0.75, 1.0, 0.25]);
        assert_eq!(table.multiplier(1.0), 0.5);
        assert_eq!(table.multiplier(4.0), 0.75);
        assert_eq!(table.multiplier(6.0), 1.0);
        assert_eq!(table.multiplier(9.0), 0.25);
        assert_eq!(table.multiplier_for_band(99), 0.25);
    }
}
