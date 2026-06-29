//! Trader capability flags, state predicates, and access helpers.

use core::fmt::{Debug, Display};

use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{Pod, Zeroable};
#[cfg(feature = "serde")]
use serde::ser::SerializeStruct;

const_assert_eq!(core::mem::size_of::<TraderCapabilityFlags>(), 4);

/// HOT participation bit for a trader in the global trader index.
pub const TRADER_CAPABILITY_HOT: u32 = 1 << 0;
/// Enables resting limit order placement for hot traders.
pub const TRADER_CAPABILITY_CAN_PLACE_LIMIT: u32 = 1 << 1;
/// Enables market order placement and risk-reducing order flows.
pub const TRADER_CAPABILITY_CAN_PLACE_MARKET: u32 = 1 << 2;
/// Enables risk-increasing trades.
pub const TRADER_CAPABILITY_CAN_RISK_INCREASE: u32 = 1 << 3;
/// Enables collateral deposits.
pub const TRADER_CAPABILITY_CAN_DEPOSIT: u32 = 1 << 4;
/// Enables collateral withdrawals.
pub const TRADER_CAPABILITY_CAN_WITHDRAW: u32 = 1 << 5;

/// Capability mask needed before a trader can trade and move collateral.
pub const REQUIRED_TRADER_CAPABILITIES: u32 = TRADER_CAPABILITY_CAN_PLACE_MARKET
    | TRADER_CAPABILITY_CAN_DEPOSIT
    | TRADER_CAPABILITY_CAN_WITHDRAW;

/// Alias for the capabilities most integrators need after onboarding.
pub const TRADER_READY_CAPABILITIES: u32 = REQUIRED_TRADER_CAPABILITIES;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[borsh(use_discriminant = true)]
#[repr(u8)]
pub enum TraderCapabilityKind {
    PlaceLimitOrder     = 0,
    PlaceMarketOrder    = 1,
    RiskIncreasingTrade = 2,
    RiskReducingTrade   = 3,
    DepositCollateral   = 4,
    WithdrawCollateral  = 5,
}

impl TraderCapabilityKind {
    pub const fn key(self) -> &'static str {
        match self {
            Self::PlaceLimitOrder => "place_limit_order",
            Self::PlaceMarketOrder => "place_market_order",
            Self::RiskIncreasingTrade => "risk_increasing_trade",
            Self::RiskReducingTrade => "risk_reducing_trade",
            Self::DepositCollateral => "deposit_collateral",
            Self::WithdrawCollateral => "withdraw_collateral",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Self::PlaceLimitOrder => "Place resting limit orders on the book.",
            Self::PlaceMarketOrder => "Submit market or IOC order flow.",
            Self::RiskIncreasingTrade => "Increase position risk via trade execution.",
            Self::RiskReducingTrade => "Reduce position risk via trade execution or cancels.",
            Self::DepositCollateral => "Deposit collateral into a trader account.",
            Self::WithdrawCollateral => "Withdraw collateral from a trader account.",
        }
    }
}

impl Display for TraderCapabilityKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.key())
    }
}

pub const ALL_TRADER_CAPABILITY_KINDS: [TraderCapabilityKind; 6] = [
    TraderCapabilityKind::PlaceLimitOrder,
    TraderCapabilityKind::PlaceMarketOrder,
    TraderCapabilityKind::RiskIncreasingTrade,
    TraderCapabilityKind::RiskReducingTrade,
    TraderCapabilityKind::DepositCollateral,
    TraderCapabilityKind::WithdrawCollateral,
];

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CapabilityAccess {
    immediate: bool,
    via_cold_activation: bool,
}

impl CapabilityAccess {
    pub const fn new(immediate: bool, via_cold_activation: bool) -> Self {
        Self {
            immediate,
            via_cold_activation,
        }
    }

    pub const fn denied() -> Self {
        Self::new(false, false)
    }

    pub const fn immediate() -> Self {
        Self::new(true, false)
    }

