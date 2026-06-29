use phoenix_rise::cpi as rise_cpi;
use phoenix_rise::ix::PhoenixIxError;
use phoenix_rise::ix::types::Instruction;
use pinocchio::account_info::AccountInfo;
use pinocchio::instruction::AccountMeta;
use pinocchio::program_error::ProgramError;
use pinocchio::pubkey::Pubkey;
use pinocchio::{ProgramResult, msg};
use solana_pubkey::Pubkey as SolanaPubkey;

pub(crate) const MAX_CPI_ACCOUNTS: usize = 64;
pub(crate) const MAX_FLIGHT_PROXY_ACCOUNTS: usize = 96;

pub(crate) fn invoke_rise_ix<const MAX_ACCOUNTS: usize>(
    ix: &Instruction,
    program: &AccountInfo,
    accounts: &[&AccountInfo],
) -> ProgramResult {
    let mut metas: [AccountMeta<'_>; MAX_ACCOUNTS] =
        core::array::from_fn(|_| AccountMeta::readonly(program.key()));

    if let Err(error) = rise_cpi::invoke_instruction(ix, program, accounts, &mut metas) {
        match error {
            ProgramError::IncorrectProgramId => {
                msg!("close-position-and-withdraw: CPI program account mismatch");
            }
            ProgramError::NotEnoughAccountKeys => {
                msg!("close-position-and-withdraw: CPI account count mismatch");
            }
            ProgramError::InvalidAccountData => {
                msg!("close-position-and-withdraw: CPI account key mismatch");
            }
            _ => {}
        }
        return Err(error);
    }
    Ok(())
}

pub(crate) fn check_key(account: &AccountInfo, expected: &SolanaPubkey) -> ProgramResult {
    if to_solana_pubkey(account.key()) != *expected {
        msg!("close-position-and-withdraw: account key mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

pub(crate) fn check_program_id(
    account: &AccountInfo,
    expected: &SolanaPubkey,
    label: &'static str,
) -> ProgramResult {
    if to_solana_pubkey(account.key()) != *expected {
        msg!(&format!(
            "close-position-and-withdraw: {label} program id mismatch"
        ));
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

pub(crate) fn to_solana_pubkeys(accounts: &[AccountInfo]) -> Vec<SolanaPubkey> {
    accounts
        .iter()
        .map(|account| to_solana_pubkey(account.key()))
        .collect()
}

pub(crate) fn to_solana_pubkey(key: &Pubkey) -> SolanaPubkey {
    SolanaPubkey::new_from_array(*key)
}

pub(crate) fn map_ix_error(_error: PhoenixIxError) -> ProgramError {
    ProgramError::InvalidInstructionData
}
