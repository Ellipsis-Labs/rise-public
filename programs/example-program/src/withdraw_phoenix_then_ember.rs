use phoenix_rise::ix;
use phoenix_rise::ix::constants::{EMBER_PROGRAM_ID, PHOENIX_PROGRAM_ID, SPL_TOKEN_PROGRAM_ID};
use pinocchio::account_info::AccountInfo;
use pinocchio::program_error::ProgramError;
use pinocchio::{ProgramResult, msg};

use crate::common::dynamic_tail;
use crate::cpi::{MAX_CPI_ACCOUNTS, check_program_id};
use crate::params::WithdrawPhoenixThenEmberParams;

pub(crate) fn process(
    accounts: &[AccountInfo],
    params: &WithdrawPhoenixThenEmberParams,
) -> ProgramResult {
    let context = WithdrawContext::load(
        accounts,
        params.global_trader_index_count as usize,
        params.active_trader_buffer_count as usize,
    )?;
    context.invoke(params)
}

struct WithdrawContext<'a> {
    phoenix_program: &'a AccountInfo,
    log_authority: &'a AccountInfo,
    global_config: &'a AccountInfo,
    trader: &'a AccountInfo,
    trader_account: &'a AccountInfo,
    perp_asset_map: &'a AccountInfo,
    global_vault: &'a AccountInfo,
    trader_phoenix_account: &'a AccountInfo,
    token_program: &'a AccountInfo,
    withdraw_queue: &'a AccountInfo,
    ember_program: &'a AccountInfo,
    ember_state: &'a AccountInfo,
    usdc_mint: &'a AccountInfo,
    canonical_mint: &'a AccountInfo,
    trader_usdc_account: &'a AccountInfo,
    ember_vault: &'a AccountInfo,
    global_trader_index: &'a [AccountInfo],
    active_trader_buffer: &'a [AccountInfo],
}

impl<'a> WithdrawContext<'a> {
    const FIXED_ACCOUNT_COUNT: usize = 16;

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
            phoenix_program: &accounts[0],
            log_authority: &accounts[1],
            global_config: &accounts[2],
            trader: &accounts[3],
            trader_account: &accounts[4],
            perp_asset_map: &accounts[5],
            global_vault: &accounts[6],
            trader_phoenix_account: &accounts[7],
            token_program: &accounts[8],
            withdraw_queue: &accounts[9],
            ember_program: &accounts[10],
            ember_state: &accounts[11],
            usdc_mint: &accounts[12],
            canonical_mint: &accounts[13],
            trader_usdc_account: &accounts[14],
            ember_vault: &accounts[15],
            global_trader_index,
            active_trader_buffer,
        };
        context.validate()?;
        Ok(context)
    }

    fn validate(&self) -> ProgramResult {
        check_program_id(self.phoenix_program, &PHOENIX_PROGRAM_ID, "Phoenix")?;
        check_program_id(self.ember_program, &EMBER_PROGRAM_ID, "Ember")?;
        check_program_id(self.token_program, &SPL_TOKEN_PROGRAM_ID, "SPL Token")?;
        if !self.trader.is_signer() {
            msg!("rise-example-program: trader must sign withdraw");
            return Err(ProgramError::MissingRequiredSignature);
        }
        Ok(())
    }

    fn invoke(&self, params: &WithdrawPhoenixThenEmberParams) -> ProgramResult {
        msg!("rise-example-program: Phoenix withdraw");
        let phoenix_withdraw = ix::cpi::phoenix::PhoenixWithdraw {
            phoenix_program: self.phoenix_program,
            log_authority: self.log_authority,
            global_config: self.global_config,
            trader: self.trader,
            trader_account: self.trader_account,
            perp_asset_map: self.perp_asset_map,
            global_vault: self.global_vault,
            trader_token_account: self.trader_phoenix_account,
            token_program: self.token_program,
            global_trader_index: self.global_trader_index,
            active_trader_buffer: self.active_trader_buffer,
            withdraw_queue: self.withdraw_queue,
        };
        let mut scratch = ix::cpi::CpiScratch::<
            { MAX_CPI_ACCOUNTS },
            { ix::cpi::phoenix::PhoenixWithdraw::DATA_LEN },
        >::new(self.phoenix_program);
        phoenix_withdraw.invoke(
            ix::cpi::phoenix::PhoenixWithdrawArgs {
                amount: params.phoenix_amount,
            },
            &mut scratch,
        )?;

        msg!("rise-example-program: Ember withdraw");
        let ember_withdraw = ix::cpi::ember::EmberWithdraw {
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
        let mut scratch = ix::cpi::CpiScratch::<
            { ix::cpi::ember::EmberWithdraw::ACCOUNT_COUNT },
            { ix::cpi::ember::EmberWithdraw::MAX_DATA_LEN },
        >::new(self.ember_program);
        ember_withdraw.invoke(
            ix::cpi::ember::EmberWithdrawArgs {
                amount: params.ember_amount,
            },
            &mut scratch,
        )
    }
}
