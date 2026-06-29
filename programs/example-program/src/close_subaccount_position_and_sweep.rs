use phoenix_rise::ix;
use phoenix_rise::ix::constants::PHOENIX_PROGRAM_ID;
use pinocchio::account_info::AccountInfo;
use pinocchio::program_error::ProgramError;
use pinocchio::{ProgramResult, msg};

use crate::common::dynamic_tail;
use crate::cpi::{MAX_CPI_ACCOUNTS, check_program_id};
use crate::market::{MarketAccountRefs, MarketContext};
use crate::params::SubaccountCloseAndSweepParams;

pub(crate) fn process(
    accounts: &[AccountInfo],
    params: &SubaccountCloseAndSweepParams,
) -> ProgramResult {
    let context = SubaccountCloseContext::load(
        accounts,
        params.global_trader_index_count as usize,
        params.active_trader_buffer_count as usize,
    )?;
    context.invoke(params)
}

struct SubaccountCloseContext<'a> {
    phoenix_program: &'a AccountInfo,
    hawkeye_program: &'a AccountInfo,
    log_authority: &'a AccountInfo,
    global_config: &'a AccountInfo,
    trader_authority: &'a AccountInfo,
    child_trader_account: &'a AccountInfo,
    parent_trader_account: &'a AccountInfo,
    perp_asset_map: &'a AccountInfo,
    orderbook: &'a AccountInfo,
    spline_collection: &'a AccountInfo,
    global_trader_index: &'a [AccountInfo],
    active_trader_buffer: &'a [AccountInfo],
}

impl<'a> SubaccountCloseContext<'a> {
    const FIXED_ACCOUNT_COUNT: usize = 10;

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
            hawkeye_program: &accounts[1],
            log_authority: &accounts[2],
            global_config: &accounts[3],
            trader_authority: &accounts[4],
            child_trader_account: &accounts[5],
            parent_trader_account: &accounts[6],
            perp_asset_map: &accounts[7],
            orderbook: &accounts[8],
            spline_collection: &accounts[9],
            global_trader_index,
            active_trader_buffer,
        };
        context.validate()?;
        Ok(context)
    }

    fn validate(&self) -> ProgramResult {
        check_program_id(self.phoenix_program, &PHOENIX_PROGRAM_ID, "Phoenix")?;
        check_program_id(self.hawkeye_program, &ix::HAWKEYE_PROGRAM_ID, "Hawkeye")?;
        if !self.trader_authority.is_signer() {
            msg!("rise-example-program: trader authority must sign subaccount close");
            return Err(ProgramError::MissingRequiredSignature);
        }
        Ok(())
    }

    fn invoke(&self, params: &SubaccountCloseAndSweepParams) -> ProgramResult {
        let market_context = MarketContext::from_refs(MarketAccountRefs {
            phoenix_program: self.phoenix_program,
            hawkeye_program: self.hawkeye_program,
            log_authority: self.log_authority,
            global_config: self.global_config,
            trader: self.trader_authority,
            trader_account: self.child_trader_account,
            perp_asset_map: self.perp_asset_map,
            orderbook: self.orderbook,
            spline_collection: self.spline_collection,
            global_trader_index: self.global_trader_index,
            active_trader_buffer: self.active_trader_buffer,
        })?;
        market_context
            .invoke_market_order_without_arenas(&params.order, "subaccount-close-order")?;

        msg!("rise-example-program: Phoenix sweep child collateral to parent");
        let transfer = ix::cpi::phoenix::TransferCollateralChildToParent {
            phoenix_program: self.phoenix_program,
            log_authority: self.log_authority,
            global_config: self.global_config,
            trader: self.trader_authority,
            child_trader_account: self.child_trader_account,
            parent_trader_account: self.parent_trader_account,
            perp_asset_map: self.perp_asset_map,
            global_trader_index: self.global_trader_index,
            active_trader_buffer: self.active_trader_buffer,
        };
        let mut scratch = ix::cpi::CpiScratch::<
            { MAX_CPI_ACCOUNTS },
            { ix::cpi::phoenix::TransferCollateralChildToParent::DATA_LEN },
        >::new(self.phoenix_program);
        transfer.invoke(
            ix::cpi::phoenix::TransferCollateralChildToParentArgs,
            &mut scratch,
        )
    }
}
