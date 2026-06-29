use phoenix_rise::ix;
use phoenix_rise::ix::constants::{
    EMBER_PROGRAM_ID, PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_LOG_AUTHORITY, PHOENIX_PROGRAM_ID,
    SPL_TOKEN_PROGRAM_ID, get_ember_state_address, get_ember_vault_address,
};
use pinocchio::account_info::AccountInfo;
use pinocchio::program_error::ProgramError;
use pinocchio::pubkey::Pubkey;
use pinocchio::{ProgramResult, msg};
use pinocchio_token::state::TokenAccount;

use crate::cpi::{MAX_CPI_ACCOUNTS, check_key, check_program_id, map_ix_error};
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
    pub(crate) global_trader_index: &'a [AccountInfo],
    pub(crate) active_trader_buffer: &'a [AccountInfo],
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
            global_trader_index: &accounts
                [dynamic_accounts_start..dynamic_accounts_start + global_trader_index_count],
            active_trader_buffer: &accounts
                [dynamic_accounts_start + global_trader_index_count..common_end],
        })
    }

    pub(crate) fn validate(&self) -> ProgramResult {
        check_program_id(self.phoenix_program, &PHOENIX_PROGRAM_ID, "Phoenix")?;
        check_program_id(self.hawkeye_program, &ix::HAWKEYE_PROGRAM_ID, "Hawkeye")?;
        check_program_id(self.ember_program, &EMBER_PROGRAM_ID, "Ember")?;
        check_key(self.phoenix_log_authority, &PHOENIX_LOG_AUTHORITY)?;
        check_key(self.phoenix_global_config, &PHOENIX_GLOBAL_CONFIGURATION)?;
        let ember_state = get_ember_state_address().map_err(map_ix_error)?;
        let ember_vault = get_ember_vault_address().map_err(map_ix_error)?;
        check_key(self.ember_state, &ember_state)?;
        check_key(self.ember_vault, &ember_vault)?;
        check_program_id(self.token_program, &SPL_TOKEN_PROGRAM_ID, "SPL Token")?;

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

        let phoenix_withdraw = ix::cpi::phoenix::PhoenixWithdraw {
            phoenix_program: self.phoenix_program,
            log_authority: self.phoenix_log_authority,
            global_config: self.phoenix_global_config,
            trader: self.trader_authority,
            trader_account: self.trader_account,
            perp_asset_map: self.perp_asset_map,
            global_vault: self.global_vault,
            trader_token_account: self.trader_phoenix_token_account,
            token_program: self.token_program,
            global_trader_index: self.global_trader_index,
            active_trader_buffer: self.active_trader_buffer,
            withdraw_queue: self.withdraw_queue,
        };
        let mut scratch = ix::cpi::CpiScratch::<
            { MAX_CPI_ACCOUNTS },
            { ix::cpi::phoenix::PhoenixWithdraw::DATA_LEN },
        >::new(self.phoenix_program);
        phoenix_withdraw.invoke(
            ix::cpi::phoenix::PhoenixWithdrawArgs { amount },
            &mut scratch,
        )?;

        let ember_withdraw = ix::cpi::ember::EmberWithdraw {
            ember_program: self.ember_program,
            trader: self.trader_authority,
            ember_state: self.ember_state,
            usdc_mint: self.usdc_mint,
            canonical_mint: self.canonical_mint,
            trader_usdc_account: self.trader_usdc_token_account,
            trader_phoenix_account: self.trader_phoenix_token_account,
            ember_vault: self.ember_vault,
            token_program: self.token_program,
        };
        let mut scratch = ix::cpi::CpiScratch::<
            { ix::cpi::ember::EmberWithdraw::ACCOUNT_COUNT },
            { ix::cpi::ember::EmberWithdraw::MAX_DATA_LEN },
        >::new(self.ember_program);
        ember_withdraw.invoke(
            ix::cpi::ember::EmberWithdrawArgs {
                amount: Some(amount),
            },
            &mut scratch,
        )
    }

    fn read_withdrawable_collateral(&self) -> Result<u64, ProgramError> {
        let view_margin = ix::cpi::hawkeye::ViewMargin {
            hawkeye_program: self.hawkeye_program,
            phoenix_program: self.phoenix_program,
            global_config: self.phoenix_global_config,
            global_trader_index: self.global_trader_index,
            active_trader_buffer: self.active_trader_buffer,
            perp_asset_map: self.perp_asset_map,
            trader: self.trader_account,
        };
        let mut scratch = ix::cpi::CpiScratch::<
            { MAX_CPI_ACCOUNTS },
            { ix::cpi::hawkeye::ViewMargin::DATA_LEN },
        >::new(self.hawkeye_program);
        let margin = view_margin.invoke_and_decode(&mut scratch).map_err(|_| {
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
