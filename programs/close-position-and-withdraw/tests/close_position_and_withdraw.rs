use std::path::PathBuf;

use borsh::to_vec;
use phoenix_rise::ix::HAWKEYE_PROGRAM_ID;
use phoenix_rise::ix::constants::{
    EMBER_PROGRAM_ID, PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_LOG_AUTHORITY, PHOENIX_PROGRAM_ID,
    SPL_TOKEN_PROGRAM_ID, compute_discriminant,
};
use phoenix_rise::ix::flight::{
    FLIGHT_PROGRAM_ID, get_flight_builder_state_address, get_flight_global_state_address,
};
use phoenix_rise::ix::hawkeye::{
    HawkeyeReturnData, HawkeyeTraderViewAccounts, ViewAssetReturn, ViewMarginReturn,
    create_hawkeye_view_margin_for_asset_ix, create_hawkeye_view_margin_ix,
    decode_hawkeye_return_data,
};
use phoenix_rise::ix::market_order::{MarketOrderParams, create_place_market_order_ix};
use phoenix_rise::ix::types::{OrderFlags, SelfTradeBehavior, Side};
use phoenix_rise_litesvm_test::{
    FixtureActor, FixtureMarket, PHOENIX_REPO_ROOT_ENV, SdkLocalnetContext, SdkLocalnetProgram,
    default_sdk_localnet_fixture, find_sdk_localnet_program_paths, mainnet_bpf_programs_enabled,
    parse_pubkey, parse_pubkeys, sdk_localnet_vm_required,
};
use rise_close_position_and_withdraw::{
    ClosePositionAndWithdrawParams, SelfTradeBehavior as CloseSelfTradeBehavior, Side as CloseSide,
    WithdrawAllCollateralParams, WithdrawMode,
};
use solana_account::Account;
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

const CLOSE_POSITION_PROGRAM_ENV: &str = "RISE_CLOSE_POSITION_AND_WITHDRAW_SO";
const BTC_ASSET_ID: u32 = 0;
const BTC_ORDER_BASE_LOTS: u64 = 100;
const BTC_OPEN_PRICE_USD: f64 = 100_500.0;
const BTC_CLOSE_PRICE_USD: f64 = 99_500.0;
const BUILDER_FEE_BPS: u64 = 5;
const MICRO_FEE_DENOMINATOR: u64 = 1_000_000;
const BPS_DENOMINATOR: u64 = 10_000;
const SPL_TOKEN_ACCOUNT_LEN: usize = 165;
const SPL_TOKEN_ACCOUNT_AMOUNT_OFFSET: usize = 64;
const SPL_TOKEN_ACCOUNT_STATE_OFFSET: usize = 108;

