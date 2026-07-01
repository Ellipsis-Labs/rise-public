use serde::ser::{SerializeSeq, SerializeStruct};
use serde::{Serialize, Serializer};

use super::internal::{
    ORDERBOOK_CAPACITY, OrderbookEntry, OrderbookRestingOrder, Reader, read_fifo_order_id,
    read_orderbook_resting_order, verify_discriminant,
};
use super::orderbook_header::{
    ORDERBOOK_HEADER_SIZE, OrderbookHeader, read_orderbook_header_after_discriminant,
};
use super::{AccountDeserialize, AccountDeserializeError};
use crate::PhoenixAccount;

const ACCOUNT: &str = "Orderbook";
const LIST_HEADER_BYTES: usize = 16;
const ALLOCATOR_HEADER_BYTES: usize = 16;
const NODE_PREFIX_BYTES: usize = 8;
const ORDER_ID_BYTES: usize = 16;
const RESTING_ORDER_BYTES: usize = 48;
const NODE_STRIDE_BYTES: usize = NODE_PREFIX_BYTES + ORDER_ID_BYTES + RESTING_ORDER_BYTES;
const TREE_BYTES: usize =
    LIST_HEADER_BYTES + ALLOCATOR_HEADER_BYTES + NODE_STRIDE_BYTES * ORDERBOOK_CAPACITY;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Orderbook {
    pub header: OrderbookHeader,
    pub bids: Vec<OrderbookEntry>,
    pub asks: Vec<OrderbookEntry>,
}

impl Serialize for Orderbook {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Orderbook", 9)?;
        state.serialize_field("header", &self.header)?;
        state.serialize_field("bid_count", &self.bids.len())?;
        state.serialize_field("ask_count", &self.asks.len())?;
        state.serialize_field(
            "best_bid",
            &self
                .bids
                .first()
                .map(|entry| SerializableOrderbookEntry::new(OrderbookSide::Bid, entry)),
        )?;
        state.serialize_field(
            "best_ask",
            &self
                .asks
                .first()
                .map(|entry| SerializableOrderbookEntry::new(OrderbookSide::Ask, entry)),
        )?;
        state.serialize_field("bid_levels", &levels(OrderbookSide::Bid, &self.bids))?;
        state.serialize_field("ask_levels", &levels(OrderbookSide::Ask, &self.asks))?;
        state.serialize_field(
            "bids",
            &SerializableOrderbookEntries::new(OrderbookSide::Bid, &self.bids),
        )?;
        state.serialize_field(
            "asks",
            &SerializableOrderbookEntries::new(OrderbookSide::Ask, &self.asks),
        )?;
        state.end()
    }
}

impl AccountDeserialize for Orderbook {
    fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        verify_discriminant(ACCOUNT, data, PhoenixAccount::Orderbook.discriminant())?;
        let mut header_reader = Reader::with_offset(ACCOUNT, data, 8);
        let header = read_orderbook_header_after_discriminant(&mut header_reader)?;
        let bids_offset = ORDERBOOK_HEADER_SIZE;
        let asks_offset = bids_offset + TREE_BYTES;
        let bids = read_tree_entries(data, bids_offset)?;
        let asks = read_tree_entries(data, asks_offset)?;
        Ok(Self { header, bids, asks })
    }
}

impl Orderbook {
    pub fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        <Self as AccountDeserialize>::try_from_account_bytes(data)
    }
}

#[derive(Clone, Copy)]
enum OrderbookSide {
    Bid,
    Ask,
}

impl OrderbookSide {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Bid => "bid",
            Self::Ask => "ask",
        }
    }

    const fn raw_sequence_number(self, order_sequence_number: u64) -> u64 {
        match self {
            Self::Bid => !order_sequence_number,
            Self::Ask => order_sequence_number,
        }
    }
}

struct SerializableOrderbookEntries<'a> {
    side: OrderbookSide,
    entries: &'a [OrderbookEntry],
}

impl<'a> SerializableOrderbookEntries<'a> {
    const fn new(side: OrderbookSide, entries: &'a [OrderbookEntry]) -> Self {
        Self { side, entries }
    }
}

impl Serialize for SerializableOrderbookEntries<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.entries.len()))?;
        for entry in self.entries {
            seq.serialize_element(&SerializableOrderbookEntry::new(self.side, entry))?;
        }
        seq.end()
    }
}

