use phoenix_rise::ix::constants::{
    PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_LOG_AUTHORITY, PHOENIX_PROGRAM_ID, SYSTEM_PROGRAM_ID,
};
use phoenix_rise::ix::register_trader::{RegisterTraderParams, create_register_trader_ix};
use phoenix_rise::ix::sync_parent_to_child::{
    SyncParentToChildParams, create_sync_parent_to_child_ix,
};
use pinocchio::account_info::AccountInfo;
use pinocchio::cpi::slice_invoke_signed;
use pinocchio::instruction::{AccountMeta, Instruction};
use pinocchio::pubkey::find_program_address;
use pinocchio::{ProgramResult, msg};
use solana_pubkey::Pubkey as SolanaPubkey;

use crate::TraderOnboarderError;
use crate::account_view::{GlobalConfigView, TraderHeaderView, global_trader_index_account_count};

/// Number of fixed accounts before the dynamic global trader index slice.
const FIXED_ACCOUNT_COUNT: usize = 8;
/// Isolated subaccounts are registered with a single position slot.
const ISOLATED_MAX_POSITIONS: u64 = 1;
/// This example only manages the default trader PDA index.
const TRADER_PDA_INDEX: u8 = 0;
/// Parent account for isolated subaccounts is always the `(0, 0)`
/// cross-margin trader.
const CROSS_MARGIN_SUBACCOUNT_INDEX: u8 = 0;

