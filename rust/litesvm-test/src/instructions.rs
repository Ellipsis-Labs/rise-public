use borsh::{BorshSerialize, to_vec};
use phoenix_rise_ix::constants::{PHOENIX_PROGRAM_ID, SYSTEM_PROGRAM_ID, compute_discriminant};
use phoenix_rise_ix::discriminants::PhoenixInstruction;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use crate::fixture::{FixtureOraclePriceUpdate, FixturePrice};

#[derive(BorshSerialize)]
struct FixtureSymbol {
    symbol_bytes: [u8; 16],
}

impl FixtureSymbol {
    fn new(symbol: &str) -> Self {
        let bytes = symbol.as_bytes();
        assert!(
            !bytes.is_empty() && bytes.len() <= 16,
            "fixture symbol must be between 1 and 16 bytes"
        );
        assert!(
            bytes.iter().all(|byte| byte.is_ascii() && *byte != 0),
            "fixture symbol must be non-null ASCII"
        );
        let mut symbol_bytes = [0_u8; 16];
        symbol_bytes[..bytes.len()].copy_from_slice(bytes);
        Self { symbol_bytes }
    }
}

#[derive(BorshSerialize)]
struct SerializedOraclePriceUpdate {
    perp_asset_id: FixtureSymbol,
    new_exchange_perp_price: Option<FixturePrice>,
    new_exchange_spot_price: FixturePrice,
}

#[derive(BorshSerialize)]
struct SerializedOraclePriceUpdateFlags {
    flags: u8,
}

#[derive(BorshSerialize)]
struct SerializedOraclePriceUpdateInstruction {
    update_timestamp: u64,
    updates: Vec<SerializedOraclePriceUpdate>,
    flags: SerializedOraclePriceUpdateFlags,
}

#[derive(BorshSerialize)]
struct SerializedUpdateSplinePriceParams {
    new_mid_price: u64,
    user_update_slot: Option<u64>,
    refresh_regions: bool,
}

pub(crate) fn oracle_price_update_data(
    update_timestamp: u64,
    updates: &[FixtureOraclePriceUpdate],
) -> Vec<u8> {
    let payload = SerializedOraclePriceUpdateInstruction {
        update_timestamp,
        updates: updates
            .iter()
            .map(|update| SerializedOraclePriceUpdate {
                perp_asset_id: FixtureSymbol::new(&update.symbol),
                new_exchange_perp_price: update.new_exchange_perp_price,
                new_exchange_spot_price: update.new_exchange_spot_price,
            })
            .collect(),
        flags: SerializedOraclePriceUpdateFlags { flags: 0 },
    };
    let mut data = PhoenixInstruction::UpdateOraclePricesWithOrdering
        .discriminant()
        .to_vec();
    data.extend_from_slice(&to_vec(&payload).expect("oracle price update should serialize"));
    data
}

pub(crate) fn spline_price_update_data(new_mid_price_ticks: u64) -> Vec<u8> {
    let payload = SerializedUpdateSplinePriceParams {
        new_mid_price: new_mid_price_ticks,
        user_update_slot: None,
        refresh_regions: true,
    };
    let mut data = PhoenixInstruction::UpdateSplinePrice
        .discriminant()
        .to_vec();
    data.extend_from_slice(&to_vec(&payload).expect("spline price update should serialize"));
    data
}

pub(crate) fn flight_init_ix(payer: Pubkey, max_fee_cap_bps: u64) -> Instruction {
    let mut data = compute_discriminant("global:init").to_vec();
    data.extend_from_slice(&max_fee_cap_bps.to_le_bytes());
    Instruction {
        program_id: phoenix_rise_ix::flight::FLIGHT_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(
                phoenix_rise_ix::flight::get_flight_global_state_address().unwrap(),
                false,
            ),
            AccountMeta::new_readonly(*PHOENIX_PROGRAM_ID, false),
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}