struct SerializableOrderbookEntry<'a> {
    side: OrderbookSide,
    entry: &'a OrderbookEntry,
}

impl<'a> SerializableOrderbookEntry<'a> {
    const fn new(side: OrderbookSide, entry: &'a OrderbookEntry) -> Self {
        Self { side, entry }
    }
}

impl Serialize for SerializableOrderbookEntry<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let order_id = &self.entry.order_id;
        let order = &self.entry.order;
        let mut state = serializer.serialize_struct("OrderbookEntry", 13)?;
        state.serialize_field("side", self.side.as_str())?;
        state.serialize_field("price_in_ticks", &order_id.price_in_ticks.as_inner())?;
        state.serialize_field("order_sequence_number", &order_id.order_sequence_number)?;
        state.serialize_field(
            "raw_sequence_number",
            &self
                .side
                .raw_sequence_number(order_id.order_sequence_number),
        )?;
        state.serialize_field("trader_position_id", &order.trader_position_id)?;
        state.serialize_field("base_lots", &SerializableBaseLots::new(order))?;
        state.serialize_field("flags", &SerializableOrderFlags::new(order))?;
        state.serialize_field(
            "optional_conditional_order_index",
            &order.optional_conditional_order_index,
        )?;
        state.serialize_field("expiration_offset", &order.expiration_offset)?;
        state.serialize_field("initial_slot", &order.initial_slot)?;
        state.serialize_field("last_valid_slot", &order.last_valid_slot)?;
        state.serialize_field("links", &SerializableOrderLinks::new(order))?;
        state.serialize_field("raw", self.entry)?;
        state.end()
    }
}

struct SerializableBaseLots<'a> {
    order: &'a OrderbookRestingOrder,
}

impl<'a> SerializableBaseLots<'a> {
    const fn new(order: &'a OrderbookRestingOrder) -> Self {
        Self { order }
    }
}

impl Serialize for SerializableBaseLots<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("OrderbookEntryBaseLots", 2)?;
        state.serialize_field("initial", &self.order.initial_trade_size)?;
        state.serialize_field("remaining", &self.order.num_base_lots_remaining)?;
        state.end()
    }
}

struct SerializableOrderFlags<'a> {
    order: &'a OrderbookRestingOrder,
}

impl<'a> SerializableOrderFlags<'a> {
    const fn new(order: &'a OrderbookRestingOrder) -> Self {
        Self { order }
    }
}

impl Serialize for SerializableOrderFlags<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("OrderbookEntryFlags", 5)?;
        state.serialize_field("raw", &self.order.order_flags)?;
        state.serialize_field("reduce_only", &self.order.reduce_only)?;
        state.serialize_field("is_stop_loss", &self.order.is_stop_loss)?;
        state.serialize_field("is_stop_loss_direction", &self.order.is_stop_loss_direction)?;
        state.serialize_field("is_conditional_order", &self.order.is_conditional_order)?;
        state.end()
    }
}

struct SerializableOrderLinks<'a> {
    order: &'a OrderbookRestingOrder,
}

impl<'a> SerializableOrderLinks<'a> {
    const fn new(order: &'a OrderbookRestingOrder) -> Self {
        Self { order }
    }
}

impl Serialize for SerializableOrderLinks<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("OrderbookEntryLinks", 2)?;
        state.serialize_field("prev_node", &self.order.prev_node)?;
        state.serialize_field("next_node", &self.order.next_node)?;
        state.end()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct OrderbookLevel {
    side: &'static str,
    price_in_ticks: u64,
    order_count: usize,
    total_initial_base_lots: u64,
    total_remaining_base_lots: u64,
    first_raw_sequence_number: u64,
    last_raw_sequence_number: u64,
}

fn levels(side: OrderbookSide, entries: &[OrderbookEntry]) -> Vec<OrderbookLevel> {
    let Some((first, rest)) = entries.split_first() else {
        return Vec::new();
    };

    let mut levels = Vec::new();
    let mut current = OrderbookLevel::new(side, first);
    for entry in rest {
        if entry.order_id.price_in_ticks == current.price_in_ticks {
            current.push(side, entry);
        } else {
            levels.push(current);
            current = OrderbookLevel::new(side, entry);
        }
    }
    levels.push(current);
    levels
}

