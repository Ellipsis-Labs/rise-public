#![allow(dead_code, unused_imports)]

mod close_position;
mod common;
mod cpi;
mod state_views;
mod withdraw_all;

use borsh::{BorshDeserialize, BorshSerialize};
use close_position::ClosePositionContext;
use phoenix_rise::ix::constants::compute_discriminant;
pub use phoenix_rise::ix::types::{SelfTradeBehavior, Side};
use pinocchio::account_info::AccountInfo;
use pinocchio::program_error::ProgramError;
use pinocchio::pubkey::Pubkey;
use pinocchio::{ProgramResult, entrypoint, msg};
use withdraw_all::WithdrawAllCollateralContext;

pinocchio_pubkey::declare_id!("6oy4nTsJWjAyMX86fxbWVK7UfpWPzzG6AdLouhLNkoen");

entrypoint!(process_instruction);

#[derive(Debug, Clone, Copy, BorshDeserialize, BorshSerialize, PartialEq, Eq)]
pub enum WithdrawMode {
    AllFreeCollateral,
    OrderFreeCollateralDelta,
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct ClosePositionAndWithdrawParams {
    pub withdraw_mode: WithdrawMode,
    pub side: Side,
    pub price_in_ticks: Option<u64>,
    pub num_base_lots: u64,
    pub num_quote_lots: Option<u64>,
    pub min_base_lots_to_fill: u64,
    pub min_quote_lots_to_fill: u64,
    pub self_trade_behavior: SelfTradeBehavior,
    pub match_limit: Option<u64>,
    pub client_order_id: u128,
    pub last_valid_slot: Option<u64>,
    pub cancel_existing: bool,
    pub global_trader_index_count: u8,
    pub active_trader_buffer_count: u8,
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct WithdrawAllCollateralParams {
    pub global_trader_index_count: u8,
    pub active_trader_buffer_count: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PhoenixCloseAndWithdrawInstruction {
    ClosePositionAndWithdraw,
    ClosePositionAndWithdrawWithBuilders,
    WithdrawAllCollateral,
}

impl PhoenixCloseAndWithdrawInstruction {
    fn from_tag(tag: &[u8]) -> Option<Self> {
        if tag == compute_discriminant("global:close_position_and_withdraw") {
            return Some(Self::ClosePositionAndWithdraw);
        }
        if tag == compute_discriminant("global:close_position_and_withdraw_with_builders") {
            return Some(Self::ClosePositionAndWithdrawWithBuilders);
        }
        if tag == compute_discriminant("global:withdraw_all_collateral") {
            return Some(Self::WithdrawAllCollateral);
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
        msg!("close-position-and-withdraw: missing instruction discriminator");
        return Err(ProgramError::InvalidInstructionData);
    }

    let (tag, data) = instruction_data.split_at(8);
    let instruction = PhoenixCloseAndWithdrawInstruction::from_tag(tag).ok_or_else(|| {
        msg!("close-position-and-withdraw: unknown instruction discriminator");
        ProgramError::InvalidInstructionData
    })?;

    match instruction {
        PhoenixCloseAndWithdrawInstruction::ClosePositionAndWithdraw => {
            let params = ClosePositionAndWithdrawParams::try_from_slice(data).map_err(|_| {
                msg!("close-position-and-withdraw: invalid close-position params");
                ProgramError::InvalidInstructionData
            })?;
            let context = ClosePositionContext::load(
                accounts,
                params.global_trader_index_count as usize,
                params.active_trader_buffer_count as usize,
                false,
            )?;
            context.invoke(&params)
        }
        PhoenixCloseAndWithdrawInstruction::ClosePositionAndWithdrawWithBuilders => {
            let params = ClosePositionAndWithdrawParams::try_from_slice(data).map_err(|_| {
                msg!("close-position-and-withdraw: invalid close-position params");
                ProgramError::InvalidInstructionData
            })?;
            let context = ClosePositionContext::load(
                accounts,
                params.global_trader_index_count as usize,
                params.active_trader_buffer_count as usize,
                true,
            )?;
            context.invoke(&params)
        }
        PhoenixCloseAndWithdrawInstruction::WithdrawAllCollateral => {
            let params = WithdrawAllCollateralParams::try_from_slice(data).map_err(|_| {
                msg!("close-position-and-withdraw: invalid withdraw-all params");
                ProgramError::InvalidInstructionData
            })?;
            let context = WithdrawAllCollateralContext::load(
                accounts,
                params.global_trader_index_count as usize,
                params.active_trader_buffer_count as usize,
            )?;
            context.invoke()
        }
    }
}
