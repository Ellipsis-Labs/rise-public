//! Risk assessment types and margin state
//!
//! This module provides types for risk management, margin calculations,
//! and risk tier assessment.

use thiserror::Error;

use crate::quantities::{MathError, QuoteLots, SignedQuoteLots, Slot};

/// Reasons the program may fail.
#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum ProgramError {
    #[error("Custom program error: {0:#x}")]
    Custom(u32),
    #[error("Invalid argument")]
    InvalidArgument,
    #[error("Invalid instruction data")]
    InvalidInstructionData,
    #[error("Invalid account data")]
    InvalidAccountData,
    #[error("Account data too small")]
    AccountDataTooSmall,
    #[error("Insufficient funds")]
    InsufficientFunds,
    #[error("Incorrect program id")]
    IncorrectProgramId,
    #[error("Missing required signature")]
    MissingRequiredSignature,
    #[error("Account already initialized")]
    AccountAlreadyInitialized,
    #[error("Uninitialized account")]
    UninitializedAccount,
    #[error("Not enough account keys")]
    NotEnoughAccountKeys,
    #[error("Account borrow failed")]
    AccountBorrowFailed,
    #[error("Max seed length exceeded")]
    MaxSeedLengthExceeded,
    #[error("Invalid seeds")]
    InvalidSeeds,
    #[error("Borsh IO error")]
    BorshIoError,
    #[error("Account not rent exempt")]
    AccountNotRentExempt,
    #[error("Unsupported sysvar")]
    UnsupportedSysvar,
    #[error("Illegal owner")]
    IllegalOwner,
    #[error("Max accounts data allocations exceeded")]
    MaxAccountsDataAllocationsExceeded,
    #[error("Invalid realloc")]
    InvalidRealloc,
    #[error("Max instruction trace length exceeded")]
    MaxInstructionTraceLengthExceeded,
    #[error("Builtin programs must consume compute units")]
    BuiltinProgramsMustConsumeComputeUnits,
    #[error("Invalid account owner")]
    InvalidAccountOwner,
    #[error("Arithmetic overflow")]
    ArithmeticOverflow,
    #[error("Account is immutable")]
    Immutable,
    #[error("Incorrect authority")]
    IncorrectAuthority,
}

/// Errors that can occur during margin calculations
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MarginError {
    #[error("Insufficient funds")]
    InsufficientFunds,
    #[error("Invalid state change")]
    InvalidStateChange,
    #[error("Overflow")]
    Overflow,
    #[error("Math error: {0:?}")]
    Math(
        #[source]
        #[from]
        MathError,
    ),
    #[error("Mark price error: {0:?}")]
    MarkPrice(ProgramError),
}

/// Risk action context for margin calculations
///
/// Specifies what action is being performed, which affects
/// price validity requirements and margin calculations.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RiskAction {
    /// View-only (no state changes)
    View,
    /// Liquidation action
    Liquidation { current_slot: Slot },
    /// Placing an order
    PlacingOrder { current_slot: Slot },
    /// Funding payment
    Funding { current_slot: Slot },
    /// Withdrawal attempt
    Withdrawal { current_slot: Slot },
    /// Auto-deleveraging
    ADL { current_slot: Slot },
}

impl RiskAction {
    /// Get the current slot for this risk action
    #[inline]
    pub const fn current_slot(&self) -> Slot {
        match self {
            Self::View => Slot::ZERO,
            Self::Liquidation { current_slot } => *current_slot,
            Self::PlacingOrder { current_slot } => *current_slot,
            Self::Funding { current_slot } => *current_slot,
            Self::Withdrawal { current_slot } => *current_slot,
            Self::ADL { current_slot } => *current_slot,
        }
    }

    /// Get the index for this risk action type
    #[inline]
    pub const fn as_index(&self) -> usize {
        match self {
            Self::View => 0,
            Self::Liquidation { .. } => 1,
            Self::PlacingOrder { .. } => 2,
            Self::Funding { .. } => 3,
            Self::Withdrawal { .. } => 4,
            Self::ADL { .. } => 5,
        }
    }
}

/// Risk state of a trader based on collateral and margin requirements
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RiskState {
    /// Healthy: effective_collateral >= initial_margin
    Healthy,
    /// Unhealthy: effective_collateral < initial_margin (but > 0)
    Unhealthy,
    /// Underwater: effective_collateral <= 0
    Underwater,
    /// Zero collateral with no positions
    ZeroCollateralNoPositions,
}

/// Risk tier determines liquidation priority and borrowing limits
///
/// Higher risk tiers have stricter requirements and higher liquidation
/// priority.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskTier {
    /// Safe: effective_collateral >= initial_margin (100%)
    Safe                 = 0,
    /// AtRisk: effective_collateral >= initial_margin but close to threshold
    AtRisk               = 1,
    /// Cancellable: Orders can be force-cancelled
    Cancellable          = 2,
    /// Liquidatable: Below maintenance margin, subject to liquidation
    Liquidatable         = 3,
    /// BackstopLiquidatable: Below backstop margin
    BackstopLiquidatable = 4,
    /// HighRisk: Below high risk margin, insurance fund territory
    HighRisk             = 5,
}

/// Margin state combining collateral and margin requirements
///
/// Used to determine if a trader can perform actions like withdrawals,
/// placing orders, etc.
#[derive(Debug, Clone)]
pub struct MarginState {
    pub initial_margin: QuoteLots,
    pub effective_collateral: SignedQuoteLots,
}

impl MarginState {
    /// Create a new margin state
    pub fn new(initial_margin: QuoteLots, effective_collateral: SignedQuoteLots) -> Self {
        Self {
            initial_margin,
            effective_collateral,
        }
    }

    /// Calculate risk state from margin and collateral values
    pub fn risk_state(&self) -> Result<RiskState, MarginError> {
        let collateral = self.effective_collateral;
        let initial_margin = self.initial_margin.checked_as_signed()?;

        if collateral < SignedQuoteLots::ZERO {
            Ok(RiskState::Underwater)
        } else if collateral == SignedQuoteLots::ZERO {
            if self.initial_margin == QuoteLots::ZERO {
                Ok(RiskState::ZeroCollateralNoPositions)
            } else {
                Ok(RiskState::Underwater)
            }
        } else if collateral >= initial_margin {
            Ok(RiskState::Healthy)
        } else {
            Ok(RiskState::Unhealthy)
        }
    }
}
