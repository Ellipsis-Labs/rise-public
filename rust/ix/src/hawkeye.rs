//! Phoenix Hawkeye instruction builders and return-data decoding.

use core::mem::size_of;

use bytemuck::{try_from_bytes, Pod, Zeroable};
use solana_pubkey::Pubkey;
use thiserror::Error;

use crate::ix::types::{AccountMeta, Instruction};
use crate::math::sha2_const;

/// Phoenix Hawkeye program ID.
pub const HAWKEYE_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("RiSeVw3ZjNfsaXPRb4mgaqYaEEt41pNNJoDvVh7pgQj");

/// Default fee payer for unsigned Hawkeye simulations.
pub const HAWKEYE_SIMULATION_FEE_PAYER: Pubkey =
    solana_pubkey::pubkey!("9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM");

/// Default compute-unit limit used by Hawkeye simulation helpers.
pub const HAWKEYE_SIMULATION_COMPUTE_UNIT_LIMIT: u32 = 1_400_000;

/// Current Hawkeye return-data version.
pub const HAWKEYE_RETURN_VERSION: u16 = 1;

pub const VIEW_MARGIN_RETURN_MAGIC: u64 = sha2_const(b"return:phoenix_hawkeye_margin");
pub const VIEW_ASSET_RETURN_MAGIC: u64 = sha2_const(b"return:phoenix_hawkeye_asset");
pub const VIEW_LIQUIDATION_PRICE_RETURN_MAGIC: u64 =
    sha2_const(b"return:phoenix_hawkeye_liquidation_price");
pub const VIEW_BBO_RETURN_MAGIC: u64 = sha2_const(b"return:phoenix_hawkeye_bbo");
pub const VIEW_FUNDING_RETURN_MAGIC: u64 = sha2_const(b"return:phoenix_hawkeye_funding");

pub const VIEW_BBO_HAS_BID: u8 = 1 << 0;
pub const VIEW_BBO_HAS_ASK: u8 = 1 << 1;
pub const VIEW_FUNDING_HAS_ACCUMULATED: u8 = 1 << 0;
pub const VIEW_FUNDING_HAS_UNSETTLED: u8 = 1 << 1;

#[repr(u64)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PhoenixHawkeyeInstruction {
    ViewMargin = sha2_const(b"global:view_margin"),
    ViewMarginForAsset = sha2_const(b"global:view_margin_for_asset"),
    ViewLiquidationPrice = sha2_const(b"global:view_liquidation_price"),
    ViewBbo = sha2_const(b"global:view_bbo"),
    ViewFunding = sha2_const(b"global:view_funding"),
}

impl PhoenixHawkeyeInstruction {
    pub fn from_tag(tag: u64) -> Option<Self> {
        match tag {
            x if x == Self::ViewMargin as u64 => Some(Self::ViewMargin),
            x if x == Self::ViewMarginForAsset as u64 => Some(Self::ViewMarginForAsset),
            x if x == Self::ViewLiquidationPrice as u64 => Some(Self::ViewLiquidationPrice),
            x if x == Self::ViewBbo as u64 => Some(Self::ViewBbo),
            x if x == Self::ViewFunding as u64 => Some(Self::ViewFunding),
            _ => None,
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Pod, Zeroable)]
pub struct ViewAssetParams {
    pub asset_id: u32,
    pub _padding: [u8; 4],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Pod, Zeroable)]
pub struct ViewMarginReturn {
    pub magic: u64,
    pub version: u16,
    pub position_count: u16,
    pub risk_state: u8,
    pub risk_tier: u8,
    pub is_liquidatable: u8,
    pub _padding: u8,
    pub collateral_quote_lots: i64,
    pub effective_collateral_quote_lots: i64,
    pub free_collateral_quote_lots: i64,
    pub withdrawable_collateral_quote_lots: u64,
    pub initial_margin_quote_lots: u64,
    pub maintenance_margin_quote_lots: u64,
    pub cancel_margin_quote_lots: u64,
    pub backstop_margin_quote_lots: u64,
    pub high_risk_margin_quote_lots: u64,
    pub unrealized_pnl_quote_lots: i64,
    pub discounted_unrealized_pnl_quote_lots: i64,
    pub unsettled_funding_quote_lots: i64,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Pod, Zeroable)]
