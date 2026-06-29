pub mod common;

use common::{
    BTC_BASE_LOTS, assert_logs_contain, close_subaccount_and_sweep_ix, prime_market,
    quote_lots_for_price, register_subaccount_and_trade_ix, send_and_print_logs, setup_context,
    trader_pda, with_compute_budget,
};
use phoenix_rise::ix::types::{OrderFlags, SelfTradeBehavior, Side};
use phoenix_rise_litesvm_test::parse_pubkey;
use rise_example_program::MarketOrderWithoutArenas;

#[test]
fn subaccount_flow_executes_and_sweeps() {
    let Some((mut context, program_id)) = setup_context() else {
        return;
    };
    prime_market(&mut context);

    let actor = context.actor("taker1");
    let market = context.market("BTC");
    let quote_decimals = context.fixture_mint("fakeUsdc").decimals;
    let parent_trader = parse_pubkey(&actor.trader_account).unwrap();
    let child_subaccount_index = 1;
    let child_trader = trader_pda(
        &parse_pubkey(&actor.pubkey).unwrap(),
        0,
        child_subaccount_index,
    );

    let open_subaccount = with_compute_budget(register_subaccount_and_trade_ix(
        &context,
        &actor,
        &market,
        program_id,
        parent_trader,
        child_trader,
        child_subaccount_index,
        MarketOrderWithoutArenas {
            side: Side::Bid,
            price_in_ticks: Some(market.price_usd_to_ticks(100_500.0, quote_decimals)),
            num_base_lots: BTC_BASE_LOTS,
            num_quote_lots: Some(quote_lots_for_price(
                &market,
                100_500.0,
                quote_decimals,
                BTC_BASE_LOTS,
            )),
            min_base_lots_to_fill: BTC_BASE_LOTS.saturating_sub(1),
            min_quote_lots_to_fill: 0,
            self_trade_behavior: SelfTradeBehavior::CancelProvide,
            match_limit: None,
            client_order_id: 31,
            last_valid_slot: None,
            order_flags: OrderFlags::None,
            cancel_existing: false,
        },
    ));
    let open_logs = send_and_print_logs(
        &mut context,
        open_subaccount,
        &actor.seed,
        "register-subaccount-and-trade",
    );
    assert_logs_contain(
        &open_logs,
        "rise-example-program: Phoenix register subaccount",
    );
    assert_logs_contain(
        &open_logs,
        "rise-example-program: Phoenix sync parent to child",
    );
    assert_logs_contain(
        &open_logs,
        "rise-example-program: Phoenix transfer collateral to child",
    );
    assert_logs_contain(&open_logs, "subaccount-open-order matching response");
    assert_logs_contain(&open_logs, "subaccount-open-order Hawkeye margin");

    let close_subaccount = with_compute_budget(close_subaccount_and_sweep_ix(
        &context,
        &actor,
        &market,
        program_id,
        parent_trader,
        child_trader,
        MarketOrderWithoutArenas {
            side: Side::Ask,
            price_in_ticks: Some(market.price_usd_to_ticks(99_500.0, quote_decimals)),
            num_base_lots: BTC_BASE_LOTS,
            num_quote_lots: None,
            min_base_lots_to_fill: BTC_BASE_LOTS.saturating_sub(1),
            min_quote_lots_to_fill: 0,
            self_trade_behavior: SelfTradeBehavior::CancelProvide,
            match_limit: None,
            client_order_id: 32,
            last_valid_slot: None,
            order_flags: OrderFlags::ReduceOnly,
            cancel_existing: false,
        },
    ));
    let close_logs = send_and_print_logs(
        &mut context,
        close_subaccount,
        &actor.seed,
        "close-subaccount-and-sweep",
    );
    assert_logs_contain(&close_logs, "subaccount-close-order matching response");
    assert_logs_contain(&close_logs, "subaccount-close-order Hawkeye margin");
    assert_logs_contain(
        &close_logs,
        "rise-example-program: Phoenix sweep child collateral to parent",
    );
}