    pub const fn via_cold_activation() -> Self {
        Self::new(false, true)
    }

    pub const fn allows_immediate(self) -> bool {
        self.immediate
    }

    pub const fn allows_via_cold_activation(self) -> bool {
        self.immediate || self.via_cold_activation
    }

    pub const fn requires_cold_activation(self) -> bool {
        self.via_cold_activation && !self.immediate
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TraderCapabilities {
    place_limit_order: CapabilityAccess,
    place_market_order: CapabilityAccess,
    risk_increasing_trade: CapabilityAccess,
    risk_reducing_trade: CapabilityAccess,
    deposit_collateral: CapabilityAccess,
    withdraw_collateral: CapabilityAccess,
}

impl TraderCapabilities {
    pub const fn new(
        place_limit_order: CapabilityAccess,
        place_market_order: CapabilityAccess,
        risk_increasing_trade: CapabilityAccess,
        risk_reducing_trade: CapabilityAccess,
        deposit_collateral: CapabilityAccess,
        withdraw_collateral: CapabilityAccess,
    ) -> Self {
        Self {
            place_limit_order,
            place_market_order,
            risk_increasing_trade,
            risk_reducing_trade,
            deposit_collateral,
            withdraw_collateral,
        }
    }

    pub const fn access(&self, capability: TraderCapabilityKind) -> CapabilityAccess {
        match capability {
            TraderCapabilityKind::PlaceLimitOrder => self.place_limit_order,
            TraderCapabilityKind::PlaceMarketOrder => self.place_market_order,
            TraderCapabilityKind::RiskIncreasingTrade => self.risk_increasing_trade,
            TraderCapabilityKind::RiskReducingTrade => self.risk_reducing_trade,
            TraderCapabilityKind::DepositCollateral => self.deposit_collateral,
            TraderCapabilityKind::WithdrawCollateral => self.withdraw_collateral,
        }
    }

    pub const fn allows(&self, capability: TraderCapabilityKind) -> bool {
        self.access(capability).allows_immediate()
    }

    pub const fn allows_with_cold_activation(&self, capability: TraderCapabilityKind) -> bool {
        self.access(capability).allows_via_cold_activation()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TraderCapabilityFlagsError {
    ReservedBitsSet(u32),
}

impl Display for TraderCapabilityFlagsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ReservedBitsSet(bits) => write!(f, "reserved capability bits set: 0x{bits:08X}"),
        }
    }
}

impl std::error::Error for TraderCapabilityFlagsError {}

#[repr(transparent)]
#[derive(Copy, Clone, Default, PartialEq, Eq, Pod, Zeroable, BorshDeserialize, BorshSerialize)]
pub struct TraderCapabilityFlags {
    flags: u32,
}

impl TraderCapabilityFlags {
    pub const CAN_DEPOSIT: u32 = TRADER_CAPABILITY_CAN_DEPOSIT;
    pub const CAN_PLACE_LIMIT: u32 = TRADER_CAPABILITY_CAN_PLACE_LIMIT;
    pub const CAN_PLACE_MARKET: u32 = TRADER_CAPABILITY_CAN_PLACE_MARKET;
    pub const CAN_RISK_INCREASE: u32 = TRADER_CAPABILITY_CAN_RISK_INCREASE;
    pub const CAN_WITHDRAW: u32 = TRADER_CAPABILITY_CAN_WITHDRAW;
    pub const HOT: u32 = TRADER_CAPABILITY_HOT;
    pub const RESERVED_MASK: u32 = !Self::VALID_MASK;
    pub const VALID_MASK: u32 = Self::HOT
        | Self::CAN_PLACE_LIMIT
        | Self::CAN_PLACE_MARKET
        | Self::CAN_RISK_INCREASE
        | Self::CAN_DEPOSIT
        | Self::CAN_WITHDRAW;

    pub const fn new(flags: u32) -> Self {
        Self { flags }
    }

