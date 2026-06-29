use core::fmt;

use borsh::{BorshDeserialize, BorshSerialize};

use super::*;

////////////////////////////////////////////////////////////////////////////////////////////////
// Admin events
////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MarketAddedEvent {}

#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MarketStatusChangedEvent {
    pub previous_market_status: MarketStatus,
    pub new_market_status: MarketStatus,
}

/// Event emitted when a market is closed with its finalized settlement price.
#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MarketClosedEvent {
    pub previous_market_status: MarketStatus,
    pub finalized_mark_price: Ticks,
}

/// Event emitted when a market is tombstoned, carrying the final sequence
/// numbers.
#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MarketTombstonedEvent {
    pub previous_market_status: MarketStatus,
    pub final_sequence_number: u64,
    pub final_trade_sequence_number: u64,
    pub final_order_sequence_number: u64,
}

/// Event emitted when positions are settled during market shutdown
/// (settle_closed_market_position instruction).
#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShutdownClosePositionsEvent {
    pub long_trader: Pubkey,
    pub short_trader: Pubkey,
    pub asset_id: u32,
    pub base_lots_closed: BaseLots,
    pub settlement_price: Ticks,
    pub trade_sequence_number: u64,
    pub sequence_number: u64,
}

/// Event emitted when a tombstoned market is permanently deleted.
#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MarketDeletedEvent {
    pub asset_id: u32,
    pub lamports_reclaimed: u64,
}

#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExchangeStatusChangedEvent {
    pub previous_bits: u8,
    pub new_bits: u8,
    pub authority: Pubkey,
}

#[deprecated(note = "This event is no longer emitted and should not be used by indexers")]
#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MarketParametersUpdatedEvent {}

#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FundingParametersUpdatedEvent {
    pub symbol: Symbol,
    pub new_funding_interval_seconds: Option<FundingRateUnitInSeconds>,
    pub new_funding_period_seconds: Option<FundingRateUnitInSeconds>,
    pub new_max_funding_rate: Option<SignedQuoteLotsPerBaseLot>,
}

#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FeesClaimedEvent {
    pub amount: u64,
}

#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SetPermissionEvent {
    pub authority: Pubkey,
    pub user: Pubkey,
    pub previous_permission: u64,
    pub new_permission: u64,
    pub previous_expires_at_timestamp: i64,
    pub new_expires_at_timestamp: i64,
    pub previous_num_signer_actions_remaining: i64,
    pub new_num_signer_actions_remaining: i64,
    pub created: bool,
}

////////////////////////////////////////////////////////////////////////////////////////////////
// Admin parameter update events
////////////////////////////////////////////////////////////////////////////////////////////////

/// Lightweight struct capturing configurable mark price parameters.
/// Used for prev/new comparison in admin parameter update events.
#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MarkPriceConfig {
    pub ema_period_slots: u64,
    pub ema_diff_radius: u64,
    pub book_price_radius: u64,
    pub commodities_after_hours_radius: u64,
    pub spot_price_weight: u64,
    pub book_price_weight: u64,
    pub perp_price_weight: u64,
    pub spot_price_stale_threshold: u64,
    pub book_price_stale_threshold: u64,
    pub perp_price_stale_threshold: u64,
    pub risk_action_price_validity_rules: RiskActionPriceValidityRules,
    pub oracle_divergence_radius: u16,
    pub min_oracle_responses: u8,
    pub commodities_after_hours_radius_bps: u64,
    pub book_hard_stale_multiplier: u8,
    pub oracle_hard_stale_multiplier: u8,
}

/// Lightweight struct capturing withdraw queue parameters.
#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WithdrawConfig {
    pub deposit_cooldown_period_in_slots: u64,
    pub withdrawal_fee: u64,
    pub enqueueing_fee: u64,
}

/// Lightweight struct capturing withdraw rate limit parameters.
#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WithdrawRateLimitConfig {
    pub max_budget: u64,
    pub replenish_amount_per_slot: u64,
}

/// Lightweight struct capturing funding parameters.
#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FundingConfig {
    pub funding_interval_seconds: u64,
    pub funding_period_seconds: u64,
    pub max_funding_rate: i64,
}

/// Lightweight struct capturing commodity market-state metadata.
#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CommodityMarketStateConfig {
    pub market_state: CommodityMarketState,
    pub is_commodity: bool,
    pub last_known_index_price: Option<Ticks>,
    pub last_index_expiry_timestamp: u64,
    pub commodities_after_hours_radius: Ticks,
}

/// Lightweight struct capturing market fee parameters.
#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MarketFeeConfig {
    pub default_taker_fee_micro: u32,
    pub default_maker_fee_micro: i32,
}

