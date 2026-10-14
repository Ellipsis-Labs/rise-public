/**
 * Spot collateral cap arithmetic.
 *
 * Spot collateral has no deposit instruction. Native SOL lamports sit in the
 * trader account itself and SPL tokens in the trader wallet PDA's custody ATA;
 * the permissionless `SyncNative` / `SyncSpot` cranks reconcile the *accounted*
 * collateral against those balances. See `buildSyncNativeIx` and
 * `buildSyncSpotIx`.
 *
 * The consequence callers must model is that an upward sync is **clamped, not
 * rejected**. The sync asks to set the trader's accounted balance to
 * `lamports - rent` (native SOL) or the custody ATA's amount (SPL), and the
 * program credits only
 *
 * ```text
 * credit = min(requested - accounted, traderMaxBalance - accounted,
 *              maxGlobalBalance - currGlobalBalance)
 * ```
 *
 * where `traderMaxBalance` is the trader's effective per-trader cap, which can
 * differ from the asset's `maxPerTraderBalance`.
 *
 * Whatever the clamp leaves over stays in the account as uncounted excess: it
 * carries no margin value, produces no error, and no transaction fails. A
 * client that offers a MAX larger than the headroom therefore sends SOL that is
 * silently not credited, which is why this arithmetic belongs here rather than
 * being restated per client.
 *
 * These are pure restatements of `SpotCollateralMetadata::update_total_collateral`,
 * `process_sync_native` and `process_sync_spot` in the on-chain program. Keep them in step.
 */

import type { SpotCollateralMetadata } from "@/accounts/SpotCollateralMetadata/types";

const clampToZero = (value: bigint): bigint => (value < 0n ? 0n : value);

const minBigint = (values: readonly bigint[]): bigint =>
  values.reduce((smallest, value) => (value < smallest ? value : smallest));

/** The cap fields a headroom calculation reads. */
export type SpotCollateralCapParams = Pick<
  SpotCollateralMetadata,
  "maxPerTraderBalance" | "maxGlobalBalance" | "currGlobalBalance"
>;

export interface NativeSolAccountStateParams {
  /** Total lamports in the trader account, rent included. */
  accountBalanceLamports: bigint;
  /** Lamports already accounted as native SOL collateral for this trader. */
  accountedLamports: bigint;
  /**
   * The account's rent-exempt minimum. On-chain this is
   * `Rent::minimum_balance(trader_account.data_len())`, so it must be read for
   * the trader account's own data length rather than assumed — see
   * `getLamportAccountState` on the rpc client.
   */
  rentExemptMinimumLamports: bigint;
}

/**
 * The signed change the next `SyncNative` would apply to the trader's accounted
 * collateral, before any new deposit: `balance - accounted - rent`.
 *
 * Positive is uncounted excess a sync would credit (caps permitting). Negative
 * means the rent floor grew past the accounted balance (a trader account
 * realloc), which a sync settles as a *downward* reconciliation — deposited
 * lamports refill that deficit before any of them are credited. Pass this value
 * as `traderSyncDeltaLamports` to the deposit helpers so both signs are
 * handled.
 */
export const nativeSolSyncDeltaLamports = ({
  accountBalanceLamports,
  accountedLamports,
  rentExemptMinimumLamports,
}: NativeSolAccountStateParams): bigint =>
  accountBalanceLamports - accountedLamports - rentExemptMinimumLamports;

/**
 * Uncounted excess lamports sitting in the trader account: the positive part of
 * {@link nativeSolSyncDeltaLamports}.
 *
 * This is the display quantity ("excess a sync or excess-withdraw would pick
 * up"). For deposit sizing and credit attribution pass the signed delta
 * instead: clamping here discards a rent deficit, which would overstate what a
 * deposit gets credited.
 */
export const nativeSolUnaccountedLamports = (
  params: NativeSolAccountStateParams
): bigint => clampToZero(nativeSolSyncDeltaLamports(params));

export interface SpotCustodyStateParams {
  /** Token amount in the trader wallet PDA's custody ATA for the asset. */
  custodyAmount: bigint;
  /** Amount already accounted as this asset's spot collateral for the trader. */
  accountedAmount: bigint;
}

/**
 * The signed change the next `SyncSpot` would apply to the trader's accounted
 * balance of an SPL asset, before any new deposit: `custodyAmount - accounted`.
 * The SPL counterpart of {@link nativeSolSyncDeltaLamports}; there is no rent
 * floor, so it is the custody ATA's token amount that is reconciled. Pass it as
 * `traderSyncDelta` to the deposit helpers.
 */
export const spotSyncDelta = ({
  custodyAmount,
  accountedAmount,
}: SpotCustodyStateParams): bigint => custodyAmount - accountedAmount;

