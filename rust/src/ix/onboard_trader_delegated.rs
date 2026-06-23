//! Delegated trader onboarding instruction construction.

use borsh::{BorshDeserialize, BorshSerialize, to_vec};
use solana_pubkey::Pubkey;

use crate::ix::constants::{
    PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_LOG_AUTHORITY, PHOENIX_PROGRAM_ID,
    set_trader_capabilities_delegated_discriminant,
};
use crate::ix::error::PhoenixIxError;
use crate::ix::types::{AccountMeta, Instruction};

/// HOT participation bit for a trader in the global trader index.
pub const TRADER_CAPABILITY_HOT: u32 = 1 << 0;
/// Enables resting limit order placement for hot traders.
pub const TRADER_CAPABILITY_CAN_PLACE_LIMIT: u32 = 1 << 1;
/// Enables market order placement and risk-reducing order flows.
pub const TRADER_CAPABILITY_CAN_PLACE_MARKET: u32 = 1 << 2;
/// Enables risk-increasing trades.
pub const TRADER_CAPABILITY_CAN_RISK_INCREASE: u32 = 1 << 3;
/// Enables collateral deposits.
pub const TRADER_CAPABILITY_CAN_DEPOSIT: u32 = 1 << 4;
/// Enables collateral withdrawals.
pub const TRADER_CAPABILITY_CAN_WITHDRAW: u32 = 1 << 5;

/// Capability mask needed before a trader can place market orders, deposit,
/// and withdraw.
pub const REQUIRED_TRADER_CAPABILITIES: u32 = TRADER_CAPABILITY_CAN_PLACE_MARKET
    | TRADER_CAPABILITY_CAN_DEPOSIT
    | TRADER_CAPABILITY_CAN_WITHDRAW;

/// Returns true when the trader has the capabilities required for market
/// orders, deposits, and withdrawals.
#[inline(always)]
pub const fn is_trader_ready(flags: u32) -> bool {
    flags & REQUIRED_TRADER_CAPABILITIES == REQUIRED_TRADER_CAPABILITIES
}

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
pub struct OnboardTraderDelegatedParams {
    /// Delegated authority signer.
    authority: Pubkey,
    /// Writable permission account for this delegated authority.
    permission_account: Pubkey,
    /// Trader account whose capability flags will be updated.
    trader_account: Pubkey,
    /// Global trader index addresses (header + arenas).
    global_trader_index: Vec<Pubkey>,
    /// Active trader buffer addresses (header + arenas).
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
    data.extend_from_slice(&set_trader_capabilities_delegated_discriminant());
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

    for addr in params.global_trader_index() {
        accounts.push(AccountMeta::writable(*addr));
    }
    for addr in params.active_trader_buffer() {
        accounts.push(AccountMeta::writable(*addr));
    }

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
            &set_trader_capabilities_delegated_discriminant()
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
