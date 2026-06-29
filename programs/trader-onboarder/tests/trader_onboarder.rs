use std::path::PathBuf;

use borsh::{BorshSerialize, to_vec};
use phoenix_rise::accounts::owned::{Permission, Trader};
use phoenix_rise::accounts::trader::capabilities::{
    TRADER_CAPABILITY_CAN_DEPOSIT, TRADER_CAPABILITY_CAN_PLACE_LIMIT,
    TRADER_CAPABILITY_CAN_PLACE_MARKET, TRADER_CAPABILITY_CAN_RISK_INCREASE,
    TRADER_CAPABILITY_CAN_WITHDRAW,
};
use phoenix_rise::core::TraderKey;
use phoenix_rise::ix::constants::{
    PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_LOG_AUTHORITY, PHOENIX_PROGRAM_ID, SYSTEM_PROGRAM_ID,
    compute_discriminant,
};
use phoenix_rise::ix::register_trader::{RegisterTraderParams, create_register_trader_ix};
use phoenix_rise_litesvm_test::{
    PHOENIX_REPO_ROOT_ENV, SdkLocalnetContext, SdkLocalnetProgram, default_sdk_localnet_fixture,
    find_sdk_localnet_program_paths, parse_pubkey, sdk_localnet_vm_required,
};
use rise_trader_onboarder::{TRADER_ONBOARDER_AUTHORITY_SEED, TraderOnboarderError};
use solana_instruction::error::InstructionError;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;
use solana_transaction_error::TransactionError;

const TRADER_ONBOARDER_PROGRAM_ENV: &str = "RISE_TRADER_ONBOARDER_SO";
const TRADER_ONBOARDING_PERMISSION: u64 = 1 << 4;
const TRADER_CAPABILITY_FROZEN: u32 =
    TRADER_CAPABILITY_CAN_PLACE_LIMIT | TRADER_CAPABILITY_CAN_PLACE_MARKET;
const TRADER_CAPABILITY_COLD: u32 = TRADER_CAPABILITY_CAN_PLACE_LIMIT
    | TRADER_CAPABILITY_CAN_PLACE_MARKET
    | TRADER_CAPABILITY_CAN_RISK_INCREASE
    | TRADER_CAPABILITY_CAN_DEPOSIT
    | TRADER_CAPABILITY_CAN_WITHDRAW;

#[test]
fn trader_onboarder_error_codes_are_stable() {
    assert_eq!(TraderOnboarderError::InvalidInstruction.code(), 6000);
    assert_eq!(TraderOnboarderError::PermissionUnavailable.code(), 6008);
    assert_eq!(TraderOnboarderError::PhoenixCpiMismatch.code(), 6010);
}

