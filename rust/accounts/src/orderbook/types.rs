use core::num::NonZeroU32;

use bytemuck::{Pod, Zeroable};
pub use phoenix_rise_math::Side;

/// Pod-compatible optional non-zero `u32`.
#[repr(C)]
#[derive(Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Pod, Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OptionalNonZeroU32 {
    value: u32,
}

impl OptionalNonZeroU32 {
    #[inline(always)]
    pub const fn null() -> Self {
        Self { value: 0 }
    }

    #[inline(always)]
    pub const fn new(value: u32) -> Self {
        Self { value }
    }

    #[inline(always)]
    pub const fn is_none(&self) -> bool {
        self.value == 0
    }

    #[inline(always)]
    pub const fn is_some(&self) -> bool {
        self.value != 0
    }

    #[inline(always)]
    pub fn get(&self) -> Option<NonZeroU32> {
        NonZeroU32::new(self.value)
    }

    #[inline(always)]
    pub fn map<T, F: FnOnce(NonZeroU32) -> T>(&self, f: F) -> Option<T> {
        self.get().map(f)
    }

    #[inline(always)]
    pub const fn raw(&self) -> u32 {
        self.value
    }
}

impl From<Option<u32>> for OptionalNonZeroU32 {
    fn from(value: Option<u32>) -> Self {
        value.map(Self::new).unwrap_or_else(Self::null)
    }
}

impl core::fmt::Debug for OptionalNonZeroU32 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.get() {
            Some(value) => write!(f, "Some({value})"),
            None => write!(f, "None"),
        }
    }
}

/// Type-safe wrapper for sokoban node indices.
#[repr(C)]
#[derive(Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Pod, Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NodePointer(OptionalNonZeroU32);

impl NodePointer {
    #[inline(always)]
    pub const fn null() -> Self {
        Self(OptionalNonZeroU32::null())
    }

    #[inline(always)]
    pub const fn new(ptr: u32) -> Self {
        Self(OptionalNonZeroU32::new(ptr))
    }

    #[inline(always)]
    pub const fn is_null(&self) -> bool {
        self.0.is_none()
    }

    #[inline(always)]
    pub fn as_u32(&self) -> Option<NonZeroU32> {
        self.0.get()
    }

    #[inline(always)]
    pub fn as_u32_checked(&self) -> Option<u32> {
        self.0.map(NonZeroU32::get)
    }

    #[inline(always)]
    pub fn to_u32_or(&self, default: u32) -> u32 {
        self.as_u32_checked().unwrap_or(default)
    }

    #[inline(always)]
    pub fn to_u32_or_sentinel(&self) -> u32 {
        self.to_u32_or(0)
    }
}

impl From<u32> for NodePointer {
    fn from(ptr: u32) -> Self {
        Self::new(ptr)
    }
}

impl From<Option<u32>> for NodePointer {
    fn from(ptr: Option<u32>) -> Self {
        Self(ptr.into())
    }
}

impl core::fmt::Debug for NodePointer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "NodePointer({})", self.to_u32_or_sentinel())
    }
}

/// Key for trader positions in active-trader/orderbook state.
#[repr(C)]
#[derive(Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Pod, Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TraderPositionId {
    trader_id: NodePointer,
    asset_id: u32,
}

impl TraderPositionId {
    #[inline(always)]
    pub const fn new(trader_id: NodePointer, asset_id: u32) -> Self {
        Self {
            trader_id,
            asset_id,
        }
    }

    #[inline(always)]
    pub const fn is_uninitialized(&self) -> bool {
        self.trader_id.is_null()
    }

    #[inline(always)]
    pub const fn trader_id(&self) -> NodePointer {
        self.trader_id
    }

    #[inline(always)]
    pub const fn asset_id(&self) -> u32 {
        self.asset_id
    }
}

impl core::fmt::Debug for TraderPositionId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TraderPositionId")
            .field("trader_id", &self.trader_id())
            .field("asset_id", &self.asset_id())
            .finish()
    }
}

const_assert_eq!(core::mem::size_of::<OptionalNonZeroU32>(), 4);
const_assert_eq!(core::mem::size_of::<NodePointer>(), 4);
const_assert_eq!(core::mem::size_of::<TraderPositionId>(), 8);