    pub const fn try_new(flags: u32) -> Result<Self, TraderCapabilityFlagsError> {
        let reserved = flags & Self::RESERVED_MASK;
        if reserved != 0 {
            Err(TraderCapabilityFlagsError::ReservedBitsSet(reserved))
        } else {
            Ok(Self { flags })
        }
    }

    pub const fn as_u32(&self) -> u32 {
        self.flags
    }

    pub const fn bits(self) -> u32 {
        self.flags
    }

    pub const fn reserved_bits(self) -> u32 {
        self.flags & Self::RESERVED_MASK
    }

    pub const fn has_reserved_bits(self) -> bool {
        self.reserved_bits() != 0
    }

    pub const fn contains(self, mask: u32) -> bool {
        (self.flags & mask) == mask
    }

    pub const fn new_uninitialized() -> Self {
        Self::new(0)
    }

    pub const fn cold() -> Self {
        Self::new(
            Self::CAN_PLACE_LIMIT
                | Self::CAN_PLACE_MARKET
                | Self::CAN_RISK_INCREASE
                | Self::CAN_DEPOSIT
                | Self::CAN_WITHDRAW,
        )
    }

    pub const fn hot_active() -> Self {
        Self::new(Self::VALID_MASK)
    }

    const fn reduce_only_mask() -> u32 {
        Self::CAN_PLACE_LIMIT | Self::CAN_PLACE_MARKET | Self::CAN_DEPOSIT | Self::CAN_WITHDRAW
    }

    const fn frozen_mask() -> u32 {
        Self::CAN_PLACE_LIMIT | Self::CAN_PLACE_MARKET
    }

    pub const fn reduce_only() -> Self {
        Self::new(Self::reduce_only_mask())
    }

    pub const fn reduce_only_hot() -> Self {
        Self::new(Self::reduce_only_mask() | Self::HOT)
    }

    pub const fn frozen() -> Self {
        Self::new(Self::frozen_mask())
    }

    pub const fn frozen_hot() -> Self {
        Self::new(Self::frozen_mask() | Self::HOT)
    }

    pub const fn is_uninitialized(self) -> bool {
        self.flags == 0
    }

    pub const fn is_hot(self) -> bool {
        self.contains(Self::HOT)
    }

    pub const fn is_cold(self) -> bool {
        !self.is_uninitialized() && !self.is_hot()
    }

    pub const fn is_ready(self) -> bool {
        is_trader_ready(self.flags)
    }

    pub const fn is_reduce_only(self) -> bool {
        is_trader_reduce_only(self.flags)
    }

    pub const fn is_frozen(self) -> bool {
        is_trader_frozen(self.flags)
    }

    pub const fn allows(self, capability: TraderCapabilityKind) -> bool {
        match capability {
            TraderCapabilityKind::PlaceLimitOrder => {
                self.is_hot() && self.contains(Self::CAN_PLACE_LIMIT)
            }
            TraderCapabilityKind::PlaceMarketOrder => self.contains(Self::CAN_PLACE_MARKET),
            TraderCapabilityKind::RiskIncreasingTrade => {
                self.contains(Self::CAN_PLACE_MARKET) && self.contains(Self::CAN_RISK_INCREASE)
            }
            TraderCapabilityKind::RiskReducingTrade => self.contains(Self::CAN_PLACE_MARKET),
            TraderCapabilityKind::DepositCollateral => self.contains(Self::CAN_DEPOSIT),
            TraderCapabilityKind::WithdrawCollateral => self.contains(Self::CAN_WITHDRAW),
        }
    }

    pub const fn allows_with_cold_activation(self, capability: TraderCapabilityKind) -> bool {
        match capability {
            TraderCapabilityKind::PlaceLimitOrder => {
                self.allows(capability) || (!self.is_hot() && self.contains(Self::CAN_PLACE_LIMIT))
            }
            _ => self.allows(capability),
        }
    }

