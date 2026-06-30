pub mod common;

use common::{
    assert_logs_contain, prime_market, send_and_print_logs, setup_context, show_orderbook_bbo_ix,
};

#[test]
fn orderbook_bbo_deserializes_in_program() {
    let Some((mut context, program_id)) = setup_context() else {
        return;
    };
    prime_market(&mut context);

    let market = context.market("BTC");
    let actor = context.actor("taker0");
    let logs = send_and_print_logs(
        &mut context,
        vec![show_orderbook_bbo_ix(&market, program_id)],
        &actor.seed,
        "show-orderbook-bbo",
    );

    assert_logs_contain(&logs, "rise-example-program: orderbook bbo market=BTC");
    assert_logs_contain(
        &logs,
        "rise-example-program: orderbook best_bid price_in_ticks=",
    );
    assert_logs_contain(
        &logs,
        "rise-example-program: orderbook best_ask price_in_ticks=",
    );
}
