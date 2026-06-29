//! Delegated trader onboarding instruction construction.

use borsh::{BorshDeserialize, BorshSerialize, to_vec};
use solana_pubkey::Pubkey;

use crate::constants::{PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_LOG_AUTHORITY, PHOENIX_PROGRAM_ID};
use crate::error::PhoenixIxError;
use crate::types::{AccountMeta, Instruction, push_trader_index_accounts};

/// Trader capability flag selected by a delegated capability update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[borsh(use_discriminant = true)]
#[repr(u8)]
enum TraderCapabilityToggleTarget {
    PlaceLimitOrder     = 0,
    PlaceMarketOrder    = 1,
    RiskIncreasingTrade = 2,
    RiskReducingTrade   = 3,
    DepositCollateral   = 4,
    WithdrawCollateral  = 5,
}

/// A single trader capability toggle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
struct TraderCapabilityToggle {
    target: TraderCapabilityToggleTarget,
    enable: bool,
}

impl TraderCapabilityToggle {
    const fn new(target: TraderCapabilityToggleTarget, enable: bool) -> Self {
        Self { target, enable }
    }
}

const ALL_TRADER_CAPABILITY_TOGGLES: [TraderCapabilityToggle; 6] = [
    TraderCapabilityToggle::new(TraderCapabilityToggleTarget::PlaceLimitOrder, true),
    TraderCapabilityToggle::new(TraderCapabilityToggleTarget::PlaceMarketOrder, true),
    TraderCapabilityToggle::new(TraderCapabilityToggleTarget::RiskReducingTrade, true),
    TraderCapabilityToggle::new(TraderCapabilityToggleTarget::RiskIncreasingTrade, true),
    TraderCapabilityToggle::new(TraderCapabilityToggleTarget::DepositCollateral, true),
    TraderCapabilityToggle::new(TraderCapabilityToggleTarget::WithdrawCollateral, true),
];

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
struct TraderCapabilityUpdate {
    toggles: Vec<TraderCapabilityToggle>,
}

/// Parameters for delegated trader onboarding.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OnboardTraderDelegatedParams {
    /// Delegated authority signer.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    authority: Pubkey,
    /// Writable permission account for this delegated authority.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    permission_account: Pubkey,
    /// Trader account whose capability flags will be updated.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader_account: Pubkey,
    /// Global trader index addresses (header + arenas).
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    global_trader_index: Vec<Pubkey>,
    /// Active trader buffer addresses (header + arenas).
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    active_trader_buffer: Vec<Pubkey>,
}

impl OnboardTraderDelegatedParams {
    pub fn builder() -> OnboardTraderDelegatedParamsBuilder {
        OnboardTraderDelegatedParamsBuilder::new()
    }

    pub fn authority(&self) -> Pubkey {
        self.authority
    }

    pub fn permission_account(&self) -> Pubkey {
        self.permission_account
    }

    pub fn trader_account(&self) -> Pubkey {
        self.trader_account
    }

    pub fn global_trader_index(&self) -> &[Pubkey] {
        &self.global_trader_index
    }

    pub fn active_trader_buffer(&self) -> &[Pubkey] {
        &self.active_trader_buffer
    }
}

/// Builder for `OnboardTraderDelegatedParams`.
#[derive(Default)]
pub struct OnboardTraderDelegatedParamsBuilder {
    authority: Option<Pubkey>,
    permission_account: Option<Pubkey>,
    trader_account: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
}

impl OnboardTraderDelegatedParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn authority(mut self, authority: Pubkey) -> Self {
        self.authority = Some(authority);
        self
    }

    pub fn permission_account(mut self, permission_account: Pubkey) -> Self {
        self.permission_account = Some(permission_account);
        self
    }

    pub fn trader_account(mut self, trader_account: Pubkey) -> Self {
        self.trader_account = Some(trader_account);
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

    pub fn build(self) -> Result<OnboardTraderDelegatedParams, PhoenixIxError> {
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

        Ok(OnboardTraderDelegatedParams {
            authority: self
                .authority
                .ok_or(PhoenixIxError::MissingField("authority"))?,
            permission_account: self
                .permission_account
                .ok_or(PhoenixIxError::MissingField("permission_account"))?,
            trader_account: self
                .trader_account
                .ok_or(PhoenixIxError::MissingField("trader_account"))?,
            global_trader_index,
            active_trader_buffer,
        })
    }
}

/// Create a delegated trader onboarding instruction.
pub fn create_onboard_trader_delegated_ix(
    params: OnboardTraderDelegatedParams,
) -> Result<Instruction, PhoenixIxError> {
    let data = encode_onboard_trader_delegated();
    let accounts = build_accounts(&params);

    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts,
        data,
    })
}

