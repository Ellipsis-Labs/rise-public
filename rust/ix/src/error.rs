//! Error types for Phoenix instruction construction.

use thiserror::Error;

/// Errors that can occur when building Phoenix instructions.
#[derive(Debug, Error)]
pub enum PhoenixIxError {
    #[error("Trader wallet is required")]
    MissingTrader,

    #[error("Trader account is required")]
    MissingTraderAccount,

    #[error("Perp asset map is required")]
    MissingPerpAssetMap,

    #[error("Orderbook is required")]
    MissingOrderbook,

    #[error("Spline collection is required")]
    MissingSplineCollection,

    #[error("Active trader buffer array is required and must not be empty")]
    EmptyActiveTraderBuffer,

    #[error("Global trader index array is required and must not be empty")]
    EmptyGlobalTraderIndex,

    #[error("At least one order ID is required")]
    NoOrderIds,

    #[error("Too many order IDs (maximum 100)")]
    TooManyOrderIds,

    #[error("Missing required field: {0}")]
    MissingField(&'static str),

    #[error("Invalid deposit amount (must be greater than 0)")]
    InvalidDepositAmount,

    #[error("Invalid withdraw amount (must be greater than 0)")]
    InvalidWithdrawAmount,

    #[error("Invalid subaccount index for isolated margin (must be 0-100)")]
    InvalidSubaccountIndex,

    #[error("Invalid transfer amount (must be greater than 0)")]
    InvalidTransferAmount,

    #[error("Inner instruction must target the Phoenix program")]
    InvalidInnerProgram,

    #[error("Invalid fee bps override (must be in 0..=10000)")]
    InvalidFeeBpsOverride,

    #[error("Invalid conditional orders capacity (must be greater than 0)")]
    InvalidConditionalOrdersCapacity,

    #[error("Invalid conditional order index (must be in 1..=191)")]
    InvalidConditionalOrderIndex,

    #[error("At least one conditional order trigger is required")]
    MissingConditionalOrderTrigger,

    #[error("Conditional order trigger direction does not match the trigger slot")]
    InvalidConditionalOrderTriggerDirection,

    #[error("Conditional order size must set exactly one of size_base_lots or size_percent")]
    InvalidConditionalOrderSize,

    #[error("Conditional order size_percent must be in 1..=100")]
    InvalidConditionalOrderPercent,

    #[error("At least one conditional order disable flag is required")]
    MissingConditionalOrderDisableFlag,

    #[error("Invalid spline tick region")]
    InvalidSplineTickRegion,

    #[error("At least one spline parameter update is required")]
    MissingSplineUpdate,

    #[error("Invalid leverage decrease bps (must be in 0..=10000)")]
    InvalidLeverageDecreaseBps,

    #[error("instruction data buffer too small: expected at least {expected} bytes, got {actual}")]
    InstructionDataBufferTooSmall { expected: usize, actual: usize },

    #[error("instruction data length overflow")]
    InstructionDataTooLarge,

    #[error("PDA derivation failed")]
    PdaDerivationUnavailable,
}
