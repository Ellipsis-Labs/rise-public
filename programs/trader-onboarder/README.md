# Phoenix Trader Onboarder

Pinocchio CPI example program for delegating Phoenix trader onboarding to a
program PDA.

The program uses one static PDA as its delegated Phoenix authority:

```text
program id: 9AuJXxrJqWjG34BJD5k5xKY77LGQz8xa8kY5dLRFdW2i
seed:       "trader-onboarder"
```

Phoenix grants trader-onboarding permission to that PDA via the permission
account derived from:

```text
["permission", phoenix_risk_authority, trader_onboarder_pda]
```

## Instructions

### `create_trader_idempotent`

Registers and enables the default cross-margin trader account:

- validates the static onboarder PDA, Phoenix program, log authority, global
  configuration, system program, permission PDA, dynamic trader-index headers,
  and trader PDA,
- verifies that the permission account grants trader-onboarding permission from
  the Phoenix risk authority to the static onboarder PDA, and logs remaining
  signer uses,
- derives dynamic GlobalTraderIndex and ActiveTraderBuffer account counts from
  the on-chain header superblocks referenced by Phoenix global config,
- checks whether the Phoenix trader account exists, and calls Phoenix
  `RegisterTrader` if missing (`max_positions=128`, `trader_pda_index=0`,
  `trader_subaccount_index=0`),
- passes the provided payer as the Phoenix `RegisterTrader` payer (paying
  trader-account rent and becoming the trader header `funding_key`),
- reads the trader header, capability flags, hot/cold status, and position-map
  length/capacity,
- calls Phoenix `SetTraderCapabilitiesDelegated` with the program PDA signer
  only when the trader lacks market-order, deposit, or withdraw capability.

Account order:

```text
0. payer                   writable signer, pays trader-account rent
1. trader_authority        signer, owns the trader
2. onboarder_authority     static PDA: ["trader-onboarder"]
3. permission_account      writable Phoenix permission PDA
4. phoenix_program         Phoenix Eternal program id
5. phoenix_log_authority   Phoenix log authority PDA
6. phoenix_global_config   Phoenix global configuration
7. trader_account          writable Phoenix trader PDA
8. system_program          system program
9. global_trader_index     dynamic Phoenix GlobalTraderIndex account group
N. active_trader_buffer    dynamic Phoenix ActiveTraderBuffer account group
```

Instruction data: 8-byte Rise-style discriminator for
`global:create_trader_idempotent` with no trailing params.

### `register_subaccount_idempotent`

Registers an isolated child subaccount and syncs it with the default
cross-margin parent account:

- takes a single `u8 subaccount_index` param (rejects `subaccount_index=0`),
- validates the Phoenix program, log authority, global configuration, system
  program, dynamic trader-index header, parent trader PDA, and child trader PDA,
- derives the child trader PDA with `trader_pda_index=0` and the nonzero
  `subaccount_index`,
- verifies that the parent cross-margin account is the
  `(trader_pda_index=0, trader_subaccount_index=0)` PDA for the same trader
  authority, exists, and is owned by Phoenix,
- checks whether the child Phoenix trader account exists, and calls Phoenix
  `RegisterTrader` with `max_positions=1` if missing,
- passes the provided payer as the Phoenix `RegisterTrader` payer (paying
  child-account rent and becoming the child trader header `funding_key`),
- calls Phoenix `SyncParentToChild` only immediately after registering a new
  child account.

Account order:

```text
0. payer                   writable signer, pays child-account rent
1. trader_authority        signer, owns the traders
2. phoenix_program         Phoenix Eternal program id
3. phoenix_log_authority   Phoenix log authority PDA
4. phoenix_global_config   Phoenix global configuration
5. parent_trader_account   readonly Phoenix trader PDA for (0, 0)
6. child_trader_account    writable Phoenix trader PDA for (0, subaccount_index)
7. system_program          system program
8. global_trader_index     dynamic Phoenix GlobalTraderIndex account group
```

Instruction data: 8-byte Rise-style discriminator for
`global:register_subaccount_idempotent` followed by a 1-byte `subaccount_index`.

## Permission Notes

Phoenix trader-onboarding delegation is narrowly scoped: the `trader-onboarding`
permission bit only permits delegated capability updates for frozen traders
while the exchange is gated. Updating capabilities outside that onboarding path
requires granting the Phoenix permission account a broader permission such as
trader management.

## Tests

From the repository root:

```sh
cargo build-sbf --manifest-path programs/trader-onboarder/Cargo.toml
cargo nextest run --manifest-path programs/trader-onboarder/Cargo.toml --no-capture
```

Set `RISE_TRADER_ONBOARDER_SO` to point at a prebuilt program artifact when
using a non-default output path.
