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

## Crate Links

[`phoenix-rise`](../sdk/README.md) |
[`phoenix-rise-accounts`](../accounts/README.md) |
[`phoenix-rise-core`](../core/README.md) |
[`phoenix-rise-math`](../math/README.md)
