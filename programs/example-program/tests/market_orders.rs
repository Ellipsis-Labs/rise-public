pub mod common;

use common::{
    BTC_BASE_LOTS, LIMIT_BASE_LOTS, assert_logs_contain, cancel_limit_ix, matching_order_id,
    place_limit_ix, place_market_ix, prime_market, quote_lots_for_price, send_and_print_logs,
    setup_context, with_compute_budget,
};
use phoenix_rise::ix::types::{OrderFlags, Side};

#[test]
fn market_limit_and_cancel_flow_executes() {
    let Some((mut context, program_id)) = setup_context() else {
        return;
    };
    prime_market(&mut context);

    let actor = context.actor("taker0");
    let market = context.market("BTC");
    let quote_decimals = context.fixture_mint("fakeUsdc").decimals;
    let market_instructions = with_compute_budget(place_market_ix(
        &context,
        &actor,
        &market,
        program_id,
        Side::Bid,
        BTC_BASE_LOTS,
        quote_lots_for_price(&market, 100_500.0, quote_decimals, BTC_BASE_LOTS),
        11,
        OrderFlags::None,
    ));
    let market_logs = send_and_print_logs(
        &mut context,
        market_instructions,
        &actor.seed,
        "place-market-order",
    );
    assert_logs_contain(
        &market_logs,
        "rise-example-program: Phoenix place market order",
    );
    assert_logs_contain(&market_logs, "place-market matching response");
    assert_logs_contain(&market_logs, "place-market Hawkeye margin");

    let limit_price = market.price_usd_to_ticks(90_000.0, quote_decimals);
    let limit_instructions = with_compute_budget(place_limit_ix(
        &context,
        &actor,
        &market,
        program_id,
        Side::Bid,
        limit_price,
        LIMIT_BASE_LOTS,
        12,
    ));
    let limit_logs = send_and_print_logs(
        &mut context,
        limit_instructions,
        &actor.seed,
        "place-limit-order",
    );
    assert_logs_contain(
        &limit_logs,
        "rise-example-program: Phoenix place limit order",
    );
    let (order_price, order_sequence_number) = matching_order_id(&limit_logs, "place-limit");
    assert_eq!(order_price, limit_price);

    let cancel_instructions = with_compute_budget(cancel_limit_ix(
        &context,
        &actor,
        &market,
        program_id,
        order_price,
        order_sequence_number,
    ));
    let cancel_logs = send_and_print_logs(
        &mut context,
        cancel_instructions,
        &actor.seed,
        "cancel-limit-order",
    );
    assert_logs_contain(
        &cancel_logs,
        "rise-example-program: Phoenix cancel limit order",
    );
}
