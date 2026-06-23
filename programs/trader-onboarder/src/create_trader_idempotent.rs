use phoenix_rise::ix;
use pinocchio::account_info::AccountInfo;
use pinocchio::cpi::slice_invoke_signed;
use pinocchio::instruction::{AccountMeta, Instruction, Signer};
use pinocchio::pubkey::find_program_address;
use pinocchio::{ProgramResult, msg, seeds};
use solana_pubkey::Pubkey as SolanaPubkey;

use crate::account_view::{
    GlobalConfigView, PermissionAccountView, TRADER_ONBOARDING_PERMISSION, TraderHeaderView,
    active_trader_buffer_account_count, global_trader_index_account_count,
};
use crate::{ID, TRADER_ONBOARDER_AUTHORITY_SEED, TraderOnboarderError};

/// Number of fixed accounts before the dynamic global trader index and active
/// trader buffer account slices.
const FIXED_ACCOUNT_COUNT: usize = 9;
/// Cross-margin trader accounts are registered with 128 position slots. Phoenix
/// will reject a different max position count for the `(0, 0)` trader.
const MAX_POSITIONS: u32 = 128;
/// This example only manages the default trader PDA index.
const TRADER_PDA_INDEX: u8 = 0;
/// `CreateTraderIdempotent` only creates or repairs the cross-margin
/// subaccount.
const TRADER_SUBACCOUNT_INDEX: u8 = 0;

