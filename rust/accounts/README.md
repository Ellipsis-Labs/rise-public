# Phoenix Rise Accounts

`phoenix-rise-accounts` provides Phoenix account discriminators, borrowed
account-data views, PDA helpers, and optional serde-friendly owned decoding.

Most users should start with the facade crate:
[`phoenix-rise`](../sdk/README.md). Protocol documentation lives at
[docs.phoenix.trade](https://docs.phoenix.trade/).

## Use This Crate When

- You need to decode account bytes from RPC or an indexer.
- You want program-friendly account layout types without API/RPC clients.
- You need PDA helpers next to the account layouts.

## Features

- `serde`: JSON-friendly output for account views. Pubkeys are encoded as
  base58 strings.

PDA helpers are always available. On Solana targets, `solana-pubkey` uses the
program-address syscall path; host builds use its curve support for off-chain
derivation.

## Crate Links

[`phoenix-rise`](../sdk/README.md) |
[`phoenix-rise-ix`](../ix/README.md) |
[`phoenix-rise-math`](../math/README.md) |
[`phoenix-rise-cli`](../cli/README.md)