pub(crate) fn process_register_subaccount_idempotent(
    accounts: &[AccountInfo],
    subaccount_index: u8,
) -> ProgramResult {
    if subaccount_index == CROSS_MARGIN_SUBACCOUNT_INDEX {
        msg!("trader-onboarder: subaccount index must be greater than zero");
        return Err(TraderOnboarderError::InvalidInstruction.into());
    }
    if accounts.len() < FIXED_ACCOUNT_COUNT {
        msg!("trader-onboarder: invalid account count");
        return Err(TraderOnboarderError::InvalidAccountList.into());
    }

    let payer = &accounts[0];
    let trader_authority = &accounts[1];
    let phoenix_program = &accounts[2];
    let phoenix_log_authority = &accounts[3];
    let phoenix_global_config = &accounts[4];
    let parent_trader_account = &accounts[5];
    let child_trader_account = &accounts[6];
    let system_program = &accounts[7];

    let payer_key = SolanaPubkey::new_from_array(*payer.key());
    let trader_authority_key = SolanaPubkey::new_from_array(*trader_authority.key());
    let phoenix_program_key = SolanaPubkey::new_from_array(*phoenix_program.key());
    let phoenix_log_authority_key = SolanaPubkey::new_from_array(*phoenix_log_authority.key());
    let phoenix_global_config_key = SolanaPubkey::new_from_array(*phoenix_global_config.key());
    let parent_trader_account_key = SolanaPubkey::new_from_array(*parent_trader_account.key());
    let child_trader_account_key = SolanaPubkey::new_from_array(*child_trader_account.key());
    let system_program_key = SolanaPubkey::new_from_array(*system_program.key());

    let (global_config_account_key, global_trader_index_header_key) = {
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
            global_config.global_trader_index_header_key(),
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

    let expected_account_count = global_trader_index_start
        .checked_add(global_trader_index_count)
        .ok_or(TraderOnboarderError::InvalidAccountList)?;
    if accounts.len() != expected_account_count {
        msg!("trader-onboarder: invalid account count");
        return Err(TraderOnboarderError::InvalidAccountList.into());
    }
    let global_trader_index: Vec<&AccountInfo> = accounts
        .get(global_trader_index_start..expected_account_count)
        .ok_or(TraderOnboarderError::InvalidAccountList)?
        .iter()
        .collect();

    if phoenix_program_key != *PHOENIX_PROGRAM_ID {
        msg!("trader-onboarder: Phoenix program id mismatch");
        return Err(TraderOnboarderError::InvalidAccountKey.into());
    }
    if phoenix_log_authority_key != *PHOENIX_LOG_AUTHORITY {
        msg!("trader-onboarder: Phoenix log authority mismatch");
        return Err(TraderOnboarderError::InvalidAccountKey.into());
    }
    if phoenix_global_config_key != *PHOENIX_GLOBAL_CONFIGURATION {
        msg!("trader-onboarder: Phoenix global configuration account mismatch");
        return Err(TraderOnboarderError::InvalidAccountKey.into());
    }
    if system_program_key != SYSTEM_PROGRAM_ID {
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
    if !child_trader_account.is_writable() {
        msg!("trader-onboarder: child trader account must be writable");
        return Err(TraderOnboarderError::InvalidAccountData.into());
    }

    let parent_pda_schema = [TRADER_PDA_INDEX, CROSS_MARGIN_SUBACCOUNT_INDEX];
    let expected_parent_trader_account = find_program_address(
        &[
            b"trader",
            trader_authority.key().as_ref(),
            &parent_pda_schema,
        ],
        phoenix_program.key(),
    )
    .0;
    if parent_trader_account.key() != &expected_parent_trader_account {
        msg!("trader-onboarder: parent cross-margin trader PDA mismatch");
        return Err(TraderOnboarderError::InvalidAccountKey.into());
    }

    let child_pda_schema = [TRADER_PDA_INDEX, subaccount_index];
    let expected_child_trader_account = find_program_address(
        &[
            b"trader",
            trader_authority.key().as_ref(),
            &child_pda_schema,
        ],
        phoenix_program.key(),
    )
    .0;
    if child_trader_account.key() != &expected_child_trader_account {
        msg!("trader-onboarder: child subaccount trader PDA mismatch");
        return Err(TraderOnboarderError::InvalidAccountKey.into());
    }

    if parent_trader_account.data_is_empty() {
        msg!("trader-onboarder: parent cross-margin trader is missing");
        return Err(TraderOnboarderError::InvalidAccountData.into());
    }
    if parent_trader_account.owner() != phoenix_program.key() {
        msg!("trader-onboarder: parent cross-margin trader has wrong owner");
        return Err(TraderOnboarderError::InvalidAccountOwner.into());
    }
    {
        let data = parent_trader_account
            .try_borrow_data()
            .map_err(|_| TraderOnboarderError::AccountBorrowFailed)?;
        let parent = TraderHeaderView::try_from_account_bytes(&data).map_err(|error| {
            msg!(&format!(
                "trader-onboarder: failed to deserialize parent trader account: {error}"
            ));
            TraderOnboarderError::AccountDeserializeFailed
        })?;
        if parent.key() != *parent_trader_account.key() {
            msg!("trader-onboarder: parent trader header key mismatch");
            return Err(TraderOnboarderError::InvalidAccountData.into());
        }
        if parent.authority() != *trader_authority.key() {
            msg!("trader-onboarder: parent trader authority mismatch");
            return Err(TraderOnboarderError::InvalidAccountData.into());
        }
        if parent.trader_pda_index() != TRADER_PDA_INDEX
            || parent.trader_subaccount_index() != CROSS_MARGIN_SUBACCOUNT_INDEX
        {
            msg!("trader-onboarder: parent trader PDA indices mismatch");
            return Err(TraderOnboarderError::InvalidAccountData.into());
        }
    }

    let registered_child_this_call = if child_trader_account.data_is_empty() {
        msg!("trader-onboarder: subaccount trader missing; invoking Phoenix RegisterTrader");
        let register_params = RegisterTraderParams::builder()
            .payer(payer_key)
            .trader(trader_authority_key)
            .trader_account(child_trader_account_key)
            .max_positions(ISOLATED_MAX_POSITIONS)
            .trader_pda_index(TRADER_PDA_INDEX)
            .subaccount_index(subaccount_index)
            .build()
            .map_err(|error| {
                msg!(&format!(
                    "trader-onboarder: failed to build RegisterTrader params: {error}"
                ));
                TraderOnboarderError::PhoenixIxBuildFailed
            })?;
        let register_ix = create_register_trader_ix(register_params).map_err(|error| {
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
            child_trader_account,
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
        if child_trader_account.owner() != phoenix_program.key() {
            msg!("trader-onboarder: existing child trader account has wrong owner");
            return Err(TraderOnboarderError::InvalidAccountOwner.into());
        }
        msg!("trader-onboarder: subaccount trader already exists");
        false
    };

    {
        let data = child_trader_account
            .try_borrow_data()
            .map_err(|_| TraderOnboarderError::AccountBorrowFailed)?;
        let child = TraderHeaderView::try_from_account_bytes(&data).map_err(|error| {
            msg!(&format!(
                "trader-onboarder: failed to deserialize child trader account: {error}"
            ));
            TraderOnboarderError::AccountDeserializeFailed
        })?;
        if child.key() != *child_trader_account.key() {
            msg!("trader-onboarder: child trader header key mismatch");
            return Err(TraderOnboarderError::InvalidAccountData.into());
        }
        if child.authority() != *trader_authority.key() {
            msg!("trader-onboarder: child trader authority mismatch");
            return Err(TraderOnboarderError::InvalidAccountData.into());
        }
        if child.max_positions() != ISOLATED_MAX_POSITIONS as u32 {
            msg!("trader-onboarder: child trader max positions mismatch");
            return Err(TraderOnboarderError::InvalidAccountData.into());
        }
        if registered_child_this_call && child.funding_key() != *payer.key() {
            msg!("trader-onboarder: child trader funding key mismatch");
            return Err(TraderOnboarderError::InvalidAccountData.into());
        }
        if child.trader_pda_index() != TRADER_PDA_INDEX
            || child.trader_subaccount_index() != subaccount_index
        {
            msg!("trader-onboarder: child trader PDA indices mismatch");
            return Err(TraderOnboarderError::InvalidAccountData.into());
        }
        msg!(&format!(
            "trader-onboarder: child trader flags={}; hot={}; cold={}; positions={}/{}",
            child.flags(),
            child.is_hot(),
            child.is_cold(),
            child.position_count(),
            child.position_capacity()
        ));
    }

    if !registered_child_this_call {
        return Ok(());
    }

    msg!("trader-onboarder: syncing parent cross-margin trader to child subaccount");
    let global_trader_index_keys: Vec<SolanaPubkey> = global_trader_index
        .iter()
        .map(|account| SolanaPubkey::new_from_array(*account.key()))
        .collect();
    let sync_params = SyncParentToChildParams::builder()
        .trader_wallet(trader_authority_key)
        .parent_trader_account(parent_trader_account_key)
        .child_trader_account(child_trader_account_key)
        .global_trader_index(global_trader_index_keys)
        .build()
        .map_err(|error| {
            msg!(&format!(
                "trader-onboarder: failed to build SyncParentToChild params: {error}"
            ));
            TraderOnboarderError::PhoenixIxBuildFailed
        })?;
    let sync_ix = create_sync_parent_to_child_ix(sync_params).map_err(|error| {
        msg!(&format!(
            "trader-onboarder: failed to build SyncParentToChild ix: {error}"
        ));
        TraderOnboarderError::PhoenixIxBuildFailed
    })?;
    if sync_ix.program_id != phoenix_program_key {
        msg!("trader-onboarder: SyncParentToChild CPI program key mismatch");
        return Err(TraderOnboarderError::PhoenixCpiMismatch.into());
    }

    let mut account_infos: Vec<&AccountInfo> = Vec::with_capacity(6 + global_trader_index.len());
    account_infos.extend([
        phoenix_program,
        phoenix_log_authority,
        phoenix_global_config,
        trader_authority,
        parent_trader_account,
        child_trader_account,
    ]);
    account_infos.extend(global_trader_index.iter().copied());

    if sync_ix.accounts.len() != account_infos.len() {
        msg!("trader-onboarder: SyncParentToChild CPI account count mismatch");
        return Err(TraderOnboarderError::PhoenixCpiMismatch.into());
    }
    let mut sync_metas = Vec::with_capacity(sync_ix.accounts.len());
    for (index, (meta, account)) in sync_ix
        .accounts
        .iter()
        .zip(account_infos.iter())
        .enumerate()
    {
        if meta.pubkey != SolanaPubkey::new_from_array(*account.key()) {
            msg!(&format!(
                "trader-onboarder: SyncParentToChild CPI account {index} key mismatch"
            ));
            return Err(TraderOnboarderError::PhoenixCpiMismatch.into());
        }
        sync_metas.push(AccountMeta::new(
            account.key(),
            meta.is_writable,
            meta.is_signer,
        ));
    }
    let sync_program_id = sync_ix.program_id.to_bytes();
    let sync_instruction = Instruction {
        program_id: &sync_program_id,
        accounts: &sync_metas,
        data: &sync_ix.data,
    };
    slice_invoke_signed(&sync_instruction, &account_infos, &[])
}
