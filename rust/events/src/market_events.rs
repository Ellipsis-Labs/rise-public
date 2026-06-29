#![allow(deprecated)]

use core::fmt;

use borsh::{BorshDeserialize, BorshSerialize};

mod shared;
pub use shared::*;

mod admin;
mod authority;
mod conditional;
mod escrow;
mod liquidation;
mod oracle;
mod orderbook;
mod pnl;
mod spline;
mod trader;
mod withdrawal;

pub use admin::*;
pub use authority::*;
pub use conditional::*;
pub use escrow::*;
pub use liquidation::*;
pub use oracle::*;
pub use orderbook::*;
pub use pnl::*;
pub use spline::*;
pub use trader::*;
pub use withdrawal::*;

#[derive(Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OffChainMarketEvent {
    pub batch_index: u32,
    pub events: Vec<MarketEvent>,
}

/// Payload of LogEventLengths instruction (after 8-byte discriminant).
/// Layout: batch_index (u32) + lengths (Vec<u16> in Borsh = length u32 +
/// elements).
#[derive(Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OffChainMarketEventLengths {
    pub batch_index: u32,
    pub lengths: Vec<u16>,
}

/// CAUTION: new events must be added to THE VERY END of the enum, to maintain
/// the backward compatibility of the variant discriminator.
#[allow(clippy::large_enum_variant)]
#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MarketEvent {
    // Header event
    SlotContext(SlotContextEvent),

    // Orderbook / Trade events
    Header(MarketEventHeader),
    OrderPlaced(OrderPlacedEvent),
    OrderFilled(OrderFilledEvent),
    OrderRejected(OrderRejectedEvent),
    SplineFilled(SplineFilledEvent),
    TradeSummary(TradeSummaryEvent),
    OrderModified(OrderModifiedEvent),
    MarketSummary(MarketSummaryEvent),

    // Trader events
    TraderRegistered(TraderRegisteredEvent),
    TraderCollateralTransferred(TraderCollateralTransferredEvent),
    TraderActivated(TraderActivatedEvent),
    TraderDeactivated(TraderDeactivatedEvent),
    TraderFundsDeposited(TraderFundsDepositedEvent),
    TraderFundsWithdrawn(TraderFundsWithdrawnEvent),
    TraderFundsWithdrawnEnqueued(TraderFundsWithdrawnEvent),
    TraderFundsWithdrawnDropped(TraderFundsWithdrawnEvent),
    TraderWithdrawCancelled(TraderWithdrawCancelledEvent),
    TraderFundingSettled(TraderFundingSettledEvent),

    // Spline events
    SplineRegistered(SplineRegisteredEvent),
    SplineActivated(SplineActivatedEvent),
    SplineDeactivated(SplineDeactivatedEvent),
    SplinePriceUpdated(SplinePriceUpdatedEvent),
    SplineParametersUpdated(SplineParametersUpdatedEvent),

    // Admin events
    MarketAdded(MarketAddedEvent),
    MarketStatusChanged(MarketStatusChangedEvent),
    #[deprecated(note = "This event is no longer emitted and should not be used by indexers")]
    MarketParametersUpdated(MarketParametersUpdatedEvent),
    FundingParametersUpdated(FundingParametersUpdatedEvent),
    FeesClaimed(FeesClaimedEvent),

    // Oracle events
    PricesUpdated(PricesUpdatedEvent),

    // Liquidation transfer events
    LiquidationTransferSummary(LiquidationTransferSummaryEvent),
    LiquidationTransfer(LiquidationTransferEvent),

    // Liquidation events
    Liquidation(LiquidationEvent),

    // Close Match Position events (ADL)
    CloseMatchedPositions(CloseMatchedPositionsEvent),

    // Authority events
    NameSuccessor(NameSuccessorEvent),
    ClaimAuthority(ClaimAuthorityEvent),

    // Withdrawal state events
    WithdrawStateTransition(WithdrawStateTransitionEvent),

    // PnL events
    PnL(PnLEvent),

    // Stop loss events
    StopLossPlaced(StopLossPlacedEvent),
    StopLossCancelled(StopLossCancelledEvent),
    StopLossExecuted(StopLossExecutedEvent),

    // Exchange / onboarding events
    ExchangeStatusChanged(ExchangeStatusChangedEvent),
    TraderCapabilitiesEnabled(TraderCapabilitiesEnabledEvent),

    // Withdrawal Fee Payment
    TraderFundsWithdrawnFeePayment(TraderFundsWithdrawnFeePaymentEvent),

    // Order packet events (raw order packet data)
    OrderPacket(OrderPacketEvent),

    // Permission events
    SetPermission(SetPermissionEvent),

    AuthorityChanged(AuthorityChangedEvent),

    // Trader delegation events
    TraderDelegated(TraderDelegatedEvent),

    AdminParameterUpdated(AdminParameterUpdatedEvent),

    // Trader fee update events
    TraderFeesUpdated(TraderFeesUpdatedEvent),

    // Market closure event (with finalized mark price)
    MarketClosed(MarketClosedEvent),

    // Market deletion event (tombstoned market permanently deleted)
    MarketDeleted(MarketDeletedEvent),

    // Escrow events
    EscrowAccountCreated(EscrowAccountCreatedEvent),
    EscrowRequestCreated(EscrowRequestCreatedEvent),
    EscrowRequestAccepted(EscrowRequestAcceptedEvent),
    EscrowRequestCancelled(EscrowRequestCancelledEvent),

    // Spline events with anti-reordering protection
    SplinePriceUpdatedWithOrdering(SplinePriceUpdatedWithOrderingEvent),
    SplineParametersUpdatedWithOrdering(SplineParametersUpdatedWithOrderingEvent),

    // Conditional order trigger events
    TriggerOrderPlaced(TriggerOrderPlacedEvent),
    TriggerOrderCancelled(TriggerOrderCancelledEvent),
    TriggerOrderExecuted(TriggerOrderExecutedEvent),

    // Conditional order ping state-machine events
    PingInvalidated(PingInvalidatedEvent),
    PingActivated(PingActivatedEvent),

    // Spline position limits config events
    SplinePositionLimitsConfigUpdated(SplinePositionLimitsConfigUpdatedEvent),
    // Market tombstone event (with final sequence numbers)
    MarketTombstoned(MarketTombstonedEvent),

    // Shutdown close positions event (emitted during settle_closed_market_position)
    ShutdownClosePositions(ShutdownClosePositionsEvent),

    // Residual quantity discarded during order placement
    OrderResidualDiscarded(OrderResidualDiscardedEvent),
}

