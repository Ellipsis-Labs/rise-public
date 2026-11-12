//! Immutable active positions and collateral in the Active Trader Buffer (ATB).

use bytemuck::{Pod, Zeroable};
use phoenix_rise_math::TraderPosition as MathTraderPosition;

use crate::common::{PhoenixAccountDecodeError, read_u64};
use crate::discriminants::PhoenixAccount;
use crate::multi_arena::MultiArenaHeader;
use crate::multi_arena_tree::MultiArenaTree;
use crate::orderbook::NodePointer;
pub use crate::orderbook::TraderPositionId;
use crate::trader::{
    LOCKED_DEPOSIT_ASSET_INDEX, NATIVE_SOL_ASSET_INDEX, TraderLockedDeposit, TraderPosition,
    TraderSpotCollateral,
};

/// A perp position together with its resting-order totals and orderbook heads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ActiveTraderPosition {
    pub position: TraderPosition,
    /// Best ask price in ticks; zero means no ask price.
    pub lowest_ask: u32,
    /// Orderbook node pointer; zero means no asks.
    pub ask_orders_head: u32,
    /// Best bid price in ticks; zero means no bid price.
    pub highest_bid: u32,
    /// Orderbook node pointer; zero means no bids.
    pub bid_orders_head: u32,
    pub total_non_reduce_only_ask_base_lots: u32,
    pub total_reduce_only_ask_base_lots: u32,
    pub total_non_reduce_only_bid_base_lots: u32,
    pub total_reduce_only_bid_base_lots: u32,
}

impl ActiveTraderPosition {
    pub const fn has_open_orders(&self) -> bool {
        self.ask_orders_head != 0 || self.bid_orders_head != 0
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ActiveTraderPositionRaw {
    position: MathTraderPosition,
    lowest_ask: u32,
    ask_orders_head: u32,
    highest_bid: u32,
    bid_orders_head: u32,
    total_non_reduce_only_ask_base_lots: u32,
    total_reduce_only_ask_base_lots: u32,
    total_non_reduce_only_bid_base_lots: u32,
    total_reduce_only_bid_base_lots: u32,
}

const_assert_eq!(core::mem::size_of::<ActiveTraderPositionRaw>(), 64);

impl From<ActiveTraderPositionRaw> for ActiveTraderPosition {
    fn from(raw: ActiveTraderPositionRaw) -> Self {
        Self {
            position: TraderPosition {
                base_lot_position: raw.position.base_lot_position,
                virtual_quote_lot_position: raw.position.virtual_quote_lot_position,
                cumulative_funding_snapshot: raw.position.cumulative_funding_snapshot,
                position_sequence_number: raw.position.position_sequence_number.as_u8(),
                accumulated_funding_for_active_position: raw
                    .position
                    .accumulated_funding_for_active_position
                    .to_signed_quote_lots(),
            },
            lowest_ask: raw.lowest_ask,
            ask_orders_head: raw.ask_orders_head,
            highest_bid: raw.highest_bid,
            bid_orders_head: raw.bid_orders_head,
            total_non_reduce_only_ask_base_lots: raw.total_non_reduce_only_ask_base_lots,
            total_reduce_only_ask_base_lots: raw.total_reduce_only_ask_base_lots,
            total_non_reduce_only_bid_base_lots: raw.total_non_reduce_only_bid_base_lots,
            total_reduce_only_bid_base_lots: raw.total_reduce_only_bid_base_lots,
        }
    }
}

/// Borrowed, immutable view of all ATB arenas.
///
/// Position helpers exclude SOL collateral, SPL collateral, locked deposits,
/// and reserved asset-index bands. Use the corresponding typed helpers for
/// collateral entries. ATB node locations can move; key lookups use the GTI
/// trader pointer and asset index instead of caching an ATB node pointer.
#[derive(Debug)]
pub struct ActiveTraderBuffer<'a> {
    tree: MultiArenaTree<'a, TraderPositionId>,
}

impl<'a> ActiveTraderBuffer<'a> {
    /// Decode full account buffers in arena-index order, with the header
    /// account first and all registered overflow accounts following it.
    /// Accepts unaligned bytes and validates tree pointers and key ordering.
    /// Account owners and addresses must be verified by the caller.
    pub fn try_from_account_buffers(
        buffers: impl IntoIterator<Item = &'a [u8]>,
    ) -> Result<Self, PhoenixAccountDecodeError> {
        let tree = MultiArenaTree::<TraderPositionId>::try_from_account_buffers(
            buffers,
            PhoenixAccount::ActiveTraderBufferHeader,
            PhoenixAccount::ActiveTraderBufferArenaHeader,
            core::mem::size_of::<ActiveTraderPositionRaw>(),
        )?;
        if tree.iter().any(|(_, id, _)| id.is_uninitialized()) {
            return Err(PhoenixAccountDecodeError::InvalidData {
                account: "ActiveTraderBuffer",
                reason: "entry has a null trader node pointer",
            });
        }
        Ok(Self { tree })
    }

