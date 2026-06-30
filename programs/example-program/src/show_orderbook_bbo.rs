use phoenix_rise::accounts::orderbook::{Orderbook, OrderbookEntryRef};
use pinocchio::account_info::AccountInfo;
use pinocchio::program_error::ProgramError;
use pinocchio::{ProgramResult, msg};

use crate::common::require_exact_accounts;

pub(crate) fn process(accounts: &[AccountInfo]) -> ProgramResult {
    require_exact_accounts(accounts, 1)?;
    let orderbook_account = &accounts[0];
    let data = orderbook_account
        .try_borrow_data()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;
    let orderbook = Orderbook::try_from_account_bytes(&data).map_err(|error| {
        msg!(&format!(
            "rise-example-program: failed to deserialize orderbook: {error}"
        ));
        ProgramError::InvalidAccountData
    })?;
    let asset_symbol = orderbook.header().asset_symbol().map_err(|error| {
        msg!(&format!(
            "rise-example-program: failed to deserialize orderbook symbol: {error}"
        ));
        ProgramError::InvalidAccountData
    })?;

    msg!(&format!(
        "rise-example-program: orderbook bbo market={} bids={} asks={}",
        asset_symbol.as_str(),
        orderbook.num_bids(),
        orderbook.num_asks()
    ));
    log_entry("best_bid", orderbook.best_bid());
    log_entry("best_ask", orderbook.best_ask());

    Ok(())
}

fn log_entry(label: &str, entry: Option<OrderbookEntryRef<'_>>) {
    match entry {
        Some(entry) => {
            let trader_id = entry
                .order()
                .trader_position_id()
                .trader_id()
                .to_u32_or_sentinel();
            msg!(&format!(
                "rise-example-program: orderbook {label} price_in_ticks={} \
                 order_sequence_number={} raw_sequence_number={} remaining_base_lots={} \
                 trader_id={} asset_id={}",
                entry.price_in_ticks().as_inner(),
                entry.order_sequence_number(),
                entry.raw_sequence_number(),
                entry.num_base_lots_remaining().as_inner(),
                trader_id,
                entry.order().trader_position_id().asset_id()
            ));
        }
        None => {
            msg!(&format!("rise-example-program: orderbook {label}=none"));
        }
    }
}
