use phoenix_rise::ix;
use pinocchio::account_info::AccountInfo;
use pinocchio::cpi::get_return_data;
use pinocchio::program_error::ProgramError;
use pinocchio::pubkey::Pubkey;
use pinocchio::{ProgramResult, msg};
use pinocchio_token::state::TokenAccount;

use crate::cpi::{
    check_key, check_program_id, invoke_rise_ix, map_ix_error, to_solana_pubkey, to_solana_pubkeys,
};
use crate::state_views::{EmberStateView, PhoenixGlobalConfigurationView};

const TRADER_FLAG_HOT: u32 = 1 << 0;

pub(crate) struct SharedAccounts<'a> {
    pub(crate) trader_authority: &'a AccountInfo,
    pub(crate) phoenix_program: &'a AccountInfo,
    pub(crate) hawkeye_program: &'a AccountInfo,
    pub(crate) ember_program: &'a AccountInfo,
    pub(crate) phoenix_log_authority: &'a AccountInfo,
    pub(crate) phoenix_global_config: &'a AccountInfo,
    pub(crate) trader_account: &'a AccountInfo,
    pub(crate) perp_asset_map: &'a AccountInfo,
    pub(crate) global_vault: &'a AccountInfo,
    pub(crate) trader_phoenix_token_account: &'a AccountInfo,
    pub(crate) withdraw_queue: &'a AccountInfo,
    pub(crate) usdc_mint: &'a AccountInfo,
    pub(crate) canonical_mint: &'a AccountInfo,
    pub(crate) trader_usdc_token_account: &'a AccountInfo,
    pub(crate) ember_state: &'a AccountInfo,
    pub(crate) ember_vault: &'a AccountInfo,
    pub(crate) token_program: &'a AccountInfo,
    pub(crate) global_trader_index: Vec<&'a AccountInfo>,
    pub(crate) active_trader_buffer: Vec<&'a AccountInfo>,
}

impl<'a> SharedAccounts<'a> {
    pub(crate) const FIXED_ACCOUNT_COUNT: usize = 17;

