//! Borrowed views for Phoenix withdraw queue accounts.

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

const WITHDRAW_QUEUE_ACCOUNT: &str = "WithdrawQueue";
pub const WITHDRAW_QUEUE_MAX_SIZE: usize = 2048;

const WITHDRAW_QUEUE_NODE_LEN: usize = core::mem::size_of::<WithdrawQueueNode>();
const WITHDRAW_QUEUE_PREFIX_LEN: usize = core::mem::size_of::<WithdrawQueuePrefix>();
const WITHDRAW_QUEUE_LEN: usize =
    WITHDRAW_QUEUE_PREFIX_LEN + (WITHDRAW_QUEUE_NODE_LEN * WITHDRAW_QUEUE_MAX_SIZE);

const_assert_eq!(core::mem::size_of::<WithdrawThrottle>(), 32);
const_assert_eq!(core::mem::size_of::<WithdrawQueueHeader>(), 120);
const_assert_eq!(core::mem::size_of::<WithdrawRequest>(), 96);
const_assert_eq!(core::mem::size_of::<WithdrawQueueNode>(), 104);
const_assert_eq!(core::mem::size_of::<DequeHeader>(), 8);
const_assert_eq!(core::mem::size_of::<NodeAllocatorHeader>(), 16);
const_assert_eq!(core::mem::size_of::<WithdrawQueuePrefix>(), 144);
const_assert_eq!(WITHDRAW_QUEUE_LEN, 213136);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum WithdrawRequestState {
    Pending    = 0,
    Queued     = 1,
    Processing = 2,
    Waiting    = 3,
    Processed  = 4,
    Rejected   = 5,
    Dropped    = 6,
    Canceled   = 7,
}

