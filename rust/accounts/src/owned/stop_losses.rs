use serde::Serialize;
use solana_pubkey::Pubkey;

use super::internal::SequenceNumber;
use super::{AccountDeserialize, AccountDeserializeError};
use crate::stop_losses as borrowed;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum StopLossDirection {
    GreaterThan,
    LessThan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum StopLossTradeSide {
    Bid,
    Ask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum StopLossOrderKind {
    IOC,
    Limit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
        Ok(borrowed::StopLosses::try_from_account_bytes(data)?.into())
    }
}

impl From<&borrowed::StopLosses> for StopLosses {
    fn from(value: &borrowed::StopLosses) -> Self {
        Self {
            stop_losses: value.stop_losses().map(Into::into),
            is_initialized: value.is_initialized(),
            sequence_number: value.sequence_number().into(),
            funding_key: Pubkey::new_from_array(value.funding_key()),
            trader_key: Pubkey::new_from_array(value.trader_key()),
            asset_id: u64::from(value.asset_id()),
        }
    }
}

impl From<borrowed::StopLosses> for StopLosses {
    fn from(value: borrowed::StopLosses) -> Self {
        Self::from(&value)
    }
}

impl From<borrowed::StopLoss> for StopLoss {
    fn from(value: borrowed::StopLoss) -> Self {
        Self {
            sequence_number: value.sequence_number(),
            trigger_price: value.trigger_price(),
            execution_price: value.execution_price(),
            trade_size: value.trade_size(),
            slot: value.slot(),
            position_sequence_number: value.position_sequence_number(),
            is_active: value.is_active(),
            execution_direction: value.trigger_direction().into(),
            trade_side: value.trade_side().into(),
            order_kind: value.order_kind().into(),
        }
    }
}

impl From<borrowed::TriggerDirection> for StopLossDirection {
    fn from(value: borrowed::TriggerDirection) -> Self {
        match value {
            borrowed::TriggerDirection::LessThan => Self::LessThan,
            borrowed::TriggerDirection::GreaterThan => Self::GreaterThan,
        }
    }
}

impl From<borrowed::TradeSide> for StopLossTradeSide {
    fn from(value: borrowed::TradeSide) -> Self {
        match value {
            borrowed::TradeSide::Ask => Self::Ask,
            borrowed::TradeSide::Bid => Self::Bid,
        }
    }
}

impl From<borrowed::StopLossTriggerOrderKind> for StopLossOrderKind {
    fn from(value: borrowed::StopLossTriggerOrderKind) -> Self {
        match value {
            borrowed::StopLossTriggerOrderKind::Ioc => Self::IOC,
            borrowed::StopLossTriggerOrderKind::Limit => Self::Limit,
        }
    }
}

impl StopLosses {
    pub fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        <Self as AccountDeserialize>::try_from_account_bytes(data)
    }
}
