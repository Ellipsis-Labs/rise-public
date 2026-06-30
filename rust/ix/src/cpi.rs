//! Pinocchio CPI helpers for Phoenix instructions.
//!
//! This module is for on-chain callers that already have `AccountInfo`
//! references and want to invoke Phoenix instructions without going through the
//! off-chain client stack. It provides two layers:
//!
//! - Low-level helpers like [`account_metas`], [`instruction_account_metas`],
//!   [`instruction_account_info_refs`], [`invoke`], and [`invoke_instruction`]
//!   when you want to work directly with account arrays and instruction bytes.
//! - Typed CPI wrappers under [`phoenix`] when you want Rust structs that model
//!   a specific Phoenix action such as registering a trader, placing an order,
//!   or toggling trader capabilities.
//!
//! # How To Use This Module
//!
//! - Use [`CpiScratch`] when you need reusable stack-allocated buffers for
//!   repeated CPI calls.
//! - Use [`CpiBuffers`] when you already have your own account/data buffers.
//! - Use [`phoenix::RegisterTrader`], [`phoenix::PlaceMarketOrder`], and the
//!   other typed wrappers when you want the account ordering and instruction
//!   payload layout to be encoded in the type system.
//!
//! The typed wrappers assume that you already know which accounts to pass. They
//! do not fetch metadata or derive account addresses for you.
//!
//! Example programs that exercise these flows live under
//! `rise/rust/sdk/examples` in this repository and are mirrored into the
//! `rise-public` repository after merge.
//!
//! # Minimal Example
//!
//! ```no_run
//! use pinocchio::account_info::AccountInfo;
//! use phoenix_rise_ix::cpi::{CpiScratch, phoenix};
//!
//! # fn invoke<'a>(program: &'a AccountInfo, account: &'a AccountInfo) -> Result<(), pinocchio::program_error::ProgramError> {
//! let mut scratch = CpiScratch::<8, 128>::new(account);
//! let args = phoenix::RegisterTraderArgs {
//!     max_positions: 16,
//!     trader_preference_bits: 0,
//!     trader_pda_index: 0,
//!     subaccount_index: 0,
//! };
//! let cpi = phoenix::RegisterTrader {
//!     phoenix_program: program,
//!     log_authority: account,
//!     global_config: account,
//!     payer: account,
//!     trader: account,
//!     trader_account: account,
//!     system_program: account,
//! };
//! cpi.invoke(args, &mut scratch)?;
//! # Ok(())
//! # }
//! ```

use core::array;

use borsh::BorshSerialize;
use pinocchio::account_info::AccountInfo;
use pinocchio::cpi::{ReturnData, get_return_data, slice_invoke, slice_invoke_signed};
use pinocchio::instruction::{AccountMeta, Instruction, Signer};
use pinocchio::program_error::ProgramError;

use crate::types::Instruction as RiseInstruction;

/// Reusable scratch space for repeated CPI calls.
///
/// This stores account-info references, account metas, and a mutable data
/// buffer in one allocation-free container. It is the easiest way to invoke a
/// sequence of Phoenix CPIs from a program while keeping stack usage explicit.
pub struct CpiScratch<'a, const MAX_ACCOUNTS: usize, const MAX_DATA_LEN: usize> {
    account_infos: [&'a AccountInfo; MAX_ACCOUNTS],
    account_metas: [AccountMeta<'a>; MAX_ACCOUNTS],
    data: [u8; MAX_DATA_LEN],
}

impl<'a, const MAX_ACCOUNTS: usize, const MAX_DATA_LEN: usize>
    CpiScratch<'a, MAX_ACCOUNTS, MAX_DATA_LEN>
{
    /// Create scratch buffers filled with a placeholder account reference.
    ///
    /// The placeholder is overwritten before invocation. A real account is
    /// still required for the actual CPI call.
    pub fn new(fill_account: &'a AccountInfo) -> Self {
        Self {
            account_infos: array::from_fn(|_| fill_account),
            account_metas: array::from_fn(|_| AccountMeta::readonly(fill_account.key())),
            data: [0; MAX_DATA_LEN],
        }
    }

    /// Borrow the underlying buffers for a single CPI preparation step.
    pub fn buffers(&mut self) -> CpiBuffers<'a, '_> {
        CpiBuffers {
            account_infos: &mut self.account_infos,
            account_metas: &mut self.account_metas,
            data: &mut self.data,
        }
    }
}

/// Borrowed account and instruction buffers used by CPI helpers.
///
/// Prefer this when you already have your own storage for account infos,
/// account metas, and instruction bytes.
pub struct CpiBuffers<'a, 'b> {
    pub account_infos: &'b mut [&'a AccountInfo],
    pub account_metas: &'b mut [AccountMeta<'a>],
    pub data: &'b mut [u8],
}

impl<'a, 'b> CpiBuffers<'a, 'b> {
    /// Wrap preallocated account and instruction buffers.
    pub fn new(
        account_infos: &'b mut [&'a AccountInfo],
        account_metas: &'b mut [AccountMeta<'a>],
        data: &'b mut [u8],
    ) -> Self {
        Self {
            account_infos,
            account_metas,
            data,
        }
    }
}

/// Populate account metas from a slice of `AccountInfo` references.
///
/// This is the simplest helper when you are invoking a generic instruction and
/// the account order is already correct.
pub fn account_metas<'a>(
    accounts: &'a [&'a AccountInfo],
    out: &mut [AccountMeta<'a>],
) -> Result<usize, ProgramError> {
    if out.len() < accounts.len() {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    for (meta, account) in out.iter_mut().zip(accounts.iter().copied()) {
        *meta = AccountMeta::new(account.key(), account.is_writable(), account.is_signer());
    }

    Ok(accounts.len())
}

/// Populate account metas from a Phoenix instruction definition.
///
/// This validates the program ID and account order against the encoded
/// instruction before building the metas.
pub fn instruction_account_metas<'a>(
    ix: &RiseInstruction,
    program: &AccountInfo,
    accounts: &'a [&'a AccountInfo],
    out: &mut [AccountMeta<'a>],
) -> Result<usize, ProgramError> {
    let expected_program_id = ix.program_id.to_bytes();
    if program.key() != &expected_program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    if ix.accounts.len() != accounts.len() || out.len() < accounts.len() {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    for ((out_meta, expected), account) in out
        .iter_mut()
        .zip(ix.accounts.iter())
        .zip(accounts.iter().copied())
    {
        let expected_pubkey = expected.pubkey.to_bytes();
        if account.key() != &expected_pubkey {
            return Err(ProgramError::InvalidAccountData);
        }
        *out_meta = AccountMeta::new(account.key(), expected.is_writable, expected.is_signer);
    }

    Ok(accounts.len())
}

/// Populate account infos in the order required by a Phoenix instruction.
///
/// This is a convenience bridge for programs that have a generated/off-chain
/// [`RiseInstruction`] with `AccountMeta`s and an outer instruction account
/// iterator or slice, but do not want to hand-write the account reordering
/// logic. For each account meta in `ix.accounts`, this helper scans
/// `available_accounts` from the beginning and copies the first `AccountInfo`
/// with a matching pubkey into `out`.
///
/// The helper itself does not allocate. If `out` is a stack array, the ordered
/// `AccountInfo` copies stay on the stack; if `out` is a `Vec`, the storage is
/// on the heap. `AccountInfo` is a small Pinocchio handle, so the copy is
/// cheap, but the lookup is a nested scan over instruction metas and available
/// accounts. Integrators on a tight CU budget should pass already-ordered
/// accounts directly or maintain their own indexed lookup.
pub fn instruction_account_infos<'info, I>(
    ix: &RiseInstruction,
    available_accounts: I,
    out: &mut [AccountInfo],
) -> Result<usize, ProgramError>
where
    I: Clone + IntoIterator<Item = &'info AccountInfo>,
{
    let account_count = ix.accounts.len();
    if out.len() < account_count {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    for (index, expected) in ix.accounts.iter().enumerate() {
        let expected_pubkey = expected.pubkey.to_bytes();
        let Some(account) = available_accounts
            .clone()
            .into_iter()
            .find(|account| account.key() == &expected_pubkey)
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };
        out[index] = *account;
    }

    Ok(account_count)
}

/// Build the ordered `&AccountInfo` slice required by Pinocchio CPI helpers.
///
/// This first calls [`instruction_account_infos`] to copy matching
/// `AccountInfo` handles into `account_storage`, then writes references to that
/// storage into `out`. This is useful when an integrator starts from an
/// instruction's `AccountMeta`s and an account iterator but wants to call
/// [`invoke_instruction`] or Pinocchio's `slice_invoke` APIs.
///
/// Like [`instruction_account_infos`], this helper does not allocate by itself:
/// stack arrays keep both the cloned account handles and reference slice on the
/// stack, while `Vec`-backed buffers place them on the heap. It also uses the
/// same nested scan, so callers with CU-sensitive paths may prefer explicit
/// account ordering.
pub fn instruction_account_info_refs<'storage, 'info, I>(
    ix: &RiseInstruction,
    available_accounts: I,
    account_storage: &'storage mut [AccountInfo],
    out: &mut [&'storage AccountInfo],
) -> Result<usize, ProgramError>
where
    I: Clone + IntoIterator<Item = &'info AccountInfo>,
{
    let account_count = ix.accounts.len();
    if out.len() < account_count {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let len = instruction_account_infos(ix, available_accounts, account_storage)?;
    for (out_account, account) in out.iter_mut().zip(account_storage.iter()).take(len) {
        *out_account = account;
    }
    Ok(len)
}

/// Invoke a raw instruction with already-encoded instruction data.
pub fn invoke<'a>(
    program: &AccountInfo,
    accounts: &'a [&'a AccountInfo],
    account_metas: &mut [AccountMeta<'a>],
    data: &[u8],
) -> Result<(), ProgramError> {
    let len = self::account_metas(accounts, account_metas)?;
    let instruction = Instruction {
        program_id: program.key(),
        accounts: &account_metas[..len],
        data,
    };
    slice_invoke(&instruction, accounts)
}

/// Invoke a Phoenix instruction after validating it against the supplied
/// `Instruction` wrapper.
pub fn invoke_instruction<'a>(
    ix: &RiseInstruction,
    program: &AccountInfo,
    accounts: &'a [&'a AccountInfo],
    account_metas: &mut [AccountMeta<'a>],
) -> Result<(), ProgramError> {
    let len = instruction_account_metas(ix, program, accounts, account_metas)?;
    let instruction = Instruction {
        program_id: program.key(),
        accounts: &account_metas[..len],
        data: &ix.data,
    };
    slice_invoke(&instruction, accounts)
}

/// Invoke a raw instruction using signer seeds.
pub fn invoke_signed<'a>(
    program: &AccountInfo,
    accounts: &'a [&'a AccountInfo],
    account_metas: &mut [AccountMeta<'a>],
    data: &[u8],
    signer_seeds: &[Signer<'_, '_>],
) -> Result<(), ProgramError> {
    let len = self::account_metas(accounts, account_metas)?;
    let instruction = Instruction {
        program_id: program.key(),
        accounts: &account_metas[..len],
        data,
    };
    slice_invoke_signed(&instruction, accounts, signer_seeds)
}

/// Invoke a raw instruction and return any captured return data.
pub fn invoke_with_return_data<'a>(
    program: &AccountInfo,
    accounts: &'a [&'a AccountInfo],
    account_metas: &mut [AccountMeta<'a>],
    data: &[u8],
) -> Result<Option<ReturnData>, ProgramError> {
    invoke(program, accounts, account_metas, data)?;
    Ok(get_return_data())
}

#[inline(always)]
fn require_accounts<'a>(
    buffers: &CpiBuffers<'a, '_>,
    account_count: usize,
) -> Result<(), ProgramError> {
    if buffers.account_infos.len() < account_count || buffers.account_metas.len() < account_count {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    Ok(())
}

#[inline(always)]
fn require_data(out: &[u8], data_len: usize) -> Result<(), crate::PhoenixIxError> {
    if out.len() < data_len {
        return Err(crate::PhoenixIxError::InstructionDataBufferTooSmall {
            expected: data_len,
            actual: out.len(),
        });
    }
    Ok(())
}

#[inline(always)]
fn write_account<'a>(
    buffers: &mut CpiBuffers<'a, '_>,
    index: usize,
    account: &'a AccountInfo,
    is_writable: bool,
    is_signer: bool,
) {
    buffers.account_infos[index] = account;
    buffers.account_metas[index] = AccountMeta::new(account.key(), is_writable, is_signer);
}

#[inline(always)]
fn write_bytes(
    out: &mut [u8],
    offset: &mut usize,
    bytes: &[u8],
) -> Result<(), crate::PhoenixIxError> {
    let end = offset
        .checked_add(bytes.len())
        .ok_or(crate::PhoenixIxError::InstructionDataTooLarge)?;
    if out.len() < end {
        return Err(crate::PhoenixIxError::InstructionDataBufferTooSmall {
            expected: end,
            actual: out.len(),
        });
    }
    out[*offset..end].copy_from_slice(bytes);
    *offset = end;
    Ok(())
}

#[inline(always)]
fn write_u8(out: &mut [u8], offset: &mut usize, value: u8) -> Result<(), crate::PhoenixIxError> {
    write_bytes(out, offset, &[value])
}

#[inline(always)]
fn write_bool(
    out: &mut [u8],
    offset: &mut usize,
    value: bool,
) -> Result<(), crate::PhoenixIxError> {
    write_u8(out, offset, u8::from(value))
}

#[inline(always)]
fn write_u64(out: &mut [u8], offset: &mut usize, value: u64) -> Result<(), crate::PhoenixIxError> {
    write_bytes(out, offset, &value.to_le_bytes())
}

#[inline(always)]
fn write_u32(out: &mut [u8], offset: &mut usize, value: u32) -> Result<(), crate::PhoenixIxError> {
    write_bytes(out, offset, &value.to_le_bytes())
}

#[inline(always)]
fn write_option_u64(
    out: &mut [u8],
    offset: &mut usize,
    value: Option<u64>,
) -> Result<(), crate::PhoenixIxError> {
    match value {
        Some(value) => {
            write_u8(out, offset, 1)?;
            write_u64(out, offset, value)
        }
        None => write_u8(out, offset, 0),
    }
}

#[inline(always)]
fn write_option_u8(
    out: &mut [u8],
    offset: &mut usize,
    value: Option<u8>,
) -> Result<(), crate::PhoenixIxError> {
    match value {
        Some(value) => {
            write_u8(out, offset, 1)?;
            write_u8(out, offset, value)
        }
        None => write_u8(out, offset, 0),
    }
}

struct SliceWriter<'a> {
    out: &'a mut [u8],
    offset: usize,
}

impl borsh::io::Write for SliceWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> borsh::io::Result<usize> {
        let end = self
            .offset
            .checked_add(bytes.len())
            .ok_or_else(|| borsh::io::Error::other("instruction data length overflow"))?;
        if self.out.len() < end {
            return Err(borsh::io::Error::new(
                borsh::io::ErrorKind::WriteZero,
                "instruction data buffer too small",
            ));
        }
        self.out[self.offset..end].copy_from_slice(bytes);
        self.offset = end;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> borsh::io::Result<()> {
        Ok(())
    }
}

#[inline(always)]
fn write_borsh<T: BorshSerialize + ?Sized>(
    out: &mut [u8],
    offset: &mut usize,
    value: &T,
) -> Result<(), crate::PhoenixIxError> {
    let start = *offset;
    let mut writer = SliceWriter {
        out: &mut out[start..],
        offset: 0,
    };
    value
        .serialize(&mut writer)
        .map_err(|_| crate::PhoenixIxError::InstructionDataTooLarge)?;
    *offset = start
        .checked_add(writer.offset)
        .ok_or(crate::PhoenixIxError::InstructionDataTooLarge)?;
    Ok(())
}

#[inline(always)]
fn map_ix_error(_error: crate::PhoenixIxError) -> ProgramError {
    ProgramError::InvalidInstructionData
}

#[inline(always)]
fn invoke_prepared<'a>(
    program: &AccountInfo,
    account_count: usize,
    data_len: usize,
    buffers: &mut CpiBuffers<'a, '_>,
) -> Result<(), ProgramError> {
    let instruction = Instruction {
        program_id: program.key(),
        accounts: &buffers.account_metas[..account_count],
        data: &buffers.data[..data_len],
    };
    slice_invoke(&instruction, &buffers.account_infos[..account_count])
}

#[inline(always)]
fn invoke_prepared_signed<'a>(
    program: &AccountInfo,
    account_count: usize,
    data_len: usize,
    signer_seeds: &[Signer<'_, '_>],
    buffers: &mut CpiBuffers<'a, '_>,
) -> Result<(), ProgramError> {
    let instruction = Instruction {
        program_id: program.key(),
        accounts: &buffers.account_metas[..account_count],
        data: &buffers.data[..data_len],
    };
    slice_invoke_signed(
        &instruction,
        &buffers.account_infos[..account_count],
        signer_seeds,
    )
}

/// Typed CPI wrappers for the Phoenix program.
///
/// These structs model the account layout and instruction payload for common
/// Phoenix CPIs. They are useful when you want the compiler to help with
/// account ordering and payload construction while still staying in on-chain
/// code.
pub mod phoenix {
    use pinocchio::account_info::AccountInfo;
    use pinocchio::instruction::Signer;
    use pinocchio::program_error::ProgramError;

    use super::{
        CpiBuffers, CpiScratch, invoke_prepared, invoke_prepared_signed, map_ix_error,
        require_accounts, require_data, write_account, write_bool, write_borsh, write_bytes,
        write_option_u8, write_option_u64, write_u8, write_u32, write_u64,
    };
    use crate::cancel_orders::MAX_CANCEL_ORDER_IDS;
    use crate::order_packet::{CondensedOrder, OrderPacket, client_order_id_to_bytes};
    use crate::prelude::{
        CancelId, Direction, FifoOrderId, OrderFlags, PhoenixInstruction, SelfTradeBehavior, Side,
        StopLossOrderKind, TriggerOrderParams,
    };
    use crate::register_trader::TraderPreferenceKind;

