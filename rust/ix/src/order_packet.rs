//! OrderPacket types for Phoenix instruction serialization.
//!
//! These types match the wire format expected by the Phoenix program,
//! using proper Borsh serialization with `Option<T>` types.

use borsh::{BorshDeserialize, BorshSerialize};

use crate::types::{OrderFlags, SelfTradeBehavior, Side};

/// An order packet for Phoenix instructions.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OrderPacket {
    pub(crate) kind: OrderPacketKind,
}

impl OrderPacket {
    /// Create a new post-only order packet.
    pub fn post_only(
        side: Side,
        price_in_ticks: u64,
        num_base_lots: u64,
        client_order_id: [u8; 16],
        slide: bool,
        last_valid_slot: Option<u64>,
        order_flags: OrderFlags,
        cancel_existing: bool,
    ) -> Self {
        Self {
            kind: OrderPacketKind::PostOnly {
                side,
                price_in_ticks,
                num_base_lots,
                client_order_id,
                slide,
                last_valid_slot,
                order_flags,
                cancel_existing,
            },
        }
    }

    /// Create a new limit order packet.
    pub fn limit(
        side: Side,
        price_in_ticks: u64,
        num_base_lots: u64,
        self_trade_behavior: SelfTradeBehavior,
        match_limit: Option<u64>,
        client_order_id: [u8; 16],
        last_valid_slot: Option<u64>,
        order_flags: OrderFlags,
        cancel_existing: bool,
    ) -> Self {
        Self {
            kind: OrderPacketKind::Limit {
                side,
                price_in_ticks,
                num_base_lots,
                self_trade_behavior,
                match_limit,
                client_order_id,
                last_valid_slot,
                order_flags,
                cancel_existing,
            },
        }
    }

    /// Create a new immediate-or-cancel order packet (used for market orders).
    pub fn immediate_or_cancel(
        side: Side,
        price_in_ticks: Option<u64>,
        num_base_lots: u64,
        num_quote_lots: Option<u64>,
        min_base_lots_to_fill: u64,
        min_quote_lots_to_fill: u64,
        self_trade_behavior: SelfTradeBehavior,
        match_limit: Option<u64>,
        client_order_id: [u8; 16],
        last_valid_slot: Option<u64>,
        order_flags: OrderFlags,
        cancel_existing: bool,
    ) -> Self {
        Self {
            kind: OrderPacketKind::ImmediateOrCancel {
                side,
                price_in_ticks,
                num_base_lots,
                num_quote_lots,
                min_base_lots_to_fill,
                min_quote_lots_to_fill,
                self_trade_behavior,
                match_limit,
                client_order_id,
                last_valid_slot,
                order_flags,
                cancel_existing,
            },
        }
    }
}

/// The kind of order packet, matching the Phoenix program's enum.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) enum OrderPacketKind {
    /// Post-only order that will not match against existing orders.
    PostOnly {
        side: Side,
        price_in_ticks: u64,
        num_base_lots: u64,
        client_order_id: [u8; 16],
        slide: bool,
        last_valid_slot: Option<u64>,
        order_flags: OrderFlags,
        cancel_existing: bool,
    },
    /// Limit order that can match against existing orders.
    Limit {
        side: Side,
        price_in_ticks: u64,
        num_base_lots: u64,
        self_trade_behavior: SelfTradeBehavior,
        match_limit: Option<u64>,
        client_order_id: [u8; 16],
        last_valid_slot: Option<u64>,
        order_flags: OrderFlags,
        cancel_existing: bool,
    },
    /// Immediate-or-cancel order (used for market orders).
    ImmediateOrCancel {
        side: Side,
        price_in_ticks: Option<u64>,
        num_base_lots: u64,
        num_quote_lots: Option<u64>,
        min_base_lots_to_fill: u64,
        min_quote_lots_to_fill: u64,
        self_trade_behavior: SelfTradeBehavior,
        match_limit: Option<u64>,
        client_order_id: [u8; 16],
        last_valid_slot: Option<u64>,
        order_flags: OrderFlags,
        cancel_existing: bool,
    },
}

