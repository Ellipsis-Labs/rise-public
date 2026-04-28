//! Cancel stop loss instruction construction.
//!
//! Cancels an active stop-loss or take-profit order for a given market and
//! execution direction. If both directions are inactive after cancellation,
//! the on-chain account is closed and rent is reclaimed to the funder.

use solana_pubkey::Pubkey;

use crate::ix::constants::{
    PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_LOG_AUTHORITY, PHOENIX_PROGRAM_ID, SYSTEM_PROGRAM_ID,
    cancel_stop_loss_discriminant, get_stop_loss_address,
};
use crate::ix::error::PhoenixIxError;
use crate::ix::types::{AccountMeta, Direction, Instruction};

/// Parameters for cancelling a stop loss order.
#[derive(Debug, Clone)]
pub struct CancelStopLossParams {
    funder: Pubkey,
    trader_account: Pubkey,
    position_authority: Pubkey,
    asset_id: u64,
    execution_direction: Direction,
}

impl CancelStopLossParams {
    pub fn builder() -> CancelStopLossParamsBuilder {
        CancelStopLossParamsBuilder::new()
    }

    pub fn funder(&self) -> Pubkey {
        self.funder
    }

    pub fn trader_account(&self) -> Pubkey {
        self.trader_account
    }

    pub fn position_authority(&self) -> Pubkey {
        self.position_authority
    }

    pub fn asset_id(&self) -> u64 {
        self.asset_id
    }

    pub fn execution_direction(&self) -> Direction {
        self.execution_direction
    }
}

/// Builder for `CancelStopLossParams`.
#[derive(Default)]
pub struct CancelStopLossParamsBuilder {
    funder: Option<Pubkey>,
    trader_account: Option<Pubkey>,
    position_authority: Option<Pubkey>,
    asset_id: Option<u64>,
    execution_direction: Option<Direction>,
}

impl CancelStopLossParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn funder(mut self, funder: Pubkey) -> Self {
        self.funder = Some(funder);
        self
    }

    pub fn trader_account(mut self, trader_account: Pubkey) -> Self {
        self.trader_account = Some(trader_account);
        self
    }

    pub fn position_authority(mut self, position_authority: Pubkey) -> Self {
        self.position_authority = Some(position_authority);
        self
    }

    pub fn asset_id(mut self, asset_id: u64) -> Self {
        self.asset_id = Some(asset_id);
        self
    }

    pub fn execution_direction(mut self, execution_direction: Direction) -> Self {
        self.execution_direction = Some(execution_direction);
        self
    }

    pub fn build(self) -> Result<CancelStopLossParams, PhoenixIxError> {
        Ok(CancelStopLossParams {
            funder: self.funder.ok_or(PhoenixIxError::MissingField("funder"))?,
            trader_account: self
                .trader_account
                .ok_or(PhoenixIxError::MissingField("trader_account"))?,
            position_authority: self
                .position_authority
                .ok_or(PhoenixIxError::MissingField("position_authority"))?,
            asset_id: self
                .asset_id
                .ok_or(PhoenixIxError::MissingField("asset_id"))?,
            execution_direction: self
                .execution_direction
                .ok_or(PhoenixIxError::MissingField("execution_direction"))?,
        })
    }
}

/// Create a cancel stop loss instruction.
pub fn create_cancel_stop_loss_ix(
    params: CancelStopLossParams,
) -> Result<Instruction, PhoenixIxError> {
    let data = encode_cancel_stop_loss(&params);
    let accounts = build_accounts(&params);

    Ok(Instruction {
        program_id: PHOENIX_PROGRAM_ID,
        accounts,
        data,
    })
}

fn encode_cancel_stop_loss(params: &CancelStopLossParams) -> Vec<u8> {
    let mut data = Vec::with_capacity(9);

    // 8 bytes: discriminant
    data.extend_from_slice(&cancel_stop_loss_discriminant());
    // 1 byte: execution_direction
    data.push(params.execution_direction as u8);

    data
}

