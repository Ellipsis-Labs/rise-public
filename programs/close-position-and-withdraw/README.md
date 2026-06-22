# Phoenix Close And Withdraw

This is a deliberately small CPI example program for Rise integrators. It shows
one flow that:

- places a reduce-only Phoenix market order to close all or part of a position,
- optionally routes that order through Flight,
- reads Hawkeye margin return data,
- withdraws Phoenix collateral to the trader authority's Phoenix-token account,
- converts that collateral to USDC through Ember.

The program has three instructions:

- `close_position_and_withdraw`
- `close_position_and_withdraw_with_builders`
- `withdraw_all_collateral`

An Anchor-compatible IDL is checked in at
`idl/phoenix_close_and_withdraw.json`. It documents the fixed account prefix,
the Flight-builder account prefix, and the dynamic Phoenix
`global_trader_index`/`active_trader_buffer` account tail.

The close-position instructions use the same Borsh-encoded params after the
8-byte Rise-style instruction discriminator. The Flight variant expects the five
Flight builder accounts as fixed accounts after the orderbook and spline
accounts. The dynamic Phoenix `global_trader_index` and `active_trader_buffer`
accounts are always appended at the end of each instruction so the fixed account
prefix can be represented by an IDL even if those dynamic account lists grow.
`withdraw_all_collateral` uses a smaller params struct with the dynamic Phoenix
index/buffer account counts. It does not require market orderbook or spline
accounts.

The example intentionally keeps account creation out of scope. The trader's
Phoenix-token account and USDC account must already exist. The program always
checks the supplied program ids, Ember state/vault PDAs, the Ember input and
output mints, and the Phoenix global-configuration accounts before issuing any
CPI. It also checks that the Phoenix-token account uses the canonical mint and
that the USDC account uses the Ember input mint. The Phoenix and Ember programs
repeat these mint and vault checks during their CPI handlers; the demo checks
are intentionally early so account-list mistakes fail before any collateral
movement. It also requires the Phoenix-token account to be owned by the signing
trader authority, because Phoenix withdraws into it and Ember burns from it.

`withdraw_all_collateral` also requires the USDC destination token account to be
owned by the signing trader authority. The program never allows this owner check
to be skipped.

The tests use `phoenix_rise::test_fixture`. Set
`PHOENIX_MAINNET_BPF_PROGRAMS=1` to fetch Phoenix, Ember, Hawkeye, and Flight
BPF from mainnet when a fresh cached copy is unavailable. The fixture caches
those programs under `target/deploy/.cache` for 24 hours by default; set
`PHOENIX_MAINNET_BPF_PROGRAM_CACHE_DIR` to override that location for external
SDK workspaces.

## Tests

From the `rise` workspace:

```sh
cargo build-sbf --manifest-path programs/close-position-and-withdraw/Cargo.toml
cargo nextest run --manifest-path programs/close-position-and-withdraw/Cargo.toml --no-capture
```

To use cached mainnet BPFs for the Phoenix protocol programs:

```sh
PHOENIX_MAINNET_BPF_PROGRAMS=1 \
cargo nextest run --manifest-path programs/close-position-and-withdraw/Cargo.toml --no-capture
```

## CLI

The nested CLI builds the demo program instruction against mainnet exchange
metadata and simulates by default. The demo program must already be deployed at
its declared program id.

```sh
cargo run --manifest-path programs/close-position-and-withdraw/cli/Cargo.toml -- \
  withdraw-all \
  --keypair-path ~/.config/solana/id.json
```

To withdraw all free collateral from a derived trader subaccount, pass the
subaccount index. The PDA index defaults to `0`.

```sh
cargo run --manifest-path programs/close-position-and-withdraw/cli/Cargo.toml -- \
  withdraw-all-subaccount \
  --keypair-path ~/.config/solana/id.json \
  --subaccount-index 1
```

To find the keypair's active SOL position automatically, close it, and withdraw
all free collateral:

```sh
cargo run --manifest-path programs/close-position-and-withdraw/cli/Cargo.toml -- \
  close \
  --keypair-path ~/.config/solana/id.json \
  --symbol SOL
```

The destination USDC token account must be owned by the signing keypair
authority. The CLI derives the authority's USDC ATA by default; pass
`--trader-usdc-token-account <token account>` only when supplying another
authority-owned USDC token account. Set `--send-transaction` to send after a
successful simulation. `--withdraw-mode` defaults to `all`; set
`--withdraw-mode` to `diff` to withdraw only the collateral freed by the close
order. The CLI
discovers the matching trader subaccount from `--keypair-path` and `--symbol`;
set `--trader-account` only when more than one subaccount has an active position
for the same symbol.