    pub fn header(&self) -> &MultiArenaHeader {
        self.tree.header()
    }

    /// Number of all entries, including collateral and reserved extensions.
    /// This can exceed the combined length of the typed iterators.
    pub fn len(&self) -> usize {
        self.tree.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn capacity(&self) -> usize {
        self.tree.capacity()
    }

    fn position(&self, bytes: &[u8]) -> Option<ActiveTraderPosition> {
        self.tree
            .read_value::<ActiveTraderPositionRaw>(bytes)
            .map(Into::into)
    }

    pub fn get_position(&self, id: &TraderPositionId) -> Option<ActiveTraderPosition> {
        if !id.is_position() {
            return None;
        }
        let (_, bytes) = self.tree.get(id)?;
        self.position(bytes)
    }

    pub fn iter_all_positions(
        &self,
    ) -> impl Iterator<Item = (TraderPositionId, ActiveTraderPosition)> + '_ {
        self.tree
            .iter()
            .filter(|(_, id, _)| id.is_position())
            .filter_map(|(_, id, bytes)| self.position(bytes).map(|position| (id, position)))
    }

    pub fn iter_positions(
        &self,
        trader_id: u32,
    ) -> impl Iterator<Item = (TraderPositionId, ActiveTraderPosition)> + '_ {
        self.iter_all_positions()
            .filter(move |(id, _)| id.trader_id().to_u32_or_sentinel() == trader_id)
    }

    /// Every entry key for a trader, including reserved or unsupported bands.
    /// Use this when checking whether the typed readers cover a whole snapshot.
    pub fn iter_entry_ids(&self, trader_id: u32) -> impl Iterator<Item = TraderPositionId> + '_ {
        self.tree
            .iter()
            .filter(move |(_, id, _)| id.trader_id().to_u32_or_sentinel() == trader_id)
            .map(|(_, id, _)| id)
    }

    fn spot_collateral(asset_index: u32, bytes: &[u8]) -> TraderSpotCollateral {
        let cap = read_u64(bytes, 8);
        TraderSpotCollateral {
            asset_index,
            balance: read_u64(bytes, 0),
            max_per_trader_balance_override: (cap != 0).then_some(cap),
        }
    }

    /// Native SOL collateral, in lamports, including an optional balance cap.
    pub fn get_native_sol_collateral(&self, trader_id: u32) -> Option<TraderSpotCollateral> {
        self.get_spot_collateral(trader_id, NATIVE_SOL_ASSET_INDEX)
    }

    /// Native SOL or SPL spot collateral. Other asset-index bands return none.
    pub fn get_spot_collateral(
        &self,
        trader_id: u32,
        asset_index: u32,
    ) -> Option<TraderSpotCollateral> {
        if asset_index != NATIVE_SOL_ASSET_INDEX && asset_index & 0xFFFF_0000 != 0xFFFE_0000 {
            return None;
        }
        let (_, bytes) = self.tree.get(&TraderPositionId::new(
            NodePointer::new(trader_id),
            asset_index,
        ))?;
        Some(Self::spot_collateral(asset_index, bytes))
    }

    /// SPL collateral entries for a trader, excluding native SOL.
    pub fn iter_spot_collaterals(
        &self,
        trader_id: u32,
    ) -> impl Iterator<Item = TraderSpotCollateral> + '_ {
        self.tree
            .iter()
            .filter(move |(_, id, _)| {
                id.trader_id().to_u32_or_sentinel() == trader_id
                    && id.asset_id() & 0xFFFF_0000 == 0xFFFE_0000
            })
            .map(|(_, id, bytes)| Self::spot_collateral(id.asset_id(), bytes))
    }

    pub fn get_locked_deposit(&self, trader_id: u32) -> Option<TraderLockedDeposit> {
        let (_, bytes) = self.tree.get(&TraderPositionId::new(
            NodePointer::new(trader_id),
            LOCKED_DEPOSIT_ASSET_INDEX,
        ))?;
        Some(TraderLockedDeposit {
            locked_quote_lots: read_u64(bytes, 0),
            consumed_quote_lots: read_u64(bytes, 8),
            is_isolated: read_u64(bytes, 16) & 1 != 0,
        })
    }
}