#[test]
fn close_position_and_withdraw_program_executes_fixture_flows() {
    let Some(program_paths) = find_sdk_localnet_program_paths() else {
        if sdk_localnet_vm_required() {
            panic!("missing local Phoenix/Ember/Hawkeye SBF artifacts");
        }
        eprintln!("skipping close-position example because local SBF artifacts are missing");
        return;
    };
    let mainnet_bpf_programs_enabled = mainnet_bpf_programs_enabled();
    if program_paths.hawkeye.is_none() && !mainnet_bpf_programs_enabled {
        if sdk_localnet_vm_required() {
            panic!("missing local Hawkeye SBF artifact");
        }
        eprintln!("skipping close-position example because phoenix_hawkeye.so is missing");
        return;
    }
    let Some(close_position_program) = find_close_position_program_path() else {
        if sdk_localnet_vm_required() {
            panic!(
                "missing close-position example SBF artifact; run cargo-build-sbf for \
                 rise/programs/close-position-and-withdraw or set {CLOSE_POSITION_PROGRAM_ENV}"
            );
        }
        eprintln!("skipping close-position example because its SBF artifact is missing");
        return;
    };
    let flight_program_available = program_paths.flight.is_some() || mainnet_bpf_programs_enabled;

    let fixture = default_sdk_localnet_fixture().expect("fixture should deserialize");
    let close_position_program_id = Pubkey::new_from_array(rise_close_position_and_withdraw::ID);
    let mut context = SdkLocalnetContext::new_with_programs(
        fixture,
        program_paths,
        [SdkLocalnetProgram::new(
            close_position_program_id,
            close_position_program,
        )],
    );
    context.execute_setup();
    context.svm.warp_to_slot(200);
    context.send_fixture_transaction("oracleSetPrices");
    context.send_fixture_transaction("splineUpdatePrices");

    let market = context.market("BTC");
    let quote_decimals = context.fixture_mint("fakeUsdc").decimals;
    let open_quote_lots = quote_lots_for_price(
        &market,
        BTC_OPEN_PRICE_USD,
        quote_decimals,
        BTC_ORDER_BASE_LOTS,
    );
    let close_quote_lots = quote_lots_for_price(
        &market,
        BTC_CLOSE_PRICE_USD,
        quote_decimals,
        BTC_ORDER_BASE_LOTS,
    );
    let open_taker_fee = taker_fee_quote_lots(open_quote_lots, market.default_taker_fee_micro);
    let close_taker_fee = taker_fee_quote_lots(close_quote_lots, market.default_taker_fee_micro);
    let expected_pnl = close_quote_lots as i64 - open_quote_lots as i64;
    let expected_close_collateral_delta =
        expected_pnl - open_taker_fee as i64 - close_taker_fee as i64;
    let builder_fee_base_quote_lots = close_quote_lots - close_taker_fee;
    let expected_builder_fee = builder_fee_quote_lots(builder_fee_base_quote_lots);

    let diff_actor = context.actor("taker0");
    open_btc_position(&mut context, &diff_actor, &market);
    assert_eq!(
        hawkeye_margin(&mut context, &diff_actor).collateral_quote_lots,
        diff_actor.phoenix_deposit_atoms as i64 - open_taker_fee as i64,
        "opening market order should deduct the expected Phoenix taker fee"
    );
    context.move_market_price_usd("BTC", 99_500.0);
    let diff_asset = hawkeye_asset(&mut context, &diff_actor);
    assert_ne!(
        diff_asset.base_lots, 0,
        "fixture open should create a BTC position"
    );
    assert_eq!(
        diff_asset.unrealized_pnl_quote_lots, expected_pnl,
        "fixture price move should produce the expected unrealized PnL"
    );
    assert!(
        diff_asset.unrealized_pnl_quote_lots < 0,
        "fixture price move should put the long BTC position at negative PnL"
    );
    let diff_usdc_before = token_amount(
        &context,
        &parse_pubkey(&diff_actor.fake_usdc_token_account).unwrap(),
    );
    let diff_close_ix = with_compute_budget(close_position_ix(
        &context,
        &diff_actor,
        &market,
        close_position_program_id,
        WithdrawMode::OrderFreeCollateralDelta,
        None,
    ));
    assert_dynamic_accounts_at_end(&diff_close_ix[1], &context);
    let diff_close_logs = send_and_print_logs(
        &mut context,
        diff_close_ix,
        &diff_actor.seed,
        "close-position-diff-withdraw",
    );
    assert_close_fee_math(
        &diff_close_logs,
        diff_actor.phoenix_deposit_atoms as i64 + expected_close_collateral_delta,
    );
    assert_order_free_collateral_delta_withdraw(&diff_close_logs);
    assert_eq!(
        hawkeye_asset_base_lots(&mut context, &diff_actor),
        0,
        "diff withdrawal should still close the position"
    );
    let diff_usdc_after = token_amount(
        &context,
        &parse_pubkey(&diff_actor.fake_usdc_token_account).unwrap(),
    );
    assert!(
        diff_usdc_after > diff_usdc_before,
        "diff withdrawal should move newly freed collateral to USDC"
    );
    assert!(
        hawkeye_margin(&mut context, &diff_actor).withdrawable_collateral_quote_lots > 0,
        "diff mode should leave pre-existing withdrawable collateral on Phoenix"
    );
    let withdraw_only_usdc_before = token_amount(
        &context,
        &parse_pubkey(&diff_actor.fake_usdc_token_account).unwrap(),
    );
    let withdraw_only_ix = with_compute_budget(withdraw_all_collateral_ix(
        &context,
        &diff_actor,
        close_position_program_id,
        None,
    ));
    assert_dynamic_accounts_at_end(&withdraw_only_ix[1], &context);
    assert_withdraw_all_collateral_omits_market_accounts(&withdraw_only_ix[1], &market);
    let withdraw_only_logs = send_and_print_logs(
        &mut context,
        withdraw_only_ix,
        &diff_actor.seed,
        "withdraw-all-collateral",
    );
    assert_logs_contain(&withdraw_only_logs, "trader is cold");
    assert_logs_contain(
        &withdraw_only_logs,
        "withdrawable collateral before withdraw",
    );
    assert_logs_contain(
        &withdraw_only_logs,
        "Withdraw request processed successfully",
    );
    assert_logs_contain(
        &withdraw_only_logs,
        &format!("Program {EMBER_PROGRAM_ID} invoke"),
    );
    assert_logs_do_not_contain(&withdraw_only_logs, "Phoenix Eternal: Place Market Order");
    assert_logs_do_not_contain(&withdraw_only_logs, "Phoenix Flight: Proxy");
    assert_logs_do_not_contain(&withdraw_only_logs, "withdrawable collateral after order");
    assert_eq!(
        hawkeye_margin(&mut context, &diff_actor).withdrawable_collateral_quote_lots,
        0,
        "withdraw-only instruction should drain free collateral without orderbook or spline \
         accounts"
    );
    let withdraw_only_usdc_after = token_amount(
        &context,
        &parse_pubkey(&diff_actor.fake_usdc_token_account).unwrap(),
    );
    assert!(
        withdraw_only_usdc_after > withdraw_only_usdc_before,
        "withdraw-only instruction should move remaining collateral to USDC"
    );

    context.send_fixture_transaction("orderbookPlaceLevels");

    let all_actor = context.actor("taker1");
    open_btc_position(&mut context, &all_actor, &market);
    let all_usdc_before = token_amount(
        &context,
        &parse_pubkey(&all_actor.fake_usdc_token_account).unwrap(),
    );
    let all_close_ix = with_compute_budget(close_position_ix(
        &context,
        &all_actor,
        &market,
        close_position_program_id,
        WithdrawMode::AllFreeCollateral,
        None,
    ));
    assert_dynamic_accounts_at_end(&all_close_ix[1], &context);
    let all_close_logs = send_and_print_logs(
        &mut context,
        all_close_ix,
        &all_actor.seed,
        "close-position-all-withdraw",
    );
    assert_close_fee_math(
        &all_close_logs,
        all_actor.phoenix_deposit_atoms as i64 + expected_close_collateral_delta,
    );
    assert_all_free_collateral_withdraw(&all_close_logs);
    assert_eq!(
        hawkeye_asset_base_lots(&mut context, &all_actor),
        0,
        "all-free-collateral withdrawal should close the position"
    );
    let all_usdc_after = token_amount(
        &context,
        &parse_pubkey(&all_actor.fake_usdc_token_account).unwrap(),
    );
    assert!(
        all_usdc_after > all_usdc_before,
        "all-free-collateral mode should move collateral to USDC"
    );
    assert_eq!(
        hawkeye_margin(&mut context, &all_actor).withdrawable_collateral_quote_lots,
        0,
        "all-free-collateral mode should leave no immediately withdrawable collateral"
    );

    let wrong_destination_actor = context.actor("stopLossTaker");
    let result = context.try_send_instructions_with_metadata(
        with_compute_budget(close_position_ix(
            &context,
            &wrong_destination_actor,
            &market,
            close_position_program_id,
            WithdrawMode::AllFreeCollateral,
            Some(parse_pubkey(&all_actor.fake_usdc_token_account).unwrap()),
        )),
        &wrong_destination_actor.seed,
    );
    match &result {
        Ok(tx) => print_transaction_logs("wrong-destination-unexpected-success", &tx.logs),
        Err(error) => print_transaction_logs("wrong-destination-rejected", &error.meta.logs),
    }
    assert!(
        result.is_err(),
        "program must reject a USDC token account not owned by the trader authority"
    );

    // Withdraw-all validation matrix:
    // - non-trader and PDA-owned USDC destinations are rejected before any CPI,
    // - mint checks and Phoenix-token source ownership are always enforced.
    let withdraw_wrong_destination_ix = with_compute_budget(withdraw_all_collateral_ix(
        &context,
        &wrong_destination_actor,
        close_position_program_id,
        Some(parse_pubkey(&all_actor.fake_usdc_token_account).unwrap()),
    ));
    let withdraw_wrong_destination_result = context.try_send_instructions_with_metadata(
        withdraw_wrong_destination_ix,
        &wrong_destination_actor.seed,
    );
    match &withdraw_wrong_destination_result {
        Ok(tx) => print_transaction_logs("withdraw-wrong-destination-unexpected-success", &tx.logs),
        Err(error) => {
            print_transaction_logs("withdraw-wrong-destination-rejected", &error.meta.logs)
        }
    }
    assert!(
        withdraw_wrong_destination_result.is_err(),
        "withdraw-only instruction must reject a USDC token account not owned by the trader \
         authority"
    );
    let withdraw_wrong_destination_error = withdraw_wrong_destination_result.unwrap_err();
    let withdraw_wrong_destination_logs = &withdraw_wrong_destination_error.meta.logs;
    assert_logs_contain(
        withdraw_wrong_destination_logs,
        "trader USDC token account owner mismatch",
    );
    assert_logs_do_not_contain(
        withdraw_wrong_destination_logs,
        "withdrawable collateral before withdraw",
    );
    assert_logs_do_not_contain(withdraw_wrong_destination_logs, "Phoenix Eternal:");
    assert_logs_do_not_contain(withdraw_wrong_destination_logs, "EMBERp");

    let pda_usdc_token_account =
        install_pda_owned_usdc_token_account(&mut context, close_position_program_id);
    let withdrawable_before_pda_destination =
        hawkeye_margin(&mut context, &wrong_destination_actor).withdrawable_collateral_quote_lots;
    assert!(
        withdrawable_before_pda_destination > 0,
        "PDA-destination test requires withdrawable collateral"
    );
    let pda_usdc_before = token_amount(&context, &pda_usdc_token_account);

    let withdraw_pda_destination_ix = with_compute_budget(withdraw_all_collateral_ix(
        &context,
        &wrong_destination_actor,
        close_position_program_id,
        Some(pda_usdc_token_account),
    ));
    let withdraw_pda_destination_result = context.try_send_instructions_with_metadata(
        withdraw_pda_destination_ix,
        &wrong_destination_actor.seed,
    );
    match &withdraw_pda_destination_result {
        Ok(tx) => print_transaction_logs("withdraw-pda-destination-unexpected-success", &tx.logs),
        Err(error) => print_transaction_logs("withdraw-pda-destination-rejected", &error.meta.logs),
    }
    assert!(
        withdraw_pda_destination_result.is_err(),
        "withdraw-only instruction must reject a USDC token account not owned by the trader \
         authority, including PDA-owned token accounts"
    );
    let withdraw_pda_destination_error = withdraw_pda_destination_result.unwrap_err();
    let withdraw_pda_destination_logs = &withdraw_pda_destination_error.meta.logs;
    assert_logs_contain(
        withdraw_pda_destination_logs,
        "trader USDC token account owner mismatch",
    );
    assert_logs_do_not_contain(
        withdraw_pda_destination_logs,
        "withdrawable collateral before withdraw",
    );
    assert_logs_do_not_contain(withdraw_pda_destination_logs, "Phoenix Eternal:");
    assert_logs_do_not_contain(withdraw_pda_destination_logs, "EMBERp");
    assert_eq!(
        token_amount(&context, &pda_usdc_token_account),
        pda_usdc_before,
        "rejected PDA destination should not receive USDC"
    );
    assert_eq!(
        hawkeye_margin(&mut context, &wrong_destination_actor).withdrawable_collateral_quote_lots,
        withdrawable_before_pda_destination,
        "rejected PDA destination should not drain Phoenix collateral"
    );

    let withdraw_wrong_mint_ix = with_compute_budget(withdraw_all_collateral_ix(
        &context,
        &wrong_destination_actor,
        close_position_program_id,
        Some(parse_pubkey(&wrong_destination_actor.phoenix_token_account).unwrap()),
    ));
    let withdraw_wrong_mint_result = context
        .try_send_instructions_with_metadata(withdraw_wrong_mint_ix, &wrong_destination_actor.seed);
    match &withdraw_wrong_mint_result {
        Ok(tx) => print_transaction_logs("withdraw-wrong-mint-unexpected-success", &tx.logs),
        Err(error) => print_transaction_logs("withdraw-wrong-mint-rejected", &error.meta.logs),
    }
    assert!(
        withdraw_wrong_mint_result.is_err(),
        "withdraw-only instruction must require a USDC-mint token account"
    );
    assert_logs_contain(
        &withdraw_wrong_mint_result.unwrap_err().meta.logs,
        "trader USDC token account mint mismatch",
    );

    let withdraw_wrong_usdc_mint_ix =
        with_compute_budget(withdraw_all_collateral_ix_with_overrides(
            &context,
            &wrong_destination_actor,
            close_position_program_id,
            Some(WithdrawAllAccountOverrides {
                usdc_mint: Some(fixture_mint_pubkey(&context, "phoenixCollateral")),
                ..WithdrawAllAccountOverrides::default()
            }),
        ));
    let withdraw_wrong_usdc_mint_result = context.try_send_instructions_with_metadata(
        withdraw_wrong_usdc_mint_ix,
        &wrong_destination_actor.seed,
    );
    match &withdraw_wrong_usdc_mint_result {
        Ok(tx) => print_transaction_logs("withdraw-wrong-usdc-mint-unexpected-success", &tx.logs),
        Err(error) => print_transaction_logs("withdraw-wrong-usdc-mint-rejected", &error.meta.logs),
    }
    assert!(
        withdraw_wrong_usdc_mint_result.is_err(),
        "withdraw-only instruction must reject an Ember input mint that does not match Ember state"
    );
    let withdraw_wrong_usdc_mint_logs = &withdraw_wrong_usdc_mint_result.unwrap_err().meta.logs;
    assert_logs_contain(withdraw_wrong_usdc_mint_logs, "Ember input mint mismatch");
    assert_logs_do_not_contain(
        withdraw_wrong_usdc_mint_logs,
        "withdrawable collateral before withdraw",
    );
    assert_logs_do_not_contain(withdraw_wrong_usdc_mint_logs, "Phoenix Eternal:");
    assert_logs_do_not_contain(withdraw_wrong_usdc_mint_logs, "EMBERp");

    let withdraw_wrong_canonical_mint_ix =
        with_compute_budget(withdraw_all_collateral_ix_with_overrides(
            &context,
            &wrong_destination_actor,
            close_position_program_id,
            Some(WithdrawAllAccountOverrides {
                canonical_mint: Some(fixture_mint_pubkey(&context, "fakeUsdc")),
                ..WithdrawAllAccountOverrides::default()
            }),
        ));
    let withdraw_wrong_canonical_mint_result = context.try_send_instructions_with_metadata(
        withdraw_wrong_canonical_mint_ix,
        &wrong_destination_actor.seed,
    );
    match &withdraw_wrong_canonical_mint_result {
        Ok(tx) => {
            print_transaction_logs("withdraw-wrong-canonical-mint-unexpected-success", &tx.logs)
        }
        Err(error) => {
            print_transaction_logs("withdraw-wrong-canonical-mint-rejected", &error.meta.logs)
        }
    }
    assert!(
        withdraw_wrong_canonical_mint_result.is_err(),
        "withdraw-only instruction must reject an Ember output mint that does not match Ember \
         state"
    );
    let withdraw_wrong_canonical_mint_logs =
        &withdraw_wrong_canonical_mint_result.unwrap_err().meta.logs;
    assert_logs_contain(
        withdraw_wrong_canonical_mint_logs,
        "Ember output mint mismatch",
    );
    assert_logs_do_not_contain(
        withdraw_wrong_canonical_mint_logs,
        "withdrawable collateral before withdraw",
    );
    assert_logs_do_not_contain(withdraw_wrong_canonical_mint_logs, "Phoenix Eternal:");
    assert_logs_do_not_contain(withdraw_wrong_canonical_mint_logs, "EMBERp");

    assert_withdraw_all_rejects_program_id(
        &mut context,
        &wrong_destination_actor,
        close_position_program_id,
        1,
        test_pubkey(101),
        "Phoenix program id mismatch",
        "withdraw-wrong-phoenix-program",
    );
    assert_withdraw_all_rejects_program_id(
        &mut context,
        &wrong_destination_actor,
        close_position_program_id,
        2,
        test_pubkey(102),
        "Hawkeye program id mismatch",
        "withdraw-wrong-hawkeye-program",
    );
    assert_withdraw_all_rejects_program_id(
        &mut context,
        &wrong_destination_actor,
        close_position_program_id,
        3,
        test_pubkey(103),
        "Ember program id mismatch",
        "withdraw-wrong-ember-program",
    );
    assert_withdraw_all_rejects_program_id(
        &mut context,
        &wrong_destination_actor,
        close_position_program_id,
        16,
        test_pubkey(104),
        "SPL Token program id mismatch",
        "withdraw-wrong-token-program",
    );

    let withdraw_wrong_phoenix_source_ix =
        with_compute_budget(withdraw_all_collateral_ix_with_overrides(
            &context,
            &wrong_destination_actor,
            close_position_program_id,
            Some(WithdrawAllAccountOverrides {
                phoenix_token_account: Some(
                    parse_pubkey(&all_actor.phoenix_token_account).unwrap(),
                ),
                ..WithdrawAllAccountOverrides::default()
            }),
        ));
    let withdraw_wrong_phoenix_source_result = context.try_send_instructions_with_metadata(
        withdraw_wrong_phoenix_source_ix,
        &wrong_destination_actor.seed,
    );
    match &withdraw_wrong_phoenix_source_result {
        Ok(tx) => {
            print_transaction_logs("withdraw-wrong-phoenix-source-unexpected-success", &tx.logs)
        }
        Err(error) => {
            print_transaction_logs("withdraw-wrong-phoenix-source-rejected", &error.meta.logs)
        }
    }
    assert!(
        withdraw_wrong_phoenix_source_result.is_err(),
        "withdraw-only instruction must reject a Phoenix-token account not owned by the trader \
         authority"
    );
    let withdraw_wrong_phoenix_source_logs =
        &withdraw_wrong_phoenix_source_result.unwrap_err().meta.logs;
    assert_logs_contain(
        withdraw_wrong_phoenix_source_logs,
        "trader Phoenix token account owner mismatch",
    );
    assert_logs_do_not_contain(
        withdraw_wrong_phoenix_source_logs,
        "withdrawable collateral before withdraw",
    );

    if flight_program_available {
        let builder_actor = context.setup_flight_builder("splineTrader", 10, BUILDER_FEE_BPS);

        context.send_fixture_transaction("orderbookPlaceLevels");

        let flight_actor = wrong_destination_actor;
        open_btc_position(&mut context, &flight_actor, &market);
        let builder_collateral_before =
            hawkeye_margin(&mut context, &builder_actor).collateral_quote_lots;
        let flight_usdc_before = token_amount(
            &context,
            &parse_pubkey(&flight_actor.fake_usdc_token_account).unwrap(),
        );
        let builder_close_ix = with_compute_budget(close_position_with_builders_ix(
            &context,
            &flight_actor,
            &builder_actor,
            &market,
            close_position_program_id,
            WithdrawMode::AllFreeCollateral,
        ));
        assert_dynamic_accounts_at_end(&builder_close_ix[1], &context);
        assert_builder_accounts_before_dynamic_tail(&builder_close_ix[1], &context, &builder_actor);

        let mut wrong_flight_program_ix = with_compute_budget(close_position_with_builders_ix(
            &context,
            &flight_actor,
            &builder_actor,
            &market,
            close_position_program_id,
            WithdrawMode::AllFreeCollateral,
        ));
        wrong_flight_program_ix[1].accounts[19].pubkey = test_pubkey(105);
        let wrong_flight_program_result = context
            .try_send_instructions_with_metadata(wrong_flight_program_ix, &flight_actor.seed);
        match &wrong_flight_program_result {
            Ok(tx) => print_transaction_logs("wrong-flight-program-unexpected-success", &tx.logs),
            Err(error) => print_transaction_logs("wrong-flight-program-rejected", &error.meta.logs),
        }
        assert!(
            wrong_flight_program_result.is_err(),
            "builder instruction must reject an unexpected Flight program id"
        );
        let wrong_flight_program_logs = &wrong_flight_program_result.unwrap_err().meta.logs;
        assert_logs_contain(wrong_flight_program_logs, "Flight program id mismatch");
        assert_logs_do_not_contain(wrong_flight_program_logs, "Phoenix Flight: Proxy");
        assert_logs_do_not_contain(
            wrong_flight_program_logs,
            "Phoenix Eternal: Place Market Order",
        );

        let builder_close_logs = send_and_print_logs(
            &mut context,
            builder_close_ix,
            &flight_actor.seed,
            "close-position-builder-withdraw",
        );
        assert_close_fee_math(
            &builder_close_logs,
            flight_actor.phoenix_deposit_atoms as i64 + expected_close_collateral_delta
                - expected_builder_fee as i64,
        );
        assert_all_free_collateral_withdraw(&builder_close_logs);
        assert_builder_fee_log(
            &builder_close_logs,
            builder_fee_base_quote_lots,
            expected_builder_fee,
        );
        assert_eq!(
            hawkeye_asset_base_lots(&mut context, &flight_actor),
            0,
            "Flight builder path should close the position"
        );
        let flight_usdc_after = token_amount(
            &context,
            &parse_pubkey(&flight_actor.fake_usdc_token_account).unwrap(),
        );
        assert!(
            flight_usdc_after > flight_usdc_before,
            "Flight builder path should withdraw collateral"
        );
        let builder_collateral_after =
            hawkeye_margin(&mut context, &builder_actor).collateral_quote_lots;
        assert_eq!(
            builder_collateral_after - builder_collateral_before,
            expected_builder_fee as i64,
            "Flight builder fee should increase builder collateral by the expected amount"
        );
    } else if sdk_localnet_vm_required() {
        panic!("missing local Flight SBF artifact");
    } else {
        eprintln!(
            "skipping Flight builder close-position path because phoenix_flight.so is missing"
        );
    }

    let hot_actor = context.actor("orderbookTrader");
    let hot_withdraw_ix = with_compute_budget(withdraw_all_collateral_ix(
        &context,
        &hot_actor,
        close_position_program_id,
        None,
    ));
    assert_dynamic_accounts_at_end(&hot_withdraw_ix[1], &context);
    assert_withdraw_all_collateral_omits_market_accounts(&hot_withdraw_ix[1], &market);
    let hot_withdraw_logs = send_and_print_logs(
        &mut context,
        hot_withdraw_ix,
        &hot_actor.seed,
        "withdraw-hot-trader-collateral",
    );
    assert_logs_contain(&hot_withdraw_logs, "trader is hot");
    assert_logs_contain(
        &hot_withdraw_logs,
        "withdrawable collateral before withdraw",
    );
    assert_logs_do_not_contain(&hot_withdraw_logs, "Phoenix Eternal: Place Market Order");
    assert_logs_do_not_contain(&hot_withdraw_logs, "Phoenix Flight: Proxy");
}