pub(crate) fn process_create_trader_idempotent(accounts: &[AccountInfo]) -> ProgramResult {
    if accounts.len() < FIXED_ACCOUNT_COUNT {
        msg!("trader-onboarder: invalid account count");
        return Err(TraderOnboarderError::InvalidAccountList.into());
    }

    let payer = &accounts[0];
    let trader_authority = &accounts[1];
    let onboarder_authority = &accounts[2];
    let permission_account = &accounts[3];
    let phoenix_program = &accounts[4];
    let phoenix_log_authority = &accounts[5];
    let phoenix_global_config = &accounts[6];
    let trader_account = &accounts[7];
    let system_program = &accounts[8];

    let payer_key = SolanaPubkey::new_from_array(*payer.key());
    let trader_authority_key = SolanaPubkey::new_from_array(*trader_authority.key());
    let onboarder_authority_key = SolanaPubkey::new_from_array(*onboarder_authority.key());
    let permission_account_key = SolanaPubkey::new_from_array(*permission_account.key());
    let phoenix_program_key = SolanaPubkey::new_from_array(*phoenix_program.key());
    let phoenix_log_authority_key = SolanaPubkey::new_from_array(*phoenix_log_authority.key());
    let phoenix_global_config_key = SolanaPubkey::new_from_array(*phoenix_global_config.key());
    let trader_account_key = SolanaPubkey::new_from_array(*trader_account.key());
    let system_program_key = SolanaPubkey::new_from_array(*system_program.key());

    let (
        global_config_account_key,
        risk_authority,
        global_trader_index_header_key,
        active_trader_buffer_header_key,
    ) = {
        let data = phoenix_global_config
            .try_borrow_data()
            .map_err(|_| TraderOnboarderError::AccountBorrowFailed)?;
        let global_config = GlobalConfigView::try_from_account_bytes(&data).map_err(|error| {
            msg!(&format!(
                "trader-onboarder: failed to deserialize GlobalConfig: {error}"
            ));
            TraderOnboarderError::AccountDeserializeFailed
        })?;
        (
            global_config.account_key(),
            global_config.risk_authority(),
            global_config.global_trader_index_header_key(),
            global_config.active_trader_buffer_header_key(),
        )
    };

    let global_trader_index_start = FIXED_ACCOUNT_COUNT;
    let Some(global_trader_index_header) = accounts.get(global_trader_index_start) else {
        msg!("trader-onboarder: missing global trader index header");
        return Err(TraderOnboarderError::InvalidAccountList.into());
    };
    if global_trader_index_header_key != *global_trader_index_header.key() {
        msg!("trader-onboarder: global trader index header mismatch");
        return Err(TraderOnboarderError::InvalidAccountKey.into());
    }
    let global_trader_index_count = {
        let data = global_trader_index_header
            .try_borrow_data()
            .map_err(|_| TraderOnboarderError::AccountBorrowFailed)?;
        global_trader_index_account_count(&data).map_err(|error| {
            msg!(&format!(
                "trader-onboarder: failed to deserialize global trader index: {error}"
            ));
            TraderOnboarderError::AccountDeserializeFailed
        })?
    };

    let active_trader_buffer_start = global_trader_index_start
        .checked_add(global_trader_index_count)
        .ok_or(TraderOnboarderError::InvalidAccountList)?;
    let Some(active_trader_buffer_header) = accounts.get(active_trader_buffer_start) else {
        msg!("trader-onboarder: missing active trader buffer header");
        return Err(TraderOnboarderError::InvalidAccountList.into());
    };
    if active_trader_buffer_header_key != *active_trader_buffer_header.key() {
        msg!("trader-onboarder: active trader buffer header mismatch");
        return Err(TraderOnboarderError::InvalidAccountKey.into());
    }
    let active_trader_buffer_count = {
        let data = active_trader_buffer_header
            .try_borrow_data()
            .map_err(|_| TraderOnboarderError::AccountBorrowFailed)?;
        active_trader_buffer_account_count(&data).map_err(|error| {
            msg!(&format!(
                "trader-onboarder: failed to deserialize active trader buffer: {error}"
            ));
            TraderOnboarderError::AccountDeserializeFailed
        })?
    };

    let expected_account_count = active_trader_buffer_start
        .checked_add(active_trader_buffer_count)
        .ok_or(TraderOnboarderError::InvalidAccountList)?;
    if accounts.len() != expected_account_count {
        msg!("trader-onboarder: invalid account count");
        return Err(TraderOnboarderError::InvalidAccountList.into());
    }

    let global_trader_index: Vec<&AccountInfo> = accounts
        .get(global_trader_index_start..active_trader_buffer_start)
        .ok_or(TraderOnboarderError::InvalidAccountList)?
        .iter()
        .collect();
    let active_trader_buffer: Vec<&AccountInfo> = accounts
        .get(active_trader_buffer_start..expected_account_count)
        .ok_or(TraderOnboarderError::InvalidAccountList)?
        .iter()
        .collect();

    if phoenix_program_key != *ix::PHOENIX_PROGRAM_ID {
        msg!("trader-onboarder: Phoenix program id mismatch");
        return Err(TraderOnboarderError::InvalidAccountKey.into());
    }
    if phoenix_log_authority_key != *ix::PHOENIX_LOG_AUTHORITY {
        msg!("trader-onboarder: Phoenix log authority mismatch");
        return Err(TraderOnboarderError::InvalidAccountKey.into());
    }
    if phoenix_global_config_key != *ix::PHOENIX_GLOBAL_CONFIGURATION {
        msg!("trader-onboarder: Phoenix global configuration account mismatch");
        return Err(TraderOnboarderError::InvalidAccountKey.into());
    }
    if system_program_key != ix::SYSTEM_PROGRAM_ID {
        msg!("trader-onboarder: System program id mismatch");
        return Err(TraderOnboarderError::InvalidAccountKey.into());
    }
    if global_config_account_key != *phoenix_global_config.key() {
        msg!("trader-onboarder: global configuration key mismatch");
        return Err(TraderOnboarderError::InvalidAccountKey.into());
    }

    if !payer.is_signer() {
        msg!("trader-onboarder: payer must sign");
        return Err(TraderOnboarderError::MissingRequiredSignature.into());
    }
    if !payer.is_writable() {
        msg!("trader-onboarder: payer must be writable");
        return Err(TraderOnboarderError::InvalidAccountData.into());
    }
    if !trader_authority.is_signer() {
        msg!("trader-onboarder: trader authority must sign");
        return Err(TraderOnboarderError::MissingRequiredSignature.into());
    }
    if !trader_account.is_writable() {
        msg!("trader-onboarder: trader account must be writable");
        return Err(TraderOnboarderError::InvalidAccountData.into());
    }
    if !permission_account.is_writable() {
        msg!("trader-onboarder: permission account must be writable");
        return Err(TraderOnboarderError::InvalidAccountData.into());
    }

    let (expected_onboarder_authority, onboarder_bump) =
        find_program_address(&[TRADER_ONBOARDER_AUTHORITY_SEED], &ID);
    if onboarder_authority.key() != &expected_onboarder_authority {
        msg!("trader-onboarder: onboarder authority PDA mismatch");
        return Err(TraderOnboarderError::InvalidAccountKey.into());
    }

    let expected_permission_account = find_program_address(
        &[
            b"permission",
            risk_authority.as_ref(),
            onboarder_authority.key().as_ref(),
        ],
        phoenix_program.key(),
    )
    .0;
    if permission_account.key() != &expected_permission_account {
        msg!("trader-onboarder: permission account PDA mismatch");
        return Err(TraderOnboarderError::InvalidAccountKey.into());
    }

    if permission_account.owner() != phoenix_program.key() {
        msg!("trader-onboarder: permission account has wrong owner");
        return Err(TraderOnboarderError::InvalidAccountOwner.into());
    }
    {
        let data = permission_account
            .try_borrow_data()
            .map_err(|_| TraderOnboarderError::AccountBorrowFailed)?;
        let permission = PermissionAccountView::try_from_account_bytes(&data).map_err(|error| {
            msg!(&format!(
                "trader-onboarder: failed to deserialize PermissionAccount: {error}"
            ));
            TraderOnboarderError::AccountDeserializeFailed
        })?;
        let permission_uses_remaining = permission.num_signer_actions_remaining();
        msg!(&format!(
            "trader-onboarder: trader onboarding permission signer actions \
             remaining={permission_uses_remaining}"
        ));
        if !permission.is_set_for(
            &risk_authority,
            onboarder_authority.key(),
            TRADER_ONBOARDING_PERMISSION,
        ) {
            msg!("trader-onboarder: permission account is not set for trader onboarding");
            return Err(TraderOnboarderError::PermissionUnavailable.into());
        }
        if !permission.has_remaining_uses() {
            msg!("trader-onboarder: permission account has no uses remaining");
            return Err(TraderOnboarderError::PermissionUnavailable.into());
        }
    }

    let pda_schema = [TRADER_PDA_INDEX, TRADER_SUBACCOUNT_INDEX];
    let expected_trader_account = find_program_address(
        &[b"trader", trader_authority.key().as_ref(), &pda_schema],
        phoenix_program.key(),
    )
    .0;
    if trader_account.key() != &expected_trader_account {
        msg!("trader-onboarder: trader account PDA mismatch");
        return Err(TraderOnboarderError::InvalidAccountKey.into());
    }

    let registered_trader_this_call = if trader_account.data_is_empty() {
        msg!("trader-onboarder: trader missing; invoking Phoenix RegisterTrader");
        let register_params = ix::RegisterTraderParams::builder()
            .payer(payer_key)
            .trader(trader_authority_key)
            .trader_account(trader_account_key)
            .max_positions(MAX_POSITIONS as u64)
            .trader_pda_index(TRADER_PDA_INDEX)
            .subaccount_index(TRADER_SUBACCOUNT_INDEX)
            .build()
            .map_err(|error| {
                msg!(&format!(
                    "trader-onboarder: failed to build RegisterTrader params: {error}"
                ));
                TraderOnboarderError::PhoenixIxBuildFailed
            })?;
        let register_ix = ix::create_register_trader_ix(register_params).map_err(|error| {
            msg!(&format!(
                "trader-onboarder: failed to build RegisterTrader ix: {error}"
            ));
            TraderOnboarderError::PhoenixIxBuildFailed
        })?;
        if register_ix.program_id != phoenix_program_key {
            msg!("trader-onboarder: RegisterTrader CPI program key mismatch");
            return Err(TraderOnboarderError::PhoenixCpiMismatch.into());
        }
        let register_accounts = [
            phoenix_program,
            phoenix_log_authority,
            phoenix_global_config,
            payer,
            trader_authority,
            trader_account,
            system_program,
        ];
        if register_ix.accounts.len() != register_accounts.len() {
            msg!("trader-onboarder: RegisterTrader CPI account count mismatch");
            return Err(TraderOnboarderError::PhoenixCpiMismatch.into());
        }
        let mut register_metas = Vec::with_capacity(register_ix.accounts.len());
        for (index, (meta, account)) in register_ix
            .accounts
            .iter()
            .zip(register_accounts.iter())
            .enumerate()
        {
            if meta.pubkey != SolanaPubkey::new_from_array(*account.key()) {
                msg!(&format!(
                    "trader-onboarder: RegisterTrader CPI account {index} key mismatch"
                ));
                return Err(TraderOnboarderError::PhoenixCpiMismatch.into());
            }
            register_metas.push(AccountMeta::new(
                account.key(),
                meta.is_writable,
                meta.is_signer,
            ));
        }
        let register_program_id = register_ix.program_id.to_bytes();
        let register_instruction = Instruction {
            program_id: &register_program_id,
            accounts: &register_metas,
            data: &register_ix.data,
        };
        slice_invoke_signed(&register_instruction, &register_accounts, &[])?;
        true
    } else {
        if trader_account.owner() != phoenix_program.key() {
            msg!("trader-onboarder: existing trader account has wrong owner");
            return Err(TraderOnboarderError::InvalidAccountOwner.into());
        }
        msg!("trader-onboarder: trader already exists");
        false
    };

    let trader_flags = {
        let data = trader_account
            .try_borrow_data()
            .map_err(|_| TraderOnboarderError::AccountBorrowFailed)?;
        let trader = TraderHeaderView::try_from_account_bytes(&data).map_err(|error| {
            msg!(&format!(
                "trader-onboarder: failed to deserialize trader account: {error}"
            ));
            TraderOnboarderError::AccountDeserializeFailed
        })?;
        if trader.key() != *trader_account.key() {
            msg!("trader-onboarder: trader header key mismatch");
            return Err(TraderOnboarderError::InvalidAccountData.into());
        }
        if trader.authority() != *trader_authority.key() {
            msg!("trader-onboarder: trader authority mismatch");
            return Err(TraderOnboarderError::InvalidAccountData.into());
        }
        if trader.position_authority() != *trader_authority.key() {
            msg!("trader-onboarder: trader position authority mismatch");
            return Err(TraderOnboarderError::InvalidAccountData.into());
        }
        if registered_trader_this_call && trader.funding_key() != *payer.key() {
            msg!("trader-onboarder: trader funding key mismatch");
            return Err(TraderOnboarderError::InvalidAccountData.into());
        }
        if trader.trader_pda_index() != TRADER_PDA_INDEX
            || trader.trader_subaccount_index() != TRADER_SUBACCOUNT_INDEX
        {
            msg!("trader-onboarder: trader PDA indices mismatch");
            return Err(TraderOnboarderError::InvalidAccountData.into());
        }
        msg!(&format!(
            "trader-onboarder: trader flags={}; hot={}; cold={}; positions={}/{}",
            trader.flags(),
            trader.is_hot(),
            trader.is_cold(),
            trader.position_count(),
            trader.position_capacity()
        ));
        trader.flags()
    };
    if ix::is_trader_ready(trader_flags) {
        msg!(&format!(
            "trader-onboarder: trader already has required capabilities; flags={trader_flags}"
        ));
        return Ok(());
    }

    msg!(&format!(
        "trader-onboarder: trader missing capabilities; flags={trader_flags}"
    ));
    let global_trader_index_keys: Vec<SolanaPubkey> = global_trader_index
        .iter()
        .map(|account| SolanaPubkey::new_from_array(*account.key()))
        .collect();
    let active_trader_buffer_keys: Vec<SolanaPubkey> = active_trader_buffer
        .iter()
        .map(|account| SolanaPubkey::new_from_array(*account.key()))
        .collect();
    let onboard_params = ix::OnboardTraderDelegatedParams::builder()
        .authority(onboarder_authority_key)
        .permission_account(permission_account_key)
        .trader_account(trader_account_key)
        .global_trader_index(global_trader_index_keys)
        .active_trader_buffer(active_trader_buffer_keys)
        .build()
        .map_err(|error| {
            msg!(&format!(
                "trader-onboarder: failed to build OnboardTraderDelegated params: {error}"
            ));
            TraderOnboarderError::PhoenixIxBuildFailed
        })?;
    let onboard_ix = ix::create_onboard_trader_delegated_ix(onboard_params).map_err(|error| {
        msg!(&format!(
            "trader-onboarder: failed to build OnboardTraderDelegated ix: {error}"
        ));
        TraderOnboarderError::PhoenixIxBuildFailed
    })?;
    if onboard_ix.program_id != phoenix_program_key {
        msg!("trader-onboarder: OnboardTraderDelegated CPI program key mismatch");
        return Err(TraderOnboarderError::PhoenixCpiMismatch.into());
    }

    let bump_seed = [onboarder_bump];
    let signer_seeds = seeds!(TRADER_ONBOARDER_AUTHORITY_SEED, &bump_seed);
    let signer = [Signer::from(&signer_seeds)];

    let mut account_infos: Vec<&AccountInfo> =
        Vec::with_capacity(6 + global_trader_index.len() + active_trader_buffer.len());
    account_infos.extend([
        phoenix_program,
        phoenix_log_authority,
        phoenix_global_config,
        onboarder_authority,
        permission_account,
        trader_account,
    ]);
    account_infos.extend(global_trader_index.iter().copied());
    account_infos.extend(active_trader_buffer.iter().copied());

    if onboard_ix.accounts.len() != account_infos.len() {
        msg!("trader-onboarder: OnboardTraderDelegated CPI account count mismatch");
        return Err(TraderOnboarderError::PhoenixCpiMismatch.into());
    }
    let mut onboard_metas = Vec::with_capacity(onboard_ix.accounts.len());
    for (index, (meta, account)) in onboard_ix
        .accounts
        .iter()
        .zip(account_infos.iter())
        .enumerate()
    {
        if meta.pubkey != SolanaPubkey::new_from_array(*account.key()) {
            msg!(&format!(
                "trader-onboarder: OnboardTraderDelegated CPI account {index} key mismatch"
            ));
            return Err(TraderOnboarderError::PhoenixCpiMismatch.into());
        }
        onboard_metas.push(AccountMeta::new(
            account.key(),
            meta.is_writable,
            meta.is_signer,
        ));
    }
    let onboard_program_id = onboard_ix.program_id.to_bytes();
    let onboard_instruction = Instruction {
        program_id: &onboard_program_id,
        accounts: &onboard_metas,
        data: &onboard_ix.data,
    };
    slice_invoke_signed(&onboard_instruction, &account_infos, &signer)
}
