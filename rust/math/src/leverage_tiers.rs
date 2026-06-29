//! Leverage tiers for position-size-dependent margin requirements
//!
//! Leverage tiers allow limiting leverage as position size increases,
//! implementing progressive margin requirements for larger positions.

use crate::quantities::{BaseLots, BasisPoints, Constant, ScalarBounds};

/// A single leverage tier defining maximum leverage for a position size range
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct LeverageTier {
    /// For all position amounts less than or equal to this bound, the max
    /// leverage is the indicated value
    pub upper_bound_size: BaseLots,
    /// The max leverage allowed for this quantity tier
    pub max_leverage: Constant,
    /// The risk factor for limit orders (basis points)
    pub limit_order_risk_factor: BasisPoints,
}

/// Collection of 4 leverage tiers with interpolation support
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct LeverageTiers {
    tiers: [LeverageTier; 4],
}

impl LeverageTiers {
    /// Create new leverage tiers with validation
    pub fn new(tiers: [LeverageTier; 4]) -> Result<Self, &'static str> {
        Self::validate(&tiers)?;
        Ok(Self { tiers })
    }

    /// Create leverage tiers without validation (use with caution)
    pub const fn new_unchecked(tiers: [LeverageTier; 4]) -> Self {
        Self { tiers }
    }

    /// Validate leverage tier configuration
    pub fn validate(tiers: &[LeverageTier; 4]) -> Result<(), &'static str> {
        for i in 1..tiers.len() {
            let prev_tier = &tiers[i - 1];
            let curr_tier = &tiers[i];

            if curr_tier.upper_bound_size == BaseLots::ZERO
                || prev_tier.upper_bound_size == BaseLots::ZERO
            {
                return Err("Leverage tier upper_bound_size cannot be zero");
            }

            // Check that upper_bound_size is increasing
            if curr_tier.upper_bound_size <= prev_tier.upper_bound_size {
                return Err("Leverage tiers must have increasing upper_bound_size");
            }

            // Check that max_leverage is non-increasing
            if curr_tier.max_leverage > prev_tier.max_leverage {
                return Err("Leverage tiers must have non-increasing max_leverage");
            }

            // Check that limit_order_risk_factor is non-decreasing
            if curr_tier.limit_order_risk_factor < prev_tier.limit_order_risk_factor {
                return Err("Leverage tiers must have non-decreasing limit_order_risk_factor");
            }
        }

        Ok(())
    }

    /// Get interpolated leverage constant for a given position size
    ///
    /// Linearly interpolates between tier boundaries to provide smooth leverage
    /// scaling
    pub fn get_leverage_constant(&self, position_size: BaseLots) -> Constant {
        for (i, tier) in self.tiers.iter().enumerate() {
            if position_size <= tier.upper_bound_size {
                if i == 0 {
                    // First tier: no interpolation needed
                    return tier.max_leverage;
                }

                // Interpolate between previous tier and current tier
                let prev_tier = &self.tiers[i - 1];
                return interpolate_leverage(
                    prev_tier.upper_bound_size,
                    prev_tier.max_leverage,
                    tier.upper_bound_size,
                    tier.max_leverage,
                    position_size,
                );
            }
        }
        // Position exceeds all tiers - use minimum leverage (1x)
        Constant::new(1)
    }

    /// Get interpolated limit order risk factor for a given position size
    ///
    /// Linearly interpolates between tier boundaries
    pub fn get_limit_order_risk_factor(&self, position_size: BaseLots) -> BasisPoints {
        for (i, tier) in self.tiers.iter().enumerate() {
            if position_size <= tier.upper_bound_size {
                if i == 0 {
                    // First tier: no interpolation needed
                    return tier.limit_order_risk_factor;
                }

                // Interpolate between previous tier and current tier
                let prev_tier = &self.tiers[i - 1];
                return interpolate_limit_order_risk_factor(
                    prev_tier.upper_bound_size,
                    prev_tier.limit_order_risk_factor,
                    tier.upper_bound_size,
                    tier.limit_order_risk_factor,
                    position_size,
                );
            }
        }
        // Position exceeds all tiers - use maximum risk factor (100%)
        BasisPoints::UPPER_BOUND.into()
    }

    /// Get a reference to a specific tier
    pub fn get(&self, index: usize) -> Option<&LeverageTier> {
        self.tiers.get(index)
    }

    /// Iterator over all tiers
    pub fn iter(&self) -> impl Iterator<Item = &LeverageTier> {
        self.tiers.iter()
    }

    /// Number of tiers (always 4)
    pub const fn len(&self) -> usize {
        4
    }

    /// Always false (always has 4 tiers)
    pub const fn is_empty(&self) -> bool {
        false
    }
}

impl Default for LeverageTiers {
    fn default() -> Self {
        // Default: 20x leverage across all sizes, no limit order discount
        Self::new_unchecked([
            LeverageTier {
                upper_bound_size: BaseLots::new(1_000_000),
                max_leverage: Constant::new(20),
                limit_order_risk_factor: BasisPoints::new(10_000),
            },
            LeverageTier {
                upper_bound_size: BaseLots::new(10_000_000),
                max_leverage: Constant::new(10),
                limit_order_risk_factor: BasisPoints::new(10_000),
            },
            LeverageTier {
                upper_bound_size: BaseLots::new(100_000_000),
                max_leverage: Constant::new(5),
                limit_order_risk_factor: BasisPoints::new(10_000),
            },
            LeverageTier {
                upper_bound_size: BaseLots::new(u32::MAX as u64),
                max_leverage: Constant::new(1),
                limit_order_risk_factor: BasisPoints::new(10_000),
            },
        ])
    }
}