fn with_compute_budget(ix: Instruction) -> Vec<Instruction> {
    vec![
        ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
        ix,
    ]
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
        "open-btc-position" => "Opening BTC position...".to_string(),
        "close-position-diff-withdraw" | "close-position-all-withdraw" => {
            "Closing position and withdrawing...".to_string()
        }
        "close-position-builder-withdraw" => {
            "Closing position and withdrawing with builder...".to_string()
        }
        "withdraw-all-collateral" => "Withdrawing all collateral...".to_string(),
        "withdraw-hot-trader-collateral" => "Withdrawing hot trader collateral...".to_string(),
        "wrong-destination-rejected" => "Rejecting wrong withdrawal destination...".to_string(),
        "wrong-destination-unexpected-success" => {
            "Unexpected wrong withdrawal destination success...".to_string()
        }
        "withdraw-wrong-destination-rejected" => {
            "Rejecting wrong withdraw-only destination...".to_string()
        }
        "withdraw-wrong-destination-unexpected-success" => {
            "Unexpected wrong withdraw-only destination success...".to_string()
        }
        "withdraw-pda-destination-rejected" => {
            "Rejecting PDA withdrawal destination...".to_string()
        }
        "withdraw-pda-destination-unexpected-success" => {
            "Unexpected PDA withdrawal destination success...".to_string()
        }
        "withdraw-wrong-mint-rejected" => "Rejecting wrong withdraw mint...".to_string(),
        "withdraw-wrong-mint-unexpected-success" => {
            "Unexpected wrong withdraw mint success...".to_string()
        }
        "withdraw-wrong-phoenix-source-rejected" => {
            "Rejecting wrong Phoenix-token source...".to_string()
        }
        "withdraw-wrong-phoenix-source-unexpected-success" => {
            "Unexpected wrong Phoenix-token source success...".to_string()
        }
        "hawkeye-view-asset" => "Viewing Hawkeye asset margin...".to_string(),
        "hawkeye-view-margin" => "Viewing Hawkeye margin...".to_string(),
        _ => format!("{}...", label.replace('-', " ")),
    }
}

