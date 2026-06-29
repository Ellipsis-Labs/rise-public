//! Program-derived address helpers for Phoenix account layouts.

use solana_pubkey::Pubkey;

/// Derives the stop-loss PDA for a given trader account and asset ID.
///
/// Seeds: `["stoploss", trader_account, asset_id.to_le_bytes()]`.
pub fn derive_stop_loss_address(
    program_id: &Pubkey,
    trader_account: &Pubkey,
    asset_id: u64,
) -> Pubkey {
    let (pda, _bump) = Pubkey::find_program_address(
        &[
            b"stoploss",
            trader_account.as_ref(),
            &asset_id.to_le_bytes(),
        ],
        program_id,
    );
    pda
}

/// Derives the conditional-orders PDA for a given trader account.
///
/// Seeds: `["conditional_orders", trader_account]`.
pub fn derive_conditional_orders_address(program_id: &Pubkey, trader_account: &Pubkey) -> Pubkey {
    let (pda, _bump) = Pubkey::find_program_address(
        &[b"conditional_orders", trader_account.as_ref()],
        program_id,
    );
    pda
}

/// Derives the permission PDA for an authority and delegated signer.
///
/// Seeds: `["permission", permission_authority, delegated_key]`.
pub fn derive_permission_address(
    program_id: &Pubkey,
    permission_authority: &Pubkey,
    delegated_key: &Pubkey,
) -> Pubkey {
    let (pda, _bump) = Pubkey::find_program_address(
        &[
            b"permission",
            permission_authority.as_ref(),
            delegated_key.as_ref(),
        ],
        program_id,
    );
    pda
}

/// Derives the spline collection PDA for a given market address.
///
/// Seeds: `["spline", market]`.
pub fn derive_spline_collection_address(program_id: &Pubkey, market: &Pubkey) -> Pubkey {
    let (pda, _bump) = Pubkey::find_program_address(&[b"spline", market.as_ref()], program_id);
    pda
}

/// Derives the global vault PDA for a given mint.
///
/// Seeds: `["vault", mint]`.
pub fn derive_global_vault_address(program_id: &Pubkey, mint: &Pubkey) -> Pubkey {
    let (pda, _bump) = Pubkey::find_program_address(&[b"vault", mint.as_ref()], program_id);
    pda
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phoenix_account_pdas_are_deterministic_and_seed_sensitive() {
        let program_id = Pubkey::new_unique();
        let trader = Pubkey::new_unique();
        let other_trader = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let delegate = Pubkey::new_unique();
        let market = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        assert_eq!(
            derive_stop_loss_address(&program_id, &trader, 1),
            derive_stop_loss_address(&program_id, &trader, 1)
        );
        assert_ne!(
            derive_stop_loss_address(&program_id, &trader, 1),
            derive_stop_loss_address(&program_id, &trader, 2)
        );
        assert_ne!(
            derive_conditional_orders_address(&program_id, &trader),
            derive_conditional_orders_address(&program_id, &other_trader)
        );
        assert_eq!(
            derive_permission_address(&program_id, &authority, &delegate),
            derive_permission_address(&program_id, &authority, &delegate)
        );
        assert_eq!(
            derive_spline_collection_address(&program_id, &market),
            derive_spline_collection_address(&program_id, &market)
        );
        assert_eq!(
            derive_global_vault_address(&program_id, &mint),
            derive_global_vault_address(&program_id, &mint)
        );
    }
}
