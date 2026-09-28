//! Flight: Onboard Trader instruction construction.
//!
//! Mirrors the TS SDK's `buildOnboardTraderIx`
//! (`ts/src/flight/core/ixBuilders/OnboardTrader`).

use solana_pubkey::Pubkey;

use crate::FlightInstruction;
use crate::constants::{
    PHOENIX_PROGRAM_ID, SYSTEM_PROGRAM_ID, phoenix_global_configuration, phoenix_log_authority,
};
use crate::error::PhoenixIxError;
use crate::flight::constants::{
    FLIGHT_PROGRAM_ID, get_flight_builder_state_address, get_flight_global_state_address,
    get_flight_trader_onboarding_authority_address,
    get_flight_trader_onboarding_permission_address,
};
use crate::types::{AccountMeta, Instruction, push_trader_index_accounts};

/// Parameters for onboarding a trader through a Flight builder.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OnboardTraderParams {
    /// The builder's authority (readonly, not a signer).
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    builder_authority: Pubkey,
    /// The builder's onboarder signer (readonly signer).
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    onboarder_signer: Pubkey,
    /// Pays for the new trader account (writable signer).
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    payer: Pubkey,
    /// The trader wallet being onboarded (readonly).
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader_wallet: Pubkey,
    /// Risk authority; derives the onboarding permission.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    risk_authority: Pubkey,
    /// Market authority; derives the fee-update permission.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    market_authority: Pubkey,
    /// Max open positions for the new trader.
    max_positions: u32,
    /// Preference bits for the new trader.
    trader_preference_bits: u32,
    /// Global trader index addresses (header + arenas).
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    global_trader_index: Vec<Pubkey>,
    /// Active trader buffer addresses (header + arenas).
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    active_trader_buffer: Vec<Pubkey>,
}

impl OnboardTraderParams {
    pub fn builder() -> OnboardTraderParamsBuilder {
        OnboardTraderParamsBuilder::new()
    }

    pub fn builder_authority(&self) -> Pubkey {
        self.builder_authority
    }

    pub fn onboarder_signer(&self) -> Pubkey {
        self.onboarder_signer
    }

    pub fn payer(&self) -> Pubkey {
        self.payer
    }

    pub fn trader_wallet(&self) -> Pubkey {
        self.trader_wallet
    }

    pub fn risk_authority(&self) -> Pubkey {
        self.risk_authority
    }

    pub fn market_authority(&self) -> Pubkey {
        self.market_authority
    }

    pub fn max_positions(&self) -> u32 {
        self.max_positions
    }

    pub fn trader_preference_bits(&self) -> u32 {
        self.trader_preference_bits
    }

    pub fn global_trader_index(&self) -> &[Pubkey] {
        &self.global_trader_index
    }

    pub fn active_trader_buffer(&self) -> &[Pubkey] {
        &self.active_trader_buffer
    }
}

/// Builder for `OnboardTraderParams`.
#[derive(Default)]
pub struct OnboardTraderParamsBuilder {
    builder_authority: Option<Pubkey>,
    onboarder_signer: Option<Pubkey>,
    payer: Option<Pubkey>,
    trader_wallet: Option<Pubkey>,
    risk_authority: Option<Pubkey>,
    market_authority: Option<Pubkey>,
    max_positions: Option<u32>,
    trader_preference_bits: Option<u32>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
}

impl OnboardTraderParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builder_authority(mut self, builder_authority: Pubkey) -> Self {
        self.builder_authority = Some(builder_authority);
        self
    }

    pub fn onboarder_signer(mut self, onboarder_signer: Pubkey) -> Self {
        self.onboarder_signer = Some(onboarder_signer);
        self
    }

    pub fn payer(mut self, payer: Pubkey) -> Self {
        self.payer = Some(payer);
        self
    }

    pub fn trader_wallet(mut self, trader_wallet: Pubkey) -> Self {
        self.trader_wallet = Some(trader_wallet);
        self
    }

    pub fn risk_authority(mut self, risk_authority: Pubkey) -> Self {
        self.risk_authority = Some(risk_authority);
        self
    }

    pub fn market_authority(mut self, market_authority: Pubkey) -> Self {
        self.market_authority = Some(market_authority);
        self
    }

    pub fn max_positions(mut self, max_positions: u32) -> Self {
        self.max_positions = Some(max_positions);
        self
    }

    pub fn trader_preference_bits(mut self, trader_preference_bits: u32) -> Self {
        self.trader_preference_bits = Some(trader_preference_bits);
        self
    }

    pub fn global_trader_index(mut self, global_trader_index: Vec<Pubkey>) -> Self {
        self.global_trader_index = Some(global_trader_index);
        self
    }

    pub fn active_trader_buffer(mut self, active_trader_buffer: Vec<Pubkey>) -> Self {
        self.active_trader_buffer = Some(active_trader_buffer);
        self
    }

    pub fn build(self) -> Result<OnboardTraderParams, PhoenixIxError> {
        let global_trader_index = self
            .global_trader_index
            .ok_or(PhoenixIxError::MissingField("global_trader_index"))?;
        if global_trader_index.is_empty() {
            return Err(PhoenixIxError::EmptyGlobalTraderIndex);
        }

        let active_trader_buffer = self
            .active_trader_buffer
            .ok_or(PhoenixIxError::MissingField("active_trader_buffer"))?;
        if active_trader_buffer.is_empty() {
            return Err(PhoenixIxError::EmptyActiveTraderBuffer);
        }

        Ok(OnboardTraderParams {
            builder_authority: self
                .builder_authority
                .ok_or(PhoenixIxError::MissingField("builder_authority"))?,
            onboarder_signer: self
                .onboarder_signer
                .ok_or(PhoenixIxError::MissingField("onboarder_signer"))?,
            payer: self.payer.ok_or(PhoenixIxError::MissingField("payer"))?,
            trader_wallet: self
                .trader_wallet
                .ok_or(PhoenixIxError::MissingField("trader_wallet"))?,
            risk_authority: self
                .risk_authority
                .ok_or(PhoenixIxError::MissingField("risk_authority"))?,
            market_authority: self
                .market_authority
                .ok_or(PhoenixIxError::MissingField("market_authority"))?,
            max_positions: self
                .max_positions
                .ok_or(PhoenixIxError::MissingField("max_positions"))?,
            trader_preference_bits: self
                .trader_preference_bits
                .ok_or(PhoenixIxError::MissingField("trader_preference_bits"))?,
            global_trader_index,
            active_trader_buffer,
        })
    }
}

