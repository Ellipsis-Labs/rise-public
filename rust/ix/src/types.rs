//! Common types for Phoenix instruction construction.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_pubkey::Pubkey;

/// Side of an order - either Bid (buy) or Ask (sell).
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[borsh(use_discriminant = true)]
#[repr(u8)]
pub enum Side {
    Bid = 0,
    Ask = 1,
}

impl Side {
    /// Returns the API wire string for this side (`"buy"` or `"sell"`).
    pub fn to_api_string(self) -> &'static str {
        match self {
            Side::Bid => "buy",
            Side::Ask => "sell",
        }
    }
}

/// Order flags for specifying order behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[borsh(use_discriminant = true)]
#[repr(u8)]
pub enum OrderFlags {
    /// No special flags.
    None       = 0,
    /// Reduce only flag - order can only reduce existing position.
    ReduceOnly = 128, // 1 << 7
}

/// Self-trade behavior for orders that can match against existing orders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[borsh(use_discriminant = true)]
#[repr(u8)]
pub enum SelfTradeBehavior {
    /// Abort the new order if it would self-trade.
    Abort         = 0,
    /// Cancel the existing order and provide the new order.
    CancelProvide = 1,
    /// Decrement the existing order and provide the new order.
    DecrementTake = 2,
}

/// A FIFO order ID, used to uniquely identify an order on the book.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct FifoOrderId {
    /// The price of the order, in ticks.
    pub price_in_ticks: u64,
    /// The order sequence number.
    pub order_sequence_number: u64,
}

/// A cancel ID, used to specify an order to cancel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct CancelId {
    /// Optional node pointer (0 if not specified).
    pub node_pointer: u32,
    /// The order ID to cancel.
    pub order_id: FifoOrderId,
}

impl CancelId {
    /// Create a new CancelId from price and sequence number.
    pub fn new(price_in_ticks: u64, order_sequence_number: u64) -> Self {
        Self {
            node_pointer: 0,
            order_id: FifoOrderId {
                price_in_ticks,
                order_sequence_number,
            },
        }
    }
}

/// Direction for price comparison triggers (used in stop-loss/take-profit
/// orders).
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[borsh(use_discriminant = true)]
#[repr(u8)]
pub enum Direction {
    /// Trigger when price moves above threshold.
    GreaterThan = 0,
    /// Trigger when price moves below threshold.
    LessThan    = 1,
}

/// Execution kind for stop-loss/take-profit orders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[borsh(use_discriminant = true)]
#[repr(u8)]
pub enum StopLossOrderKind {
    /// Immediate-or-cancel execution.
    IOC   = 0,
    /// Limit order execution.
    Limit = 1,
}

/// How to fund an isolated subaccount before placing an order.
///
/// Collateral amounts are in **quote lots** (native USDC base units,
/// i.e. 1 USDC = 1 000 000 quote lots).
pub enum IsolatedCollateralFlow {
    /// Desired total collateral level — only the delta above existing
    /// collateral is transferred from cross-margin.
    TransferFromCrossMargin { collateral: u64 },
    /// Deposit fresh USDC directly into the isolated subaccount.
    Deposit { usdc_amount: u64 },
}

/// Account metadata for Solana instructions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountMeta {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

impl AccountMeta {
    /// Create a readonly account.
    pub fn readonly(pubkey: Pubkey) -> Self {
        Self {
            pubkey,
            is_signer: false,
            is_writable: false,
        }
    }

    /// Create a writable account.
    pub fn writable(pubkey: Pubkey) -> Self {
        Self {
            pubkey,
            is_signer: false,
            is_writable: true,
        }
    }

    /// Create a readonly signer account.
    pub fn readonly_signer(pubkey: Pubkey) -> Self {
        Self {
            pubkey,
            is_signer: true,
            is_writable: false,
        }
    }

    /// Create a writable signer account.
    pub fn writable_signer(pubkey: Pubkey) -> Self {
        Self {
            pubkey,
            is_signer: true,
            is_writable: true,
        }
    }
}

/// A Solana instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    /// The program ID to invoke.
    pub program_id: Pubkey,
    /// The accounts required by the instruction.
    pub accounts: Vec<AccountMeta>,
    /// The instruction data.
    pub data: Vec<u8>,
}

impl From<AccountMeta> for solana_instruction::AccountMeta {
    fn from(meta: AccountMeta) -> Self {
        Self {
            pubkey: meta.pubkey,
            is_signer: meta.is_signer,
            is_writable: meta.is_writable,
        }
    }
}

impl From<Instruction> for solana_instruction::Instruction {
    fn from(ix: Instruction) -> Self {
        Self {
            program_id: ix.program_id,
            accounts: ix.accounts.into_iter().map(Into::into).collect(),
            data: ix.data,
        }
    }
}