#[test]
fn trader_onboarder_program_creates_and_onboards_idempotently() {
    let Some(program_paths) = find_sdk_localnet_program_paths() else {
        if sdk_localnet_vm_required() {
            panic!("missing local Phoenix/Ember SBF artifacts");
        }
        eprintln!(
            "skipping trader-onboarder example because local Phoenix SBF artifacts are missing"
        );
        return;
    };
    let Some(trader_onboarder_program) = find_trader_onboarder_program_path() else {
        if sdk_localnet_vm_required() {
            panic!(
                "missing trader-onboarder example SBF artifact; run cargo-build-sbf for \
                 rise/programs/trader-onboarder or set {TRADER_ONBOARDER_PROGRAM_ENV}"
            );
        }
        eprintln!("skipping trader-onboarder example because its SBF artifact is missing");
        return;
    };

    let fixture = default_sdk_localnet_fixture().expect("fixture should deserialize");
    let program_id = Pubkey::new_from_array(rise_trader_onboarder::ID);
    let mut context = SdkLocalnetContext::new_with_programs(
        fixture,
        program_paths,
        [SdkLocalnetProgram::new(
            program_id,
            trader_onboarder_program,
        )],
    );
    context.execute_setup();

    let payer = context.signer_pubkey("payer");
    let trader_authority = context.add_signer("rise-program-onboarded-trader", 10_000_000_000);
    let rent_payer = context.add_signer("rise-program-onboarder-rent-payer", 10_000_000_000);
    let trader_key = TraderKey::new(trader_authority);
    let trader_account = trader_key.pda();
    let onboarder_authority =
        Pubkey::find_program_address(&[TRADER_ONBOARDER_AUTHORITY_SEED], &program_id).0;
    let permission_account = get_permission_address(&payer, &onboarder_authority);

    set_exchange_gated(&mut context, payer);
    send_and_print_logs(
        &mut context,
        vec![
            create_permission_ix(&payer, &payer, &onboarder_authority),
            set_permission_ix(&payer, &onboarder_authority, TRADER_ONBOARDING_PERMISSION),
        ],
        "payer",
        "grant-program-onboarder-permission",
    );

    let created_trader_authority =
        context.add_signer("rise-program-created-trader", 10_000_000_000);
    let created_rent_payer = context.add_signer("rise-program-created-rent-payer", 10_000_000_000);
    let created_trader_key = TraderKey::new(created_trader_authority);
    let created_trader_account = created_trader_key.pda();
    let created_with_alternate_payer_ix = create_trader_idempotent_ix(
        program_id,
        created_rent_payer,
        created_trader_authority,
        onboarder_authority,
        permission_account,
        created_trader_account,
        &context.fixture.addresses.global_trader_index,
        &context.fixture.addresses.active_trader_buffer,
    );
    let created_rent_payer_lamports_before = context
        .svm
        .get_account(&created_rent_payer)
        .unwrap()
        .lamports;
    let created_logs = send_and_print_logs(
        &mut context,
        vec![created_with_alternate_payer_ix],
        "rise-program-created-rent-payer",
        "create-trader-idempotent-create-with-alternate-payer",
    );
    assert_logs_contain(
        &created_logs,
        "trader missing; invoking Phoenix RegisterTrader",
    );
    assert_logs_contain(
        &created_logs,
        "trader onboarding permission signer actions remaining=-1",
    );
    assert_logs_contain(&created_logs, "Phoenix Eternal: RegisterTrader");
    assert_logs_contain(&created_logs, "Phoenix Eternal Admin: SetTraderCapability");

    let created_trader = decode_trader(&context, &created_trader_account);
    assert_eq!(created_trader.authority, created_trader_authority);
    assert_eq!(created_trader.position_authority, created_trader_authority);
    assert_eq!(created_trader.funding_key, created_rent_payer);
    assert_eq!(created_trader.state.flags, TRADER_CAPABILITY_COLD);
    let created_rent_payer_lamports_after = context
        .svm
        .get_account(&created_rent_payer)
        .unwrap()
        .lamports;
    assert!(
        created_rent_payer_lamports_after < created_rent_payer_lamports_before,
        "alternate payer should pay rent and transaction fees for trader account creation"
    );

    let create_ix = create_trader_idempotent_ix(
        program_id,
        rent_payer,
        trader_authority,
        onboarder_authority,
        permission_account,
        trader_account,
        &context.fixture.addresses.global_trader_index,
        &context.fixture.addresses.active_trader_buffer,
    );

    let lamports_before = context.svm.get_account(&trader_authority).unwrap().lamports;
    let register_logs = send_and_print_logs(
        &mut context,
        vec![register_trader_ix(&trader_key)],
        "rise-program-onboarded-trader",
        "register-gated-trader",
    );
    assert_logs_contain(&register_logs, "Phoenix Eternal: RegisterTrader");
    assert_logs_do_not_contain(&register_logs, "Phoenix Eternal Admin: SetTraderCapability");

    let registered_trader = decode_trader(&context, &trader_account);
    assert_eq!(registered_trader.authority, trader_authority);
    assert_eq!(registered_trader.position_authority, trader_authority);
    assert_eq!(registered_trader.funding_key, trader_authority);
    assert_eq!(
        registered_trader.state.flags, TRADER_CAPABILITY_FROZEN,
        "gated RegisterTrader should leave deposit, withdraw, and risk-increase capabilities \
         disabled"
    );
    assert_ne!(registered_trader.state.flags, TRADER_CAPABILITY_COLD);
    assert_eq!(registered_trader.trader_pda_index, trader_key.pda_index);
    assert_eq!(
        registered_trader.trader_subaccount_index,
        trader_key.subaccount_index
    );

    let trader_owner = context.svm.get_account(&trader_account).unwrap().owner;
    assert_eq!(trader_owner, *PHOENIX_PROGRAM_ID);
    let lamports_after = context.svm.get_account(&trader_authority).unwrap().lamports;
    assert!(
        lamports_after < lamports_before,
        "trader authority should pay rent and transaction fees for trader account creation"
    );

    let create_logs = send_and_print_logs(
        &mut context,
        vec![create_ix.clone()],
        "rise-program-onboarded-trader",
        "create-trader-idempotent-enable-existing",
    );
    assert_logs_contain(&create_logs, "trader already exists");
    assert_logs_contain(&create_logs, "trader missing capabilities; flags=6");
    assert_logs_do_not_contain(&create_logs, "Phoenix Eternal: RegisterTrader");
    assert_logs_contain(&create_logs, "Phoenix Eternal Admin: SetTraderCapability");

    let trader = decode_trader(&context, &trader_account);
    assert_eq!(trader.authority, trader_authority);
    assert_eq!(trader.position_authority, trader_authority);
    assert_eq!(trader.funding_key, trader_authority);
    assert_ne!(trader.state.flags, registered_trader.state.flags);
    assert_eq!(trader.state.flags, TRADER_CAPABILITY_COLD);
    assert_eq!(trader.trader_pda_index, trader_key.pda_index);
    assert_eq!(trader.trader_subaccount_index, trader_key.subaccount_index);

    let second_logs = send_and_print_logs(
        &mut context,
        vec![create_ix],
        "rise-program-onboarded-trader",
        "create-trader-idempotent-second",
    );
    assert_logs_contain(&second_logs, "trader already exists");
    assert_logs_contain(&second_logs, "trader already has required capabilities");
    assert_logs_do_not_contain(&second_logs, "Phoenix Eternal: RegisterTrader");
    assert_logs_do_not_contain(&second_logs, "Phoenix Eternal Admin: SetTraderCapability");

    let child_trader_key = TraderKey::new_with_idx(trader_authority, 0, 1);
    let child_trader_account = child_trader_key.pda();
    let register_subaccount_ix = register_subaccount_idempotent_ix(
        program_id,
        rent_payer,
        trader_authority,
        trader_account,
        child_trader_account,
        child_trader_key.subaccount_index,
        &context.fixture.addresses.global_trader_index,
    );
    let register_subaccount_logs = send_and_print_logs(
        &mut context,
        vec![register_subaccount_ix.clone()],
        "rise-program-onboarded-trader",
        "register-subaccount-idempotent-first",
    );
    assert_logs_contain(
        &register_subaccount_logs,
        "subaccount trader missing; invoking Phoenix RegisterTrader",
    );
    assert_logs_contain(&register_subaccount_logs, "Phoenix Eternal: RegisterTrader");
    assert_logs_contain(
        &register_subaccount_logs,
        "syncing parent cross-margin trader to child subaccount",
    );

    let child_trader = decode_trader(&context, &child_trader_account);
    assert_eq!(child_trader.authority, trader_authority);
    assert_eq!(child_trader.funding_key, rent_payer);
    assert_eq!(child_trader.max_positions, 1);
    assert_eq!(child_trader.state.flags, trader.state.flags);
    assert_eq!(child_trader.trader_pda_index, child_trader_key.pda_index);
    assert_eq!(
        child_trader.trader_subaccount_index,
        child_trader_key.subaccount_index
    );

    let child_owner = context
        .svm
        .get_account(&child_trader_account)
        .unwrap()
        .owner;
    assert_eq!(child_owner, *PHOENIX_PROGRAM_ID);

    let second_subaccount_logs = send_and_print_logs(
        &mut context,
        vec![register_subaccount_ix],
        "rise-program-onboarded-trader",
        "register-subaccount-idempotent-second",
    );
    assert_logs_contain(&second_subaccount_logs, "subaccount trader already exists");
    assert_logs_do_not_contain(&second_subaccount_logs, "Phoenix Eternal: RegisterTrader");
    assert_logs_do_not_contain(
        &second_subaccount_logs,
        "syncing parent cross-margin trader to child subaccount",
    );
}

