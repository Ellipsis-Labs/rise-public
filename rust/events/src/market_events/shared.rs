use core::fmt;

use borsh::{BorshDeserialize, BorshSerialize};
pub use phoenix_rise_math::{
    BaseLots, BasisPoints, Constant, FundingRateUnitInSeconds, QuoteLots, SignedBaseLots,
    SignedFeeRateMicro, SignedQuoteLots, SignedQuoteLotsPerBaseLot,
    SignedQuoteLotsPerBaseLotUpcasted, SignedTicks, Ticks,
};

pub type Pubkey = [u8; 32];
pub type Slot = u64;

#[cfg(feature = "serde")]
fn zero_padding_128() -> [u8; 128] {
    [0; 128]
}

#[repr(transparent)]
#[derive(Copy, Clone, Default, BorshDeserialize, BorshSerialize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Symbol {
    symbol_bytes: [u8; 16],
}

impl Symbol {
    pub fn new(symbol: &str) -> Option<Self> {
        if symbol.len() > 16 {
            return None;
        }
        let mut symbol_bytes = [0u8; 16];
        symbol_bytes[..symbol.len()].copy_from_slice(symbol.as_bytes());
        let symbol = Self { symbol_bytes };
        symbol.is_valid().then_some(symbol)
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > 16 {
            return None;
        }
        let mut symbol_bytes = [0u8; 16];
        symbol_bytes[..bytes.len()].copy_from_slice(bytes);
        let symbol = Self { symbol_bytes };
        symbol.is_valid().then_some(symbol)
    }

    pub const fn to_bytes(&self) -> [u8; 16] {
        self.symbol_bytes
    }

    pub fn is_valid(&self) -> bool {
        let mut seen_zero = false;
        for &byte in &self.symbol_bytes {
            if byte == 0 {
                seen_zero = true;
            } else if seen_zero {
                return false;
            }
        }
        true
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let end = self
            .symbol_bytes
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(self.symbol_bytes.len());
        write!(f, "{}", String::from_utf8_lossy(&self.symbol_bytes[..end]))
    }
}

