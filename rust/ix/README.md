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

## Instruction and error diagnostics

`diagnostics::identify_instruction(&program_id, &data)` identifies an instruction
without decoding its arguments. `diagnostics::decode_program_error(&program_id,
code)` looks up a custom code's IDL name and message, and
`diagnostics::decode_instruction_error(&program_id, &data, code)` combines both.
Pass the originating CPI instruction when a caller propagated an error: code
numbers are program-specific.

The catalogs cover Eternal (including its beta address), Ember, Flame, Flight,
Flicker, Hawkeye, and Punchcard. IDLs without errors still supply instruction names. Unknown
programs, codes, and discriminators have no decoded name; custom numbers remain
available. Messages retain the IDL's `<field>` placeholders. These helpers are
available without the API/RPC feature stack; transaction-level log attribution
is in `phoenix_rise::core::transaction_errors` with `tx-builder` enabled.

When another invocation parser has already attributed a failure, use
`diagnostics::decode_runtime_failure(&program_id, &data, reason, own_logs)` to
construct the same diagnostic. Supply the resolved
returning instruction's bytes and only its own runtime reason and logs. This
function does not validate a transaction trace. Omit unavailable CPI bytes;
the instruction remains unnamed. Applications own user-facing messages and recovery policy.

## Crate Links

[`phoenix-rise`](../sdk/README.md) |
[`phoenix-rise-accounts`](../accounts/README.md) |
[`phoenix-rise-core`](../core/README.md) |
[`phoenix-rise-math`](../math/README.md)
