use phoenix_rise::ix;
use phoenix_rise::ix::constants::{EMBER_PROGRAM_ID, PHOENIX_PROGRAM_ID, SPL_TOKEN_PROGRAM_ID};
use pinocchio::account_info::AccountInfo;
use pinocchio::program_error::ProgramError;
use pinocchio::{ProgramResult, msg};

use crate::common::dynamic_tail;
use crate::cpi::{MAX_CPI_ACCOUNTS, check_program_id};
use crate::params::DepositEmberThenPhoenixParams;

pub(crate) fn process(
    accounts: &[AccountInfo],
    params: &DepositEmberThenPhoenixParams,
) -> ProgramResult {
    let context = DepositContext::load(
        accounts,
        params.global_trader_index_count as usize,
        params.active_trader_buffer_count as usize,
    )?;
    context.invoke(params)
}

struct DepositContext<'a> {
    ember_program: &'a AccountInfo,
    trader: &'a AccountInfo,
    ember_state: &'a AccountInfo,
    usdc_mint: &'a AccountInfo,
    canonical_mint: &'a AccountInfo,
    trader_usdc_account: &'a AccountInfo,
    trader_phoenix_account: &'a AccountInfo,
    ember_vault: &'a AccountInfo,
    token_program: &'a AccountInfo,
    phoenix_program: &'a AccountInfo,
    log_authority: &'a AccountInfo,
    global_config: &'a AccountInfo,
    trader_account: &'a AccountInfo,
    global_vault: &'a AccountInfo,
    global_trader_index: &'a [AccountInfo],
    active_trader_buffer: &'a [AccountInfo],
}

impl<'a> DepositContext<'a> {
    const FIXED_ACCOUNT_COUNT: usize = 14;

    fn load(
        accounts: &'a [AccountInfo],
        global_trader_index_count: usize,
        active_trader_buffer_count: usize,
    ) -> Result<Self, ProgramError> {
        let (global_trader_index, active_trader_buffer) = dynamic_tail(
            accounts,
            Self::FIXED_ACCOUNT_COUNT,
            global_trader_index_count,
            active_trader_buffer_count,
        )?;
        let context = Self {
            ember_program: &accounts[0],
            trader: &accounts[1],
            ember_state: &accounts[2],
            usdc_mint: &accounts[3],
            canonical_mint: &accounts[4],
            trader_usdc_account: &accounts[5],
            trader_phoenix_account: &accounts[6],
            ember_vault: &accounts[7],
            token_program: &accounts[8],
            phoenix_program: &accounts[9],
            log_authority: &accounts[10],
            global_config: &accounts[11],
            trader_account: &accounts[12],
            global_vault: &accounts[13],
            global_trader_index,
            active_trader_buffer,
        };
        context.validate()?;
        Ok(context)
    }

    fn validate(&self) -> ProgramResult {
        check_program_id(self.ember_program, &EMBER_PROGRAM_ID, "Ember")?;
        check_program_id(self.phoenix_program, &PHOENIX_PROGRAM_ID, "Phoenix")?;
        check_program_id(self.token_program, &SPL_TOKEN_PROGRAM_ID, "SPL Token")?;
        if !self.trader.is_signer() {
            msg!("rise-example-program: trader must sign deposit");
            return Err(ProgramError::MissingRequiredSignature);
        }
        Ok(())
    }

    fn invoke(&self, params: &DepositEmberThenPhoenixParams) -> ProgramResult {
        msg!("rise-example-program: Ember deposit");
        let ember_deposit = ix::cpi::ember::EmberDeposit {
            ember_program: self.ember_program,
            trader: self.trader,
            ember_state: self.ember_state,
            usdc_mint: self.usdc_mint,
            canonical_mint: self.canonical_mint,
            trader_usdc_account: self.trader_usdc_account,
            trader_phoenix_account: self.trader_phoenix_account,
            ember_vault: self.ember_vault,
            token_program: self.token_program,
        };
        let mut ember_scratch = ix::cpi::CpiScratch::<
            { ix::cpi::ember::EmberDeposit::ACCOUNT_COUNT },
            { ix::cpi::ember::EmberDeposit::DATA_LEN },
        >::new(self.ember_program);
        ember_deposit.invoke(
            ix::cpi::ember::EmberDepositArgs {
                amount: params.ember_amount,
            },
            &mut ember_scratch,
        )?;

        msg!("rise-example-program: Phoenix deposit");
        let phoenix_deposit = ix::cpi::phoenix::PhoenixDeposit {
            phoenix_program: self.phoenix_program,
            log_authority: self.log_authority,
            global_config: self.global_config,
            trader: self.trader,
            trader_token_account: self.trader_phoenix_account,
            trader_account: self.trader_account,
            global_vault: self.global_vault,
            token_program: self.token_program,
            global_trader_index: self.global_trader_index,
            active_trader_buffer: self.active_trader_buffer,
            permission_account: None,
        };
        let mut scratch = ix::cpi::CpiScratch::<
            { MAX_CPI_ACCOUNTS },
            { ix::cpi::phoenix::PhoenixDeposit::DATA_LEN },
        >::new(self.phoenix_program);
        phoenix_deposit.invoke(
            ix::cpi::phoenix::PhoenixDepositArgs {
                amount: params.phoenix_amount,
            },
            &mut scratch,
        )
    }
}
