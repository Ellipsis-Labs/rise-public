mod account_view;
mod create_trader_idempotent;
mod error;
mod register_subaccount_idempotent;

use create_trader_idempotent::process_create_trader_idempotent;
pub use error::TraderOnboarderError;
use phoenix_rise::ix::constants::compute_discriminant;
use pinocchio::account_info::AccountInfo;
use pinocchio::program_error::ProgramError;
use pinocchio::pubkey::Pubkey;
use pinocchio::{ProgramResult, entrypoint, msg};
use register_subaccount_idempotent::process_register_subaccount_idempotent;

pinocchio_pubkey::declare_id!("9AuJXxrJqWjG34BJD5k5xKY77LGQz8xa8kY5dLRFdW2i");

pub const TRADER_ONBOARDER_AUTHORITY_SEED: &[u8] = b"trader-onboarder";

entrypoint!(process_instruction);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TraderOnboarderInstruction {
    CreateTraderIdempotent,
    RegisterSubaccountIdempotent,
}

impl TraderOnboarderInstruction {
    fn from_tag(tag: &[u8]) -> Option<Self> {
        if tag == compute_discriminant("global:create_trader_idempotent") {
            return Some(Self::CreateTraderIdempotent);
        }
        if tag == compute_discriminant("global:register_subaccount_idempotent") {
            return Some(Self::RegisterSubaccountIdempotent);
        }
        None
    }
}

#[inline(always)]
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if program_id != &ID {
        msg!("trader-onboarder: program id mismatch");
        return Err(TraderOnboarderError::InvalidAccountKey.into());
    }
    if instruction_data.len() < 8 {
        msg!("trader-onboarder: missing instruction discriminator");
        return Err(TraderOnboarderError::InvalidInstruction.into());
    }

    let (tag, data) = instruction_data.split_at(8);
    let instruction = TraderOnboarderInstruction::from_tag(tag).ok_or_else(|| {
        msg!("trader-onboarder: unknown instruction discriminator");
        ProgramError::from(TraderOnboarderError::InvalidInstruction)
    })?;

    match instruction {
        TraderOnboarderInstruction::CreateTraderIdempotent => {
            if !data.is_empty() {
                msg!("trader-onboarder: create-trader takes no params");
                return Err(TraderOnboarderError::InvalidInstruction.into());
            }
            process_create_trader_idempotent(accounts)
        }
        TraderOnboarderInstruction::RegisterSubaccountIdempotent => {
            let [subaccount_index] = data else {
                msg!("trader-onboarder: register-subaccount takes one u8 subaccount index");
                return Err(TraderOnboarderError::InvalidInstruction.into());
            };
            process_register_subaccount_idempotent(accounts, *subaccount_index)
        }
    }
}
