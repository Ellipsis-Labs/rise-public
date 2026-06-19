use phoenix_rise::ix;
use pinocchio::account_info::AccountInfo;
use pinocchio::cpi::slice_invoke;
use pinocchio::instruction::{AccountMeta, Instruction};
use pinocchio::program_error::ProgramError;
use pinocchio::pubkey::Pubkey;
use pinocchio::{ProgramResult, msg};
use solana_pubkey::Pubkey as SolanaPubkey;

pub(crate) fn invoke_rise_ix(
    ix: &ix::Instruction,
    program: &AccountInfo,
    accounts: &[&AccountInfo],
) -> ProgramResult {
    if to_solana_pubkey(program.key()) != ix.program_id {
        msg!("close-position-and-withdraw: CPI program account mismatch");
        return Err(ProgramError::IncorrectProgramId);
    }
    if ix.accounts.len() != accounts.len() {
        msg!("close-position-and-withdraw: CPI account count mismatch");
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let mut metas = Vec::with_capacity(ix.accounts.len());
    for (meta, account) in ix.accounts.iter().zip(accounts.iter()) {
        if meta.pubkey != to_solana_pubkey(account.key()) {
            msg!("close-position-and-withdraw: CPI account key mismatch");
            return Err(ProgramError::InvalidAccountData);
        }
        metas.push(AccountMeta::new(
            account.key(),
            meta.is_writable,
            meta.is_signer,
        ));
    }

    let program_id = ix.program_id.to_bytes();
    let instruction = Instruction {
        program_id: &program_id,
        accounts: &metas,
        data: &ix.data,
    };
    slice_invoke(&instruction, accounts)
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

pub(crate) fn to_solana_pubkeys(accounts: &[&AccountInfo]) -> Vec<SolanaPubkey> {
    accounts
        .iter()
        .map(|account| to_solana_pubkey(account.key()))
        .collect()
}

pub(crate) fn to_solana_pubkey(key: &Pubkey) -> SolanaPubkey {
    SolanaPubkey::new_from_array(*key)
}

pub(crate) fn map_ix_error(_error: ix::PhoenixIxError) -> ProgramError {
    ProgramError::InvalidInstructionData
}
