//! Permission account instruction construction.

use borsh::{BorshSerialize, to_vec};
use solana_pubkey::Pubkey;

use crate::ix::constants::{
    PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_LOG_AUTHORITY, PHOENIX_PROGRAM_ID, SYSTEM_PROGRAM_ID,
    create_permission_discriminant, set_permission_delegated_discriminant,
};
use crate::ix::error::PhoenixIxError;
use crate::ix::types::{AccountMeta, Instruction};

/// Permission bit for delegated trader onboarding.
///
/// Accounts with this permission can call `SetTraderCapabilitiesDelegated` for
/// traders under the configured permission authority.
pub const TRADER_ONBOARDING_PERMISSION: u64 = 1 << 4;

/// Permission bit for delegated trader management.
///
/// Accounts with this permission can use `SetPermissionDelegated` to grant or
/// revoke trader-onboarding permission budgets for other delegated keys.
pub const TRADER_MANAGEMENT_PERMISSION: u64 = 1 << 7;

/// Parameters for creating a permission account PDA.
#[derive(Debug, Clone)]
pub struct CreatePermissionParams {
    /// Signer that pays rent for the permission account.
    payer: Pubkey,
    /// Permission authority stored in the PDA, usually the current Phoenix
    /// risk authority for trader-onboarding grants.
    permission_authority: Pubkey,
    /// Delegated key stored in the PDA.
    delegated_key: Pubkey,
    /// Permission PDA derived from `(permission_authority, delegated_key)`.
    permission_account: Pubkey,
}

impl CreatePermissionParams {
    pub fn builder() -> CreatePermissionParamsBuilder {
        CreatePermissionParamsBuilder::new()
    }

    pub fn payer(&self) -> Pubkey {
        self.payer
    }

    pub fn permission_authority(&self) -> Pubkey {
        self.permission_authority
    }

    pub fn delegated_key(&self) -> Pubkey {
        self.delegated_key
    }

    pub fn permission_account(&self) -> Pubkey {
        self.permission_account
    }
}

/// Builder for `CreatePermissionParams`.
#[derive(Default)]
pub struct CreatePermissionParamsBuilder {
    payer: Option<Pubkey>,
    permission_authority: Option<Pubkey>,
    delegated_key: Option<Pubkey>,
    permission_account: Option<Pubkey>,
}

impl CreatePermissionParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn payer(mut self, payer: Pubkey) -> Self {
        self.payer = Some(payer);
        self
    }

    pub fn permission_authority(mut self, permission_authority: Pubkey) -> Self {
        self.permission_authority = Some(permission_authority);
        self
    }

    pub fn delegated_key(mut self, delegated_key: Pubkey) -> Self {
        self.delegated_key = Some(delegated_key);
        self
    }

    pub fn permission_account(mut self, permission_account: Pubkey) -> Self {
        self.permission_account = Some(permission_account);
        self
    }

    pub fn build(self) -> Result<CreatePermissionParams, PhoenixIxError> {
        Ok(CreatePermissionParams {
            payer: self.payer.ok_or(PhoenixIxError::MissingField("payer"))?,
            permission_authority: self
                .permission_authority
                .ok_or(PhoenixIxError::MissingField("permission_authority"))?,
            delegated_key: self
                .delegated_key
                .ok_or(PhoenixIxError::MissingField("delegated_key"))?,
            permission_account: self
                .permission_account
                .ok_or(PhoenixIxError::MissingField("permission_account"))?,
        })
    }
}

/// Parameters for delegated permission updates.
#[derive(Debug, Clone)]
pub struct SetPermissionDelegatedParams {
    /// Signer that is either the permission authority itself or a delegated key
    /// with trader-management permission.
    authority: Pubkey,
    /// Writable permission account that authorizes `authority`.
    ///
    /// When `authority` is the permission authority itself, pass a writable
    /// placeholder account owned by a non-Phoenix program, commonly
    /// `authority`.
    authority_permission_account: Pubkey,
    /// Writable permission account being granted or revoked.
    target_permission_account: Pubkey,
    /// Permission authority for the target PDA.
    permission_authority: Pubkey,
    /// Delegated key for the target PDA.
    delegated_key: Pubkey,
    /// Permission mask to set on the target account.
    permission: u64,
    /// Number of signer actions to grant. `None` means unlimited.
    allowed_signer_actions: Option<i64>,
}

