# Rise Example Program

Pinocchio example program exercising the `phoenix-rise` CPI surface from an
on-chain caller. Integration tests run against the generated Rise localnet
fixture and cover deposits, withdrawals, market orders with return data, limit
orders, stop losses, Hawkeye margin reads after market orders, and isolated
subaccount collateral flows.

This crate is a reference example, not a reusable library. Each instruction
takes a fixed account prefix followed by dynamic `global_trader_index` and
`active_trader_buffer` accounts at the tail, then adapts them into the typed CPI
contexts in `phoenix_rise::ix::cpi`.