/// Stable event type for a decoded [`MarketEvent`].
///
/// This is useful when callers need to classify events without matching on the
/// full payload enum. `Display` prints the canonical Rust variant name.
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MarketEventType {
    SlotContext,
    Header,
    OrderPlaced,
    OrderFilled,
    OrderRejected,
    SplineFilled,
    TradeSummary,
    OrderModified,
    MarketSummary,
    TraderRegistered,
    TraderCollateralTransferred,
    TraderActivated,
    TraderDeactivated,
    TraderFundsDeposited,
    TraderFundsWithdrawn,
    TraderFundsWithdrawnEnqueued,
    TraderFundsWithdrawnDropped,
    TraderWithdrawCancelled,
    TraderFundingSettled,
    SplineRegistered,
    SplineActivated,
    SplineDeactivated,
    SplinePriceUpdated,
    SplineParametersUpdated,
    MarketAdded,
    MarketStatusChanged,
    MarketParametersUpdated,
    FundingParametersUpdated,
    FeesClaimed,
    PricesUpdated,
    LiquidationTransferSummary,
    LiquidationTransfer,
    Liquidation,
    CloseMatchedPositions,
    NameSuccessor,
    ClaimAuthority,
    WithdrawStateTransition,
    PnL,
    StopLossPlaced,
    StopLossCancelled,
    StopLossExecuted,
    ExchangeStatusChanged,
    TraderCapabilitiesEnabled,
    TraderFundsWithdrawnFeePayment,
    OrderPacket,
    SetPermission,
    AuthorityChanged,
    TraderDelegated,
    AdminParameterUpdated,
    TraderFeesUpdated,
    MarketClosed,
    MarketDeleted,
    EscrowAccountCreated,
    EscrowRequestCreated,
    EscrowRequestAccepted,
    EscrowRequestCancelled,
    SplinePriceUpdatedWithOrdering,
    SplineParametersUpdatedWithOrdering,
    TriggerOrderPlaced,
    TriggerOrderCancelled,
    TriggerOrderExecuted,
    PingInvalidated,
    PingActivated,
    SplinePositionLimitsConfigUpdated,
    MarketTombstoned,
    ShutdownClosePositions,
    OrderResidualDiscarded,
}

