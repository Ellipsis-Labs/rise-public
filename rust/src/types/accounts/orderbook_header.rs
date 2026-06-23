use super::discriminants::ACCOUNT_DISCRIMINANTS;
use super::internal::{
    Reader, SequenceNumber, TradeEvent, read_sequence_number, read_trade_event, verify_discriminant,
};
use super::{AccountDeserialize, AccountDeserializeError};

const ACCOUNT: &str = "OrderbookHeader";
pub(crate) const ORDERBOOK_HEADER_SIZE: usize = 5504;
const TRADE_LIST_CAPACITY: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderbookHeader {
    pub market_status: u8,
    pub base_lots_decimals: i8,
    pub sequence_number: SequenceNumber,
    pub asset_id: u32,
    pub asset_symbol: String,
    pub tick_size_in_quote_lots_per_base_lot: u64,
    pub order_sequence_number: SequenceNumber,
    pub trade_sequence_number: SequenceNumber,
    pub default_taker_fee_micro: u32,
    pub default_maker_fee_micro: i32,
    pub total_maker_quote_lot_fees: i64,
    pub total_taker_quote_lot_fees: u64,
    pub trade_list: Vec<TradeEvent>,
}

impl AccountDeserialize for OrderbookHeader {
    fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        verify_discriminant(ACCOUNT, data, ACCOUNT_DISCRIMINANTS.orderbook_header)?;
        let mut reader = Reader::with_offset(ACCOUNT, data, 8);
        read_orderbook_header_after_discriminant(&mut reader)
    }
}

impl OrderbookHeader {
    pub fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        <Self as AccountDeserialize>::try_from_account_bytes(data)
    }
}

pub(crate) fn read_orderbook_header_after_discriminant(
    reader: &mut Reader<'_>,
) -> Result<OrderbookHeader, AccountDeserializeError> {
    let market_status = reader.read_u8()?;
    let base_lots_decimals = reader.read_i8()?;
    reader.skip(6)?;
    let sequence_number = read_sequence_number(reader)?;
    let asset_id = reader.read_u32()?;
    reader.skip(4)?;
    let asset_symbol = reader.read_symbol()?;
    let tick_size_in_quote_lots_per_base_lot = reader.read_u64()?;
    let order_sequence_number = read_sequence_number(reader)?;
    let trade_sequence_number = read_sequence_number(reader)?;
    let default_taker_fee_micro = reader.read_u32()?;
    let default_maker_fee_micro = reader.read_i32()?;
    let total_maker_quote_lot_fees = reader.read_i64()?;
    let total_taker_quote_lot_fees = reader.read_u64()?;
    reader.skip(8 * 32)?;
    let trade_list = read_circ_buf(reader)?;
    Ok(OrderbookHeader {
        market_status,
        base_lots_decimals,
        sequence_number,
        asset_id,
        asset_symbol,
        tick_size_in_quote_lots_per_base_lot,
        order_sequence_number,
        trade_sequence_number,
        default_taker_fee_micro,
        default_maker_fee_micro,
        total_maker_quote_lot_fees,
        total_taker_quote_lot_fees,
        trade_list,
    })
}

fn read_circ_buf(reader: &mut Reader<'_>) -> Result<Vec<TradeEvent>, AccountDeserializeError> {
    let ptr = reader.read_u32()? as usize;
    let len = reader.read_u32()? as usize;
    if len > TRADE_LIST_CAPACITY {
        return Err(AccountDeserializeError::invalid_data(
            ACCOUNT,
            format!("trade list length {len} exceeds capacity {TRADE_LIST_CAPACITY}"),
        ));
    }
    let raw = {
        let mut raw = Vec::with_capacity(TRADE_LIST_CAPACITY);
        for _ in 0..TRADE_LIST_CAPACITY {
            raw.push(read_trade_event(reader)?);
        }
        raw
    };
    let mut ordered = Vec::with_capacity(len);
    for i in 0..len {
        ordered.push(raw[(ptr + i) % TRADE_LIST_CAPACITY].clone());
    }
    Ok(ordered)
}