impl WithdrawRequestState {
    #[inline(always)]
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Pending),
            1 => Some(Self::Queued),
            2 => Some(Self::Processing),
            3 => Some(Self::Waiting),
            4 => Some(Self::Processed),
            5 => Some(Self::Rejected),
            6 => Some(Self::Dropped),
            7 => Some(Self::Canceled),
            _ => None,
        }
    }

    #[inline(always)]
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Processed | Self::Rejected | Self::Dropped | Self::Canceled
        )
    }

    #[inline(always)]
    pub const fn is_active(self) -> bool {
        matches!(
            self,
            Self::Pending | Self::Queued | Self::Processing | Self::Waiting
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum WithdrawTransitionReason {
    ImmediateProcessing      = 0,
    InsufficientBudget       = 1,
    QueueOrdering            = 2,
    CrankProcessing          = 3,
    Success                  = 4,
    RequestTooLarge          = 5,
    InsufficientMargin       = 6,
    InvalidTraderState       = 7,
    ExchangeInactive         = 8,
    InsufficientVaultBalance = 9,
    QueueFull                = 10,
    UserCancellation         = 11,
    TemporaryBudgetShortage  = 12,
    ValidationFailed         = 13,
    CapabilityRevoked        = 14,
}

impl WithdrawTransitionReason {
    #[inline(always)]
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::ImmediateProcessing),
            1 => Some(Self::InsufficientBudget),
            2 => Some(Self::QueueOrdering),
            3 => Some(Self::CrankProcessing),
            4 => Some(Self::Success),
            5 => Some(Self::RequestTooLarge),
            6 => Some(Self::InsufficientMargin),
            7 => Some(Self::InvalidTraderState),
            8 => Some(Self::ExchangeInactive),
            9 => Some(Self::InsufficientVaultBalance),
            10 => Some(Self::QueueFull),
            11 => Some(Self::UserCancellation),
            12 => Some(Self::TemporaryBudgetShortage),
            13 => Some(Self::ValidationFailed),
            14 => Some(Self::CapabilityRevoked),
            _ => None,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct WithdrawThrottle {
    max_budget: u64,
    remaining_budget: u64,
    replenish_amount_per_slot: u64,
    last_update_slot: u64,
}

impl WithdrawThrottle {
    #[inline(always)]
    pub const fn max_budget(&self) -> u64 {
        self.max_budget
    }

    #[inline(always)]
    pub const fn remaining_budget(&self) -> u64 {
        self.remaining_budget
    }

    #[inline(always)]
    pub const fn replenish_amount_per_slot(&self) -> u64 {
        self.replenish_amount_per_slot
    }

    #[inline(always)]
    pub const fn last_update_slot(&self) -> u64 {
        self.last_update_slot
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct WithdrawQueueHeader {
    discriminant: u64,
    sequence_number: SequenceNumber,
    withdraw_throttle: WithdrawThrottle,
    total_queued_amount: u64,
    withdrawal_fee: u64,
    enqueueing_fee: u64,
    _reserved: [u64; 5],
}

impl WithdrawQueueHeader {
    pub fn try_from_account_bytes(data: &[u8]) -> Result<Self, PhoenixAccountDecodeError> {
        require_len(
            WITHDRAW_QUEUE_ACCOUNT,
            data,
            core::mem::size_of::<WithdrawQueueHeader>(),
        )?;
        verify_discriminant(
            WITHDRAW_QUEUE_ACCOUNT,
            data,
            PhoenixAccount::WithdrawQueueHeader.discriminant(),
        )?;
        read_prefix(WITHDRAW_QUEUE_ACCOUNT, data)
    }

    #[inline(always)]
    pub const fn sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }

    #[inline(always)]
    pub const fn withdraw_throttle(&self) -> WithdrawThrottle {
        self.withdraw_throttle
    }

    #[inline(always)]
    pub const fn total_queued_amount(&self) -> u64 {
        self.total_queued_amount
    }

    #[inline(always)]
    pub const fn withdrawal_fee(&self) -> u64 {
        self.withdrawal_fee
    }

    #[inline(always)]
    pub const fn enqueueing_fee(&self) -> u64 {
        self.enqueueing_fee
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct WithdrawRequest {
    trader_key: [u8; 32],
    wallet_key: [u8; 32],
    amount: u64,
    submission_slot: u64,
    last_update_slot: u64,
    transition_count: u16,
    state: u8,
    last_transition_reason: u8,
    _reserved: [u8; 4],
}

impl WithdrawRequest {
    #[inline(always)]
    pub const fn trader_key(&self) -> [u8; 32] {
        self.trader_key
    }

    #[inline(always)]
    pub const fn wallet_key(&self) -> [u8; 32] {
        self.wallet_key
    }

    #[inline(always)]
    pub const fn amount(&self) -> u64 {
        self.amount
    }

    #[inline(always)]
    pub const fn submission_slot(&self) -> u64 {
        self.submission_slot
    }

    #[inline(always)]
    pub const fn last_update_slot(&self) -> u64 {
        self.last_update_slot
    }

    #[inline(always)]
    pub const fn transition_count(&self) -> u16 {
        self.transition_count
    }

    #[inline(always)]
    pub const fn raw_state(&self) -> u8 {
        self.state
    }

    #[inline(always)]
    pub const fn state(&self) -> Option<WithdrawRequestState> {
        WithdrawRequestState::from_u8(self.state)
    }

    #[inline(always)]
    pub const fn raw_last_transition_reason(&self) -> u8 {
        self.last_transition_reason
    }

    #[inline(always)]
    pub const fn last_transition_reason(&self) -> Option<WithdrawTransitionReason> {
        WithdrawTransitionReason::from_u8(self.last_transition_reason)
    }

    #[inline(always)]
    pub const fn is_terminal(&self) -> bool {
        match self.state() {
            Some(state) => state.is_terminal(),
            None => false,
        }
    }

    #[inline(always)]
    pub const fn is_active(&self) -> bool {
        match self.state() {
            Some(state) => state.is_active(),
            None => false,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct WithdrawQueueNode {
    prev: u32,
    next: u32,
    request: WithdrawRequest,
}

impl WithdrawQueueNode {
    #[inline(always)]
    pub const fn prev_node_index(&self) -> Option<u32> {
        if self.prev == 0 {
            None
        } else {
            Some(self.prev)
        }
    }

    #[inline(always)]
    pub const fn next_node_index(&self) -> Option<u32> {
        if self.next == 0 {
            None
        } else {
            Some(self.next)
        }
    }

    #[inline(always)]
    pub const fn request(&self) -> &WithdrawRequest {
        &self.request
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
struct DequeHeader {
    head: u32,
    tail: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
struct NodeAllocatorHeader {
    size: u64,
    bump_index: u32,
    free_list_head: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct WithdrawQueuePrefix {
    header: WithdrawQueueHeader,
    deque_header: DequeHeader,
    allocator_header: NodeAllocatorHeader,
}

/// View over a Phoenix withdraw queue account.
#[derive(Clone, Copy, Debug)]
pub struct WithdrawQueue<'a> {
    prefix: WithdrawQueuePrefix,
    nodes: &'a [u8],
}

impl<'a> WithdrawQueue<'a> {
    pub fn try_from_account_bytes(data: &'a [u8]) -> Result<Self, PhoenixAccountDecodeError> {
        require_len(WITHDRAW_QUEUE_ACCOUNT, data, WITHDRAW_QUEUE_LEN)?;
        verify_discriminant(
            WITHDRAW_QUEUE_ACCOUNT,
            data,
            PhoenixAccount::WithdrawQueueHeader.discriminant(),
        )?;
        let prefix = read_prefix::<WithdrawQueuePrefix>(WITHDRAW_QUEUE_ACCOUNT, data)?;
        if prefix.allocator_header.size > WITHDRAW_QUEUE_MAX_SIZE as u64 {
            return Err(PhoenixAccountDecodeError::InvalidData {
                account: WITHDRAW_QUEUE_ACCOUNT,
                reason: "withdraw queue length exceeds capacity",
            });
        }
        if prefix.allocator_header.bump_index > (WITHDRAW_QUEUE_MAX_SIZE as u32 + 1) {
            return Err(PhoenixAccountDecodeError::InvalidData {
                account: WITHDRAW_QUEUE_ACCOUNT,
                reason: "withdraw queue bump index exceeds capacity",
            });
        }
        let nodes_bytes = data
            .get(WITHDRAW_QUEUE_PREFIX_LEN..WITHDRAW_QUEUE_LEN)
            .ok_or(PhoenixAccountDecodeError::AccountTooSmall {
                account: WITHDRAW_QUEUE_ACCOUNT,
                expected: WITHDRAW_QUEUE_LEN,
                actual: data.len(),
            })?;
        Ok(Self {
            prefix,
            nodes: nodes_bytes,
        })
    }

    #[inline(always)]
    pub const fn header(&self) -> &WithdrawQueueHeader {
        &self.prefix.header
    }

    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.prefix.allocator_header.size as usize
    }

    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        WITHDRAW_QUEUE_MAX_SIZE
    }

    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline(always)]
    pub const fn is_full(&self) -> bool {
        self.len() >= WITHDRAW_QUEUE_MAX_SIZE
    }

    #[inline(always)]
    pub const fn head_node_index(&self) -> Option<u32> {
        if self.prefix.deque_header.head == 0 {
            None
        } else {
            Some(self.prefix.deque_header.head)
        }
    }

    #[inline(always)]
    pub const fn tail_node_index(&self) -> Option<u32> {
        if self.prefix.deque_header.tail == 0 {
            None
        } else {
            Some(self.prefix.deque_header.tail)
        }
    }

    #[inline(always)]
    pub const fn bump_index(&self) -> u32 {
        self.prefix.allocator_header.bump_index
    }

    #[inline(always)]
    pub const fn free_list_head(&self) -> u32 {
        self.prefix.allocator_header.free_list_head
    }

    #[inline(always)]
    pub fn nodes(&self) -> WithdrawQueueNodeIter<'a> {
        WithdrawQueueNodeIter {
            remaining: self.nodes,
        }
    }

    #[inline(always)]
    pub fn iter(&self) -> WithdrawQueueIter<'_, 'a> {
        WithdrawQueueIter {
            queue: self,
            next_index: self.prefix.deque_header.head,
            remaining: self.len(),
        }
    }

    fn node_at(&self, node_index: u32) -> Result<WithdrawQueueNode, PhoenixAccountDecodeError> {
        if node_index == 0 || node_index as usize > WITHDRAW_QUEUE_MAX_SIZE {
            return Err(PhoenixAccountDecodeError::InvalidData {
                account: WITHDRAW_QUEUE_ACCOUNT,
                reason: "withdraw queue node pointer out of bounds",
            });
        }
        let node_index = node_index as usize - 1;
        let offset = node_index.checked_mul(WITHDRAW_QUEUE_NODE_LEN).ok_or(
            PhoenixAccountDecodeError::InvalidData {
                account: WITHDRAW_QUEUE_ACCOUNT,
                reason: "withdraw queue node offset overflow",
            },
        )?;
        let end = offset.checked_add(WITHDRAW_QUEUE_NODE_LEN).ok_or(
            PhoenixAccountDecodeError::InvalidData {
                account: WITHDRAW_QUEUE_ACCOUNT,
                reason: "withdraw queue node end offset overflow",
            },
        )?;
        let bytes = self
            .nodes
            .get(offset..end)
            .ok_or(PhoenixAccountDecodeError::InvalidData {
                account: WITHDRAW_QUEUE_ACCOUNT,
                reason: "withdraw queue node pointer out of bounds",
            })?;
        read_pod(WITHDRAW_QUEUE_ACCOUNT, bytes)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WithdrawQueueEntry {
    pub node_index: u32,
    pub request: WithdrawRequest,
}

pub struct WithdrawQueueIter<'queue, 'data> {
    queue: &'queue WithdrawQueue<'data>,
    next_index: u32,
    remaining: usize,
}

impl Iterator for WithdrawQueueIter<'_, '_> {
    type Item = Result<WithdrawQueueEntry, PhoenixAccountDecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let node_index = self.next_index;
        let node = match self.queue.node_at(node_index) {
            Ok(node) => node,
            Err(error) => {
                self.remaining = 0;
                return Some(Err(error));
            }
        };
        self.next_index = node.next;
        self.remaining -= 1;
        Some(Ok(WithdrawQueueEntry {
            node_index,
            request: *node.request(),
        }))
    }
}

pub struct WithdrawQueueNodeIter<'a> {
    remaining: &'a [u8],
}

impl Iterator for WithdrawQueueNodeIter<'_> {
    type Item = Result<WithdrawQueueNode, PhoenixAccountDecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.remaining.get(..WITHDRAW_QUEUE_NODE_LEN)?;
        self.remaining = &self.remaining[WITHDRAW_QUEUE_NODE_LEN..];
        Some(read_pod(WITHDRAW_QUEUE_ACCOUNT, node))
    }
}

impl ExactSizeIterator for WithdrawQueueNodeIter<'_> {
    fn len(&self) -> usize {
        self.remaining.len() / WITHDRAW_QUEUE_NODE_LEN
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for WithdrawThrottle {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("WithdrawThrottle", 4)?;
        state.serialize_field("max_budget", &self.max_budget())?;
        state.serialize_field("remaining_budget", &self.remaining_budget())?;
        state.serialize_field(
            "replenish_amount_per_slot",
            &self.replenish_amount_per_slot(),
        )?;
        state.serialize_field("last_update_slot", &self.last_update_slot())?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for WithdrawQueueHeader {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("WithdrawQueueHeader", 5)?;
        state.serialize_field("sequence_number", &self.sequence_number())?;
        state.serialize_field("withdraw_throttle", &self.withdraw_throttle())?;
        state.serialize_field("total_queued_amount", &self.total_queued_amount())?;
        state.serialize_field("withdrawal_fee", &self.withdrawal_fee())?;
        state.serialize_field("enqueueing_fee", &self.enqueueing_fee())?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for WithdrawRequest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("WithdrawRequest", 12)?;
        state.serialize_field("trader_key", &pubkey_string(&self.trader_key()))?;
        state.serialize_field("wallet_key", &pubkey_string(&self.wallet_key()))?;
        state.serialize_field("amount", &self.amount())?;
        state.serialize_field("submission_slot", &self.submission_slot())?;
        state.serialize_field("last_update_slot", &self.last_update_slot())?;
        state.serialize_field("transition_count", &self.transition_count())?;
        state.serialize_field("raw_state", &self.raw_state())?;
        state.serialize_field("state", &self.state())?;
        state.serialize_field(
            "raw_last_transition_reason",
            &self.raw_last_transition_reason(),
        )?;
        state.serialize_field("last_transition_reason", &self.last_transition_reason())?;
        state.serialize_field("is_terminal", &self.is_terminal())?;
        state.serialize_field("is_active", &self.is_active())?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for WithdrawQueueNode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("WithdrawQueueNode", 3)?;
        state.serialize_field("prev_node_index", &self.prev_node_index())?;
        state.serialize_field("next_node_index", &self.next_node_index())?;
        state.serialize_field("request", self.request())?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for WithdrawQueueEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("WithdrawQueueEntry", 2)?;
        state.serialize_field("node_index", &self.node_index)?;
        state.serialize_field("request", &self.request)?;
        state.end()
    }
}

#[cfg(feature = "serde")]
struct WithdrawQueueEntries<'a>(&'a WithdrawQueue<'a>);

#[cfg(feature = "serde")]
impl serde::Serialize for WithdrawQueueEntries<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for entry in self.0.iter() {
            let entry = entry.map_err(serde::ser::Error::custom)?;
            seq.serialize_element(&entry)?;
        }
        seq.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for WithdrawQueue<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("WithdrawQueue", 10)?;
        state.serialize_field("header", self.header())?;
        state.serialize_field("len", &self.len())?;
        state.serialize_field("capacity", &self.capacity())?;
        state.serialize_field("is_empty", &self.is_empty())?;
        state.serialize_field("is_full", &self.is_full())?;
        state.serialize_field("head_node_index", &self.head_node_index())?;
        state.serialize_field("tail_node_index", &self.tail_node_index())?;
        state.serialize_field("bump_index", &self.bump_index())?;
        state.serialize_field("free_list_head", &self.free_list_head())?;
        state.serialize_field("entries", &WithdrawQueueEntries(self))?;
        state.end()
    }
}