    pub const fn capabilities(self) -> TraderCapabilities {
        let limit_immediate = self.allows(TraderCapabilityKind::PlaceLimitOrder);
        let limit_cold_activation = self
            .allows_with_cold_activation(TraderCapabilityKind::PlaceLimitOrder)
            && !limit_immediate;
        TraderCapabilities::new(
            CapabilityAccess::new(limit_immediate, limit_cold_activation),
            CapabilityAccess::new(self.allows(TraderCapabilityKind::PlaceMarketOrder), false),
            CapabilityAccess::new(
                self.allows(TraderCapabilityKind::RiskIncreasingTrade),
                false,
            ),
            CapabilityAccess::new(self.allows(TraderCapabilityKind::RiskReducingTrade), false),
            CapabilityAccess::new(self.allows(TraderCapabilityKind::DepositCollateral), false),
            CapabilityAccess::new(self.allows(TraderCapabilityKind::WithdrawCollateral), false),
        )
    }

    pub fn mark_hot(&mut self) {
        self.flags = (self.flags | Self::HOT) & Self::VALID_MASK;
    }

    pub fn unmark_hot(&mut self) {
        self.flags = (self.flags & !Self::HOT) & Self::VALID_MASK;
    }

    pub fn enable_capability(&mut self, capability: TraderCapabilityKind) {
        self.flags |= match capability {
            TraderCapabilityKind::PlaceLimitOrder => Self::CAN_PLACE_LIMIT,
            TraderCapabilityKind::PlaceMarketOrder | TraderCapabilityKind::RiskReducingTrade => {
                Self::CAN_PLACE_MARKET
            }
            TraderCapabilityKind::RiskIncreasingTrade => Self::CAN_RISK_INCREASE,
            TraderCapabilityKind::DepositCollateral => Self::CAN_DEPOSIT,
            TraderCapabilityKind::WithdrawCollateral => Self::CAN_WITHDRAW,
        };
        self.flags &= Self::VALID_MASK;
    }

    pub fn disable_capability(&mut self, capability: TraderCapabilityKind) {
        match capability {
            TraderCapabilityKind::PlaceLimitOrder => self.flags &= !Self::CAN_PLACE_LIMIT,
            TraderCapabilityKind::PlaceMarketOrder => self.flags &= !Self::CAN_PLACE_MARKET,
            TraderCapabilityKind::RiskReducingTrade => {
                self.flags &= !Self::CAN_PLACE_MARKET;
                self.flags &= !Self::CAN_PLACE_LIMIT;
            }
            TraderCapabilityKind::RiskIncreasingTrade => self.flags &= !Self::CAN_RISK_INCREASE,
            TraderCapabilityKind::DepositCollateral => self.flags &= !Self::CAN_DEPOSIT,
            TraderCapabilityKind::WithdrawCollateral => self.flags &= !Self::CAN_WITHDRAW,
        }
        self.flags &= Self::VALID_MASK;
    }

    pub fn mark_reduce_only(&mut self) {
        self.enable_capability(TraderCapabilityKind::PlaceLimitOrder);
        self.enable_capability(TraderCapabilityKind::PlaceMarketOrder);
        self.enable_capability(TraderCapabilityKind::DepositCollateral);
        self.enable_capability(TraderCapabilityKind::WithdrawCollateral);
        self.disable_capability(TraderCapabilityKind::RiskIncreasingTrade);
    }

    pub fn freeze(&mut self) {
        self.mark_reduce_only();
        self.disable_capability(TraderCapabilityKind::DepositCollateral);
        self.disable_capability(TraderCapabilityKind::WithdrawCollateral);
    }

    pub fn enable_all_capabilities(&mut self) {
        self.enable_capability(TraderCapabilityKind::PlaceLimitOrder);
        self.enable_capability(TraderCapabilityKind::PlaceMarketOrder);
        self.enable_capability(TraderCapabilityKind::RiskIncreasingTrade);
        self.enable_capability(TraderCapabilityKind::DepositCollateral);
        self.enable_capability(TraderCapabilityKind::WithdrawCollateral);
    }
}

impl Debug for TraderCapabilityFlags {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let state = if self.is_uninitialized() {
            "uninitialized"
        } else if self.is_frozen() {
            if self.is_hot() {
                "frozen_hot"
            } else {
                "frozen_cold"
            }
        } else if self.is_reduce_only() {
            if self.is_hot() {
                "reduce_only_hot"
            } else {
                "reduce_only_cold"
            }
        } else if self.is_hot() {
            "hot_active"
        } else if self.is_cold() {
            "cold"
        } else {
            "custom"
        };

