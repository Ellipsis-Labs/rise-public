use bytemuck::{Pod, Zeroable};
use phoenix_rise::ix;
use pinocchio::msg;
use pinocchio::program_error::ProgramError;
use pinocchio::pubkey::Pubkey;

// Local account prefixes keep the demo self-contained and avoid importing
// on-chain program crates. Each view includes only the fields this CPI wrapper
// validates before handing control to Phoenix, Hawkeye, Flight, or Ember.

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct AuthoritySetView {
    _authorities: [Pubkey; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct PhoenixGlobalConfigurationView {
    _discriminant: u64,
    _account_key: Pubkey,
    _current_authorities: AuthoritySetView,
    pub(crate) canonical_token_mint_key: Pubkey,
    pub(crate) global_vault_key: Pubkey,
    pub(crate) perp_asset_map_key: Pubkey,
    _global_trader_index_header_key: Pubkey,
    _active_trader_buffer_header_key: Pubkey,
    _total_quote_lot_fees: u64,
    _unclaimed_quote_lot_fees: u64,
    pub(crate) withdraw_queue_key: Pubkey,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct EmberStateView {
    _discriminant: u64,
    _authority: Pubkey,
    pub(crate) input_mint: Pubkey,
    pub(crate) output_mint: Pubkey,
    pub(crate) phoenix_program: Pubkey,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SequenceNumberView {
    _sequence_number: u64,
    _last_update_slot: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TraderStateView {
    _quote_lot_collateral: i64,
    flags: u32,
    _remaining_header_fields: [u8; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TraderAccountView {
    _discriminant: u64,
    _sequence_number: SequenceNumberView,
    _key: Pubkey,
    _authority: Pubkey,
    state: TraderStateView,
}

impl PhoenixGlobalConfigurationView {
    pub(crate) fn load(data: &[u8]) -> Result<&Self, ProgramError> {
        load_account_view(
            data,
            "Phoenix global configuration",
            ix::compute_discriminant("account:global_configuration"),
        )
    }
}

impl EmberStateView {
    pub(crate) fn load(data: &[u8]) -> Result<&Self, ProgramError> {
        load_account_view(
            data,
            "Ember state",
            ix::compute_discriminant("account:state_account"),
        )
    }
}

pub(crate) fn trader_flags(data: &[u8]) -> Result<u32, ProgramError> {
    let trader = read_view_prefix::<TraderAccountView>(data, "trader account")?;
    Ok(trader.state.flags)
}

fn load_account_view<'a, T: Pod>(
    data: &'a [u8],
    label: &'static str,
    expected_discriminant: [u8; 8],
) -> Result<&'a T, ProgramError> {
    if data.len() < 8 {
        msg!(&format!("close-position-and-withdraw: {label} too small"));
        return Err(ProgramError::InvalidAccountData);
    }
    if data[..8] != expected_discriminant {
        msg!(&format!(
            "close-position-and-withdraw: {label} discriminant mismatch"
        ));
        return Err(ProgramError::InvalidAccountData);
    }
    read_view_prefix(data, label)
}

fn read_view_prefix<'a, T: Pod>(
    data: &'a [u8],
    label: &'static str,
) -> Result<&'a T, ProgramError> {
    let view_len = core::mem::size_of::<T>();
    let Some(prefix) = data.get(..view_len) else {
        msg!(&format!("close-position-and-withdraw: {label} too small"));
        return Err(ProgramError::InvalidAccountData);
    };
    bytemuck::try_from_bytes(prefix).map_err(|_| {
        msg!(&format!(
            "close-position-and-withdraw: {label} layout mismatch"
        ));
        ProgramError::InvalidAccountData
    })
}