    pub(crate) fn load(
        accounts: &'a [AccountInfo],
        dynamic_accounts_start: usize,
        global_trader_index_count: usize,
        active_trader_buffer_count: usize,
    ) -> Result<Self, ProgramError> {
        let common_end = dynamic_accounts_start
            .checked_add(global_trader_index_count)
            .and_then(|count| count.checked_add(active_trader_buffer_count))
            .ok_or(ProgramError::InvalidInstructionData)?;
        if accounts.len() < common_end {
            msg!("close-position-and-withdraw: invalid account count");
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        Ok(Self {
            trader_authority: &accounts[0],
            phoenix_program: &accounts[1],
            hawkeye_program: &accounts[2],
            ember_program: &accounts[3],
            phoenix_log_authority: &accounts[4],
            phoenix_global_config: &accounts[5],
            trader_account: &accounts[6],
            perp_asset_map: &accounts[7],
            global_vault: &accounts[8],
            trader_phoenix_token_account: &accounts[9],
            withdraw_queue: &accounts[10],
            usdc_mint: &accounts[11],
            canonical_mint: &accounts[12],
            trader_usdc_token_account: &accounts[13],
            ember_state: &accounts[14],
            ember_vault: &accounts[15],
            token_program: &accounts[16],
            global_trader_index: accounts
                [dynamic_accounts_start..dynamic_accounts_start + global_trader_index_count]
                .iter()
                .collect(),
            active_trader_buffer: accounts
                [dynamic_accounts_start + global_trader_index_count..common_end]
                .iter()
                .collect(),
        })
    }

    pub(crate) fn validate(&self) -> ProgramResult {
        check_program_id(self.phoenix_program, &ix::PHOENIX_PROGRAM_ID, "Phoenix")?;
        check_program_id(self.hawkeye_program, &ix::HAWKEYE_PROGRAM_ID, "Hawkeye")?;
        check_program_id(self.ember_program, &ix::EMBER_PROGRAM_ID, "Ember")?;
        check_key(self.phoenix_log_authority, &ix::PHOENIX_LOG_AUTHORITY)?;
        check_key(
            self.phoenix_global_config,
            &ix::PHOENIX_GLOBAL_CONFIGURATION,
        )?;
        check_key(self.ember_state, &ix::get_ember_state_address())?;
        check_key(self.ember_vault, &ix::get_ember_vault_address())?;
        check_program_id(self.token_program, &ix::SPL_TOKEN_PROGRAM_ID, "SPL Token")?;

        if !self.trader_authority.is_signer() {
            msg!("close-position-and-withdraw: trader authority must sign");
            return Err(ProgramError::MissingRequiredSignature);
        }

        // These early checks keep the demo's account contract explicit. Phoenix
        // withdraw and Ember withdraw repeat the canonical mint/vault and
        // wrapper mint checks inside their CPI handlers.
        self.validate_ember_state()?;
        self.validate_phoenix_global_configuration()?;

        validate_token_account(
            self.trader_phoenix_token_account,
            self.canonical_mint.key(),
            Some(self.trader_authority.key()),
            "trader Phoenix token account",
        )?;
        validate_token_account(
            self.trader_usdc_token_account,
            self.usdc_mint.key(),
            Some(self.trader_authority.key()),
            "trader USDC token account",
        )?;

        Ok(())
    }

    fn validate_ember_state(&self) -> ProgramResult {
        let data = self
            .ember_state
            .try_borrow_data()
            .map_err(|_| ProgramError::AccountBorrowFailed)?;
        let state = EmberStateView::load(&data)?;

        check_pubkey(
            &state.phoenix_program,
            self.phoenix_program.key(),
            "Ember state Phoenix program",
        )?;

        check_pubkey(&state.input_mint, self.usdc_mint.key(), "Ember input mint")?;

        check_pubkey(
            &state.output_mint,
            self.canonical_mint.key(),
            "Ember output mint",
        )
    }

    fn validate_phoenix_global_configuration(&self) -> ProgramResult {
        let data = self
            .phoenix_global_config
            .try_borrow_data()
            .map_err(|_| ProgramError::AccountBorrowFailed)?;
        let global_config = PhoenixGlobalConfigurationView::load(&data)?;

        check_pubkey(
            &global_config.canonical_token_mint_key,
            self.canonical_mint.key(),
            "Phoenix canonical mint",
        )?;

        check_pubkey(
            &global_config.global_vault_key,
            self.global_vault.key(),
            "Phoenix global vault",
        )?;

        check_pubkey(
            &global_config.perp_asset_map_key,
            self.perp_asset_map.key(),
            "Phoenix perp asset map",
        )?;

        check_pubkey(
            &global_config.withdraw_queue_key,
            self.withdraw_queue.key(),
            "Phoenix withdraw queue",
        )
    }

    pub(crate) fn log_trader_status(&self) -> ProgramResult {
        let flags = self.trader_flags()?;
        if flags & TRADER_FLAG_HOT != 0 {
            msg!(&format!(
                "close-position-and-withdraw: trader is hot; flags={flags}"
            ));
        } else {
            msg!(&format!(
                "close-position-and-withdraw: trader is cold; flags={flags}"
            ));
        }
        Ok(())
    }

    pub(crate) fn read_and_log_withdrawable_collateral(
        &self,
        label: &str,
    ) -> Result<u64, ProgramError> {
        let amount = self.read_withdrawable_collateral()?;
        msg!(&format!(
            "close-position-and-withdraw: withdrawable collateral {label}: {amount}"
        ));
        Ok(amount)
    }

    pub(crate) fn withdraw(&self, amount: u64) -> ProgramResult {
        msg!(&format!(
            "close-position-and-withdraw: collateral amount to withdraw: {amount}"
        ));

        if amount == 0 {
            msg!("close-position-and-withdraw: no collateral to withdraw");
            return Ok(());
        }

        let withdraw_ix = self.build_phoenix_withdraw_ix(amount)?;
        invoke_rise_ix(
            &withdraw_ix,
            self.phoenix_program,
            &self.phoenix_withdraw_account_infos(),
        )?;

        let ember_ix = self.build_ember_withdraw_ix(amount)?;
        invoke_rise_ix(
            &ember_ix,
            self.ember_program,
            &self.ember_withdraw_account_infos(),
        )
    }

    fn read_withdrawable_collateral(&self) -> Result<u64, ProgramError> {
        let hawkeye_ix = ix::create_hawkeye_view_margin_ix(ix::HawkeyeTraderViewAccounts {
            phoenix_program_id: to_solana_pubkey(self.phoenix_program.key()),
            global_config: to_solana_pubkey(self.phoenix_global_config.key()),
            global_trader_index: to_solana_pubkeys(&self.global_trader_index),
            active_trader_buffer: to_solana_pubkeys(&self.active_trader_buffer),
            perp_asset_map: to_solana_pubkey(self.perp_asset_map.key()),
            trader: to_solana_pubkey(self.trader_account.key()),
        });
        invoke_rise_ix(
            &hawkeye_ix,
            self.hawkeye_program,
            &self.hawkeye_account_infos(),
        )?;

        let return_data = get_return_data().ok_or_else(|| {
            msg!("close-position-and-withdraw: missing Hawkeye return data");
            ProgramError::InvalidAccountData
        })?;
        if return_data.program_id() != &ix::HAWKEYE_PROGRAM_ID.to_bytes() {
            msg!("close-position-and-withdraw: unexpected return-data program");
            return Err(ProgramError::InvalidAccountData);
        }
        let margin = ix::decode_hawkeye_return::<ix::ViewMarginReturn>(
            return_data.as_slice(),
            "view_margin",
        )
        .map_err(|_| {
            msg!("close-position-and-withdraw: invalid Hawkeye margin return data");
            ProgramError::InvalidAccountData
        })?;
        let free_collateral = margin.free_collateral_quote_lots.max(0) as u64;
        Ok(free_collateral.min(margin.withdrawable_collateral_quote_lots))
    }

    fn trader_flags(&self) -> Result<u32, ProgramError> {
        let data = self
            .trader_account
            .try_borrow_data()
            .map_err(|_| ProgramError::AccountBorrowFailed)?;
        crate::state_views::trader_flags(&data)
    }

    fn build_phoenix_withdraw_ix(&self, amount: u64) -> Result<ix::Instruction, ProgramError> {
        let params = ix::WithdrawFundsParams::builder()
            .trader(to_solana_pubkey(self.trader_authority.key()))
            .trader_account(to_solana_pubkey(self.trader_account.key()))
            .perp_asset_map(to_solana_pubkey(self.perp_asset_map.key()))
            .global_vault(to_solana_pubkey(self.global_vault.key()))
            .trader_token_account(to_solana_pubkey(self.trader_phoenix_token_account.key()))
            .global_trader_index(to_solana_pubkeys(&self.global_trader_index))
            .active_trader_buffer(to_solana_pubkeys(&self.active_trader_buffer))
            .withdraw_queue(to_solana_pubkey(self.withdraw_queue.key()))
            .amount(amount)
            .build()
            .map_err(map_ix_error)?;
        ix::create_withdraw_funds_ix(params).map_err(map_ix_error)
    }

    fn build_ember_withdraw_ix(&self, amount: u64) -> Result<ix::Instruction, ProgramError> {
        let params = ix::EmberWithdrawParams::builder()
            .trader(to_solana_pubkey(self.trader_authority.key()))
            .usdc_mint(to_solana_pubkey(self.usdc_mint.key()))
            .canonical_mint(to_solana_pubkey(self.canonical_mint.key()))
            .trader_usdc_account(to_solana_pubkey(self.trader_usdc_token_account.key()))
            .trader_phoenix_account(to_solana_pubkey(self.trader_phoenix_token_account.key()))
            .amount(Some(amount))
            .build()
            .map_err(map_ix_error)?;
        ix::create_ember_withdraw_ix(params).map_err(map_ix_error)
    }

    fn hawkeye_account_infos(&self) -> Vec<&'a AccountInfo> {
        let mut account_infos = Vec::with_capacity(
            4 + self.global_trader_index.len() + self.active_trader_buffer.len(),
        );
        account_infos.extend([self.phoenix_program, self.phoenix_global_config]);
        account_infos.extend(self.global_trader_index.iter().copied());
        account_infos.extend(self.active_trader_buffer.iter().copied());
        account_infos.extend([self.perp_asset_map, self.trader_account]);
        account_infos
    }