        f.debug_struct("TraderCapabilityFlags")
            .field("bits", &format_args!("0x{:08X}", self.flags))
            .field("state", &state)
            .field("capabilities", &format_args!("{self}"))
            .finish()
    }
}

impl Display for TraderCapabilityFlags {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        const CAPABILITIES: [(&str, u32); 6] = [
            ("HOT", TraderCapabilityFlags::HOT),
            ("CAN_PLACE_LIMIT", TraderCapabilityFlags::CAN_PLACE_LIMIT),
            ("CAN_PLACE_MARKET", TraderCapabilityFlags::CAN_PLACE_MARKET),
            (
                "CAN_RISK_INCREASE",
                TraderCapabilityFlags::CAN_RISK_INCREASE,
            ),
            ("CAN_DEPOSIT", TraderCapabilityFlags::CAN_DEPOSIT),
            ("CAN_WITHDRAW", TraderCapabilityFlags::CAN_WITHDRAW),
        ];

        let mut wrote_any = false;
        for (name, mask) in CAPABILITIES {
            if self.contains(mask) {
                if wrote_any {
                    write!(f, " | ")?;
                }
                write!(f, "{name}")?;
                wrote_any = true;
            }
        }

        let unknown = self.reserved_bits();
        if unknown != 0 {
            if wrote_any {
                write!(f, " | ")?;
            }
            write!(f, "UNKNOWN(0x{unknown:08X})")?;
            wrote_any = true;
        }

        if !wrote_any {
            write!(f, "NONE")?;
        }

        Ok(())
    }
}

impl From<TraderCapabilityFlags> for u32 {
    fn from(flags: TraderCapabilityFlags) -> Self {
        flags.bits()
    }
}

