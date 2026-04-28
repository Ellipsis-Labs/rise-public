//! Phoenix program constants and addresses.

use sha2::{Digest, Sha256};
use solana_pubkey::Pubkey;

/// The Phoenix program ID (mainnet).
pub const PHOENIX_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("EtrnLzgbS7nMMy5fbD42kXiUzGg8XQzJ972Xtk1cjWih");

/// The Phoenix log authority address (mainnet).
pub const PHOENIX_LOG_AUTHORITY: Pubkey =
    solana_pubkey::pubkey!("GdxfTLSsdSY37G6fZoYtdGDSfgFnbT2EmRpuePZxWShS");

/// The Phoenix global configuration address (mainnet).
pub const PHOENIX_GLOBAL_CONFIGURATION: Pubkey =
    solana_pubkey::pubkey!("2zskx2iyCvb6Stg7RBZkt1f6MrF4dpYtMG3yMvKwqtUZ");

/// The Ember program ID (for USDC -> Phoenix token conversion).
pub const EMBER_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("EMBERpYNE6ehWmXymZZS2skiFmCa9V5dp14e1iduM5qy");

/// USDC mint address (mainnet).
pub const USDC_MINT: Pubkey =
    solana_pubkey::pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");

/// SPL Token program ID.
pub const SPL_TOKEN_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// Associated Token program ID.
pub const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

/// System program ID.
pub const SYSTEM_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("11111111111111111111111111111111");

/// Compute the instruction discriminant using SHA-256.
/// Takes the first 8 bytes of SHA-256 hash of the input string.
pub fn compute_discriminant(input: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    let mut discriminant = [0u8; 8];
    discriminant.copy_from_slice(&result[..8]);
    discriminant
}

/// Instruction discriminant for place_limit_order.
pub fn place_limit_order_discriminant() -> [u8; 8] {
    compute_discriminant("global:place_limit_order")
}

/// Instruction discriminant for place_market_order.
pub fn place_market_order_discriminant() -> [u8; 8] {
    compute_discriminant("global:place_market_order")
}

/// Instruction discriminant for cancel_orders_by_id.
pub fn cancel_orders_by_id_discriminant() -> [u8; 8] {
    compute_discriminant("global:cancel_orders_by_id")
}

/// Instruction discriminant for deposit_funds.
pub fn deposit_funds_discriminant() -> [u8; 8] {
    compute_discriminant("global:deposit_funds")
}

/// Instruction discriminant for ember deposit.
pub fn ember_deposit_discriminant() -> [u8; 8] {
    compute_discriminant("global:deposit")
}

/// Instruction discriminant for withdraw_funds.
pub fn withdraw_funds_discriminant() -> [u8; 8] {
    compute_discriminant("global:withdraw_funds")
}

/// Instruction discriminant for ember withdraw.
pub fn ember_withdraw_discriminant() -> [u8; 8] {
    compute_discriminant("global:withdraw")
}

/// Instruction discriminant for register_trader.
pub fn register_trader_discriminant() -> [u8; 8] {
    compute_discriminant("global:register_trader")
}

/// Instruction discriminant for transfer_collateral.
pub fn transfer_collateral_discriminant() -> [u8; 8] {
    compute_discriminant("global:transfer_collateral")
}

/// Instruction discriminant for transfer_collateral_child_to_parent.
pub fn transfer_collateral_child_to_parent_discriminant() -> [u8; 8] {
    compute_discriminant("global:transfer_collateral_child_to_parent")
}

/// Instruction discriminant for sync_parent_to_child.
pub fn sync_parent_to_child_discriminant() -> [u8; 8] {
    compute_discriminant("global:sync_parent_to_child")
}

/// Instruction discriminant for place_multi_limit_order.
pub fn place_multi_limit_order_discriminant() -> [u8; 8] {
    compute_discriminant("global:place_multi_limit_order")
}

/// Instruction discriminant for cancel_stop_loss.
pub fn cancel_stop_loss_discriminant() -> [u8; 8] {
    compute_discriminant("global:cancel_stop_loss")
}

/// Instruction discriminant for create_conditional_orders_account.
pub fn create_conditional_orders_account_discriminant() -> [u8; 8] {
    compute_discriminant("global:create_conditional_orders_account")
}