#[test]
fn trader_onboarder_rejects_permission_with_no_uses_remaining() {
    let Some(program_paths) = find_sdk_localnet_program_paths() else {
        if sdk_localnet_vm_required() {
            panic!("missing local Phoenix/Ember SBF artifacts");
        }
        eprintln!(
            "skipping trader-onboarder exhausted-permission test because local Phoenix SBF \
             artifacts are missing"
        );
        return;
    };
    let Some(trader_onboarder_program) = find_trader_onboarder_program_path() else {
        if sdk_localnet_vm_required() {
            panic!(
                "missing trader-onboarder example SBF artifact; run cargo-build-sbf for \
                 rise/programs/trader-onboarder or set {TRADER_ONBOARDER_PROGRAM_ENV}"
            );
        }
        eprintln!(
            "skipping trader-onboarder exhausted-permission test because its SBF artifact is \
             missing"
        );
        return;
    };

    let fixture = default_sdk_localnet_fixture().expect("fixture should deserialize");
    let program_id = Pubkey::new_from_array(rise_trader_onboarder::ID);
    let mut context = SdkLocalnetContext::new_with_programs(
        fixture,
        program_paths,
        [SdkLocalnetProgram::new(
            program_id,
            trader_onboarder_program,
        )],
    );
    context.execute_setup();

    let payer = context.signer_pubkey("payer");
    let onboarder_authority =
        Pubkey::find_program_address(&[TRADER_ONBOARDER_AUTHORITY_SEED], &program_id).0;
    let permission_account = get_permission_address(&payer, &onboarder_authority);

    set_exchange_gated(&mut context, payer);
    send_and_print_logs(
        &mut context,
        vec![
            create_permission_ix(&payer, &payer, &onboarder_authority),
            set_permission_ix_with_allowed_signer_actions(
                &payer,
                &onboarder_authority,
                TRADER_ONBOARDING_PERMISSION,
                Some(1),
            ),
        ],
        "payer",
        "grant-single-use-program-onboarder-permission",
    );

    let first_trader_authority =
        context.add_signer("rise-program-single-use-trader", 10_000_000_000);
    let first_rent_payer = context.add_signer("rise-program-single-use-rent-payer", 10_000_000_000);
    let first_trader_account = TraderKey::new(first_trader_authority).pda();
    let first_create_ix = create_trader_idempotent_ix(
        program_id,
        first_rent_payer,
        first_trader_authority,
        onboarder_authority,
        permission_account,
        first_trader_account,
        &context.fixture.addresses.global_trader_index,
        &context.fixture.addresses.active_trader_buffer,
    );
    let first_logs = send_and_print_logs(
        &mut context,
        vec![first_create_ix],
        "rise-program-single-use-rent-payer",
        "create-trader-idempotent-consume-only-permission-use",
    );
    assert_logs_contain(
        &first_logs,
        "trader onboarding permission signer actions remaining=1",
    );
    assert_logs_contain(&first_logs, "Phoenix Eternal Admin: SetTraderCapability");

    let permission_data = context.account_data(&permission_account);
    let permission = Permission::try_from_account_bytes(&permission_data)
        .expect("permission account should decode");
    assert_eq!(permission.allowed_signer_actions, 0);

    let ready_retry_result = context.try_send_instructions_with_metadata(
        vec![create_trader_idempotent_ix(
            program_id,
            first_rent_payer,
            first_trader_authority,
            onboarder_authority,
            permission_account,
            first_trader_account,
            &context.fixture.addresses.global_trader_index,
            &context.fixture.addresses.active_trader_buffer,
        )],
        "rise-program-single-use-rent-payer",
    );
    match &ready_retry_result {
        Ok(tx) => print_transaction_logs(
            "create-trader-idempotent-ready-retry-with-exhausted-permission-unexpected-success",
            &tx.logs,
        ),
        Err(error) => print_transaction_logs(
            "create-trader-idempotent-ready-retry-with-exhausted-permission-rejected",
            &error.meta.logs,
        ),
    }
    let Err(error) = ready_retry_result else {
        panic!("ready-trader retry with exhausted permission should fail");
    };
    assert_eq!(
        error.err,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(TraderOnboarderError::PermissionUnavailable.code())
        )
    );
    let logs = &error.meta.logs;
    assert_logs_contain(
        logs,
        "trader onboarding permission signer actions remaining=0",
    );
    assert_logs_contain(logs, "permission account has no uses remaining");
    assert_logs_do_not_contain(logs, "trader already exists");
    assert_logs_do_not_contain(logs, "trader already has required capabilities");
    assert_logs_do_not_contain(logs, "Phoenix Eternal: RegisterTrader");
    assert_logs_do_not_contain(logs, "Phoenix Eternal Admin: SetTraderCapability");

    let second_trader_authority =
        context.add_signer("rise-program-exhausted-permission-trader", 10_000_000_000);
    let second_rent_payer = context.add_signer(
        "rise-program-exhausted-permission-rent-payer",
        10_000_000_000,
    );
    let second_trader_account = TraderKey::new(second_trader_authority).pda();
    let exhausted_result = context.try_send_instructions_with_metadata(
        vec![create_trader_idempotent_ix(
            program_id,
            second_rent_payer,
            second_trader_authority,
            onboarder_authority,
            permission_account,
            second_trader_account,
            &context.fixture.addresses.global_trader_index,
            &context.fixture.addresses.active_trader_buffer,
        )],
        "rise-program-exhausted-permission-rent-payer",
    );
    match &exhausted_result {
        Ok(tx) => print_transaction_logs(
            "create-trader-idempotent-exhausted-permission-unexpected-success",
            &tx.logs,
        ),
        Err(error) => print_transaction_logs(
            "create-trader-idempotent-exhausted-permission-rejected",
            &error.meta.logs,
        ),
    }
    let Err(error) = exhausted_result else {
        panic!("exhausted permission should fail");
    };
    assert_eq!(
        error.err,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(TraderOnboarderError::PermissionUnavailable.code())
        )
    );
    let logs = &error.meta.logs;
    assert_logs_contain(
        logs,
        "trader onboarding permission signer actions remaining=0",
    );
    assert_logs_contain(logs, "permission account has no uses remaining");
    assert_logs_do_not_contain(logs, "Phoenix Eternal: RegisterTrader");
    assert_logs_do_not_contain(logs, "Phoenix Eternal Admin: SetTraderCapability");
}

