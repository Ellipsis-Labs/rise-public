use serde::Serialize;
use solana_pubkey::Pubkey;

use super::internal::{
    SequenceNumber, ShortEntries, TraderPosition, TraderPositionEntry, TraderState,
    occupied_conditional_order_indices,
};
use super::{AccountDeserialize, AccountDeserializeError};
use crate::trader as borrowed;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Trader {
    pub sequence_number: SequenceNumber,
    pub key: Pubkey,
    pub authority: Pubkey,
    pub state: TraderState,
    pub withdraw_queue_node: Option<u32>,
    pub max_positions: u32,
    pub trader_preference_bits: u32,
    pub trader_preference_flags: borrowed::TraderPreferenceFlags,
    pub preferences: borrowed::TraderPreferences,
    pub disable_collateral_sweep: bool,
    pub disable_position_authority_swap: bool,
    pub position_authority: Pubkey,
    pub num_markets_with_splines: u16,
    pub trader_pda_index: u8,
    pub trader_subaccount_index: u8,
    pub funding_key: Pubkey,
    pub last_deposit_slot: u64,
    pub conditional_order_bits: Vec<u8>,
    pub occupied_conditional_order_indices: Vec<u8>,
    pub native_sol_collateral: u64,
    pub positions: ShortEntries<u64, TraderPosition>,
}

impl AccountDeserialize for Trader {
    fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        Ok(borrowed::Trader::try_from_account_bytes(data)?.into())
    }
}

impl From<borrowed::Trader<'_>> for Trader {
    fn from(trader: borrowed::Trader<'_>) -> Self {
        let header = trader.header();
        let conditional_order_bits = header.conditional_order_bits().to_vec();
        let entries = trader
            .positions()
            .map(|(asset_id, position)| (asset_id, position.to_position().into()))
            .collect::<Vec<_>>();
        Self {
            sequence_number: header.sequence_number().into(),
            key: Pubkey::new_from_array(*header.key()),
            authority: Pubkey::new_from_array(*header.authority()),
            state: (*header.state()).into(),
            withdraw_queue_node: header.withdraw_queue_node(),
            max_positions: header.max_positions(),
            trader_preference_bits: header.trader_preference_bits(),
            trader_preference_flags: header.trader_preference_flags(),
            preferences: header.preferences(),
            disable_collateral_sweep: header.disable_collateral_sweep(),
            disable_position_authority_swap: header.disable_position_authority_swap(),
            position_authority: Pubkey::new_from_array(*header.position_authority()),
            num_markets_with_splines: header.num_markets_with_splines(),
            trader_pda_index: header.trader_pda_index(),
            trader_subaccount_index: header.trader_subaccount_index(),
            funding_key: Pubkey::new_from_array(*header.funding_key()),
            last_deposit_slot: header.last_deposit_slot(),
            occupied_conditional_order_indices: occupied_conditional_order_indices(
                &conditional_order_bits,
            ),
            conditional_order_bits,
            native_sol_collateral: trader.native_sol_collateral(),
            positions: ShortEntries {
                len: entries.len() as u64,
                capacity: trader.capacity(),
                entries,
            },
        }
    }
}

impl Trader {
    pub fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        <Self as AccountDeserialize>::try_from_account_bytes(data)
    }

    pub fn position_entries(&self) -> impl Iterator<Item = TraderPositionEntry> + '_ {
        self.positions
            .entries
            .iter()
            .map(|(asset_id, position)| TraderPositionEntry {
                asset_id: *asset_id,
                position: *position,
            })
    }
}
