import { describe, expect, it } from "vitest";
import {
  attributedNativeSolDepositLamports,
  nativeSolCollateralHeadroomLamports,
  nativeSolSyncDeltaLamports,
  nativeSolUnaccountedLamports,
  type SpotCollateralCapParams,
} from "@/margin/spotCollateralCaps";

const metadata = (params: SpotCollateralCapParams): SpotCollateralCapParams =>
  params;

/**
 * Reference implementation of the on-chain reconciliation
 * (`SpotCollateralMetadata::update_total_collateral` fed by
 * `process_sync_native`), used to check the exported helpers against the rule
 * they restate rather than against hand-computed numbers alone. Returns the
 * signed change to the trader's accounted balance.
 */
const onChainSyncChange = (
  caps: SpotCollateralCapParams,
  accounted: bigint,
  balanceMinusRent: bigint
): bigint => {
  if (balanceMinusRent <= accounted) {
    // Debit path: applies in full, no caps.
    return balanceMinusRent - accounted;
  }
  const requestedCredit = balanceMinusRent - accounted;
  const perTrader =
    caps.maxPerTraderBalance > accounted
      ? caps.maxPerTraderBalance - accounted
      : 0n;
  const global =
    caps.maxGlobalBalance > caps.currGlobalBalance
      ? caps.maxGlobalBalance - caps.currGlobalBalance
      : 0n;
  return [requestedCredit, perTrader, global].reduce((a, b) => (b < a ? b : a));
};

describe("nativeSolSyncDeltaLamports", () => {
  it("is positive for uncounted excess and negative for a rent deficit", () => {
    expect(
      nativeSolSyncDeltaLamports({
        accountBalanceLamports: 1_500n,
        accountedLamports: 400n,
        rentExemptMinimumLamports: 600n,
      })
    ).toBe(500n);
    expect(
      nativeSolSyncDeltaLamports({
        accountBalanceLamports: 1_000n,
        accountedLamports: 900n,
        rentExemptMinimumLamports: 400n,
      })
    ).toBe(-300n);
  });
});

describe("nativeSolUnaccountedLamports", () => {
  it("removes rent and accounted collateral from the raw account balance", () => {
    expect(
      nativeSolUnaccountedLamports({
        accountBalanceLamports: 1_500n,
        accountedLamports: 400n,
        rentExemptMinimumLamports: 600n,
      })
    ).toBe(500n);
  });

  it("clamps stale or incomplete account data to zero", () => {
    expect(
      nativeSolUnaccountedLamports({
        accountBalanceLamports: 500n,
        accountedLamports: 400n,
        rentExemptMinimumLamports: 600n,
      })
    ).toBe(0n);
  });
});

describe("nativeSolCollateralHeadroomLamports", () => {
  it("uses the per-trader cap when it binds first", () => {
    expect(
      nativeSolCollateralHeadroomLamports({
        metadata: metadata({
          maxPerTraderBalance: 100n,
          maxGlobalBalance: 1_000n,
          currGlobalBalance: 0n,
        }),
        traderNativeSolLamports: 40n,
      })
    ).toBe(60n);
  });

  it("uses the exchange-wide cap when it binds first", () => {
    expect(
      nativeSolCollateralHeadroomLamports({
        metadata: metadata({
          maxPerTraderBalance: 1_000n,
          maxGlobalBalance: 1_000n,
          currGlobalBalance: 990n,
        }),
        traderNativeSolLamports: 0n,
      })
    ).toBe(10n);
  });

  it("clamps to zero when a trader is already over the per-trader cap", () => {
    expect(
      nativeSolCollateralHeadroomLamports({
        metadata: metadata({
          maxPerTraderBalance: 100n,
          maxGlobalBalance: 1_000n,
          currGlobalBalance: 0n,
        }),
        traderNativeSolLamports: 150n,
      })
    ).toBe(0n);
  });

  it("clamps to zero when the exchange-wide cap is already exceeded", () => {
    expect(
      nativeSolCollateralHeadroomLamports({
        metadata: metadata({
          maxPerTraderBalance: 1_000n,
          maxGlobalBalance: 500n,
          currGlobalBalance: 600n,
        }),
        traderNativeSolLamports: 0n,
      })
    ).toBe(0n);
  });

  it("subtracts existing unaccounted SOL that the next sync consumes first", () => {
    expect(
      nativeSolCollateralHeadroomLamports({
        metadata: metadata({
          maxPerTraderBalance: 1_000n,
          maxGlobalBalance: 2_000n,
          currGlobalBalance: 500n,
        }),
        traderNativeSolLamports: 400n,
        traderSyncDeltaLamports: 250n,
      })
    ).toBe(350n);
  });

  it("grows by a rent deficit, which the deposit refills before crediting", () => {
    expect(
      nativeSolCollateralHeadroomLamports({
        metadata: metadata({
          maxPerTraderBalance: 1_000n,
          maxGlobalBalance: 2_000n,
          currGlobalBalance: 500n,
        }),
        traderNativeSolLamports: 400n,
        traderSyncDeltaLamports: -100n,
      })
    ).toBe(700n);
  });

  it("returns the largest deposit the on-chain reconciliation leaves fully counted", () => {
    const caps = metadata({
      maxPerTraderBalance: 1_000n,
      maxGlobalBalance: 2_000n,
      currGlobalBalance: 500n,
    });

    for (const accounted of [0n, 400n]) {
      for (const delta of [-100n, 0n, 250n]) {
        const headroom = nativeSolCollateralHeadroomLamports({
          metadata: caps,
          traderNativeSolLamports: accounted,
          traderSyncDeltaLamports: delta,
        });

        // Depositing exactly the headroom leaves no uncounted excess...
        const change = onChainSyncChange(
          caps,
          accounted,
          accounted + delta + headroom
        );
        expect(change).toBe(delta + headroom);
        // ...and one lamport more is silently left uncredited.
        expect(
          onChainSyncChange(caps, accounted, accounted + delta + headroom + 1n)
        ).toBe(change);
      }
    }
  });
});

