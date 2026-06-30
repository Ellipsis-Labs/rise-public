mod cancel_limit_order;
mod cancel_stop_loss;
mod close_subaccount_position_and_sweep;
mod common;
mod cpi;
mod deposit_ember_then_phoenix;
mod market;
mod params;
mod place_limit_order;
mod place_market_order;
mod place_stop_loss;
mod register_subaccount_sync_transfer_and_market_order;
mod show_orderbook_bbo;
mod withdraw_phoenix_then_ember;

use borsh::BorshDeserialize;
pub use params::{
    CancelLimitOrderParams, CancelStopLossCpiParams, DepositEmberThenPhoenixParams,
    LimitOrderCpiParams, MarketOrderCpiParams, MarketOrderWithoutArenas, StopLossCpiParams,
    SubaccountCloseAndSweepParams, SubaccountOpenAndTradeParams, WithdrawPhoenixThenEmberParams,
};
use phoenix_rise::ix::constants::compute_discriminant;
use pinocchio::account_info::AccountInfo;
use pinocchio::program_error::ProgramError;
use pinocchio::pubkey::Pubkey;
use pinocchio::{ProgramResult, entrypoint, msg};

pinocchio_pubkey::declare_id!("Hguafkpn6erDXYAZMmTVnaLyjH4EmykiKYGiLwMYTgsJ");

entrypoint!(process_instruction);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExampleInstruction {
    DepositEmberThenPhoenix,
    PlaceMarketOrderAndLogReturn,
    PlaceLimitOrder,
    CancelLimitOrder,
    PlaceStopLoss,
    CancelStopLoss,
    RegisterSubaccountSyncTransferAndMarketOrder,
    CloseSubaccountPositionAndSweep,
    WithdrawPhoenixThenEmber,
    ShowOrderbookBbo,
}

impl ExampleInstruction {
    fn from_tag(tag: &[u8]) -> Option<Self> {
        if tag == compute_discriminant("global:deposit_ember_then_phoenix") {
            return Some(Self::DepositEmberThenPhoenix);
        }
        if tag == compute_discriminant("global:place_market_order_and_log_return") {
            return Some(Self::PlaceMarketOrderAndLogReturn);
        }
        if tag == compute_discriminant("global:place_limit_order") {
            return Some(Self::PlaceLimitOrder);
        }
        if tag == compute_discriminant("global:cancel_limit_order") {
            return Some(Self::CancelLimitOrder);
        }
        if tag == compute_discriminant("global:place_stop_loss") {
            return Some(Self::PlaceStopLoss);
        }
        if tag == compute_discriminant("global:cancel_stop_loss") {
            return Some(Self::CancelStopLoss);
        }
        if tag == compute_discriminant("global:register_subaccount_sync_transfer_and_market_order")
        {
            return Some(Self::RegisterSubaccountSyncTransferAndMarketOrder);
        }
        if tag == compute_discriminant("global:close_subaccount_position_and_sweep") {
            return Some(Self::CloseSubaccountPositionAndSweep);
        }
        if tag == compute_discriminant("global:withdraw_phoenix_then_ember") {
            return Some(Self::WithdrawPhoenixThenEmber);
        }
        if tag == compute_discriminant("global:show_orderbook_bbo") {
            return Some(Self::ShowOrderbookBbo);
        }
        None
    }
}

#[inline(always)]
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if program_id != &ID {
        return Err(ProgramError::IncorrectProgramId);
    }
    if instruction_data.len() < 8 {
        msg!("rise-example-program: missing instruction discriminator");
        return Err(ProgramError::InvalidInstructionData);
    }

    let (tag, data) = instruction_data.split_at(8);
    let instruction = ExampleInstruction::from_tag(tag).ok_or_else(|| {
        msg!("rise-example-program: unknown instruction discriminator");
        ProgramError::InvalidInstructionData
    })?;

    match instruction {
        ExampleInstruction::DepositEmberThenPhoenix => {
            let params = parse_params::<DepositEmberThenPhoenixParams>(
                data,
                "rise-example-program: invalid deposit params",
            )?;
            deposit_ember_then_phoenix::process(accounts, &params)
        }
        ExampleInstruction::WithdrawPhoenixThenEmber => {
            let params = parse_params::<WithdrawPhoenixThenEmberParams>(
                data,
                "rise-example-program: invalid withdraw params",
            )?;
            withdraw_phoenix_then_ember::process(accounts, &params)
        }
        ExampleInstruction::PlaceMarketOrderAndLogReturn => {
            let params = parse_params::<MarketOrderCpiParams>(
                data,
                "rise-example-program: invalid market-order params",
            )?;
            place_market_order::process(accounts, &params)
        }
        ExampleInstruction::PlaceLimitOrder => {
            let params = parse_params::<LimitOrderCpiParams>(
                data,
                "rise-example-program: invalid limit-order params",
            )?;
            place_limit_order::process(accounts, &params)
        }
        ExampleInstruction::CancelLimitOrder => {
            let params = parse_params::<CancelLimitOrderParams>(
                data,
                "rise-example-program: invalid cancel-limit params",
            )?;
            cancel_limit_order::process(accounts, &params)
        }
        ExampleInstruction::PlaceStopLoss => {
            let params = parse_params::<StopLossCpiParams>(
                data,
                "rise-example-program: invalid stop-loss params",
            )?;
            place_stop_loss::process(accounts, &params)
        }
        ExampleInstruction::CancelStopLoss => {
            let params = parse_params::<CancelStopLossCpiParams>(
                data,
                "rise-example-program: invalid cancel-stop-loss params",
            )?;
            cancel_stop_loss::process(accounts, &params)
        }
        ExampleInstruction::RegisterSubaccountSyncTransferAndMarketOrder => {
            let params = parse_params::<SubaccountOpenAndTradeParams>(
                data,
                "rise-example-program: invalid subaccount-open params",
            )?;
            register_subaccount_sync_transfer_and_market_order::process(accounts, &params)
        }
        ExampleInstruction::CloseSubaccountPositionAndSweep => {
            let params = parse_params::<SubaccountCloseAndSweepParams>(
                data,
                "rise-example-program: invalid subaccount-close params",
            )?;
            close_subaccount_position_and_sweep::process(accounts, &params)
        }
        ExampleInstruction::ShowOrderbookBbo => show_orderbook_bbo::process(accounts),
    }
}

fn parse_params<T: BorshDeserialize>(
    data: &[u8],
    invalid_log: &'static str,
) -> Result<T, ProgramError> {
    T::try_from_slice(data).map_err(|_| {
        msg!(invalid_log);
        ProgramError::InvalidInstructionData
    })
}
