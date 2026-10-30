# Phoenix Rise IX

`phoenix-rise-ix` provides Phoenix, Ember, Flight, and Hawkeye instruction
discriminants, payload types, account-meta builders, and optional CPI helpers.

Most users should start with [`phoenix-rise`](../sdk/README.md). Protocol
documentation lives at [docs.phoenix.trade](https://docs.phoenix.trade/).

## Use This Crate When

- You build low-level instructions directly.
- You write an on-chain program that invokes Phoenix via CPI.
- You need CPI helpers without API or RPC clients.

## Features

- `solana`: builds `solana_instruction::Instruction` values for off-chain use.
- `cpi`: enables Pinocchio CPI invoke helpers.
- `accounts`: enables account layout types from `phoenix-rise-accounts`.
- `serde`: JSON serialization for instruction params and discriminants.

PDA helpers are always enabled. On-chain builds use Solana's program-address
syscall; off-chain builds use `solana-pubkey` curve derivation.

## Native SOL deposits

`native_sol::create_deposit_native_sol_ixs(payer, trader_wallet, lamports, sync_params)`
returns `ReallocTrader` → System transfer → `SyncNative`. The trader may be
registered earlier in the same transaction. Reallocation reserves space for the
non-position SOL entry without increasing the allowed perpetual-position count;
it follows the contract's bounded growth/no-op rules and never shrinks an
existing account. `payer` funds additional rent and `trader_wallet` supplies the
SOL (pass the same wallet when no rent sponsor is used).

Reallocation must precede the transfer so a higher rent floor does not consume
deposit principal. For custom instruction bundles,
`native_sol::create_realloc_trader_ix` builds the standalone no-argument
instruction.

## Crate Links

[`phoenix-rise`](../sdk/README.md) |
[`phoenix-rise-accounts`](../accounts/README.md) |
[`phoenix-rise-core`](../core/README.md) |
[`phoenix-rise-math`](../math/README.md)