fn register_trader_ix(trader_key: &TraderKey) -> Instruction {
    create_register_trader_ix(
        RegisterTraderParams::builder()
            .payer(trader_key.authority)
            .trader(trader_key.authority)
            .trader_account(trader_key.pda())
            .max_positions(128)
            .trader_pda_index(trader_key.pda_index)
            .subaccount_index(trader_key.subaccount_index)
            .build()
            .expect("register trader params should build"),
    )
    .expect("register trader instruction should build")
    .into()
}

fn create_trader_idempotent_ix(
    program_id: Pubkey,
    payer: Pubkey,
    trader_authority: Pubkey,
    onboarder_authority: Pubkey,
    permission_account: Pubkey,
    trader_account: Pubkey,
    global_trader_index: &[String],
    active_trader_buffer: &[String],
) -> Instruction {
    let data = compute_discriminant("global:create_trader_idempotent").to_vec();

    let mut accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(trader_authority, true),
        AccountMeta::new_readonly(onboarder_authority, false),
        AccountMeta::new(permission_account, false),
        AccountMeta::new_readonly(*PHOENIX_PROGRAM_ID, false),
        AccountMeta::new_readonly(*PHOENIX_LOG_AUTHORITY, false),
        AccountMeta::new_readonly(*PHOENIX_GLOBAL_CONFIGURATION, false),
        AccountMeta::new(trader_account, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
    ];
    accounts.extend(
        global_trader_index
            .iter()
            .map(|address| AccountMeta::new(parse_pubkey(address).unwrap(), false)),
    );
    accounts.extend(
        active_trader_buffer
            .iter()
            .map(|address| AccountMeta::new(parse_pubkey(address).unwrap(), false)),
    );

    Instruction {
        program_id,
        accounts,
        data,
    }
}

