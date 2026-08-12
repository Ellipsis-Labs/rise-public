//! Borrowed views for Phoenix global configuration accounts.

use bytemuck::{Pod, Zeroable};
#[cfg(feature = "serde")]
use serde::ser::SerializeStruct;

use super::common::{PhoenixAccountDecodeError, read_prefix, require_len, verify_discriminant};
use super::discriminants::PhoenixAccount;
use super::spot_collateral::SpotCollateralMetadata;
#[cfg(feature = "serde")]
use crate::serde_helpers::pubkey_string;

const GLOBAL_CONFIG_ACCOUNT: &str = "GlobalConfig";
const GLOBAL_CONFIG_PREFIX_LEN: usize = core::mem::size_of::<GlobalConfigPrefixRaw>();

const_assert_eq!(core::mem::size_of::<AuthoritySetRaw>(), 256);
const_assert_eq!(core::mem::size_of::<GlobalConfigPrefixRaw>(), 1096);

// `native_sol_spot_metadata` was carved out of reserved bytes, so a drifting
// prefix silently decodes garbage rather than failing a length check.
const_assert_eq!(
    core::mem::offset_of!(GlobalConfigPrefixRaw, pending_authorities),
    520
);
const_assert_eq!(
    core::mem::offset_of!(GlobalConfigPrefixRaw, native_sol_spot_metadata),
    776
);

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct AuthoritySetRaw {
    root_authority: [u8; 32],
    risk_authority: [u8; 32],
    market_authority: [u8; 32],
    oracle_authority: [u8; 32],
    reserved_authority: [u8; 32],
    adl_authority: [u8; 32],
    cancel_authority: [u8; 32],
    backstop_authority: [u8; 32],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct GlobalConfigPrefixRaw {
    discriminant: u64,
    account_key: [u8; 32],
    current_authorities: AuthoritySetRaw,
    canonical_token_mint_key: [u8; 32],
    global_vault_key: [u8; 32],
    perp_asset_map_key: [u8; 32],
    global_trader_index_header_key: [u8; 32],
    active_trader_buffer_header_key: [u8; 32],
    total_quote_lot_fees: u64,
    unclaimed_quote_lot_fees: u64,
    withdraw_queue_key: [u8; 32],
    exchange_status: u8,
    quote_decimals: u8,
    withdrawal_margin_factor_bps: u16,
    _padding0: [u8; 4],
    deposit_cooldown_period_in_slots: u64,
    pending_authorities: AuthoritySetRaw,
    native_sol_spot_metadata: SpotCollateralMetadata,
}

/// View over the GlobalConfig fields most CPI callers need.
#[derive(Clone, Copy, Debug)]
pub struct GlobalConfig {
    raw: GlobalConfigPrefixRaw,
}

impl GlobalConfig {
    pub fn try_from_account_bytes(data: &[u8]) -> Result<Self, PhoenixAccountDecodeError> {
        require_len(GLOBAL_CONFIG_ACCOUNT, data, GLOBAL_CONFIG_PREFIX_LEN)?;
        verify_discriminant(
            GLOBAL_CONFIG_ACCOUNT,
            data,
            PhoenixAccount::GlobalConfiguration.discriminant(),
        )?;
        Ok(Self {
            raw: read_prefix(GLOBAL_CONFIG_ACCOUNT, data)?,
        })
    }

    #[inline(always)]
    pub const fn account_key(&self) -> [u8; 32] {
        self.raw.account_key
    }

    #[inline(always)]
    pub const fn root_authority(&self) -> [u8; 32] {
        self.raw.current_authorities.root_authority
    }

    #[inline(always)]
    pub const fn risk_authority(&self) -> [u8; 32] {
        self.raw.current_authorities.risk_authority
    }

    #[inline(always)]
    pub const fn market_authority(&self) -> [u8; 32] {
        self.raw.current_authorities.market_authority
    }

    #[inline(always)]
    pub const fn oracle_authority(&self) -> [u8; 32] {
        self.raw.current_authorities.oracle_authority
    }

    #[inline(always)]
    pub const fn reserved_authority(&self) -> [u8; 32] {
        self.raw.current_authorities.reserved_authority
    }

    #[inline(always)]
    pub const fn adl_authority(&self) -> [u8; 32] {
        self.raw.current_authorities.adl_authority
    }

    #[inline(always)]
    pub const fn cancel_authority(&self) -> [u8; 32] {
        self.raw.current_authorities.cancel_authority
    }

    #[inline(always)]
    pub const fn backstop_authority(&self) -> [u8; 32] {
        self.raw.current_authorities.backstop_authority
    }

    #[inline(always)]
    pub const fn canonical_token_mint_key(&self) -> [u8; 32] {
        self.raw.canonical_token_mint_key
    }

    #[inline(always)]
    pub const fn global_vault_key(&self) -> [u8; 32] {
        self.raw.global_vault_key
    }

    #[inline(always)]
    pub const fn perp_asset_map_key(&self) -> [u8; 32] {
        self.raw.perp_asset_map_key
    }

    #[inline(always)]
    pub const fn global_trader_index_header_key(&self) -> [u8; 32] {
        self.raw.global_trader_index_header_key
    }

    #[inline(always)]
    pub const fn active_trader_buffer_header_key(&self) -> [u8; 32] {
        self.raw.active_trader_buffer_header_key
    }

    #[inline(always)]
    pub const fn total_quote_lot_fees(&self) -> u64 {
        self.raw.total_quote_lot_fees
    }

    #[inline(always)]
    pub const fn unclaimed_quote_lot_fees(&self) -> u64 {
        self.raw.unclaimed_quote_lot_fees
    }

    #[inline(always)]
    pub const fn withdraw_queue_key(&self) -> [u8; 32] {
        self.raw.withdraw_queue_key
    }

    #[inline(always)]
    pub const fn exchange_status(&self) -> u8 {
        self.raw.exchange_status
    }

    #[inline(always)]
    pub const fn quote_decimals(&self) -> u8 {
        self.raw.quote_decimals
    }

    #[inline(always)]
    pub const fn withdrawal_margin_factor_bps(&self) -> u16 {
        self.raw.withdrawal_margin_factor_bps
    }

    #[inline(always)]
    pub const fn deposit_cooldown_period_in_slots(&self) -> u64 {
        self.raw.deposit_cooldown_period_in_slots
    }

    /// Spot collateral configuration for native SOL. All-zero (and therefore
    /// inactive) on exchanges that never configured it.
    #[inline(always)]
    pub const fn native_sol_spot_metadata(&self) -> &SpotCollateralMetadata {
        &self.raw.native_sol_spot_metadata
    }

    #[inline(always)]
    pub const fn pending_root_authority(&self) -> [u8; 32] {
        self.raw.pending_authorities.root_authority
    }

    #[inline(always)]
    pub const fn pending_risk_authority(&self) -> [u8; 32] {
        self.raw.pending_authorities.risk_authority
    }

    #[inline(always)]
    pub const fn pending_market_authority(&self) -> [u8; 32] {
        self.raw.pending_authorities.market_authority
    }

    #[inline(always)]
    pub const fn pending_oracle_authority(&self) -> [u8; 32] {
        self.raw.pending_authorities.oracle_authority
    }

    #[inline(always)]
    pub const fn pending_reserved_authority(&self) -> [u8; 32] {
        self.raw.pending_authorities.reserved_authority
    }

    #[inline(always)]
    pub const fn pending_adl_authority(&self) -> [u8; 32] {
        self.raw.pending_authorities.adl_authority
    }

    #[inline(always)]
    pub const fn pending_cancel_authority(&self) -> [u8; 32] {
        self.raw.pending_authorities.cancel_authority
    }

    #[inline(always)]
    pub const fn pending_backstop_authority(&self) -> [u8; 32] {
        self.raw.pending_authorities.backstop_authority
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for GlobalConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("GlobalConfig", 30)?;
        state.serialize_field("account_key", &pubkey_string(&self.account_key()))?;
        state.serialize_field("root_authority", &pubkey_string(&self.root_authority()))?;
        state.serialize_field("risk_authority", &pubkey_string(&self.risk_authority()))?;
        state.serialize_field("market_authority", &pubkey_string(&self.market_authority()))?;
        state.serialize_field("oracle_authority", &pubkey_string(&self.oracle_authority()))?;
        state.serialize_field(
            "reserved_authority",
            &pubkey_string(&self.reserved_authority()),
        )?;
        state.serialize_field("adl_authority", &pubkey_string(&self.adl_authority()))?;
        state.serialize_field("cancel_authority", &pubkey_string(&self.cancel_authority()))?;
        state.serialize_field(
            "backstop_authority",
            &pubkey_string(&self.backstop_authority()),
        )?;
        state.serialize_field(
            "canonical_token_mint_key",
            &pubkey_string(&self.canonical_token_mint_key()),
        )?;
        state.serialize_field("global_vault_key", &pubkey_string(&self.global_vault_key()))?;
        state.serialize_field(
            "perp_asset_map_key",
            &pubkey_string(&self.perp_asset_map_key()),
        )?;
        state.serialize_field(
            "global_trader_index_header_key",
            &pubkey_string(&self.global_trader_index_header_key()),
        )?;
        state.serialize_field(
            "active_trader_buffer_header_key",
            &pubkey_string(&self.active_trader_buffer_header_key()),
        )?;
        state.serialize_field("total_quote_lot_fees", &self.total_quote_lot_fees())?;
        state.serialize_field("unclaimed_quote_lot_fees", &self.unclaimed_quote_lot_fees())?;
        state.serialize_field(
            "withdraw_queue_key",
            &pubkey_string(&self.withdraw_queue_key()),
        )?;
        state.serialize_field("exchange_status", &self.exchange_status())?;
        state.serialize_field("quote_decimals", &self.quote_decimals())?;
        state.serialize_field(
            "withdrawal_margin_factor_bps",
            &self.withdrawal_margin_factor_bps(),
        )?;
        state.serialize_field(
            "deposit_cooldown_period_in_slots",
            &self.deposit_cooldown_period_in_slots(),
        )?;
        state.serialize_field(
            "pending_root_authority",
            &pubkey_string(&self.pending_root_authority()),
        )?;
        state.serialize_field(
            "pending_risk_authority",
            &pubkey_string(&self.pending_risk_authority()),
        )?;
        state.serialize_field(
            "pending_market_authority",
            &pubkey_string(&self.pending_market_authority()),
        )?;
        state.serialize_field(
            "pending_oracle_authority",
            &pubkey_string(&self.pending_oracle_authority()),
        )?;
        state.serialize_field(
            "pending_reserved_authority",
            &pubkey_string(&self.pending_reserved_authority()),
        )?;
        state.serialize_field(
            "pending_adl_authority",
            &pubkey_string(&self.pending_adl_authority()),
        )?;
        state.serialize_field(
            "pending_cancel_authority",
            &pubkey_string(&self.pending_cancel_authority()),
        )?;
        state.serialize_field(
            "pending_backstop_authority",
            &pubkey_string(&self.pending_backstop_authority()),
        )?;
        state.serialize_field("native_sol_spot_metadata", self.native_sol_spot_metadata())?;
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spot_collateral::{
        SPOT_COLLATERAL_FLAG_HAS_PERP_ASSET, SPOT_COLLATERAL_FLAG_IS_ACTIVE,
    };

    /// On-chain `GlobalConfiguration` is 2560 bytes; the prefix this crate
    /// decodes is only the leading 1096.
    const GLOBAL_CONFIG_ACCOUNT_LEN: usize = 2560;
    const SPOT_METADATA_OFFSET: usize = 776;
    const SPOT_FLAGS_OFFSET: usize = SPOT_METADATA_OFFSET + 104;
    const SPOT_MAX_GLOBAL_BALANCE_OFFSET: usize = SPOT_METADATA_OFFSET + 48;

    fn global_config_bytes(spot_flags: u8) -> Vec<u8> {
        let mut data = vec![0u8; GLOBAL_CONFIG_ACCOUNT_LEN];
        data[..8].copy_from_slice(&PhoenixAccount::GlobalConfiguration.discriminant());
        data[SPOT_FLAGS_OFFSET] = spot_flags;
        data
    }

    #[test]
    fn native_sol_spot_metadata_decodes_at_offset_776() {
        let mut data = global_config_bytes(
            SPOT_COLLATERAL_FLAG_IS_ACTIVE | SPOT_COLLATERAL_FLAG_HAS_PERP_ASSET,
        );
        data[SPOT_MAX_GLOBAL_BALANCE_OFFSET..SPOT_MAX_GLOBAL_BALANCE_OFFSET + 8]
            .copy_from_slice(&100_000_000_000_000u64.to_le_bytes());
        // perp_asset_index lives 36 bytes into the metadata.
        data[SPOT_METADATA_OFFSET + 36..SPOT_METADATA_OFFSET + 40]
            .copy_from_slice(&3u32.to_le_bytes());

        let config = GlobalConfig::try_from_account_bytes(&data).unwrap();
        let metadata = config.native_sol_spot_metadata();

        assert_eq!(metadata.max_global_balance(), 100_000_000_000_000);
        assert_eq!(metadata.get_asset_id(), Some(3));
        assert!(!metadata.is_zeroed());
    }

    #[test]
    fn exchanges_without_spot_collateral_decode_as_zeroed_and_inactive() {
        let data = global_config_bytes(0);
        let config = GlobalConfig::try_from_account_bytes(&data).unwrap();

        assert!(config.native_sol_spot_metadata().is_zeroed());
        assert!(!config.native_sol_spot_metadata().is_active());
    }
}
