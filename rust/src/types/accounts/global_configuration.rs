use solana_pubkey::Pubkey;

use super::discriminants::ACCOUNT_DISCRIMINANTS;
use super::internal::{AuthoritySet, Reader, read_authority_set, verify_discriminant};
use super::{AccountDeserialize, AccountDeserializeError};

const ACCOUNT: &str = "GlobalConfiguration";

#[derive(Debug, Clone, PartialEq, Eq)]
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
}

impl AccountDeserialize for GlobalConfiguration {
    fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        verify_discriminant(ACCOUNT, data, ACCOUNT_DISCRIMINANTS.global_configuration)?;
        let mut reader = Reader::with_offset(ACCOUNT, data, 8);
        let account_key = reader.read_pubkey()?;
        let current_authorities = read_authority_set(&mut reader)?;
        let canonical_token_mint_key = reader.read_pubkey()?;
        let global_vault_key = reader.read_pubkey()?;
        let perp_asset_map_key = reader.read_pubkey()?;
        let global_trader_index_header_key = reader.read_pubkey()?;
        let active_trader_buffer_header_key = reader.read_pubkey()?;
        let total_quote_lot_fees = reader.read_u64()?;
        let unclaimed_quote_lot_fees = reader.read_u64()?;
        let withdraw_queue_key = reader.read_pubkey()?;
        let exchange_status = reader.read_u8()?;
        let quote_decimals = reader.read_u8()?;
        let withdrawal_margin_factor_bps = reader.read_u16()?;
        reader.skip(4)?;
        let deposit_cooldown_period_in_slots = reader.read_u64()?;
        let pending_authorities = read_authority_set(&mut reader)?;
        // Reserved regions.
        reader.skip(8 * 31)?;
        reader.skip(8 * 32)?;
        reader.skip(8 * 32)?;
        reader.skip(8 * 32)?;
        reader.skip(8 * 32)?;
        reader.skip(8 * 32)?;
        reader.skip(8 * 32)?;
        Ok(Self {
            account_key,
            current_authorities,
            canonical_token_mint_key,
            global_vault_key,
            perp_asset_map_key,
            global_trader_index_header_key,
            active_trader_buffer_header_key,
            total_quote_lot_fees,
            unclaimed_quote_lot_fees,
            withdraw_queue_key,
            exchange_status,
            quote_decimals,
            withdrawal_margin_factor_bps,
            deposit_cooldown_period_in_slots,
            pending_authorities,
        })
    }
}

impl GlobalConfiguration {
    pub fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        <Self as AccountDeserialize>::try_from_account_bytes(data)
    }
}
