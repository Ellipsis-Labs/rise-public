use pinocchio::account_info::AccountInfo;
use pinocchio::program_error::ProgramError;
use pinocchio::{ProgramResult, msg};

use crate::common::SharedAccounts;

pub(crate) struct WithdrawAllCollateralContext<'a> {
    common: SharedAccounts<'a>,
}

impl<'a> WithdrawAllCollateralContext<'a> {
    pub(crate) fn load(
        accounts: &'a [AccountInfo],
        global_trader_index_count: usize,
        active_trader_buffer_count: usize,
    ) -> Result<Self, ProgramError> {
        let expected = SharedAccounts::FIXED_ACCOUNT_COUNT
            .checked_add(global_trader_index_count)
            .and_then(|count| count.checked_add(active_trader_buffer_count))
            .ok_or(ProgramError::InvalidInstructionData)?;
        if accounts.len() != expected {
            msg!("close-position-and-withdraw: invalid account count");
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        let context = Self {
            common: SharedAccounts::load(
                accounts,
                SharedAccounts::FIXED_ACCOUNT_COUNT,
                global_trader_index_count,
                active_trader_buffer_count,
            )?,
        };
        context.common.validate()?;
        Ok(context)
    }

    pub(crate) fn invoke(&self) -> ProgramResult {
        self.common.log_trader_status()?;
        let amount = self
            .common
            .read_and_log_withdrawable_collateral("before withdraw")?;
        self.common.withdraw(amount)
    }
}
