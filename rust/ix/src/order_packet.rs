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

/// Fixed-width optional `u64`: `0` encodes `None` on the wire (no Borsh
/// `Option` tag). Same pattern as the market-event type in
/// `phoenix-rise-events`.
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

/// One-byte per-order wire flags for [`CondensedOrderV2`]; matches
/// `program-core`'s `CondensedOrderFlags` and the TS enum. Distinct from the
/// account-layout [`OrderFlags`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CondensedOrderFlags(u8);

impl CondensedOrderFlags {
    pub const REDUCE_ONLY: u8 = 1 << 1;
    /// Slide to the top of the book instead of erroring when the order would
    /// cross.
    pub const SLIDE: u8 = 1 << 0;

    pub const fn from_parts(slide: bool, reduce_only: bool) -> Self {
        let mut bits = 0;
        if slide {
            bits |= Self::SLIDE;
        }
        if reduce_only {
            bits |= Self::REDUCE_ONLY;
        }
        Self(bits)
    }

    pub const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }

    pub const fn as_u8(self) -> u8 {
        self.0
    }

    pub const fn is_slide(self) -> bool {
        self.0 & Self::SLIDE != 0
    }

    pub const fn is_reduce_only(self) -> bool {
        self.0 & Self::REDUCE_ONLY != 0
    }
}

/// V2 of [`CondensedOrder`]: fixed-width expiry plus a trailing per-order
/// flags byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CondensedOrderV2 {
    pub price_in_ticks: u64,
    pub size_in_base_lots: u64,
    pub last_valid_slot: OptionalNonZeroU64,
    pub flags: CondensedOrderFlags,
}

impl CondensedOrderV2 {
    /// `last_valid_slot: Some(0)` collapses to the no-expiry sentinel
    /// ([`OptionalNonZeroU64`] encodes 0 as none), matching program-core's
    /// `CondensedOrderV2Builder`; the TS encoder instead throws on an explicit
    /// 0.
    pub const fn new(
        price_in_ticks: u64,
        size_in_base_lots: u64,
        last_valid_slot: Option<u64>,
        slide: bool,
        reduce_only: bool,
    ) -> Self {
        Self {
            price_in_ticks,
            size_in_base_lots,
            last_valid_slot: match last_valid_slot {
                Some(slot) => OptionalNonZeroU64::new(slot),
                None => OptionalNonZeroU64::null(),
            },
            flags: CondensedOrderFlags::from_parts(slide, reduce_only),
        }
    }
}

/// V2 of [`MultipleOrderPacket`] for `place_multi_limit_order_v2`: per-order
/// flags instead of packet-wide fields, plus a trailing `scale_set_id`.
/// Matches `program-core`'s `MultipleOrderPacketV2` and the TS type.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MultipleOrderPacketV2 {
    pub bids: Vec<CondensedOrderV2>,
    pub asks: Vec<CondensedOrderV2>,
    pub client_order_id: Option<[u8; 16]>,
    /// 0 = not part of a scale-order set; 1-255 = caller-assigned ladder id.
    pub scale_set_id: u8,
}

/// The canonical `MultipleOrderPacketV2` body pinned by the Rust and TS parity
/// tests. Shared with `multi_limit_order`'s instruction-data test, which is
/// exactly this vector prefixed by the V2 discriminant.
#[cfg(test)]
pub(crate) const V2_PARITY_PACKET_BYTES: &[u8] = &[
    1, 0, 0, 0, // bids: Vec len = 1
    80, 195, 0, 0, 0, 0, 0, 0, // bid.price_in_ticks = 50_000
    232, 3, 0, 0, 0, 0, 0, 0, // bid.size_in_base_lots = 1_000
    0, 0, 0, 0, 0, 0, 0, 0, // bid.last_valid_slot = None (raw 0)
    0, // bid.flags = None
    1, 0, 0, 0, // asks: Vec len = 1
    56, 199, 0, 0, 0, 0, 0, 0, // ask.price_in_ticks = 51_000
    244, 1, 0, 0, 0, 0, 0, 0, // ask.size_in_base_lots = 500
    231, 3, 0, 0, 0, 0, 0, 0, // ask.last_valid_slot = Some(999)
    2, // ask.flags = ReduceOnly
    1, // client_order_id: Option tag = Some
    16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, // client_order_id bytes
    7, // scale_set_id
];

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

    #[test]
    fn test_optional_nonzero_u64_wire_format() {
        assert_eq!(to_vec(&OptionalNonZeroU64::null()).unwrap(), vec![0u8; 8]);
        assert_eq!(OptionalNonZeroU64::null().get(), None);

        let some = OptionalNonZeroU64::new(12345);
        assert_eq!(some.get(), Some(12345));
        assert_eq!(
            to_vec(&some).unwrap(),
            12345u64.to_le_bytes().to_vec(),
            "should serialize as a raw little-endian u64, not a Borsh Option"
        );

        // 0 reads back as None.
        assert_eq!(OptionalNonZeroU64::new(0), OptionalNonZeroU64::null());
        assert_eq!(OptionalNonZeroU64::new(0).get(), None);
    }

    #[test]
    fn test_condensed_order_flags_bits() {
        let none = CondensedOrderFlags::from_parts(false, false);
        assert_eq!(none.as_u8(), 0);
        assert!(!none.is_slide());
        assert!(!none.is_reduce_only());

        let slide = CondensedOrderFlags::from_parts(true, false);
        assert_eq!(slide.as_u8(), 1);
        assert!(slide.is_slide());
        assert!(!slide.is_reduce_only());

        let both = CondensedOrderFlags::from_parts(true, true);
        assert_eq!(both.as_u8(), 3, "slide and reduce-only bits should combine");
        assert!(both.is_slide());
        assert!(both.is_reduce_only());

        assert_eq!(CondensedOrderFlags::from_bits(3), both);
    }

    /// Fixture shared verbatim with the TS parity test in
    /// `rise/ts/tests/order-packets-parity.test.ts`.
    #[test]
    fn test_condensed_order_v2_byte_layout() {
        let order = CondensedOrderV2::new(50_000, 1_000, Some(12_345), true, false);

        let bytes = to_vec(&order).unwrap();
        assert_eq!(
            bytes,
            vec![
                // price_in_ticks = 50_000 (u64 LE)
                0x50, 0xC3, 0, 0, 0, 0, 0, 0, //
                // size_in_base_lots = 1_000 (u64 LE)
                0xE8, 0x03, 0, 0, 0, 0, 0, 0, //
                // last_valid_slot = Some(12_345) (raw u64 LE, no Option tag)
                0x39, 0x30, 0, 0, 0, 0, 0, 0, //
                // flags = Slide (0b01)
                0x01,
            ]
        );
    }

    /// Fixture shared verbatim with the TS parity test in
    /// `rise/ts/tests/order-packets-parity.test.ts`.
    #[test]
    fn test_multiple_order_packet_v2_byte_layout() {
        let packet = MultipleOrderPacketV2 {
            bids: vec![CondensedOrderV2::new(50_000, 1_000, None, false, false)],
            asks: vec![CondensedOrderV2::new(51_000, 500, Some(999), false, true)],
            client_order_id: Some(client_order_id_to_bytes(0x0102030405060708090a0b0c0d0e0f10)),
            scale_set_id: 7,
        };

        assert_eq!(to_vec(&packet).unwrap(), V2_PARITY_PACKET_BYTES);
    }
}
