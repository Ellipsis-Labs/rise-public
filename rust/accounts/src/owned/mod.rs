//! Owned Phoenix on-chain account deserialization.
//!
//! These account readers allocate owned Rust collections where needed. For
//! zero-copy account header views intended for on-chain integrators, use the
//! sibling modules at the crate root.

mod conditional_order_collection;
mod discriminants;
mod error;
mod global_configuration;
mod internal;
mod mint;
mod orderbook;
mod orderbook_header;
mod permission;
mod perp_asset_map;
mod spline_collection;
mod stop_losses;
mod token_account;
mod trader;
mod withdraw_queue_header;

pub use conditional_order_collection::{
    ConditionalOrder, ConditionalOrderCollection, ConditionalOrderHeader, ConditionalOrderTrigger,
};
#[allow(deprecated)]
pub use discriminants::{ACCOUNT_DISCRIMINANTS, AccountDiscriminants};
pub use error::AccountDeserializeError;
pub use global_configuration::GlobalConfiguration;
pub use internal::{
    AssetFlags, AuthoritySet, BookPriceComponent, FifoOrderId, FundingAccumulator, LeverageTier,
    MarkPrice, OpenInterestParams, OracleData, OracleParameters, OrderFill, OrderbookEntry,
    OrderbookRestingOrder, PerpAssetEntry, PerpAssetMetadata, PerpPriceComponent, PriceComponent,
    RiskParams, SequenceNumber, ShortEntries, Spline, SplineLiquidity, SplineLiquidityRegion,
    SplineRiskCapacity, SplineSide, SpotPriceComponent, StaticMarketParams, TickRegion,
    TicksAtSlot, TradeEvent, TraderPosition, TraderPositionEntry, TraderPositionId, TraderState,
    TransferFeeTier,
};
pub use mint::Mint;
pub use orderbook::Orderbook;
pub use orderbook_header::OrderbookHeader;
pub use permission::Permission;
#[allow(deprecated)]
pub use perp_asset_map::PerpAssetMap;
pub use perp_asset_map::PerpAssetMapOwned;
pub use spline_collection::SplineCollection;
pub use stop_losses::{
    StopLoss, StopLossDirection, StopLossOrderKind, StopLossTradeSide, StopLosses,
};
pub use token_account::{TokenAccount, TokenAccountState};
pub use trader::Trader;
pub use withdraw_queue_header::{WithdrawQueueHeader, WithdrawThrottle};

/// Deserializes an owned account view from Solana account data.
pub trait AccountDeserialize: Sized {
    fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError>;
}
