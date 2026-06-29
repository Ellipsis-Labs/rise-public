use phoenix_rise::ix;
use phoenix_rise::ix::market_order::{MarketOrderParams, create_place_market_order_ix};
use phoenix_rise::ix::types::{Instruction, OrderFlags};
use pinocchio::account_info::AccountInfo;
use pinocchio::program_error::ProgramError;
use pinocchio::{ProgramResult, msg};

use crate::common::SharedAccounts;
use crate::cpi::{
    MAX_CPI_ACCOUNTS, MAX_FLIGHT_PROXY_ACCOUNTS, check_key, check_program_id, invoke_rise_ix,
    map_ix_error, to_solana_pubkey, to_solana_pubkeys,
};
use crate::{ClosePositionAndWithdrawParams, WithdrawMode};

pub(crate) struct ClosePositionContext<'a> {
    common: SharedAccounts<'a>,
    orderbook: &'a AccountInfo,
    spline_collection: &'a AccountInfo,
    flight: Option<FlightAccounts<'a>>,
}

struct FlightAccounts<'a> {
    flight_program: &'a AccountInfo,
    builder_authority: &'a AccountInfo,
    builder_trader_account: &'a AccountInfo,
    flight_global_state: &'a AccountInfo,
    flight_builder_state: &'a AccountInfo,
}

