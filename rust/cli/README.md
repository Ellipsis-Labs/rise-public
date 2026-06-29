# Phoenix Rise CLI

`phoenix-rise-cli` installs the `phoenix-rise` binary for script-friendly
Phoenix API, websocket, transaction, trading-instruction, and RPC account
inspection.

For SDK documentation, see [`phoenix-rise`](../sdk/README.md). For protocol
concepts and product documentation, see
[docs.phoenix.trade](https://docs.phoenix.trade/).

## Install

From the Rise Rust workspace:

```bash
cargo install --path cli
```

From crates.io after release:

```bash
cargo install phoenix-rise-cli
```

## Configuration

- The CLI is built against `phoenix-rise` with the `api`, `ws`, `tx-builder`,
  `events`, and `serde` features enabled so API responses, websocket streams,
  parsed transactions, and decoded accounts can be printed as JSON.
- `--api-url`: Phoenix API URL. Defaults to `PHOENIX_API_URL`, then
  `https://perp-api.phoenix.trade`.
- `--ws-url`: explicit websocket URL. When omitted, the CLI derives `/v1/ws`
  from the API URL.
- `--rpc-url`: Solana RPC URL for `rpc` and `tx` subcommands. Defaults
  to `PHOENIX_RPC_URL`, then `SOLANA_RPC_URL`, then mainnet-beta.
- `--keypair`: default signing wallet. It is also the default transaction
  payer unless `--payer-keypair` is provided. `--keypair-path` remains accepted
  as an alias.
- `--payer-keypair`: transaction fee-payer keypair for flows that sign locally.
- `--ledger`: Ledger locator such as `usb://ledger?key=0/0`. The CLI keeps
  hardware-wallet dependencies out of SDK crates; instruction-building commands
  include the locator and required signer pubkeys in their output so Ledger
  operators can hand the instructions to their signing flow.
- `--json`: compact JSON only, suitable for shell pipelines.
- `--pretty`: pretty JSON for interactive inspection.

Authentication subcommands use `PHOENIX_AUTH_KEYPAIR_PATH`,
`SOLANA_KEYPAIR_PATH`, or the default Solana keypair path unless
global `--keypair`, `--keypair-path`, or `--service-credential-file` is
supplied.
Access and refresh tokens are cached at
`~/.config/phoenix/rise-cli/session.json`; use `auth cache-path` to print
the exact path.

## Commands

The CLI uses clap derive comments for `--help`; this README includes generated
help for common commands plus copy-pasteable examples.

<!-- phoenix-rise-cli-command-docs:start -->

Generated from the `phoenix-rise` clap command tree. Regenerate with:

```bash
cargo run -p phoenix-rise-cli --bin generate-cli-docs
```

This section shows help for common commands. The CLI supports additional commands and flags; run `phoenix-rise --help`, `phoenix-rise <group> --help`, or `phoenix-rise <group> <command> --help` for the full surface.

### Selected Command Help

#### `phoenix-rise`

CLI for Phoenix Rise API, websocket, transaction, and RPC account inspection

```text
CLI for Phoenix Rise API, websocket, transaction, and RPC account inspection

Usage: phoenix-rise [OPTIONS] <COMMAND>

Commands:
  auth      Authenticate to the Phoenix API
  exchange  Fetch exchange configuration, keys, status, and onboarding helpers
  market    Fetch market metadata, orderbooks, candles, and funding rates
  rpc       Fetch and decode Phoenix Rise accounts from Solana RPC
  tx        Fetch and decode transactions from RPC
  flight    Build Flight builder instructions and builder collateral withdrawals
  trader    Trader-focused summary and diagnostics
  ws        Subscribe to Phoenix websocket feeds and print one message

Options:
      --api-url <API_URL>
          Base API URL. Defaults to PHOENIX_API_URL, then https://perp-api.phoenix.trade

      --rpc-url <RPC_URL>
          Solana RPC URL used by RPC and tx commands

      --ws-url <WS_URL>
          Explicit WebSocket URL. Defaults to /v1/ws derived from the API URL

      --keypair <KEYPAIR_PATH>
          Default Solana signing keypair. Also used as the default payer unless --payer-keypair is provided

          [aliases: --keypair-path]

      --payer-keypair <PAYER_KEYPAIR_PATH>
          Fee-payer keypair. Overrides --keypair for transaction fee payment

          [aliases: --payer-keypair-path]

      --ledger <LEDGER>
          Ledger locator/path, for example usb://ledger or usb://ledger?key=0/0

      --json
          Print compact JSON only. Useful for piping into scripts

      --pretty
          Pretty-print JSON output

  -h, --help
          Print help
```

#### `phoenix-rise auth login`

Login with a wallet keypair or service-account credential file

```text
Login with a wallet keypair or service-account credential file

Usage: phoenix-rise auth login [OPTIONS]

Options:
      --api-url <API_URL>
          Base API URL. Defaults to PHOENIX_API_URL, then https://perp-api.phoenix.trade

      --keypair-path <KEYPAIR_PATH>
          Solana keypair path. Falls back to PHOENIX_AUTH_KEYPAIR_PATH, SOLANA_KEYPAIR_PATH, then ~/.config/solana/id.json

      --rpc-url <RPC_URL>
          Solana RPC URL used by RPC and tx commands

      --service-credential-file <SERVICE_CREDENTIAL_FILE>
          Service-account credential JSON file containing client_id, key_id, and private_key for challenge-based service auth

      --output <OUTPUT>
          Output format for auth session material

          [default: json]
          [possible values: json, env]

      --ws-url <WS_URL>
          Explicit WebSocket URL. Defaults to /v1/ws derived from the API URL

      --payer-keypair <PAYER_KEYPAIR_PATH>
          Fee-payer keypair. Overrides --keypair for transaction fee payment

          [aliases: --payer-keypair-path]

      --ledger <LEDGER>
          Ledger locator/path, for example usb://ledger or usb://ledger?key=0/0

      --json
          Print compact JSON only. Useful for piping into scripts

      --pretty
          Pretty-print JSON output

  -h, --help
          Print help
```

#### `phoenix-rise auth session`

Print the cached API session

```text
Print the cached API session

Usage: phoenix-rise auth session [OPTIONS]

Options:
      --api-url <API_URL>
          Base API URL. Defaults to PHOENIX_API_URL, then https://perp-api.phoenix.trade

      --output <OUTPUT>
          Output format for auth session material

          [default: json]
          [possible values: json, env]

      --rpc-url <RPC_URL>
          Solana RPC URL used by RPC and tx commands

      --ws-url <WS_URL>
          Explicit WebSocket URL. Defaults to /v1/ws derived from the API URL

      --keypair <KEYPAIR_PATH>
          Default Solana signing keypair. Also used as the default payer unless --payer-keypair is provided

          [aliases: --keypair-path]

      --payer-keypair <PAYER_KEYPAIR_PATH>
          Fee-payer keypair. Overrides --keypair for transaction fee payment

          [aliases: --payer-keypair-path]

      --ledger <LEDGER>
          Ledger locator/path, for example usb://ledger or usb://ledger?key=0/0

      --json
          Print compact JSON only. Useful for piping into scripts

      --pretty
          Pretty-print JSON output

  -h, --help
          Print help
```

#### `phoenix-rise exchange keys`

Fetch exchange key accounts

```text
Fetch exchange key accounts

Usage: phoenix-rise exchange keys [OPTIONS]

Options:
      --api-url <API_URL>
          Base API URL. Defaults to PHOENIX_API_URL, then https://perp-api.phoenix.trade

      --rpc-url <RPC_URL>
          Solana RPC URL used by RPC and tx commands

      --ws-url <WS_URL>
          Explicit WebSocket URL. Defaults to /v1/ws derived from the API URL

      --keypair <KEYPAIR_PATH>
          Default Solana signing keypair. Also used as the default payer unless --payer-keypair is provided

          [aliases: --keypair-path]

      --payer-keypair <PAYER_KEYPAIR_PATH>
          Fee-payer keypair. Overrides --keypair for transaction fee payment

          [aliases: --payer-keypair-path]

      --ledger <LEDGER>
          Ledger locator/path, for example usb://ledger or usb://ledger?key=0/0

      --json
          Print compact JSON only. Useful for piping into scripts

      --pretty
          Pretty-print JSON output

  -h, --help
          Print help
```

#### `phoenix-rise exchange build-register-ixs`

Build public register-trader instructions

```text
Build public register-trader instructions

Usage: phoenix-rise exchange build-register-ixs [OPTIONS] --trader-authority <TRADER_AUTHORITY> --tx-fee-payer <TX_FEE_PAYER>

Options:
      --api-url <API_URL>
          Base API URL. Defaults to PHOENIX_API_URL, then https://perp-api.phoenix.trade

      --trader-authority <TRADER_AUTHORITY>
          Trader authority pubkey

      --rpc-url <RPC_URL>
          Solana RPC URL used by RPC and tx commands

      --tx-fee-payer <TX_FEE_PAYER>
          Transaction fee payer pubkey

      --max-positions <MAX_POSITIONS>
          Max positions to allocate for the trader account

      --ws-url <WS_URL>
          Explicit WebSocket URL. Defaults to /v1/ws derived from the API URL

      --keypair <KEYPAIR_PATH>
          Default Solana signing keypair. Also used as the default payer unless --payer-keypair is provided

          [aliases: --keypair-path]

      --payer-keypair <PAYER_KEYPAIR_PATH>
          Fee-payer keypair. Overrides --keypair for transaction fee payment

          [aliases: --payer-keypair-path]

      --ledger <LEDGER>
          Ledger locator/path, for example usb://ledger or usb://ledger?key=0/0

      --json
          Print compact JSON only. Useful for piping into scripts

      --pretty
          Pretty-print JSON output

  -h, --help
          Print help
```

#### `phoenix-rise market list`

List all Phoenix Rise markets

```text
List all Phoenix Rise markets

Usage: phoenix-rise market list [OPTIONS]

Options:
      --api-url <API_URL>
          Base API URL. Defaults to PHOENIX_API_URL, then https://perp-api.phoenix.trade

      --rpc-url <RPC_URL>
          Solana RPC URL used by RPC and tx commands

      --ws-url <WS_URL>
          Explicit WebSocket URL. Defaults to /v1/ws derived from the API URL

      --keypair <KEYPAIR_PATH>
          Default Solana signing keypair. Also used as the default payer unless --payer-keypair is provided

          [aliases: --keypair-path]

      --payer-keypair <PAYER_KEYPAIR_PATH>
          Fee-payer keypair. Overrides --keypair for transaction fee payment

          [aliases: --payer-keypair-path]

      --ledger <LEDGER>
          Ledger locator/path, for example usb://ledger or usb://ledger?key=0/0

      --json
          Print compact JSON only. Useful for piping into scripts

      --pretty
          Pretty-print JSON output

  -h, --help
          Print help
```

#### `phoenix-rise market show`

Fetch metadata for one market symbol

```text
Fetch metadata for one market symbol

Usage: phoenix-rise market show [OPTIONS] --symbol <SYMBOL>

Options:
      --api-url <API_URL>
          Base API URL. Defaults to PHOENIX_API_URL, then https://perp-api.phoenix.trade

      --symbol <SYMBOL>
          Market symbol, for example SOL

      --rpc-url <RPC_URL>
          Solana RPC URL used by RPC and tx commands

      --ws-url <WS_URL>
          Explicit WebSocket URL. Defaults to /v1/ws derived from the API URL

      --keypair <KEYPAIR_PATH>
          Default Solana signing keypair. Also used as the default payer unless --payer-keypair is provided

          [aliases: --keypair-path]

      --payer-keypair <PAYER_KEYPAIR_PATH>
          Fee-payer keypair. Overrides --keypair for transaction fee payment

          [aliases: --payer-keypair-path]

      --ledger <LEDGER>
          Ledger locator/path, for example usb://ledger or usb://ledger?key=0/0

      --json
          Print compact JSON only. Useful for piping into scripts

      --pretty
          Pretty-print JSON output

  -h, --help
          Print help
```

#### `phoenix-rise market orderbook`

Fetch L2 orderbook data for one market

```text
Fetch L2 orderbook data for one market

Usage: phoenix-rise market orderbook [OPTIONS] --symbol <SYMBOL>

Options:
      --api-url <API_URL>
          Base API URL. Defaults to PHOENIX_API_URL, then https://perp-api.phoenix.trade

      --symbol <SYMBOL>
          Market symbol, for example SOL

      --include-splines
          Include spline liquidity in the response

      --rpc-url <RPC_URL>
          Solana RPC URL used by RPC and tx commands

      --ws-url <WS_URL>
          Explicit WebSocket URL. Defaults to /v1/ws derived from the API URL

      --keypair <KEYPAIR_PATH>
          Default Solana signing keypair. Also used as the default payer unless --payer-keypair is provided

          [aliases: --keypair-path]

      --payer-keypair <PAYER_KEYPAIR_PATH>
          Fee-payer keypair. Overrides --keypair for transaction fee payment

          [aliases: --payer-keypair-path]

      --ledger <LEDGER>
          Ledger locator/path, for example usb://ledger or usb://ledger?key=0/0

      --json
          Print compact JSON only. Useful for piping into scripts

      --pretty
          Pretty-print JSON output

  -h, --help
          Print help
```

#### `phoenix-rise market candles`

Fetch paginated market candles and print a table or JSON

```text
Fetch paginated market candles and print a table or JSON

Usage: phoenix-rise market candles [OPTIONS] --symbol <SYMBOL>

Options:
      --api-url <API_URL>
          Base API URL. Defaults to PHOENIX_API_URL, then https://perp-api.phoenix.trade

      --symbol <SYMBOL>


      --rpc-url <RPC_URL>
          Solana RPC URL used by RPC and tx commands

      --timeframe <TIMEFRAME>
          [default: 1m]

      --page-size <PAGE_SIZE>
          [default: 2500]

      --ws-url <WS_URL>
          Explicit WebSocket URL. Defaults to /v1/ws derived from the API URL

      --keypair <KEYPAIR_PATH>
          Default Solana signing keypair. Also used as the default payer unless --payer-keypair is provided

          [aliases: --keypair-path]

      --max-pages <MAX_PAGES>


      --max-items <MAX_ITEMS>


      --payer-keypair <PAYER_KEYPAIR_PATH>
          Fee-payer keypair. Overrides --keypair for transaction fee payment

          [aliases: --payer-keypair-path]

      --ledger <LEDGER>
          Ledger locator/path, for example usb://ledger or usb://ledger?key=0/0

      --start-time <START_TIME>
          Inclusive lower timestamp bound. Accepts unix seconds, unix milliseconds, RFC3339, or YYYY-MM-DD

      --end-time <END_TIME>
          Inclusive upper timestamp bound. Accepts unix seconds, unix milliseconds, RFC3339, or YYYY-MM-DD

      --json
          Print compact JSON only. Useful for piping into scripts

      --lookback <LOOKBACK>
          Human-readable duration ending at --end-time or now, for example 7d, 24h, or 1h30m

      --pretty
          Pretty-print JSON output

      --output <OUTPUT>
          Write the complete JSON response to this path instead of stdout

          [aliases: --out]

  -h, --help
          Print help
```

#### `phoenix-rise rpc account`

```text
Usage: phoenix-rise rpc account [OPTIONS] --address <ADDRESS> --account-type <ACCOUNT_TYPE>

Options:
      --address <ADDRESS>
          Account pubkey to fetch and decode

      --api-url <API_URL>
          Base API URL. Defaults to PHOENIX_API_URL, then https://perp-api.phoenix.trade

      --account-type <ACCOUNT_TYPE>
          Expected account layout

          [possible values: conditional-orders, global-configuration, mint, orderbook, orderbook-header, permission, perp-asset-map, spline-collection, stop-losses, token-account, trader, withdraw-queue, withdraw-queue-header]

      --rpc-url <RPC_URL>
          Solana RPC URL used by RPC and tx commands

      --ws-url <WS_URL>
          Explicit WebSocket URL. Defaults to /v1/ws derived from the API URL

      --keypair <KEYPAIR_PATH>
          Default Solana signing keypair. Also used as the default payer unless --payer-keypair is provided

          [aliases: --keypair-path]

      --payer-keypair <PAYER_KEYPAIR_PATH>
          Fee-payer keypair. Overrides --keypair for transaction fee payment

          [aliases: --payer-keypair-path]

      --ledger <LEDGER>
          Ledger locator/path, for example usb://ledger or usb://ledger?key=0/0

      --json
          Print compact JSON only. Useful for piping into scripts

      --pretty
          Pretty-print JSON output

  -h, --help
          Print help
```

#### `phoenix-rise trader register`

Register or onboard a trader account

```text
Register or onboard a trader account

Usage: phoenix-rise trader register [OPTIONS]

Options:
      --api-url <API_URL>
          Base API URL. Defaults to PHOENIX_API_URL, then https://perp-api.phoenix.trade

      --referral-code <REFERRAL_CODE>
          Referral code to activate while registering/onboarding

      --authority <AUTHORITY>
          Trader authority pubkey. Without a referral code this can be any authority pubkey; with a referral code it must match global --keypair

      --rpc-url <RPC_URL>
          Solana RPC URL used by RPC and tx commands

      --max-positions <MAX_POSITIONS>
          Max positions to use when creating a missing trader account

          [default: 128]

      --ws-url <WS_URL>
          Explicit WebSocket URL. Defaults to /v1/ws derived from the API URL

      --keypair <KEYPAIR_PATH>
          Default Solana signing keypair. Also used as the default payer unless --payer-keypair is provided

          [aliases: --keypair-path]

      --trader-pda-index <TRADER_PDA_INDEX>
          Trader PDA index. The public onboarding endpoints currently support 0

          [default: 0]
          [aliases: --pda-index]

      --payer-keypair <PAYER_KEYPAIR_PATH>
          Fee-payer keypair. Overrides --keypair for transaction fee payment

          [aliases: --payer-keypair-path]

      --trader-subaccount-index <TRADER_SUBACCOUNT_INDEX>
          Trader subaccount index. The public onboarding endpoints currently support 0

          [default: 0]
          [aliases: --subaccount-index]

      --ledger <LEDGER>
          Ledger locator/path, for example usb://ledger or usb://ledger?key=0/0

      --recent-blockhash <RECENT_BLOCKHASH>
          Optional recent blockhash to use when signing the transaction

      --json
          Print compact JSON only. Useful for piping into scripts

      --pretty
          Pretty-print JSON output

  -h, --help
          Print help
```

#### `phoenix-rise trader summary`

Print a consolidated trader account summary

```text
Print a consolidated trader account summary

Usage: phoenix-rise trader summary [OPTIONS] --authority <AUTHORITY>

Options:
      --api-url <API_URL>
          Base API URL. Defaults to PHOENIX_API_URL, then https://perp-api.phoenix.trade

      --authority <AUTHORITY>


      --pda-index <PDA_INDEX>
          [default: 0]

      --rpc-url <RPC_URL>
          Solana RPC URL used by RPC and tx commands

      --subaccount-index <SUBACCOUNT_INDEX>
          [default: 0]

      --ws-url <WS_URL>
          Explicit WebSocket URL. Defaults to /v1/ws derived from the API URL

      --keypair <KEYPAIR_PATH>
          Default Solana signing keypair. Also used as the default payer unless --payer-keypair is provided

          [aliases: --keypair-path]

      --limit <LIMIT>
          [default: 20]

      --payer-keypair <PAYER_KEYPAIR_PATH>
          Fee-payer keypair. Overrides --keypair for transaction fee payment

          [aliases: --payer-keypair-path]

      --ledger <LEDGER>
          Ledger locator/path, for example usb://ledger or usb://ledger?key=0/0

      --json
          Print compact JSON only. Useful for piping into scripts

      --pretty
          Pretty-print JSON output

  -h, --help
          Print help
```

#### `phoenix-rise trader trade-history`

Fetch paginated trade history and print a table or JSON

```text
Fetch paginated trade history and print a table or JSON

Usage: phoenix-rise trader trade-history [OPTIONS] --authority <AUTHORITY>

Options:
      --api-url <API_URL>
          Base API URL. Defaults to PHOENIX_API_URL, then https://perp-api.phoenix.trade

      --authority <AUTHORITY>


      --pda-index <PDA_INDEX>
          [default: 0]

      --rpc-url <RPC_URL>
          Solana RPC URL used by RPC and tx commands

      --market-symbol <MARKET_SYMBOL>


      --ws-url <WS_URL>
          Explicit WebSocket URL. Defaults to /v1/ws derived from the API URL

      --keypair <KEYPAIR_PATH>
          Default Solana signing keypair. Also used as the default payer unless --payer-keypair is provided

          [aliases: --keypair-path]

      --page-size <PAGE_SIZE>
          [default: 1000]

      --max-pages <MAX_PAGES>


      --payer-keypair <PAYER_KEYPAIR_PATH>
          Fee-payer keypair. Overrides --keypair for transaction fee payment

          [aliases: --payer-keypair-path]

      --ledger <LEDGER>
          Ledger locator/path, for example usb://ledger or usb://ledger?key=0/0

      --max-items <MAX_ITEMS>


      --json
          Print compact JSON only. Useful for piping into scripts

      --start-time <START_TIME>
          Inclusive lower timestamp bound. Accepts unix seconds, unix milliseconds, RFC3339, or YYYY-MM-DD

      --end-time <END_TIME>
          Inclusive upper timestamp bound. Accepts unix seconds, unix milliseconds, RFC3339, or YYYY-MM-DD

      --pretty
          Pretty-print JSON output

      --lookback <LOOKBACK>
          Human-readable duration ending at --end-time or now, for example 7d, 24h, or 1h30m

      --output <OUTPUT>
          Write the complete JSON response to this path instead of stdout

          [aliases: --out]

  -h, --help
          Print help
```

#### `phoenix-rise trader set-position-authority`

Set a trader account's position authority

```text
Set a trader account's position authority

Usage: phoenix-rise trader set-position-authority [OPTIONS] --new-position-authority <NEW_POSITION_AUTHORITY>

Options:
      --api-url <API_URL>
          Base API URL. Defaults to PHOENIX_API_URL, then https://perp-api.phoenix.trade

      --authority <AUTHORITY>
          Trader wallet authority pubkey. Defaults to global --keypair when omitted

      --new-position-authority <NEW_POSITION_AUTHORITY>
          New position authority pubkey

          [aliases: --position-authority]

      --rpc-url <RPC_URL>
          Solana RPC URL used by RPC and tx commands

      --trader-pda-index <TRADER_PDA_INDEX>
          Trader PDA index

          [default: 0]
          [aliases: --pda-index]

      --ws-url <WS_URL>
          Explicit WebSocket URL. Defaults to /v1/ws derived from the API URL

      --keypair <KEYPAIR_PATH>
          Default Solana signing keypair. Also used as the default payer unless --payer-keypair is provided

          [aliases: --keypair-path]

      --trader-subaccount-index <TRADER_SUBACCOUNT_INDEX>
          Trader subaccount index

          [default: 0]
          [aliases: --subaccount-index]

      --payer-keypair <PAYER_KEYPAIR_PATH>
          Fee-payer keypair. Overrides --keypair for transaction fee payment

          [aliases: --payer-keypair-path]

      --trader-account <TRADER_ACCOUNT>
          Explicit trader account address. When omitted, the CLI derives it from authority, trader PDA index, and subaccount index

      --ledger <LEDGER>
          Ledger locator/path, for example usb://ledger or usb://ledger?key=0/0

      --json
          Print compact JSON only. Useful for piping into scripts

      --pretty
          Pretty-print JSON output

  -h, --help
          Print help
```

#### `phoenix-rise trader reset-position-authority`

Reset a trader account's position authority back to its trader wallet

```text
Reset a trader account's position authority back to its trader wallet

Usage: phoenix-rise trader reset-position-authority [OPTIONS]

Options:
      --api-url <API_URL>
          Base API URL. Defaults to PHOENIX_API_URL, then https://perp-api.phoenix.trade

      --authority <AUTHORITY>
          Trader wallet authority pubkey. Defaults to global --keypair when omitted

      --rpc-url <RPC_URL>
          Solana RPC URL used by RPC and tx commands

      --trader-pda-index <TRADER_PDA_INDEX>
          Trader PDA index

          [default: 0]
          [aliases: --pda-index]

      --trader-subaccount-index <TRADER_SUBACCOUNT_INDEX>
          Trader subaccount index

          [default: 0]
          [aliases: --subaccount-index]

      --ws-url <WS_URL>
          Explicit WebSocket URL. Defaults to /v1/ws derived from the API URL

      --keypair <KEYPAIR_PATH>
          Default Solana signing keypair. Also used as the default payer unless --payer-keypair is provided

          [aliases: --keypair-path]

      --trader-account <TRADER_ACCOUNT>
          Explicit trader account address. When omitted, the CLI derives it from authority, trader PDA index, and subaccount index

      --payer-keypair <PAYER_KEYPAIR_PATH>
          Fee-payer keypair. Overrides --keypair for transaction fee payment

          [aliases: --payer-keypair-path]

      --ledger <LEDGER>
          Ledger locator/path, for example usb://ledger or usb://ledger?key=0/0

      --json
          Print compact JSON only. Useful for piping into scripts

      --pretty
          Pretty-print JSON output

  -h, --help
          Print help
```

#### `phoenix-rise trader place-market-order`

Build an isolated market-order instruction bundle

```text
Build an isolated market-order instruction bundle

Usage: phoenix-rise trader place-market-order [OPTIONS] --authority <AUTHORITY> --symbol <SYMBOL> --side <SIDE>

Options:
      --api-url <API_URL>
          Base API URL. Defaults to PHOENIX_API_URL, then https://perp-api.phoenix.trade

      --authority <AUTHORITY>


      --rpc-url <RPC_URL>
          Solana RPC URL used by RPC and tx commands

      --symbol <SYMBOL>


      --side <SIDE>
          [possible values: buy, sell, bid, ask]

      --ws-url <WS_URL>
          Explicit WebSocket URL. Defaults to /v1/ws derived from the API URL

      --keypair <KEYPAIR_PATH>
          Default Solana signing keypair. Also used as the default payer unless --payer-keypair is provided

          [aliases: --keypair-path]

      --num-base-lots <NUM_BASE_LOTS>


      --payer-keypair <PAYER_KEYPAIR_PATH>
          Fee-payer keypair. Overrides --keypair for transaction fee payment

          [aliases: --payer-keypair-path]

      --quantity <QUANTITY>


      --ledger <LEDGER>
          Ledger locator/path, for example usb://ledger or usb://ledger?key=0/0

      --transfer-amount <TRANSFER_AMOUNT>
          [default: 0]

      --json
          Print compact JSON only. Useful for piping into scripts

      --max-price-in-ticks <MAX_PRICE_IN_TICKS>


      --pda-index <PDA_INDEX>


      --pretty
          Pretty-print JSON output

      --fee-payer <FEE_PAYER>


      --position-authority <POSITION_AUTHORITY>


      --reduce-only <REDUCE_ONLY>
          [possible values: true, false]

      --skip-transfer-to-parent <SKIP_TRANSFER_TO_PARENT>
          [possible values: true, false]

      --allow-cross-and-isolated


      --enhanced


  -h, --help
          Print help
```

#### `phoenix-rise flight view`

Show a Flight builder's config and current withdrawable fee collateral

```text
Show a Flight builder's config and current withdrawable fee collateral

Usage: phoenix-rise flight view [OPTIONS] --authority <AUTHORITY>

Options:
      --api-url <API_URL>
          Base API URL. Defaults to PHOENIX_API_URL, then https://perp-api.phoenix.trade

      --authority <AUTHORITY>
          Builder authority pubkey

      --rpc-url <RPC_URL>
          Solana RPC URL used by RPC and tx commands

      --ws-url <WS_URL>
          Explicit WebSocket URL. Defaults to /v1/ws derived from the API URL

      --keypair <KEYPAIR_PATH>
          Default Solana signing keypair. Also used as the default payer unless --payer-keypair is provided

          [aliases: --keypair-path]

      --payer-keypair <PAYER_KEYPAIR_PATH>
          Fee-payer keypair. Overrides --keypair for transaction fee payment

          [aliases: --payer-keypair-path]

      --ledger <LEDGER>
          Ledger locator/path, for example usb://ledger or usb://ledger?key=0/0

      --json
          Print compact JSON only. Useful for piping into scripts

      --pretty
          Pretty-print JSON output

  -h, --help
          Print help
```

#### `phoenix-rise flight withdraw-collateral`

Build the Phoenix/Ember withdrawal flow for a Flight builder's collateralized fees

```text
Build the Phoenix/Ember withdrawal flow for a Flight builder's collateralized fees

Usage: phoenix-rise flight withdraw-collateral [OPTIONS] --authority <AUTHORITY>

Options:
      --api-url <API_URL>
          Base API URL. Defaults to PHOENIX_API_URL, then https://perp-api.phoenix.trade

      --authority <AUTHORITY>
          Builder authority pubkey. This account signs the withdrawal flow

      --rpc-url <RPC_URL>
          Solana RPC URL used by RPC and tx commands

      --trader-account <TRADER_ACCOUNT>
          Builder trader account. Defaults to the trader PDA derived from authority, --trader-pda-index, and --subaccount-index

      --trader-pda-index <TRADER_PDA_INDEX>
          Phoenix trader PDA index

          [default: 0]
          [aliases: --pda-index]

      --ws-url <WS_URL>
          Explicit WebSocket URL. Defaults to /v1/ws derived from the API URL

      --keypair <KEYPAIR_PATH>
          Default Solana signing keypair. Also used as the default payer unless --payer-keypair is provided

          [aliases: --keypair-path]

      --subaccount-index <SUBACCOUNT_INDEX>
          Phoenix subaccount index

          [default: 0]

      --amount <AMOUNT>
          Amount to withdraw in canonical token base units

      --payer-keypair <PAYER_KEYPAIR_PATH>
          Fee-payer keypair. Overrides --keypair for transaction fee payment

          [aliases: --payer-keypair-path]

      --ledger <LEDGER>
          Ledger locator/path, for example usb://ledger or usb://ledger?key=0/0

      --usdc-amount <USDC_AMOUNT>
          Amount to withdraw in USDC units. Prefer --amount for exact scripts

      --json
          Print compact JSON only. Useful for piping into scripts

      --pretty
          Pretty-print JSON output

  -h, --help
          Print help
```

#### `phoenix-rise tx parse`

Fetch a transaction and parse Phoenix instructions and events

```text
Fetch a transaction and parse Phoenix instructions and events

Usage: phoenix-rise tx parse [OPTIONS] --signature <SIGNATURE>

Options:
      --api-url <API_URL>
          Base API URL. Defaults to PHOENIX_API_URL, then https://perp-api.phoenix.trade

      --signature <SIGNATURE>
          Transaction signature to fetch from RPC

      --program-id <PROGRAM_ID>
          Phoenix program id whose events should be parsed. Defaults to the active SDK program id

      --rpc-url <RPC_URL>
          Solana RPC URL used by RPC and tx commands

      --with-errors
          Include malformed event payloads and parser failures in the output

      --ws-url <WS_URL>
          Explicit WebSocket URL. Defaults to /v1/ws derived from the API URL

      --keypair <KEYPAIR_PATH>
          Default Solana signing keypair. Also used as the default payer unless --payer-keypair is provided

          [aliases: --keypair-path]

      --payer-keypair <PAYER_KEYPAIR_PATH>
          Fee-payer keypair. Overrides --keypair for transaction fee payment

          [aliases: --payer-keypair-path]

      --ledger <LEDGER>
          Ledger locator/path, for example usb://ledger or usb://ledger?key=0/0

      --json
          Print compact JSON only. Useful for piping into scripts

      --pretty
          Pretty-print JSON output

  -h, --help
          Print help
```

#### `phoenix-rise ws trader-state`

Subscribe to trader-state snapshots and deltas

```text
Subscribe to trader-state snapshots and deltas

Usage: phoenix-rise ws trader-state [OPTIONS] --authority <AUTHORITY>

Options:
      --api-url <API_URL>
          Base API URL. Defaults to PHOENIX_API_URL, then https://perp-api.phoenix.trade

      --authority <AUTHORITY>


      --rpc-url <RPC_URL>
          Solana RPC URL used by RPC and tx commands

      --trader-pda-index <TRADER_PDA_INDEX>
          [default: 0]

      --timeout-secs <TIMEOUT_SECS>
          Maximum seconds to wait for each websocket message when --duration-secs is not set

          [default: 10]

      --ws-url <WS_URL>
          Explicit WebSocket URL. Defaults to /v1/ws derived from the API URL

      --keypair <KEYPAIR_PATH>
          Default Solana signing keypair. Also used as the default payer unless --payer-keypair is provided

          [aliases: --keypair-path]

      --messages <MESSAGES>
          Maximum messages to consume before printing the last one. Defaults to 1 unless --duration-secs is provided

      --duration-secs <DURATION_SECS>
          Total seconds to keep the subscription open before printing the last observed message

      --payer-keypair <PAYER_KEYPAIR_PATH>
          Fee-payer keypair. Overrides --keypair for transaction fee payment

          [aliases: --payer-keypair-path]

      --ledger <LEDGER>
          Ledger locator/path, for example usb://ledger or usb://ledger?key=0/0

      --json
          Print compact JSON only. Useful for piping into scripts

      --pretty
          Pretty-print JSON output

  -h, --help
          Print help
```

<!-- phoenix-rise-cli-command-docs:end -->

## Exchange Examples

```bash
phoenix-rise --json exchange keys
phoenix-rise --pretty exchange config
phoenix-rise --json exchange snapshot
phoenix-rise --json exchange status
phoenix-rise --json exchange referral-activation-permission
phoenix-rise --json exchange build-register-ixs \
  --trader-authority <TRADER_AUTHORITY> \
  --tx-fee-payer <PAYER> \
  --max-positions 128
```

## Market Examples

```bash
phoenix-rise --json market list
phoenix-rise --pretty market show --symbol SOL
phoenix-rise --json market orderbook --symbol SOL --include-splines
phoenix-rise --json market mark-price --symbol SOL
phoenix-rise --json market calendar --symbol SOL
phoenix-rise --json market commodity-calendar
phoenix-rise --json market next-transition --symbol SOL
phoenix-rise --json market funding-rates --symbol SOL --limit 24
```

## History And Candle Examples

Trader history and market candle commands follow cursors and merge pages for
you. Without `--json` or `--pretty`, they print a compact terminal table. With
`--json`, they print the full merged JSON payload. Time bounds accept unix
seconds, unix milliseconds, RFC3339, or `YYYY-MM-DD`; `--lookback` accepts
compact durations such as `7d`, `24h`, or `1h30m`.

```bash
phoenix-rise trader trade-history \
  --authority <AUTHORITY> \
  --market-symbol SOL \
  --lookback 7d

phoenix-rise --json trader trade-history \
  --authority <AUTHORITY> \
  --market-symbol SOL \
  --lookback 7d \
  --output trade-history.json

phoenix-rise trader order-history \
  --authority <AUTHORITY> \
  --start-time 2026-01-01T00:00:00Z \
  --end-time 2026-02-01T00:00:00Z

phoenix-rise trader funding-history \
  --authority <AUTHORITY> \
  --symbol SOL \
  --lookback 30d

phoenix-rise trader liquidation-history \
  --authority <AUTHORITY> \
  --lookback 90d

phoenix-rise market candles \
  --symbol SOL \
  --timeframe 1m \
  --lookback 24h \
  --output sol-1m-candles.json
```

## Flight Builder Examples

Flight builder fees are transferred into the builder's Phoenix trader account
as collateral. Use `flight withdraw-collateral` or its `withdraw-fees` alias to
build the Phoenix/Ember withdrawal flow for accrued builder fees. `claim-fees`
is the separate Phoenix risk-authority path for protocol fee claims.

```bash
phoenix-rise flight view \
  --authority <BUILDER_AUTHORITY>

phoenix-rise --json flight register-builder \
  --authority <BUILDER_AUTHORITY> \
  --fee-bps 25

phoenix-rise --json flight update-fee \
  --authority <BUILDER_AUTHORITY> \
  --fee-bps 20

phoenix-rise --ledger usb://ledger?key=0/0 --json flight withdraw-fees \
  --authority <BUILDER_AUTHORITY> \
  --amount 1000000

phoenix-rise --json flight claim-fees \
  --authority <RISK_AUTHORITY> \
  --amount 1000000
```

## Auth Examples

```bash
phoenix-rise --json auth login --keypair-path ~/.config/solana/id.json
phoenix-rise auth login --output env --keypair-path ~/.config/solana/id.json
phoenix-rise --json auth refresh
phoenix-rise --json auth session
phoenix-rise --json auth cache-path
phoenix-rise auth logout
```

## Register Trader

Without a referral code, the CLI uses
`/v1/exchange/build-register-ixs`, signs the returned transaction with the
fee payer, then submits it to `/v1/exchange/send-register-ixs`. The trader
authority keypair is not required for this builder-onboarding path.

```bash
phoenix-rise --keypair ~/.config/solana/payer.json --json trader register \
  --authority <TRADER_AUTHORITY>

phoenix-rise --payer-keypair ~/.config/solana/payer.json --json trader register \
  --authority <TRADER_AUTHORITY>
```

With a referral code, the trader authority must sign. Use global `--keypair`
for the authority keypair. If `--payer-keypair` is also provided, it is used as
the transaction fee payer and the authority is included as an additional signer.

```bash
phoenix-rise --keypair ~/.config/solana/authority.json --json trader register \
  --referral-code <CODE>

phoenix-rise --keypair ~/.config/solana/authority.json \
  --payer-keypair ~/.config/solana/payer.json \
  --json trader register \
  --referral-code <CODE>
```

## RPC Account Examples

```bash
phoenix-rise --rpc-url https://api.mainnet-beta.solana.com --json rpc account \
  --address <ACCOUNT_PUBKEY> \
  --account-type perp-asset-map

phoenix-rise --json rpc account \
  --address <TRADER_ACCOUNT> \
  --account-type trader

phoenix-rise --json rpc perp-asset \
  --perp-asset-map <PERP_ASSET_MAP_ACCOUNT> \
  --symbol SOL
```

Supported account types include `conditional-orders`,
`global-configuration`, `mint`, `orderbook`, `orderbook-header`,
`permission`, `perp-asset-map`, `spline-collection`, `stop-losses`,
`token-account`, `trader`, `withdraw-queue`, and `withdraw-queue-header`.

## Trader Summary

```bash
phoenix-rise --json trader state \
  --authority <AUTHORITY> \
  --pda-index 0 \
  --subaccount-index 0

phoenix-rise --json trader summary \
  --authority <AUTHORITY> \
  --pda-index 0 \
  --subaccount-index 0

phoenix-rise --json trader pnl \
  --authority <AUTHORITY> \
  --resolution 1d \
  --limit 30

phoenix-rise --json trader collateral-history \
  --authority <AUTHORITY> \
  --pda-index 0 \
  --limit 20

phoenix-rise --json trader funding-hourly \
  --authority <AUTHORITY> \
  --symbol SOL \
  --limit 20
```

The summary combines trader state, position diagnostics, recent PnL, orders,
collateral, funding, and trades. Position diagnostics include stop-loss and
liquidation distance percentages when the API response has enough price data.

## Trading Instruction Examples

These commands call API instruction-building endpoints and print Solana
instructions. They do not sign or submit transactions.

```bash
phoenix-rise --json trader place-market-order \
  --authority <AUTHORITY> \
  --symbol SOL \
  --side buy \
  --quantity 1.0 \
  --transfer-amount 1000000 \
  --enhanced

phoenix-rise --json trader place-limit-order \
  --authority <AUTHORITY> \
  --symbol SOL \
  --side sell \
  --price 250.0 \
  --num-base-lots 10 \
  --post-only true

phoenix-rise --json trader place-stop-loss \
  --authority <AUTHORITY> \
  --symbol SOL \
  --side sell \
  --stop-loss-trigger-price 220 \
  --stop-loss-execution-price 219

phoenix-rise --json trader cancel-stop-loss \
  --authority <AUTHORITY> \
  --symbol SOL \
  --execution-direction less-than
```

## Transaction Parsing

```bash
phoenix-rise --json tx parse \
  --signature <SIGNATURE> \
  --with-errors
```

`tx parse` fetches the transaction from RPC, decodes Phoenix instructions, and
attaches emitted Phoenix market events using the same stack-height-aware parser
as `phoenix-rise-core`.

## WebSocket Examples

Each websocket command subscribes and prints one message before exiting.
Use `--messages <N>` to consume multiple messages and print the last one, or
`--duration-secs <N>` to keep the subscription open for a fixed test window.
For trader-state subscriptions, the CLI applies snapshots and deltas before
printing the final materialized trader state.

```bash
phoenix-rise --json ws all-mids
phoenix-rise --json ws market --symbol SOL
phoenix-rise --json ws orderbook --symbol SOL --timeout-secs 20 --messages 5
phoenix-rise --json ws funding-rate --symbol SOL
phoenix-rise --json ws trades --symbol SOL
phoenix-rise --json ws candles --symbol SOL --timeframe 1m
phoenix-rise ws trader-state --authority <AUTHORITY> --trader-pda-index 0 --duration-secs 30
phoenix-rise --json ws trader-state --authority <AUTHORITY> --trader-pda-index 0 --messages 10
```

## Smoke Checks

```bash
rise/rust/cli/scripts/smoke_http_client.sh --symbol SOL
rise/rust/cli/scripts/smoke_http_client.sh --authority <AUTHORITY> --symbol SOL
```

The smoke script runs public JSON API checks against the local CLI command tree
and fails if any `--json` command prints malformed JSON. When an authority or
keypair is available it also exercises auth, trader, and history routes. It
uses `--calendar-symbol WTIOIL` by default for calendar-specific checks because
not every market has a configured market calendar.

From the repository root, the `rise` mise task forwards arguments to the local
CLI binary:

```bash
mise rise -- market list --json
mise rise -- trader summary --authority <AUTHORITY> --json
mise rise -- tx parse --signature <SIGNATURE> --json
```

## Crate Links

[`phoenix-rise`](../sdk/README.md) |
[`phoenix-rise-api`](../api/README.md) |
[`phoenix-rise-accounts`](../accounts/README.md) |
[`phoenix-rise-events`](../events/README.md)