fn assert_logs_contain(logs: &[String], needle: &str) {
    assert!(
        logs.iter().any(|log| log.contains(needle)),
        "expected logs to contain {needle:?}"
    );
}

fn assert_logs_do_not_contain(logs: &[String], needle: &str) {
    assert!(
        logs.iter().all(|log| !log.contains(needle)),
        "expected logs not to contain {needle:?}"
    );
}

fn assert_withdraw_all_rejects_program_id(
    context: &mut SdkLocalnetContext,
    actor: &FixtureActor,
    program_id: Pubkey,
    account_index: usize,
    wrong_program_id: Pubkey,
    expected_log: &str,
    label: &str,
) {
    let mut ix = with_compute_budget(withdraw_all_collateral_ix(context, actor, program_id, None));
    ix[1].accounts[account_index].pubkey = wrong_program_id;
    let result = context.try_send_instructions_with_metadata(ix, &actor.seed);
    match &result {
        Ok(tx) => print_transaction_logs(&format!("{label}-unexpected-success"), &tx.logs),
        Err(error) => print_transaction_logs(&format!("{label}-rejected"), &error.meta.logs),
    }
    assert!(
        result.is_err(),
        "withdraw-only instruction must reject {expected_log}"
    );
    let logs = &result.unwrap_err().meta.logs;
    assert_logs_contain(logs, expected_log);
    assert_logs_do_not_contain(logs, "withdrawable collateral before withdraw");
    assert_logs_do_not_contain(logs, "Phoenix Eternal:");
    assert_logs_do_not_contain(logs, "EMBERp");
}

