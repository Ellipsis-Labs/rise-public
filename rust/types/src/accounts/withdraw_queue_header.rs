use super::discriminants::ACCOUNT_DISCRIMINANTS;
use super::internal::{Reader, SequenceNumber, read_sequence_number, verify_discriminant};
use super::{AccountDeserialize, AccountDeserializeError};

const ACCOUNT: &str = "WithdrawQueueHeader";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WithdrawThrottle {
    pub max_budget: u64,
    pub remaining_budget: u64,
    pub replenish_amount_per_slot: u64,
    pub last_update_slot: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WithdrawQueueHeader {
    pub sequence_number: SequenceNumber,
    pub withdraw_throttle: WithdrawThrottle,
    pub total_queued_amount: u64,
    pub withdrawal_fee: u64,
    pub enqueuing_fee: u64,
}

impl AccountDeserialize for WithdrawQueueHeader {
    fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        verify_discriminant(ACCOUNT, data, ACCOUNT_DISCRIMINANTS.withdraw_queue_header)?;
        let mut reader = Reader::with_offset(ACCOUNT, data, 8);
        let sequence_number = read_sequence_number(&mut reader)?;
        let withdraw_throttle = WithdrawThrottle {
            max_budget: reader.read_u64()?,
            remaining_budget: reader.read_u64()?,
            replenish_amount_per_slot: reader.read_u64()?,
            last_update_slot: reader.read_u64()?,
        };
        let total_queued_amount = reader.read_u64()?;
        let withdrawal_fee = reader.read_u64()?;
        let enqueuing_fee = reader.read_u64()?;
        reader.skip(8 * 5)?;
        Ok(Self {
            sequence_number,
            withdraw_throttle,
            total_queued_amount,
            withdrawal_fee,
            enqueuing_fee,
        })
    }
}

impl WithdrawQueueHeader {
    pub fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        <Self as AccountDeserialize>::try_from_account_bytes(data)
    }
}
