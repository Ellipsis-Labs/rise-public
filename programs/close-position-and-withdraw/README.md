# Phoenix Close And Withdraw

Minimal CPI example program demonstrating a single flow that:

- places a reduce-only Phoenix market order to close all or part of a position,
- optionally routes that order through Flight,
- reads Hawkeye margin return data,
- withdraws Phoenix collateral to the trader authority's Phoenix-token account,
- converts that collateral to USDC through Ember.

The program exposes three instructions:

- `close_position_and_withdraw`
- `close_position_and_withdraw_with_builders`
- `withdraw_all_collateral`

An Anchor-compatible IDL at `idl/phoenix_close_and_withdraw.json` documents the
fixed account prefix, the Flight-builder account prefix, and the dynamic Phoenix
`global_trader_index`/`active_trader_buffer` account tail.

Both close-position instructions share the same Borsh-encoded params after the
8-byte Rise-style instruction discriminator. The Flight variant places the five
Flight builder accounts as fixed accounts after the orderbook and spline
accounts. Both instructions append the dynamic Phoenix `global_trader_index` and
`active_trader_buffer` accounts at the tail so the fixed prefix remains
IDL-compatible as dynamic account lists grow. `withdraw_all_collateral` takes a
smaller params struct with the dynamic Phoenix index/buffer account counts and
omits the market orderbook and spline accounts.

Account creation is out of scope: the trader's Phoenix-token account and USDC
account must already exist. Before issuing any CPI, the program validates
supplied program IDs, Ember state/vault PDAs, Ember input/output mints, and
Phoenix global-configuration accounts. It verifies that the Phoenix-token
account uses the canonical mint and the USDC account uses the Ember input mint
(Phoenix and Ember repeat these mint and vault checks during CPI, but checking
early fails fast on account-list errors before moving collateral). It also
requires the signing trader authority to own the Phoenix-token account, since
Phoenix withdraws into it and Ember burns from it.

`withdraw_all_collateral` also requires the signing trader authority to own the
USDC destination token account; this check cannot be skipped.

Tests use `phoenix-rise-litesvm-test`. Set `PHOENIX_MAINNET_BPF_PROGRAMS=1` to
fetch Phoenix, Ember, Hawkeye, and Flight BPF binaries from mainnet when a fresh
cached copy is unavailable. The fixture caches these programs under
`target/deploy/.cache` for 24 hours by default; set
`PHOENIX_MAINNET_BPF_PROGRAM_CACHE_DIR` to override the cache directory.

## Tests

From the repository root:

```sh
cargo build-sbf --manifest-path programs/close-position-and-withdraw/Cargo.toml
cargo nextest run --manifest-path programs/close-position-and-withdraw/Cargo.toml --no-capture
```

With cached mainnet BPFs for the Phoenix protocol programs:

```sh
PHOENIX_MAINNET_BPF_PROGRAMS=1 \
cargo nextest run --manifest-path programs/close-position-and-withdraw/Cargo.toml --no-capture
```

## CLI

The nested CLI builds the program instruction against mainnet exchange metadata
and simulates by default. The program must already be deployed at its declared
program ID.

```sh
cargo run --manifest-path programs/close-position-and-withdraw/cli/Cargo.toml -- \
  withdraw-all \
  --keypair-path ~/.config/solana/id.json
```

Withdraw all free collateral from a derived trader subaccount (PDA index
defaults to `0`):

```sh
cargo run --manifest-path programs/close-position-and-withdraw/cli/Cargo.toml -- \
  withdraw-all-subaccount \
  --keypair-path ~/.config/solana/id.json \
  --subaccount-index 1
```

Find the keypair's active SOL position, close it, and withdraw all free
collateral:

```sh
cargo run --manifest-path programs/close-position-and-withdraw/cli/Cargo.toml -- \
  close \
  --keypair-path ~/.config/solana/id.json \
  --symbol SOL
```

The signing keypair authority must own the destination USDC token account. The
CLI derives the authority's USDC ATA by default; pass
`--trader-usdc-token-account <token account>` to use another authority-owned
USDC token account. Pass `--send-transaction` to submit after simulation
succeeds. `--withdraw-mode` defaults to `all`; pass `diff` to withdraw only the
collateral freed by the close order. The CLI resolves the trader subaccount from
`--keypair-path` and `--symbol`; pass `--trader-account` only when multiple
subaccounts hold an active position for that symbol.