/// Instruction discriminant for place_position_conditional_order.
pub fn place_position_conditional_order_discriminant() -> [u8; 8] {
    compute_discriminant("global:place_position_conditional_order")
}

/// Instruction discriminant for cancel_conditional_order.
pub fn cancel_conditional_order_discriminant() -> [u8; 8] {
    compute_discriminant("global:cancel_conditional_order")
}

/// Instruction discriminant for place_attached_conditional_order.
pub fn place_attached_conditional_order_discriminant() -> [u8; 8] {
    compute_discriminant("global:place_attached_conditional_order")
}

/// Instruction discriminant for place_limit_order_with_conditionals.
pub fn place_limit_order_with_conditionals_discriminant() -> [u8; 8] {
    compute_discriminant("global:place_limit_order_with_conditionals")
}

/// Derives the stop loss PDA for a given trader account and asset ID.
///
/// Seeds: ["stoploss", trader_account, &asset_id.to_le_bytes()]
pub fn get_stop_loss_address(trader_account: &Pubkey, asset_id: u64) -> Pubkey {
    let (pda, _bump) = Pubkey::find_program_address(
        &[
            b"stoploss",
            trader_account.as_ref(),
            &asset_id.to_le_bytes(),
        ],
        &PHOENIX_PROGRAM_ID,
    );
    pda
}

/// Derives the conditional orders PDA for a given trader account.
///
/// Seeds: ["conditional_orders", trader_account]
pub fn get_conditional_orders_address(trader_account: &Pubkey) -> Pubkey {
    let (pda, _bump) = Pubkey::find_program_address(
        &[b"conditional_orders", trader_account.as_ref()],
        &PHOENIX_PROGRAM_ID,
    );
    pda
}

/// Derives the spline collection PDA for a given market (orderbook) address.
///
/// Seeds: ["spline", market_address]
pub fn get_spline_collection_address(market: &Pubkey) -> Pubkey {
    let (pda, _bump) =
        Pubkey::find_program_address(&[b"spline", market.as_ref()], &PHOENIX_PROGRAM_ID);
    pda
}

/// Derives the Ember state PDA.
///
/// Seeds: [phoenix_program_id, "state"] against Ember program
pub fn get_ember_state_address() -> Pubkey {
    let (pda, _bump) =
        Pubkey::find_program_address(&[PHOENIX_PROGRAM_ID.as_ref(), b"state"], &EMBER_PROGRAM_ID);
    pda
}

/// Derives the Ember vault PDA.
///
/// Seeds: [phoenix_program_id, "vault"] against Ember program
pub fn get_ember_vault_address() -> Pubkey {
    let (pda, _bump) =
        Pubkey::find_program_address(&[PHOENIX_PROGRAM_ID.as_ref(), b"vault"], &EMBER_PROGRAM_ID);
    pda
}

/// Derives the global vault PDA for a given mint.
///
/// Seeds: ["vault", mint] against Phoenix program
pub fn get_global_vault_address(mint: &Pubkey) -> Pubkey {
    let (pda, _bump) =
        Pubkey::find_program_address(&[b"vault", mint.as_ref()], &PHOENIX_PROGRAM_ID);
    pda
}

