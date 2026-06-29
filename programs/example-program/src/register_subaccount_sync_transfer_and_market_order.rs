use phoenix_rise::ix;
use phoenix_rise::ix::constants::{PHOENIX_PROGRAM_ID, SYSTEM_PROGRAM_ID};
use pinocchio::account_info::AccountInfo;
use pinocchio::program_error::ProgramError;
use pinocchio::{ProgramResult, msg};

use crate::common::dynamic_tail;
use crate::cpi::{MAX_CPI_ACCOUNTS, check_program_id};
use crate::market::{MarketAccountRefs, MarketContext};
use crate::params::SubaccountOpenAndTradeParams;

pub(crate) fn process(
    accounts: &[AccountInfo],
    params: &SubaccountOpenAndTradeParams,
) -> ProgramResult {
    let context = SubaccountOpenContext::load(
        accounts,
        params.global_trader_index_count as usize,
        params.active_trader_buffer_count as usize,
    )?;
    context.invoke(params)
}

struct SubaccountOpenContext<'a> {
    phoenix_program: &'a AccountInfo,
    hawkeye_program: &'a AccountInfo,
    log_authority: &'a AccountInfo,
    global_config: &'a AccountInfo,
    payer: &'a AccountInfo,
    trader_authority: &'a AccountInfo,
    parent_trader_account: &'a AccountInfo,
    child_trader_account: &'a AccountInfo,
    system_program: &'a AccountInfo,
    perp_asset_map: &'a AccountInfo,
    orderbook: &'a AccountInfo,
    spline_collection: &'a AccountInfo,
    global_trader_index: &'a [AccountInfo],
    active_trader_buffer: &'a [AccountInfo],
}

impl<'a> SubaccountOpenContext<'a> {
    const FIXED_ACCOUNT_COUNT: usize = 12;

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
            payer: &accounts[4],
            trader_authority: &accounts[5],
            parent_trader_account: &accounts[6],
            child_trader_account: &accounts[7],
            system_program: &accounts[8],
            perp_asset_map: &accounts[9],
            orderbook: &accounts[10],
            spline_collection: &accounts[11],
            global_trader_index,
            active_trader_buffer,
        };
        context.validate()?;
        Ok(context)
    }

    fn validate(&self) -> ProgramResult {
        check_program_id(self.phoenix_program, &PHOENIX_PROGRAM_ID, "Phoenix")?;
        check_program_id(self.hawkeye_program, &ix::HAWKEYE_PROGRAM_ID, "Hawkeye")?;
        check_program_id(self.system_program, &SYSTEM_PROGRAM_ID, "System")?;
        if !self.payer.is_signer() || !self.trader_authority.is_signer() {
            msg!("rise-example-program: payer and trader authority must sign subaccount open");
            return Err(ProgramError::MissingRequiredSignature);
        }
        Ok(())
    }

    fn invoke(&self, params: &SubaccountOpenAndTradeParams) -> ProgramResult {
        msg!("rise-example-program: Phoenix register subaccount");
        let register = ix::cpi::phoenix::RegisterTrader {
            phoenix_program: self.phoenix_program,
            log_authority: self.log_authority,
            global_config: self.global_config,
            payer: self.payer,
            trader: self.trader_authority,
            trader_account: self.child_trader_account,
            system_program: self.system_program,
        };
        let mut register_scratch = ix::cpi::CpiScratch::<
            { ix::cpi::phoenix::RegisterTrader::ACCOUNT_COUNT },
            { ix::cpi::phoenix::RegisterTrader::DATA_LEN },
        >::new(self.phoenix_program);
        register.invoke(
            ix::cpi::phoenix::RegisterTraderArgs {
                max_positions: params.max_positions,
                trader_pda_index: params.trader_pda_index,
                subaccount_index: params.child_subaccount_index,
            },
            &mut register_scratch,
        )?;

        msg!("rise-example-program: Phoenix sync parent to child");
        let sync = ix::cpi::phoenix::SyncParentToChild {
            phoenix_program: self.phoenix_program,
            log_authority: self.log_authority,
            global_config: self.global_config,
            trader_wallet: self.trader_authority,
            parent_trader_account: self.parent_trader_account,
            child_trader_account: self.child_trader_account,
            global_trader_index: self.global_trader_index,
        };
        let mut sync_scratch = ix::cpi::CpiScratch::<
            { MAX_CPI_ACCOUNTS },
            { ix::cpi::phoenix::SyncParentToChild::DATA_LEN },
        >::new(self.phoenix_program);
        sync.invoke(ix::cpi::phoenix::SyncParentToChildArgs, &mut sync_scratch)?;

        msg!("rise-example-program: Phoenix transfer collateral to child");
        let transfer = ix::cpi::phoenix::TransferCollateral {
            phoenix_program: self.phoenix_program,
            log_authority: self.log_authority,
            global_config: self.global_config,
            trader: self.trader_authority,
            src_trader_account: self.parent_trader_account,
            dst_trader_account: self.child_trader_account,
            perp_asset_map: self.perp_asset_map,
            global_trader_index: self.global_trader_index,
            active_trader_buffer: self.active_trader_buffer,
            permission_account: None,
        };
        let mut transfer_scratch = ix::cpi::CpiScratch::<
            { MAX_CPI_ACCOUNTS },
            { ix::cpi::phoenix::TransferCollateral::DATA_LEN },
        >::new(self.phoenix_program);
        transfer.invoke(
            ix::cpi::phoenix::TransferCollateralArgs {
                amount: params.transfer_amount,
            },
            &mut transfer_scratch,
        )?;

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
        market_context.invoke_market_order_without_arenas(&params.order, "subaccount-open-order")
    }
}
