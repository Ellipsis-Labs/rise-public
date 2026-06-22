use phoenix_rise::ix;
use pinocchio::account_info::AccountInfo;
use pinocchio::program_error::ProgramError;
use pinocchio::{ProgramResult, msg};

use crate::common::SharedAccounts;
use crate::cpi::{
    check_key, check_program_id, invoke_rise_ix, map_ix_error, to_solana_pubkey, to_solana_pubkeys,
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
        let market_order_ix = self.build_market_order_ix(params)?;
        if let Some(flight) = &self.flight {
            let flight_ix = self.build_flight_ix(flight, market_order_ix)?;
            let mut flight_accounts = Vec::with_capacity(6 + self.market_order_account_count());
            flight_accounts.extend([
                flight.flight_global_state,
                self.common.phoenix_program,
                flight.builder_authority,
                flight.builder_trader_account,
                flight.flight_builder_state,
                self.common.trader_authority,
            ]);
            flight_accounts.extend(self.market_order_account_infos());
            return invoke_rise_ix(&flight_ix, flight.flight_program, &flight_accounts);
        }

        invoke_rise_ix(
            &market_order_ix,
            self.common.phoenix_program,
            &self.market_order_account_infos(),
        )
    }

    fn build_market_order_ix(
        &self,
        params: &ClosePositionAndWithdrawParams,
    ) -> Result<ix::Instruction, ProgramError> {
        let mut builder = ix::MarketOrderParams::builder()
            .trader(to_solana_pubkey(self.common.trader_authority.key()))
            .trader_account(to_solana_pubkey(self.common.trader_account.key()))
            .perp_asset_map(to_solana_pubkey(self.common.perp_asset_map.key()))
            .orderbook(to_solana_pubkey(self.orderbook.key()))
            .spline_collection(to_solana_pubkey(self.spline_collection.key()))
            .global_trader_index(to_solana_pubkeys(&self.common.global_trader_index))
            .active_trader_buffer(to_solana_pubkeys(&self.common.active_trader_buffer))
            .side(params.side)
            .num_base_lots(params.num_base_lots)
            .min_base_lots_to_fill(params.min_base_lots_to_fill)
            .min_quote_lots_to_fill(params.min_quote_lots_to_fill)
            .self_trade_behavior(params.self_trade_behavior)
            .client_order_id(params.client_order_id)
            .order_flags(ix::OrderFlags::ReduceOnly)
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
        ix::create_place_market_order_ix(params).map_err(map_ix_error)
    }

    fn build_flight_ix(
        &self,
        flight: &FlightAccounts<'_>,
        inner: ix::Instruction,
    ) -> Result<ix::Instruction, ProgramError> {
        let params = ix::flight::ProxyInstructionParams::builder()
            .builder_authority(to_solana_pubkey(flight.builder_authority.key()))
            .builder_trader_account(to_solana_pubkey(flight.builder_trader_account.key()))
            .trader_wallet(to_solana_pubkey(self.common.trader_authority.key()))
            .inner_instruction(inner)
            .build()
            .map_err(map_ix_error)?;
        ix::flight::create_proxy_instruction_ix(params).map_err(map_ix_error)
    }

    fn market_order_account_infos(&self) -> Vec<&'a AccountInfo> {
        let mut account_infos = Vec::with_capacity(self.market_order_account_count());
        account_infos.extend([
            self.common.phoenix_program,
            self.common.phoenix_log_authority,
            self.common.phoenix_global_config,
            self.common.trader_authority,
            self.common.trader_account,
            self.common.perp_asset_map,
        ]);
        account_infos.extend(self.common.global_trader_index.iter().copied());
        account_infos.extend(self.common.active_trader_buffer.iter().copied());
        account_infos.extend([self.orderbook, self.spline_collection]);
        account_infos
    }

    fn market_order_account_count(&self) -> usize {
        8 + self.common.global_trader_index.len() + self.common.active_trader_buffer.len()
    }
}

impl FlightAccounts<'_> {
    fn validate(&self) -> ProgramResult {
        check_program_id(
            self.flight_program,
            &ix::flight::FLIGHT_PROGRAM_ID,
            "Flight",
        )?;
        check_key(
            self.flight_global_state,
            &ix::flight::get_flight_global_state_address(),
        )?;
        check_key(
            self.flight_builder_state,
            &ix::flight::get_flight_builder_state_address(&to_solana_pubkey(
                self.builder_authority.key(),
            )),
        )
    }
}
