//! Borrowed views for Phoenix multi-arena header accounts.

use bytemuck::{Pod, Zeroable};
#[cfg(feature = "serde")]
use serde::ser::SerializeStruct;

use super::common::{
    PhoenixAccountDecodeError, SequenceNumber, read_pod, read_prefix, require_len,
    verify_discriminant,
};

const MULTI_ARENA_HEADER_LEN: usize = core::mem::size_of::<MultiArenaHeaderView>();
const SUPERBLOCK_LEN: usize = core::mem::size_of::<SuperblockView>();
const MULTI_ARENA_PREFIX_LEN: usize = MULTI_ARENA_HEADER_LEN + SUPERBLOCK_LEN;

const_assert_eq!(core::mem::size_of::<MultiArenaHeaderView>(), 48);
const_assert_eq!(core::mem::size_of::<SuperblockView>(), 32);

/// Fixed header prefix shared by Phoenix multi-arena accounts such as the
/// global trader index and active trader buffer.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct MultiArenaHeaderView {
    discriminant: u64,
    sequence_number: SequenceNumber,
    num_additional_nodes: u32,
    _padding0: [u8; 4],
    _padding1: [u8; 16],
}

impl MultiArenaHeaderView {
    #[inline(always)]
    pub const fn sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }

    #[inline(always)]
    pub const fn num_additional_nodes(&self) -> u32 {
        self.num_additional_nodes
    }
}

/// Sokoban superblock prefix embedded immediately after the multi-arena
/// header. `num_arenas` is the number of account infos to pass for the
/// multi-arena account group, including the header account.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SuperblockView {
    size: u32,
    num_arenas: u16,
    num_active_arenas: u16,
    num_nodes_per_arena: u32,
    bump_index: u32,
    free_list_head: u32,
    _padding: [u32; 3],
}

impl SuperblockView {
    #[inline(always)]
    pub const fn size(&self) -> u32 {
        self.size
    }

    #[inline(always)]
    pub const fn num_arenas(&self) -> u16 {
        self.num_arenas
    }

    #[inline(always)]
    pub const fn num_active_arenas(&self) -> u16 {
        self.num_active_arenas
    }

    #[inline(always)]
    pub const fn num_nodes_per_arena(&self) -> u32 {
        self.num_nodes_per_arena
    }

    #[inline(always)]
    pub const fn bump_index(&self) -> u32 {
        self.bump_index
    }

    #[inline(always)]
    pub const fn free_list_head(&self) -> u32 {
        self.free_list_head
    }
}

/// View over the fixed multi-arena header and superblock prefix.
#[derive(Clone, Copy, Debug)]
pub struct MultiArenaHeader {
    header: MultiArenaHeaderView,
    superblock: SuperblockView,
}

impl MultiArenaHeader {
    pub fn try_from_account_bytes(
        account: &'static str,
        data: &[u8],
        expected_discriminant: [u8; 8],
    ) -> Result<Self, PhoenixAccountDecodeError> {
        require_len(account, data, MULTI_ARENA_PREFIX_LEN)?;
        verify_discriminant(account, data, expected_discriminant)?;
        let header = read_prefix::<MultiArenaHeaderView>(account, data)?;
        let superblock = data
            .get(MULTI_ARENA_HEADER_LEN..MULTI_ARENA_PREFIX_LEN)
            .ok_or(PhoenixAccountDecodeError::AccountTooSmall {
                account,
                expected: MULTI_ARENA_PREFIX_LEN,
                actual: data.len(),
            })
            .and_then(|bytes| read_pod::<SuperblockView>(account, bytes))?;
        Ok(Self { header, superblock })
    }

    #[inline(always)]
    pub const fn header(&self) -> &MultiArenaHeaderView {
        &self.header
    }

    #[inline(always)]
    pub const fn superblock(&self) -> &SuperblockView {
        &self.superblock
    }

    #[inline(always)]
    pub const fn sequence_number(&self) -> SequenceNumber {
        self.header.sequence_number()
    }

    #[inline(always)]
    pub const fn num_additional_nodes(&self) -> u32 {
        self.header.num_additional_nodes()
    }

    #[inline(always)]
    pub const fn num_arenas(&self) -> u16 {
        self.superblock.num_arenas()
    }

    #[inline(always)]
    pub const fn account_count(&self) -> usize {
        self.superblock.num_arenas() as usize
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for MultiArenaHeaderView {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("MultiArenaHeaderView", 2)?;
        state.serialize_field("sequence_number", &self.sequence_number())?;
        state.serialize_field("num_additional_nodes", &self.num_additional_nodes())?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for SuperblockView {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("SuperblockView", 6)?;
        state.serialize_field("size", &self.size())?;
        state.serialize_field("num_arenas", &self.num_arenas())?;
        state.serialize_field("num_active_arenas", &self.num_active_arenas())?;
        state.serialize_field("num_nodes_per_arena", &self.num_nodes_per_arena())?;
        state.serialize_field("bump_index", &self.bump_index())?;
        state.serialize_field("free_list_head", &self.free_list_head())?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for MultiArenaHeader {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("MultiArenaHeader", 5)?;
        state.serialize_field("header", self.header())?;
        state.serialize_field("superblock", self.superblock())?;
        state.serialize_field("sequence_number", &self.sequence_number())?;
        state.serialize_field("num_additional_nodes", &self.num_additional_nodes())?;
        state.serialize_field("account_count", &self.account_count())?;
        state.end()
    }
}
