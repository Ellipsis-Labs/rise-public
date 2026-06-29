use phoenix_rise::ix;
use phoenix_rise::ix::types::{CancelId, FifoOrderId};
use pinocchio::account_info::AccountInfo;
use pinocchio::program_error::ProgramError;
use pinocchio::{ProgramResult, msg};

use crate::common::{dynamic_tail, log_matching_response};
use crate::cpi::{MAX_CPI_ACCOUNTS, check_program_id};
use crate::params::{
    CancelLimitOrderParams, LimitOrderCpiParams, MarketOrderCpiParams, MarketOrderWithoutArenas,
};

pub(crate) struct MarketAccountRefs<'a> {
    pub(crate) phoenix_program: &'a AccountInfo,
    pub(crate) hawkeye_program: &'a AccountInfo,
    pub(crate) log_authority: &'a AccountInfo,
    pub(crate) global_config: &'a AccountInfo,
    pub(crate) trader: &'a AccountInfo,
    pub(crate) trader_account: &'a AccountInfo,
    pub(crate) perp_asset_map: &'a AccountInfo,
    pub(crate) orderbook: &'a AccountInfo,
    pub(crate) spline_collection: &'a AccountInfo,
    pub(crate) global_trader_index: &'a [AccountInfo],
    pub(crate) active_trader_buffer: &'a [AccountInfo],
}

pub(crate) struct MarketContext<'a> {
    phoenix_program: &'a AccountInfo,
    hawkeye_program: &'a AccountInfo,
    log_authority: &'a AccountInfo,
    global_config: &'a AccountInfo,
    trader: &'a AccountInfo,
    trader_account: &'a AccountInfo,
    perp_asset_map: &'a AccountInfo,
    orderbook: &'a AccountInfo,
    spline_collection: &'a AccountInfo,
    global_trader_index: &'a [AccountInfo],
    active_trader_buffer: &'a [AccountInfo],
}

impl<'a> MarketContext<'a> {
    const FIXED_ACCOUNT_COUNT: usize = 9;

    pub(crate) fn load(
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
        Self::from_refs(MarketAccountRefs {
            phoenix_program: &accounts[0],
            hawkeye_program: &accounts[1],
            log_authority: &accounts[2],
            global_config: &accounts[3],
            trader: &accounts[4],
            trader_account: &accounts[5],
            perp_asset_map: &accounts[6],
            orderbook: &accounts[7],
            spline_collection: &accounts[8],
            global_trader_index,
            active_trader_buffer,
        })
    }

    pub(crate) fn from_refs(accounts: MarketAccountRefs<'a>) -> Result<Self, ProgramError> {
        let context = Self {
            phoenix_program: accounts.phoenix_program,
            hawkeye_program: accounts.hawkeye_program,
            log_authority: accounts.log_authority,
            global_config: accounts.global_config,
            trader: accounts.trader,
            trader_account: accounts.trader_account,
            perp_asset_map: accounts.perp_asset_map,
            orderbook: accounts.orderbook,
            spline_collection: accounts.spline_collection,
            global_trader_index: accounts.global_trader_index,
            active_trader_buffer: accounts.active_trader_buffer,
        };
        context.validate()?;
        Ok(context)
    }

    fn validate(&self) -> ProgramResult {
        check_program_id(self.phoenix_program, &ix::PHOENIX_PROGRAM_ID, "Phoenix")?;
        check_program_id(self.hawkeye_program, &ix::HAWKEYE_PROGRAM_ID, "Hawkeye")?;
        if !self.trader.is_signer() {
            msg!("rise-example-program: trader must sign market action");
            return Err(ProgramError::MissingRequiredSignature);
        }
        Ok(())
    }

