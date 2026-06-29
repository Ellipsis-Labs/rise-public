use phoenix_rise::ix;
use phoenix_rise::ix::constants::{PHOENIX_PROGRAM_ID, SYSTEM_PROGRAM_ID};
use pinocchio::account_info::AccountInfo;
use pinocchio::program_error::ProgramError;
use pinocchio::{ProgramResult, msg};

use crate::common::dynamic_tail;
use crate::cpi::{MAX_CPI_ACCOUNTS, check_program_id};
use crate::params::StopLossCpiParams;

pub(crate) fn process(accounts: &[AccountInfo], params: &StopLossCpiParams) -> ProgramResult {
    let context = StopLossContext::load(
        accounts,
        params.global_trader_index_count as usize,
        params.active_trader_buffer_count as usize,
    )?;
    context.invoke_place(params)
}

struct StopLossContext<'a> {
    phoenix_program: &'a AccountInfo,
    log_authority: &'a AccountInfo,
    global_config: &'a AccountInfo,
    funder: &'a AccountInfo,
    trader_account: &'a AccountInfo,
    perp_asset_map: &'a AccountInfo,
    orderbook: &'a AccountInfo,
    spline_collection: &'a AccountInfo,
    position_authority: &'a AccountInfo,
    stop_loss_account: &'a AccountInfo,
    system_program: &'a AccountInfo,
    global_trader_index: &'a [AccountInfo],
    active_trader_buffer: &'a [AccountInfo],
}

impl<'a> StopLossContext<'a> {
    const FIXED_ACCOUNT_COUNT: usize = 11;

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
            funder: &accounts[3],
            trader_account: &accounts[4],
            perp_asset_map: &accounts[5],
            orderbook: &accounts[6],
            spline_collection: &accounts[7],
            position_authority: &accounts[8],
            stop_loss_account: &accounts[9],
            system_program: &accounts[10],
            global_trader_index,
            active_trader_buffer,
        };
        context.validate()?;
        Ok(context)
    }

    fn validate(&self) -> ProgramResult {
        check_program_id(self.phoenix_program, &PHOENIX_PROGRAM_ID, "Phoenix")?;
        check_program_id(self.system_program, &SYSTEM_PROGRAM_ID, "System")?;
        if !self.funder.is_signer() || !self.position_authority.is_signer() {
            msg!("rise-example-program: funder and position authority must sign stop loss");
            return Err(ProgramError::MissingRequiredSignature);
        }
        Ok(())
    }

    fn invoke_place(&self, params: &StopLossCpiParams) -> ProgramResult {
        msg!("rise-example-program: Phoenix place stop loss");
        let place_stop_loss = ix::cpi::phoenix::PlaceStopLoss {
            phoenix_program: self.phoenix_program,
            log_authority: self.log_authority,
            global_config: self.global_config,
            funder: self.funder,
            trader_account: self.trader_account,
            perp_asset_map: self.perp_asset_map,
            global_trader_index: self.global_trader_index,
            active_trader_buffer: self.active_trader_buffer,
            orderbook: self.orderbook,
            spline_collection: self.spline_collection,
            position_authority: self.position_authority,
            stop_loss_account: self.stop_loss_account,
            system_program: self.system_program,
        };
        let mut scratch = ix::cpi::CpiScratch::<
            { MAX_CPI_ACCOUNTS },
            { ix::cpi::phoenix::PlaceStopLoss::DATA_LEN },
        >::new(self.phoenix_program);
        place_stop_loss.invoke(
            ix::cpi::phoenix::PlaceStopLossArgs {
                trigger_price: params.trigger_price,
                execution_price: params.execution_price,
                trade_side: params.trade_side,
                execution_direction: params.execution_direction,
                order_kind: params.order_kind,
            },
            &mut scratch,
        )
    }
}