// ============================================================================
// Internal Interpolation Helpers
// ============================================================================

/// Core interpolation logic working with raw u64 values
///
/// Calculates the "percentage" of the way between x1 and x2 for a given x value
/// and returns the corresponding y value. Does not assume x2 > x1 or y2 > y1.
fn interpolate_u64(x1: u64, y1: u64, x2: u64, y2: u64, x: u64) -> u64 {
    // Handle degenerate cases
    if x1 == x2 || y1 == y2 {
        return y1;
    }

    // Linear interpolation: y = y1 + (y2 - y1) * (x - x1) / (x2 - x1)
    let x_range = x2 as f64 - x1 as f64;
    let y_range = y2 as f64 - y1 as f64;
    let x_offset = x as f64 - x1 as f64;

    // Clamp percentage to [0, 1]
    let percent_of_x_range = (x_offset / x_range).clamp(0.0, 1.0);

    let interpolated_value = (y1 as f64) + percent_of_x_range * y_range;
    interpolated_value as u64
}

fn interpolate_leverage(
    x1: BaseLots,
    y1: Constant,
    x2: BaseLots,
    y2: Constant,
    x: BaseLots,
) -> Constant {
    let result = interpolate_u64(
        x1.as_inner(),
        y1.as_inner(),
        x2.as_inner(),
        y2.as_inner(),
        x.as_inner(),
    );
    Constant::new(result)
}

fn interpolate_limit_order_risk_factor(
    x1: BaseLots,
    y1: BasisPoints,
    x2: BaseLots,
    y2: BasisPoints,
    x: BaseLots,
) -> BasisPoints {
    let result = interpolate_u64(
        x1.as_inner(),
        y1.as_inner(),
        x2.as_inner(),
        y2.as_inner(),
        x.as_inner(),
    );
    BasisPoints::new(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolation_at_boundaries() {
        let tiers = LeverageTiers::default();

        // At first boundary
        let leverage = tiers.get_leverage_constant(BaseLots::new(1_000_000));
        assert_eq!(leverage, Constant::new(20));

        // At second boundary
        let leverage = tiers.get_leverage_constant(BaseLots::new(10_000_000));
        assert_eq!(leverage, Constant::new(10));
    }

    #[test]
    fn test_interpolation_between_tiers() {
        let tiers = LeverageTiers::default();

        // Midpoint between first and second tier should be ~15x
        let mid_point = (1_000_000 + 10_000_000) / 2;
        let leverage = tiers.get_leverage_constant(BaseLots::new(mid_point));
        // Should be between 10 and 20
        assert!(leverage.as_inner() >= 10 && leverage.as_inner() <= 20);
    }

    #[test]
    fn test_exceeds_all_tiers() {
        let tiers = LeverageTiers::default();

        // Beyond all tiers should return 1x
        let leverage = tiers.get_leverage_constant(BaseLots::new(u32::MAX as u64 + 1));
        assert_eq!(leverage, Constant::new(1));
    }

    #[test]
    fn test_validation_increasing_sizes() {
        let invalid_tiers = [
            LeverageTier {
                upper_bound_size: BaseLots::new(10_000),
                max_leverage: Constant::new(20),
                limit_order_risk_factor: BasisPoints::new(5_000),
            },
            LeverageTier {
                upper_bound_size: BaseLots::new(5_000), // Decreasing!
                max_leverage: Constant::new(10),
                limit_order_risk_factor: BasisPoints::new(5_000),
            },
            LeverageTier {
                upper_bound_size: BaseLots::new(20_000),
                max_leverage: Constant::new(5),
                limit_order_risk_factor: BasisPoints::new(7_500),
            },
            LeverageTier {
                upper_bound_size: BaseLots::new(100_000),
                max_leverage: Constant::new(1),
                limit_order_risk_factor: BasisPoints::new(10_000),
            },
        ];

        assert!(LeverageTiers::new(invalid_tiers).is_err());
    }

    #[test]
    fn test_validation_non_increasing_leverage() {
        let invalid_tiers = [
            LeverageTier {
                upper_bound_size: BaseLots::new(1_000),
                max_leverage: Constant::new(10),
                limit_order_risk_factor: BasisPoints::new(5_000),
            },
            LeverageTier {
                upper_bound_size: BaseLots::new(10_000),
                max_leverage: Constant::new(20), // Increasing!
                limit_order_risk_factor: BasisPoints::new(5_000),
            },
            LeverageTier {
                upper_bound_size: BaseLots::new(20_000),
                max_leverage: Constant::new(5),
                limit_order_risk_factor: BasisPoints::new(7_500),
            },
            LeverageTier {
                upper_bound_size: BaseLots::new(100_000),
                max_leverage: Constant::new(1),
                limit_order_risk_factor: BasisPoints::new(10_000),
            },
        ];

        assert!(LeverageTiers::new(invalid_tiers).is_err());
    }
}
