use serde::Serialize;
use solana_pubkey::Pubkey;

use super::internal::{AuthoritySet, SpotCollateralMetadata};
use super::{AccountDeserialize, AccountDeserializeError};
use crate::global_config as borrowed;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GlobalConfiguration {
    pub account_key: Pubkey,
    pub current_authorities: AuthoritySet,
    pub canonical_token_mint_key: Pubkey,
    pub global_vault_key: Pubkey,
    pub perp_asset_map_key: Pubkey,
    pub global_trader_index_header_key: Pubkey,
    pub active_trader_buffer_header_key: Pubkey,
    pub total_quote_lot_fees: u64,
    pub unclaimed_quote_lot_fees: u64,
    pub withdraw_queue_key: Pubkey,
    pub exchange_status: u8,
    pub quote_decimals: u8,
    pub withdrawal_margin_factor_bps: u16,
    pub deposit_cooldown_period_in_slots: u64,
    pub pending_authorities: AuthoritySet,
    pub native_sol_spot_metadata: SpotCollateralMetadata,
}

impl AccountDeserialize for GlobalConfiguration {
    fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        Ok(borrowed::GlobalConfig::try_from_account_bytes(data)?.into())
    }
}

impl From<borrowed::GlobalConfig> for GlobalConfiguration {
    fn from(value: borrowed::GlobalConfig) -> Self {
        Self {
            account_key: Pubkey::new_from_array(value.account_key()),
            current_authorities: AuthoritySet {
                root_authority: Pubkey::new_from_array(value.root_authority()),
                risk_authority: Pubkey::new_from_array(value.risk_authority()),
                market_authority: Pubkey::new_from_array(value.market_authority()),
                oracle_authority: Pubkey::new_from_array(value.oracle_authority()),
                reserved_authority: Pubkey::new_from_array(value.reserved_authority()),
                adl_authority: Pubkey::new_from_array(value.adl_authority()),
                cancel_authority: Pubkey::new_from_array(value.cancel_authority()),
                backstop_authority: Pubkey::new_from_array(value.backstop_authority()),
            },
            canonical_token_mint_key: Pubkey::new_from_array(value.canonical_token_mint_key()),
            global_vault_key: Pubkey::new_from_array(value.global_vault_key()),
            perp_asset_map_key: Pubkey::new_from_array(value.perp_asset_map_key()),
            global_trader_index_header_key: Pubkey::new_from_array(
                value.global_trader_index_header_key(),
            ),
            active_trader_buffer_header_key: Pubkey::new_from_array(
                value.active_trader_buffer_header_key(),
            ),
            total_quote_lot_fees: value.total_quote_lot_fees(),
            unclaimed_quote_lot_fees: value.unclaimed_quote_lot_fees(),
            withdraw_queue_key: Pubkey::new_from_array(value.withdraw_queue_key()),
            exchange_status: value.exchange_status(),
            quote_decimals: value.quote_decimals(),
            withdrawal_margin_factor_bps: value.withdrawal_margin_factor_bps(),
            deposit_cooldown_period_in_slots: value.deposit_cooldown_period_in_slots(),
            pending_authorities: AuthoritySet {
                root_authority: Pubkey::new_from_array(value.pending_root_authority()),
                risk_authority: Pubkey::new_from_array(value.pending_risk_authority()),
                market_authority: Pubkey::new_from_array(value.pending_market_authority()),
                oracle_authority: Pubkey::new_from_array(value.pending_oracle_authority()),
                reserved_authority: Pubkey::new_from_array(value.pending_reserved_authority()),
                adl_authority: Pubkey::new_from_array(value.pending_adl_authority()),
                cancel_authority: Pubkey::new_from_array(value.pending_cancel_authority()),
                backstop_authority: Pubkey::new_from_array(value.pending_backstop_authority()),
            },
            native_sol_spot_metadata: value.native_sol_spot_metadata().into(),
        }
    }
}

impl GlobalConfiguration {
    pub fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        <Self as AccountDeserialize>::try_from_account_bytes(data)
    }
}
