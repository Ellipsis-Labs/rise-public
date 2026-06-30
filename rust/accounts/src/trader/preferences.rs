//! Trader preference flags and access helpers.

use core::fmt::{Debug, Display};

use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{Pod, Zeroable};
#[cfg(feature = "serde")]
use serde::ser::SerializeStruct;

const_assert_eq!(core::mem::size_of::<TraderPreferenceFlags>(), 4);

/// Disables permissionless isolated child-to-parent collateral sweeps.
pub const TRADER_PREFERENCE_DISABLE_COLLATERAL_SWEEP: u32 = 1 << 0;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[borsh(use_discriminant = true)]
#[repr(u8)]
pub enum TraderPreferenceKind {
    DisableCollateralSweep = 0,
}

impl TraderPreferenceKind {
    pub const fn key(self) -> &'static str {
        match self {
            Self::DisableCollateralSweep => "disable_collateral_sweep",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Self::DisableCollateralSweep => {
                "Disable permissionless isolated child-to-parent collateral sweeps."
            }
        }
    }

    pub const fn bit(self) -> u32 {
        1 << (self as u8)
    }
}

impl Display for TraderPreferenceKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.key())
    }
}

pub const ALL_TRADER_PREFERENCE_KINDS: [TraderPreferenceKind; 1] =
    [TraderPreferenceKind::DisableCollateralSweep];

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TraderPreferences {
    disable_collateral_sweep: bool,
}

impl TraderPreferences {
    pub const fn new(disable_collateral_sweep: bool) -> Self {
        Self {
            disable_collateral_sweep,
        }
    }

    pub const fn is_enabled(self, preference: TraderPreferenceKind) -> bool {
        match preference {
            TraderPreferenceKind::DisableCollateralSweep => self.disable_collateral_sweep,
        }
    }

    pub const fn disable_collateral_sweep(self) -> bool {
        self.disable_collateral_sweep
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TraderPreferenceFlagsError {
    ReservedBitsSet(u32),
}

impl Display for TraderPreferenceFlagsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ReservedBitsSet(bits) => {
                write!(f, "reserved trader preference bits set: 0x{bits:08X}")
            }
        }
    }
}

impl std::error::Error for TraderPreferenceFlagsError {}

#[repr(transparent)]
#[derive(Copy, Clone, Default, PartialEq, Eq, Pod, Zeroable, BorshDeserialize, BorshSerialize)]
pub struct TraderPreferenceFlags {
    flags: u32,
}

impl TraderPreferenceFlags {
    pub const DISABLE_COLLATERAL_SWEEP: u32 = TRADER_PREFERENCE_DISABLE_COLLATERAL_SWEEP;
    pub const RESERVED_MASK: u32 = !Self::VALID_MASK;
    pub const VALID_MASK: u32 = Self::DISABLE_COLLATERAL_SWEEP;

    pub const fn new(flags: u32) -> Self {
        Self { flags }
    }

    pub const fn try_new(flags: u32) -> Result<Self, TraderPreferenceFlagsError> {
        let reserved = flags & Self::RESERVED_MASK;
        if reserved != 0 {
            Err(TraderPreferenceFlagsError::ReservedBitsSet(reserved))
        } else {
            Ok(Self { flags })
        }
    }

    pub const fn as_u32(&self) -> u32 {
        self.flags
    }

    pub const fn bits(self) -> u32 {
        self.flags
    }

    pub const fn reserved_bits(self) -> u32 {
        self.flags & Self::RESERVED_MASK
    }

    pub const fn has_reserved_bits(self) -> bool {
        self.reserved_bits() != 0
    }

    pub const fn contains(self, mask: u32) -> bool {
        (self.flags & mask) == mask
    }

    pub const fn is_enabled(self, preference: TraderPreferenceKind) -> bool {
        self.contains(preference.bit())
    }

    pub const fn disable_collateral_sweep(self) -> bool {
        self.is_enabled(TraderPreferenceKind::DisableCollateralSweep)
    }

    pub const fn preferences(self) -> TraderPreferences {
        TraderPreferences::new(self.disable_collateral_sweep())
    }

    pub fn enable_preference(&mut self, preference: TraderPreferenceKind) {
        self.flags |= preference.bit();
    }

    pub fn disable_preference(&mut self, preference: TraderPreferenceKind) {
        self.flags &= !preference.bit();
    }

    pub fn set_disable_collateral_sweep(&mut self, disable_collateral_sweep: bool) {
        if disable_collateral_sweep {
            self.enable_preference(TraderPreferenceKind::DisableCollateralSweep);
        } else {
            self.disable_preference(TraderPreferenceKind::DisableCollateralSweep);
        }
    }
}

impl Debug for TraderPreferenceFlags {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TraderPreferenceFlags")
            .field("bits", &format_args!("0x{:08X}", self.flags))
            .field("preferences", &format_args!("{self}"))
            .finish()
    }
}

impl Display for TraderPreferenceFlags {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        const PREFERENCES: [(&str, u32); 1] = [(
            "DISABLE_COLLATERAL_SWEEP",
            TraderPreferenceFlags::DISABLE_COLLATERAL_SWEEP,
        )];

        let mut wrote_any = false;
        for (name, mask) in PREFERENCES {
            if self.contains(mask) {
                if wrote_any {
                    write!(f, " | ")?;
                }
                write!(f, "{name}")?;
                wrote_any = true;
            }
        }

        let unknown = self.reserved_bits();
        if unknown != 0 {
            if wrote_any {
                write!(f, " | ")?;
            }
            write!(f, "UNKNOWN(0x{unknown:08X})")?;
            wrote_any = true;
        }

        if !wrote_any {
            write!(f, "NONE")?;
        }

        Ok(())
    }
}

impl From<TraderPreferenceFlags> for u32 {
    fn from(flags: TraderPreferenceFlags) -> Self {
        flags.bits()
    }
}

impl TryFrom<u32> for TraderPreferenceFlags {
    type Error = TraderPreferenceFlagsError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for TraderPreferenceFlags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("TraderPreferenceFlags", 5)?;
        state.serialize_field("bits", &self.bits())?;
        state.serialize_field("reserved_bits", &self.reserved_bits())?;
        state.serialize_field("has_reserved_bits", &self.has_reserved_bits())?;
        state.serialize_field("preferences", &self.preferences())?;
        state.serialize_field("disable_collateral_sweep", &self.disable_collateral_sweep())?;
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preference_flags_decode_known_and_reserved_bits() {
        let flags =
            TraderPreferenceFlags::new(TraderPreferenceFlags::DISABLE_COLLATERAL_SWEEP | (1 << 7));

        assert!(flags.disable_collateral_sweep());
        assert!(
            flags
                .preferences()
                .is_enabled(TraderPreferenceKind::DisableCollateralSweep)
        );
        assert_eq!(flags.reserved_bits(), 1 << 7);
        assert!(flags.has_reserved_bits());
        assert_eq!(
            flags.to_string(),
            "DISABLE_COLLATERAL_SWEEP | UNKNOWN(0x00000080)"
        );
    }
}
