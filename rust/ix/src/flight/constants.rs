use solana_pubkey::Pubkey;

use crate::constants::get_permission_address;
use crate::{FlightAccount, FlightInstruction, PHOENIX_PROGRAM_ID, PhoenixIxError};

pub const FLIGHT_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("F1ightu9cujFYo34k9CabifLrJT8qzfDVM2Q7BqhJn2W");

const COLLATERAL_TRANSFER_AUTHORITY_SEED: &[u8] = b"collateral_transfer_authority";

pub fn flight_register_builder_discriminant() -> [u8; 8] {
    FlightInstruction::RegisterBuilder.discriminant()
}

pub fn flight_update_fee_discriminant() -> [u8; 8] {
    FlightInstruction::UpdateFee.discriminant()
}

pub fn flight_proxy_instruction_discriminant() -> [u8; 8] {
    FlightInstruction::ProxyInstruction.discriminant()
}

pub fn flight_proxy_instruction_with_fee_override_discriminant() -> [u8; 8] {
    FlightInstruction::ProxyInstructionWithFeeOverride.discriminant()
}

pub fn flight_global_state_account_discriminant() -> [u8; 8] {
    FlightAccount::GlobalState.discriminant()
}

pub fn flight_builder_state_account_discriminant() -> [u8; 8] {
    FlightAccount::BuilderState.discriminant()
}

/// Derives the global state PDA for the Flight program.
///
/// Seeds: [PHOENIX_PROGRAM_ID, "global_state"] against Flight program
pub fn get_flight_global_state_address() -> Result<Pubkey, PhoenixIxError> {
    let program_id = *PHOENIX_PROGRAM_ID;
    derive_program_address(&[program_id.as_ref(), b"global_state"], &FLIGHT_PROGRAM_ID)
}

/// Derives the builder state PDA for the Flight program.
///
/// Seeds: [PHOENIX_PROGRAM_ID, authority, "builder_state"] against Flight
/// program
pub fn get_flight_builder_state_address(authority: &Pubkey) -> Result<Pubkey, PhoenixIxError> {
    let program_id = *PHOENIX_PROGRAM_ID;
    derive_program_address(
        &[program_id.as_ref(), authority.as_ref(), b"builder_state"],
        &FLIGHT_PROGRAM_ID,
    )
}

/// Derives the Flight PDA that can be granted authorized collateral transfer
/// permission by Phoenix root authority.
///
/// Seeds: ["collateral_transfer_authority", PHOENIX_PROGRAM_ID] against Flight
/// program.
pub fn get_flight_collateral_transfer_authority_address() -> Result<Pubkey, PhoenixIxError> {
    derive_program_address(
        &[
            COLLATERAL_TRANSFER_AUTHORITY_SEED,
            PHOENIX_PROGRAM_ID.as_ref(),
        ],
        &FLIGHT_PROGRAM_ID,
    )
}

/// Derives the Phoenix permission PDA that allows Flight's collateral-transfer
/// PDA to collect builder fees for delegated market orders.
///
/// Seeds: ["permission", root_authority, flight_collateral_transfer_authority]
/// against the Phoenix program.
pub fn get_flight_authorized_collateral_transfer_permission_address(
    root_authority: &Pubkey,
) -> Result<Pubkey, PhoenixIxError> {
    let collateral_transfer_authority = get_flight_collateral_transfer_authority_address()?;
    get_permission_address(root_authority, &collateral_transfer_authority)
}

fn derive_program_address(seeds: &[&[u8]], program_id: &Pubkey) -> Result<Pubkey, PhoenixIxError> {
    let (pda, _bump) = Pubkey::find_program_address(seeds, program_id);
    Ok(pda)
}