    pub const REGISTER_TRADER_ACCOUNT_COUNT: usize = 7;
    pub const REGISTER_TRADER_DATA_LEN: usize = 18;
    pub const SET_TRADER_CAPABILITIES_DELEGATED_MIN_ACCOUNT_COUNT: usize = 6;
    pub const SET_TRADER_CAPABILITIES_DELEGATED_DATA_LEN: usize = 24;
    pub const PLACE_MARKET_ORDER_MIN_ACCOUNT_COUNT: usize = 8;
    pub const PLACE_MARKET_ORDER_MAX_DATA_LEN: usize = 89;
    pub const PLACE_LIMIT_ORDER_MIN_ACCOUNT_COUNT: usize = 8;
    pub const PLACE_LIMIT_ORDER_MAX_DATA_LEN: usize = 63;
    pub const PLACE_MULTI_LIMIT_ORDER_MIN_ACCOUNT_COUNT: usize = 8;
    pub const PLACE_MARKET_ORDER_DELEGATED_MIN_ACCOUNT_COUNT: usize = 9;
    pub const PHOENIX_DEPOSIT_MIN_ACCOUNT_COUNT: usize = 8;
    pub const PHOENIX_DEPOSIT_DATA_LEN: usize = 16;
    pub const PHOENIX_WITHDRAW_MIN_ACCOUNT_COUNT: usize = 10;
    pub const PHOENIX_WITHDRAW_DATA_LEN: usize = 16;
    pub const TRANSFER_COLLATERAL_MIN_ACCOUNT_COUNT: usize = 7;
    pub const TRANSFER_COLLATERAL_DATA_LEN: usize = 16;
    pub const TRANSFER_COLLATERAL_CHILD_TO_PARENT_MIN_ACCOUNT_COUNT: usize = 7;
    pub const SYNC_PARENT_TO_CHILD_MIN_ACCOUNT_COUNT: usize = 6;
    pub const UPDATE_TRADER_STATE_MIN_ACCOUNT_COUNT: usize = 5;
    pub const PLACE_STOP_LOSS_MIN_ACCOUNT_COUNT: usize = 11;
    pub const PLACE_STOP_LOSS_DATA_LEN: usize = 35;
    pub const CANCEL_STOP_LOSS_ACCOUNT_COUNT: usize = 8;
    pub const CANCEL_STOP_LOSS_DATA_LEN: usize = 9;
    pub const CREATE_CONDITIONAL_ORDERS_ACCOUNT_COUNT: usize = 8;
    pub const CREATE_CONDITIONAL_ORDERS_ACCOUNT_DATA_LEN: usize = 9;
    pub const PLACE_POSITION_CONDITIONAL_ORDER_MIN_ACCOUNT_COUNT: usize = 10;
    pub const PLACE_POSITION_CONDITIONAL_ORDER_MAX_DATA_LEN: usize = 63;
    pub const CANCEL_CONDITIONAL_ORDER_ACCOUNT_COUNT: usize = 7;
    pub const CANCEL_CONDITIONAL_ORDER_DATA_LEN: usize = 11;
    pub const PLACE_ATTACHED_CONDITIONAL_ORDER_MIN_ACCOUNT_COUNT: usize = 9;
    pub const PLACE_ATTACHED_CONDITIONAL_ORDER_MAX_DATA_LEN: usize = 68;
    pub const PLACE_LIMIT_ORDER_WITH_CONDITIONALS_MIN_ACCOUNT_COUNT: usize = 11;
    pub const PLACE_LIMIT_ORDER_WITH_CONDITIONALS_MAX_DATA_LEN: usize = 137;
    pub const CANCEL_ALL_PLUS_CONDITIONAL_MIN_ACCOUNT_COUNT: usize = 9;
    pub const CANCEL_ORDER_MIN_ACCOUNT_COUNT: usize = 8;
    pub const CANCEL_ALL_DATA_LEN: usize = 8;
    pub const CANCEL_UP_TO_MAX_DATA_LEN: usize = 27;
    pub const CANCEL_ORDERS_BY_ID_MAX_DATA_LEN: usize = 8 + 4 + MAX_CANCEL_ORDER_IDS * 20;
    pub const DISCRIMINANT_DATA_LEN: usize = 8;

    const ALL_TRADER_CAPABILITY_UPDATE_BYTES: [u8; 16] = [
        6, 0, 0, 0, // Vec length.
        0, 1, // PlaceLimitOrder.
        1, 1, // PlaceMarketOrder.
        3, 1, // RiskReducingTrade.
        2, 1, // RiskIncreasingTrade.
        4, 1, // DepositCollateral.
        5, 1, // WithdrawCollateral.
    ];

    macro_rules! impl_phoenix_invoke_methods {
        ($context:ident, $args:ty, $write_data:ident) => {
            pub fn invoke<const MAX_ACCOUNTS: usize, const MAX_DATA_LEN: usize>(
                &self,
                args: $args,
                scratch: &mut CpiScratch<'a, MAX_ACCOUNTS, MAX_DATA_LEN>,
            ) -> Result<(), ProgramError> {
                self.invoke_with_buffers(args, &mut scratch.buffers())
            }

            pub fn invoke_signed<const MAX_ACCOUNTS: usize, const MAX_DATA_LEN: usize>(
                &self,
                args: $args,
                signer_seeds: &[Signer<'_, '_>],
                scratch: &mut CpiScratch<'a, MAX_ACCOUNTS, MAX_DATA_LEN>,
            ) -> Result<(), ProgramError> {
                self.invoke_signed_with_buffers(args, signer_seeds, &mut scratch.buffers())
            }

            pub fn invoke_with_buffers(
                &self,
                args: $args,
                buffers: &mut CpiBuffers<'a, '_>,
            ) -> Result<(), ProgramError> {
                let account_count = self.write_accounts(buffers)?;
                let data_len = $write_data(args, buffers.data).map_err(map_ix_error)?;
                invoke_prepared(self.phoenix_program, account_count, data_len, buffers)
            }

            pub fn invoke_signed_with_buffers(
                &self,
                args: $args,
                signer_seeds: &[Signer<'_, '_>],
                buffers: &mut CpiBuffers<'a, '_>,
            ) -> Result<(), ProgramError> {
                let account_count = self.write_accounts(buffers)?;
                let data_len = $write_data(args, buffers.data).map_err(map_ix_error)?;
                invoke_prepared_signed(
                    self.phoenix_program,
                    account_count,
                    data_len,
                    signer_seeds,
                    buffers,
                )
            }
        };
    }