impl SetPermissionDelegatedParams {
    pub fn builder() -> SetPermissionDelegatedParamsBuilder {
        SetPermissionDelegatedParamsBuilder::new()
    }

    pub fn authority(&self) -> Pubkey {
        self.authority
    }

    pub fn authority_permission_account(&self) -> Pubkey {
        self.authority_permission_account
    }

    pub fn target_permission_account(&self) -> Pubkey {
        self.target_permission_account
    }

    pub fn permission_authority(&self) -> Pubkey {
        self.permission_authority
    }

    pub fn delegated_key(&self) -> Pubkey {
        self.delegated_key
    }

    pub fn permission(&self) -> u64 {
        self.permission
    }

    pub fn allowed_signer_actions(&self) -> Option<i64> {
        self.allowed_signer_actions
    }
}

/// Builder for `SetPermissionDelegatedParams`.
#[derive(Default)]
pub struct SetPermissionDelegatedParamsBuilder {
    authority: Option<Pubkey>,
    authority_permission_account: Option<Pubkey>,
    target_permission_account: Option<Pubkey>,
    permission_authority: Option<Pubkey>,
    delegated_key: Option<Pubkey>,
    permission: Option<u64>,
    allowed_signer_actions: Option<Option<i64>>,
}

impl SetPermissionDelegatedParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn authority(mut self, authority: Pubkey) -> Self {
        self.authority = Some(authority);
        self
    }

    pub fn authority_permission_account(mut self, authority_permission_account: Pubkey) -> Self {
        self.authority_permission_account = Some(authority_permission_account);
        self
    }

    pub fn target_permission_account(mut self, target_permission_account: Pubkey) -> Self {
        self.target_permission_account = Some(target_permission_account);
        self
    }

    pub fn permission_authority(mut self, permission_authority: Pubkey) -> Self {
        self.permission_authority = Some(permission_authority);
        self
    }

    pub fn delegated_key(mut self, delegated_key: Pubkey) -> Self {
        self.delegated_key = Some(delegated_key);
        self
    }

    pub fn permission(mut self, permission: u64) -> Self {
        self.permission = Some(permission);
        self
    }

    pub fn allowed_signer_actions(mut self, allowed_signer_actions: Option<i64>) -> Self {
        self.allowed_signer_actions = Some(allowed_signer_actions);
        self
    }

    pub fn build(self) -> Result<SetPermissionDelegatedParams, PhoenixIxError> {
        Ok(SetPermissionDelegatedParams {
            authority: self
                .authority
                .ok_or(PhoenixIxError::MissingField("authority"))?,
            authority_permission_account: self
                .authority_permission_account
                .ok_or(PhoenixIxError::MissingField("authority_permission_account"))?,
            target_permission_account: self
                .target_permission_account
                .ok_or(PhoenixIxError::MissingField("target_permission_account"))?,
            permission_authority: self
                .permission_authority
                .ok_or(PhoenixIxError::MissingField("permission_authority"))?,
            delegated_key: self
                .delegated_key
                .ok_or(PhoenixIxError::MissingField("delegated_key"))?,
            permission: self
                .permission
                .ok_or(PhoenixIxError::MissingField("permission"))?,
            allowed_signer_actions: self
                .allowed_signer_actions
                .ok_or(PhoenixIxError::MissingField("allowed_signer_actions"))?,
        })
    }
}

#[derive(Debug, Clone, BorshSerialize)]
struct SetPermissionData {
    permission: u64,
    expires_at_timestamp: Option<i64>,
    allowed_signer_actions: Option<i64>,
}

/// Create a permission account PDA.
pub fn create_create_permission_ix(
    params: CreatePermissionParams,
) -> Result<Instruction, PhoenixIxError> {
    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts: vec![
            AccountMeta::readonly(*PHOENIX_PROGRAM_ID),
            AccountMeta::readonly(*PHOENIX_LOG_AUTHORITY),
            AccountMeta::writable_signer(params.payer()),
            AccountMeta::writable(params.permission_account()),
            AccountMeta::readonly(params.permission_authority()),
            AccountMeta::readonly(params.delegated_key()),
            AccountMeta::readonly(SYSTEM_PROGRAM_ID),
        ],
        data: create_permission_discriminant().to_vec(),
    })
}

