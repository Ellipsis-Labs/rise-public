//! Create Associated Token Account instruction construction.

use solana_pubkey::Pubkey;

use crate::constants::{
    ASSOCIATED_TOKEN_PROGRAM_ID, SPL_TOKEN_PROGRAM_ID, SYSTEM_PROGRAM_ID,
    get_associated_token_address,
};
use crate::error::PhoenixIxError;
use crate::types::{AccountMeta, Instruction};

/// Create an idempotent Associated Token Account instruction.
///
/// This instruction creates an ATA for the owner if it doesn't exist,
/// and does nothing if it already exists.
///
/// # Arguments
///
/// * `payer` - The account that will pay for the ATA creation
/// * `owner` - The owner of the ATA
/// * `mint` - The token mint
///
/// # Returns
///
/// A Solana instruction ready to be included in a transaction.
pub fn create_associated_token_account_idempotent_ix(
    payer: Pubkey,
    owner: Pubkey,
    mint: Pubkey,
) -> Result<Instruction, PhoenixIxError> {
    let ata = get_associated_token_address(&owner, &mint)?;

    let accounts = vec![
        AccountMeta::writable_signer(payer),
        AccountMeta::writable(ata),
        AccountMeta::readonly(owner),
        AccountMeta::readonly(mint),
        AccountMeta::readonly(SYSTEM_PROGRAM_ID),
        AccountMeta::readonly(SPL_TOKEN_PROGRAM_ID),
    ];

    // CreateIdempotent discriminant is 1
    let data = vec![1u8];

    Ok(Instruction {
        program_id: ASSOCIATED_TOKEN_PROGRAM_ID,
        accounts,
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_ata_idempotent_ix() {
        let payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let ix = create_associated_token_account_idempotent_ix(payer, owner, mint).unwrap();

        assert_eq!(ix.program_id, ASSOCIATED_TOKEN_PROGRAM_ID);
        assert_eq!(ix.accounts.len(), 6);
        assert_eq!(ix.data, vec![1u8]);

        // Verify account order
        assert_eq!(ix.accounts[0].pubkey, payer);
        assert!(ix.accounts[0].is_signer);
        assert!(ix.accounts[0].is_writable);

        // ATA should be writable but not signer
        let expected_ata = get_associated_token_address(&owner, &mint).unwrap();
        assert_eq!(ix.accounts[1].pubkey, expected_ata);
        assert!(ix.accounts[1].is_writable);
        assert!(!ix.accounts[1].is_signer);

        // Owner should be readonly
        assert_eq!(ix.accounts[2].pubkey, owner);
        assert!(!ix.accounts[2].is_writable);

        // Mint should be readonly
        assert_eq!(ix.accounts[3].pubkey, mint);
        assert!(!ix.accounts[3].is_writable);
    }
}