    struct MarketActionAccountRefs<'a> {
        phoenix_program: &'a AccountInfo,
        log_authority: &'a AccountInfo,
        global_config: &'a AccountInfo,
        trader: &'a AccountInfo,
        trader_account: &'a AccountInfo,
        perp_asset_map: &'a AccountInfo,
        global_trader_index: &'a [AccountInfo],
        active_trader_buffer: &'a [AccountInfo],
        orderbook: &'a AccountInfo,
        spline_collection: &'a AccountInfo,
    }

    fn write_market_action_accounts<'a>(
        buffers: &mut CpiBuffers<'a, '_>,
        accounts: MarketActionAccountRefs<'a>,
    ) -> Result<usize, ProgramError> {
        let account_count = PLACE_MARKET_ORDER_MIN_ACCOUNT_COUNT
            + accounts.global_trader_index.len()
            + accounts.active_trader_buffer.len();
        require_accounts(buffers, account_count)?;
        require_trader_arenas(accounts.global_trader_index, accounts.active_trader_buffer)?;

        write_account(buffers, 0, accounts.phoenix_program, false, false);
        write_account(buffers, 1, accounts.log_authority, false, false);
        write_account(buffers, 2, accounts.global_config, true, false);
        write_account(buffers, 3, accounts.trader, false, true);
        write_account(buffers, 4, accounts.trader_account, true, false);
        write_account(buffers, 5, accounts.perp_asset_map, true, false);
        let mut index = 6;
        for account in accounts.global_trader_index {
            write_account(buffers, index, account, true, false);
            index += 1;
        }
        for account in accounts.active_trader_buffer {
            write_account(buffers, index, account, true, false);
            index += 1;
        }
        write_account(buffers, index, accounts.orderbook, true, false);
        index += 1;
        write_account(buffers, index, accounts.spline_collection, true, false);
        Ok(account_count)
    }

    fn require_trader_arenas(
        global_trader_index: &[AccountInfo],
        active_trader_buffer: &[AccountInfo],
    ) -> Result<(), ProgramError> {
        if global_trader_index.is_empty() || active_trader_buffer.is_empty() {
            return Err(ProgramError::NotEnoughAccountKeys);
        }
        Ok(())
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct RegisterTraderArgs {
        pub max_positions: u64,
        pub trader_preference_bits: u32,
        pub trader_pda_index: u8,
        pub subaccount_index: u8,
    }

    impl RegisterTraderArgs {
        pub const fn new(max_positions: u64, trader_pda_index: u8, subaccount_index: u8) -> Self {
            Self {
                max_positions,
                trader_preference_bits: 0,
                trader_pda_index,
                subaccount_index,
            }
        }

        pub const fn with_trader_preference_bits(mut self, trader_preference_bits: u32) -> Self {
            self.trader_preference_bits = trader_preference_bits;
            self
        }

        pub const fn with_trader_preference(mut self, preference: TraderPreferenceKind) -> Self {
            self.trader_preference_bits |= preference.bit();
            self
        }

        pub const fn with_disable_collateral_sweep(self, disable_collateral_sweep: bool) -> Self {
            if disable_collateral_sweep {
                self.with_trader_preference(TraderPreferenceKind::DisableCollateralSweep)
            } else {
                let mut args = self;
                args.trader_preference_bits &= !TraderPreferenceKind::DisableCollateralSweep.bit();
                args
            }
        }
    }

    pub struct RegisterTrader<'a> {
        pub phoenix_program: &'a AccountInfo,
        pub log_authority: &'a AccountInfo,
        pub global_config: &'a AccountInfo,
        pub payer: &'a AccountInfo,
        pub trader: &'a AccountInfo,
        pub trader_account: &'a AccountInfo,
        pub system_program: &'a AccountInfo,
    }

    impl<'a> RegisterTrader<'a> {
        pub const ACCOUNT_COUNT: usize = REGISTER_TRADER_ACCOUNT_COUNT;
        pub const DATA_LEN: usize = REGISTER_TRADER_DATA_LEN;

        pub fn invoke<const MAX_ACCOUNTS: usize, const MAX_DATA_LEN: usize>(
            &self,
            args: RegisterTraderArgs,
            scratch: &mut CpiScratch<'a, MAX_ACCOUNTS, MAX_DATA_LEN>,
        ) -> Result<(), ProgramError> {
            self.invoke_with_buffers(args, &mut scratch.buffers())
        }

        pub fn invoke_signed<const MAX_ACCOUNTS: usize, const MAX_DATA_LEN: usize>(
            &self,
            args: RegisterTraderArgs,
            signer_seeds: &[Signer<'_, '_>],
            scratch: &mut CpiScratch<'a, MAX_ACCOUNTS, MAX_DATA_LEN>,
        ) -> Result<(), ProgramError> {
            self.invoke_signed_with_buffers(args, signer_seeds, &mut scratch.buffers())
        }

        pub fn invoke_with_buffers(
            &self,
            args: RegisterTraderArgs,
            buffers: &mut CpiBuffers<'a, '_>,
        ) -> Result<(), ProgramError> {
            self.write_accounts(buffers)?;
            let data_len = write_register_trader_data(args, buffers.data).map_err(map_ix_error)?;
            invoke_prepared(self.phoenix_program, Self::ACCOUNT_COUNT, data_len, buffers)
        }

        pub fn invoke_signed_with_buffers(
            &self,
            args: RegisterTraderArgs,
            signer_seeds: &[Signer<'_, '_>],
            buffers: &mut CpiBuffers<'a, '_>,
        ) -> Result<(), ProgramError> {
            self.write_accounts(buffers)?;
            let data_len = write_register_trader_data(args, buffers.data).map_err(map_ix_error)?;
            invoke_prepared_signed(
                self.phoenix_program,
                Self::ACCOUNT_COUNT,
                data_len,
                signer_seeds,
                buffers,
            )
        }

        fn write_accounts(&self, buffers: &mut CpiBuffers<'a, '_>) -> Result<(), ProgramError> {
            require_accounts(buffers, Self::ACCOUNT_COUNT)?;
            write_account(buffers, 0, self.phoenix_program, false, false);
            write_account(buffers, 1, self.log_authority, false, false);
            write_account(buffers, 2, self.global_config, false, false);
            write_account(buffers, 3, self.payer, true, true);
            write_account(buffers, 4, self.trader, false, false);
            write_account(buffers, 5, self.trader_account, true, false);
            write_account(buffers, 6, self.system_program, false, false);
            Ok(())
        }
    }

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub struct SetTraderCapabilitiesDelegatedArgs;

    pub struct SetTraderCapabilitiesDelegated<'a> {
        pub phoenix_program: &'a AccountInfo,
        pub log_authority: &'a AccountInfo,
        pub global_config: &'a AccountInfo,
        pub authority: &'a AccountInfo,
        pub permission_account: &'a AccountInfo,
        pub trader_account: &'a AccountInfo,
        pub global_trader_index: &'a [AccountInfo],
        pub active_trader_buffer: &'a [AccountInfo],
    }

    pub type SetTraderCapabilityDelegated<'a> = SetTraderCapabilitiesDelegated<'a>;

    impl<'a> SetTraderCapabilitiesDelegated<'a> {
        pub const DATA_LEN: usize = SET_TRADER_CAPABILITIES_DELEGATED_DATA_LEN;
        pub const MIN_ACCOUNT_COUNT: usize = SET_TRADER_CAPABILITIES_DELEGATED_MIN_ACCOUNT_COUNT;

        pub fn account_count(&self) -> usize {
            Self::MIN_ACCOUNT_COUNT
                + self.global_trader_index.len()
                + self.active_trader_buffer.len()
        }

        pub fn invoke<const MAX_ACCOUNTS: usize, const MAX_DATA_LEN: usize>(
            &self,
            args: SetTraderCapabilitiesDelegatedArgs,
            scratch: &mut CpiScratch<'a, MAX_ACCOUNTS, MAX_DATA_LEN>,
        ) -> Result<(), ProgramError> {
            self.invoke_with_buffers(args, &mut scratch.buffers())
        }

        pub fn invoke_signed<const MAX_ACCOUNTS: usize, const MAX_DATA_LEN: usize>(
            &self,
            args: SetTraderCapabilitiesDelegatedArgs,
            signer_seeds: &[Signer<'_, '_>],
            scratch: &mut CpiScratch<'a, MAX_ACCOUNTS, MAX_DATA_LEN>,
        ) -> Result<(), ProgramError> {
            self.invoke_signed_with_buffers(args, signer_seeds, &mut scratch.buffers())
        }

        pub fn invoke_with_buffers(
            &self,
            _args: SetTraderCapabilitiesDelegatedArgs,
            buffers: &mut CpiBuffers<'a, '_>,
        ) -> Result<(), ProgramError> {
            let account_count = self.write_accounts(buffers)?;
            let data_len =
                write_set_trader_capabilities_delegated_data(buffers.data).map_err(map_ix_error)?;
            invoke_prepared(self.phoenix_program, account_count, data_len, buffers)
        }

        pub fn invoke_signed_with_buffers(
            &self,
            _args: SetTraderCapabilitiesDelegatedArgs,
            signer_seeds: &[Signer<'_, '_>],
            buffers: &mut CpiBuffers<'a, '_>,
        ) -> Result<(), ProgramError> {
            let account_count = self.write_accounts(buffers)?;
            let data_len =
                write_set_trader_capabilities_delegated_data(buffers.data).map_err(map_ix_error)?;
            invoke_prepared_signed(
                self.phoenix_program,
                account_count,
                data_len,
                signer_seeds,
                buffers,
            )
        }

        fn write_accounts(&self, buffers: &mut CpiBuffers<'a, '_>) -> Result<usize, ProgramError> {
            let account_count = self.account_count();
            require_accounts(buffers, account_count)?;
            if self.global_trader_index.is_empty() {
                return Err(ProgramError::NotEnoughAccountKeys);
            }
            if self.active_trader_buffer.is_empty() {
                return Err(ProgramError::NotEnoughAccountKeys);
            }

            write_account(buffers, 0, self.phoenix_program, false, false);
            write_account(buffers, 1, self.log_authority, false, false);
            write_account(buffers, 2, self.global_config, false, false);
            write_account(buffers, 3, self.authority, false, true);
            write_account(buffers, 4, self.permission_account, true, false);
            write_account(buffers, 5, self.trader_account, true, false);
            let mut index = 6;
            for account in self.global_trader_index {
                write_account(buffers, index, account, true, false);
                index += 1;
            }
            for account in self.active_trader_buffer {
                write_account(buffers, index, account, true, false);
                index += 1;
            }
            Ok(account_count)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PlaceMarketOrderArgs {
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
        pub order_flags: OrderFlags,
        pub cancel_existing: bool,
    }

    pub struct PlaceMarketOrder<'a> {
        pub phoenix_program: &'a AccountInfo,
        pub log_authority: &'a AccountInfo,
        pub global_config: &'a AccountInfo,
        pub trader: &'a AccountInfo,
        pub trader_account: &'a AccountInfo,
        pub perp_asset_map: &'a AccountInfo,
        pub global_trader_index: &'a [AccountInfo],
        pub active_trader_buffer: &'a [AccountInfo],
        pub orderbook: &'a AccountInfo,
        pub spline_collection: &'a AccountInfo,
    }

    impl<'a> PlaceMarketOrder<'a> {
        pub const MAX_DATA_LEN: usize = PLACE_MARKET_ORDER_MAX_DATA_LEN;
        pub const MIN_ACCOUNT_COUNT: usize = PLACE_MARKET_ORDER_MIN_ACCOUNT_COUNT;

        pub fn account_count(&self) -> usize {
            Self::MIN_ACCOUNT_COUNT
                + self.global_trader_index.len()
                + self.active_trader_buffer.len()
        }

        pub fn invoke<const MAX_ACCOUNTS: usize, const MAX_DATA_LEN: usize>(
            &self,
            args: PlaceMarketOrderArgs,
            scratch: &mut CpiScratch<'a, MAX_ACCOUNTS, MAX_DATA_LEN>,
        ) -> Result<(), ProgramError> {
            self.invoke_with_buffers(args, &mut scratch.buffers())
        }

        pub fn invoke_signed<const MAX_ACCOUNTS: usize, const MAX_DATA_LEN: usize>(
            &self,
            args: PlaceMarketOrderArgs,
            signer_seeds: &[Signer<'_, '_>],
            scratch: &mut CpiScratch<'a, MAX_ACCOUNTS, MAX_DATA_LEN>,
        ) -> Result<(), ProgramError> {
            self.invoke_signed_with_buffers(args, signer_seeds, &mut scratch.buffers())
        }

        pub fn invoke_with_buffers(
            &self,
            args: PlaceMarketOrderArgs,
            buffers: &mut CpiBuffers<'a, '_>,
        ) -> Result<(), ProgramError> {
            let account_count = self.write_accounts(buffers)?;
            let data_len =
                write_place_market_order_data(args, buffers.data).map_err(map_ix_error)?;
            invoke_prepared(self.phoenix_program, account_count, data_len, buffers)
        }

        pub fn invoke_signed_with_buffers(
            &self,
            args: PlaceMarketOrderArgs,
            signer_seeds: &[Signer<'_, '_>],
            buffers: &mut CpiBuffers<'a, '_>,
        ) -> Result<(), ProgramError> {
            let account_count = self.write_accounts(buffers)?;
            let data_len =
                write_place_market_order_data(args, buffers.data).map_err(map_ix_error)?;
            invoke_prepared_signed(
                self.phoenix_program,
                account_count,
                data_len,
                signer_seeds,
                buffers,
            )
        }

        fn write_accounts(&self, buffers: &mut CpiBuffers<'a, '_>) -> Result<usize, ProgramError> {
            let account_count = self.account_count();
            require_accounts(buffers, account_count)?;
            if self.global_trader_index.is_empty() {
                return Err(ProgramError::NotEnoughAccountKeys);
            }
            if self.active_trader_buffer.is_empty() {
                return Err(ProgramError::NotEnoughAccountKeys);
            }

            write_account(buffers, 0, self.phoenix_program, false, false);
            write_account(buffers, 1, self.log_authority, false, false);
            write_account(buffers, 2, self.global_config, true, false);
            write_account(buffers, 3, self.trader, false, true);
            write_account(buffers, 4, self.trader_account, true, false);
            write_account(buffers, 5, self.perp_asset_map, true, false);
            let mut index = 6;
            for account in self.global_trader_index {
                write_account(buffers, index, account, true, false);
                index += 1;
            }
            for account in self.active_trader_buffer {
                write_account(buffers, index, account, true, false);
                index += 1;
            }
            write_account(buffers, index, self.orderbook, true, false);
            index += 1;
            write_account(buffers, index, self.spline_collection, true, false);
            Ok(account_count)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PlaceLimitOrderArgs {
        pub side: Side,
        pub price_in_ticks: u64,
        pub num_base_lots: u64,
        pub self_trade_behavior: SelfTradeBehavior,
        pub match_limit: Option<u64>,
        pub client_order_id: u128,
        pub last_valid_slot: Option<u64>,
        pub order_flags: OrderFlags,
        pub cancel_existing: bool,
    }

    pub struct PlaceLimitOrder<'a> {
        pub phoenix_program: &'a AccountInfo,
        pub log_authority: &'a AccountInfo,
        pub global_config: &'a AccountInfo,
        pub trader: &'a AccountInfo,
        pub trader_account: &'a AccountInfo,
        pub perp_asset_map: &'a AccountInfo,
        pub global_trader_index: &'a [AccountInfo],
        pub active_trader_buffer: &'a [AccountInfo],
        pub orderbook: &'a AccountInfo,
        pub spline_collection: &'a AccountInfo,
    }

    impl<'a> PlaceLimitOrder<'a> {
        pub const MAX_DATA_LEN: usize = PLACE_LIMIT_ORDER_MAX_DATA_LEN;
        pub const MIN_ACCOUNT_COUNT: usize = PLACE_LIMIT_ORDER_MIN_ACCOUNT_COUNT;

        impl_phoenix_invoke_methods!(
            PlaceLimitOrder,
            PlaceLimitOrderArgs,
            write_place_limit_order_data
        );

        pub fn account_count(&self) -> usize {
            Self::MIN_ACCOUNT_COUNT
                + self.global_trader_index.len()
                + self.active_trader_buffer.len()
        }

        fn write_accounts(&self, buffers: &mut CpiBuffers<'a, '_>) -> Result<usize, ProgramError> {
            write_market_action_accounts(
                buffers,
                MarketActionAccountRefs {
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
                },
            )
        }
    }

    pub type PlaceMarketOrderDelegatedArgs = PlaceMarketOrderArgs;

    pub struct PlaceMarketOrderDelegated<'a> {
        pub phoenix_program: &'a AccountInfo,
        pub log_authority: &'a AccountInfo,
        pub global_config: &'a AccountInfo,
        pub trader_wallet: &'a AccountInfo,
        pub permission_account: &'a AccountInfo,
        pub trader_account: &'a AccountInfo,
        pub perp_asset_map: &'a AccountInfo,
        pub global_trader_index: &'a [AccountInfo],
        pub active_trader_buffer: &'a [AccountInfo],
        pub orderbook: &'a AccountInfo,
        pub spline_collection: &'a AccountInfo,
    }

    impl<'a> PlaceMarketOrderDelegated<'a> {
        pub const MAX_DATA_LEN: usize = PLACE_MARKET_ORDER_MAX_DATA_LEN;
        pub const MIN_ACCOUNT_COUNT: usize = PLACE_MARKET_ORDER_DELEGATED_MIN_ACCOUNT_COUNT;

        impl_phoenix_invoke_methods!(
            PlaceMarketOrderDelegated,
            PlaceMarketOrderDelegatedArgs,
            write_place_market_order_delegated_data
        );

        pub fn account_count(&self) -> usize {
            Self::MIN_ACCOUNT_COUNT
                + self.global_trader_index.len()
                + self.active_trader_buffer.len()
        }

        fn write_accounts(&self, buffers: &mut CpiBuffers<'a, '_>) -> Result<usize, ProgramError> {
            let account_count = self.account_count();
            require_accounts(buffers, account_count)?;
            require_trader_arenas(self.global_trader_index, self.active_trader_buffer)?;

            write_account(buffers, 0, self.phoenix_program, false, false);
            write_account(buffers, 1, self.log_authority, false, false);
            write_account(buffers, 2, self.global_config, true, false);
            write_account(buffers, 3, self.trader_wallet, false, true);
            write_account(buffers, 4, self.permission_account, true, false);
            write_account(buffers, 5, self.trader_account, true, false);
            write_account(buffers, 6, self.perp_asset_map, true, false);
            let mut index = 7;
            for account in self.global_trader_index {
                write_account(buffers, index, account, true, false);
                index += 1;
            }
            for account in self.active_trader_buffer {
                write_account(buffers, index, account, true, false);
                index += 1;
            }
            write_account(buffers, index, self.orderbook, true, false);
            index += 1;
            write_account(buffers, index, self.spline_collection, true, false);
            Ok(account_count)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PhoenixDepositArgs {
        pub amount: u64,
    }

    pub struct PhoenixDeposit<'a> {
        pub phoenix_program: &'a AccountInfo,
        pub log_authority: &'a AccountInfo,
        pub global_config: &'a AccountInfo,
        pub trader: &'a AccountInfo,
        pub trader_token_account: &'a AccountInfo,
        pub trader_account: &'a AccountInfo,
        pub global_vault: &'a AccountInfo,
        pub token_program: &'a AccountInfo,
        pub global_trader_index: &'a [AccountInfo],
        pub active_trader_buffer: &'a [AccountInfo],
        pub permission_account: Option<&'a AccountInfo>,
    }

    pub type DepositFunds<'a> = PhoenixDeposit<'a>;

    impl<'a> PhoenixDeposit<'a> {
        pub const DATA_LEN: usize = PHOENIX_DEPOSIT_DATA_LEN;
        pub const MIN_ACCOUNT_COUNT: usize = PHOENIX_DEPOSIT_MIN_ACCOUNT_COUNT;

        impl_phoenix_invoke_methods!(
            PhoenixDeposit,
            PhoenixDepositArgs,
            write_phoenix_deposit_data
        );

        pub fn account_count(&self) -> usize {
            Self::MIN_ACCOUNT_COUNT
                + self.global_trader_index.len()
                + self.active_trader_buffer.len()
                + usize::from(self.permission_account.is_some())
        }

        fn write_accounts(&self, buffers: &mut CpiBuffers<'a, '_>) -> Result<usize, ProgramError> {
            let account_count = self.account_count();
            require_accounts(buffers, account_count)?;
            require_trader_arenas(self.global_trader_index, self.active_trader_buffer)?;

            write_account(buffers, 0, self.phoenix_program, false, false);
            write_account(buffers, 1, self.log_authority, false, false);
            write_account(buffers, 2, self.global_config, true, false);
            write_account(buffers, 3, self.trader, false, true);
            write_account(buffers, 4, self.trader_token_account, true, false);
            write_account(buffers, 5, self.trader_account, true, false);
            write_account(buffers, 6, self.global_vault, true, false);
            write_account(buffers, 7, self.token_program, false, false);
            let mut index = 8;
            for account in self.global_trader_index {
                write_account(buffers, index, account, true, false);
                index += 1;
            }
            for account in self.active_trader_buffer {
                write_account(buffers, index, account, true, false);
                index += 1;
            }
            if let Some(permission_account) = self.permission_account {
                write_account(buffers, index, permission_account, true, false);
            }
            Ok(account_count)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PhoenixWithdrawArgs {
        pub amount: u64,
    }

    pub struct PhoenixWithdraw<'a> {
        pub phoenix_program: &'a AccountInfo,
        pub log_authority: &'a AccountInfo,
        pub global_config: &'a AccountInfo,
        pub trader: &'a AccountInfo,
        pub trader_account: &'a AccountInfo,
        pub perp_asset_map: &'a AccountInfo,
        pub global_vault: &'a AccountInfo,
        pub trader_token_account: &'a AccountInfo,
        pub token_program: &'a AccountInfo,
        pub global_trader_index: &'a [AccountInfo],
        pub active_trader_buffer: &'a [AccountInfo],
        pub withdraw_queue: &'a AccountInfo,
    }

    impl<'a> PhoenixWithdraw<'a> {
        pub const DATA_LEN: usize = PHOENIX_WITHDRAW_DATA_LEN;
        pub const MIN_ACCOUNT_COUNT: usize = PHOENIX_WITHDRAW_MIN_ACCOUNT_COUNT;

        pub fn account_count(&self) -> usize {
            Self::MIN_ACCOUNT_COUNT
                + self.global_trader_index.len()
                + self.active_trader_buffer.len()
        }

        pub fn invoke<const MAX_ACCOUNTS: usize, const MAX_DATA_LEN: usize>(
            &self,
            args: PhoenixWithdrawArgs,
            scratch: &mut CpiScratch<'a, MAX_ACCOUNTS, MAX_DATA_LEN>,
        ) -> Result<(), ProgramError> {
            self.invoke_with_buffers(args, &mut scratch.buffers())
        }

        pub fn invoke_signed<const MAX_ACCOUNTS: usize, const MAX_DATA_LEN: usize>(
            &self,
            args: PhoenixWithdrawArgs,
            signer_seeds: &[Signer<'_, '_>],
            scratch: &mut CpiScratch<'a, MAX_ACCOUNTS, MAX_DATA_LEN>,
        ) -> Result<(), ProgramError> {
            self.invoke_signed_with_buffers(args, signer_seeds, &mut scratch.buffers())
        }

        pub fn invoke_with_buffers(
            &self,
            args: PhoenixWithdrawArgs,
            buffers: &mut CpiBuffers<'a, '_>,
        ) -> Result<(), ProgramError> {
            let account_count = self.write_accounts(buffers)?;
            let data_len = write_phoenix_withdraw_data(args, buffers.data).map_err(map_ix_error)?;
            invoke_prepared(self.phoenix_program, account_count, data_len, buffers)
        }

        pub fn invoke_signed_with_buffers(
            &self,
            args: PhoenixWithdrawArgs,
            signer_seeds: &[Signer<'_, '_>],
            buffers: &mut CpiBuffers<'a, '_>,
        ) -> Result<(), ProgramError> {
            let account_count = self.write_accounts(buffers)?;
            let data_len = write_phoenix_withdraw_data(args, buffers.data).map_err(map_ix_error)?;
            invoke_prepared_signed(
                self.phoenix_program,
                account_count,
                data_len,
                signer_seeds,
                buffers,
            )
        }

        fn write_accounts(&self, buffers: &mut CpiBuffers<'a, '_>) -> Result<usize, ProgramError> {
            let account_count = self.account_count();
            require_accounts(buffers, account_count)?;
            if self.global_trader_index.is_empty() {
                return Err(ProgramError::NotEnoughAccountKeys);
            }
            if self.active_trader_buffer.is_empty() {
                return Err(ProgramError::NotEnoughAccountKeys);
            }

            write_account(buffers, 0, self.phoenix_program, false, false);
            write_account(buffers, 1, self.log_authority, false, false);
            write_account(buffers, 2, self.global_config, true, false);
            write_account(buffers, 3, self.trader, false, true);
            write_account(buffers, 4, self.trader_account, true, false);
            write_account(buffers, 5, self.perp_asset_map, true, false);
            write_account(buffers, 6, self.global_vault, true, false);
            write_account(buffers, 7, self.trader_token_account, true, false);
            write_account(buffers, 8, self.token_program, false, false);
            let mut index = 9;
            for account in self.global_trader_index {
                write_account(buffers, index, account, true, false);
                index += 1;
            }
            for account in self.active_trader_buffer {
                write_account(buffers, index, account, true, false);
                index += 1;
            }
            write_account(buffers, index, self.withdraw_queue, true, false);
            Ok(account_count)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TransferCollateralArgs {
        pub amount: u64,
    }

    pub struct TransferCollateral<'a> {
        pub phoenix_program: &'a AccountInfo,
        pub log_authority: &'a AccountInfo,
        pub global_config: &'a AccountInfo,
        pub trader: &'a AccountInfo,
        pub src_trader_account: &'a AccountInfo,
        pub dst_trader_account: &'a AccountInfo,
        pub perp_asset_map: &'a AccountInfo,
        pub global_trader_index: &'a [AccountInfo],
        pub active_trader_buffer: &'a [AccountInfo],
        pub permission_account: Option<&'a AccountInfo>,
    }

    impl<'a> TransferCollateral<'a> {
        pub const DATA_LEN: usize = TRANSFER_COLLATERAL_DATA_LEN;
        pub const MIN_ACCOUNT_COUNT: usize = TRANSFER_COLLATERAL_MIN_ACCOUNT_COUNT;

        impl_phoenix_invoke_methods!(
            TransferCollateral,
            TransferCollateralArgs,
            write_transfer_collateral_data
        );

        pub fn account_count(&self) -> usize {
            Self::MIN_ACCOUNT_COUNT
                + self.global_trader_index.len()
                + self.active_trader_buffer.len()
                + usize::from(self.permission_account.is_some())
        }

        fn write_accounts(&self, buffers: &mut CpiBuffers<'a, '_>) -> Result<usize, ProgramError> {
            let account_count = self.account_count();
            require_accounts(buffers, account_count)?;
            require_trader_arenas(self.global_trader_index, self.active_trader_buffer)?;

            write_account(buffers, 0, self.phoenix_program, false, false);
            write_account(buffers, 1, self.log_authority, false, false);
            write_account(buffers, 2, self.global_config, false, false);
            write_account(buffers, 3, self.trader, false, true);
            write_account(buffers, 4, self.src_trader_account, true, false);
            write_account(buffers, 5, self.dst_trader_account, true, false);
            write_account(buffers, 6, self.perp_asset_map, false, false);
            let mut index = 7;
            for account in self.global_trader_index {
                write_account(buffers, index, account, true, false);
                index += 1;
            }
            for account in self.active_trader_buffer {
                write_account(buffers, index, account, true, false);
                index += 1;
            }
            if let Some(permission_account) = self.permission_account {
                write_account(buffers, index, permission_account, true, false);
            }
            Ok(account_count)
        }
    }

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub struct TransferCollateralChildToParentArgs;

    pub struct TransferCollateralChildToParent<'a> {
        pub phoenix_program: &'a AccountInfo,
        pub log_authority: &'a AccountInfo,
        pub global_config: &'a AccountInfo,
        pub trader: &'a AccountInfo,
        pub child_trader_account: &'a AccountInfo,
        pub parent_trader_account: &'a AccountInfo,
        pub perp_asset_map: &'a AccountInfo,
        pub global_trader_index: &'a [AccountInfo],
        pub active_trader_buffer: &'a [AccountInfo],
    }

    impl<'a> TransferCollateralChildToParent<'a> {
        pub const DATA_LEN: usize = DISCRIMINANT_DATA_LEN;
        pub const MIN_ACCOUNT_COUNT: usize = TRANSFER_COLLATERAL_CHILD_TO_PARENT_MIN_ACCOUNT_COUNT;

        impl_phoenix_invoke_methods!(
            TransferCollateralChildToParent,
            TransferCollateralChildToParentArgs,
            write_transfer_collateral_child_to_parent_data
        );

        pub fn account_count(&self) -> usize {
            Self::MIN_ACCOUNT_COUNT
                + self.global_trader_index.len()
                + self.active_trader_buffer.len()
        }

        fn write_accounts(&self, buffers: &mut CpiBuffers<'a, '_>) -> Result<usize, ProgramError> {
            let account_count = self.account_count();
            require_accounts(buffers, account_count)?;
            require_trader_arenas(self.global_trader_index, self.active_trader_buffer)?;

            write_account(buffers, 0, self.phoenix_program, false, false);
            write_account(buffers, 1, self.log_authority, false, false);
            write_account(buffers, 2, self.global_config, false, false);
            write_account(buffers, 3, self.trader, false, true);
            write_account(buffers, 4, self.child_trader_account, true, false);
            write_account(buffers, 5, self.parent_trader_account, true, false);
            write_account(buffers, 6, self.perp_asset_map, false, false);
            let mut index = 7;
            for account in self.global_trader_index {
                write_account(buffers, index, account, true, false);
                index += 1;
            }
            for account in self.active_trader_buffer {
                write_account(buffers, index, account, true, false);
                index += 1;
            }
            Ok(account_count)
        }
    }

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub struct SyncParentToChildArgs;

    pub struct SyncParentToChild<'a> {
        pub phoenix_program: &'a AccountInfo,
        pub log_authority: &'a AccountInfo,
        pub global_config: &'a AccountInfo,
        pub trader_wallet: &'a AccountInfo,
        pub parent_trader_account: &'a AccountInfo,
        pub child_trader_account: &'a AccountInfo,
        pub global_trader_index: &'a [AccountInfo],
    }

    impl<'a> SyncParentToChild<'a> {
        pub const DATA_LEN: usize = DISCRIMINANT_DATA_LEN;
        pub const MIN_ACCOUNT_COUNT: usize = SYNC_PARENT_TO_CHILD_MIN_ACCOUNT_COUNT;

        impl_phoenix_invoke_methods!(
            SyncParentToChild,
            SyncParentToChildArgs,
            write_sync_parent_to_child_data
        );

        pub fn account_count(&self) -> usize {
            Self::MIN_ACCOUNT_COUNT + self.global_trader_index.len()
        }

        fn write_accounts(&self, buffers: &mut CpiBuffers<'a, '_>) -> Result<usize, ProgramError> {
            let account_count = self.account_count();
            require_accounts(buffers, account_count)?;
            if self.global_trader_index.is_empty() {
                return Err(ProgramError::NotEnoughAccountKeys);
            }

            write_account(buffers, 0, self.phoenix_program, false, false);
            write_account(buffers, 1, self.log_authority, false, false);
            write_account(buffers, 2, self.global_config, false, false);
            write_account(buffers, 3, self.trader_wallet, false, true);
            write_account(buffers, 4, self.parent_trader_account, false, false);
            write_account(buffers, 5, self.child_trader_account, true, false);
            for (offset, account) in self.global_trader_index.iter().enumerate() {
                write_account(buffers, 6 + offset, account, true, false);
            }
            Ok(account_count)
        }
    }

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub struct UpdateTraderStateArgs;

    pub struct UpdateTraderState<'a> {
        pub phoenix_program: &'a AccountInfo,
        pub log_authority: &'a AccountInfo,
        pub global_config: &'a AccountInfo,
        pub trader_account: &'a AccountInfo,
        pub perp_asset_map: &'a AccountInfo,
        pub global_trader_index: &'a [AccountInfo],
        pub active_trader_buffer: &'a [AccountInfo],
    }

    impl<'a> UpdateTraderState<'a> {
        pub const DATA_LEN: usize = DISCRIMINANT_DATA_LEN;
        pub const MIN_ACCOUNT_COUNT: usize = UPDATE_TRADER_STATE_MIN_ACCOUNT_COUNT;

        impl_phoenix_invoke_methods!(
            UpdateTraderState,
            UpdateTraderStateArgs,
            write_update_trader_state_data
        );

        pub fn account_count(&self) -> usize {
            Self::MIN_ACCOUNT_COUNT
                + self.global_trader_index.len()
                + self.active_trader_buffer.len()
        }

        fn write_accounts(&self, buffers: &mut CpiBuffers<'a, '_>) -> Result<usize, ProgramError> {
            let account_count = self.account_count();
            require_accounts(buffers, account_count)?;
            require_trader_arenas(self.global_trader_index, self.active_trader_buffer)?;

            write_account(buffers, 0, self.phoenix_program, false, false);
            write_account(buffers, 1, self.log_authority, false, false);
            write_account(buffers, 2, self.global_config, false, false);
            write_account(buffers, 3, self.trader_account, true, false);
            write_account(buffers, 4, self.perp_asset_map, false, false);
            let mut index = 5;
            for account in self.global_trader_index {
                write_account(buffers, index, account, true, false);
                index += 1;
            }
            for account in self.active_trader_buffer {
                write_account(buffers, index, account, true, false);
                index += 1;
            }
            Ok(account_count)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PlaceStopLossArgs {
        pub trigger_price: u64,
        pub execution_price: u64,
        pub trade_side: Side,
        pub execution_direction: Direction,
        pub order_kind: StopLossOrderKind,
    }

    pub struct PlaceStopLoss<'a> {
        pub phoenix_program: &'a AccountInfo,
        pub log_authority: &'a AccountInfo,
        pub global_config: &'a AccountInfo,
        pub funder: &'a AccountInfo,
        pub trader_account: &'a AccountInfo,
        pub perp_asset_map: &'a AccountInfo,
        pub global_trader_index: &'a [AccountInfo],
        pub active_trader_buffer: &'a [AccountInfo],
        pub orderbook: &'a AccountInfo,
        pub spline_collection: &'a AccountInfo,
        pub position_authority: &'a AccountInfo,
        pub stop_loss_account: &'a AccountInfo,
        pub system_program: &'a AccountInfo,
    }

    impl<'a> PlaceStopLoss<'a> {
        pub const DATA_LEN: usize = PLACE_STOP_LOSS_DATA_LEN;
        pub const MIN_ACCOUNT_COUNT: usize = PLACE_STOP_LOSS_MIN_ACCOUNT_COUNT;

        impl_phoenix_invoke_methods!(PlaceStopLoss, PlaceStopLossArgs, write_place_stop_loss_data);

        pub fn account_count(&self) -> usize {
            Self::MIN_ACCOUNT_COUNT
                + self.global_trader_index.len()
                + self.active_trader_buffer.len()
        }

        fn write_accounts(&self, buffers: &mut CpiBuffers<'a, '_>) -> Result<usize, ProgramError> {
            let account_count = self.account_count();
            require_accounts(buffers, account_count)?;
            require_trader_arenas(self.global_trader_index, self.active_trader_buffer)?;

            write_account(buffers, 0, self.phoenix_program, false, false);
            write_account(buffers, 1, self.log_authority, false, false);
            write_account(buffers, 2, self.global_config, false, false);
            write_account(buffers, 3, self.funder, true, true);
            write_account(buffers, 4, self.trader_account, true, false);
            write_account(buffers, 5, self.perp_asset_map, true, false);
            let mut index = 6;
            for account in self.global_trader_index {
                write_account(buffers, index, account, true, false);
                index += 1;
            }
            for account in self.active_trader_buffer {
                write_account(buffers, index, account, true, false);
                index += 1;
            }
            write_account(buffers, index, self.orderbook, true, false);
            index += 1;
            write_account(buffers, index, self.spline_collection, true, false);
            index += 1;
            write_account(buffers, index, self.position_authority, false, true);
            index += 1;
            write_account(buffers, index, self.stop_loss_account, true, false);
            index += 1;
            write_account(buffers, index, self.system_program, false, false);
            Ok(account_count)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CancelStopLossArgs {
        pub execution_direction: Direction,
    }

    pub struct CancelStopLoss<'a> {
        pub phoenix_program: &'a AccountInfo,
        pub log_authority: &'a AccountInfo,
        pub global_config: &'a AccountInfo,
        pub funder: &'a AccountInfo,
        pub trader_account: &'a AccountInfo,
        pub position_authority: &'a AccountInfo,
        pub stop_loss_account: &'a AccountInfo,
        pub system_program: &'a AccountInfo,
    }

    impl<'a> CancelStopLoss<'a> {
        pub const ACCOUNT_COUNT: usize = CANCEL_STOP_LOSS_ACCOUNT_COUNT;
        pub const DATA_LEN: usize = CANCEL_STOP_LOSS_DATA_LEN;

        impl_phoenix_invoke_methods!(
            CancelStopLoss,
            CancelStopLossArgs,
            write_cancel_stop_loss_data
        );

        fn write_accounts(&self, buffers: &mut CpiBuffers<'a, '_>) -> Result<usize, ProgramError> {
            require_accounts(buffers, Self::ACCOUNT_COUNT)?;
            write_account(buffers, 0, self.phoenix_program, false, false);
            write_account(buffers, 1, self.log_authority, false, false);
            write_account(buffers, 2, self.global_config, false, false);
            write_account(buffers, 3, self.funder, true, false);
            write_account(buffers, 4, self.trader_account, false, false);
            write_account(buffers, 5, self.position_authority, false, true);
            write_account(buffers, 6, self.stop_loss_account, true, false);
            write_account(buffers, 7, self.system_program, false, false);
            Ok(Self::ACCOUNT_COUNT)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CreateConditionalOrdersAccountArgs {
        pub capacity: u8,
    }

    pub struct CreateConditionalOrdersAccount<'a> {
        pub phoenix_program: &'a AccountInfo,
        pub log_authority: &'a AccountInfo,
        pub global_config: &'a AccountInfo,
        pub payer: &'a AccountInfo,
        pub trader_wallet: &'a AccountInfo,
        pub trader_account: &'a AccountInfo,
        pub trader_conditional_orders: &'a AccountInfo,
        pub system_program: &'a AccountInfo,
    }

    impl<'a> CreateConditionalOrdersAccount<'a> {
        pub const ACCOUNT_COUNT: usize = CREATE_CONDITIONAL_ORDERS_ACCOUNT_COUNT;
        pub const DATA_LEN: usize = CREATE_CONDITIONAL_ORDERS_ACCOUNT_DATA_LEN;

        impl_phoenix_invoke_methods!(
            CreateConditionalOrdersAccount,
            CreateConditionalOrdersAccountArgs,
            write_create_conditional_orders_account_data
        );

        fn write_accounts(&self, buffers: &mut CpiBuffers<'a, '_>) -> Result<usize, ProgramError> {
            require_accounts(buffers, Self::ACCOUNT_COUNT)?;
            write_account(buffers, 0, self.phoenix_program, false, false);
            write_account(buffers, 1, self.log_authority, false, false);
            write_account(buffers, 2, self.global_config, false, false);
            write_account(buffers, 3, self.payer, true, true);
            write_account(buffers, 4, self.trader_wallet, false, false);
            write_account(buffers, 5, self.trader_account, false, false);
            write_account(buffers, 6, self.trader_conditional_orders, true, false);
            write_account(buffers, 7, self.system_program, false, false);
            Ok(Self::ACCOUNT_COUNT)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PlacePositionConditionalOrderArgs {
        pub asset_id: u32,
        pub greater_trigger_order: Option<TriggerOrderParams>,
        pub less_trigger_order: Option<TriggerOrderParams>,
        pub size_base_lots: Option<u64>,
        pub size_percent: Option<u8>,
    }

    pub struct PlacePositionConditionalOrder<'a> {
        pub phoenix_program: &'a AccountInfo,
        pub log_authority: &'a AccountInfo,
        pub global_config: &'a AccountInfo,
        pub payer: &'a AccountInfo,
        pub trader_account: &'a AccountInfo,
        pub perp_asset_map: &'a AccountInfo,
        pub global_trader_index: &'a [AccountInfo],
        pub active_trader_buffer: &'a [AccountInfo],
        pub orderbook: &'a AccountInfo,
        pub spline_collection: &'a AccountInfo,
        pub trader_wallet: &'a AccountInfo,
        pub trader_conditional_orders: &'a AccountInfo,
        pub system_program: &'a AccountInfo,
    }

    impl<'a> PlacePositionConditionalOrder<'a> {
        pub const MAX_DATA_LEN: usize = PLACE_POSITION_CONDITIONAL_ORDER_MAX_DATA_LEN;
        pub const MIN_ACCOUNT_COUNT: usize = PLACE_POSITION_CONDITIONAL_ORDER_MIN_ACCOUNT_COUNT;

        impl_phoenix_invoke_methods!(
            PlacePositionConditionalOrder,
            PlacePositionConditionalOrderArgs,
            write_place_position_conditional_order_data
        );

        pub fn account_count(&self) -> usize {
            Self::MIN_ACCOUNT_COUNT
                + self.global_trader_index.len()
                + self.active_trader_buffer.len()
        }

        fn write_accounts(&self, buffers: &mut CpiBuffers<'a, '_>) -> Result<usize, ProgramError> {
            let account_count = self.account_count();
            require_accounts(buffers, account_count)?;
            require_trader_arenas(self.global_trader_index, self.active_trader_buffer)?;

            write_account(buffers, 0, self.phoenix_program, false, false);
            write_account(buffers, 1, self.log_authority, false, false);
            write_account(buffers, 2, self.global_config, false, false);
            write_account(buffers, 3, self.payer, true, true);
            write_account(buffers, 4, self.trader_account, true, false);
            write_account(buffers, 5, self.perp_asset_map, true, false);
            let mut index = 6;
            for account in self.global_trader_index {
                write_account(buffers, index, account, true, false);
                index += 1;
            }
            for account in self.active_trader_buffer {
                write_account(buffers, index, account, true, false);
                index += 1;
            }
            write_account(buffers, index, self.orderbook, true, false);
            index += 1;
            write_account(buffers, index, self.spline_collection, true, false);
            index += 1;
            write_account(buffers, index, self.trader_wallet, false, true);
            index += 1;
            write_account(buffers, index, self.trader_conditional_orders, true, false);
            index += 1;
            write_account(buffers, index, self.system_program, false, false);
            Ok(account_count)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CancelConditionalOrderArgs {
        pub conditional_order_index: u8,
        pub disable_first: bool,
        pub disable_second: bool,
    }

    pub struct CancelConditionalOrder<'a> {
        pub phoenix_program: &'a AccountInfo,
        pub log_authority: &'a AccountInfo,
        pub global_config: &'a AccountInfo,
        pub trader_account: &'a AccountInfo,
        pub trader_wallet: &'a AccountInfo,
        pub orderbook: &'a AccountInfo,
        pub trader_conditional_orders: &'a AccountInfo,
    }

    impl<'a> CancelConditionalOrder<'a> {
        pub const ACCOUNT_COUNT: usize = CANCEL_CONDITIONAL_ORDER_ACCOUNT_COUNT;
        pub const DATA_LEN: usize = CANCEL_CONDITIONAL_ORDER_DATA_LEN;

        impl_phoenix_invoke_methods!(
            CancelConditionalOrder,
            CancelConditionalOrderArgs,
            write_cancel_conditional_order_data
        );

        fn write_accounts(&self, buffers: &mut CpiBuffers<'a, '_>) -> Result<usize, ProgramError> {
            require_accounts(buffers, Self::ACCOUNT_COUNT)?;
            write_account(buffers, 0, self.phoenix_program, false, false);
            write_account(buffers, 1, self.log_authority, false, false);
            write_account(buffers, 2, self.global_config, false, false);
            write_account(buffers, 3, self.trader_account, true, false);
            write_account(buffers, 4, self.trader_wallet, false, true);
            write_account(buffers, 5, self.orderbook, true, false);
            write_account(buffers, 6, self.trader_conditional_orders, true, false);
            Ok(Self::ACCOUNT_COUNT)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PlaceAttachedConditionalOrderArgs {
        pub order_id: FifoOrderId,
        pub asset_id: u32,
        pub greater_trigger_order: Option<TriggerOrderParams>,
        pub less_trigger_order: Option<TriggerOrderParams>,
    }

    pub struct PlaceAttachedConditionalOrder<'a> {
        pub phoenix_program: &'a AccountInfo,
        pub log_authority: &'a AccountInfo,
        pub global_config: &'a AccountInfo,
        pub trader_account: &'a AccountInfo,
        pub trader_wallet: &'a AccountInfo,
        pub orderbook: &'a AccountInfo,
        pub trader_conditional_orders: &'a AccountInfo,
        pub payer: &'a AccountInfo,
        pub global_trader_index: &'a [AccountInfo],
        pub active_trader_buffer: &'a [AccountInfo],
        pub system_program: &'a AccountInfo,
    }

    impl<'a> PlaceAttachedConditionalOrder<'a> {
        pub const MAX_DATA_LEN: usize = PLACE_ATTACHED_CONDITIONAL_ORDER_MAX_DATA_LEN;
        pub const MIN_ACCOUNT_COUNT: usize = PLACE_ATTACHED_CONDITIONAL_ORDER_MIN_ACCOUNT_COUNT;

        impl_phoenix_invoke_methods!(
            PlaceAttachedConditionalOrder,
            PlaceAttachedConditionalOrderArgs,
            write_place_attached_conditional_order_data
        );

        pub fn account_count(&self) -> usize {
            Self::MIN_ACCOUNT_COUNT
                + self.global_trader_index.len()
                + self.active_trader_buffer.len()
        }

        fn write_accounts(&self, buffers: &mut CpiBuffers<'a, '_>) -> Result<usize, ProgramError> {
            let account_count = self.account_count();
            require_accounts(buffers, account_count)?;
            require_trader_arenas(self.global_trader_index, self.active_trader_buffer)?;

            write_account(buffers, 0, self.phoenix_program, false, false);
            write_account(buffers, 1, self.log_authority, false, false);
            write_account(buffers, 2, self.global_config, false, false);
            write_account(buffers, 3, self.trader_account, true, false);
            write_account(buffers, 4, self.trader_wallet, false, true);
            write_account(buffers, 5, self.orderbook, true, false);
            write_account(buffers, 6, self.trader_conditional_orders, true, false);
            write_account(buffers, 7, self.payer, true, true);
            let mut index = 8;
            for account in self.global_trader_index {
                write_account(buffers, index, account, true, false);
                index += 1;
            }
            for account in self.active_trader_buffer {
                write_account(buffers, index, account, true, false);
                index += 1;
            }
            write_account(buffers, index, self.system_program, false, false);
            Ok(account_count)
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub struct PlaceLimitOrderWithConditionalsArgs<'b> {
        pub order_packet: &'b OrderPacket,
        pub slot: u64,
        pub greater_trigger_order: Option<TriggerOrderParams>,
        pub less_trigger_order: Option<TriggerOrderParams>,
    }

    pub struct PlaceLimitOrderWithConditionals<'a> {
        pub phoenix_program: &'a AccountInfo,
        pub log_authority: &'a AccountInfo,
        pub global_config: &'a AccountInfo,
        pub trader_wallet: &'a AccountInfo,
        pub trader_account: &'a AccountInfo,
        pub perp_asset_map: &'a AccountInfo,
        pub global_trader_index: &'a [AccountInfo],
        pub active_trader_buffer: &'a [AccountInfo],
        pub orderbook: &'a AccountInfo,
        pub spline_collection: &'a AccountInfo,
        pub payer: &'a AccountInfo,
        pub trader_conditional_orders: &'a AccountInfo,
        pub system_program: &'a AccountInfo,
    }

    impl<'a> PlaceLimitOrderWithConditionals<'a> {
        pub const MAX_DATA_LEN: usize = PLACE_LIMIT_ORDER_WITH_CONDITIONALS_MAX_DATA_LEN;
        pub const MIN_ACCOUNT_COUNT: usize = PLACE_LIMIT_ORDER_WITH_CONDITIONALS_MIN_ACCOUNT_COUNT;

        impl_phoenix_invoke_methods!(
            PlaceLimitOrderWithConditionals,
            PlaceLimitOrderWithConditionalsArgs<'_>,
            write_place_limit_order_with_conditionals_data
        );

        pub fn account_count(&self) -> usize {
            Self::MIN_ACCOUNT_COUNT
                + self.global_trader_index.len()
                + self.active_trader_buffer.len()
        }

        fn write_accounts(&self, buffers: &mut CpiBuffers<'a, '_>) -> Result<usize, ProgramError> {
            let account_count = self.account_count();
            require_accounts(buffers, account_count)?;
            require_trader_arenas(self.global_trader_index, self.active_trader_buffer)?;

            write_account(buffers, 0, self.phoenix_program, false, false);
            write_account(buffers, 1, self.log_authority, false, false);
            write_account(buffers, 2, self.global_config, true, false);
            write_account(buffers, 3, self.trader_wallet, false, true);
            write_account(buffers, 4, self.trader_account, true, false);
            write_account(buffers, 5, self.perp_asset_map, true, false);
            let mut index = 6;
            for account in self.global_trader_index {
                write_account(buffers, index, account, true, false);
                index += 1;
            }
            for account in self.active_trader_buffer {
                write_account(buffers, index, account, true, false);
                index += 1;
            }
            write_account(buffers, index, self.orderbook, true, false);
            index += 1;
            write_account(buffers, index, self.spline_collection, true, false);
            index += 1;
            write_account(buffers, index, self.payer, true, true);
            index += 1;
            write_account(buffers, index, self.trader_conditional_orders, true, false);
            index += 1;
            write_account(buffers, index, self.system_program, false, false);
            Ok(account_count)
        }
    }

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub struct CancelAllPlusConditionalArgs;

    pub struct CancelAllPlusConditional<'a> {
        pub phoenix_program: &'a AccountInfo,
        pub log_authority: &'a AccountInfo,
        pub global_config: &'a AccountInfo,
        pub trader: &'a AccountInfo,
        pub trader_account: &'a AccountInfo,
        pub perp_asset_map: &'a AccountInfo,
        pub global_trader_index: &'a [AccountInfo],
        pub active_trader_buffer: &'a [AccountInfo],
        pub orderbook: &'a AccountInfo,
        pub spline_collection: &'a AccountInfo,
        pub trader_conditional_orders: &'a AccountInfo,
    }

    impl<'a> CancelAllPlusConditional<'a> {
        pub const DATA_LEN: usize = DISCRIMINANT_DATA_LEN;
        pub const MIN_ACCOUNT_COUNT: usize = CANCEL_ALL_PLUS_CONDITIONAL_MIN_ACCOUNT_COUNT;

        impl_phoenix_invoke_methods!(
            CancelAllPlusConditional,
            CancelAllPlusConditionalArgs,
            write_cancel_all_plus_conditional_data
        );

        pub fn account_count(&self) -> usize {
            Self::MIN_ACCOUNT_COUNT
                + self.global_trader_index.len()
                + self.active_trader_buffer.len()
        }

        fn write_accounts(&self, buffers: &mut CpiBuffers<'a, '_>) -> Result<usize, ProgramError> {
            let market_count = write_market_action_accounts(
                buffers,
                MarketActionAccountRefs {
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
                },
            )?;
            let account_count = market_count + 1;
            require_accounts(buffers, account_count)?;
            write_account(
                buffers,
                market_count,
                self.trader_conditional_orders,
                true,
                false,
            );
            Ok(account_count)
        }
    }

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub struct CancelAllArgs;

    pub struct CancelAll<'a> {
        pub phoenix_program: &'a AccountInfo,
        pub log_authority: &'a AccountInfo,
        pub global_config: &'a AccountInfo,
        pub trader: &'a AccountInfo,
        pub trader_account: &'a AccountInfo,
        pub perp_asset_map: &'a AccountInfo,
        pub global_trader_index: &'a [AccountInfo],
        pub active_trader_buffer: &'a [AccountInfo],
        pub orderbook: &'a AccountInfo,
        pub spline_collection: &'a AccountInfo,
    }

    impl<'a> CancelAll<'a> {
        pub const DATA_LEN: usize = CANCEL_ALL_DATA_LEN;
        pub const MIN_ACCOUNT_COUNT: usize = CANCEL_ORDER_MIN_ACCOUNT_COUNT;

        impl_phoenix_invoke_methods!(CancelAll, CancelAllArgs, write_cancel_all_data);

        pub fn account_count(&self) -> usize {
            Self::MIN_ACCOUNT_COUNT
                + self.global_trader_index.len()
                + self.active_trader_buffer.len()
        }

        fn write_accounts(&self, buffers: &mut CpiBuffers<'a, '_>) -> Result<usize, ProgramError> {
            write_market_action_accounts(
                buffers,
                MarketActionAccountRefs {
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
                },
            )
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CancelUpToArgs {
        pub side: Side,
        pub num_orders_to_cancel: Option<u64>,
        pub tick_limit: Option<u64>,
    }

    pub struct CancelUpTo<'a> {
        pub phoenix_program: &'a AccountInfo,
        pub log_authority: &'a AccountInfo,
        pub global_config: &'a AccountInfo,
        pub trader: &'a AccountInfo,
        pub trader_account: &'a AccountInfo,
        pub perp_asset_map: &'a AccountInfo,
        pub global_trader_index: &'a [AccountInfo],
        pub active_trader_buffer: &'a [AccountInfo],
        pub orderbook: &'a AccountInfo,
        pub spline_collection: &'a AccountInfo,
    }

    impl<'a> CancelUpTo<'a> {
        pub const MAX_DATA_LEN: usize = CANCEL_UP_TO_MAX_DATA_LEN;
        pub const MIN_ACCOUNT_COUNT: usize = CANCEL_ORDER_MIN_ACCOUNT_COUNT;

        impl_phoenix_invoke_methods!(CancelUpTo, CancelUpToArgs, write_cancel_up_to_data);

        pub fn account_count(&self) -> usize {
            Self::MIN_ACCOUNT_COUNT
                + self.global_trader_index.len()
                + self.active_trader_buffer.len()
        }

        fn write_accounts(&self, buffers: &mut CpiBuffers<'a, '_>) -> Result<usize, ProgramError> {
            write_market_action_accounts(
                buffers,
                MarketActionAccountRefs {
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
                },
            )
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CancelOrdersByIdArgs<'b> {
        pub order_ids: &'b [CancelId],
    }

    pub struct CancelOrdersById<'a> {
        pub phoenix_program: &'a AccountInfo,
        pub log_authority: &'a AccountInfo,
        pub global_config: &'a AccountInfo,
        pub trader: &'a AccountInfo,
        pub trader_account: &'a AccountInfo,
        pub perp_asset_map: &'a AccountInfo,
        pub global_trader_index: &'a [AccountInfo],
        pub active_trader_buffer: &'a [AccountInfo],
        pub orderbook: &'a AccountInfo,
        pub spline_collection: &'a AccountInfo,
    }

    impl<'a> CancelOrdersById<'a> {
        pub const MAX_DATA_LEN: usize = CANCEL_ORDERS_BY_ID_MAX_DATA_LEN;
        pub const MIN_ACCOUNT_COUNT: usize = CANCEL_ORDER_MIN_ACCOUNT_COUNT;

        impl_phoenix_invoke_methods!(
            CancelOrdersById,
            CancelOrdersByIdArgs<'_>,
            write_cancel_orders_by_id_data
        );

        pub fn account_count(&self) -> usize {
            Self::MIN_ACCOUNT_COUNT
                + self.global_trader_index.len()
                + self.active_trader_buffer.len()
        }

        fn write_accounts(&self, buffers: &mut CpiBuffers<'a, '_>) -> Result<usize, ProgramError> {
            write_market_action_accounts(
                buffers,
                MarketActionAccountRefs {
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
                },
            )
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub struct PlaceMultiLimitOrderArgs<'b> {
        pub bids: &'b [CondensedOrder],
        pub asks: &'b [CondensedOrder],
        pub client_order_id: Option<u128>,
        pub slide: bool,
    }

    pub struct PlaceMultiLimitOrder<'a> {
        pub phoenix_program: &'a AccountInfo,
        pub log_authority: &'a AccountInfo,
        pub global_config: &'a AccountInfo,
        pub trader: &'a AccountInfo,
        pub trader_account: &'a AccountInfo,
        pub perp_asset_map: &'a AccountInfo,
        pub global_trader_index: &'a [AccountInfo],
        pub active_trader_buffer: &'a [AccountInfo],
        pub orderbook: &'a AccountInfo,
        pub spline_collection: &'a AccountInfo,
    }

    impl<'a> PlaceMultiLimitOrder<'a> {
        pub const MIN_ACCOUNT_COUNT: usize = PLACE_MULTI_LIMIT_ORDER_MIN_ACCOUNT_COUNT;

        impl_phoenix_invoke_methods!(
            PlaceMultiLimitOrder,
            PlaceMultiLimitOrderArgs<'_>,
            write_place_multi_limit_order_data
        );

        pub fn account_count(&self) -> usize {
            Self::MIN_ACCOUNT_COUNT
                + self.global_trader_index.len()
                + self.active_trader_buffer.len()
        }

        fn write_accounts(&self, buffers: &mut CpiBuffers<'a, '_>) -> Result<usize, ProgramError> {
            write_market_action_accounts(
                buffers,
                MarketActionAccountRefs {
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
                },
            )
        }
    }

    pub(crate) fn write_register_trader_data(
        args: RegisterTraderArgs,
        out: &mut [u8],
    ) -> Result<usize, crate::PhoenixIxError> {
        if args.max_positions == 0 || args.max_positions > 128 {
            return Err(crate::PhoenixIxError::MissingField(
                "max_positions must be between 1 and 128",
            ));
        }
        if args.subaccount_index > 100 {
            return Err(crate::PhoenixIxError::InvalidSubaccountIndex);
        }
        require_data(out, REGISTER_TRADER_DATA_LEN)?;
        let mut offset = 0;
        write_bytes(
            out,
            &mut offset,
            &PhoenixInstruction::RegisterTrader.discriminant(),
        )?;
        write_u32(out, &mut offset, args.max_positions as u32)?;
        write_u32(out, &mut offset, args.trader_preference_bits)?;
        write_u8(out, &mut offset, args.trader_pda_index)?;
        write_u8(out, &mut offset, args.subaccount_index)?;
        Ok(offset)
    }

    pub(crate) fn write_set_trader_capabilities_delegated_data(
        out: &mut [u8],
    ) -> Result<usize, crate::PhoenixIxError> {
        require_data(out, SET_TRADER_CAPABILITIES_DELEGATED_DATA_LEN)?;
        let mut offset = 0;
        write_bytes(
            out,
            &mut offset,
            &PhoenixInstruction::SetTraderCapabilitiesDelegated.discriminant(),
        )?;
        write_bytes(out, &mut offset, &ALL_TRADER_CAPABILITY_UPDATE_BYTES)?;
        Ok(offset)
    }

    fn ioc_order_data_len(args: PlaceMarketOrderArgs) -> usize {
        8 + 1
            + 1
            + option_u64_len(args.price_in_ticks)
            + 8
            + option_u64_len(args.num_quote_lots)
            + 8
            + 8
            + 1
            + option_u64_len(args.match_limit)
            + 16
            + option_u64_len(args.last_valid_slot)
            + 1
            + 1
    }

    fn limit_order_data_len(args: PlaceLimitOrderArgs) -> usize {
        8 + 1
            + 1
            + 8
            + 8
            + 1
            + option_u64_len(args.match_limit)
            + 16
            + option_u64_len(args.last_valid_slot)
            + 1
            + 1
    }

    const fn option_u64_len(value: Option<u64>) -> usize {
        match value {
            Some(_) => 9,
            None => 1,
        }
    }

    const fn option_u8_len(value: Option<u8>) -> usize {
        match value {
            Some(_) => 2,
            None => 1,
        }
    }

    const fn option_trigger_len(value: Option<TriggerOrderParams>) -> usize {
        match value {
            Some(_) => 20,
            None => 1,
        }
    }

    fn write_ioc_order_data(
        discriminant: [u8; 8],
        args: PlaceMarketOrderArgs,
        out: &mut [u8],
    ) -> Result<usize, crate::PhoenixIxError> {
        let expected = ioc_order_data_len(args);
        require_data(out, expected)?;
        let mut offset = 0;
        write_bytes(out, &mut offset, &discriminant)?;
        write_u8(out, &mut offset, 2)?;
        write_u8(out, &mut offset, args.side as u8)?;
        write_option_u64(out, &mut offset, args.price_in_ticks)?;
        write_u64(out, &mut offset, args.num_base_lots)?;
        write_option_u64(out, &mut offset, args.num_quote_lots)?;
        write_u64(out, &mut offset, args.min_base_lots_to_fill)?;
        write_u64(out, &mut offset, args.min_quote_lots_to_fill)?;
        write_u8(out, &mut offset, args.self_trade_behavior as u8)?;
        write_option_u64(out, &mut offset, args.match_limit)?;
        write_bytes(
            out,
            &mut offset,
            &client_order_id_to_bytes(args.client_order_id),
        )?;
        write_option_u64(out, &mut offset, args.last_valid_slot)?;
        write_u8(out, &mut offset, args.order_flags as u8)?;
        write_bool(out, &mut offset, args.cancel_existing)?;
        Ok(offset)
    }

    pub(crate) fn write_place_market_order_data(
        args: PlaceMarketOrderArgs,
        out: &mut [u8],
    ) -> Result<usize, crate::PhoenixIxError> {
        write_ioc_order_data(
            PhoenixInstruction::PlaceMarketOrder.discriminant(),
            args,
            out,
        )
    }

    pub(crate) fn write_place_market_order_delegated_data(
        args: PlaceMarketOrderDelegatedArgs,
        out: &mut [u8],
    ) -> Result<usize, crate::PhoenixIxError> {
        write_ioc_order_data(
            PhoenixInstruction::PlaceMarketOrderDelegated.discriminant(),
            args,
            out,
        )
    }

    pub(crate) fn write_place_limit_order_data(
        args: PlaceLimitOrderArgs,
        out: &mut [u8],
    ) -> Result<usize, crate::PhoenixIxError> {
        let expected = limit_order_data_len(args);
        require_data(out, expected)?;
        let mut offset = 0;
        write_bytes(
            out,
            &mut offset,
            &PhoenixInstruction::PlaceLimitOrder.discriminant(),
        )?;
        write_u8(out, &mut offset, 1)?;
        write_u8(out, &mut offset, args.side as u8)?;
        write_u64(out, &mut offset, args.price_in_ticks)?;
        write_u64(out, &mut offset, args.num_base_lots)?;
        write_u8(out, &mut offset, args.self_trade_behavior as u8)?;
        write_option_u64(out, &mut offset, args.match_limit)?;
        write_bytes(
            out,
            &mut offset,
            &client_order_id_to_bytes(args.client_order_id),
        )?;
        write_option_u64(out, &mut offset, args.last_valid_slot)?;
        write_u8(out, &mut offset, args.order_flags as u8)?;
        write_bool(out, &mut offset, args.cancel_existing)?;
        Ok(offset)
    }

    pub(crate) fn write_phoenix_deposit_data(
        args: PhoenixDepositArgs,
        out: &mut [u8],
    ) -> Result<usize, crate::PhoenixIxError> {
        if args.amount == 0 {
            return Err(crate::PhoenixIxError::InvalidDepositAmount);
        }
        require_data(out, PHOENIX_DEPOSIT_DATA_LEN)?;
        let mut offset = 0;
        write_bytes(
            out,
            &mut offset,
            &PhoenixInstruction::DepositFunds.discriminant(),
        )?;
        write_u64(out, &mut offset, args.amount)?;
        Ok(offset)
    }

    pub(crate) fn write_phoenix_withdraw_data(
        args: PhoenixWithdrawArgs,
        out: &mut [u8],
    ) -> Result<usize, crate::PhoenixIxError> {
        if args.amount == 0 {
            return Err(crate::PhoenixIxError::InvalidWithdrawAmount);
        }
        require_data(out, PHOENIX_WITHDRAW_DATA_LEN)?;
        let mut offset = 0;
        write_bytes(
            out,
            &mut offset,
            &PhoenixInstruction::WithdrawFunds.discriminant(),
        )?;
        write_u64(out, &mut offset, args.amount)?;
        Ok(offset)
    }

    pub(crate) fn write_transfer_collateral_data(
        args: TransferCollateralArgs,
        out: &mut [u8],
    ) -> Result<usize, crate::PhoenixIxError> {
        if args.amount == 0 {
            return Err(crate::PhoenixIxError::InvalidTransferAmount);
        }
        require_data(out, TRANSFER_COLLATERAL_DATA_LEN)?;
        let mut offset = 0;
        write_bytes(
            out,
            &mut offset,
            &PhoenixInstruction::TransferCollateral.discriminant(),
        )?;
        write_u64(out, &mut offset, args.amount)?;
        Ok(offset)
    }

    pub(crate) fn write_transfer_collateral_child_to_parent_data(
        _args: TransferCollateralChildToParentArgs,
        out: &mut [u8],
    ) -> Result<usize, crate::PhoenixIxError> {
        write_discriminant_data(
            PhoenixInstruction::TransferCollateralChildToParent.discriminant(),
            out,
        )
    }

    pub(crate) fn write_sync_parent_to_child_data(
        _args: SyncParentToChildArgs,
        out: &mut [u8],
    ) -> Result<usize, crate::PhoenixIxError> {
        write_discriminant_data(PhoenixInstruction::SyncParentToChild.discriminant(), out)
    }

    pub(crate) fn write_update_trader_state_data(
        _args: UpdateTraderStateArgs,
        out: &mut [u8],
    ) -> Result<usize, crate::PhoenixIxError> {
        write_discriminant_data(PhoenixInstruction::UpdateTraderState.discriminant(), out)
    }

    pub(crate) fn write_place_stop_loss_data(
        args: PlaceStopLossArgs,
        out: &mut [u8],
    ) -> Result<usize, crate::PhoenixIxError> {
        require_data(out, PLACE_STOP_LOSS_DATA_LEN)?;
        let mut offset = 0;
        write_bytes(
            out,
            &mut offset,
            &PhoenixInstruction::PlaceStopLoss.discriminant(),
        )?;
        write_u64(out, &mut offset, args.trigger_price)?;
        write_u64(out, &mut offset, args.execution_price)?;
        write_u64(out, &mut offset, 0)?;
        write_u8(out, &mut offset, args.trade_side as u8)?;
        write_u8(out, &mut offset, args.execution_direction as u8)?;
        write_u8(out, &mut offset, args.order_kind as u8)?;
        Ok(offset)
    }

    pub(crate) fn write_cancel_stop_loss_data(
        args: CancelStopLossArgs,
        out: &mut [u8],
    ) -> Result<usize, crate::PhoenixIxError> {
        require_data(out, CANCEL_STOP_LOSS_DATA_LEN)?;
        let mut offset = 0;
        write_bytes(
            out,
            &mut offset,
            &PhoenixInstruction::CancelStopLoss.discriminant(),
        )?;
        write_u8(out, &mut offset, args.execution_direction as u8)?;
        Ok(offset)
    }

    pub(crate) fn write_create_conditional_orders_account_data(
        args: CreateConditionalOrdersAccountArgs,
        out: &mut [u8],
    ) -> Result<usize, crate::PhoenixIxError> {
        if args.capacity == 0 {
            return Err(crate::PhoenixIxError::InvalidConditionalOrdersCapacity);
        }
        require_data(out, CREATE_CONDITIONAL_ORDERS_ACCOUNT_DATA_LEN)?;
        let mut offset = 0;
        write_bytes(
            out,
            &mut offset,
            &PhoenixInstruction::CreateConditionalOrdersAccount.discriminant(),
        )?;
        write_u8(out, &mut offset, args.capacity)?;
        Ok(offset)
    }

    pub(crate) fn write_place_position_conditional_order_data(
        args: PlacePositionConditionalOrderArgs,
        out: &mut [u8],
    ) -> Result<usize, crate::PhoenixIxError> {
        validate_conditional_triggers(args.greater_trigger_order, args.less_trigger_order)?;
        validate_conditional_sizing(args.size_base_lots, args.size_percent)?;
        let expected = 8
            + 4
            + option_trigger_len(args.greater_trigger_order)
            + option_trigger_len(args.less_trigger_order)
            + option_u64_len(args.size_base_lots)
            + option_u8_len(args.size_percent);
        require_data(out, expected)?;
        let mut offset = 0;
        write_bytes(
            out,
            &mut offset,
            &PhoenixInstruction::PlacePositionConditionalOrder.discriminant(),
        )?;
        write_u32(out, &mut offset, args.asset_id)?;
        write_option_trigger(out, &mut offset, args.greater_trigger_order)?;
        write_option_trigger(out, &mut offset, args.less_trigger_order)?;
        write_option_u64(out, &mut offset, args.size_base_lots)?;
        write_option_u8(out, &mut offset, args.size_percent)?;
        Ok(offset)
    }

    pub(crate) fn write_cancel_conditional_order_data(
        args: CancelConditionalOrderArgs,
        out: &mut [u8],
    ) -> Result<usize, crate::PhoenixIxError> {
        validate_conditional_order_index(args.conditional_order_index)?;
        if !args.disable_first && !args.disable_second {
            return Err(crate::PhoenixIxError::MissingConditionalOrderDisableFlag);
        }
        require_data(out, CANCEL_CONDITIONAL_ORDER_DATA_LEN)?;
        let mut offset = 0;
        write_bytes(
            out,
            &mut offset,
            &PhoenixInstruction::CancelConditionalOrder.discriminant(),
        )?;
        write_u8(out, &mut offset, args.conditional_order_index)?;
        write_bool(out, &mut offset, args.disable_first)?;
        write_bool(out, &mut offset, args.disable_second)?;
        Ok(offset)
    }

    pub(crate) fn write_place_attached_conditional_order_data(
        args: PlaceAttachedConditionalOrderArgs,
        out: &mut [u8],
    ) -> Result<usize, crate::PhoenixIxError> {
        validate_conditional_triggers(args.greater_trigger_order, args.less_trigger_order)?;
        let expected = 8
            + 16
            + 4
            + option_trigger_len(args.greater_trigger_order)
            + option_trigger_len(args.less_trigger_order);
        require_data(out, expected)?;
        let mut offset = 0;
        write_bytes(
            out,
            &mut offset,
            &PhoenixInstruction::PlaceAttachedConditionalOrder.discriminant(),
        )?;
        write_fifo_order_id(out, &mut offset, args.order_id)?;
        write_u32(out, &mut offset, args.asset_id)?;
        write_option_trigger(out, &mut offset, args.greater_trigger_order)?;
        write_option_trigger(out, &mut offset, args.less_trigger_order)?;
        Ok(offset)
    }

    pub(crate) fn write_place_limit_order_with_conditionals_data(
        args: PlaceLimitOrderWithConditionalsArgs<'_>,
        out: &mut [u8],
    ) -> Result<usize, crate::PhoenixIxError> {
        validate_conditional_triggers(args.greater_trigger_order, args.less_trigger_order)?;
        let mut offset = 0;
        write_bytes(
            out,
            &mut offset,
            &PhoenixInstruction::PlaceLimitOrderWithConditionals.discriminant(),
        )?;
        write_borsh(out, &mut offset, args.order_packet)?;
        write_u64(out, &mut offset, args.slot)?;
        write_option_trigger(out, &mut offset, args.greater_trigger_order)?;
        write_option_trigger(out, &mut offset, args.less_trigger_order)?;
        Ok(offset)
    }

    pub(crate) fn write_cancel_all_plus_conditional_data(
        _args: CancelAllPlusConditionalArgs,
        out: &mut [u8],
    ) -> Result<usize, crate::PhoenixIxError> {
        write_discriminant_data(
            PhoenixInstruction::CancelAllPlusConditional.discriminant(),
            out,
        )
    }

    pub(crate) fn write_cancel_all_data(
        _args: CancelAllArgs,
        out: &mut [u8],
    ) -> Result<usize, crate::PhoenixIxError> {
        write_discriminant_data(PhoenixInstruction::CancelAll.discriminant(), out)
    }

    pub(crate) fn write_cancel_up_to_data(
        args: CancelUpToArgs,
        out: &mut [u8],
    ) -> Result<usize, crate::PhoenixIxError> {
        let expected =
            8 + 1 + option_u64_len(args.num_orders_to_cancel) + option_u64_len(args.tick_limit);
        require_data(out, expected)?;
        let mut offset = 0;
        write_bytes(
            out,
            &mut offset,
            &PhoenixInstruction::CancelUpTo.discriminant(),
        )?;
        write_u8(out, &mut offset, args.side as u8)?;
        write_option_u64(out, &mut offset, args.num_orders_to_cancel)?;
        write_option_u64(out, &mut offset, args.tick_limit)?;
        Ok(offset)
    }

    pub(crate) fn write_cancel_orders_by_id_data(
        args: CancelOrdersByIdArgs<'_>,
        out: &mut [u8],
    ) -> Result<usize, crate::PhoenixIxError> {
        if args.order_ids.is_empty() {
            return Err(crate::PhoenixIxError::NoOrderIds);
        }
        if args.order_ids.len() > MAX_CANCEL_ORDER_IDS {
            return Err(crate::PhoenixIxError::TooManyOrderIds);
        }
        let expected = 8 + 4 + args.order_ids.len() * 20;
        require_data(out, expected)?;
        let mut offset = 0;
        write_bytes(
            out,
            &mut offset,
            &PhoenixInstruction::CancelOrdersById.discriminant(),
        )?;
        write_u32(out, &mut offset, args.order_ids.len() as u32)?;
        for cancel_id in args.order_ids {
            write_cancel_id(out, &mut offset, *cancel_id)?;
        }
        Ok(offset)
    }

    pub(crate) fn write_place_multi_limit_order_data(
        args: PlaceMultiLimitOrderArgs<'_>,
        out: &mut [u8],
    ) -> Result<usize, crate::PhoenixIxError> {
        let expected = place_multi_limit_order_data_len(args)?;
        require_data(out, expected)?;
        let mut offset = 0;
        write_bytes(
            out,
            &mut offset,
            &PhoenixInstruction::PlaceMultiLimitOrder.discriminant(),
        )?;
        write_condensed_order_slice(out, &mut offset, args.bids)?;
        write_condensed_order_slice(out, &mut offset, args.asks)?;
        match args.client_order_id {
            Some(client_order_id) => {
                write_u8(out, &mut offset, 1)?;
                write_bytes(out, &mut offset, &client_order_id_to_bytes(client_order_id))?;
            }
            None => write_u8(out, &mut offset, 0)?,
        }
        write_bool(out, &mut offset, args.slide)?;
        Ok(offset)
    }

    fn write_discriminant_data(
        discriminant: [u8; 8],
        out: &mut [u8],
    ) -> Result<usize, crate::PhoenixIxError> {
        require_data(out, DISCRIMINANT_DATA_LEN)?;
        let mut offset = 0;
        write_bytes(out, &mut offset, &discriminant)?;
        Ok(offset)
    }

    fn write_trigger(
        out: &mut [u8],
        offset: &mut usize,
        trigger: TriggerOrderParams,
    ) -> Result<(), crate::PhoenixIxError> {
        write_u8(out, offset, trigger.trigger_direction() as u8)?;
        write_u8(out, offset, trigger.trade_side() as u8)?;
        write_u8(out, offset, trigger.order_kind() as u8)?;
        write_u64(out, offset, trigger.trigger_price())?;
        write_u64(out, offset, trigger.execution_price())
    }

    fn write_option_trigger(
        out: &mut [u8],
        offset: &mut usize,
        trigger: Option<TriggerOrderParams>,
    ) -> Result<(), crate::PhoenixIxError> {
        match trigger {
            Some(trigger) => {
                write_u8(out, offset, 1)?;
                write_trigger(out, offset, trigger)
            }
            None => write_u8(out, offset, 0),
        }
    }

    fn write_fifo_order_id(
        out: &mut [u8],
        offset: &mut usize,
        order_id: FifoOrderId,
    ) -> Result<(), crate::PhoenixIxError> {
        write_u64(out, offset, order_id.price_in_ticks)?;
        write_u64(out, offset, order_id.order_sequence_number)
    }

    fn write_cancel_id(
        out: &mut [u8],
        offset: &mut usize,
        cancel_id: CancelId,
    ) -> Result<(), crate::PhoenixIxError> {
        write_u32(out, offset, cancel_id.node_pointer)?;
        write_fifo_order_id(out, offset, cancel_id.order_id)
    }

    fn write_condensed_order(
        out: &mut [u8],
        offset: &mut usize,
        order: &CondensedOrder,
    ) -> Result<(), crate::PhoenixIxError> {
        write_u64(out, offset, order.price_in_ticks)?;
        write_u64(out, offset, order.size_in_base_lots)?;
        write_option_u64(out, offset, order.last_valid_slot)
    }

    fn write_condensed_order_slice(
        out: &mut [u8],
        offset: &mut usize,
        orders: &[CondensedOrder],
    ) -> Result<(), crate::PhoenixIxError> {
        let len = u32::try_from(orders.len())
            .map_err(|_| crate::PhoenixIxError::InstructionDataTooLarge)?;
        write_u32(out, offset, len)?;
        for order in orders {
            write_condensed_order(out, offset, order)?;
        }
        Ok(())
    }

    fn condensed_order_len(order: &CondensedOrder) -> usize {
        8 + 8 + option_u64_len(order.last_valid_slot)
    }

    fn place_multi_limit_order_data_len(
        args: PlaceMultiLimitOrderArgs<'_>,
    ) -> Result<usize, crate::PhoenixIxError> {
        let mut len = 8usize;
        len = len
            .checked_add(4)
            .ok_or(crate::PhoenixIxError::InstructionDataTooLarge)?;
        for order in args.bids {
            len = len
                .checked_add(condensed_order_len(order))
                .ok_or(crate::PhoenixIxError::InstructionDataTooLarge)?;
        }
        len = len
            .checked_add(4)
            .ok_or(crate::PhoenixIxError::InstructionDataTooLarge)?;
        for order in args.asks {
            len = len
                .checked_add(condensed_order_len(order))
                .ok_or(crate::PhoenixIxError::InstructionDataTooLarge)?;
        }
        let client_id_len = if args.client_order_id.is_some() {
            17
        } else {
            1
        };
        len.checked_add(client_id_len + 1)
            .ok_or(crate::PhoenixIxError::InstructionDataTooLarge)
    }

    fn validate_conditional_triggers(
        greater_trigger_order: Option<TriggerOrderParams>,
        less_trigger_order: Option<TriggerOrderParams>,
    ) -> Result<(), crate::PhoenixIxError> {
        if greater_trigger_order.is_none() && less_trigger_order.is_none() {
            return Err(crate::PhoenixIxError::MissingConditionalOrderTrigger);
        }
        if let Some(trigger) = greater_trigger_order
            && trigger.trigger_direction() != Direction::GreaterThan
        {
            return Err(crate::PhoenixIxError::InvalidConditionalOrderTriggerDirection);
        }
        if let Some(trigger) = less_trigger_order
            && trigger.trigger_direction() != Direction::LessThan
        {
            return Err(crate::PhoenixIxError::InvalidConditionalOrderTriggerDirection);
        }
        Ok(())
    }

    fn validate_conditional_sizing(
        size_base_lots: Option<u64>,
        size_percent: Option<u8>,
    ) -> Result<(), crate::PhoenixIxError> {
        if size_base_lots.is_some() == size_percent.is_some() {
            return Err(crate::PhoenixIxError::InvalidConditionalOrderSize);
        }
        if let Some(percent) = size_percent
            && (percent == 0 || percent > 100)
        {
            return Err(crate::PhoenixIxError::InvalidConditionalOrderPercent);
        }
        Ok(())
    }

    fn validate_conditional_order_index(index: u8) -> Result<(), crate::PhoenixIxError> {
        if index == 0 || index > 191 {
            return Err(crate::PhoenixIxError::InvalidConditionalOrderIndex);
        }
        Ok(())
    }
}

pub mod ember {
    use pinocchio::account_info::AccountInfo;
    use pinocchio::instruction::Signer;
    use pinocchio::program_error::ProgramError;

    use super::{
        CpiBuffers, CpiScratch, invoke_prepared, invoke_prepared_signed, map_ix_error,
        require_accounts, require_data, write_account, write_bytes, write_option_u64, write_u64,
    };
    use crate::EmberInstruction;

    pub const EMBER_DEPOSIT_ACCOUNT_COUNT: usize = 8;
    pub const EMBER_DEPOSIT_DATA_LEN: usize = 16;
    pub const EMBER_WITHDRAW_ACCOUNT_COUNT: usize = 8;
    pub const EMBER_WITHDRAW_MAX_DATA_LEN: usize = 17;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct EmberDepositArgs {
        pub amount: u64,
    }

    pub struct EmberDeposit<'a> {
        pub ember_program: &'a AccountInfo,
        pub trader: &'a AccountInfo,
        pub ember_state: &'a AccountInfo,
        pub usdc_mint: &'a AccountInfo,
        pub canonical_mint: &'a AccountInfo,
        pub trader_usdc_account: &'a AccountInfo,
        pub trader_phoenix_account: &'a AccountInfo,
        pub ember_vault: &'a AccountInfo,
        pub token_program: &'a AccountInfo,
    }

    impl<'a> EmberDeposit<'a> {
        pub const ACCOUNT_COUNT: usize = EMBER_DEPOSIT_ACCOUNT_COUNT;
        pub const DATA_LEN: usize = EMBER_DEPOSIT_DATA_LEN;

        pub fn invoke<const MAX_ACCOUNTS: usize, const MAX_DATA_LEN: usize>(
            &self,
            args: EmberDepositArgs,
            scratch: &mut CpiScratch<'a, MAX_ACCOUNTS, MAX_DATA_LEN>,
        ) -> Result<(), ProgramError> {
            self.invoke_with_buffers(args, &mut scratch.buffers())
        }

        pub fn invoke_signed<const MAX_ACCOUNTS: usize, const MAX_DATA_LEN: usize>(
            &self,
            args: EmberDepositArgs,
            signer_seeds: &[Signer<'_, '_>],
            scratch: &mut CpiScratch<'a, MAX_ACCOUNTS, MAX_DATA_LEN>,
        ) -> Result<(), ProgramError> {
            self.invoke_signed_with_buffers(args, signer_seeds, &mut scratch.buffers())
        }

        pub fn invoke_with_buffers(
            &self,
            args: EmberDepositArgs,
            buffers: &mut CpiBuffers<'a, '_>,
        ) -> Result<(), ProgramError> {
            self.write_accounts(buffers)?;
            let data_len = write_ember_deposit_data(args, buffers.data).map_err(map_ix_error)?;
            invoke_prepared(self.ember_program, Self::ACCOUNT_COUNT, data_len, buffers)
        }

        pub fn invoke_signed_with_buffers(
            &self,
            args: EmberDepositArgs,
            signer_seeds: &[Signer<'_, '_>],
            buffers: &mut CpiBuffers<'a, '_>,
        ) -> Result<(), ProgramError> {
            self.write_accounts(buffers)?;
            let data_len = write_ember_deposit_data(args, buffers.data).map_err(map_ix_error)?;
            invoke_prepared_signed(
                self.ember_program,
                Self::ACCOUNT_COUNT,
                data_len,
                signer_seeds,
                buffers,
            )
        }

        fn write_accounts(&self, buffers: &mut CpiBuffers<'a, '_>) -> Result<(), ProgramError> {
            require_accounts(buffers, Self::ACCOUNT_COUNT)?;
            write_account(buffers, 0, self.trader, false, true);
            write_account(buffers, 1, self.ember_state, false, false);
            write_account(buffers, 2, self.usdc_mint, false, false);
            write_account(buffers, 3, self.canonical_mint, true, false);
            write_account(buffers, 4, self.trader_usdc_account, true, false);
            write_account(buffers, 5, self.trader_phoenix_account, true, false);
            write_account(buffers, 6, self.ember_vault, true, false);
            write_account(buffers, 7, self.token_program, false, false);
            Ok(())
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct EmberWithdrawArgs {
        pub amount: Option<u64>,
    }

    pub struct EmberWithdraw<'a> {
        pub ember_program: &'a AccountInfo,
        pub trader: &'a AccountInfo,
        pub ember_state: &'a AccountInfo,
        pub usdc_mint: &'a AccountInfo,
        pub canonical_mint: &'a AccountInfo,
        pub trader_usdc_account: &'a AccountInfo,
        pub trader_phoenix_account: &'a AccountInfo,
        pub ember_vault: &'a AccountInfo,
        pub token_program: &'a AccountInfo,
    }

    impl<'a> EmberWithdraw<'a> {
        pub const ACCOUNT_COUNT: usize = EMBER_WITHDRAW_ACCOUNT_COUNT;
        pub const MAX_DATA_LEN: usize = EMBER_WITHDRAW_MAX_DATA_LEN;

        pub fn invoke<const MAX_ACCOUNTS: usize, const MAX_DATA_LEN: usize>(
            &self,
            args: EmberWithdrawArgs,
            scratch: &mut CpiScratch<'a, MAX_ACCOUNTS, MAX_DATA_LEN>,
        ) -> Result<(), ProgramError> {
            self.invoke_with_buffers(args, &mut scratch.buffers())
        }

        pub fn invoke_signed<const MAX_ACCOUNTS: usize, const MAX_DATA_LEN: usize>(
            &self,
            args: EmberWithdrawArgs,
            signer_seeds: &[Signer<'_, '_>],
            scratch: &mut CpiScratch<'a, MAX_ACCOUNTS, MAX_DATA_LEN>,
        ) -> Result<(), ProgramError> {
            self.invoke_signed_with_buffers(args, signer_seeds, &mut scratch.buffers())
        }

        pub fn invoke_with_buffers(
            &self,
            args: EmberWithdrawArgs,
            buffers: &mut CpiBuffers<'a, '_>,
        ) -> Result<(), ProgramError> {
            self.write_accounts(buffers)?;
            let data_len = write_ember_withdraw_data(args, buffers.data).map_err(map_ix_error)?;
            invoke_prepared(self.ember_program, Self::ACCOUNT_COUNT, data_len, buffers)
        }

        pub fn invoke_signed_with_buffers(
            &self,
            args: EmberWithdrawArgs,
            signer_seeds: &[Signer<'_, '_>],
            buffers: &mut CpiBuffers<'a, '_>,
        ) -> Result<(), ProgramError> {
            self.write_accounts(buffers)?;
            let data_len = write_ember_withdraw_data(args, buffers.data).map_err(map_ix_error)?;
            invoke_prepared_signed(
                self.ember_program,
                Self::ACCOUNT_COUNT,
                data_len,
                signer_seeds,
                buffers,
            )
        }

        fn write_accounts(&self, buffers: &mut CpiBuffers<'a, '_>) -> Result<(), ProgramError> {
            require_accounts(buffers, Self::ACCOUNT_COUNT)?;
            write_account(buffers, 0, self.trader, false, true);
            write_account(buffers, 1, self.ember_state, false, false);
            write_account(buffers, 2, self.usdc_mint, false, false);
            write_account(buffers, 3, self.canonical_mint, true, false);
            write_account(buffers, 4, self.trader_usdc_account, true, false);
            write_account(buffers, 5, self.trader_phoenix_account, true, false);
            write_account(buffers, 6, self.ember_vault, true, false);
            write_account(buffers, 7, self.token_program, false, false);
            Ok(())
        }
    }

    pub(crate) fn write_ember_deposit_data(
        args: EmberDepositArgs,
        out: &mut [u8],
    ) -> Result<usize, crate::PhoenixIxError> {
        if args.amount == 0 {
            return Err(crate::PhoenixIxError::InvalidDepositAmount);
        }
        require_data(out, EMBER_DEPOSIT_DATA_LEN)?;
        let mut offset = 0;
        write_bytes(out, &mut offset, &EmberInstruction::Deposit.discriminant())?;
        write_u64(out, &mut offset, args.amount)?;
        Ok(offset)
    }

    pub(crate) fn write_ember_withdraw_data(
        args: EmberWithdrawArgs,
        out: &mut [u8],
    ) -> Result<usize, crate::PhoenixIxError> {
        if matches!(args.amount, Some(0)) {
            return Err(crate::PhoenixIxError::InvalidWithdrawAmount);
        }
        let expected = if args.amount.is_some() { 17 } else { 9 };
        require_data(out, expected)?;
        let mut offset = 0;
        write_bytes(out, &mut offset, &EmberInstruction::Withdraw.discriminant())?;
        write_option_u64(out, &mut offset, args.amount)?;
        Ok(offset)
    }
}

pub mod hawkeye {
    use pinocchio::account_info::AccountInfo;
    use pinocchio::cpi::get_return_data;
    use pinocchio::instruction::Signer;
    use pinocchio::program_error::ProgramError;

    use super::{
        CpiBuffers, CpiScratch, invoke_prepared, invoke_prepared_signed, map_ix_error,
        require_accounts, require_data, write_account, write_bytes,
    };
    use crate::hawkeye::{ViewMarginReturn, decode_hawkeye_return};
    use crate::{HAWKEYE_PROGRAM_ID, HawkeyeInstruction};

    pub const VIEW_MARGIN_MIN_ACCOUNT_COUNT: usize = 4;
    pub const VIEW_MARGIN_DATA_LEN: usize = 8;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub struct ViewMarginArgs;

    pub struct ViewMargin<'a> {
        pub hawkeye_program: &'a AccountInfo,
        pub phoenix_program: &'a AccountInfo,
        pub global_config: &'a AccountInfo,
        pub global_trader_index: &'a [AccountInfo],
        pub active_trader_buffer: &'a [AccountInfo],
        pub perp_asset_map: &'a AccountInfo,
        pub trader: &'a AccountInfo,
    }

    pub type HawkeyeViewMargin<'a> = ViewMargin<'a>;

    impl<'a> ViewMargin<'a> {
        pub const DATA_LEN: usize = VIEW_MARGIN_DATA_LEN;
        pub const MIN_ACCOUNT_COUNT: usize = VIEW_MARGIN_MIN_ACCOUNT_COUNT;

        pub fn account_count(&self) -> usize {
            Self::MIN_ACCOUNT_COUNT
                + self.global_trader_index.len()
                + self.active_trader_buffer.len()
        }

        pub fn invoke<const MAX_ACCOUNTS: usize, const MAX_DATA_LEN: usize>(
            &self,
            args: ViewMarginArgs,
            scratch: &mut CpiScratch<'a, MAX_ACCOUNTS, MAX_DATA_LEN>,
        ) -> Result<(), ProgramError> {
            self.invoke_with_buffers(args, &mut scratch.buffers())
        }

        pub fn invoke_signed<const MAX_ACCOUNTS: usize, const MAX_DATA_LEN: usize>(
            &self,
            args: ViewMarginArgs,
            signer_seeds: &[Signer<'_, '_>],
            scratch: &mut CpiScratch<'a, MAX_ACCOUNTS, MAX_DATA_LEN>,
        ) -> Result<(), ProgramError> {
            self.invoke_signed_with_buffers(args, signer_seeds, &mut scratch.buffers())
        }

        pub fn invoke_and_decode<const MAX_ACCOUNTS: usize, const MAX_DATA_LEN: usize>(
            &self,
            scratch: &mut CpiScratch<'a, MAX_ACCOUNTS, MAX_DATA_LEN>,
        ) -> Result<ViewMarginReturn, ProgramError> {
            self.invoke(ViewMarginArgs, scratch)?;
            decode_margin_return()
        }

        pub fn invoke_with_buffers(
            &self,
            _args: ViewMarginArgs,
            buffers: &mut CpiBuffers<'a, '_>,
        ) -> Result<(), ProgramError> {
            let account_count = self.write_accounts(buffers)?;
            let data_len = write_view_margin_data(buffers.data).map_err(map_ix_error)?;
            invoke_prepared(self.hawkeye_program, account_count, data_len, buffers)
        }

        pub fn invoke_signed_with_buffers(
            &self,
            _args: ViewMarginArgs,
            signer_seeds: &[Signer<'_, '_>],
            buffers: &mut CpiBuffers<'a, '_>,
        ) -> Result<(), ProgramError> {
            let account_count = self.write_accounts(buffers)?;
            let data_len = write_view_margin_data(buffers.data).map_err(map_ix_error)?;
            invoke_prepared_signed(
                self.hawkeye_program,
                account_count,
                data_len,
                signer_seeds,
                buffers,
            )
        }

        pub fn invoke_and_decode_with_buffers(
            &self,
            buffers: &mut CpiBuffers<'a, '_>,
        ) -> Result<ViewMarginReturn, ProgramError> {
            self.invoke_with_buffers(ViewMarginArgs, buffers)?;
            decode_margin_return()
        }

        fn write_accounts(&self, buffers: &mut CpiBuffers<'a, '_>) -> Result<usize, ProgramError> {
            let account_count = self.account_count();
            require_accounts(buffers, account_count)?;
            write_account(buffers, 0, self.phoenix_program, false, false);
            write_account(buffers, 1, self.global_config, false, false);
            let mut index = 2;
            for account in self.global_trader_index {
                write_account(buffers, index, account, false, false);
                index += 1;
            }
            for account in self.active_trader_buffer {
                write_account(buffers, index, account, false, false);
                index += 1;
            }
            write_account(buffers, index, self.perp_asset_map, false, false);
            index += 1;
            write_account(buffers, index, self.trader, false, false);
            Ok(account_count)
        }
    }

    pub(crate) fn write_view_margin_data(out: &mut [u8]) -> Result<usize, crate::PhoenixIxError> {
        require_data(out, VIEW_MARGIN_DATA_LEN)?;
        let mut offset = 0;
        write_bytes(
            out,
            &mut offset,
            &HawkeyeInstruction::ViewMargin.discriminant(),
        )?;
        Ok(offset)
    }

    fn decode_margin_return() -> Result<ViewMarginReturn, ProgramError> {
        let return_data = get_return_data().ok_or(ProgramError::InvalidAccountData)?;
        if return_data.program_id() != &HAWKEYE_PROGRAM_ID.to_bytes() {
            return Err(ProgramError::InvalidAccountData);
        }
        decode_hawkeye_return::<ViewMarginReturn>(return_data.as_slice(), "view_margin")
            .map_err(|_| ProgramError::InvalidAccountData)
    }
}

#[cfg(test)]
mod tests {
    use solana_pubkey::Pubkey;

    use super::*;
    use crate::prelude::{
        CancelAllParams, CancelConditionalOrderParams, CancelId, CancelOrdersByIdParams,
        CancelStopLossParams, CancelUpToParams, CondensedOrder,
        CreateConditionalOrdersAccountParams, DepositFundsParams, Direction, EmberDepositParams,
        EmberWithdrawParams, FifoOrderId, HawkeyeTraderViewAccounts, LimitOrderParams,
        MarketOrderDelegatedParams, MarketOrderParams, MultiLimitOrderParams,
        OnboardTraderDelegatedParams, OrderFlags, OrderPacket, PlaceAttachedConditionalOrderParams,
        PlaceLimitOrderWithConditionalsParams, PlacePositionConditionalOrderParams,
        PlaceStopLossParams, RegisterTraderParams, SelfTradeBehavior, Side, StopLossOrderKind,
        SyncParentToChildParams, TransferCollateralChildToParentParams, TransferCollateralParams,
        TriggerOrderParams, WithdrawFundsParams, create_cancel_all_ix,
        create_cancel_conditional_order_ix, create_cancel_orders_by_id_ix,
        create_cancel_stop_loss_ix, create_cancel_up_to_ix,
        create_create_conditional_orders_account_ix, create_deposit_funds_ix,
        create_ember_deposit_ix, create_ember_withdraw_ix, create_hawkeye_view_margin_ix,
        create_onboard_trader_delegated_ix, create_place_attached_conditional_order_ix,
        create_place_limit_order_ix, create_place_limit_order_with_conditionals_ix,
        create_place_market_order_delegated_ix, create_place_market_order_ix,
        create_place_multi_limit_order_ix, create_place_position_conditional_order_ix,
        create_place_stop_loss_ix, create_register_trader_ix, create_sync_parent_to_child_ix,
        create_transfer_collateral_child_to_parent_ix, create_transfer_collateral_ix,
        create_withdraw_funds_ix,
    };

    fn trigger(direction: Direction) -> TriggerOrderParams {
        TriggerOrderParams::new(direction, Side::Ask, StopLossOrderKind::IOC, 50_000, 49_999)
    }

    #[test]
    fn register_trader_cpi_data_matches_builder() {
        let payer = Pubkey::new_unique();
        let trader = Pubkey::new_unique();
        let trader_account = Pubkey::new_unique();
        let args = phoenix::RegisterTraderArgs {
            max_positions: 128,
            trader_preference_bits: 1,
            trader_pda_index: 7,
            subaccount_index: 2,
        };
        let params = RegisterTraderParams::builder()
            .payer(payer)
            .trader(trader)
            .trader_account(trader_account)
            .max_positions(args.max_positions)
            .trader_preference_bits(args.trader_preference_bits)
            .trader_pda_index(args.trader_pda_index)
            .subaccount_index(args.subaccount_index)
            .build()
            .unwrap();
        let ix = create_register_trader_ix(params).unwrap();
        let mut data = [0; phoenix::REGISTER_TRADER_DATA_LEN];
        let len = phoenix::write_register_trader_data(args, &mut data).unwrap();

        assert_eq!(&data[..len], ix.data.as_slice());
    }

    #[test]
    fn set_trader_capabilities_delegated_cpi_data_matches_builder() {
        let params = OnboardTraderDelegatedParams::builder()
            .authority(Pubkey::new_unique())
            .permission_account(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .build()
            .unwrap();
        let ix = create_onboard_trader_delegated_ix(params).unwrap();
        let mut data = [0; phoenix::SET_TRADER_CAPABILITIES_DELEGATED_DATA_LEN];
        let len = phoenix::write_set_trader_capabilities_delegated_data(&mut data).unwrap();

        assert_eq!(&data[..len], ix.data.as_slice());
    }

    #[test]
    fn place_limit_order_cpi_data_matches_builder() {
        let args = phoenix::PlaceLimitOrderArgs {
            side: Side::Bid,
            price_in_ticks: 50_000,
            num_base_lots: 1_000,
            self_trade_behavior: SelfTradeBehavior::CancelProvide,
            match_limit: Some(4),
            client_order_id: 123,
            last_valid_slot: Some(456),
            order_flags: OrderFlags::ReduceOnly,
            cancel_existing: true,
        };
        let params = LimitOrderParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .side(args.side)
            .price_in_ticks(args.price_in_ticks)
            .num_base_lots(args.num_base_lots)
            .self_trade_behavior(args.self_trade_behavior)
            .match_limit(args.match_limit.unwrap())
            .client_order_id(args.client_order_id)
            .last_valid_slot(args.last_valid_slot.unwrap())
            .order_flags(args.order_flags)
            .cancel_existing(args.cancel_existing)
            .build()
            .unwrap();
        let ix = create_place_limit_order_ix(params).unwrap();
        let mut data = [0; phoenix::PLACE_LIMIT_ORDER_MAX_DATA_LEN];
        let len = phoenix::write_place_limit_order_data(args, &mut data).unwrap();

        assert_eq!(&data[..len], ix.data.as_slice());
    }

    #[test]
    fn place_market_order_cpi_data_matches_builder() {
        let args = phoenix::PlaceMarketOrderArgs {
            side: Side::Ask,
            price_in_ticks: Some(50_000),
            num_base_lots: 1_000,
            num_quote_lots: Some(25_000),
            min_base_lots_to_fill: 10,
            min_quote_lots_to_fill: 20,
            self_trade_behavior: SelfTradeBehavior::CancelProvide,
            match_limit: Some(4),
            client_order_id: 123,
            last_valid_slot: Some(456),
            order_flags: OrderFlags::ReduceOnly,
            cancel_existing: true,
        };
        let params = MarketOrderParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .side(args.side)
            .price_in_ticks(args.price_in_ticks.unwrap())
            .num_base_lots(args.num_base_lots)
            .num_quote_lots(args.num_quote_lots.unwrap())
            .min_base_lots_to_fill(args.min_base_lots_to_fill)
            .min_quote_lots_to_fill(args.min_quote_lots_to_fill)
            .self_trade_behavior(args.self_trade_behavior)
            .match_limit(args.match_limit.unwrap())
            .client_order_id(args.client_order_id)
            .last_valid_slot(args.last_valid_slot.unwrap())
            .order_flags(args.order_flags)
            .cancel_existing(args.cancel_existing)
            .build()
            .unwrap();
        let ix = create_place_market_order_ix(params).unwrap();
        let mut data = [0; phoenix::PLACE_MARKET_ORDER_MAX_DATA_LEN];
        let len = phoenix::write_place_market_order_data(args, &mut data).unwrap();

        assert_eq!(&data[..len], ix.data.as_slice());
    }

    #[test]
    fn place_market_order_delegated_cpi_data_matches_builder() {
        let args = phoenix::PlaceMarketOrderArgs {
            side: Side::Ask,
            price_in_ticks: Some(50_000),
            num_base_lots: 1_000,
            num_quote_lots: Some(25_000),
            min_base_lots_to_fill: 10,
            min_quote_lots_to_fill: 20,
            self_trade_behavior: SelfTradeBehavior::CancelProvide,
            match_limit: Some(4),
            client_order_id: 123,
            last_valid_slot: Some(456),
            order_flags: OrderFlags::ReduceOnly,
            cancel_existing: true,
        };
        let market_order = MarketOrderParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .side(args.side)
            .price_in_ticks(args.price_in_ticks.unwrap())
            .num_base_lots(args.num_base_lots)
            .num_quote_lots(args.num_quote_lots.unwrap())
            .min_base_lots_to_fill(args.min_base_lots_to_fill)
            .min_quote_lots_to_fill(args.min_quote_lots_to_fill)
            .self_trade_behavior(args.self_trade_behavior)
            .match_limit(args.match_limit.unwrap())
            .client_order_id(args.client_order_id)
            .last_valid_slot(args.last_valid_slot.unwrap())
            .order_flags(args.order_flags)
            .cancel_existing(args.cancel_existing)
            .build()
            .unwrap();
        let params = MarketOrderDelegatedParams::builder()
            .market_order(market_order)
            .trader_wallet(Pubkey::new_unique())
            .permission_account(Pubkey::new_unique())
            .build()
            .unwrap();
        let ix = create_place_market_order_delegated_ix(params).unwrap();
        let mut data = [0; phoenix::PLACE_MARKET_ORDER_MAX_DATA_LEN];
        let len = phoenix::write_place_market_order_delegated_data(args, &mut data).unwrap();

        assert_eq!(&data[..len], ix.data.as_slice());
    }

    #[test]
    fn phoenix_deposit_cpi_data_matches_builder() {
        let amount = 100_000;
        let params = DepositFundsParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .canonical_mint(Pubkey::new_unique())
            .global_vault(Pubkey::new_unique())
            .trader_token_account(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .amount(amount)
            .build()
            .unwrap();
        let ix = create_deposit_funds_ix(params).unwrap();
        let mut data = [0; phoenix::PHOENIX_DEPOSIT_DATA_LEN];
        let len =
            phoenix::write_phoenix_deposit_data(phoenix::PhoenixDepositArgs { amount }, &mut data)
                .unwrap();

        assert_eq!(&data[..len], ix.data.as_slice());
    }

    #[test]
    fn phoenix_withdraw_cpi_data_matches_builder() {
        let amount = 100_000;
        let params = WithdrawFundsParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .global_vault(Pubkey::new_unique())
            .trader_token_account(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .withdraw_queue(Pubkey::new_unique())
            .amount(amount)
            .build()
            .unwrap();
        let ix = create_withdraw_funds_ix(params).unwrap();
        let mut data = [0; phoenix::PHOENIX_WITHDRAW_DATA_LEN];
        let len = phoenix::write_phoenix_withdraw_data(
            phoenix::PhoenixWithdrawArgs { amount },
            &mut data,
        )
        .unwrap();

        assert_eq!(&data[..len], ix.data.as_slice());
    }

    #[test]
    fn transfer_collateral_cpi_data_matches_builder() {
        let amount = 100_000;
        let params = TransferCollateralParams::builder()
            .trader(Pubkey::new_unique())
            .src_trader_account(Pubkey::new_unique())
            .dst_trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .amount(amount)
            .build()
            .unwrap();
        let ix = create_transfer_collateral_ix(params).unwrap();
        let mut data = [0; phoenix::TRANSFER_COLLATERAL_DATA_LEN];
        let len = phoenix::write_transfer_collateral_data(
            phoenix::TransferCollateralArgs { amount },
            &mut data,
        )
        .unwrap();

        assert_eq!(&data[..len], ix.data.as_slice());
    }

    #[test]
    fn transfer_child_to_parent_cpi_data_matches_builder() {
        let params = TransferCollateralChildToParentParams::builder()
            .trader(Pubkey::new_unique())
            .child_trader_account(Pubkey::new_unique())
            .parent_trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .build()
            .unwrap();
        let ix = create_transfer_collateral_child_to_parent_ix(params).unwrap();
        let mut data = [0; phoenix::DISCRIMINANT_DATA_LEN];
        let len = phoenix::write_transfer_collateral_child_to_parent_data(
            phoenix::TransferCollateralChildToParentArgs,
            &mut data,
        )
        .unwrap();

        assert_eq!(&data[..len], ix.data.as_slice());
    }

    #[test]
    fn sync_parent_to_child_cpi_data_matches_builder() {
        let params = SyncParentToChildParams::builder()
            .trader_wallet(Pubkey::new_unique())
            .parent_trader_account(Pubkey::new_unique())
            .child_trader_account(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .build()
            .unwrap();
        let ix = create_sync_parent_to_child_ix(params).unwrap();
        let mut data = [0; phoenix::DISCRIMINANT_DATA_LEN];
        let len =
            phoenix::write_sync_parent_to_child_data(phoenix::SyncParentToChildArgs, &mut data)
                .unwrap();

        assert_eq!(&data[..len], ix.data.as_slice());
    }

    #[test]
    fn update_trader_state_cpi_data_is_discriminant() {
        let mut data = [0; phoenix::DISCRIMINANT_DATA_LEN];
        let len =
            phoenix::write_update_trader_state_data(phoenix::UpdateTraderStateArgs, &mut data)
                .unwrap();

        assert_eq!(
            &data[..len],
            &crate::PhoenixInstruction::UpdateTraderState.discriminant()
        );
    }

    #[test]
    fn place_stop_loss_cpi_data_matches_builder() {
        let args = phoenix::PlaceStopLossArgs {
            trigger_price: 50_000,
            execution_price: 49_000,
            trade_side: Side::Ask,
            execution_direction: Direction::LessThan,
            order_kind: StopLossOrderKind::IOC,
        };
        let params = PlaceStopLossParams::builder()
            .funder(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .position_authority(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .asset_id(1)
            .trigger_price(args.trigger_price)
            .execution_price(args.execution_price)
            .trade_side(args.trade_side)
            .execution_direction(args.execution_direction)
            .order_kind(args.order_kind)
            .build()
            .unwrap();
        let ix = create_place_stop_loss_ix(params).unwrap();
        let mut data = [0; phoenix::PLACE_STOP_LOSS_DATA_LEN];
        let len = phoenix::write_place_stop_loss_data(args, &mut data).unwrap();

        assert_eq!(&data[..len], ix.data.as_slice());
    }

    #[test]
    fn cancel_stop_loss_cpi_data_matches_builder() {
        let args = phoenix::CancelStopLossArgs {
            execution_direction: Direction::LessThan,
        };
        let params = CancelStopLossParams::builder()
            .funder(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .position_authority(Pubkey::new_unique())
            .asset_id(1)
            .execution_direction(args.execution_direction)
            .build()
            .unwrap();
        let ix = create_cancel_stop_loss_ix(params).unwrap();
        let mut data = [0; phoenix::CANCEL_STOP_LOSS_DATA_LEN];
        let len = phoenix::write_cancel_stop_loss_data(args, &mut data).unwrap();

        assert_eq!(&data[..len], ix.data.as_slice());
    }

    #[test]
    fn ember_deposit_cpi_data_matches_builder() {
        let amount = 100_000;
        let params = EmberDepositParams::builder()
            .trader(Pubkey::new_unique())
            .usdc_mint(Pubkey::new_unique())
            .canonical_mint(Pubkey::new_unique())
            .trader_usdc_account(Pubkey::new_unique())
            .trader_phoenix_account(Pubkey::new_unique())
            .amount(amount)
            .build()
            .unwrap();
        let ix = create_ember_deposit_ix(params).unwrap();
        let mut data = [0; ember::EMBER_DEPOSIT_DATA_LEN];
        let len =
            ember::write_ember_deposit_data(ember::EmberDepositArgs { amount }, &mut data).unwrap();

        assert_eq!(&data[..len], ix.data.as_slice());
    }

    #[test]
    fn ember_withdraw_cpi_data_matches_builder() {
        let args = ember::EmberWithdrawArgs {
            amount: Some(100_000),
        };
        let params = EmberWithdrawParams::builder()
            .trader(Pubkey::new_unique())
            .usdc_mint(Pubkey::new_unique())
            .canonical_mint(Pubkey::new_unique())
            .trader_usdc_account(Pubkey::new_unique())
            .trader_phoenix_account(Pubkey::new_unique())
            .amount(args.amount)
            .build()
            .unwrap();
        let ix = create_ember_withdraw_ix(params).unwrap();
        let mut data = [0; ember::EMBER_WITHDRAW_MAX_DATA_LEN];
        let len = ember::write_ember_withdraw_data(args, &mut data).unwrap();

        assert_eq!(&data[..len], ix.data.as_slice());
    }

    #[test]
    fn create_conditional_orders_account_cpi_data_matches_builder() {
        let args = phoenix::CreateConditionalOrdersAccountArgs { capacity: 32 };
        let params = CreateConditionalOrdersAccountParams::builder()
            .payer(Pubkey::new_unique())
            .trader_wallet(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .capacity(args.capacity)
            .build()
            .unwrap();
        let ix = create_create_conditional_orders_account_ix(params).unwrap();
        let mut data = [0; phoenix::CREATE_CONDITIONAL_ORDERS_ACCOUNT_DATA_LEN];
        let len = phoenix::write_create_conditional_orders_account_data(args, &mut data).unwrap();

        assert_eq!(&data[..len], ix.data.as_slice());
    }

    #[test]
    fn place_position_conditional_order_cpi_data_matches_builder() {
        let args = phoenix::PlacePositionConditionalOrderArgs {
            asset_id: 1,
            greater_trigger_order: Some(trigger(Direction::GreaterThan)),
            less_trigger_order: None,
            size_base_lots: Some(10),
            size_percent: None,
        };
        let params = PlacePositionConditionalOrderParams::builder()
            .payer(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .trader_wallet(Pubkey::new_unique())
            .asset_id(args.asset_id)
            .greater_trigger_order(args.greater_trigger_order.unwrap())
            .size_base_lots(args.size_base_lots.unwrap())
            .build()
            .unwrap();
        let ix = create_place_position_conditional_order_ix(params).unwrap();
        let mut data = [0; phoenix::PLACE_POSITION_CONDITIONAL_ORDER_MAX_DATA_LEN];
        let len = phoenix::write_place_position_conditional_order_data(args, &mut data).unwrap();

        assert_eq!(&data[..len], ix.data.as_slice());
    }

    #[test]
    fn cancel_conditional_order_cpi_data_matches_builder() {
        let args = phoenix::CancelConditionalOrderArgs {
            conditional_order_index: 1,
            disable_first: true,
            disable_second: false,
        };
        let params = CancelConditionalOrderParams::builder()
            .trader_account(Pubkey::new_unique())
            .trader_wallet(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .conditional_order_index(args.conditional_order_index)
            .disable_first(args.disable_first)
            .disable_second(args.disable_second)
            .build()
            .unwrap();
        let ix = create_cancel_conditional_order_ix(params).unwrap();
        let mut data = [0; phoenix::CANCEL_CONDITIONAL_ORDER_DATA_LEN];
        let len = phoenix::write_cancel_conditional_order_data(args, &mut data).unwrap();

        assert_eq!(&data[..len], ix.data.as_slice());
    }

    #[test]
    fn place_attached_conditional_order_cpi_data_matches_builder() {
        let order_id = FifoOrderId {
            price_in_ticks: 50_000,
            order_sequence_number: 7,
        };
        let args = phoenix::PlaceAttachedConditionalOrderArgs {
            order_id,
            asset_id: 1,
            greater_trigger_order: Some(trigger(Direction::GreaterThan)),
            less_trigger_order: None,
        };
        let params = PlaceAttachedConditionalOrderParams::builder()
            .trader_account(Pubkey::new_unique())
            .trader_wallet(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .payer(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .order_id(order_id)
            .asset_id(args.asset_id)
            .greater_trigger_order(args.greater_trigger_order.unwrap())
            .build()
            .unwrap();
        let ix = create_place_attached_conditional_order_ix(params).unwrap();
        let mut data = [0; phoenix::PLACE_ATTACHED_CONDITIONAL_ORDER_MAX_DATA_LEN];
        let len = phoenix::write_place_attached_conditional_order_data(args, &mut data).unwrap();

        assert_eq!(&data[..len], ix.data.as_slice());
    }

    #[test]
    fn place_limit_order_with_conditionals_cpi_data_matches_builder() {
        let packet = OrderPacket::limit(
            Side::Bid,
            50_000,
            1_000,
            SelfTradeBehavior::CancelProvide,
            Some(4),
            [0; 16],
            Some(456),
            OrderFlags::ReduceOnly,
            true,
        );
        let args = phoenix::PlaceLimitOrderWithConditionalsArgs {
            order_packet: &packet,
            slot: 999,
            greater_trigger_order: Some(trigger(Direction::GreaterThan)),
            less_trigger_order: None,
        };
        let params = PlaceLimitOrderWithConditionalsParams::builder()
            .trader_wallet(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .payer(Pubkey::new_unique())
            .order_packet(packet.clone())
            .slot(args.slot)
            .greater_trigger_order(args.greater_trigger_order.unwrap())
            .build()
            .unwrap();
        let ix = create_place_limit_order_with_conditionals_ix(params).unwrap();
        let mut data = [0; phoenix::PLACE_LIMIT_ORDER_WITH_CONDITIONALS_MAX_DATA_LEN];
        let len = phoenix::write_place_limit_order_with_conditionals_data(args, &mut data).unwrap();

        assert_eq!(&data[..len], ix.data.as_slice());
    }

    #[test]
    fn cancel_all_plus_conditional_cpi_data_is_discriminant() {
        let mut data = [0; phoenix::DISCRIMINANT_DATA_LEN];
        let len = phoenix::write_cancel_all_plus_conditional_data(
            phoenix::CancelAllPlusConditionalArgs,
            &mut data,
        )
        .unwrap();

        assert_eq!(
            &data[..len],
            &crate::PhoenixInstruction::CancelAllPlusConditional.discriminant()
        );
    }

    #[test]
    fn cancel_order_cpi_data_matches_builders() {
        let cancel_id = CancelId::new(50_000, 7);
        let cancel_all_params = CancelAllParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .build()
            .unwrap();
        let cancel_all_ix = create_cancel_all_ix(cancel_all_params).unwrap();
        let mut cancel_all_data = [0; phoenix::CANCEL_ALL_DATA_LEN];
        let cancel_all_len =
            phoenix::write_cancel_all_data(phoenix::CancelAllArgs, &mut cancel_all_data).unwrap();
        assert_eq!(
            &cancel_all_data[..cancel_all_len],
            cancel_all_ix.data.as_slice()
        );

        let cancel_up_to_args = phoenix::CancelUpToArgs {
            side: Side::Bid,
            num_orders_to_cancel: Some(3),
            tick_limit: Some(55_000),
        };
        let cancel_up_to_params = CancelUpToParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .side(cancel_up_to_args.side)
            .num_orders_to_cancel(cancel_up_to_args.num_orders_to_cancel.unwrap())
            .tick_limit(cancel_up_to_args.tick_limit.unwrap())
            .build()
            .unwrap();
        let cancel_up_to_ix = create_cancel_up_to_ix(cancel_up_to_params).unwrap();
        let mut cancel_up_to_data = [0; phoenix::CANCEL_UP_TO_MAX_DATA_LEN];
        let cancel_up_to_len =
            phoenix::write_cancel_up_to_data(cancel_up_to_args, &mut cancel_up_to_data).unwrap();
        assert_eq!(
            &cancel_up_to_data[..cancel_up_to_len],
            cancel_up_to_ix.data.as_slice()
        );

        let cancel_by_id_args = phoenix::CancelOrdersByIdArgs {
            order_ids: &[cancel_id],
        };
        let cancel_by_id_params = CancelOrdersByIdParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .order_ids(vec![cancel_id])
            .build()
            .unwrap();
        let cancel_by_id_ix = create_cancel_orders_by_id_ix(cancel_by_id_params).unwrap();
        let mut cancel_by_id_data = [0; phoenix::CANCEL_ORDERS_BY_ID_MAX_DATA_LEN];
        let cancel_by_id_len =
            phoenix::write_cancel_orders_by_id_data(cancel_by_id_args, &mut cancel_by_id_data)
                .unwrap();
        assert_eq!(
            &cancel_by_id_data[..cancel_by_id_len],
            cancel_by_id_ix.data.as_slice()
        );
    }

    #[test]
    fn place_multi_limit_order_cpi_data_matches_builder() {
        let bids = [CondensedOrder {
            price_in_ticks: 50_000,
            size_in_base_lots: 1_000,
            last_valid_slot: Some(456),
        }];
        let asks = [CondensedOrder {
            price_in_ticks: 60_000,
            size_in_base_lots: 2_000,
            last_valid_slot: None,
        }];
        let args = phoenix::PlaceMultiLimitOrderArgs {
            bids: &bids,
            asks: &asks,
            client_order_id: Some(123),
            slide: true,
        };
        let params = MultiLimitOrderParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .bids(bids.to_vec())
            .asks(asks.to_vec())
            .client_order_id(args.client_order_id.unwrap())
            .slide(args.slide)
            .build()
            .unwrap();
        let ix = create_place_multi_limit_order_ix(params).unwrap();
        let mut data = [0; 128];
        let len = phoenix::write_place_multi_limit_order_data(args, &mut data).unwrap();

        assert_eq!(&data[..len], ix.data.as_slice());
    }

    #[test]
    fn hawkeye_view_margin_cpi_data_matches_builder() {
        let ix = create_hawkeye_view_margin_ix(HawkeyeTraderViewAccounts {
            phoenix_program_id: Pubkey::new_unique(),
            global_config: Pubkey::new_unique(),
            global_trader_index: vec![Pubkey::new_unique()],
            active_trader_buffer: vec![Pubkey::new_unique()],
            perp_asset_map: Pubkey::new_unique(),
            trader: Pubkey::new_unique(),
        });
        let mut data = [0; hawkeye::VIEW_MARGIN_DATA_LEN];
        let len = hawkeye::write_view_margin_data(&mut data).unwrap();

        assert_eq!(&data[..len], ix.data.as_slice());
    }
}
