//! Immutable access to active traders in the Global Trader Index (GTI).

use solana_pubkey::Pubkey;

use crate::common::PhoenixAccountDecodeError;
use crate::discriminants::PhoenixAccount;
use crate::multi_arena::MultiArenaHeader;
use crate::multi_arena_tree::MultiArenaTree;
use crate::trader::TraderState;

/// Borrowed view of the GTI header account and its overflow arenas.
///
/// Values are copied from the borrowed bytes, so unaligned buffers are
/// accepted. Trader indices are one-based node pointers used by the active
/// trader buffer. Resolve an index again after refreshing the account snapshot:
/// removed GTI nodes can be reused for another trader.
#[derive(Debug)]
pub struct GlobalTraderIndex<'a> {
    tree: MultiArenaTree<'a, [u8; 32]>,
}

impl<'a> GlobalTraderIndex<'a> {
    /// Decode complete account buffers in arena-index order, beginning with
    /// the header account. Include every registered arena, including empty
    /// overflow accounts, and retain each account's discriminator and header.
    ///
    /// Validates the tree's pointers and ordering before exposing lookups.
    /// Account owners and addresses must be verified by the caller.
    pub fn try_from_account_buffers(
        buffers: impl IntoIterator<Item = &'a [u8]>,
    ) -> Result<Self, PhoenixAccountDecodeError> {
        Ok(Self {
            tree: MultiArenaTree::try_from_account_buffers(
                buffers,
                PhoenixAccount::GlobalTraderIndexHeader,
                PhoenixAccount::GlobalTraderIndexArenaHeader,
                core::mem::size_of::<TraderState>(),
            )?,
        })
    }

    pub fn header(&self) -> &MultiArenaHeader {
        self.tree.header()
    }

    pub fn len(&self) -> usize {
        self.tree.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn capacity(&self) -> usize {
        self.tree.capacity()
    }

    /// Resolve a trader account pubkey to its current GTI node pointer.
    pub fn index(&self, trader: &Pubkey) -> Option<u32> {
        self.tree.get(&trader.to_bytes()).map(|(index, _)| index)
    }

    pub fn get(&self, trader: &Pubkey) -> Option<TraderState> {
        let (_, bytes) = self.tree.get(&trader.to_bytes())?;
        self.tree.read_value(bytes)
    }

    /// Resolve a live GTI node pointer to its trader account and state.
    ///
    /// Removed and unallocated nodes return none. Pointers are valid for this
    /// snapshot: a later snapshot can reuse an index for a different trader.
    pub fn get_by_index(&self, index: u32) -> Option<(Pubkey, TraderState)> {
        let (key, bytes) = self.tree.get_by_index(index)?;
        self.tree
            .read_value(bytes)
            .map(|state| (Pubkey::new_from_array(key), state))
    }

    pub fn contains(&self, trader: &Pubkey) -> bool {
        self.index(trader).is_some()
    }

    /// Iterate active traders in pubkey order, including the GTI node pointer.
    pub fn iter_with_indices(&self) -> impl Iterator<Item = (u32, Pubkey, TraderState)> + '_ {
        self.tree.iter().filter_map(|(index, key, bytes)| {
            self.tree
                .read_value(bytes)
                .map(|state| (index, Pubkey::new_from_array(key), state))
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = (Pubkey, TraderState)> + '_ {
        self.iter_with_indices().map(|(_, key, state)| (key, state))
    }
}