fn assert_close_fee_math(logs: &[String], expected_withdrawable_after_order: i64) {
    assert_eq!(
        log_value_i64(logs, "withdrawable collateral after order"),
        expected_withdrawable_after_order,
        "closed collateral should equal deposit plus realized PnL minus Phoenix taker fees"
    );
}

fn assert_order_free_collateral_delta_withdraw(logs: &[String]) {
    let withdrawable_before_order = log_value_u64(logs, "withdrawable collateral before order");
    let withdrawable_after_order = log_value_u64(logs, "withdrawable collateral after order");
    let amount_to_withdraw = log_value_u64(logs, "collateral amount to withdraw");
    assert_eq!(
        amount_to_withdraw,
        withdrawable_after_order - withdrawable_before_order,
        "diff mode should withdraw only the collateral freed by the close order"
    );
}

fn assert_all_free_collateral_withdraw(logs: &[String]) {
    let withdrawable_after_order = log_value_u64(logs, "withdrawable collateral after order");
    let amount_to_withdraw = log_value_u64(logs, "collateral amount to withdraw");
    assert_eq!(
        amount_to_withdraw, withdrawable_after_order,
        "all-free-collateral mode should withdraw the full post-order withdrawable collateral"
    );
}

fn assert_builder_fee_log(logs: &[String], expected_trade_amount: u64, expected_fee: u64) {
    let Some(log) = logs
        .iter()
        .find(|log| log.contains("Found trade amount") && log.contains("collected fee"))
    else {
        panic!("expected Flight builder fee log");
    };
    let Some((trade_amount_text, fee_text)) = log
        .strip_prefix("Program log: Found trade amount ")
        .and_then(|rest| rest.split_once(", collected fee: "))
    else {
        panic!("unexpected Flight builder fee log format: {log}");
    };
    assert_eq!(
        trade_amount_text.parse::<u64>().unwrap(),
        expected_trade_amount,
        "Flight should compute builder fees from the post-Phoenix close proceeds"
    );
    assert_eq!(
        fee_text.parse::<u64>().unwrap(),
        expected_fee,
        "Flight should collect the expected builder fee"
    );
}