fn register_subaccount_idempotent_ix(
    program_id: Pubkey,
    payer: Pubkey,
    trader_authority: Pubkey,
    parent_trader_account: Pubkey,
    child_trader_account: Pubkey,
    subaccount_index: u8,
    global_trader_index: &[String],
) -> Instruction {
    let mut data = compute_discriminant("global:register_subaccount_idempotent").to_vec();
    data.push(subaccount_index);

    let mut accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(trader_authority, true),
        AccountMeta::new_readonly(*PHOENIX_PROGRAM_ID, false),
        AccountMeta::new_readonly(*PHOENIX_LOG_AUTHORITY, false),
        AccountMeta::new_readonly(*PHOENIX_GLOBAL_CONFIGURATION, false),
        AccountMeta::new_readonly(parent_trader_account, false),
        AccountMeta::new(child_trader_account, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
    ];
    accounts.extend(
        global_trader_index
            .iter()
            .map(|address| AccountMeta::new(parse_pubkey(address).unwrap(), false)),
    );

    Instruction {
        program_id,
        accounts,
        data,
    }
}

fn set_exchange_gated(context: &mut SdkLocalnetContext, root_authority: Pubkey) {
    #[derive(BorshSerialize)]
    struct ChangeExchangeStatusData {
        active: Option<bool>,
        gated: Option<bool>,
    }

    let mut data = compute_discriminant("global:change_exchange_status").to_vec();
    data.extend_from_slice(
        &to_vec(&ChangeExchangeStatusData {
            active: Some(true),
            gated: Some(true),
        })
        .unwrap(),
    );

    let logs = send_and_print_logs(
        context,
        vec![Instruction {
            program_id: *PHOENIX_PROGRAM_ID,
            accounts: vec![
                AccountMeta::new_readonly(*PHOENIX_PROGRAM_ID, false),
                AccountMeta::new_readonly(*PHOENIX_LOG_AUTHORITY, false),
                AccountMeta::new(*PHOENIX_GLOBAL_CONFIGURATION, false),
                AccountMeta::new_readonly(root_authority, true),
                AccountMeta::new(root_authority, false),
            ],
            data,
        }],
        "payer",
        "enable-gated-exchange",
    );
    assert_logs_contain(&logs, "Phoenix Eternal Admin: ChangeExchangeStatus");
    assert_logs_contain(&logs, "0b10000011");
}