export interface SpotCollateralHeadroomParams {
  metadata: SpotCollateralCapParams;
  /** The trader's currently accounted balance of the asset, in native units. */
  traderBalance: bigint;
  /**
   * The trader's effective per-trader cap, in native units:
   * `BigInt(row.maxBalance)` from the asset's row of trader state. Omit only
   * when the trader has no account yet, in which case
   * `metadata.maxPerTraderBalance` applies.
   */
  traderMaxBalance?: bigint;
  /**
   * Signed reconciliation delta: {@link nativeSolSyncDeltaLamports} for native
   * SOL, {@link spotSyncDelta} for SPL assets.
   *
   * A sync credits one lump — it does not distinguish pre-existing excess from
   * freshly deposited units — so positive excess consumes headroom a new
   * deposit would otherwise get, and a negative deficit is refilled by the
   * deposit before anything is credited. Omitting it assumes the account is
   * exactly reconciled.
   */
  traderSyncDelta?: bigint;
}

/**
 * How many *additional* native units a trader can deposit before some of them
 * land uncounted, against both caps the program clamps to.
 *
 * With a deficit the first `-traderSyncDelta` of the deposit refill it rather
 * than being credited, so the cap-imposed maximum grows by the deficit; nothing
 * lands *uncounted* below the returned value either way.
 */
export const spotCollateralHeadroom = ({
  metadata,
  traderBalance,
  traderMaxBalance,
  traderSyncDelta = 0n,
}: SpotCollateralHeadroomParams): bigint => {
  const excess = clampToZero(traderSyncDelta);
  const deficit = clampToZero(-traderSyncDelta);
  const perTraderHeadroom = clampToZero(
    (traderMaxBalance ?? metadata.maxPerTraderBalance) - traderBalance - excess
  );
  const globalHeadroom = clampToZero(
    metadata.maxGlobalBalance - metadata.currGlobalBalance - excess
  );

  return minBigint([perTraderHeadroom, globalHeadroom]) + deficit;
};

export interface AttributedSpotDepositParams extends SpotCollateralHeadroomParams {
  /** Native units the caller is about to deposit. */
  depositAmount: bigint;
}

/**
 * Of `depositAmount`, how many units the next sync will actually credit.
 *
 * The single credit covers pre-existing excess first, and a deficit swallows
 * deposited units before any are credited — so a deposit smaller than the
 * deficit returns `0n` even though the sync will still lower the accounted
 * balance (by less than it would have without the deposit). Use this to report
 * "you were credited X", not {@link spotCollateralHeadroom}, which answers the
 * different question of how large a deposit can be before any of it is left
 * uncredited.
 */
export const attributedSpotDeposit = ({
  metadata,
  traderBalance,
  traderMaxBalance,
  traderSyncDelta = 0n,
  depositAmount,
}: AttributedSpotDepositParams): bigint => {
  const perTraderHeadroom = clampToZero(
    (traderMaxBalance ?? metadata.maxPerTraderBalance) - traderBalance
  );
  const globalHeadroom = clampToZero(
    metadata.maxGlobalBalance - metadata.currGlobalBalance
  );
  const requestedCredit = clampToZero(traderSyncDelta + depositAmount);
  const credit = minBigint([
    requestedCredit,
    perTraderHeadroom,
    globalHeadroom,
  ]);

  // Pre-existing excess is credited first; only what is left lands on the
  // new deposit.
  return clampToZero(credit - clampToZero(traderSyncDelta));
};

export interface NativeSolCollateralHeadroomParams {
  metadata: SpotCollateralCapParams;
  /** The trader's currently accounted native SOL collateral, in lamports. */
  traderNativeSolLamports: bigint;
  /** See {@link SpotCollateralHeadroomParams.traderMaxBalance}. */
  traderMaxBalanceLamports?: bigint;
  /** Signed delta from {@link nativeSolSyncDeltaLamports}; a negative value is
   * a rent deficit. */
  traderSyncDeltaLamports?: bigint;
}

/** {@link spotCollateralHeadroom} for native SOL, in lamports. */
export const nativeSolCollateralHeadroomLamports = (
  params: NativeSolCollateralHeadroomParams
): bigint => spotCollateralHeadroom(fromNativeSolParams(params));

export interface AttributedNativeSolDepositParams extends NativeSolCollateralHeadroomParams {
  /** Lamports the caller is about to transfer into the trader account. */
  depositLamports: bigint;
}

/** {@link attributedSpotDeposit} for native SOL, in lamports. */
export const attributedNativeSolDepositLamports = (
  params: AttributedNativeSolDepositParams
): bigint =>
  attributedSpotDeposit({
    ...fromNativeSolParams(params),
    depositAmount: params.depositLamports,
  });

const fromNativeSolParams = ({
  metadata,
  traderNativeSolLamports,
  traderMaxBalanceLamports,
  traderSyncDeltaLamports,
}: NativeSolCollateralHeadroomParams): SpotCollateralHeadroomParams => ({
  metadata,
  traderBalance: traderNativeSolLamports,
  traderMaxBalance: traderMaxBalanceLamports,
  traderSyncDelta: traderSyncDeltaLamports,
});