impl OrderbookLevel {
    fn new(side: OrderbookSide, entry: &OrderbookEntry) -> Self {
        let raw_sequence_number = side.raw_sequence_number(entry.order_id.order_sequence_number);
        Self {
            side: side.as_str(),
            price_in_ticks: entry.order_id.price_in_ticks.as_inner(),
            order_count: 1,
            total_initial_base_lots: entry.order.initial_trade_size,
            total_remaining_base_lots: entry.order.num_base_lots_remaining,
            first_raw_sequence_number: raw_sequence_number,
            last_raw_sequence_number: raw_sequence_number,
        }
    }

    fn push(&mut self, side: OrderbookSide, entry: &OrderbookEntry) {
        self.order_count += 1;
        self.total_initial_base_lots = self
            .total_initial_base_lots
            .saturating_add(entry.order.initial_trade_size);
        self.total_remaining_base_lots = self
            .total_remaining_base_lots
            .saturating_add(entry.order.num_base_lots_remaining);
        self.last_raw_sequence_number =
            side.raw_sequence_number(entry.order_id.order_sequence_number);
    }
}

fn read_tree_entries(
    data: &[u8],
    offset: usize,
) -> Result<Vec<OrderbookEntry>, AccountDeserializeError> {
    let end = offset
        .checked_add(TREE_BYTES)
        .ok_or_else(|| AccountDeserializeError::invalid_data(ACCOUNT, "tree offset overflow"))?;
    if end > data.len() {
        return Err(AccountDeserializeError::too_short(ACCOUNT, end, data.len()));
    }

    let head = read_u32_at(data, offset)?;
    let len = read_u64_at(data, offset + LIST_HEADER_BYTES)? as usize;
    if head == 0 || len == 0 {
        return Ok(Vec::new());
    }

    if len > ORDERBOOK_CAPACITY {
        return Err(AccountDeserializeError::invalid_data(
            ACCOUNT,
            format!("orderbook list length {len} exceeds capacity {ORDERBOOK_CAPACITY}"),
        ));
    }

    let nodes_base = offset + LIST_HEADER_BYTES + ALLOCATOR_HEADER_BYTES;
    let mut entries = Vec::with_capacity(len);
    let mut current = head;

    for _ in 0..len {
        validate_node_index(current)?;
        let node_start = node_start(nodes_base, current)?;
        let kv_start = node_start + NODE_PREFIX_BYTES;
        let mut reader = Reader::with_offset(ACCOUNT, data, kv_start);
        let order_id = read_fifo_order_id(&mut reader)?;
        let order = read_orderbook_resting_order(&mut reader)?;
        entries.push(OrderbookEntry { order_id, order });

        current = read_u32_at(data, node_start + 4)?;
    }

    Ok(entries)
}

fn read_u32_at(data: &[u8], offset: usize) -> Result<u32, AccountDeserializeError> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| AccountDeserializeError::invalid_data(ACCOUNT, "offset overflow"))?;
    if end > data.len() {
        return Err(AccountDeserializeError::too_short(ACCOUNT, end, data.len()));
    }
    Ok(u32::from_le_bytes(
        data[offset..end]
            .try_into()
            .expect("slice length checked above"),
    ))
}

fn read_u64_at(data: &[u8], offset: usize) -> Result<u64, AccountDeserializeError> {
    let end = offset
        .checked_add(8)
        .ok_or_else(|| AccountDeserializeError::invalid_data(ACCOUNT, "offset overflow"))?;
    if end > data.len() {
        return Err(AccountDeserializeError::too_short(ACCOUNT, end, data.len()));
    }
    Ok(u64::from_le_bytes(
        data[offset..end]
            .try_into()
            .expect("slice length checked above"),
    ))
}

fn node_start(nodes_base: usize, node_index: u32) -> Result<usize, AccountDeserializeError> {
    validate_node_index(node_index)?;
    nodes_base
        .checked_add((node_index as usize - 1) * NODE_STRIDE_BYTES)
        .ok_or_else(|| AccountDeserializeError::invalid_data(ACCOUNT, "node offset overflow"))
}

