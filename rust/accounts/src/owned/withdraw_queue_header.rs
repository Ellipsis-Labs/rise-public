use phoenix_rise_math::QuoteLots;
use serde::Serialize;

use super::internal::SequenceNumber;
use super::{AccountDeserialize, AccountDeserializeError};
use crate::withdraw_queue as borrowed;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct WithdrawThrottle {
    pub max_budget: QuoteLots,
    pub remaining_budget: QuoteLots,
    pub replenish_amount_per_slot: QuoteLots,
    pub last_update_slot: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct WithdrawQueueHeader {
    pub sequence_number: SequenceNumber,
    pub withdraw_throttle: WithdrawThrottle,
    pub total_queued_amount: QuoteLots,
    pub withdrawal_fee: QuoteLots,
    pub enqueuing_fee: QuoteLots,
}

impl AccountDeserialize for WithdrawQueueHeader {
    fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        Ok(borrowed::WithdrawQueueHeader::try_from_account_bytes(data)?.into())
    }
}

impl From<&borrowed::WithdrawQueueHeader> for WithdrawQueueHeader {
    fn from(value: &borrowed::WithdrawQueueHeader) -> Self {
        Self {
            sequence_number: value.sequence_number().into(),
            withdraw_throttle: value.withdraw_throttle().into(),
            total_queued_amount: value.total_queued_amount(),
            withdrawal_fee: value.withdrawal_fee(),
            enqueuing_fee: value.enqueueing_fee(),
        }
    }
}

impl From<borrowed::WithdrawQueueHeader> for WithdrawQueueHeader {
    fn from(value: borrowed::WithdrawQueueHeader) -> Self {
        Self::from(&value)
    }
}

impl From<borrowed::WithdrawThrottle> for WithdrawThrottle {
    fn from(value: borrowed::WithdrawThrottle) -> Self {
        Self {
            max_budget: value.max_budget(),
            remaining_budget: value.remaining_budget(),
            replenish_amount_per_slot: value.replenish_amount_per_slot(),
            last_update_slot: value.last_update_slot(),
        }
    }
}

impl WithdrawQueueHeader {
    pub fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        <Self as AccountDeserialize>::try_from_account_bytes(data)
    }
}
