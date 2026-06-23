# Phoenix Trader Onboarder

This is a small Pinocchio CPI example program for teams that want a program
PDA to hold Phoenix trader-onboarding delegation.

The program uses one static PDA as its delegated Phoenix authority:

```text
program id: 9AuJXxrJqWjG34BJD5k5xKY77LGQz8xa8kY5dLRFdW2i
seed:       "trader-onboarder"
```

The Phoenix exchange grants trader-onboarding permission to that PDA by
creating and setting the Phoenix permission account derived from:

```text
["permission", phoenix_risk_authority, trader_onboarder_pda]
```

## Instructions

### `create_trader_idempotent`

It:

- validates the static onboarder PDA,
- validates the Phoenix program, log authority, global configuration, system
  program, permission PDA, dynamic trader-index headers, and trader PDA,
- verifies the permission account grants trader-onboarding permission from the
  Phoenix risk authority to the static onboarder PDA and logs remaining signer
  uses,
- derives the dynamic GlobalTraderIndex and ActiveTraderBuffer account counts
  from the on-chain header superblocks identified by Phoenix global config,
- checks whether the Phoenix trader account already exists,
- calls Phoenix `RegisterTrader` when the trader is missing,
- always creates the default cross-margin trader account with
  `max_positions=128`, `trader_pda_index=0`, and `trader_subaccount_index=0`,
- makes the provided payer the Phoenix `RegisterTrader` payer, so that key pays
  trader-account rent and becomes the trader header `funding_key`,
- reads the trader header, capability flags, hot/cold status, and position-map
  length/capacity,
- calls Phoenix `SetTraderCapabilitiesDelegated` with the program PDA signer
  only when the trader is missing market-order, deposit, or withdraw
  capability.

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

Instruction data uses an 8-byte Rise-style discriminator for
`global:create_trader_idempotent` and no trailing params.

### `register_subaccount_idempotent`

This instruction registers an isolated child subaccount and syncs it with the
default cross-margin parent account.

It:

- takes a single `u8 subaccount_index` instruction param,
- rejects `subaccount_index=0`,
- validates the Phoenix program, log authority, global configuration, system
  program, dynamic trader-index header, parent trader PDA, and child trader PDA,
- always derives the child trader PDA with `trader_pda_index=0` and the
  provided nonzero `subaccount_index`,
- always verifies the parent cross-margin account is the expected
  `(trader_pda_index=0, trader_subaccount_index=0)` PDA for the same trader
  authority,
- requires the parent cross-margin trader to exist and be owned by Phoenix,
- checks whether the child Phoenix trader account already exists,
- calls Phoenix `RegisterTrader` with `max_positions=1` when the child is
  missing,
- makes the provided payer the Phoenix `RegisterTrader` payer, so that key pays
  child-account rent and becomes the child trader header `funding_key`,
- calls Phoenix `SyncParentToChild` only immediately after registering the
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

Instruction data uses an 8-byte Rise-style discriminator for
`global:register_subaccount_idempotent` followed by one byte:
`subaccount_index`.

## Permission Notes

Phoenix trader-onboarding delegation is intentionally narrow. With only the
`trader-onboarding` permission bit, Phoenix allows delegated capability updates
for frozen traders while the exchange is gated. If a team needs this program to
change capabilities outside that onboarding path, the Phoenix permission account
must be granted a broader permission such as trader management.

## Tests

From the `rise` workspace:

```sh
cargo build-sbf --manifest-path programs/trader-onboarder/Cargo.toml
cargo nextest run --manifest-path programs/trader-onboarder/Cargo.toml --no-capture
```

Set `RISE_TRADER_ONBOARDER_SO` to point at a prebuilt example program artifact
when using a non-default output path.
