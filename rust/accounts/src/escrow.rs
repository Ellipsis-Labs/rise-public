//! Borrowed views for Phoenix escrow accounts.

use bytemuck::{Pod, Zeroable};
#[cfg(feature = "serde")]
use serde::ser::{SerializeSeq, SerializeStruct};

use super::common::{
    PhoenixAccountDecodeError, SequenceNumber, read_pod, read_prefix, require_len,
    verify_discriminant,
};
use super::discriminants::PhoenixAccount;
#[cfg(feature = "serde")]
use crate::serde_helpers::pubkey_string;

const ESCROW_ACCOUNT: &str = "Escrow";
const ESCROW_HEADER_LEN: usize = core::mem::size_of::<EscrowHeader>();
const ESCROW_REQUEST_LEN: usize = core::mem::size_of::<EscrowRequest>();

const_assert_eq!(core::mem::size_of::<EscrowHeader>(), 104);
const_assert_eq!(core::mem::size_of::<EscrowAction>(), 144);
const_assert_eq!(core::mem::size_of::<EscrowRequest>(), 896);

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct EscrowHeader {
    discriminant: u64,
    authority: [u8; 32],
    funder_key: [u8; 32],
    sequence_number: SequenceNumber,
    len: u64,
    capacity: u64,
}

impl EscrowHeader {
    pub fn try_from_account_bytes(data: &[u8]) -> Result<Self, PhoenixAccountDecodeError> {
        require_len(ESCROW_ACCOUNT, data, ESCROW_HEADER_LEN)?;
        verify_discriminant(
            ESCROW_ACCOUNT,
            data,
            PhoenixAccount::EscrowHeader.discriminant(),
        )?;
        read_prefix(ESCROW_ACCOUNT, data)
    }

    #[inline(always)]
    pub const fn authority(&self) -> [u8; 32] {
        self.authority
    }

    #[inline(always)]
    pub const fn funder_key(&self) -> [u8; 32] {
        self.funder_key
    }

    #[inline(always)]
    pub const fn sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }

    #[inline(always)]
    pub const fn len(&self) -> u64 {
        self.len
    }

    #[inline(always)]
    pub const fn capacity(&self) -> u64 {
        self.capacity
    }

    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline(always)]
    pub fn required_account_size(capacity: u64) -> Option<usize> {
        let requests_len = ESCROW_REQUEST_LEN.checked_mul(capacity as usize)?;
        ESCROW_HEADER_LEN.checked_add(requests_len)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EscrowActionKind {
    Noop,
    Cash,
    Unknown(u8),
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct EscrowAction {
    kind: u8,
    _discriminant_padding: [u8; 7],
    amount: u64,
    _padding: [u8; 128],
}

impl EscrowAction {
    #[inline(always)]
    pub const fn raw_kind(&self) -> u8 {
        self.kind
    }

    #[inline(always)]
    pub const fn kind(&self) -> EscrowActionKind {
        match self.kind {
            0 => EscrowActionKind::Noop,
            1 => EscrowActionKind::Cash,
            other => EscrowActionKind::Unknown(other),
        }
    }

    #[inline(always)]
    pub const fn cash_amount(&self) -> Option<u64> {
        match self.kind {
            1 => Some(self.amount),
            _ => None,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct EscrowRequest {
    sender_authority: [u8; 32],
    sender_pda_index: u8,
    sender_subaccount_index: u8,
    receiver_pda_index: u8,
    receiver_subaccount_index: u8,
    expiration_offset: u32,
    sequence_number: u64,
    initial_slot: u64,
    _padding0: [u8; 8],
    _padding1: [u8; 128],
    actions: [EscrowAction; 4],
    _padding2: [u8; 128],
}

impl EscrowRequest {
    #[inline(always)]
    pub const fn sender_authority(&self) -> [u8; 32] {
        self.sender_authority
    }

    #[inline(always)]
    pub const fn sender_pda_index(&self) -> u8 {
        self.sender_pda_index
    }

    #[inline(always)]
    pub const fn sender_subaccount_index(&self) -> u8 {
        self.sender_subaccount_index
    }

    #[inline(always)]
    pub const fn receiver_pda_index(&self) -> u8 {
        self.receiver_pda_index
    }

    #[inline(always)]
    pub const fn receiver_subaccount_index(&self) -> u8 {
        self.receiver_subaccount_index
    }

    #[inline(always)]
    pub const fn expiration_offset(&self) -> Option<u32> {
        if self.expiration_offset == 0 {
            None
        } else {
            Some(self.expiration_offset)
        }
    }

    #[inline(always)]
    pub const fn sequence_number(&self) -> Option<u64> {
        if self.sequence_number == 0 {
            None
        } else {
            Some(self.sequence_number)
        }
    }

    #[inline(always)]
    pub const fn initial_slot(&self) -> u64 {
        self.initial_slot
    }

    #[inline(always)]
    pub const fn actions(&self) -> &[EscrowAction; 4] {
        &self.actions
    }

    #[inline(always)]
    pub const fn is_tombstoned(&self) -> bool {
        self.sequence_number == 0
    }

    #[inline(always)]
    pub const fn is_expired(&self, current_slot: u64) -> bool {
        match self.expiration_offset() {
            Some(offset) => current_slot > self.initial_slot.saturating_add(offset as u64),
            None => false,
        }
    }

    #[inline(always)]
    pub const fn is_active(&self, current_slot: u64) -> bool {
        !self.is_tombstoned() && !self.is_expired(current_slot)
    }
}

/// View over an escrow account and its request slots.
#[derive(Clone, Copy, Debug)]
pub struct Escrow<'a> {
    header: EscrowHeader,
    requests: &'a [u8],
}

impl<'a> Escrow<'a> {
    pub fn try_from_account_bytes(data: &'a [u8]) -> Result<Self, PhoenixAccountDecodeError> {
        let header = EscrowHeader::try_from_account_bytes(data)?;
        if header.len() > header.capacity() {
            return Err(PhoenixAccountDecodeError::InvalidData {
                account: ESCROW_ACCOUNT,
                reason: "escrow length exceeds capacity",
            });
        }

        let capacity = usize::try_from(header.capacity()).map_err(|_| {
            PhoenixAccountDecodeError::InvalidData {
                account: ESCROW_ACCOUNT,
                reason: "escrow capacity exceeds usize",
            }
        })?;
        let requests_len = capacity.checked_mul(ESCROW_REQUEST_LEN).ok_or(
            PhoenixAccountDecodeError::InvalidData {
                account: ESCROW_ACCOUNT,
                reason: "escrow request capacity overflow",
            },
        )?;
        let requests_end = ESCROW_HEADER_LEN.checked_add(requests_len).ok_or(
            PhoenixAccountDecodeError::InvalidData {
                account: ESCROW_ACCOUNT,
                reason: "escrow request end offset overflow",
            },
        )?;
        let request_bytes = data.get(ESCROW_HEADER_LEN..requests_end).ok_or(
            PhoenixAccountDecodeError::AccountTooSmall {
                account: ESCROW_ACCOUNT,
                expected: requests_end,
                actual: data.len(),
            },
        )?;
        Ok(Self {
            header,
            requests: request_bytes,
        })
    }

    #[inline(always)]
    pub const fn header(&self) -> &EscrowHeader {
        &self.header
    }

    #[inline(always)]
    pub fn requests(&self) -> EscrowRequestIter<'a> {
        self.iter()
    }

    #[inline(always)]
    pub const fn request_capacity(&self) -> usize {
        self.requests.len() / ESCROW_REQUEST_LEN
    }

    #[inline(always)]
    pub fn iter(&self) -> EscrowRequestIter<'a> {
        EscrowRequestIter {
            remaining: self.requests,
        }
    }

    pub fn find_request(
        &self,
        sequence_number: u64,
    ) -> Result<Option<EscrowRequest>, PhoenixAccountDecodeError> {
        if sequence_number == 0 {
            return Ok(None);
        }
        for request in self.iter() {
            let request = request?;
            if request.sequence_number() == Some(sequence_number) {
                return Ok(Some(request));
            }
        }
        Ok(None)
    }

    pub fn find_request_by_sender(
        &self,
        sender_authority: &[u8; 32],
    ) -> Result<Option<EscrowRequest>, PhoenixAccountDecodeError> {
        for request in self.iter() {
            let request = request?;
            if !request.is_tombstoned() && request.sender_authority() == *sender_authority {
                return Ok(Some(request));
            }
        }
        Ok(None)
    }
}

pub struct EscrowRequestIter<'a> {
    remaining: &'a [u8],
}

impl Iterator for EscrowRequestIter<'_> {
    type Item = Result<EscrowRequest, PhoenixAccountDecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        let request = self.remaining.get(..ESCROW_REQUEST_LEN)?;
        self.remaining = &self.remaining[ESCROW_REQUEST_LEN..];
        Some(read_pod(ESCROW_ACCOUNT, request))
    }
}