/// Create a delegated permission update instruction.
pub fn create_set_permission_delegated_ix(
    params: SetPermissionDelegatedParams,
) -> Result<Instruction, PhoenixIxError> {
    validate_set_permission_delegated(&params)?;

    let mut data = set_permission_delegated_discriminant().to_vec();
    data.extend_from_slice(
        &to_vec(&SetPermissionData {
            permission: params.permission(),
            expires_at_timestamp: None,
            allowed_signer_actions: params.allowed_signer_actions(),
        })
        .expect("serializing set permission data should not fail"),
    );

    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts: vec![
            AccountMeta::readonly(*PHOENIX_PROGRAM_ID),
            AccountMeta::readonly(*PHOENIX_LOG_AUTHORITY),
            AccountMeta::readonly(*PHOENIX_GLOBAL_CONFIGURATION),
            AccountMeta::readonly_signer(params.authority()),
            AccountMeta::writable(params.authority_permission_account()),
            AccountMeta::writable(params.target_permission_account()),
            AccountMeta::readonly(params.permission_authority()),
            AccountMeta::readonly(params.delegated_key()),
        ],
        data,
    })
}

fn validate_set_permission_delegated(
    params: &SetPermissionDelegatedParams,
) -> Result<(), PhoenixIxError> {
    if params.permission() == 0 {
        return Err(PhoenixIxError::MissingField(
            "permission must contain at least one bit",
        ));
    }
    if matches!(params.allowed_signer_actions(), Some(actions) if actions <= 0) {
        return Err(PhoenixIxError::MissingField(
            "allowed_signer_actions must be positive or None",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_permission_accounts_match_processor_order() {
        let payer = Pubkey::new_unique();
        let permission_authority = Pubkey::new_unique();
        let delegated_key = Pubkey::new_unique();
        let permission_account = Pubkey::new_unique();

        let params = CreatePermissionParams::builder()
            .payer(payer)
            .permission_authority(permission_authority)
            .delegated_key(delegated_key)
            .permission_account(permission_account)
            .build()
            .unwrap();
        let ix = create_create_permission_ix(params).unwrap();

        assert_eq!(ix.accounts[2], AccountMeta::writable_signer(payer));
        assert_eq!(ix.accounts[3], AccountMeta::writable(permission_account));
        assert_eq!(ix.accounts[4], AccountMeta::readonly(permission_authority));
        assert_eq!(ix.accounts[5], AccountMeta::readonly(delegated_key));
        assert_eq!(ix.data, create_permission_discriminant().to_vec());
    }

    #[test]
    fn set_permission_delegated_accounts_match_processor_order() {
        let authority = Pubkey::new_unique();
        let authority_permission_account = Pubkey::new_unique();
        let target_permission_account = Pubkey::new_unique();
        let permission_authority = Pubkey::new_unique();
        let delegated_key = Pubkey::new_unique();

        let params = SetPermissionDelegatedParams::builder()
            .authority(authority)
            .authority_permission_account(authority_permission_account)
            .target_permission_account(target_permission_account)
            .permission_authority(permission_authority)
            .delegated_key(delegated_key)
            .permission(TRADER_ONBOARDING_PERMISSION)
            .allowed_signer_actions(Some(5))
            .build()
            .unwrap();
        let ix = create_set_permission_delegated_ix(params).unwrap();

        assert_eq!(ix.accounts[3], AccountMeta::readonly_signer(authority));
        assert_eq!(
            ix.accounts[4],
            AccountMeta::writable(authority_permission_account)
        );
        assert_eq!(
            ix.accounts[5],
            AccountMeta::writable(target_permission_account)
        );
        assert_eq!(ix.accounts[6], AccountMeta::readonly(permission_authority));
        assert_eq!(ix.accounts[7], AccountMeta::readonly(delegated_key));
        let discriminant = set_permission_delegated_discriminant();
        assert_eq!(&ix.data[..8], discriminant.as_slice());
    }
}
