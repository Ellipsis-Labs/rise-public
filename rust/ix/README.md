# Phoenix Rise IX

`phoenix-rise-ix` provides Phoenix, Ember, Flight, and Hawkeye instruction
discriminants, payload types, account-meta builders, and optional CPI helpers.

Most users should start with [`phoenix-rise`](../sdk/README.md). Protocol
documentation lives at [docs.phoenix.trade](https://docs.phoenix.trade/).

## Use This Crate When

- You need low-level instruction construction.
- You are writing an on-chain integration that invokes Phoenix programs.
- You want CPI helpers without importing API/RPC clients.

## Features

- `solana`: builds `solana_instruction::Instruction` values for off-chain use.
- `cpi`: enables Pinocchio CPI invoke helpers.
- `accounts`: enables account layout types from `phoenix-rise-accounts`.
- `serde`: JSON serialization for instruction params and discriminants.

PDA helpers are always available. On-chain builds use Solana's syscall-backed
program address derivation; off-chain builds use `solana-pubkey` curve support.

## Native SOL deposits

`native_sol::create_deposit_native_sol_ixs(payer, trader_wallet, lamports, sync_params)`
returns `ReallocTrader` → System transfer → `SyncNative`. The trader can be
registered earlier in the same transaction. Reallocation reserves space for
SOL's non-position entry without increasing its allowed perpetual-position
count. It uses the contract's bounded growth/no-op rules and never shrinks an
existing account. `payer` funds additional rent, while `trader_wallet` supplies
the SOL; use the same wallet for both when no rent sponsor is involved.

Reallocation must precede the transfer so the deposit principal is not consumed
by a higher rent floor. For custom instruction bundles,
`native_sol::create_realloc_trader_ix` exposes the same no-argument instruction.

## Crate Links

[`phoenix-rise`](../sdk/README.md) |
[`phoenix-rise-accounts`](../accounts/README.md) |
[`phoenix-rise-core`](../core/README.md) |
[`phoenix-rise-math`](../math/README.md)
