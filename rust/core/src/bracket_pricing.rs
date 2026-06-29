use phoenix_rise_ix::types::{Side, StopLossOrderKind};

use crate::order_tickets::{BracketLeg, BracketLegExecution, DEFAULT_BRACKET_LEG_SLIPPAGE_BPS};
use crate::tx_builder::PhoenixTxBuilderError;

const BPS_DENOMINATOR: u128 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BracketLegExecutionPriceError {
    #[error("sell-side slippage must be less than 10000 bps")]
    SellSlippageTooLarge,

    #[error("sell-side slippage resolves execution price to 0 ticks")]
    SellSlippageResolvesToZero,

    #[error("slippage overflows execution ticks")]
    SlippageOverflow,
}

pub(crate) fn bracket_leg_execution_ticks(
    calc: &phoenix_rise_math::MarketCalculator,
    leg: &BracketLeg,
    trigger_ticks: u64,
    trade_side: Side,
    order_kind: StopLossOrderKind,
) -> Result<u64, PhoenixTxBuilderError> {
    match leg.execution {
        Some(BracketLegExecution::Price(price)) => Ok(calc.price_to_ticks(price)?.as_inner()),
        Some(BracketLegExecution::SlippageBps(slippage_bps)) => {
            execution_ticks_with_slippage(trigger_ticks, trade_side, slippage_bps)
        }
        None if order_kind == StopLossOrderKind::Limit => Ok(trigger_ticks),
        None => execution_ticks_with_slippage(
            trigger_ticks,
            trade_side,
            DEFAULT_BRACKET_LEG_SLIPPAGE_BPS,
        ),
    }
}

fn execution_ticks_with_slippage(
    trigger_ticks: u64,
    trade_side: Side,
    slippage_bps: u32,
) -> Result<u64, PhoenixTxBuilderError> {
    match trade_side {
        Side::Ask => sell_execution_ticks(trigger_ticks, slippage_bps),
        Side::Bid => buy_execution_ticks(trigger_ticks, slippage_bps),
    }
}

fn sell_execution_ticks(
    trigger_ticks: u64,
    slippage_bps: u32,
) -> Result<u64, PhoenixTxBuilderError> {
    if slippage_bps >= 10_000 {
        return Err(BracketLegExecutionPriceError::SellSlippageTooLarge.into());
    }

    let numerator = (trigger_ticks as u128)
        .checked_mul(BPS_DENOMINATOR - slippage_bps as u128)
        .ok_or(BracketLegExecutionPriceError::SlippageOverflow)?;
    let mut ticks = (numerator / BPS_DENOMINATOR) as u64;

    if slippage_bps > 0 && ticks == trigger_ticks {
        ticks = trigger_ticks
            .checked_sub(1)
            .ok_or(BracketLegExecutionPriceError::SellSlippageResolvesToZero)?;
    }

    if ticks == 0 {
        return Err(BracketLegExecutionPriceError::SellSlippageResolvesToZero.into());
    }

    Ok(ticks)
}

fn buy_execution_ticks(
    trigger_ticks: u64,
    slippage_bps: u32,
) -> Result<u64, PhoenixTxBuilderError> {
    let numerator = (trigger_ticks as u128)
        .checked_mul(BPS_DENOMINATOR + slippage_bps as u128)
        .ok_or(BracketLegExecutionPriceError::SlippageOverflow)?;
    let ticks = numerator
        .checked_add(BPS_DENOMINATOR - 1)
        .ok_or(BracketLegExecutionPriceError::SlippageOverflow)?
        / BPS_DENOMINATOR;

    if ticks > u64::MAX as u128 {
        return Err(BracketLegExecutionPriceError::SlippageOverflow.into());
    }

    let mut ticks = ticks as u64;
    if slippage_bps > 0 && ticks == trigger_ticks {
        ticks = trigger_ticks
            .checked_add(1)
            .ok_or(BracketLegExecutionPriceError::SlippageOverflow)?;
    }

    Ok(ticks)
}