fn encode_onboard_trader_delegated() -> Vec<u8> {
    let mut data = Vec::with_capacity(8 + 4 + ALL_TRADER_CAPABILITY_TOGGLES.len() * 2);
    data.extend_from_slice(
        &crate::PhoenixInstruction::SetTraderCapabilitiesDelegated.discriminant(),
    );
    let update = TraderCapabilityUpdate {
        toggles: ALL_TRADER_CAPABILITY_TOGGLES.to_vec(),
    };
    data.extend_from_slice(
        &to_vec(&update).expect("serializing trader capability update should not fail"),
    );
    data
}

fn build_accounts(params: &OnboardTraderDelegatedParams) -> Vec<AccountMeta> {
    let mut accounts = vec![
        // LogAccountGroupAccounts
        AccountMeta::readonly(*PHOENIX_PROGRAM_ID),
        AccountMeta::readonly(*PHOENIX_LOG_AUTHORITY),
        // SetTraderCapabilityInstructionGroupAccounts
        AccountMeta::readonly(*PHOENIX_GLOBAL_CONFIGURATION),
        AccountMeta::readonly_signer(params.authority()),
        AccountMeta::writable(params.permission_account()),
        AccountMeta::writable(params.trader_account()),
    ];

    push_trader_index_accounts(
        &mut accounts,
        params.global_trader_index(),
        params.active_trader_buffer(),
    );

    accounts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_params() -> OnboardTraderDelegatedParams {
        OnboardTraderDelegatedParams::builder()
            .authority(Pubkey::new_unique())
            .permission_account(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique(), Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique(), Pubkey::new_unique()])
            .build()
            .unwrap()
    }

    #[test]
    fn test_create_onboard_trader_delegated_ix() {
        let ix = create_onboard_trader_delegated_ix(build_params()).unwrap();

        assert_eq!(ix.program_id, *PHOENIX_PROGRAM_ID);
        assert_eq!(ix.accounts.len(), 10);
        assert_eq!(
            &ix.data[..8],
            &crate::PhoenixInstruction::SetTraderCapabilitiesDelegated.discriminant()
        );
    }

    #[test]
    fn test_data_encoding() {
        let ix = create_onboard_trader_delegated_ix(build_params()).unwrap();

        assert_eq!(
            &ix.data[8..],
            &[
                6,
                0,
                0,
                0,
                TraderCapabilityToggleTarget::PlaceLimitOrder as u8,
                1,
                TraderCapabilityToggleTarget::PlaceMarketOrder as u8,
                1,
                TraderCapabilityToggleTarget::RiskReducingTrade as u8,
                1,
                TraderCapabilityToggleTarget::RiskIncreasingTrade as u8,
                1,
                TraderCapabilityToggleTarget::DepositCollateral as u8,
                1,
                TraderCapabilityToggleTarget::WithdrawCollateral as u8,
                1,
            ]
        );
    }

    #[test]
    fn test_account_order() {
        let authority = Pubkey::new_unique();
        let permission_account = Pubkey::new_unique();
        let trader_account = Pubkey::new_unique();
        let gti_header = Pubkey::new_unique();
        let gti_arena = Pubkey::new_unique();
        let atb_header = Pubkey::new_unique();
        let atb_arena = Pubkey::new_unique();

        let params = OnboardTraderDelegatedParams::builder()
            .authority(authority)
            .permission_account(permission_account)
            .trader_account(trader_account)
            .global_trader_index(vec![gti_header, gti_arena])
            .active_trader_buffer(vec![atb_header, atb_arena])
            .build()
            .unwrap();
        let ix = create_onboard_trader_delegated_ix(params).unwrap();

        assert_eq!(ix.accounts[0].pubkey, *PHOENIX_PROGRAM_ID);
        assert!(!ix.accounts[0].is_writable);
        assert_eq!(ix.accounts[1].pubkey, *PHOENIX_LOG_AUTHORITY);
        assert!(!ix.accounts[1].is_writable);
        assert_eq!(ix.accounts[2].pubkey, *PHOENIX_GLOBAL_CONFIGURATION);
        assert!(!ix.accounts[2].is_writable);
        assert_eq!(ix.accounts[3].pubkey, authority);
        assert!(ix.accounts[3].is_signer);
        assert!(!ix.accounts[3].is_writable);
        assert_eq!(ix.accounts[4].pubkey, permission_account);
        assert!(ix.accounts[4].is_writable);
        assert_eq!(ix.accounts[5].pubkey, trader_account);
        assert!(ix.accounts[5].is_writable);
        assert_eq!(ix.accounts[6].pubkey, gti_header);
        assert_eq!(ix.accounts[7].pubkey, gti_arena);
        assert_eq!(ix.accounts[8].pubkey, atb_header);
        assert_eq!(ix.accounts[9].pubkey, atb_arena);
    }
}