fn assert_withdraw_all_collateral_omits_market_accounts(ix: &Instruction, market: &FixtureMarket) {
    let orderbook = parse_pubkey(&market.orderbook).unwrap();
    let spline = parse_pubkey(&market.spline).unwrap();
    assert!(
        ix.accounts
            .iter()
            .all(|account| account.pubkey != orderbook),
        "withdraw-all collateral instruction should not require the orderbook account"
    );
    assert!(
        ix.accounts.iter().all(|account| account.pubkey != spline),
        "withdraw-all collateral instruction should not require the spline account"
    );
}

fn assert_dynamic_accounts_at_end(ix: &Instruction, context: &SdkLocalnetContext) {
    let dynamic_accounts = dynamic_account_pubkeys(context);
    let dynamic_start = ix
        .accounts
        .len()
        .checked_sub(dynamic_accounts.len())
        .expect("instruction should contain dynamic accounts");
    let tail_accounts = ix.accounts[dynamic_start..]
        .iter()
        .map(|account| account.pubkey)
        .collect::<Vec<_>>();
    assert_eq!(
        tail_accounts, dynamic_accounts,
        "global_trader_index and active_trader_buffer accounts should be the instruction tail"
    );
    assert!(
        ix.accounts[..dynamic_start]
            .iter()
            .all(|account| !dynamic_accounts.contains(&account.pubkey)),
        "dynamic accounts should not appear before the tail"
    );
}

fn assert_builder_accounts_before_dynamic_tail(
    ix: &Instruction,
    context: &SdkLocalnetContext,
    builder_actor: &FixtureActor,
) {
    let dynamic_start = ix.accounts.len() - dynamic_account_pubkeys(context).len();
    let builder_authority = parse_pubkey(&builder_actor.pubkey).unwrap();
    let flight_accounts = ix.accounts[dynamic_start - 5..dynamic_start]
        .iter()
        .map(|account| account.pubkey)
        .collect::<Vec<_>>();
    assert_eq!(
        flight_accounts,
        vec![
            FLIGHT_PROGRAM_ID,
            builder_authority,
            parse_pubkey(&builder_actor.trader_account).unwrap(),
            get_flight_global_state_address().expect("derive flight global-state PDA"),
            get_flight_builder_state_address(&builder_authority)
                .expect("derive flight builder-state PDA"),
        ],
        "Flight builder accounts should be fixed accounts immediately before the dynamic tail"
    );
}

fn dynamic_account_pubkeys(context: &SdkLocalnetContext) -> Vec<Pubkey> {
    parse_pubkeys(&context.fixture.addresses.global_trader_index)
        .into_iter()
        .chain(parse_pubkeys(
            &context.fixture.addresses.active_trader_buffer,
        ))
        .collect()
}

fn quote_lots_for_price(
    market: &FixtureMarket,
    price_usd: f64,
    quote_decimals: u8,
    base_lots: u64,
) -> u64 {
    market.price_usd_to_ticks(price_usd, quote_decimals)
        * market.tick_size_in_quote_lots_per_base_lot
        * base_lots
}

fn taker_fee_quote_lots(quote_lots: u64, taker_fee_micro: u32) -> u64 {
    quote_lots * taker_fee_micro as u64 / MICRO_FEE_DENOMINATOR
}

fn builder_fee_quote_lots(quote_lots: u64) -> u64 {
    quote_lots * BUILDER_FEE_BPS / BPS_DENOMINATOR
}

fn test_pubkey(byte: u8) -> Pubkey {
    Pubkey::new_from_array([byte; 32])
}

fn log_value_i64(logs: &[String], label: &str) -> i64 {
    log_value_u64(logs, label) as i64
}

fn log_value_u64(logs: &[String], label: &str) -> u64 {
    let needle = format!("{label}: ");
    logs.iter()
        .find_map(|log| {
            log.split_once(&needle)
                .map(|(_, value)| value.parse::<u64>().unwrap())
        })
        .unwrap_or_else(|| panic!("expected log value for {label:?}"))
}

fn open_btc_position(
    context: &mut SdkLocalnetContext,
    actor: &FixtureActor,
    market: &FixtureMarket,
) {
    let params = MarketOrderParams::builder()
        .trader(parse_pubkey(&actor.pubkey).unwrap())
        .trader_account(parse_pubkey(&actor.trader_account).unwrap())
        .perp_asset_map(parse_pubkey(&context.fixture.addresses.perp_asset_map).unwrap())
        .orderbook(parse_pubkey(&market.orderbook).unwrap())
        .spline_collection(parse_pubkey(&market.spline).unwrap())
        .global_trader_index(parse_pubkeys(
            &context.fixture.addresses.global_trader_index,
        ))
        .active_trader_buffer(parse_pubkeys(
            &context.fixture.addresses.active_trader_buffer,
        ))
        .side(Side::Bid)
        .num_base_lots(BTC_ORDER_BASE_LOTS)
        .min_base_lots_to_fill(BTC_ORDER_BASE_LOTS)
        .min_quote_lots_to_fill(1)
        .self_trade_behavior(SelfTradeBehavior::CancelProvide)
        .order_flags(OrderFlags::None)
        .build()
        .unwrap();
    let ix = create_place_market_order_ix(params).unwrap();
    send_and_print_logs(context, vec![ix.into()], &actor.seed, "open-btc-position");
}

fn close_position_ix(
    context: &SdkLocalnetContext,
    actor: &FixtureActor,
    market: &FixtureMarket,
    program_id: Pubkey,
    withdraw_mode: WithdrawMode,
    usdc_token_account_override: Option<Pubkey>,
) -> Instruction {
    close_position_ix_inner(
        context,
        actor,
        market,
        program_id,
        withdraw_mode,
        usdc_token_account_override,
        "global:close_position_and_withdraw",
    )
}

fn close_position_with_builders_ix(
    context: &SdkLocalnetContext,
    actor: &FixtureActor,
    builder_actor: &FixtureActor,
    market: &FixtureMarket,
    program_id: Pubkey,
    withdraw_mode: WithdrawMode,
) -> Instruction {
    let mut ix = close_position_ix_inner(
        context,
        actor,
        market,
        program_id,
        withdraw_mode,
        None,
        "global:close_position_and_withdraw_with_builders",
    );
    let builder_authority = parse_pubkey(&builder_actor.pubkey).unwrap();
    ix.accounts.splice(
        19..19,
        [
            AccountMeta::new_readonly(FLIGHT_PROGRAM_ID, false),
            AccountMeta::new_readonly(builder_authority, false),
            AccountMeta::new(parse_pubkey(&builder_actor.trader_account).unwrap(), false),
            AccountMeta::new_readonly(
                get_flight_global_state_address().expect("derive flight global-state PDA"),
                false,
            ),
            AccountMeta::new_readonly(
                get_flight_builder_state_address(&builder_authority)
                    .expect("derive flight builder-state PDA"),
                false,
            ),
        ],
    );
    ix
}

