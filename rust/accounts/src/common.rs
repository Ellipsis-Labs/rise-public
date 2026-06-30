//! Shared account deserialization helpers.

use bytemuck::{Pod, Zeroable};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PhoenixAccountDecodeError {
    #[error("{account} account is too small: expected at least {expected} bytes, got {actual}")]
    AccountTooSmall {
        account: &'static str,
        expected: usize,
        actual: usize,
    },

    #[error("{account} account discriminant mismatch")]
    InvalidDiscriminant {
        account: &'static str,
        expected: [u8; 8],
        actual: [u8; 8],
    },

    #[error("{account} account layout mismatch")]
    InvalidLayout { account: &'static str },

    #[error("{account} account data is invalid: {reason}")]
    InvalidData {
        account: &'static str,
        reason: &'static str,
    },
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SequenceNumber {
    pub sequence_number: u64,
    pub last_update_slot: u64,
}

const_assert_eq!(core::mem::size_of::<SequenceNumber>(), 16);

pub(crate) fn verify_discriminant(
    account: &'static str,
    data: &[u8],
    expected: [u8; 8],
) -> Result<(), PhoenixAccountDecodeError> {
    let actual = data
        .get(..8)
        .ok_or(PhoenixAccountDecodeError::AccountTooSmall {
            account,
            expected: 8,
            actual: data.len(),
        })?;
    let mut actual_discriminant = [0u8; 8];
    actual_discriminant.copy_from_slice(actual);
    if actual_discriminant != expected {
        return Err(PhoenixAccountDecodeError::InvalidDiscriminant {
            account,
            expected,
            actual: actual_discriminant,
        });
    }
    Ok(())
}

pub(crate) fn require_len(
    account: &'static str,
    data: &[u8],
    expected: usize,
) -> Result<(), PhoenixAccountDecodeError> {
    if data.len() < expected {
        return Err(PhoenixAccountDecodeError::AccountTooSmall {
            account,
            expected,
            actual: data.len(),
        });
    }
    Ok(())
}

pub(crate) fn read_pod<T: Pod>(
    account: &'static str,
    data: &[u8],
) -> Result<T, PhoenixAccountDecodeError> {
    bytemuck::try_pod_read_unaligned(data)
        .map_err(|_| PhoenixAccountDecodeError::InvalidLayout { account })
}

pub(crate) fn borrow_pod<'a, T: Pod>(
    account: &'static str,
    data: &'a [u8],
) -> Result<&'a T, PhoenixAccountDecodeError> {
    bytemuck::try_from_bytes(data).map_err(|_| PhoenixAccountDecodeError::InvalidLayout { account })
}

pub(crate) fn borrow_slice<'a, T: Pod>(
    account: &'static str,
    data: &'a [u8],
) -> Result<&'a [T], PhoenixAccountDecodeError> {
    bytemuck::try_cast_slice(data).map_err(|_| PhoenixAccountDecodeError::InvalidLayout { account })
}

pub(crate) fn read_prefix<T: Pod>(
    account: &'static str,
    data: &[u8],
) -> Result<T, PhoenixAccountDecodeError> {
    let len = core::mem::size_of::<T>();
    let prefix = data
        .get(..len)
        .ok_or(PhoenixAccountDecodeError::AccountTooSmall {
            account,
            expected: len,
            actual: data.len(),
        })?;
    read_pod(account, prefix)
}

pub(crate) fn borrow_prefix<'a, T: Pod>(
    account: &'static str,
    data: &'a [u8],
) -> Result<&'a T, PhoenixAccountDecodeError> {
    let len = core::mem::size_of::<T>();
    let prefix = data
        .get(..len)
        .ok_or(PhoenixAccountDecodeError::AccountTooSmall {
            account,
            expected: len,
            actual: data.len(),
        })?;
    borrow_pod(account, prefix)
}

pub(crate) fn read_u64(data: &[u8], offset: usize) -> u64 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&data[offset..offset + 8]);
    u64::from_le_bytes(bytes)
}

pub(crate) fn read_i64(data: &[u8], offset: usize) -> i64 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&data[offset..offset + 8]);
    i64::from_le_bytes(bytes)
}

pub(crate) fn read_i56(data: &[u8], offset: usize) -> i64 {
    let bytes = &data[offset..offset + 7];
    let mut extended = [0u8; 8];
    extended[..7].copy_from_slice(bytes);
    if bytes[6] & 0x80 != 0 {
        extended[7] = u8::MAX;
    }
    i64::from_le_bytes(extended)
}
