import { describe, expect, it } from "vitest";
import { getLockedDepositTotals } from "@/trader-state";
import {
  buildSubaccountMarginInputsFromSnapshot,
  createMarginCalculator,
} from "@/margin";
import { TraderStateServerMessageSchema } from "@/ws/adapters/trader-state/wire";

const access = { immediate: true, viaColdActivation: false };
const capabilities = {
  flags: 1,
  state: "active",
  capabilities: {
    placeLimitOrder: access,
    placeMarketOrder: access,
    riskIncreasingTrade: access,
    riskReducingTrade: access,
    depositCollateral: access,
    withdrawCollateral: access,
  },
};
const lockedDeposit = { lockedQuoteLots: "100", consumedQuoteLots: "20" };

describe("locked deposits", () => {
  it("aggregates three isolated children while retaining the parent's local reserve", () => {
    const parent = {
      subaccountIndex: 0,
      lockedDeposit: {
        lockedQuoteLots: "1000000",
        consumedQuoteLots: "200000",
      },
    };
    const subaccounts = [
      ...["150000", "250000", "100000"].map((consumedQuoteLots, index) => ({
        subaccountIndex: index + 1,
        lockedDeposit: { lockedQuoteLots: "800000", consumedQuoteLots },
      })),
      parent,
    ];
    expect(getLockedDepositTotals(subaccounts)).toEqual({
      totalQuoteLots: 1000000n,
      consumedQuoteLots: 700000n,
      remainingQuoteLots: 300000n,
    });
    expect(getLockedDepositTotals([parent]).remainingQuoteLots).toBe(800000n);
  });

  it("counts only the parent grant, sums consumption and clamps the remainder", () => {
    const subaccounts = [
      { subaccountIndex: 0, lockedDeposit },
      {
        subaccountIndex: 1,
        lockedDeposit: { lockedQuoteLots: "80", consumedQuoteLots: "30" },
      },
    ];
    expect(getLockedDepositTotals(subaccounts)).toEqual({
      totalQuoteLots: 100n,
      consumedQuoteLots: 50n,
      remainingQuoteLots: 50n,
    });
    subaccounts.push({
      subaccountIndex: 2,
      lockedDeposit: { lockedQuoteLots: "80", consumedQuoteLots: "80" },
    });
    expect(getLockedDepositTotals(subaccounts).remainingQuoteLots).toBe(0n);
    expect(getLockedDepositTotals(subaccounts.slice(1)).totalQuoteLots).toBe(
      0n
    );
  });

  it.each(["snapshot", "delta"])(
    "preserves large integer amounts and explicit clearing in %s",
    (messageType) => {
      const row = {
        subaccountIndex: 0,
        sequence: 1,
        collateral: "1000",
        lockedDeposit: {
          lockedQuoteLots: "18446744073709551615",
          consumedQuoteLots: "9007199254740993",
        },
      };
      const message = {
        channel: "traderState",
        authority: "wallet",
        traderPdaIndex: 0,
        slot: 1,
        messageType,
        version: 1,
        capabilities,
        makerFeeOverrideMultiplier: 1,
        takerFeeOverrideMultiplier: 1,
        ...(messageType === "snapshot"
          ? { subaccounts: [row] }
          : { deltas: [row] }),
      };
      const parsed = TraderStateServerMessageSchema.parse(message);
      expect((parsed.subaccounts ?? parsed.deltas)?.[0]?.lockedDeposit).toEqual(
        row.lockedDeposit
      );
      row.lockedDeposit = { lockedQuoteLots: "0", consumedQuoteLots: "0" };
      const cleared = TraderStateServerMessageSchema.parse(message);
      expect(
        (cleared.subaccounts ?? cleared.deltas)?.[0]?.lockedDeposit
      ).toEqual(row.lockedDeposit);
      const absent: Record<string, unknown> = { ...row, lockedDeposit: null };
      const parsedNull = TraderStateServerMessageSchema.parse({
        ...message,
        ...(messageType === "snapshot"
          ? { subaccounts: [absent] }
          : { deltas: [absent] }),
      });
      expect(
        (parsedNull.subaccounts ?? parsedNull.deltas)?.[0]?.lockedDeposit
      ).toBeNull();
    }
  );

  it.each([
    // cash, lock, consumed, PnL, PnL factor, funding, pre-lock bound, payout
    [100, 50, 0, 50, 10000, 0, 140, 90],
    [100, 50, 0, 50, 5000, 0, 115, 65],
    [100, 50, 0, -60, 10000, 0, 30, 0],
    [50, 50, 0, 50, 10000, 0, 90, 40],
    [40, 50, 0, 50, 10000, 0, 80, 30],
    [-10, 50, 0, 100, 10000, 0, 80, 30],
    [100, 0, 0, 50, 10000, 0, 140, 140],
    [-10, 0, 0, 100, 10000, 0, 80, 80],
    [100, 50, 20, 50, 10000, 0, 140, 110],
    [100, 50, 50, 50, 10000, 0, 140, 140],
    [100, 50, 0, 0, 10000, 50, 140, 90],
    [100, 50, 0, 0, 10000, -60, 30, 0],
    [1000, 0, 0, -600, 10000, 0, 390, 390],
    [1000, 100, 0, -600, 10000, 0, 390, 290],
    [100, 150, 0, 0, 10000, 0, 90, 0],
  ])(
    "subtracts the lock after margin, PnL and funding: %s/%s/%s/%s/%s/%s",
    (
      cash,
      locked,
      consumed,
      pnl,
      pnlFactor,
      funding,
      transferable,
      withdrawable
    ) => {
      const calculator = createMarginCalculator([
        {
          symbol: "SOL",
          assetId: 1,
          markPriceTicks: "200",
          tickSize: "1",
          baseLotDecimals: 0,
          leverageTiers: [
            {
              upperBoundSize: "1000",
              maxLeverage: "20",
              limitOrderRiskFactorBps: "10000",
            },
          ],
          riskFactors: {
            maintenanceMarginFactorBps: "5000",
            backstopMarginFactorBps: "10000",
            highRiskMarginFactorBps: "10000",
          },
          cancelOrderRiskFactorBps: "5000",
          upnlRiskFactor: "10000",
          upnlRiskFactorForWithdrawals: pnlFactor.toString(),
          isolatedOnly: false,
        },
      ]);
      const inputs = buildSubaccountMarginInputsFromSnapshot({
        subaccountIndex: 0,
        collateral: cash.toString(),
        lockedDeposit: {
          lockedQuoteLots: locked.toString(),
          consumedQuoteLots: consumed.toString(),
        },
        positions: [
          {
            symbol: "SOL",
            basePositionLots: "1",
            virtualQuotePositionLots: (pnl - 200).toString(),
            entryPriceTicks: (200 - pnl).toString(),
            unsettledFundingQuoteLots: funding.toString(),
            accumulatedFundingQuoteLots: "0",
          },
        ],
      });
      const { margin } = calculator.computeSubaccountMarginFromInputs(inputs);
      expect(margin.withdrawableQuoteCollateralPreLockQuoteLots).toBe(
        transferable.toString()
      );
      expect(margin.withdrawableQuoteCollateralQuoteLots).toBe(
        withdrawable.toString()
      );
      const simulation = calculator.simulateMargin({
        subaccount: inputs,
        actions: [],
      });
      expect(simulation.after.margin.withdrawableQuoteCollateralQuoteLots).toBe(
        withdrawable.toString()
      );
    }
  );

  it("subtracts the account reserve from withdrawals without positions", () => {
    const calculator = createMarginCalculator([]);
    for (const [collateral, expectedTransferable, expectedWithdrawable] of [
      ["200", "200", "120"],
      ["50", "50", "0"],
      ["-10", "0", "0"],
    ]) {
      const inputs = buildSubaccountMarginInputsFromSnapshot({
        subaccountIndex: 0,
        collateral,
        lockedDeposit,
      });
      const result = calculator.computeSubaccountMarginFromInputs(inputs);
      expect(result.margin.withdrawableQuoteCollateralPreLockQuoteLots).toBe(
        expectedTransferable
      );
      expect(result.margin.withdrawableQuoteCollateralQuoteLots).toBe(
        expectedWithdrawable
      );
      expect(result.margin.effectiveCollateralQuoteLots).toBe(collateral);
    }
  });
});