fn create_permission_ix(payer: &Pubkey, authority: &Pubkey, user: &Pubkey) -> Instruction {
    Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*PHOENIX_PROGRAM_ID, false),
            AccountMeta::new_readonly(*PHOENIX_LOG_AUTHORITY, false),
            AccountMeta::new_readonly(*payer, true),
            AccountMeta::new(get_permission_address(authority, user), false),
            AccountMeta::new_readonly(*authority, false),
            AccountMeta::new_readonly(*user, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data: compute_discriminant("global:create_permission").to_vec(),
    }
}

fn set_permission_ix(authority: &Pubkey, user: &Pubkey, permission: u64) -> Instruction {
    set_permission_ix_with_allowed_signer_actions(authority, user, permission, None)
}

fn set_permission_ix_with_allowed_signer_actions(
    authority: &Pubkey,
    user: &Pubkey,
    permission: u64,
    allowed_signer_actions: Option<i64>,
) -> Instruction {
    #[derive(BorshSerialize)]
    struct SetPermissionData {
        permission: u64,
        expires_at_timestamp: Option<i64>,
        allowed_signer_actions: Option<i64>,
    }

    let mut data = compute_discriminant("global:set_permission").to_vec();
    data.extend_from_slice(
        &to_vec(&SetPermissionData {
            permission,
            expires_at_timestamp: None,
            allowed_signer_actions,
        })
        .unwrap(),
    );

    Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*PHOENIX_PROGRAM_ID, false),
            AccountMeta::new_readonly(*PHOENIX_LOG_AUTHORITY, false),
            AccountMeta::new(get_permission_address(authority, user), false),
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new_readonly(*user, false),
        ],
        data,
    }
}

