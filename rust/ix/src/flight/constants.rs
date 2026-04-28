use solana_pubkey::Pubkey;

use crate::ix::{PHOENIX_PROGRAM_ID, compute_discriminant};

pub const FLIGHT_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("F1ightu9cujFYo34k9CabifLrJT8qzfDVM2Q7BqhJn2W");

pub fn flight_register_builder_discriminant() -> [u8; 8] {
    compute_discriminant("global:register_builder")
}

pub fn flight_update_fee_discriminant() -> [u8; 8] {
    compute_discriminant("global:update_fee")
}

pub fn flight_proxy_instruction_discriminant() -> [u8; 8] {
    compute_discriminant("global:proxy_instruction")
}

/// Derives the global state PDA for the Flight program.
///
/// Seeds: [phoenix_program_id, "global_state"] against Flight program
pub fn get_flight_global_state_address() -> Pubkey {
    let (pda, _bump) = Pubkey::find_program_address(
        &[PHOENIX_PROGRAM_ID.as_ref(), b"global_state"],
        &FLIGHT_PROGRAM_ID,
    );
    pda
}

/// Derives the builder state PDA for the Flight program.
///
/// Seeds: [phoenix_program_id, authority, "builder_state"] against Flight
/// program
pub fn get_flight_builder_state_address(authority: &Pubkey) -> Pubkey {
    let (pda, _bump) = Pubkey::find_program_address(
        &[
            PHOENIX_PROGRAM_ID.as_ref(),
            authority.as_ref(),
            b"builder_state",
        ],
        &FLIGHT_PROGRAM_ID,
    );
    pda
}