impl TryFrom<u32> for TraderCapabilityFlags {
    type Error = TraderCapabilityFlagsError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

/// Returns true when the trader has the capabilities required for market
/// orders, deposits, and withdrawals.
#[inline(always)]
pub const fn is_trader_ready(flags: u32) -> bool {
    flags & TRADER_READY_CAPABILITIES == TRADER_READY_CAPABILITIES
}

/// Returns true when the trader participates in the hot trader index.
#[inline(always)]
pub const fn is_trader_hot(flags: u32) -> bool {
    flags & TRADER_CAPABILITY_HOT != 0
}

/// Returns true when the trader is initialized but not hot.
#[inline(always)]
pub const fn is_trader_cold(flags: u32) -> bool {
    flags != 0 && !is_trader_hot(flags)
}

/// Returns true when the trader can only reduce risk but can still move
/// collateral.
#[inline(always)]
pub const fn is_trader_reduce_only(flags: u32) -> bool {
    flags & TRADER_CAPABILITY_CAN_PLACE_MARKET != 0
        && flags & TRADER_CAPABILITY_CAN_RISK_INCREASE == 0
        && flags & TRADER_CAPABILITY_CAN_WITHDRAW != 0
        && flags & TRADER_CAPABILITY_CAN_DEPOSIT != 0
}

/// Returns true when the trader is quarantined from risk increases and
/// collateral movement.
#[inline(always)]
pub const fn is_trader_frozen(flags: u32) -> bool {
    flags & TRADER_CAPABILITY_CAN_PLACE_MARKET != 0
        && flags & TRADER_CAPABILITY_CAN_RISK_INCREASE == 0
        && flags & TRADER_CAPABILITY_CAN_WITHDRAW == 0
        && flags & TRADER_CAPABILITY_CAN_DEPOSIT == 0
}

#[cfg(feature = "serde")]
fn trader_capability_state(flags: TraderCapabilityFlags) -> &'static str {
    if flags.is_uninitialized() {
        "uninitialized"
    } else if flags.is_frozen() {
        if flags.is_hot() {
            "frozen_hot"
        } else {
            "frozen_cold"
        }
    } else if flags.is_reduce_only() {
        if flags.is_hot() {
            "reduce_only_hot"
        } else {
            "reduce_only_cold"
        }
    } else if flags.is_hot() {
        "hot_active"
    } else if flags.is_cold() {
        "cold"
    } else {
        "custom"
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for TraderCapabilityFlags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("TraderCapabilityFlags", 10)?;
        state.serialize_field("bits", &self.bits())?;
        state.serialize_field("reserved_bits", &self.reserved_bits())?;
        state.serialize_field("has_reserved_bits", &self.has_reserved_bits())?;
        state.serialize_field("state", &trader_capability_state(*self))?;
        state.serialize_field("is_uninitialized", &self.is_uninitialized())?;
        state.serialize_field("is_hot", &self.is_hot())?;
        state.serialize_field("is_cold", &self.is_cold())?;
        state.serialize_field("is_ready", &self.is_ready())?;
        state.serialize_field("is_reduce_only", &self.is_reduce_only())?;
        state.serialize_field("is_frozen", &self.is_frozen())?;
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trader_ready_helper_requires_market_deposit_and_withdraw() {
        assert!(is_trader_ready(REQUIRED_TRADER_CAPABILITIES));
        assert!(!is_trader_ready(
            REQUIRED_TRADER_CAPABILITIES & !TRADER_CAPABILITY_CAN_WITHDRAW
        ));
    }

    #[test]
    fn capability_flags_model_cold_hot_and_restricted_states() {
        let cold = TraderCapabilityFlags::cold();
        assert!(cold.is_cold());
        assert!(!cold.allows(TraderCapabilityKind::PlaceLimitOrder));
        assert!(
            cold.capabilities()
                .access(TraderCapabilityKind::PlaceLimitOrder)
                .requires_cold_activation()
        );
        assert!(cold.allows(TraderCapabilityKind::RiskIncreasingTrade));

        let hot = TraderCapabilityFlags::hot_active();
        assert!(hot.is_hot());
        assert!(hot.allows(TraderCapabilityKind::PlaceLimitOrder));
        assert!(hot.allows(TraderCapabilityKind::RiskIncreasingTrade));
        assert!(hot.allows(TraderCapabilityKind::WithdrawCollateral));

        let mut reduce_only = TraderCapabilityFlags::reduce_only();
        assert!(reduce_only.is_reduce_only());
        assert!(reduce_only.allows(TraderCapabilityKind::RiskReducingTrade));
        assert!(!reduce_only.allows(TraderCapabilityKind::RiskIncreasingTrade));
        assert!(!reduce_only.allows(TraderCapabilityKind::PlaceLimitOrder));
        reduce_only.mark_hot();
        assert!(reduce_only.allows(TraderCapabilityKind::PlaceLimitOrder));

        let frozen = TraderCapabilityFlags::frozen_hot();
        assert!(frozen.is_frozen());
        assert!(frozen.allows(TraderCapabilityKind::RiskReducingTrade));
        assert!(!frozen.allows(TraderCapabilityKind::RiskIncreasingTrade));
        assert!(!frozen.allows(TraderCapabilityKind::DepositCollateral));
        assert!(!frozen.allows(TraderCapabilityKind::WithdrawCollateral));
    }

    #[test]
    fn capability_flags_strict_conversion_rejects_reserved_bits() {
        let reserved_bit = 1 << 9;
        assert_eq!(
            TraderCapabilityFlags::try_new(reserved_bit),
            Err(TraderCapabilityFlagsError::ReservedBitsSet(reserved_bit))
        );

        let permissive = TraderCapabilityFlags::new(reserved_bit);
        assert!(permissive.has_reserved_bits());
    }
}