impl fmt::Debug for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MarketStatus {
    #[default]
    Uninitialized,
    Active,
    PostOnly,
    Paused,
    Closed,
    Tombstoned,
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CommodityMarketState {
    #[default]
    Active,
    AfterHours,
    Reopen,
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FeatureSet(u32);

impl FeatureSet {
    pub const fn new(bits: u32) -> Self {
        Self(bits)
    }

    pub const fn bits(&self) -> u32 {
        self.0
    }
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TraderCapabilityFlags(u32);

impl TraderCapabilityFlags {
    pub const fn new(bits: u32) -> Self {
        Self(bits)
    }

    pub const fn bits(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AuthorityType {
    Root,
    Risk,
    Market,
    Oracle,
    Cancel,
    Backstop,
    ADL,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Side {
    Bid,
    Ask,
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Side::Bid => write!(f, "bid"),
            Side::Ask => write!(f, "ask"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SelfTradeBehavior {
    Abort,
    CancelProvide,
    DecrementTake,
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OrderFlags {
    flags: u8,
}

impl OrderFlags {
    pub const fn from_bits(flags: u8) -> Self {
        Self { flags }
    }

    pub const fn as_u8(&self) -> u8 {
        self.flags
    }

    pub const fn is_reduce_only(&self) -> bool {
        self.flags & (1 << 7) != 0
    }

    pub const fn is_stop_loss(&self) -> bool {
        self.flags & (1 << 6) != 0
    }

    pub const fn is_conditional_order(&self) -> bool {
        self.flags & (1 << 5) != 0
    }

    pub const fn is_stop_loss_direction(&self) -> bool {
        self.flags & (1 << 4) != 0
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OrderPacket {
    pub kind: OrderPacketKind,
}

impl OrderPacket {
    pub const fn new(kind: OrderPacketKind) -> Self {
        Self { kind }
    }

    pub const fn kind(&self) -> &OrderPacketKind {
        &self.kind
    }

    pub const fn to_str(&self) -> &'static str {
        match &self.kind {
            OrderPacketKind::PostOnly { .. } => "PostOnly",
            OrderPacketKind::Limit { .. } => "Limit",
            OrderPacketKind::ImmediateOrCancel { .. } => "ImmediateOrCancel",
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OrderPacketKind {
    PostOnly {
        side: Side,
        price_in_ticks: Ticks,
        num_base_lots: BaseLots,
        client_order_id: [u8; 16],
        slide: bool,
        last_valid_slot: Option<u64>,
        order_flags: OrderFlags,
        cancel_existing: bool,
    },
    Limit {
        side: Side,
        price_in_ticks: Ticks,
        num_base_lots: BaseLots,
        self_trade_behavior: SelfTradeBehavior,
        match_limit: Option<u64>,
        client_order_id: [u8; 16],
        last_valid_slot: Option<u64>,
        order_flags: OrderFlags,
        cancel_existing: bool,
    },
    ImmediateOrCancel {
        side: Side,
        price_in_ticks: Option<Ticks>,
        num_base_lots: BaseLots,
        num_quote_lots: Option<QuoteLots>,
        min_base_lots_to_fill: BaseLots,
        min_quote_lots_to_fill: QuoteLots,
        self_trade_behavior: SelfTradeBehavior,
        match_limit: Option<u64>,
        client_order_id: [u8; 16],
        last_valid_slot: Option<u64>,
        order_flags: OrderFlags,
        cancel_existing: bool,
    },
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OptionalNonZeroU32(u32);

impl OptionalNonZeroU32 {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn null() -> Self {
        Self(0)
    }

    pub const fn get(&self) -> Option<u32> {
        if self.0 == 0 { None } else { Some(self.0) }
    }
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OptionalNonZeroU64(u64);

impl OptionalNonZeroU64 {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn null() -> Self {
        Self(0)
    }

    pub const fn get(&self) -> Option<u64> {
        if self.0 == 0 { None } else { Some(self.0) }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NodePointer(pub OptionalNonZeroU32);

impl NodePointer {
    pub const fn null() -> Self {
        Self(OptionalNonZeroU32::null())
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OptionalFIFOOrderId {
    pub price_in_ticks: Ticks,
    pub order_sequence_number: u64,
}

impl OptionalFIFOOrderId {
    pub const fn none() -> Self {
        Self {
            price_in_ticks: Ticks::ZERO,
            order_sequence_number: 0,
        }
    }

    pub fn is_none(&self) -> bool {
        self.price_in_ticks == Ticks::ZERO && self.order_sequence_number == 0
    }
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BaseLotsU32(pub u32);

#[repr(transparent)]
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BaseLotsPerTickU32(pub u32);

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TickRegion {
    pub start_offset: Ticks,
    pub end_offset: Ticks,
    pub density: BaseLotsPerTickU32,
    pub top_level_hidden_take_size: BaseLotsPerTickU32,
    pub total_size: BaseLots,
    pub filled_size: BaseLotsU32,
    pub hidden_filled_size: BaseLotsU32,
    pub lifespan: Slot,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ValidationRule {
    #[default]
    Ignore,
    Require,
    Forbid,
}

pub type RiskActionPriceValidityRules = [[[ValidationRule; 8]; 4]; 8];

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LeverageTier {
    pub upper_bound_size: BaseLots,
    pub max_leverage: Constant,
    pub limit_order_risk_factor: BasisPoints,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LeverageTiers {
    pub tiers: [LeverageTier; 4],
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Direction {
    GreaterThan,
    LessThan,
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Direction::GreaterThan => write!(f, "gt"),
            Direction::LessThan => write!(f, "lt"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StopLossOrderKind {
    IOC,
    Limit,
}

impl fmt::Display for StopLossOrderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StopLossOrderKind::IOC => write!(f, "ioc"),
            StopLossOrderKind::Limit => write!(f, "limit"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EscrowAction {
    Noop {
        #[cfg_attr(feature = "serde", serde(skip))]
        _padding0: [u8; 8],
        #[cfg_attr(feature = "serde", serde(skip, default = "zero_padding_128"))]
        _padding1: [u8; 128],
    },
    Cash {
        amount: u64,
        #[cfg_attr(feature = "serde", serde(skip, default = "zero_padding_128"))]
        _padding0: [u8; 128],
    },
}
