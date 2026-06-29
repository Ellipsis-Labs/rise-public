use pinocchio::account_info::AccountInfo;
use pinocchio::program_error::ProgramError;
use pinocchio::pubkey::Pubkey;
use pinocchio::{ProgramResult, msg};
use solana_pubkey::Pubkey as SolanaPubkey;

// Keep CPI scratch buffers small enough for BPF stack frames in multi-CPI
// example instructions. The default LiteSVM fixture uses 1 global trader index
// and 1 active trader buffer account; 16 accounts covers the current examples.
pub(crate) const MAX_CPI_ACCOUNTS: usize = 16;

pub(crate) fn check_program_id(
    account: &AccountInfo,
    expected: &SolanaPubkey,
    label: &'static str,
) -> ProgramResult {
    if to_solana_pubkey(account.key()) != *expected {
        msg!(&format!(
            "rise-example-program: {label} program id mismatch"
        ));
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

pub(crate) fn to_solana_pubkey(key: &Pubkey) -> SolanaPubkey {
    SolanaPubkey::new_from_array(*key)
}