impl fmt::Display for MarketEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl MarketEvent {
    /// Return the stable type tag for this event without moving the payload.
    #[must_use]
    pub const fn event_type(&self) -> MarketEventType {
        match self {
            MarketEvent::SlotContext(_) => MarketEventType::SlotContext,
            MarketEvent::Header(_) => MarketEventType::Header,
            MarketEvent::OrderPlaced(_) => MarketEventType::OrderPlaced,
            MarketEvent::OrderFilled(_) => MarketEventType::OrderFilled,
            MarketEvent::OrderRejected(_) => MarketEventType::OrderRejected,
            MarketEvent::SplineFilled(_) => MarketEventType::SplineFilled,
            MarketEvent::TradeSummary(_) => MarketEventType::TradeSummary,
            MarketEvent::OrderModified(_) => MarketEventType::OrderModified,
            MarketEvent::MarketSummary(_) => MarketEventType::MarketSummary,
            MarketEvent::TraderRegistered(_) => MarketEventType::TraderRegistered,
            MarketEvent::TraderCollateralTransferred(_) => {
                MarketEventType::TraderCollateralTransferred
            }
            MarketEvent::TraderActivated(_) => MarketEventType::TraderActivated,
            MarketEvent::TraderDeactivated(_) => MarketEventType::TraderDeactivated,
            MarketEvent::TraderFundsDeposited(_) => MarketEventType::TraderFundsDeposited,
            MarketEvent::TraderFundsWithdrawn(_) => MarketEventType::TraderFundsWithdrawn,
            MarketEvent::TraderFundsWithdrawnEnqueued(_) => {
                MarketEventType::TraderFundsWithdrawnEnqueued
            }
            MarketEvent::TraderFundsWithdrawnDropped(_) => {
                MarketEventType::TraderFundsWithdrawnDropped
            }
            MarketEvent::TraderWithdrawCancelled(_) => MarketEventType::TraderWithdrawCancelled,
            MarketEvent::TraderFundingSettled(_) => MarketEventType::TraderFundingSettled,
            MarketEvent::SplineRegistered(_) => MarketEventType::SplineRegistered,
            MarketEvent::SplineActivated(_) => MarketEventType::SplineActivated,
            MarketEvent::SplineDeactivated(_) => MarketEventType::SplineDeactivated,
            MarketEvent::SplinePriceUpdated(_) => MarketEventType::SplinePriceUpdated,
            MarketEvent::SplineParametersUpdated(_) => MarketEventType::SplineParametersUpdated,
            MarketEvent::MarketAdded(_) => MarketEventType::MarketAdded,
            MarketEvent::MarketStatusChanged(_) => MarketEventType::MarketStatusChanged,
            MarketEvent::MarketParametersUpdated(_) => MarketEventType::MarketParametersUpdated,
            MarketEvent::FundingParametersUpdated(_) => MarketEventType::FundingParametersUpdated,
            MarketEvent::FeesClaimed(_) => MarketEventType::FeesClaimed,
            MarketEvent::PricesUpdated(_) => MarketEventType::PricesUpdated,
            MarketEvent::LiquidationTransferSummary(_) => {
                MarketEventType::LiquidationTransferSummary
            }
            MarketEvent::LiquidationTransfer(_) => MarketEventType::LiquidationTransfer,
            MarketEvent::Liquidation(_) => MarketEventType::Liquidation,
            MarketEvent::CloseMatchedPositions(_) => MarketEventType::CloseMatchedPositions,
            MarketEvent::NameSuccessor(_) => MarketEventType::NameSuccessor,
            MarketEvent::ClaimAuthority(_) => MarketEventType::ClaimAuthority,
            MarketEvent::WithdrawStateTransition(_) => MarketEventType::WithdrawStateTransition,
            MarketEvent::PnL(_) => MarketEventType::PnL,
            MarketEvent::StopLossPlaced(_) => MarketEventType::StopLossPlaced,
            MarketEvent::StopLossCancelled(_) => MarketEventType::StopLossCancelled,
            MarketEvent::StopLossExecuted(_) => MarketEventType::StopLossExecuted,
            MarketEvent::ExchangeStatusChanged(_) => MarketEventType::ExchangeStatusChanged,
            MarketEvent::TraderCapabilitiesEnabled(_) => MarketEventType::TraderCapabilitiesEnabled,
            MarketEvent::TraderFundsWithdrawnFeePayment(_) => {
                MarketEventType::TraderFundsWithdrawnFeePayment
            }
            MarketEvent::OrderPacket(_) => MarketEventType::OrderPacket,
            MarketEvent::SetPermission(_) => MarketEventType::SetPermission,
            MarketEvent::AuthorityChanged(_) => MarketEventType::AuthorityChanged,
            MarketEvent::TraderDelegated(_) => MarketEventType::TraderDelegated,
            MarketEvent::AdminParameterUpdated(_) => MarketEventType::AdminParameterUpdated,
            MarketEvent::TraderFeesUpdated(_) => MarketEventType::TraderFeesUpdated,
            MarketEvent::MarketClosed(_) => MarketEventType::MarketClosed,
            MarketEvent::MarketDeleted(_) => MarketEventType::MarketDeleted,
            MarketEvent::EscrowAccountCreated(_) => MarketEventType::EscrowAccountCreated,
            MarketEvent::EscrowRequestCreated(_) => MarketEventType::EscrowRequestCreated,
            MarketEvent::EscrowRequestAccepted(_) => MarketEventType::EscrowRequestAccepted,
            MarketEvent::EscrowRequestCancelled(_) => MarketEventType::EscrowRequestCancelled,
            MarketEvent::SplinePriceUpdatedWithOrdering(_) => {
                MarketEventType::SplinePriceUpdatedWithOrdering
            }
            MarketEvent::SplineParametersUpdatedWithOrdering(_) => {
                MarketEventType::SplineParametersUpdatedWithOrdering
            }
            MarketEvent::TriggerOrderPlaced(_) => MarketEventType::TriggerOrderPlaced,
            MarketEvent::TriggerOrderCancelled(_) => MarketEventType::TriggerOrderCancelled,
            MarketEvent::TriggerOrderExecuted(_) => MarketEventType::TriggerOrderExecuted,
            MarketEvent::PingInvalidated(_) => MarketEventType::PingInvalidated,
            MarketEvent::PingActivated(_) => MarketEventType::PingActivated,
            MarketEvent::SplinePositionLimitsConfigUpdated(_) => {
                MarketEventType::SplinePositionLimitsConfigUpdated
            }
            MarketEvent::MarketTombstoned(_) => MarketEventType::MarketTombstoned,
            MarketEvent::ShutdownClosePositions(_) => MarketEventType::ShutdownClosePositions,
            MarketEvent::OrderResidualDiscarded(_) => MarketEventType::OrderResidualDiscarded,
        }
    }
}

impl From<&MarketEvent> for MarketEventType {
    fn from(event: &MarketEvent) -> Self {
        event.event_type()
    }
}
