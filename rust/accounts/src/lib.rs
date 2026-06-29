//! Phoenix account deserialization helpers.
//!
//! This crate is the on-chain-friendly account API. Its public account views
//! accept raw account-data byte slices and copy fixed-layout values with
//! unaligned reads so callers do not need Rust struct alignment guarantees.
//! Optional owned account readers are available from [`owned`] when the
//! `serde` feature is enabled.
//!
//! Start with [`perp_asset_map::PerpAssetMap`] for market metadata, mark
//! prices, funding, risk, and spline account references. Use
//! [`trader::TraderHeader`] for fixed trader state and capability flags, then
//! [`trader::TraderPositions`] to iterate the dynamic position map. Trader
//! capability helpers live under [`trader::capabilities`]. PDA helpers are
//! available on-chain and off-chain; borrowed account views themselves only
//! need account-data bytes.

#[cfg(feature = "serde")]
pub mod owned;
#[cfg(feature = "serde")]
mod serde_helpers;

use sha2_const_stable::Sha256;

macro_rules! const_assert_eq {
    ($left:expr, $right:expr $(,)?) => {
        const _: () = assert!($left == $right);
    };
}

pub mod common;
pub mod conditional_orders;
pub mod discriminants;
pub mod escrow;
pub mod global_config;
pub mod multi_arena;
pub mod pda;
pub mod permission;
pub mod perp_asset_map;
pub mod spline_collection;
pub mod stop_losses;
pub mod trader;
pub mod withdraw_queue;

pub use common::PhoenixAccountDecodeError;
pub use discriminants::PhoenixAccount;

/// Explicit broad import surface for account views and account helper types.
pub mod prelude {
    pub use super::common::*;
    pub use super::conditional_orders::*;
    pub use super::discriminants::*;
    pub use super::escrow::*;
    pub use super::global_config::*;
    pub use super::multi_arena::*;
    pub use super::pda::*;
    pub use super::permission::*;
    pub use super::perp_asset_map::*;
    pub use super::spline_collection::*;
    pub use super::stop_losses::*;
    pub use super::trader::capabilities::*;
    pub use super::trader::{
        TraderHeader, TraderPosition, TraderPositionEntry, TraderPositionIter, TraderPositions,
        TraderState,
    };
    pub use super::withdraw_queue::*;
}

pub const fn sha2_const(input: &[u8]) -> u64 {
    let hash = Sha256::new().update(input).finalize();
    u64::from_le_bytes([
        hash[0], hash[1], hash[2], hash[3], hash[4], hash[5], hash[6], hash[7],
    ])
}