fn close_position_ix_inner(
    context: &SdkLocalnetContext,
    actor: &FixtureActor,
    market: &FixtureMarket,
    program_id: Pubkey,
    withdraw_mode: WithdrawMode,
    usdc_token_account_override: Option<Pubkey>,
    discriminant: &str,
) -> Instruction {
    let params = ClosePositionAndWithdrawParams {
        withdraw_mode,
        side: CloseSide::Ask,
        price_in_ticks: None,
        num_base_lots: BTC_ORDER_BASE_LOTS,
        num_quote_lots: None,
        min_base_lots_to_fill: BTC_ORDER_BASE_LOTS,
        min_quote_lots_to_fill: 1,
        self_trade_behavior: CloseSelfTradeBehavior::CancelProvide,
        match_limit: None,
        client_order_id: 0,
        last_valid_slot: None,
        cancel_existing: false,
        global_trader_index_count: context.fixture.addresses.global_trader_index.len() as u8,
        active_trader_buffer_count: context.fixture.addresses.active_trader_buffer.len() as u8,
    };
    let mut data = compute_discriminant(discriminant).to_vec();
    data.extend_from_slice(&to_vec(&params).unwrap());

    let fake_usdc_mint = context
        .fixture
        .mints
        .iter()
        .find(|mint| mint.name == "fakeUsdc")
        .expect("fixture should include fake USDC mint");
    let phoenix_mint = context
        .fixture
        .mints
        .iter()
        .find(|mint| mint.name == "phoenixCollateral")
        .expect("fixture should include Phoenix collateral mint");
    let usdc_token_account = usdc_token_account_override
        .unwrap_or_else(|| parse_pubkey(&actor.fake_usdc_token_account).unwrap());

    let mut accounts = vec![
        AccountMeta::new_readonly(parse_pubkey(&actor.pubkey).unwrap(), true),
        AccountMeta::new_readonly(*PHOENIX_PROGRAM_ID, false),
        AccountMeta::new_readonly(HAWKEYE_PROGRAM_ID, false),
        AccountMeta::new_readonly(EMBER_PROGRAM_ID, false),
        AccountMeta::new_readonly(*PHOENIX_LOG_AUTHORITY, false),
        AccountMeta::new(*PHOENIX_GLOBAL_CONFIGURATION, false),
        AccountMeta::new(parse_pubkey(&actor.trader_account).unwrap(), false),
        AccountMeta::new(
            parse_pubkey(&context.fixture.addresses.perp_asset_map).unwrap(),
            false,
        ),
        AccountMeta::new(
            parse_pubkey(&context.fixture.addresses.global_vault).unwrap(),
            false,
        ),
        AccountMeta::new(parse_pubkey(&actor.phoenix_token_account).unwrap(), false),
        AccountMeta::new(
            parse_pubkey(&context.fixture.addresses.withdraw_queue).unwrap(),
            false,
        ),
        AccountMeta::new_readonly(parse_pubkey(&fake_usdc_mint.pubkey).unwrap(), false),
        AccountMeta::new(parse_pubkey(&phoenix_mint.pubkey).unwrap(), false),
        AccountMeta::new(usdc_token_account, false),
        AccountMeta::new_readonly(
            parse_pubkey(&context.fixture.addresses.ember_state).unwrap(),
            false,
        ),
        AccountMeta::new(
            parse_pubkey(&context.fixture.addresses.ember_vault).unwrap(),
            false,
        ),
        AccountMeta::new_readonly(SPL_TOKEN_PROGRAM_ID, false),
        AccountMeta::new(parse_pubkey(&market.orderbook).unwrap(), false),
        AccountMeta::new(parse_pubkey(&market.spline).unwrap(), false),
    ];
    accounts.extend(
        parse_pubkeys(&context.fixture.addresses.global_trader_index)
            .into_iter()
            .map(|pubkey| AccountMeta::new(pubkey, false)),
    );
    accounts.extend(
        parse_pubkeys(&context.fixture.addresses.active_trader_buffer)
            .into_iter()
            .map(|pubkey| AccountMeta::new(pubkey, false)),
    );

    Instruction {
        program_id,
        accounts,
        data,
    }
}

fn withdraw_all_collateral_ix(
    context: &SdkLocalnetContext,
    actor: &FixtureActor,
    program_id: Pubkey,
    usdc_token_account_override: Option<Pubkey>,
) -> Instruction {
    withdraw_all_collateral_ix_with_overrides(
        context,
        actor,
        program_id,
        Some(WithdrawAllAccountOverrides {
            usdc_token_account: usdc_token_account_override,
            ..WithdrawAllAccountOverrides::default()
        }),
    )
}

#[derive(Default)]
struct WithdrawAllAccountOverrides {
    phoenix_token_account: Option<Pubkey>,
    usdc_mint: Option<Pubkey>,
    canonical_mint: Option<Pubkey>,
    usdc_token_account: Option<Pubkey>,
}

