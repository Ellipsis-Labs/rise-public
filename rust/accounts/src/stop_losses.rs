//! Borrowed views for Phoenix stop-loss accounts.

use bytemuck::{Pod, Zeroable};
#[cfg(feature = "serde")]
use serde::ser::SerializeStruct;

use super::common::{
    PhoenixAccountDecodeError, SequenceNumber, read_prefix, require_len, verify_discriminant,
};
use super::conditional_orders::TriggerOrderFlags;
use super::discriminants::PhoenixAccount;
#[cfg(feature = "serde")]
use crate::serde_helpers::pubkey_string;

const STOP_LOSSES_ACCOUNT: &str = "StopLosses";
const STOP_LOSSES_LEN: usize = core::mem::size_of::<StopLosses>();

const_assert_eq!(core::mem::size_of::<StopLoss>(), 80);
const_assert_eq!(core::mem::size_of::<StopLosses>(), 328);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TriggerDirection {
    LessThan,
    GreaterThan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TradeSide {
    Ask,
    Bid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StopLossTriggerOrderKind {
    Ioc,
    Limit,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct StopLoss {
    sequence_number: u64,
    trigger_price: u64,
    execution_price: u64,
    trade_size: u64,
    slot: u64,
    position_sequence_number: u8,
    flags: TriggerOrderFlags,
    _padding: [u8; 6],
    _padding2: [u8; 32],
}

impl StopLoss {
    #[inline(always)]
    pub const fn sequence_number(&self) -> u64 {
        self.sequence_number
    }

    #[inline(always)]
    pub const fn trigger_price(&self) -> u64 {
        self.trigger_price
    }

    #[inline(always)]
    pub const fn execution_price(&self) -> u64 {
        self.execution_price
    }

    #[inline(always)]
    pub const fn trade_size(&self) -> u64 {
        self.trade_size
    }

    #[inline(always)]
    pub const fn slot(&self) -> u64 {
        self.slot
    }

    #[inline(always)]
    pub const fn position_sequence_number(&self) -> u8 {
        self.position_sequence_number
    }

    #[inline(always)]
    pub const fn flags(&self) -> TriggerOrderFlags {
        self.flags
    }

    #[inline(always)]
    pub const fn is_active(&self) -> bool {
        self.flags.is_active()
    }

    #[inline(always)]
    pub const fn trigger_direction(&self) -> TriggerDirection {
        if self.flags.is_greater_than_trigger() {
            TriggerDirection::GreaterThan
        } else {
            TriggerDirection::LessThan
        }
    }

    #[inline(always)]
    pub const fn trade_side(&self) -> TradeSide {
        if self.flags.is_bid() {
            TradeSide::Bid
        } else {
            TradeSide::Ask
        }
    }

    #[inline(always)]
    pub const fn order_kind(&self) -> StopLossTriggerOrderKind {
        if self.flags.is_limit_order() {
            StopLossTriggerOrderKind::Limit
        } else {
            StopLossTriggerOrderKind::Ioc
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct StopLosses {
    discriminant: u64,
    stop_losses: [StopLoss; 2],
    initialized: u64,
    sequence_number: SequenceNumber,
    funding_key: [u8; 32],
    trader_key: [u8; 32],
    asset_id: u32,
    _asset_id_padding: [u8; 4],
    _padding: [u64; 8],
}

impl StopLosses {
    pub fn try_from_account_bytes(data: &[u8]) -> Result<Self, PhoenixAccountDecodeError> {
        require_len(STOP_LOSSES_ACCOUNT, data, STOP_LOSSES_LEN)?;
        verify_discriminant(
            STOP_LOSSES_ACCOUNT,
            data,
            PhoenixAccount::StopLosses.discriminant(),
        )?;
        read_prefix(STOP_LOSSES_ACCOUNT, data)
    }

    #[inline(always)]
    pub const fn stop_losses(&self) -> &[StopLoss; 2] {
        &self.stop_losses
    }

    #[inline(always)]
    pub const fn greater_than_stop_loss(&self) -> &StopLoss {
        &self.stop_losses[0]
    }

    #[inline(always)]
    pub const fn less_than_stop_loss(&self) -> &StopLoss {
        &self.stop_losses[1]
    }

    #[inline(always)]
    pub const fn is_initialized(&self) -> bool {
        self.initialized != 0
    }

    #[inline(always)]
    pub const fn sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }

    #[inline(always)]
    pub const fn funding_key(&self) -> [u8; 32] {
        self.funding_key
    }

    #[inline(always)]
    pub const fn trader_key(&self) -> [u8; 32] {
        self.trader_key
    }

    #[inline(always)]
    pub const fn asset_id(&self) -> u32 {
        self.asset_id
    }

    #[inline(always)]
    pub fn validate(
        &self,
        trader_key: &[u8; 32],
        funding_key: Option<&[u8; 32]>,
        asset_id: Option<u32>,
    ) -> bool {
        self.trader_key() == *trader_key
            && funding_key.is_none_or(|expected| self.funding_key() == *expected)
            && asset_id.is_none_or(|expected| self.asset_id == expected)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for StopLoss {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("StopLoss", 11)?;
        state.serialize_field("sequence_number", &self.sequence_number())?;
        state.serialize_field("trigger_price", &self.trigger_price())?;
        state.serialize_field("execution_price", &self.execution_price())?;
        state.serialize_field("trade_size", &self.trade_size())?;
        state.serialize_field("slot", &self.slot())?;
        state.serialize_field("position_sequence_number", &self.position_sequence_number())?;
        state.serialize_field("flags", &self.flags())?;
        state.serialize_field("is_active", &self.is_active())?;
        state.serialize_field("trigger_direction", &self.trigger_direction())?;
        state.serialize_field("trade_side", &self.trade_side())?;
        state.serialize_field("order_kind", &self.order_kind())?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for StopLosses {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("StopLosses", 9)?;
        state.serialize_field("stop_losses", self.stop_losses())?;
        state.serialize_field("greater_than_stop_loss", self.greater_than_stop_loss())?;
        state.serialize_field("less_than_stop_loss", self.less_than_stop_loss())?;
        state.serialize_field("is_initialized", &self.is_initialized())?;
        state.serialize_field("sequence_number", &self.sequence_number())?;
        state.serialize_field("funding_key", &pubkey_string(&self.funding_key()))?;
        state.serialize_field("trader_key", &pubkey_string(&self.trader_key()))?;
        state.serialize_field("asset_id", &self.asset_id())?;
        state.serialize_field("initialized", &self.initialized)?;
        state.end()
    }
}
