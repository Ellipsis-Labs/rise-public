pub mod common;

use common::{claim_escrow_ix, setup_context, with_compute_budget};
use phoenix_rise::accounts::owned::Trader;
use phoenix_rise::ix::constants::{PHOENIX_PROGRAM_ID, SYSTEM_PROGRAM_ID};
use phoenix_rise::ix::delegate_trader::{DelegateTraderParams, create_delegate_trader_ix};
use phoenix_rise_litesvm_test::SdkLocalnetContext;
use solana_instruction::error::InstructionError;
use solana_pubkey::Pubkey;
use solana_transaction_error::TransactionError;

const ESCROW_AMOUNT: i64 = 2_500_000;

fn claimer_pda(context: &SdkLocalnetContext, actor: &str, program_id: Pubkey) -> Pubkey {
    let trader_account = context.actor_trader(actor);
    Pubkey::find_program_address(&[b"escrow_claimer", trader_account.as_ref()], &program_id).0
}

fn claim(
    context: &mut SdkLocalnetContext,
    program_id: Pubkey,
    claimer: Pubkey,
    phoenix_program: Pubkey,
) -> Result<(), TransactionError> {
    let ix = claim_escrow_ix(context, program_id, claimer, phoenix_program);
    context
        .try_send_instructions_with_metadata(with_compute_budget(ix), "payer")
        .map(|_| ())
        .map_err(|failure| failure.err)
}

fn instruction_error(error: InstructionError) -> Result<(), TransactionError> {
    Err(TransactionError::InstructionError(1, error))
}

fn quote_lot_collateral(context: &SdkLocalnetContext, actor: &str) -> i64 {
    Trader::try_from_account_bytes(&context.account_data(&context.actor_trader(actor)))
        .expect("decode trader")
        .state
        .quote_lot_collateral
        .as_inner()
}

#[test]
fn claim_escrow_via_delegated_pda() {
    let Some((mut context, program_id)) = setup_context() else {
        return;
    };
    context.send_fixture_transaction("escrowCashRequest");
    let receiver_claimer = claimer_pda(&context, "taker0", program_id);
    let sender_claimer = claimer_pda(&context, "taker1", program_id);
    let delegate = DelegateTraderParams::builder()
        .trader_wallet(context.actor_pubkey("taker0"))
        .trader_account(context.actor_trader("taker0"))
        .new_position_authority(receiver_claimer)
        .build()
        .expect("delegate params");
    let delegate = create_delegate_trader_ix(delegate).expect("delegate ix");
    context.send_instructions(vec![delegate.into()], "payer", "delegate-trader");

    assert_eq!(
        claim(
            &mut context,
            program_id,
            sender_claimer,
            *PHOENIX_PROGRAM_ID
        ),
        instruction_error(InstructionError::InvalidSeeds)
    );
    assert_eq!(
        claim(
            &mut context,
            program_id,
            receiver_claimer,
            SYSTEM_PROGRAM_ID
        ),
        instruction_error(InstructionError::IncorrectProgramId)
    );

    let receiver_before = quote_lot_collateral(&context, "taker0");
    let sender_before = quote_lot_collateral(&context, "taker1");
    claim(
        &mut context,
        program_id,
        receiver_claimer,
        *PHOENIX_PROGRAM_ID,
    )
    .expect("claim escrow");
    assert_eq!(
        quote_lot_collateral(&context, "taker0") - receiver_before,
        ESCROW_AMOUNT
    );
    assert_eq!(
        sender_before - quote_lot_collateral(&context, "taker1"),
        ESCROW_AMOUNT
    );
}