pub struct ViewAssetReturn {
    pub magic: u64,
    pub asset_id: u32,
    pub version: u16,
    pub has_position_or_orders: u8,
    pub _padding: u8,
    pub risk_state: u8,
    pub risk_tier: u8,
    pub _padding1: [u8; 6],
    pub base_lots: i64,
    pub virtual_quote_lots: i64,
    pub mark_price_ticks: u64,
    pub entry_price_quote_lots_per_base_lot: u64,
    pub position_value_quote_lots: i64,
    pub unrealized_pnl_quote_lots: i64,
    pub discounted_unrealized_pnl_quote_lots: i64,
    pub unsettled_funding_quote_lots: i64,
    pub initial_margin_quote_lots: u64,
    pub maintenance_margin_quote_lots: u64,
    pub cancel_margin_quote_lots: u64,
    pub backstop_margin_quote_lots: u64,
    pub high_risk_margin_quote_lots: u64,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Pod, Zeroable)]
pub struct ViewLiquidationPriceReturn {
    pub magic: u64,
    pub asset_id: u32,
    pub version: u16,
    pub status: u8,
    pub side: u8,
    pub liquidation_price_ticks: u64,
    pub mark_price_ticks: u64,
    pub entry_price_quote_lots_per_base_lot: u64,
    pub effective_collateral_quote_lots: i64,
    pub maintenance_margin_quote_lots: u64,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Pod, Zeroable)]
pub struct ViewBboReturn {
    pub magic: u64,
    pub version: u16,
    pub flags: u8,
    pub _padding: [u8; 5],
    pub best_bid_ticks: u64,
    pub best_ask_ticks: u64,
    pub mark_price_ticks: u64,
    pub index_price_ticks: u64,
    pub mark_price_last_updated_slot: u64,
    pub index_price_last_updated_slot: u64,
}

impl ViewBboReturn {
    pub fn best_bid_ticks(&self) -> Option<u64> {
        ((self.flags & VIEW_BBO_HAS_BID) != 0).then_some(self.best_bid_ticks)
    }

    pub fn best_ask_ticks(&self) -> Option<u64> {
        ((self.flags & VIEW_BBO_HAS_ASK) != 0).then_some(self.best_ask_ticks)
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Pod, Zeroable)]
pub struct ViewFundingReturn {
    pub magic: u64,
    pub asset_id: u32,
    pub version: u16,
    pub flags: u8,
    pub _padding: u8,
    pub total_accumulated_funding_quote_lots: i64,
    pub total_unsettled_funding_quote_lots: i64,
    pub current_funding_rate_micro_bps: i64,
    pub projected_1h_funding_rate_micro_bps: i64,
}

impl ViewFundingReturn {
    pub fn total_accumulated_funding_quote_lots(&self) -> Option<i64> {
        ((self.flags & VIEW_FUNDING_HAS_ACCUMULATED) != 0)
            .then_some(self.total_accumulated_funding_quote_lots)
    }

