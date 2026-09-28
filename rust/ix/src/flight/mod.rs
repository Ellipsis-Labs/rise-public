mod constants;
mod onboard_trader;
mod proxy_instruction;
mod register_builder;
mod set_onboarder_signer;
mod update_fee;

pub use constants::{
    FLIGHT_PROGRAM_ID, flight_builder_state_account_discriminant,
    flight_global_state_account_discriminant, flight_proxy_instruction_discriminant,
    flight_proxy_instruction_with_fee_override_discriminant, flight_register_builder_discriminant,
    flight_update_fee_discriminant, get_flight_authorized_collateral_transfer_permission_address,
    get_flight_builder_state_address, get_flight_collateral_transfer_authority_address,
    get_flight_global_state_address, get_flight_trader_onboarding_authority_address,
    get_flight_trader_onboarding_permission_address,
};
pub use onboard_trader::{
    OnboardTraderParams, OnboardTraderParamsBuilder, create_onboard_trader_ix,
};
pub use proxy_instruction::{
    ProxyInstructionParams, ProxyInstructionParamsBuilder, create_proxy_instruction_ix,
};
pub use register_builder::{
    RegisterBuilderParams, RegisterBuilderParamsBuilder, create_register_builder_ix,
};
pub use set_onboarder_signer::{
    SetOnboarderSignerParams, SetOnboarderSignerParamsBuilder, create_set_onboarder_signer_ix,
};
pub use update_fee::{UpdateFeeParams, UpdateFeeParamsBuilder, create_update_fee_ix};

pub use crate::{FlightAccount, FlightInstruction};