fn build_accounts(params: &CancelStopLossParams) -> Vec<AccountMeta> {
    let stop_loss_pda = get_stop_loss_address(&params.trader_account, params.asset_id);

    let mut accounts = Vec::new();

    // LogAccountGroupAccounts
    accounts.push(AccountMeta::readonly(PHOENIX_PROGRAM_ID));
    accounts.push(AccountMeta::readonly(PHOENIX_LOG_AUTHORITY));

    // CancelStopLossInstructionGroupAccounts
    accounts.push(AccountMeta::readonly(PHOENIX_GLOBAL_CONFIGURATION));
    accounts.push(AccountMeta::writable_signer(params.funder));
    accounts.push(AccountMeta::readonly(params.trader_account));
    accounts.push(AccountMeta::readonly_signer(params.position_authority));
    accounts.push(AccountMeta::writable(stop_loss_pda));
    accounts.push(AccountMeta::readonly(SYSTEM_PROGRAM_ID));

    accounts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_params() -> CancelStopLossParams {
        CancelStopLossParams::builder()
            .funder(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .position_authority(Pubkey::new_unique())
            .asset_id(1)
            .execution_direction(Direction::LessThan)
            .build()
            .unwrap()
    }

    #[test]
    fn test_create_cancel_stop_loss_ix() {
        let params = test_params();
        let ix = create_cancel_stop_loss_ix(params).unwrap();

        assert_eq!(ix.program_id, PHOENIX_PROGRAM_ID);
        // 2 log + 6 (global_config, funder, trader, authority, stop_loss, system) = 8
        assert_eq!(ix.accounts.len(), 8);
        assert_eq!(&ix.data[..8], &cancel_stop_loss_discriminant());
    }

    #[test]
    fn test_cancel_stop_loss_data_encoding() {
        let params = test_params();
        let data = encode_cancel_stop_loss(&params);

        // 8 discriminant + 1 direction = 9
        assert_eq!(data.len(), 9);
        assert_eq!(data[8], Direction::LessThan as u8);
    }

    #[test]
    fn test_cancel_stop_loss_account_positions() {
        let params = test_params();
        let accounts = build_accounts(&params);

        // Position 0: program id (readonly)
        assert_eq!(accounts[0].pubkey, PHOENIX_PROGRAM_ID);
        assert!(!accounts[0].is_signer);
        assert!(!accounts[0].is_writable);

        // Position 1: log authority (readonly)
        assert_eq!(accounts[1].pubkey, PHOENIX_LOG_AUTHORITY);
        assert!(!accounts[1].is_signer);

        // Position 2: global config (readonly)
        assert_eq!(accounts[2].pubkey, PHOENIX_GLOBAL_CONFIGURATION);
        assert!(!accounts[2].is_writable);

        // Position 3: funder (writable signer)
        assert_eq!(accounts[3].pubkey, params.funder);
        assert!(accounts[3].is_signer);
        assert!(accounts[3].is_writable);

        // Position 4: trader_account (readonly)
        assert_eq!(accounts[4].pubkey, params.trader_account);
        assert!(!accounts[4].is_writable);

        // Position 5: position_authority (readonly signer)
        assert_eq!(accounts[5].pubkey, params.position_authority);
        assert!(accounts[5].is_signer);
        assert!(!accounts[5].is_writable);

        // Position 6: stop_loss_account (writable)
        let expected_sl_pda = get_stop_loss_address(&params.trader_account, params.asset_id);
        assert_eq!(accounts[6].pubkey, expected_sl_pda);
        assert!(accounts[6].is_writable);

        // Position 7: system_program (readonly)
        assert_eq!(accounts[7].pubkey, SYSTEM_PROGRAM_ID);
        assert!(!accounts[7].is_signer);
        assert!(!accounts[7].is_writable);
    }

    #[test]
    fn test_builder_missing_required_field() {
        let result = CancelStopLossParams::builder()
            .funder(Pubkey::new_unique())
            .build();
        assert!(matches!(result, Err(PhoenixIxError::MissingField(_))));
    }
}