    pub fn total_unsettled_funding_quote_lots(&self) -> Option<i64> {
        ((self.flags & VIEW_FUNDING_HAS_UNSETTLED) != 0)
            .then_some(self.total_unsettled_funding_quote_lots)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum HawkeyeReturnData {
    Margin(ViewMarginReturn),
    Asset(ViewAssetReturn),
    LiquidationPrice(ViewLiquidationPriceReturn),
    Bbo(ViewBboReturn),
    Funding(ViewFundingReturn),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum HawkeyeReturnDataError {
    #[error("Hawkeye return data is too short: got {actual} bytes, expected at least 8")]
    MissingMagic { actual: usize },

    #[error("Unknown Hawkeye return data magic {magic}")]
    UnknownMagic { magic: u64 },

    #[error(
        "Invalid Hawkeye return data length for {kind}: got {actual} bytes, expected {expected}"
    )]
    InvalidLength {
        kind: &'static str,
        actual: usize,
        expected: usize,
    },

    #[error("Invalid Hawkeye return data layout for {kind}")]
    InvalidLayout { kind: &'static str },
}

pub fn decode_hawkeye_return_data(
    bytes: &[u8],
) -> Result<HawkeyeReturnData, HawkeyeReturnDataError> {
    if bytes.len() < size_of::<u64>() {
        return Err(HawkeyeReturnDataError::MissingMagic {
            actual: bytes.len(),
        });
    }

    let magic = u64::from_le_bytes(bytes[..size_of::<u64>()].try_into().expect("u64 slice"));
    match magic {
        VIEW_MARGIN_RETURN_MAGIC => {
            decode_typed(bytes, "view_margin").map(HawkeyeReturnData::Margin)
        }
        VIEW_ASSET_RETURN_MAGIC => {
            decode_typed(bytes, "view_margin_for_asset").map(HawkeyeReturnData::Asset)
        }
        VIEW_LIQUIDATION_PRICE_RETURN_MAGIC => {
            decode_typed(bytes, "view_liquidation_price").map(HawkeyeReturnData::LiquidationPrice)
        }
        VIEW_BBO_RETURN_MAGIC => decode_typed(bytes, "view_bbo").map(HawkeyeReturnData::Bbo),
        VIEW_FUNDING_RETURN_MAGIC => {
            decode_typed(bytes, "view_funding").map(HawkeyeReturnData::Funding)
        }
        magic => Err(HawkeyeReturnDataError::UnknownMagic { magic }),
    }
}

pub fn decode_hawkeye_return<T: Pod + Copy>(
    bytes: &[u8],
    kind: &'static str,
) -> Result<T, HawkeyeReturnDataError> {
    decode_typed(bytes, kind)
}

fn decode_typed<T: Pod + Copy>(
    bytes: &[u8],
    kind: &'static str,
) -> Result<T, HawkeyeReturnDataError> {
    if bytes.len() != size_of::<T>() {
        return Err(HawkeyeReturnDataError::InvalidLength {
            kind,
            actual: bytes.len(),
            expected: size_of::<T>(),
        });
    }

    try_from_bytes::<T>(bytes)
        .copied()
        .map_err(|_| HawkeyeReturnDataError::InvalidLayout { kind })
}

#[derive(Debug, Clone)]
pub struct HawkeyeTraderViewAccounts {
    pub phoenix_program_id: Pubkey,
    pub global_config: Pubkey,
    pub global_trader_index: Vec<Pubkey>,
    pub active_trader_buffer: Vec<Pubkey>,
    pub perp_asset_map: Pubkey,
    pub trader: Pubkey,
}

#[derive(Debug, Clone)]
pub struct HawkeyeBboViewAccounts {
    pub phoenix_program_id: Pubkey,
    pub global_config: Pubkey,
    pub global_trader_index: Vec<Pubkey>,
    pub active_trader_buffer: Vec<Pubkey>,
    pub perp_asset_map: Pubkey,
    pub orderbook: Pubkey,
    pub spline_collection: Pubkey,
}

pub fn create_hawkeye_view_margin_ix(accounts: HawkeyeTraderViewAccounts) -> Instruction {
    hawkeye_trader_instruction(PhoenixHawkeyeInstruction::ViewMargin, &[], accounts)
}

pub fn create_hawkeye_view_margin_for_asset_ix(
    accounts: HawkeyeTraderViewAccounts,
    asset_id: u32,
) -> Instruction {
    let params = ViewAssetParams {
        asset_id,
        _padding: [0; 4],
    };
    hawkeye_trader_instruction(
        PhoenixHawkeyeInstruction::ViewMarginForAsset,
        bytemuck::bytes_of(&params),
        accounts,
    )
}

pub fn create_hawkeye_view_liquidation_price_ix(
    accounts: HawkeyeTraderViewAccounts,
    asset_id: u32,
) -> Instruction {
    let params = ViewAssetParams {
        asset_id,
        _padding: [0; 4],
    };
    hawkeye_trader_instruction(
        PhoenixHawkeyeInstruction::ViewLiquidationPrice,
        bytemuck::bytes_of(&params),
        accounts,
    )
}

pub fn create_hawkeye_view_funding_ix(
    accounts: HawkeyeTraderViewAccounts,
    asset_id: u32,
) -> Instruction {
    let params = ViewAssetParams {
        asset_id,
        _padding: [0; 4],
    };
    hawkeye_trader_instruction(
        PhoenixHawkeyeInstruction::ViewFunding,
        bytemuck::bytes_of(&params),
        accounts,
    )
}

pub fn create_hawkeye_view_bbo_ix(accounts: HawkeyeBboViewAccounts) -> Instruction {
    let mut account_metas = Vec::with_capacity(
        5 + accounts.global_trader_index.len() + accounts.active_trader_buffer.len(),
    );
    push_hawkeye_base_accounts(
        &mut account_metas,
        accounts.phoenix_program_id,
        accounts.global_config,
        &accounts.global_trader_index,
        &accounts.active_trader_buffer,
        accounts.perp_asset_map,
    );
    account_metas.push(AccountMeta::readonly(accounts.orderbook));
    account_metas.push(AccountMeta::readonly(accounts.spline_collection));
    hawkeye_instruction(PhoenixHawkeyeInstruction::ViewBbo, &[], account_metas)
}

fn hawkeye_trader_instruction(
    instruction: PhoenixHawkeyeInstruction,
    params: &[u8],
    accounts: HawkeyeTraderViewAccounts,
) -> Instruction {
    let mut account_metas = Vec::with_capacity(
        4 + accounts.global_trader_index.len() + accounts.active_trader_buffer.len(),
    );
    push_hawkeye_base_accounts(
        &mut account_metas,
        accounts.phoenix_program_id,
        accounts.global_config,
        &accounts.global_trader_index,
        &accounts.active_trader_buffer,
        accounts.perp_asset_map,
    );
    account_metas.push(AccountMeta::readonly(accounts.trader));
    hawkeye_instruction(instruction, params, account_metas)
}

fn push_hawkeye_base_accounts(
    account_metas: &mut Vec<AccountMeta>,
    phoenix_program_id: Pubkey,
    global_config: Pubkey,
    global_trader_index: &[Pubkey],
    active_trader_buffer: &[Pubkey],
    perp_asset_map: Pubkey,
) {
    account_metas.push(AccountMeta::readonly(phoenix_program_id));
    account_metas.push(AccountMeta::readonly(global_config));
    account_metas.extend(
        global_trader_index
            .iter()
            .copied()
            .map(AccountMeta::readonly),
    );
    account_metas.extend(
        active_trader_buffer
            .iter()
            .copied()
            .map(AccountMeta::readonly),
    );
    account_metas.push(AccountMeta::readonly(perp_asset_map));
}

fn hawkeye_instruction(
    instruction: PhoenixHawkeyeInstruction,
    params: &[u8],
    accounts: Vec<AccountMeta>,
) -> Instruction {
    let mut data = Vec::with_capacity(size_of::<u64>() + params.len());
    data.extend_from_slice(&(instruction as u64).to_le_bytes());
    data.extend_from_slice(params);

    Instruction {
        program_id: HAWKEYE_PROGRAM_ID,
        accounts,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_bbo_options() {
        let ret = ViewBboReturn {
            magic: VIEW_BBO_RETURN_MAGIC,
            version: HAWKEYE_RETURN_VERSION,
            flags: VIEW_BBO_HAS_BID,
            best_bid_ticks: 99,
            best_ask_ticks: 101,
            ..ViewBboReturn::default()
        };

        assert_eq!(ret.best_bid_ticks(), Some(99));
        assert_eq!(ret.best_ask_ticks(), None);
        assert_eq!(
            decode_hawkeye_return_data(bytemuck::bytes_of(&ret)).unwrap(),
            HawkeyeReturnData::Bbo(ret)
        );
    }

    #[test]
    fn decodes_funding_options() {
        let ret = ViewFundingReturn {
            magic: VIEW_FUNDING_RETURN_MAGIC,
            version: HAWKEYE_RETURN_VERSION,
            flags: VIEW_FUNDING_HAS_UNSETTLED,
            total_accumulated_funding_quote_lots: -10,
            total_unsettled_funding_quote_lots: -20,
            ..ViewFundingReturn::default()
        };

        assert_eq!(ret.total_accumulated_funding_quote_lots(), None);
        assert_eq!(ret.total_unsettled_funding_quote_lots(), Some(-20));
        assert_eq!(
            decode_hawkeye_return_data(bytemuck::bytes_of(&ret)).unwrap(),
            HawkeyeReturnData::Funding(ret)
        );
    }
}