fn derive_trader_account_address(trader_wallet: &Pubkey) -> Pubkey {
    let (pda, _bump) = Pubkey::find_program_address(
        &[b"trader", trader_wallet.as_ref(), &[0u8, 0u8]],
        &PHOENIX_PROGRAM_ID,
    );
    pda
}

/// Create a Flight `onboard_trader` instruction.
pub fn create_onboard_trader_ix(
    params: OnboardTraderParams,
) -> Result<Instruction, PhoenixIxError> {
    let data = encode_onboard_trader(&params);
    let accounts = build_accounts(&params)?;

    Ok(Instruction {
        program_id: FLIGHT_PROGRAM_ID,
        accounts,
        data,
    })
}

fn encode_onboard_trader(params: &OnboardTraderParams) -> Vec<u8> {
    let mut data = Vec::with_capacity(16);
    data.extend_from_slice(&FlightInstruction::OnboardTrader.discriminant());
    data.extend_from_slice(&params.max_positions().to_le_bytes());
    data.extend_from_slice(&params.trader_preference_bits().to_le_bytes());
    data
}

fn build_accounts(params: &OnboardTraderParams) -> Result<Vec<AccountMeta>, PhoenixIxError> {
    let mut accounts = vec![
        AccountMeta::readonly(get_flight_global_state_address()?),
        AccountMeta::readonly(*PHOENIX_PROGRAM_ID),
        AccountMeta::readonly(params.builder_authority()),
        AccountMeta::writable(get_flight_builder_state_address(
            &params.builder_authority(),
        )?),
        AccountMeta::readonly_signer(params.onboarder_signer()),
        AccountMeta::readonly(get_flight_trader_onboarding_authority_address()?),
        AccountMeta::readonly(phoenix_log_authority()),
        AccountMeta::readonly(phoenix_global_configuration()),
        AccountMeta::writable_signer(params.payer()),
        AccountMeta::readonly(params.trader_wallet()),
        AccountMeta::writable(derive_trader_account_address(&params.trader_wallet())),
        AccountMeta::readonly(SYSTEM_PROGRAM_ID),
        AccountMeta::writable(get_flight_trader_onboarding_permission_address(
            &params.risk_authority(),
        )?),
        AccountMeta::writable(get_flight_trader_onboarding_permission_address(
            &params.market_authority(),
        )?),
    ];

    push_trader_index_accounts(
        &mut accounts,
        params.global_trader_index(),
        params.active_trader_buffer(),
    );

    Ok(accounts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(global_trader_index: Vec<Pubkey>) -> Result<OnboardTraderParams, PhoenixIxError> {
        OnboardTraderParams::builder()
            .builder_authority(Pubkey::new_unique())
            .onboarder_signer(Pubkey::new_unique())
            .payer(Pubkey::new_unique())
            .trader_wallet(Pubkey::new_unique())
            .risk_authority(Pubkey::new_unique())
            .market_authority(Pubkey::new_unique())
            .max_positions(8)
            .trader_preference_bits(0)
            .global_trader_index(global_trader_index)
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .build()
    }

    #[test]
    fn test_empty_global_trader_index() {
        assert!(matches!(
            params(vec![]),
            Err(PhoenixIxError::EmptyGlobalTraderIndex)
        ));
    }

    #[test]
    fn test_account_order() {
        let gti = Pubkey::new_unique();
        let params = params(vec![gti]).unwrap();
        let expected = [
            (get_flight_global_state_address().unwrap(), false, false),
            (*PHOENIX_PROGRAM_ID, false, false),
            (params.builder_authority(), false, false),
            (
                get_flight_builder_state_address(&params.builder_authority()).unwrap(),
                true,
                false,
            ),
            (params.onboarder_signer(), false, true),
            (
                get_flight_trader_onboarding_authority_address().unwrap(),
                false,
                false,
            ),
            (phoenix_log_authority(), false, false),
            (phoenix_global_configuration(), false, false),
            (params.payer(), true, true),
            (params.trader_wallet(), false, false),
            (
                derive_trader_account_address(&params.trader_wallet()),
                true,
                false,
            ),
            (SYSTEM_PROGRAM_ID, false, false),
            (
                get_flight_trader_onboarding_permission_address(&params.risk_authority()).unwrap(),
                true,
                false,
            ),
            (
                get_flight_trader_onboarding_permission_address(&params.market_authority())
                    .unwrap(),
                true,
                false,
            ),
            (gti, true, false),
            (params.active_trader_buffer()[0], true, false),
        ];
        let ix = create_onboard_trader_ix(params).unwrap();
        let actual: Vec<_> = ix
            .accounts
            .iter()
            .map(|a| (a.pubkey, a.is_writable, a.is_signer))
            .collect();
        assert_eq!(actual, expected);
    }
}
