use pinocchio::ProgramResult;
use pinocchio::account_info::AccountInfo;

use crate::market::MarketContext;
use crate::params::LimitOrderCpiParams;

pub(crate) fn process(accounts: &[AccountInfo], params: &LimitOrderCpiParams) -> ProgramResult {
    let context = MarketContext::load(
        accounts,
        params.global_trader_index_count as usize,
        params.active_trader_buffer_count as usize,
    )?;
    context.invoke_limit_order(params)
}
