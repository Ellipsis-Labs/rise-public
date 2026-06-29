pub mod common;

use common::{
    DEPOSIT_AMOUNT, assert_logs_contain, deposit_ix, send_and_print_logs, setup_context,
    with_compute_budget, withdraw_ix,
};
use phoenix_rise_litesvm_test::parse_pubkey;

#[test]
fn deposit_and_withdraw_flow_executes() {
    let Some((mut context, program_id)) = setup_context() else {
        return;
    };
    let actor = context.actor("taker0");
    let usdc_account = parse_pubkey(&actor.fake_usdc_token_account).unwrap();
    let usdc_before = context.token_amount(&usdc_account);

    let deposit_instructions =
        with_compute_budget(deposit_ix(&context, &actor, program_id, DEPOSIT_AMOUNT));
    let deposit_logs = send_and_print_logs(
        &mut context,
        deposit_instructions,
        &actor.seed,
        "deposit-ember-then-phoenix",
    );
    assert_logs_contain(&deposit_logs, "rise-example-program: Ember deposit");
    assert_logs_contain(&deposit_logs, "rise-example-program: Phoenix deposit");
    assert!(
        context.token_amount(&usdc_account) < usdc_before,
        "deposit should move USDC out of the actor token account"
    );

    context.svm.warp_to_slot(351);

    let withdraw_instructions = with_compute_budget(withdraw_ix(
        &context,
        &actor,
        program_id,
        DEPOSIT_AMOUNT,
        Some(DEPOSIT_AMOUNT),
    ));
    let withdraw_logs = send_and_print_logs(
        &mut context,
        withdraw_instructions,
        &actor.seed,
        "withdraw-phoenix-then-ember",
    );
    assert_logs_contain(&withdraw_logs, "rise-example-program: Phoenix withdraw");
    assert_logs_contain(&withdraw_logs, "rise-example-program: Ember withdraw");
    assert_eq!(
        context.token_amount(&usdc_account),
        usdc_before,
        "withdraw should unwrap the same amount back to USDC"
    );
}
