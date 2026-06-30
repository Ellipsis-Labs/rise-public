//! Compatibility re-exports for account views used by instruction builders.

pub use phoenix_rise_accounts::common::{PhoenixAccountDecodeError, SequenceNumber};
pub use phoenix_rise_accounts::conditional_orders::{
    ConditionalOrder, ConditionalOrderFlags, ConditionalOrderHeader, ConditionalOrderIter,
    ConditionalOrders, OptionalFifoOrderId, TriggerOrder, TriggerOrderFlags,
};
pub use phoenix_rise_accounts::discriminants::PhoenixAccount;
pub use phoenix_rise_accounts::escrow::{
    Escrow, EscrowAction, EscrowActionKind, EscrowHeader, EscrowRequest, EscrowRequestIter,
};
pub use phoenix_rise_accounts::global_config::GlobalConfig;
pub use phoenix_rise_accounts::multi_arena::{
    MultiArenaHeader, MultiArenaHeaderView, SuperblockView,
};
pub use phoenix_rise_accounts::permission::{PermissionAccount, TRADER_ONBOARDING_PERMISSION};
pub use phoenix_rise_accounts::perp_asset_map::{
    AssetFlags, AssetSymbol, BookPriceComponent, FundingAccumulator, LeverageTier, MarkPrice,
    OpenInterestParams, OracleData, OracleParameters, PerpAssetMap, PerpAssetMapIter,
    PerpAssetMetadata, PerpAssetMetadataEntry, PerpPriceComponent, PriceComponent, RiskParams,
    SpotPriceComponent, StaticMarketParams, TicksAtSlot, TransferFeeTier,
};
pub use phoenix_rise_accounts::stop_losses::{
    StopLoss, StopLossTriggerOrderKind, StopLosses, TradeSide, TriggerDirection,
};
pub use phoenix_rise_accounts::trader;
pub use phoenix_rise_accounts::trader::{
    Trader, TraderHeader, TraderPosition, TraderPositionEntry, TraderPositionIter, TraderPositions,
    TraderState,
};
pub use phoenix_rise_accounts::withdraw_queue::{
    WITHDRAW_QUEUE_MAX_SIZE, WithdrawQueue, WithdrawQueueEntry, WithdrawQueueHeader,
    WithdrawQueueIter, WithdrawQueueNode, WithdrawQueueNodeIter, WithdrawRequest,
    WithdrawRequestState, WithdrawThrottle, WithdrawTransitionReason,
};
