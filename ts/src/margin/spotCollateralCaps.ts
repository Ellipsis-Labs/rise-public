/**
 * Spot collateral cap arithmetic.
 *
 * Native SOL has no deposit instruction. Its lamports sit in the trader account
 * itself, and the permissionless `SyncNative` crank reconciles the *accounted*
 * collateral against the account's actual lamports. See `buildSyncNativeIx`.
 *
 * The consequence callers must model is that an upward sync is **clamped, not
 * rejected**. `SyncNative` asks to set the trader's accounted balance to
 * `lamports - rent`, and the program credits only
 *
 * ```text
 * credit = min(requested - accounted, maxPerTraderBalance - accounted,
 *              maxGlobalBalance - currGlobalBalance)
 * ```
 *
 * Whatever the clamp leaves over stays in the account as uncounted excess: it
 * carries no margin value, produces no error, and no transaction fails. A
 * client that offers a MAX larger than the headroom therefore sends SOL that is
 * silently not credited, which is why this arithmetic belongs here rather than
 * being restated per client.
 *
 * These are pure restatements of `SpotCollateralMetadata::update_total_collateral`
 * and `process_sync_native` in the on-chain program. Keep them in step.
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

export interface NativeSolCollateralHeadroomParams {
  metadata: SpotCollateralCapParams;
  /** The trader's currently accounted native SOL collateral, in lamports. */
  traderNativeSolLamports: bigint;
  /**
   * Signed reconciliation delta from {@link nativeSolSyncDeltaLamports}.
   *
   * `SyncNative` credits one lump — it does not distinguish pre-existing excess
   * from freshly deposited lamports — so positive excess consumes headroom a
   * new deposit would otherwise get, and a negative rent deficit is refilled by
   * the deposit before anything is credited. Omitting it assumes the account is
   * exactly reconciled.
   */
  traderSyncDeltaLamports?: bigint;
}

/**
 * How many *additional* lamports a trader can deposit before some of them land
 * uncounted, against both caps the program clamps to.
 *
 * With a rent deficit the first `-traderSyncDeltaLamports` of the deposit
 * refill the rent floor rather than being credited, so the cap-imposed maximum
 * grows by the deficit; nothing lands *uncounted* below the returned value
 * either way.
 */
export const nativeSolCollateralHeadroomLamports = ({
  metadata,
  traderNativeSolLamports,
  traderSyncDeltaLamports = 0n,
}: NativeSolCollateralHeadroomParams): bigint => {
  const excess = clampToZero(traderSyncDeltaLamports);
  const deficit = clampToZero(-traderSyncDeltaLamports);
  const perTraderHeadroom = clampToZero(
    metadata.maxPerTraderBalance - traderNativeSolLamports - excess
  );
  const globalHeadroom = clampToZero(
    metadata.maxGlobalBalance - metadata.currGlobalBalance - excess
  );

  return minBigint([perTraderHeadroom, globalHeadroom]) + deficit;
};

export interface AttributedNativeSolDepositParams extends NativeSolCollateralHeadroomParams {
  /** Lamports the caller is about to transfer into the trader account. */
  depositLamports: bigint;
}

/**
 * Of `depositLamports`, how many the next `SyncNative` will actually credit.
 *
 * The single credit covers pre-existing excess first, and a rent deficit
 * swallows deposited lamports before any are credited — so a deposit smaller
 * than the deficit returns `0n` even though the sync will still lower the
 * accounted balance (by less than it would have without the deposit). Use this
 * to report "you were credited X", not
 * {@link nativeSolCollateralHeadroomLamports}, which answers the different
 * question of how large a deposit can be before any of it is left uncredited.
 */
export const attributedNativeSolDepositLamports = ({
  metadata,
  traderNativeSolLamports,
  traderSyncDeltaLamports = 0n,
  depositLamports,
}: AttributedNativeSolDepositParams): bigint => {
  const perTraderHeadroom = clampToZero(
    metadata.maxPerTraderBalance - traderNativeSolLamports
  );
  const globalHeadroom = clampToZero(
    metadata.maxGlobalBalance - metadata.currGlobalBalance
  );
  const requestedCredit = clampToZero(
    traderSyncDeltaLamports + depositLamports
  );
  const credit = minBigint([
    requestedCredit,
    perTraderHeadroom,
    globalHeadroom,
  ]);

  // Pre-existing excess is credited first; only what is left lands on the
  // new deposit.
  return clampToZero(credit - clampToZero(traderSyncDeltaLamports));
};
