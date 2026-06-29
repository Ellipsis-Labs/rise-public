use borsh::{BorshDeserialize, BorshSerialize};

use super::*;

#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SlotContextEvent {
    pub timestamp: u64,
    pub slot: u64,
}

#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MarketEventHeader {
    pub sequence_number: u64,
    pub prev_sequence_number_slot: u64,
    pub asset_symbol: Symbol,
    pub asset_id: u32,
    pub tick_size: u32,
    pub base_lot_decimals: i8,
    pub quote_lot_decimals: u8,
    pub signer: Pubkey,
    pub trader_account: Pubkey,
}

#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OrderPlacedEvent {
    pub order_id: u32,
    pub order_flags: OrderFlags,
    pub order_sequence_number: u64,
    pub prev_order_sequence_number_slot: u64,
    pub client_order_id: [u8; 16],
    pub price: Ticks,
    /// Use the sign of the quantity to determine the side of the order
    pub quantity: SignedBaseLots,

    pub last_valid_slot: OptionalNonZeroU64,
    pub initial_slot: u64,
}

/// Raw order packet event - captures the order packet as submitted by the
/// trader
#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OrderPacketEvent {
    pub order_packet: OrderPacket,
    pub trader: Pubkey,
    pub next_order_sequence_number: u64,
}

#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OrderRejectionReason {
    TooManyLimitOrders,
    PostOnlyCross,
    InvalidOrderPacket,
    TiFInvalid,
    OutsideExecutionPriceBand,
}
impl OrderRejectionReason {
    pub fn to_bytes(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        let s = match self {
            OrderRejectionReason::TooManyLimitOrders => "TooManyLimitOrders",
            OrderRejectionReason::PostOnlyCross => "PostOnlyCross",
            OrderRejectionReason::InvalidOrderPacket => "InvalidOrderPacket",
            OrderRejectionReason::TiFInvalid => "TiFInvalid",
            OrderRejectionReason::OutsideExecutionPriceBand => "OutsideExecutionPriceBand",
        };
        bytes[..s.len()].copy_from_slice(s.as_bytes());
        bytes
    }
}
impl From<OrderRejectionReason> for [u8; 32] {
    fn from(val: OrderRejectionReason) -> Self {
        val.to_bytes()
    }
}

#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OrderRejectedEvent {
    pub order_index: u32,
    pub client_order_id: [u8; 16],
    pub price: Ticks,
    pub side: Side,
    pub num_base_lots: BaseLots,
    // TODO: maybe put entire order packet here?
    pub reason: [u8; 32],
}
impl OrderRejectedEvent {
    pub fn reason_str(&self) -> String {
        // Reasons are stored in a fixed-width buffer padded with NUL bytes; slice up to
        // the first padding byte
        let len = self
            .reason
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(self.reason.len());
        String::from_utf8_lossy(&self.reason[..len]).to_string()
    }

    pub fn reason(&self) -> Option<OrderRejectionReason> {
        let reason_str = self.reason_str();
        match reason_str.as_str() {
            "TooManyLimitOrders" => Some(OrderRejectionReason::TooManyLimitOrders),
            "PostOnlyCross" => Some(OrderRejectionReason::PostOnlyCross),
            "InvalidOrderPacket" => Some(OrderRejectionReason::InvalidOrderPacket),
            "TiFInvalid" => Some(OrderRejectionReason::TiFInvalid),
            "OutsideExecutionPriceBand" => Some(OrderRejectionReason::OutsideExecutionPriceBand),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reason_str_trims_null_padding() {
        let mut reason = [0u8; 32];
        let text = b"TiFInvalid";
        reason[..text.len()].copy_from_slice(text);
        // Add explicit padding to ensure the implementation drops it
        reason[text.len()] = 0u8;

        let event = OrderRejectedEvent {
            order_index: 0,
            client_order_id: [0; 16],
            price: Ticks::from(0),
            side: Side::Bid,
            num_base_lots: BaseLots::from(0),
            reason,
        };

        assert_eq!(event.reason_str(), "TiFInvalid");
    }

    #[test]
    fn rejection_reason_roundtrips_outside_execution_band() {
        let event = OrderRejectedEvent {
            order_index: 0,
            client_order_id: [0; 16],
            price: Ticks::from(0),
            side: Side::Bid,
            num_base_lots: BaseLots::from(0),
            reason: OrderRejectionReason::OutsideExecutionPriceBand.to_bytes(),
        };

        assert!(matches!(
            event.reason(),
            Some(OrderRejectionReason::OutsideExecutionPriceBand)
        ));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn event_quantity_wrappers_serialize_as_strings_and_accept_numbers() {
        let event = OrderRejectedEvent {
            order_index: 0,
            client_order_id: [0; 16],
            price: Ticks::new(9_007_199_254_740_993),
            side: Side::Bid,
            num_base_lots: BaseLots::new(123),
            reason: OrderRejectionReason::InvalidOrderPacket.to_bytes(),
        };

        let json = serde_json::to_value(event).unwrap();
        assert_eq!(json["price"], "9007199254740993");
        assert_eq!(json["num_base_lots"], "123");

        let decoded: OrderRejectedEvent = serde_json::from_value(serde_json::json!({
            "order_index": 0,
            "client_order_id": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            "price": 42,
            "side": "Bid",
            "num_base_lots": "123",
            "reason": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        }))
        .unwrap();
        assert_eq!(decoded.price, Ticks::new(42));
        assert_eq!(decoded.num_base_lots, BaseLots::new(123));
    }
}

#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OrderResidualDiscardedEvent {
    pub client_order_id: [u8; 16],
    pub price: Ticks,
    pub side: Side,
    pub base_lots_discarded: BaseLots,
    pub reason: OrderResidualDiscardReason,
}

#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OrderResidualDiscardReason {
    OutsideExecutionPriceBand,
}

// Keep this data structure as small as possible to reduce the size of the log
// instruction data
#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OrderFilledEvent {
    pub order_sequence_number: u64,

