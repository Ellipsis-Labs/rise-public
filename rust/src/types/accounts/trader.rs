use solana_pubkey::Pubkey;

use super::discriminants::ACCOUNT_DISCRIMINANTS;
use super::internal::{
    CONDITIONAL_ORDER_BITS_LEN, Reader, SequenceNumber, ShortEntries, TraderPosition,
    TraderPositionEntry, TraderState, none_if_zero, occupied_conditional_order_indices,
    read_sequence_number, read_trader_position, read_trader_state, verify_discriminant,
};
use super::{AccountDeserialize, AccountDeserializeError};

const ACCOUNT: &str = "Trader";
const POSITION_ENTRY_BYTES: usize = 40;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trader {
    pub sequence_number: SequenceNumber,
    pub key: Pubkey,
    pub authority: Pubkey,
    pub state: TraderState,
    pub withdraw_queue_node: Option<u32>,
    pub max_positions: u64,
    pub position_authority: Pubkey,
    pub num_markets_with_splines: u16,
    pub trader_pda_index: u8,
    pub trader_subaccount_index: u8,
    pub funding_key: Pubkey,
    pub last_deposit_slot: u64,
    pub conditional_order_bits: Vec<u8>,
    pub occupied_conditional_order_indices: Vec<u8>,
    pub positions: ShortEntries<u64, TraderPosition>,
}

impl AccountDeserialize for Trader {
    fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
        verify_discriminant(ACCOUNT, data, ACCOUNT_DISCRIMINANTS.trader)?;
        let mut reader = Reader::with_offset(ACCOUNT, data, 8);
        let sequence_number = read_sequence_number(&mut reader)?;
        let key = reader.read_pubkey()?;
        let authority = reader.read_pubkey()?;
        let state = read_trader_state(&mut reader)?;
        reader.skip(4)?;
        let withdraw_queue_node = none_if_zero(reader.read_u32()?);
        let max_positions = reader.read_u64()?;
        let position_authority = reader.read_pubkey()?;
        let num_markets_with_splines = reader.read_u16()?;
        let trader_pda_index = reader.read_u8()?;
        let trader_subaccount_index = reader.read_u8()?;
        let funding_key = reader.read_pubkey()?;
        reader.skip(4)?;
        let last_deposit_slot = reader.read_u64()?;
        let conditional_order_bits = reader.read_bytes(CONDITIONAL_ORDER_BITS_LEN)?.to_vec();
        let occupied_conditional_order_indices =
            occupied_conditional_order_indices(&conditional_order_bits);
        let positions = read_positions(&mut reader)?;
        Ok(Self {
            sequence_number,
            key,
            authority,
            state,
            withdraw_queue_node,
            max_positions,
            position_authority,
            num_markets_with_splines,
            trader_pda_index,
            trader_subaccount_index,
            funding_key,
            last_deposit_slot,
            conditional_order_bits,
            occupied_conditional_order_indices,
            positions,
        })
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

fn read_positions(
    reader: &mut Reader<'_>,
) -> Result<ShortEntries<u64, TraderPosition>, AccountDeserializeError> {
    let len = reader.read_u64()?;
    let capacity = reader.read_u64()?;
    let bytes_needed = (len as usize)
        .checked_mul(POSITION_ENTRY_BYTES)
        .ok_or_else(|| {
            AccountDeserializeError::invalid_data(ACCOUNT, "position map length overflow")
        })?;
    if bytes_needed > reader.remaining() {
        return Err(AccountDeserializeError::too_short(
            ACCOUNT,
            reader.offset() + bytes_needed,
            reader.offset() + reader.remaining(),
        ));
    }
    let mut entries = Vec::with_capacity(len as usize);
    for _ in 0..len {
        let key = reader.read_u64()?;
        let value = read_trader_position(reader)?;
        entries.push((key, value));
    }
    Ok(ShortEntries {
        len,
        capacity,
        entries,
    })
}
