use phoenix_rise::ix::return_data::decode_matching_engine_cpi_response;
use pinocchio::account_info::AccountInfo;
use pinocchio::cpi::get_return_data;
use pinocchio::program_error::ProgramError;
use pinocchio::pubkey::Pubkey;
use pinocchio::{ProgramResult, msg};

pub(crate) fn dynamic_tail<'a>(
    accounts: &'a [AccountInfo],
    fixed_account_count: usize,
    global_trader_index_count: usize,
    active_trader_buffer_count: usize,
) -> Result<(&'a [AccountInfo], &'a [AccountInfo]), ProgramError> {
    let gti_end = fixed_account_count
        .checked_add(global_trader_index_count)
        .ok_or(ProgramError::InvalidInstructionData)?;
    let expected = gti_end
        .checked_add(active_trader_buffer_count)
        .ok_or(ProgramError::InvalidInstructionData)?;
    require_exact_accounts(accounts, expected)?;
    Ok((
        &accounts[fixed_account_count..gti_end],
        &accounts[gti_end..expected],
    ))
}

pub(crate) fn require_exact_accounts(accounts: &[AccountInfo], expected: usize) -> ProgramResult {
    if accounts.len() != expected {
        msg!("rise-example-program: invalid account count");
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    Ok(())
}

pub(crate) fn log_matching_response(label: &str, expected_program_id: &Pubkey) {
    let Some(return_data) = get_return_data() else {
        msg!(&format!(
            "rise-example-program: {label} matching response missing"
        ));
        return;
    };
    if return_data.program_id() != expected_program_id {
        msg!(&format!(
            "rise-example-program: {label} matching response wrong program"
        ));
        return;
    }

    match decode_matching_engine_cpi_response(return_data.as_slice()) {
        Ok(response) => {
            if let Some(order_id) = response.order_id() {
                msg!(&format!(
                    "rise-example-program: {label} matching response order_id_price_in_ticks={} \
                     order_sequence_number={} quote_in={} base_in={} quote_out={} base_out={} \
                     quote_posted={} base_posted={}",
                    order_id.price_in_ticks,
                    order_id.order_sequence_number,
                    response.num_quote_lots_in,
                    response.num_base_lots_in,
                    response.num_quote_lots_out,
                    response.num_base_lots_out,
                    response.num_quote_lots_posted,
                    response.num_base_lots_posted
                ));
            } else {
                msg!(&format!(
                    "rise-example-program: {label} matching response order_id=none quote_in={} \
                     base_in={} quote_out={} base_out={} quote_posted={} base_posted={}",
                    response.num_quote_lots_in,
                    response.num_base_lots_in,
                    response.num_quote_lots_out,
                    response.num_base_lots_out,
                    response.num_quote_lots_posted,
                    response.num_base_lots_posted
                ));
            }
        }
        Err(_) => {
            msg!(&format!(
                "rise-example-program: {label} matching response decode failed"
            ));
        }
    }
}