fn validate_node_index(node_index: u32) -> Result<(), AccountDeserializeError> {
    if node_index == 0 || node_index as usize > ORDERBOOK_CAPACITY {
        return Err(AccountDeserializeError::invalid_data(
            ACCOUNT,
            format!("orderbook tree node index {node_index} is out of bounds"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use phoenix_rise_math::Ticks;
    use serde_json::json;

    use super::*;
    use crate::owned::internal::{SequenceNumber, TraderPositionId};

    #[test]
    fn serializes_orderbook_for_display() {
        let orderbook = Orderbook {
            header: OrderbookHeader {
                market_status: 1,
                base_lots_decimals: 3,
                sequence_number: sequence_number(10),
                asset_id: 0,
                asset_symbol: "BTC".to_string(),
                tick_size_in_quote_lots_per_base_lot: 100,
                order_sequence_number: sequence_number(20),
                trade_sequence_number: sequence_number(30),
                default_taker_fee_micro: 400,
                default_maker_fee_micro: -50,
                total_maker_quote_lot_fees: -12,
                total_taker_quote_lot_fees: 34,
                trade_list: Vec::new(),
            },
            bids: vec![
                orderbook_entry(OrderbookSide::Bid, 101, 7, 10, 4),
                orderbook_entry(OrderbookSide::Bid, 101, 8, 20, 6),
            ],
            asks: vec![orderbook_entry(OrderbookSide::Ask, 103, 9, 30, 5)],
        };

        let value = serde_json::to_value(orderbook).expect("orderbook should serialize");

        assert_eq!(value["bid_count"], json!(2));
        assert_eq!(value["ask_count"], json!(1));
        assert_eq!(value["best_bid"]["side"], json!("bid"));
        assert_eq!(value["best_bid"]["price_in_ticks"], json!(101));
        assert_eq!(value["best_bid"]["raw_sequence_number"], json!(7));
        assert_eq!(
            value["best_bid"]["base_lots"],
            json!({
                "initial": 10,
                "remaining": 4
            })
        );
        assert_eq!(
            value["best_bid"]["flags"],
            json!({
                "raw": (1u8 << 7) | (1u8 << 5),
                "reduce_only": true,
                "is_stop_loss": false,
                "is_stop_loss_direction": false,
                "is_conditional_order": true
            })
        );
        assert_eq!(
            value["best_bid"]["links"],
            json!({
                "prev_node": null,
                "next_node": null
            })
        );
        assert_eq!(
            value["bid_levels"][0],
            json!({
                "side": "bid",
                "price_in_ticks": 101,
                "order_count": 2,
                "total_initial_base_lots": 30,
                "total_remaining_base_lots": 10,
                "first_raw_sequence_number": 7,
                "last_raw_sequence_number": 8
            })
        );
        assert_eq!(value["asks"][0]["side"], json!("ask"));
        assert_eq!(value["asks"][0]["raw_sequence_number"], json!(9));
        assert!(value["bids"][0].get("raw").is_some());
    }

    fn sequence_number(sequence_number: u64) -> SequenceNumber {
        SequenceNumber {
            sequence_number,
            last_update_slot: 1,
        }
    }

    fn orderbook_entry(
        side: OrderbookSide,
        price_in_ticks: u64,
        raw_sequence_number: u64,
        initial_trade_size: u64,
        num_base_lots_remaining: u64,
    ) -> OrderbookEntry {
        let order_sequence_number = match side {
            OrderbookSide::Bid => !raw_sequence_number,
            OrderbookSide::Ask => raw_sequence_number,
        };
        OrderbookEntry {
            order_id: super::super::internal::FifoOrderId {
                price_in_ticks: Ticks::new(price_in_ticks),
                order_sequence_number,
            },
            order: super::super::internal::OrderbookRestingOrder {
                trader_position_id: TraderPositionId {
                    trader_id: Some(42),
                    asset_id: 0,
                },
                initial_trade_size,
                num_base_lots_remaining,
                order_flags: (1 << 7) | (1 << 5),
                optional_conditional_order_index: Some(3),
                expiration_offset: Some(5),
                initial_slot: 100,
                prev_node: None,
                next_node: None,
                last_valid_slot: Some(105),
                reduce_only: true,
                is_stop_loss: false,
                is_stop_loss_direction: false,
                is_conditional_order: true,
            },
        }
    }
}
