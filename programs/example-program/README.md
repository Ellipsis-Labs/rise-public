# Rise Example Program

This Pinocchio program exercises the `phoenix-rise` CPI surface from an
on-chain caller. Its integration tests run against the generated Rise localnet
fixture and cover deposits, withdrawals, market orders with return data, limit
orders, stop losses, Hawkeye margin reads after market orders, and isolated
subaccount collateral flows.

The program is intentionally an example, not a reusable business-logic crate.
Each instruction uses a simple fixed account prefix with dynamic
`global_trader_index` and `active_trader_buffer` accounts at the tail, then
adapts those accounts into the typed CPI contexts exposed by
`phoenix-rise::ix::cpi`.