impl ExactSizeIterator for EscrowRequestIter<'_> {
    fn len(&self) -> usize {
        self.remaining.len() / ESCROW_REQUEST_LEN
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for EscrowHeader {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("EscrowHeader", 6)?;
        state.serialize_field("authority", &pubkey_string(&self.authority()))?;
        state.serialize_field("funder_key", &pubkey_string(&self.funder_key()))?;
        state.serialize_field("sequence_number", &self.sequence_number())?;
        state.serialize_field("len", &self.len())?;
        state.serialize_field("capacity", &self.capacity())?;
        state.serialize_field("is_empty", &self.is_empty())?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for EscrowAction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("EscrowAction", 3)?;
        state.serialize_field("raw_kind", &self.raw_kind())?;
        state.serialize_field("kind", &self.kind())?;
        state.serialize_field("cash_amount", &self.cash_amount())?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for EscrowRequest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("EscrowRequest", 10)?;
        state.serialize_field("sender_authority", &pubkey_string(&self.sender_authority()))?;
        state.serialize_field("sender_pda_index", &self.sender_pda_index())?;
        state.serialize_field("sender_subaccount_index", &self.sender_subaccount_index())?;
        state.serialize_field("receiver_pda_index", &self.receiver_pda_index())?;
        state.serialize_field(
            "receiver_subaccount_index",
            &self.receiver_subaccount_index(),
        )?;
        state.serialize_field("expiration_offset", &self.expiration_offset())?;
        state.serialize_field("sequence_number", &self.sequence_number())?;
        state.serialize_field("initial_slot", &self.initial_slot())?;
        state.serialize_field("actions", self.actions())?;
        state.serialize_field("is_tombstoned", &self.is_tombstoned())?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Escrow<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Escrow", 2)?;
        state.serialize_field("header", self.header())?;
        state.serialize_field("requests", &EscrowRequestList(self))?;
        state.end()
    }
}

#[cfg(feature = "serde")]
struct EscrowRequestList<'a>(&'a Escrow<'a>);

#[cfg(feature = "serde")]
impl serde::Serialize for EscrowRequestList<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.request_capacity()))?;
        for request in self.0.iter() {
            seq.serialize_element(&request.map_err(serde::ser::Error::custom)?)?;
        }
        seq.end()
    }
}