describe("attributedNativeSolDepositLamports", () => {
  it("credits the whole deposit when both caps have room", () => {
    expect(
      attributedNativeSolDepositLamports({
        metadata: metadata({
          maxPerTraderBalance: 1_000n,
          maxGlobalBalance: 2_000n,
          currGlobalBalance: 0n,
        }),
        traderNativeSolLamports: 0n,
        depositLamports: 100n,
      })
    ).toBe(100n);
  });

  it("gives pre-existing excess priority over the new deposit", () => {
    // 60 lamports of headroom, 50 of which the existing excess consumes first.
    expect(
      attributedNativeSolDepositLamports({
        metadata: metadata({
          maxPerTraderBalance: 100n,
          maxGlobalBalance: 1_000n,
          currGlobalBalance: 0n,
        }),
        traderNativeSolLamports: 40n,
        traderSyncDeltaLamports: 50n,
        depositLamports: 100n,
      })
    ).toBe(10n);
  });

  it("credits nothing once the excess alone fills the headroom", () => {
    expect(
      attributedNativeSolDepositLamports({
        metadata: metadata({
          maxPerTraderBalance: 100n,
          maxGlobalBalance: 1_000n,
          currGlobalBalance: 0n,
        }),
        traderNativeSolLamports: 40n,
        traderSyncDeltaLamports: 60n,
        depositLamports: 100n,
      })
    ).toBe(0n);
  });

  it("reports no credit when a deposit is swallowed by a rent deficit", () => {
    // Accounted 900, balance-rent 800: the next sync is a downward
    // reconciliation. A 50 lamport deposit softens the drop but credits
    // nothing, and previously-clamped arithmetic would have reported 50.
    expect(
      attributedNativeSolDepositLamports({
        metadata: metadata({
          maxPerTraderBalance: 10_000n,
          maxGlobalBalance: 20_000n,
          currGlobalBalance: 1_000n,
        }),
        traderNativeSolLamports: 900n,
        traderSyncDeltaLamports: -100n,
        depositLamports: 50n,
      })
    ).toBe(0n);
  });

  it("credits only what remains after a rent deficit is refilled", () => {
    expect(
      attributedNativeSolDepositLamports({
        metadata: metadata({
          maxPerTraderBalance: 10_000n,
          maxGlobalBalance: 20_000n,
          currGlobalBalance: 1_000n,
        }),
        traderNativeSolLamports: 900n,
        traderSyncDeltaLamports: -100n,
        depositLamports: 150n,
      })
    ).toBe(50n);
  });

  it("matches the on-chain reconciliation across cap, excess, and deficit combinations", () => {
    const caps = metadata({
      maxPerTraderBalance: 1_000n,
      maxGlobalBalance: 1_500n,
      currGlobalBalance: 900n,
    });

    for (const accounted of [0n, 250n, 900n, 1_200n]) {
      for (const delta of [-200n, -75n, 0n, 75n, 800n]) {
        for (const deposit of [0n, 50n, 400n, 5_000n]) {
          const change = onChainSyncChange(
            caps,
            accounted,
            accounted + delta + deposit
          );
          const excess = delta > 0n ? delta : 0n;
          const expected = change > excess ? change - excess : 0n;

          expect(
            attributedNativeSolDepositLamports({
              metadata: caps,
              traderNativeSolLamports: accounted,
              traderSyncDeltaLamports: delta,
              depositLamports: deposit,
            })
          ).toBe(expected);
        }
      }
    }
  });

  it("never credits more than the headroom a caller was offered", () => {
    const caps = metadata({
      maxPerTraderBalance: 800n,
      maxGlobalBalance: 1_000n,
      currGlobalBalance: 400n,
    });

    for (const accounted of [0n, 300n, 800n]) {
      for (const delta of [-90n, 0n, 120n]) {
        const headroom = nativeSolCollateralHeadroomLamports({
          metadata: caps,
          traderNativeSolLamports: accounted,
          traderSyncDeltaLamports: delta,
        });

        const attributed = attributedNativeSolDepositLamports({
          metadata: caps,
          traderNativeSolLamports: accounted,
          traderSyncDeltaLamports: delta,
          depositLamports: headroom,
        });

        // With a deficit, the refilled portion is not "credited"; everything
        // else of a headroom-sized deposit is.
        expect(attributed).toBe(headroom - (delta < 0n ? -delta : 0n));
      }
    }
  });
});
