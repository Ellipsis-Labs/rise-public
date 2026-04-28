use solana_pubkey::Pubkey;

use super::discriminants::ACCOUNT_DISCRIMINANTS;
use super::internal::{Reader, SequenceNumber, read_sequence_number, verify_discriminant};
use super::{AccountDeserialize, AccountDeserializeError};

const ACCOUNT: &str = "StopLosses";
const STOP_LOSS_SIZE: usize = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopLossDirection {
    GreaterThan,
    LessThan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopLossTradeSide {
    Bid,
    Ask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopLossOrderKind {
    IOC,
    Limit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StopLoss {
    pub sequence_number: u64,
    pub trigger_price: u64,
    pub execution_price: u64,
    pub trade_size: u64,
    pub slot: u64,
    pub position_sequence_number: u8,
    pub is_active: bool,
    pub execution_direction: StopLossDirection,
    pub trade_side: StopLossTradeSide,
    pub order_kind: StopLossOrderKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopLosses {
    pub stop_losses: [StopLoss; 2],
    pub is_initialized: bool,
    pub sequence_number: SequenceNumber,
    pub funding_key: Pubkey,
    pub trader_key: Pubkey,
    pub asset_id: u64,
}

impl AccountDeserialize for StopLosses {
    fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        verify_discriminant(ACCOUNT, data, ACCOUNT_DISCRIMINANTS.stop_losses)?;
        let mut reader = Reader::with_offset(ACCOUNT, data, 8);
        let greater_than_stop_loss = read_stop_loss(&mut reader)?;
        let less_than_stop_loss = read_stop_loss(&mut reader)?;
        let is_initialized = reader.read_u64()? != 0;
        let sequence_number = read_sequence_number(&mut reader)?;
        let funding_key = reader.read_pubkey()?;
        let trader_key = reader.read_pubkey()?;
        let asset_id = reader.read_u64()?;
        reader.skip(64)?;
        Ok(Self {
            stop_losses: [greater_than_stop_loss, less_than_stop_loss],
            is_initialized,
            sequence_number,
            funding_key,
            trader_key,
            asset_id,
        })
    }
}

impl StopLosses {
    pub fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        <Self as AccountDeserialize>::try_from_account_bytes(data)
    }
}

fn read_stop_loss(reader: &mut Reader<'_>) -> Result<StopLoss, AccountDeserializeError> {
    let start = reader.offset();
    let sequence_number = reader.read_u64()?;
    let trigger_price = reader.read_u64()?;
    let execution_price = reader.read_u64()?;
    let trade_size = reader.read_u64()?;
    let slot = reader.read_u64()?;
    let position_sequence_number = reader.read_u8()?;
    let flags = reader.read_u8()?;
    let consumed = reader.offset() - start;
    reader.skip(STOP_LOSS_SIZE - consumed)?;
    Ok(StopLoss {
        sequence_number,
        trigger_price,
        execution_price,
        trade_size,
        slot,
        position_sequence_number,
        is_active: flags & 0x01 != 0,
        execution_direction: if flags & 0x02 != 0 {
            StopLossDirection::GreaterThan
        } else {
            StopLossDirection::LessThan
        },
        trade_side: if flags & 0x04 != 0 {
            StopLossTradeSide::Bid
        } else {
            StopLossTradeSide::Ask
        },
        order_kind: if flags & 0x08 != 0 {
            StopLossOrderKind::Limit
        } else {
            StopLossOrderKind::IOC
        },
    })
}