    /// The side of the order from the maker's perspective
    pub side: Side,
    pub price: Ticks,
    pub base_lots_filled: BaseLots,
    pub quote_lots_filled: QuoteLots,
    pub quantity_remaining: BaseLots,

    // We dont compress this to simplify the off-chain idexing logic.
    // We could be smart about how we emit this multiple times but we're heap constrained.
    pub maker: Pubkey,
    pub maker_fee_rate: SignedFeeRateMicro,
    pub maker_base_lot_position: SignedBaseLots,
    pub maker_virtual_quote_lot_position: SignedQuoteLots,
    pub maker_quote_lot_collateral: SignedQuoteLots,
    pub maker_cumulative_funding_snapshot: SignedQuoteLotsPerBaseLot,
}
impl OrderFilledEvent {
    pub fn quantity(&self) -> SignedBaseLots {
        match self.side {
            Side::Bid => self.base_lots_filled.as_signed(),
            Side::Ask => SignedBaseLots::new(-1) * self.base_lots_filled.as_signed(),
        }
    }
}

#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SplineFilledEvent {
    pub spline_sequence_number: u64,

    /// The side of the order from the spline's perspective
    pub side: Side,
    pub price: Ticks,
    pub base_lots_filled: BaseLots,
    pub quote_lots_filled: QuoteLots,

    pub maker: Pubkey,
    pub maker_fee_rate: SignedFeeRateMicro,
    pub maker_base_lot_position: SignedBaseLots,
    pub maker_virtual_quote_lot_position: SignedQuoteLots,
    pub maker_quote_lot_collateral: SignedQuoteLots,
    pub maker_cumulative_funding_snapshot: SignedQuoteLotsPerBaseLot,
}
impl SplineFilledEvent {
    pub fn quantity(&self) -> SignedBaseLots {
        match self.side {
            Side::Bid => self.base_lots_filled.as_signed(),
            Side::Ask => SignedBaseLots::new(-1) * self.base_lots_filled.as_signed(),
        }
    }
}

#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OrderModifiedEvent {
    pub order_sequence_number: u64,
    pub price: Ticks,
    /// Use the sign of the quantity to determine the side of the order
    pub base_lots_released: SignedBaseLots,
    pub quote_lots_released: SignedQuoteLots,
    pub base_lots_remaining: BaseLots,
    pub reason: OrderModificationReason,
}

#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OrderModificationReason {
    /// User explicitly cancelled the order
    UserRequested,
    /// Order cancelled by cancel authority
    AuthorityForced,
    /// Order cancelled by self trade with CancelProvide behavior
    SelfTradeCancelProvide,
    /// Order cancelled by self trade with DecrementTake behavior
    SelfTradeDecrementTake,
    /// Order expired
    Expired,
    /// Reduce-only order invalidated
    ReduceOnlyInvalidated,
    /// Risk capacity exceeded
    RiskCapacityExceeded,
    /// Order evicted due to book capacity limit
    BookCapacityEvicted,
    /// Tombstone order
    Tombstone,
    /// Top-of-book order invalidated because it is outside the execution price
    /// band
    TopOfBookOutOfBounds,
    /// Cancelled as collateral to a keeper-triggered trigger-order execution
    /// (stop-loss, take-profit, or conditional-order IOC) that needed to free
    /// margin capacity in order to place the trigger order.
    StopLossCascade,
}

#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TradeSummaryEvent {
    /// The taker of the trade.
    pub trader: Pubkey,
    pub trade_sequence_number: u64,
    pub prev_trade_sequence_number_slot: u64,
    pub side: Side,
    pub base_lots_filled: BaseLots,
    pub quote_lots_filled: QuoteLots,
    pub fee_in_quote_lots: QuoteLots,

    pub base_lot_position: SignedBaseLots,
    pub virtual_quote_lot_position: SignedQuoteLots,
    pub quote_lot_collateral: SignedQuoteLots,
    pub cumulative_funding_snapshot: SignedQuoteLotsPerBaseLot,
}

#[derive(Copy, Clone, BorshDeserialize, BorshSerialize, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MarketSummaryEvent {
    pub asset_symbol: Symbol,
    pub asset_id: u32,
    pub open_interest: BaseLots,
    pub total_maker_quote_lot_fees: Option<SignedQuoteLots>,
    pub total_taker_quote_lot_fees: Option<QuoteLots>,
    pub mark_price: Ticks,
    pub spot_price: Ticks,
}