fn get_permission_address(authority: &Pubkey, user: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"permission", authority.as_ref(), user.as_ref()],
        &*PHOENIX_PROGRAM_ID,
    )
    .0
}

fn decode_trader(context: &SdkLocalnetContext, trader_account: &Pubkey) -> Trader {
    Trader::try_from_account_bytes(&context.account_data(trader_account))
        .expect("trader account should decode")
}

fn send_and_print_logs(
    context: &mut SdkLocalnetContext,
    instructions: Vec<Instruction>,
    fee_payer_seed: &str,
    label: &str,
) -> Vec<String> {
    let tx = context.send_instructions_with_metadata(instructions, fee_payer_seed, label);
    print_transaction_logs(label, &tx.logs);
    tx.logs
}

fn print_transaction_logs(label: &str, logs: &[String]) {
    println!("\n{}\n", transaction_log_title(label));
    if logs.is_empty() {
        println!("(no logs)");
    } else {
        for log in logs {
            println!("{log}");
        }
    }
}

fn transaction_log_title(label: &str) -> String {
    match label {
        "enable-gated-exchange" => "Enabling gated exchange...".to_string(),
        "grant-program-onboarder-permission" => {
            "Granting program onboarder permission...".to_string()
        }
        "create-trader-idempotent-create-with-alternate-payer" => {
            "Creating trader with alternate payer...".to_string()
        }
        "register-gated-trader" => "Registering gated trader...".to_string(),
        "create-trader-idempotent-enable-existing" => "Enabling existing trader...".to_string(),
        "create-trader-idempotent-second" => "Retrying idempotent trader creation...".to_string(),
        "register-subaccount-idempotent-first" => "Registering subaccount...".to_string(),
        "register-subaccount-idempotent-second" => {
            "Retrying idempotent subaccount registration...".to_string()
        }
        "grant-single-use-program-onboarder-permission" => {
            "Granting single-use program onboarder permission...".to_string()
        }
        "create-trader-idempotent-consume-only-permission-use" => {
            "Consuming only permission use...".to_string()
        }
        "create-trader-idempotent-ready-retry-with-exhausted-permission-rejected" => {
            "Rejecting ready trader retry with exhausted permission...".to_string()
        }
        "create-trader-idempotent-ready-retry-with-exhausted-permission-unexpected-success" => {
            "Unexpected ready trader retry with exhausted permission success...".to_string()
        }
        "create-trader-idempotent-exhausted-permission-rejected" => {
            "Rejecting exhausted permission...".to_string()
        }
        "create-trader-idempotent-exhausted-permission-unexpected-success" => {
            "Unexpected exhausted permission success...".to_string()
        }
        _ => format!("{label}..."),
    }
}

fn assert_logs_contain(logs: &[String], needle: &str) {
    assert!(
        logs.iter().any(|line| line.contains(needle)),
        "expected logs to contain `{needle}`\n{}",
        logs.join("\n")
    );
}

fn assert_logs_do_not_contain(logs: &[String], needle: &str) {
    assert!(
        !logs.iter().any(|line| line.contains(needle)),
        "expected logs not to contain `{needle}`\n{}",
        logs.join("\n")
    );
}

fn find_trader_onboarder_program_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var(TRADER_ONBOARDER_PROGRAM_ENV) {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }

    for root in default_roots() {
        for path in [
            root.join("rise/programs/trader-onboarder/target/deploy/rise_trader_onboarder.so"),
            root.join("target/deploy/rise_trader_onboarder.so"),
        ] {
            if path.exists() {
                return Some(path);
            }
        }
    }
    None
}

fn default_roots() -> Vec<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut roots = vec![
        manifest_dir.join("../.."),
        std::env::current_dir().unwrap_or_else(|_| manifest_dir.clone()),
    ];
    if let Ok(root) = std::env::var(PHOENIX_REPO_ROOT_ENV) {
        roots.push(PathBuf::from(root));
    }
    roots.sort();
    roots.dedup();
    roots
}
