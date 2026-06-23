use super::discriminants::ACCOUNT_DISCRIMINANTS;
use super::internal::{
    ORDERBOOK_CAPACITY, OrderbookEntry, Reader, read_fifo_order_id, read_orderbook_resting_order,
    verify_discriminant,
};
use super::orderbook_header::{
    ORDERBOOK_HEADER_SIZE, OrderbookHeader, read_orderbook_header_after_discriminant,
};
use super::{AccountDeserialize, AccountDeserializeError};

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

impl AccountDeserialize for Orderbook {
    fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        verify_discriminant(ACCOUNT, data, ACCOUNT_DISCRIMINANTS.orderbook)?;
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
