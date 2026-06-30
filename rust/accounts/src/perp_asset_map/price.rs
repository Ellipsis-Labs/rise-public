use bytemuck::{Pod, Zeroable};
use phoenix_rise_math::{SignedTicks, Ticks};
#[cfg(feature = "serde")]
use serde::ser::SerializeStruct;

use super::MAX_ORACLES;
use crate::common::SequenceNumber;
#[cfg(feature = "serde")]
use crate::serde_helpers::pubkey_string;

const_assert_eq!(core::mem::size_of::<TicksAtSlot>(), 16);
const_assert_eq!(core::mem::size_of::<OracleData>(), 40);
const_assert_eq!(core::mem::size_of::<BookPriceComponent>(), 88);
const_assert_eq!(core::mem::size_of::<PerpPriceComponent>(), 96);
const_assert_eq!(core::mem::size_of::<SpotPriceComponent>(), 136);
const_assert_eq!(core::mem::size_of::<MarkPrice>(), 872);
const_assert_eq!(core::mem::size_of::<PriceComponent>(), 888);

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TicksAtSlot {
    pub slot: u64,
    pub ticks: Ticks,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct OracleData {
    pub oracle_pubkey: [u8; 32],
    pub divergence_score: u8,
    _padding: [u8; 7],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OracleParameters {
    pub oracle_divergence_radius: u16,
    pub min_oracle_responses: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BookPriceComponent {
    pub clamped_book_mid_price: TicksAtSlot,
    pub weight: u64,
    pub stale_threshold: u64,
    pub book_price_radius: u64,
    pub last_best_bid: TicksAtSlot,
    pub last_best_ask: TicksAtSlot,
    pub last_trade_price: TicksAtSlot,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PerpPriceComponent {
    pub last_exchange_perp_price: [TicksAtSlot; MAX_ORACLES],
    pub weight: u64,
    pub stale_threshold: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpotPriceComponent {
    pub last_exchange_spot_price: [TicksAtSlot; MAX_ORACLES],
    pub weight: u64,
    pub stale_threshold: u64,
    pub slot: u64,
    pub mid_spot_diff_ema_ticks: SignedTicks,
    pub mid_spot_diff_ema_ticks_dust: i64,
    pub ema_period_slots: u64,
    pub ema_diff_radius: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct MarkPrice {
    pub price: TicksAtSlot,
    pub spot_price_component: SpotPriceComponent,
    pub perp_price_component: PerpPriceComponent,
    pub book_price_component: BookPriceComponent,
    pub risk_action_price_validity_rules: [u8; 256],
    pub oracle_data: [OracleData; MAX_ORACLES],
    pub oracle_divergence_radius: u16,
    pub min_oracle_responses: u8,
    pub book_hard_stale_multiplier: u8,
    pub oracle_hard_stale_multiplier: u8,
    _padding: [u8; 3],
    pub mark_price_last_validated_slot: [u64; 4],
    pub oracle_last_updated_timestamps: [u64; MAX_ORACLES],
}

impl MarkPrice {
    #[inline(always)]
    pub const fn oracle_parameters(&self) -> OracleParameters {
        OracleParameters {
            oracle_divergence_radius: self.oracle_divergence_radius,
            min_oracle_responses: self.min_oracle_responses,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PriceComponent {
    pub price_sequence_number: SequenceNumber,
    pub mark_price: MarkPrice,
}

#[cfg(feature = "serde")]
impl serde::Serialize for OracleData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("OracleData", 2)?;
        state.serialize_field("oracle_pubkey", &pubkey_string(&self.oracle_pubkey))?;
        state.serialize_field("divergence_score", &self.divergence_score)?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for MarkPrice {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("MarkPrice", 13)?;
        state.serialize_field("price", &self.price)?;
        state.serialize_field("spot_price_component", &self.spot_price_component)?;
        state.serialize_field("perp_price_component", &self.perp_price_component)?;
        state.serialize_field("book_price_component", &self.book_price_component)?;
        state.serialize_field(
            "risk_action_price_validity_rules",
            &self.risk_action_price_validity_rules.as_slice(),
        )?;
        state.serialize_field("oracle_data", &self.oracle_data)?;
        state.serialize_field("oracle_parameters", &self.oracle_parameters())?;
        state.serialize_field("oracle_divergence_radius", &self.oracle_divergence_radius)?;
        state.serialize_field("min_oracle_responses", &self.min_oracle_responses)?;
        state.serialize_field(
            "book_hard_stale_multiplier",
            &self.book_hard_stale_multiplier,
        )?;
        state.serialize_field(
            "oracle_hard_stale_multiplier",
            &self.oracle_hard_stale_multiplier,
        )?;
        state.serialize_field(
            "mark_price_last_validated_slot",
            &self.mark_price_last_validated_slot,
        )?;
        state.serialize_field(
            "oracle_last_updated_timestamps",
            &self.oracle_last_updated_timestamps,
        )?;
        state.end()
    }
}
