//! Read-only Phoenix on-chain account deserialization.
//!
//! The public entry point for each account is `try_from_account_bytes`. These
//! account layouts are fixed byte layouts, so the implementations use checked
//! layout readers instead of a TypeScript-style codec layer.

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
pub use discriminants::ACCOUNT_DISCRIMINANTS;
pub use error::AccountDeserializeError;
pub use global_configuration::GlobalConfiguration;
pub use internal::{
    AssetFlags, AuthoritySet, BookPriceComponent, FifoOrderId, FundingAccumulator, LeverageTier,
    MarkPrice, OpenInterestParams, OracleData, OracleParameters, OrderFill, OrderbookEntry,
    OrderbookRestingOrder, PerpAssetEntry, PerpAssetMetadata, PerpPriceComponent, PriceComponent,
    RiskParams, SequenceNumber, ShortEntries, Spline, SpotPriceComponent, StaticMarketParams,
    TickRegion, TicksAtSlot, TradeEvent, TraderPosition, TraderPositionEntry, TraderPositionId,
    TraderState, TransferFeeTier,
};
pub use mint::Mint;
pub use orderbook::Orderbook;
pub use orderbook_header::OrderbookHeader;
pub use permission::Permission;
pub use perp_asset_map::PerpAssetMap;
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
