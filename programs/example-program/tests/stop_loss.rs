pub mod common;

use common::{
    BTC_ASSET_ID, BTC_BASE_LOTS, assert_logs_contain, cancel_stop_loss_ix, place_market_ix,
    place_stop_loss_ix, prime_market, quote_lots_for_price, send_and_print_logs, setup_context,
    with_compute_budget,
};
use phoenix_rise::ix::constants::get_stop_loss_address;
use phoenix_rise::ix::types::{OrderFlags, Side};
use phoenix_rise_litesvm_test::parse_pubkey;

const RENT_EXEMPT_EMPTY_ACCOUNT_LAMPORTS: u64 = 890_880;

#[test]
fn stop_loss_flow_executes() {
    let Some((mut context, program_id)) = setup_context() else {
        return;
    };
    prime_market(&mut context);

    let actor = context.actor("stopLossTaker");
    let market = context.market("BTC");
    let quote_decimals = context.fixture_mint("fakeUsdc").decimals;
    let open_stop_loss_position = with_compute_budget(place_market_ix(
        &context,
        &actor,
        &market,
        program_id,
        Side::Bid,
        BTC_BASE_LOTS,
        quote_lots_for_price(&market, 100_500.0, quote_decimals, BTC_BASE_LOTS),
        21,
        OrderFlags::None,
    ));
    let open_stop_loss_logs = send_and_print_logs(
        &mut context,
        open_stop_loss_position,
        &actor.seed,
        "open-stop-loss-position",
    );
    assert_logs_contain(&open_stop_loss_logs, "place-market Hawkeye margin");

    let stop_loss =
        get_stop_loss_address(&parse_pubkey(&actor.trader_account).unwrap(), BTC_ASSET_ID)
            .expect("derive stop-loss PDA");
    context
        .svm
        .airdrop(&stop_loss, RENT_EXEMPT_EMPTY_ACCOUNT_LAMPORTS)
        .expect("seed empty stop-loss PDA account");
    let place_stop_loss = with_compute_budget(place_stop_loss_ix(
        &context,
        &actor,
        &market,
        program_id,
        stop_loss,
        market.price_usd_to_ticks(99_000.0, quote_decimals),
        market.price_usd_to_ticks(98_500.0, quote_decimals),
    ));
    let place_logs = send_and_print_logs(
        &mut context,
        place_stop_loss,
        &actor.seed,
        "place-stop-loss",
    );
    assert_logs_contain(&place_logs, "rise-example-program: Phoenix place stop loss");

    let cancel_stop_loss =
        with_compute_budget(cancel_stop_loss_ix(&context, &actor, program_id, stop_loss));
    let cancel_logs = send_and_print_logs(
        &mut context,
        cancel_stop_loss,
        &actor.seed,
        "cancel-stop-loss",
    );
    assert_logs_contain(
        &cancel_logs,
        "rise-example-program: Phoenix cancel stop loss",
    );
}
