use phoenix_rise::ix;
use phoenix_rise::ix::constants::{PHOENIX_PROGRAM_ID, SYSTEM_PROGRAM_ID};
use pinocchio::account_info::AccountInfo;
use pinocchio::program_error::ProgramError;
use pinocchio::{ProgramResult, msg};

use crate::common::require_exact_accounts;
use crate::cpi::check_program_id;
use crate::params::CancelStopLossCpiParams;

pub(crate) fn process(accounts: &[AccountInfo], params: &CancelStopLossCpiParams) -> ProgramResult {
    let context = CancelStopLossContext::load(accounts)?;
    context.invoke(params)
}

struct CancelStopLossContext<'a> {
    phoenix_program: &'a AccountInfo,
    log_authority: &'a AccountInfo,
    global_config: &'a AccountInfo,
    funder: &'a AccountInfo,
    trader_account: &'a AccountInfo,
    position_authority: &'a AccountInfo,
    stop_loss_account: &'a AccountInfo,
    system_program: &'a AccountInfo,
}

impl<'a> CancelStopLossContext<'a> {
    const ACCOUNT_COUNT: usize = 8;

    fn load(accounts: &'a [AccountInfo]) -> Result<Self, ProgramError> {
        require_exact_accounts(accounts, Self::ACCOUNT_COUNT)?;
        let context = Self {
            phoenix_program: &accounts[0],
            log_authority: &accounts[1],
            global_config: &accounts[2],
            funder: &accounts[3],
            trader_account: &accounts[4],
            position_authority: &accounts[5],
            stop_loss_account: &accounts[6],
            system_program: &accounts[7],
        };
        context.validate()?;
        Ok(context)
    }

    fn validate(&self) -> ProgramResult {
        check_program_id(self.phoenix_program, &PHOENIX_PROGRAM_ID, "Phoenix")?;
        check_program_id(self.system_program, &SYSTEM_PROGRAM_ID, "System")?;
        if !self.position_authority.is_signer() {
            msg!("rise-example-program: position authority must sign stop-loss cancellation");
            return Err(ProgramError::MissingRequiredSignature);
        }
        Ok(())
    }

    fn invoke(&self, params: &CancelStopLossCpiParams) -> ProgramResult {
        msg!("rise-example-program: Phoenix cancel stop loss");
        let cancel_stop_loss = ix::cpi::phoenix::CancelStopLoss {
            phoenix_program: self.phoenix_program,
            log_authority: self.log_authority,
            global_config: self.global_config,
            funder: self.funder,
            trader_account: self.trader_account,
            position_authority: self.position_authority,
            stop_loss_account: self.stop_loss_account,
            system_program: self.system_program,
        };
        let mut scratch = ix::cpi::CpiScratch::<
            { ix::cpi::phoenix::CancelStopLoss::ACCOUNT_COUNT },
            { ix::cpi::phoenix::CancelStopLoss::DATA_LEN },
        >::new(self.phoenix_program);
        cancel_stop_loss.invoke(
            ix::cpi::phoenix::CancelStopLossArgs {
                execution_direction: params.execution_direction,
            },
            &mut scratch,
        )
    }
}