    fn market_accounts(&self) -> ix::cpi::phoenix::PlaceMarketOrder<'a> {
        ix::cpi::phoenix::PlaceMarketOrder {
            phoenix_program: self.phoenix_program,
            log_authority: self.log_authority,
            global_config: self.global_config,
            trader: self.trader,
            trader_account: self.trader_account,
            perp_asset_map: self.perp_asset_map,
            global_trader_index: self.global_trader_index,
            active_trader_buffer: self.active_trader_buffer,
            orderbook: self.orderbook,
            spline_collection: self.spline_collection,
        }
    }

    pub(crate) fn invoke_market_order(
        &self,
        params: &MarketOrderCpiParams,
        label: &str,
    ) -> ProgramResult {
        msg!("rise-example-program: Phoenix place market order");
        let market_order = self.market_accounts();
        let mut scratch = ix::cpi::CpiScratch::<
            { MAX_CPI_ACCOUNTS },
            { ix::cpi::phoenix::PlaceMarketOrder::MAX_DATA_LEN },
        >::new(self.phoenix_program);
        market_order.invoke(
            ix::cpi::phoenix::PlaceMarketOrderArgs {
                side: params.side,
                price_in_ticks: params.price_in_ticks,
                num_base_lots: params.num_base_lots,
                num_quote_lots: params.num_quote_lots,
                min_base_lots_to_fill: params.min_base_lots_to_fill,
                min_quote_lots_to_fill: params.min_quote_lots_to_fill,
                self_trade_behavior: params.self_trade_behavior,
                match_limit: params.match_limit,
                client_order_id: params.client_order_id,
                last_valid_slot: params.last_valid_slot,
                order_flags: params.order_flags,
                cancel_existing: params.cancel_existing,
            },
            &mut scratch,
        )?;
        log_matching_response(label, self.phoenix_program.key());
        self.log_hawkeye_margin(label)
    }

    pub(crate) fn invoke_market_order_without_arenas(
        &self,
        params: &MarketOrderWithoutArenas,
        label: &str,
    ) -> ProgramResult {
        msg!("rise-example-program: Phoenix place market order");
        let market_order = self.market_accounts();
        let mut scratch = ix::cpi::CpiScratch::<
            { MAX_CPI_ACCOUNTS },
            { ix::cpi::phoenix::PlaceMarketOrder::MAX_DATA_LEN },
        >::new(self.phoenix_program);
        market_order.invoke(params.into(), &mut scratch)?;
        log_matching_response(label, self.phoenix_program.key());
        self.log_hawkeye_margin(label)
    }

    pub(crate) fn invoke_limit_order(&self, params: &LimitOrderCpiParams) -> ProgramResult {
        msg!("rise-example-program: Phoenix place limit order");
        let limit_order = ix::cpi::phoenix::PlaceLimitOrder {
            phoenix_program: self.phoenix_program,
            log_authority: self.log_authority,
            global_config: self.global_config,
            trader: self.trader,
            trader_account: self.trader_account,
            perp_asset_map: self.perp_asset_map,
            global_trader_index: self.global_trader_index,
            active_trader_buffer: self.active_trader_buffer,
            orderbook: self.orderbook,
            spline_collection: self.spline_collection,
        };
        let mut scratch = ix::cpi::CpiScratch::<
            { MAX_CPI_ACCOUNTS },
            { ix::cpi::phoenix::PlaceLimitOrder::MAX_DATA_LEN },
        >::new(self.phoenix_program);
        limit_order.invoke(
            ix::cpi::phoenix::PlaceLimitOrderArgs {
                side: params.side,
                price_in_ticks: params.price_in_ticks,
                num_base_lots: params.num_base_lots,
                self_trade_behavior: params.self_trade_behavior,
                match_limit: params.match_limit,
                client_order_id: params.client_order_id,
                last_valid_slot: params.last_valid_slot,
                order_flags: params.order_flags,
                cancel_existing: params.cancel_existing,
            },
            &mut scratch,
        )?;
        log_matching_response("place-limit", self.phoenix_program.key());
        Ok(())
    }

    pub(crate) fn invoke_cancel_limit_order(
        &self,
        params: &CancelLimitOrderParams,
    ) -> ProgramResult {
        msg!("rise-example-program: Phoenix cancel limit order");
        let cancel = ix::cpi::phoenix::CancelOrdersById {
            phoenix_program: self.phoenix_program,
            log_authority: self.log_authority,
            global_config: self.global_config,
            trader: self.trader,
            trader_account: self.trader_account,
            perp_asset_map: self.perp_asset_map,
            global_trader_index: self.global_trader_index,
            active_trader_buffer: self.active_trader_buffer,
            orderbook: self.orderbook,
            spline_collection: self.spline_collection,
        };
        let order_id = CancelId {
            node_pointer: params.node_pointer,
            order_id: FifoOrderId {
                price_in_ticks: params.price_in_ticks,
                order_sequence_number: params.order_sequence_number,
            },
        };
        let mut scratch = ix::cpi::CpiScratch::<
            { MAX_CPI_ACCOUNTS },
            { ix::cpi::phoenix::CancelOrdersById::MAX_DATA_LEN },
        >::new(self.phoenix_program);
        cancel.invoke(
            ix::cpi::phoenix::CancelOrdersByIdArgs {
                order_ids: &[order_id],
            },
            &mut scratch,
        )
    }

    fn log_hawkeye_margin(&self, label: &str) -> ProgramResult {
        let view_margin = ix::cpi::hawkeye::ViewMargin {
            hawkeye_program: self.hawkeye_program,
            phoenix_program: self.phoenix_program,
            global_config: self.global_config,
            global_trader_index: self.global_trader_index,
            active_trader_buffer: self.active_trader_buffer,
            perp_asset_map: self.perp_asset_map,
            trader: self.trader_account,
        };
        let mut scratch = ix::cpi::CpiScratch::<
            { MAX_CPI_ACCOUNTS },
            { ix::cpi::hawkeye::ViewMargin::DATA_LEN },
        >::new(self.hawkeye_program);
        let margin = view_margin.invoke_and_decode(&mut scratch)?;
        msg!(&format!(
            "rise-example-program: {label} Hawkeye margin collateral={} free={} withdrawable={} \
             initial={} maintenance={} positions={} risk_state={} risk_tier={} liquidatable={}",
            margin.collateral_quote_lots,
            margin.free_collateral_quote_lots,
            margin.withdrawable_collateral_quote_lots,
            margin.initial_margin_quote_lots,
            margin.maintenance_margin_quote_lots,
            margin.position_count,
            margin.risk_state,
            margin.risk_tier,
            margin.is_liquidatable
        ));
        Ok(())
    }
}