    fn phoenix_withdraw_account_infos(&self) -> Vec<&'a AccountInfo> {
        let mut account_infos = Vec::with_capacity(
            10 + self.global_trader_index.len() + self.active_trader_buffer.len(),
        );
        account_infos.extend([
            self.phoenix_program,
            self.phoenix_log_authority,
            self.phoenix_global_config,
            self.trader_authority,
            self.trader_account,
            self.perp_asset_map,
            self.global_vault,
            self.trader_phoenix_token_account,
            self.token_program,
        ]);
        account_infos.extend(self.global_trader_index.iter().copied());
        account_infos.extend(self.active_trader_buffer.iter().copied());
        account_infos.push(self.withdraw_queue);
        account_infos
    }

    fn ember_withdraw_account_infos(&self) -> [&'a AccountInfo; 8] {
        [
            self.trader_authority,
            self.ember_state,
            self.usdc_mint,
            self.canonical_mint,
            self.trader_usdc_token_account,
            self.trader_phoenix_token_account,
            self.ember_vault,
            self.token_program,
        ]
    }
}

fn check_pubkey(actual: &Pubkey, expected: &Pubkey, label: &'static str) -> ProgramResult {
    if actual != expected {
        msg!(&format!("close-position-and-withdraw: {label} mismatch"));
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

fn validate_token_account(
    account: &AccountInfo,
    expected_mint: &Pubkey,
    expected_owner: Option<&Pubkey>,
    label: &'static str,
) -> ProgramResult {
    let token_account = TokenAccount::from_account_info(account).map_err(|_| {
        msg!(label);
        ProgramError::InvalidAccountData
    })?;
    if token_account.mint() != expected_mint {
        msg!(&format!(
            "close-position-and-withdraw: {label} mint mismatch"
        ));
        return Err(ProgramError::InvalidAccountData);
    }
    if let Some(expected_owner) = expected_owner {
        if token_account.owner() != expected_owner {
            msg!(&format!(
                "close-position-and-withdraw: {label} owner mismatch"
            ));
            return Err(ProgramError::IllegalOwner);
        }
    }
    Ok(())
}