/// Derives the associated token address for an owner and mint.
///
/// This follows the standard SPL ATA derivation.
pub fn get_associated_token_address(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    let (pda, _bump) = Pubkey::find_program_address(
        &[owner.as_ref(), SPL_TOKEN_PROGRAM_ID.as_ref(), mint.as_ref()],
        &ASSOCIATED_TOKEN_PROGRAM_ID,
    );
    pda
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discriminant_computation() {
        // These values should match the TypeScript SDK
        let limit_disc = place_limit_order_discriminant();
        let market_disc = place_market_order_discriminant();
        let cancel_disc = cancel_orders_by_id_discriminant();

        // Discriminants should be 8 bytes and non-zero
        assert_ne!(limit_disc, [0u8; 8]);
        assert_ne!(market_disc, [0u8; 8]);
        assert_ne!(cancel_disc, [0u8; 8]);

        // Each discriminant should be unique
        assert_ne!(limit_disc, market_disc);
        assert_ne!(limit_disc, cancel_disc);
        assert_ne!(market_disc, cancel_disc);
    }

    #[test]
    fn test_spline_collection_pda_derivation() {
        // Test that PDA derivation is deterministic
        let market = Pubkey::new_unique();
        let pda1 = get_spline_collection_address(&market);
        let pda2 = get_spline_collection_address(&market);
        assert_eq!(pda1, pda2);

        // Different markets should produce different PDAs
        let market2 = Pubkey::new_unique();
        let pda3 = get_spline_collection_address(&market2);
        assert_ne!(pda1, pda3);
    }

    #[test]
    fn test_deposit_discriminants() {
        let deposit_disc = deposit_funds_discriminant();
        let ember_disc = ember_deposit_discriminant();

        // Discriminants should be non-zero and unique
        assert_ne!(deposit_disc, [0u8; 8]);
        assert_ne!(ember_disc, [0u8; 8]);
        assert_ne!(deposit_disc, ember_disc);
    }

    #[test]
    fn test_register_trader_discriminant() {
        let disc = register_trader_discriminant();
        assert_ne!(disc, [0u8; 8]);
        assert_ne!(disc, place_limit_order_discriminant());
        assert_ne!(disc, place_market_order_discriminant());
        assert_ne!(disc, cancel_orders_by_id_discriminant());
        assert_ne!(disc, deposit_funds_discriminant());
        assert_ne!(disc, withdraw_funds_discriminant());
    }

    #[test]
    fn test_withdraw_discriminants() {
        let withdraw_disc = withdraw_funds_discriminant();
        let ember_withdraw_disc = ember_withdraw_discriminant();
        let deposit_disc = deposit_funds_discriminant();
        let ember_deposit_disc = ember_deposit_discriminant();

        // Discriminants should be non-zero
        assert_ne!(withdraw_disc, [0u8; 8]);
        assert_ne!(ember_withdraw_disc, [0u8; 8]);

        // All discriminants should be unique
        assert_ne!(withdraw_disc, ember_withdraw_disc);
        assert_ne!(withdraw_disc, deposit_disc);
        assert_ne!(ember_withdraw_disc, ember_deposit_disc);
    }

    #[test]
    fn test_ember_pda_derivation() {
        // Ember PDAs should be deterministic
        let state1 = get_ember_state_address();
        let state2 = get_ember_state_address();
        assert_eq!(state1, state2);

        let vault1 = get_ember_vault_address();
        let vault2 = get_ember_vault_address();
        assert_eq!(vault1, vault2);

        // State and vault should be different
        assert_ne!(state1, vault1);
    }

    #[test]
    fn test_global_vault_pda_derivation() {
        let mint = Pubkey::new_unique();
        let vault1 = get_global_vault_address(&mint);
        let vault2 = get_global_vault_address(&mint);
        assert_eq!(vault1, vault2);

        // Different mints should produce different vaults
        let mint2 = Pubkey::new_unique();
        let vault3 = get_global_vault_address(&mint2);
        assert_ne!(vault1, vault3);
    }

    #[test]
    fn test_stop_loss_pda_derivation() {
        let trader_account = Pubkey::new_unique();
        let asset_id: u64 = 42;
        let pda1 = get_stop_loss_address(&trader_account, asset_id);
        let pda2 = get_stop_loss_address(&trader_account, asset_id);
        assert_eq!(pda1, pda2);

        // Different asset_id should produce different PDA
        let pda3 = get_stop_loss_address(&trader_account, 99);
        assert_ne!(pda1, pda3);

        // Different trader should produce different PDA
        let trader2 = Pubkey::new_unique();
        let pda4 = get_stop_loss_address(&trader2, asset_id);
        assert_ne!(pda1, pda4);
    }

    #[test]
    fn test_ata_derivation() {
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let ata1 = get_associated_token_address(&owner, &mint);
        let ata2 = get_associated_token_address(&owner, &mint);
        assert_eq!(ata1, ata2);

        // Different owner or mint should produce different ATA
        let owner2 = Pubkey::new_unique();
        let ata3 = get_associated_token_address(&owner2, &mint);
        assert_ne!(ata1, ata3);
    }
}