/// Convert a u128 client order ID to the [u8; 16] format expected by the
/// program.
pub fn client_order_id_to_bytes(id: u128) -> [u8; 16] {
    id.to_le_bytes()
}

/// A condensed order for use in multi-limit-order instructions.
///
/// Contains only price, size, and optional expiry — the minimal data
/// needed per order in a batch.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CondensedOrder {
    pub price_in_ticks: u64,
    pub size_in_base_lots: u64,
    pub last_valid_slot: Option<u64>,
}

/// A batch of post-only limit orders (bids and asks) sent in a single
/// instruction.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MultipleOrderPacket {
    pub bids: Vec<CondensedOrder>,
    pub asks: Vec<CondensedOrder>,
    pub client_order_id: Option<[u8; 16]>,
    /// Whether orders should slide to the top of the book if they would cross.
    pub slide: bool,
}

#[cfg(test)]
mod tests {
    use borsh::to_vec;

    use super::*;

    #[test]
    fn test_ioc_serialization_with_none_price() {
        let packet = OrderPacket::immediate_or_cancel(
            Side::Bid,
            None, // price_in_ticks
            1000,
            None, // num_quote_lots
            0,
            0,
            SelfTradeBehavior::Abort,
            None, // match_limit
            [0u8; 16],
            None, // last_valid_slot
            OrderFlags::None,
            false,
        );

        let bytes = to_vec(&packet.kind).unwrap();

        // Verify the discriminant is 2 (ImmediateOrCancel)
        assert_eq!(bytes[0], 2);

        // Verify side is 0 (Bid)
        assert_eq!(bytes[1], 0);

        // Verify price_in_ticks Option discriminant is 0 (None)
        assert_eq!(bytes[2], 0);
        // After None, next field should start immediately (no 8-byte value)
    }

    #[test]
    fn test_ioc_serialization_with_some_price() {
        let packet = OrderPacket::immediate_or_cancel(
            Side::Ask,
            Some(50000),
            1000,
            None,
            0,
            0,
            SelfTradeBehavior::Abort,
            None,
            [0u8; 16],
            None,
            OrderFlags::None,
            false,
        );

        let bytes = to_vec(&packet.kind).unwrap();

        // Verify the discriminant is 2 (ImmediateOrCancel)
        assert_eq!(bytes[0], 2);

        // Verify side is 1 (Ask)
        assert_eq!(bytes[1], 1);

        // Verify price_in_ticks Option discriminant is 1 (Some)
        assert_eq!(bytes[2], 1);

        // Verify price value (50000 as little-endian u64)
        let price_bytes = &bytes[3..11];
        let price = u64::from_le_bytes(price_bytes.try_into().unwrap());
        assert_eq!(price, 50000);
    }

    #[test]
    fn test_limit_serialization() {
        let packet = OrderPacket::limit(
            Side::Bid,
            50000,
            1000,
            SelfTradeBehavior::CancelProvide,
            None,
            [0u8; 16],
            None,
            OrderFlags::None,
            false,
        );

        let bytes = to_vec(&packet.kind).unwrap();

        // Verify the discriminant is 1 (Limit)
        assert_eq!(bytes[0], 1);

        // Verify side is 0 (Bid)
        assert_eq!(bytes[1], 0);
    }

    #[test]
    fn test_client_order_id_to_bytes() {
        let id: u128 = 0x0102030405060708090a0b0c0d0e0f10;
        let bytes = client_order_id_to_bytes(id);
        assert_eq!(
            bytes,
            [
                0x10, 0x0f, 0x0e, 0x0d, 0x0c, 0x0b, 0x0a, 0x09, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03,
                0x02, 0x01
            ]
        );
    }
}