/// The type of admin parameter update that occurred.
/// Used to emit events when risk/market/root authority parameters are changed.
#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AdminParameterUpdateKind {
    // Perp risk parameters (risk authority)
    CancelRiskFactor {
        previous: u16,
        new: u16,
    },
    IsolatedOnly {
        previous: bool,
        new: bool,
    },
    LeverageTiers {
        previous: LeverageTiers,
        new: LeverageTiers,
    },
    MarkPriceParameters {
        previous: MarkPriceConfig,
        new: MarkPriceConfig,
    },
    OpenInterestCap {
        previous: u64,
        new: u64,
    },
    UpnlRiskFactor {
        previous: u16,
        new: u16,
    },
    UpnlRiskFactorForWithdrawals {
        previous: u16,
        new: u16,
    },

    // Global risk parameters (risk authority)
    WithdrawParameters {
        previous: WithdrawConfig,
        new: WithdrawConfig,
    },
    WithdrawRateLimits {
        previous: WithdrawRateLimitConfig,
        new: WithdrawRateLimitConfig,
    },

    // Trader parameters (risk authority)
    TraderCapability {
        trader: Pubkey,
        previous_flags: TraderCapabilityFlags,
        new_flags: TraderCapabilityFlags,
    },

    // Market authority parameters
    FundingParameters {
        previous: FundingConfig,
        new: FundingConfig,
    },
    MarketFees {
        previous: MarketFeeConfig,
        new: MarketFeeConfig,
    },

    // Root authority parameters
    OpenInterestAdjustment {
        previous_open_interest: u64,
        new_open_interest: u64,
    },

    // Added later
    CommodityMarketState {
        previous: CommodityMarketStateConfig,
        new: CommodityMarketStateConfig,
    },
    MaxLiquidationSize {
        previous: u64,
        new: u64,
    },
    FeatureSet {
        previous: FeatureSet,
        new: FeatureSet,
    },
}

impl fmt::Display for AdminParameterUpdateKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CancelRiskFactor { previous, new } => {
                write!(f, "CancelRiskFactor({previous} -> {new})")
            }
            Self::IsolatedOnly { previous, new } => {
                write!(f, "IsolatedOnly({previous} -> {new})")
            }
            Self::LeverageTiers { previous, new } => {
                write!(f, "LeverageTiers({previous:?} -> {new:?})")
            }
            Self::MarkPriceParameters { previous, new } => {
                write!(f, "MarkPriceParameters({previous:?} -> {new:?})")
            }
            Self::OpenInterestCap { previous, new } => {
                write!(f, "OpenInterestCap({previous} -> {new})")
            }
            Self::UpnlRiskFactor { previous, new } => {
                write!(f, "UpnlRiskFactor({previous} -> {new})")
            }
            Self::UpnlRiskFactorForWithdrawals { previous, new } => {
                write!(f, "UpnlRiskFactorForWithdrawals({previous} -> {new})")
            }
            Self::WithdrawParameters { previous, new } => {
                write!(f, "WithdrawParameters({previous:?} -> {new:?})")
            }
            Self::WithdrawRateLimits { previous, new } => {
                write!(f, "WithdrawRateLimits({previous:?} -> {new:?})")
            }
            Self::TraderCapability {
                trader,
                previous_flags,
                new_flags,
            } => {
                write!(
                    f,
                    "TraderCapability(trader={trader:?}, {previous_flags:?} -> {new_flags:?})"
                )
            }
            Self::FundingParameters { previous, new } => {
                write!(f, "FundingParameters({previous:?} -> {new:?})")
            }
            Self::MarketFees { previous, new } => {
                write!(f, "MarketFees({previous:?} -> {new:?})")
            }
            Self::OpenInterestAdjustment {
                previous_open_interest,
                new_open_interest,
            } => {
                write!(
                    f,
                    "OpenInterestAdjustment({previous_open_interest} -> {new_open_interest})"
                )
            }
            Self::CommodityMarketState { previous, new } => {
                write!(f, "CommodityMarketState({previous:?} -> {new:?})")
            }
            Self::MaxLiquidationSize { previous, new } => {
                write!(f, "MaxLiquidationSize({previous} -> {new})")
            }
            Self::FeatureSet { previous, new } => {
                write!(
                    f,
                    "FeatureSet(0b{:08b} -> 0b{:08b})",
                    previous.bits(),
                    new.bits()
                )
            }
        }
    }
}

/// Event emitted when admin parameters are updated
#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AdminParameterUpdatedEvent {
    /// The authority that made the change
    pub authority: Pubkey,
    /// Symbol of the perp asset (if applicable)
    pub asset_symbol: Option<Symbol>,
    /// ID of the perp asset (if applicable)
    pub asset_id: Option<u32>,
    /// The type of parameter update
    pub update_kind: AdminParameterUpdateKind,
}