fn withdraw_all_collateral_ix_with_overrides(
    context: &SdkLocalnetContext,
    actor: &FixtureActor,
    program_id: Pubkey,
    overrides: Option<WithdrawAllAccountOverrides>,
) -> Instruction {
    let params = WithdrawAllCollateralParams {
        global_trader_index_count: context.fixture.addresses.global_trader_index.len() as u8,
        active_trader_buffer_count: context.fixture.addresses.active_trader_buffer.len() as u8,
    };
    let mut data = compute_discriminant("global:withdraw_all_collateral").to_vec();
    data.extend_from_slice(&to_vec(&params).unwrap());

    let fake_usdc_mint = context
        .fixture
        .mints
        .iter()
        .find(|mint| mint.name == "fakeUsdc")
        .expect("fixture should include fake USDC mint");
    let phoenix_mint = context
        .fixture
        .mints
        .iter()
        .find(|mint| mint.name == "phoenixCollateral")
        .expect("fixture should include Phoenix collateral mint");
    let overrides = overrides.unwrap_or_default();
    let usdc_mint = overrides
        .usdc_mint
        .unwrap_or_else(|| parse_pubkey(&fake_usdc_mint.pubkey).unwrap());
    let canonical_mint = overrides
        .canonical_mint
        .unwrap_or_else(|| parse_pubkey(&phoenix_mint.pubkey).unwrap());
    let usdc_token_account = overrides
        .usdc_token_account
        .unwrap_or_else(|| parse_pubkey(&actor.fake_usdc_token_account).unwrap());
    let phoenix_token_account = overrides
        .phoenix_token_account
        .unwrap_or_else(|| parse_pubkey(&actor.phoenix_token_account).unwrap());

    let mut accounts = vec![
        AccountMeta::new_readonly(parse_pubkey(&actor.pubkey).unwrap(), true),
        AccountMeta::new_readonly(*PHOENIX_PROGRAM_ID, false),
        AccountMeta::new_readonly(HAWKEYE_PROGRAM_ID, false),
        AccountMeta::new_readonly(EMBER_PROGRAM_ID, false),
        AccountMeta::new_readonly(*PHOENIX_LOG_AUTHORITY, false),
        AccountMeta::new(*PHOENIX_GLOBAL_CONFIGURATION, false),
        AccountMeta::new(parse_pubkey(&actor.trader_account).unwrap(), false),
        AccountMeta::new(
            parse_pubkey(&context.fixture.addresses.perp_asset_map).unwrap(),
            false,
        ),
        AccountMeta::new(
            parse_pubkey(&context.fixture.addresses.global_vault).unwrap(),
            false,
        ),
        AccountMeta::new(phoenix_token_account, false),
        AccountMeta::new(
            parse_pubkey(&context.fixture.addresses.withdraw_queue).unwrap(),
            false,
        ),
        AccountMeta::new_readonly(usdc_mint, false),
        AccountMeta::new(canonical_mint, false),
        AccountMeta::new(usdc_token_account, false),
        AccountMeta::new_readonly(
            parse_pubkey(&context.fixture.addresses.ember_state).unwrap(),
            false,
        ),
        AccountMeta::new(
            parse_pubkey(&context.fixture.addresses.ember_vault).unwrap(),
            false,
        ),
        AccountMeta::new_readonly(SPL_TOKEN_PROGRAM_ID, false),
    ];
    accounts.extend(
        parse_pubkeys(&context.fixture.addresses.global_trader_index)
            .into_iter()
            .map(|pubkey| AccountMeta::new(pubkey, false)),
    );
    accounts.extend(
        parse_pubkeys(&context.fixture.addresses.active_trader_buffer)
            .into_iter()
            .map(|pubkey| AccountMeta::new(pubkey, false)),
    );

    Instruction {
        program_id,
        accounts,
        data,
    }
}

fn hawkeye_asset_base_lots(context: &mut SdkLocalnetContext, actor: &FixtureActor) -> i64 {
    hawkeye_asset(context, actor).base_lots
}

fn hawkeye_asset(context: &mut SdkLocalnetContext, actor: &FixtureActor) -> ViewAssetReturn {
    let tx = context.send_instructions_with_metadata(
        vec![
            create_hawkeye_view_margin_for_asset_ix(
                hawkeye_trader_view_accounts(context, actor),
                BTC_ASSET_ID,
            )
            .into(),
        ],
        "payer",
        "hawkeye-view-asset",
    );
    print_transaction_logs("hawkeye-view-asset", &tx.logs);
    let HawkeyeReturnData::Asset(asset) =
        decode_hawkeye_return_data(&tx.return_data.data).expect("decode Hawkeye asset")
    else {
        panic!("expected Hawkeye asset return");
    };
    asset
}

fn hawkeye_margin(context: &mut SdkLocalnetContext, actor: &FixtureActor) -> ViewMarginReturn {
    let tx = context.send_instructions_with_metadata(
        vec![create_hawkeye_view_margin_ix(hawkeye_trader_view_accounts(context, actor)).into()],
        "payer",
        "hawkeye-view-margin",
    );
    print_transaction_logs("hawkeye-view-margin", &tx.logs);
    let HawkeyeReturnData::Margin(margin) =
        decode_hawkeye_return_data(&tx.return_data.data).expect("decode Hawkeye margin")
    else {
        panic!("expected Hawkeye margin return");
    };
    margin
}

fn hawkeye_trader_view_accounts(
    context: &SdkLocalnetContext,
    actor: &FixtureActor,
) -> HawkeyeTraderViewAccounts {
    HawkeyeTraderViewAccounts {
        phoenix_program_id: *PHOENIX_PROGRAM_ID,
        global_config: *PHOENIX_GLOBAL_CONFIGURATION,
        global_trader_index: parse_pubkeys(&context.fixture.addresses.global_trader_index),
        active_trader_buffer: parse_pubkeys(&context.fixture.addresses.active_trader_buffer),
        perp_asset_map: parse_pubkey(&context.fixture.addresses.perp_asset_map).unwrap(),
        trader: parse_pubkey(&actor.trader_account).unwrap(),
    }
}

fn token_amount(context: &SdkLocalnetContext, token_account: &Pubkey) -> u64 {
    context.token_amount(token_account)
}

fn install_pda_owned_usdc_token_account(
    context: &mut SdkLocalnetContext,
    program_id: Pubkey,
) -> Pubkey {
    let (owner, _) = Pubkey::find_program_address(&[b"pda-usdc-owner"], &program_id);
    let (token_account, _) =
        Pubkey::find_program_address(&[b"pda-usdc-token-account"], &program_id);
    install_spl_token_account(
        context,
        token_account,
        fixture_mint_pubkey(context, "fakeUsdc"),
        owner,
        0,
    );
    token_account
}

fn install_spl_token_account(
    context: &mut SdkLocalnetContext,
    token_account: Pubkey,
    mint: Pubkey,
    owner: Pubkey,
    amount: u64,
) {
    let mut data = vec![0; SPL_TOKEN_ACCOUNT_LEN];
    data[..32].copy_from_slice(&mint.to_bytes());
    data[32..64].copy_from_slice(&owner.to_bytes());
    data[SPL_TOKEN_ACCOUNT_AMOUNT_OFFSET..SPL_TOKEN_ACCOUNT_AMOUNT_OFFSET + 8]
        .copy_from_slice(&amount.to_le_bytes());
    data[SPL_TOKEN_ACCOUNT_STATE_OFFSET] = 1;
    context
        .svm
        .set_account(
            token_account,
            Account {
                lamports: 10_000_000,
                data,
                owner: SPL_TOKEN_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        )
        .expect("install test token account");
}

fn fixture_mint_pubkey(context: &SdkLocalnetContext, name: &str) -> Pubkey {
    parse_pubkey(
        &context
            .fixture
            .mints
            .iter()
            .find(|mint| mint.name == name)
            .unwrap_or_else(|| panic!("fixture should include {name} mint"))
            .pubkey,
    )
    .unwrap()
}

fn find_close_position_program_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var(CLOSE_POSITION_PROGRAM_ENV) {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }

    for root in default_roots() {
        for path in [
            root.join(
                "rise/programs/close-position-and-withdraw/target/deploy/\
                 rise_close_position_and_withdraw.so",
            ),
            root.join("target/deploy/rise_close_position_and_withdraw.so"),
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