impl<'a> ClosePositionContext<'a> {
    pub(crate) fn load(
        accounts: &'a [AccountInfo],
        global_trader_index_count: usize,
        active_trader_buffer_count: usize,
        uses_builders: bool,
    ) -> Result<Self, ProgramError> {
        const ORDER_ACCOUNT_COUNT: usize = 2;
        const FLIGHT_ACCOUNT_COUNT: usize = 5;

        let order_accounts_start = SharedAccounts::FIXED_ACCOUNT_COUNT;
        let flight_accounts_start = order_accounts_start
            .checked_add(ORDER_ACCOUNT_COUNT)
            .ok_or(ProgramError::InvalidInstructionData)?;
        let dynamic_accounts_start = flight_accounts_start
            .checked_add(if uses_builders {
                FLIGHT_ACCOUNT_COUNT
            } else {
                0
            })
            .ok_or(ProgramError::InvalidInstructionData)?;
        let expected = dynamic_accounts_start
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
                dynamic_accounts_start,
                global_trader_index_count,
                active_trader_buffer_count,
            )?,
            orderbook: &accounts[order_accounts_start],
            spline_collection: &accounts[order_accounts_start + 1],
            flight: uses_builders.then(|| FlightAccounts {
                flight_program: &accounts[flight_accounts_start],
                builder_authority: &accounts[flight_accounts_start + 1],
                builder_trader_account: &accounts[flight_accounts_start + 2],
                flight_global_state: &accounts[flight_accounts_start + 3],
                flight_builder_state: &accounts[flight_accounts_start + 4],
            }),
        };
        context.validate()?;
        Ok(context)
    }

    pub(crate) fn invoke(&self, params: &ClosePositionAndWithdrawParams) -> ProgramResult {
        self.common.log_trader_status()?;
        let before_withdrawable = self
            .common
            .read_and_log_withdrawable_collateral("before order")?;
        self.invoke_close_order(params)?;
        let after_withdrawable = self
            .common
            .read_and_log_withdrawable_collateral("after order")?;
        let amount = match params.withdraw_mode {
            WithdrawMode::AllFreeCollateral => after_withdrawable,
            WithdrawMode::OrderFreeCollateralDelta => {
                after_withdrawable.saturating_sub(before_withdrawable)
            }
        };
        self.common.withdraw(amount)
    }

    fn validate(&self) -> ProgramResult {
        self.common.validate()?;
        if let Some(flight) = &self.flight {
            flight.validate()?;
        }
        Ok(())
    }

    fn invoke_close_order(&self, params: &ClosePositionAndWithdrawParams) -> ProgramResult {
        if let Some(flight) = &self.flight {
            let market_order_ix = self.build_market_order_ix(params)?;
            let flight_ix = self.build_flight_ix(flight, market_order_ix)?;
            let mut flight_accounts = [flight.flight_program; MAX_FLIGHT_PROXY_ACCOUNTS];
            let mut account_count = 0usize;
            push_account(
                &mut flight_accounts,
                &mut account_count,
                flight.flight_global_state,
            )?;
            push_account(
                &mut flight_accounts,
                &mut account_count,
                self.common.phoenix_program,
            )?;
            push_account(
                &mut flight_accounts,
                &mut account_count,
                flight.builder_authority,
            )?;
            push_account(
                &mut flight_accounts,
                &mut account_count,
                flight.builder_trader_account,
            )?;
            push_account(
                &mut flight_accounts,
                &mut account_count,
                flight.flight_builder_state,
            )?;
            push_account(
                &mut flight_accounts,
                &mut account_count,
                self.common.trader_authority,
            )?;
            account_count +=
                self.write_market_order_account_infos(&mut flight_accounts[account_count..])?;
            return invoke_rise_ix::<MAX_FLIGHT_PROXY_ACCOUNTS>(
                &flight_ix,
                flight.flight_program,
                &flight_accounts[..account_count],
            );
        }

        self.invoke_market_order(params)
    }

    fn invoke_market_order(&self, params: &ClosePositionAndWithdrawParams) -> ProgramResult {
        let market_order = ix::cpi::phoenix::PlaceMarketOrder {
            phoenix_program: self.common.phoenix_program,
            log_authority: self.common.phoenix_log_authority,
            global_config: self.common.phoenix_global_config,
            trader: self.common.trader_authority,
            trader_account: self.common.trader_account,
            perp_asset_map: self.common.perp_asset_map,
            global_trader_index: self.common.global_trader_index,
            active_trader_buffer: self.common.active_trader_buffer,
            orderbook: self.orderbook,
            spline_collection: self.spline_collection,
        };
        let mut scratch = ix::cpi::CpiScratch::<
            { MAX_CPI_ACCOUNTS },
            { ix::cpi::phoenix::PlaceMarketOrder::MAX_DATA_LEN },
        >::new(self.common.phoenix_program);
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
                order_flags: OrderFlags::ReduceOnly,
                cancel_existing: params.cancel_existing,
            },
            &mut scratch,
        )
    }

    fn build_market_order_ix(
        &self,
        params: &ClosePositionAndWithdrawParams,
    ) -> Result<Instruction, ProgramError> {
        let mut builder = MarketOrderParams::builder()
            .trader(to_solana_pubkey(self.common.trader_authority.key()))
            .trader_account(to_solana_pubkey(self.common.trader_account.key()))
            .perp_asset_map(to_solana_pubkey(self.common.perp_asset_map.key()))
            .orderbook(to_solana_pubkey(self.orderbook.key()))
            .spline_collection(to_solana_pubkey(self.spline_collection.key()))
            .global_trader_index(to_solana_pubkeys(self.common.global_trader_index))
            .active_trader_buffer(to_solana_pubkeys(self.common.active_trader_buffer))
            .side(params.side)
            .num_base_lots(params.num_base_lots)
            .min_base_lots_to_fill(params.min_base_lots_to_fill)
            .min_quote_lots_to_fill(params.min_quote_lots_to_fill)
            .self_trade_behavior(params.self_trade_behavior)
            .client_order_id(params.client_order_id)
            .order_flags(OrderFlags::ReduceOnly)
            .cancel_existing(params.cancel_existing);

        if let Some(price_in_ticks) = params.price_in_ticks {
            builder = builder.price_in_ticks(price_in_ticks);
        }
        if let Some(num_quote_lots) = params.num_quote_lots {
            builder = builder.num_quote_lots(num_quote_lots);
        }
        if let Some(match_limit) = params.match_limit {
            builder = builder.match_limit(match_limit);
        }
        if let Some(last_valid_slot) = params.last_valid_slot {
            builder = builder.last_valid_slot(last_valid_slot);
        }

        let params = builder.build().map_err(map_ix_error)?;
        create_place_market_order_ix(params).map_err(map_ix_error)
    }

    fn build_flight_ix(
        &self,
        flight: &FlightAccounts<'_>,
        inner: Instruction,
    ) -> Result<Instruction, ProgramError> {
        let params = ix::flight::ProxyInstructionParams::builder()
            .builder_authority(to_solana_pubkey(flight.builder_authority.key()))
            .builder_trader_account(to_solana_pubkey(flight.builder_trader_account.key()))
            .trader_wallet(to_solana_pubkey(self.common.trader_authority.key()))
            .inner_instruction(inner)
            .build()
            .map_err(map_ix_error)?;
        ix::flight::create_proxy_instruction_ix(params).map_err(map_ix_error)
    }

    fn write_market_order_account_infos(
        &self,
        out: &mut [&'a AccountInfo],
    ) -> Result<usize, ProgramError> {
        let mut account_count = 0usize;
        for account in [
            self.common.phoenix_program,
            self.common.phoenix_log_authority,
            self.common.phoenix_global_config,
            self.common.trader_authority,
            self.common.trader_account,
            self.common.perp_asset_map,
        ] {
            push_account(out, &mut account_count, account)?;
        }
        for account in self.common.global_trader_index {
            push_account(out, &mut account_count, account)?;
        }
        for account in self.common.active_trader_buffer {
            push_account(out, &mut account_count, account)?;
        }
        for account in [self.orderbook, self.spline_collection] {
            push_account(out, &mut account_count, account)?;
        }
        Ok(account_count)
    }

    fn market_order_account_count(&self) -> usize {
        8 + self.common.global_trader_index.len() + self.common.active_trader_buffer.len()
    }
}

fn push_account<'a>(
    out: &mut [&'a AccountInfo],
    account_count: &mut usize,
    account: &'a AccountInfo,
) -> ProgramResult {
    if *account_count >= out.len() {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    out[*account_count] = account;
    *account_count += 1;
    Ok(())
}

impl FlightAccounts<'_> {
    fn validate(&self) -> ProgramResult {
        check_program_id(
            self.flight_program,
            &ix::flight::FLIGHT_PROGRAM_ID,
            "Flight",
        )?;
        let flight_global_state =
            ix::flight::get_flight_global_state_address().map_err(map_ix_error)?;
        let flight_builder_state = ix::flight::get_flight_builder_state_address(&to_solana_pubkey(
            self.builder_authority.key(),
        ))
        .map_err(map_ix_error)?;
        check_key(self.flight_global_state, &flight_global_state)?;
        check_key(self.flight_builder_state, &flight_builder_state)
    }
}
