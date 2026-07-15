import { describe, expect, it } from "vitest";
import {
  buildNormalizedMarketParamsBySymbol,
  buildSubaccountMarginInputsFromSnapshot,
  computeDraftOrderMarginRequirementFromInputs,
  computeDraftOrderMarginRequirementFromSnapshot,
  computeMaxDraftOrderSizeForAvailableMarginFromInputs,
  computeMaxDraftOrderSizeForAvailableMarginFromSnapshot,
  type DraftOrderMarginInput,
  type MarginCalculationOptions,
  type MarginMarketsInput,
  type MarginSnapshotMarketLimitOrderRow,
  type MarginSnapshotSubaccount,
  type MarketParams,
  type SubaccountMarginInputs,
} from "@/margin";

const marketParams: MarketParams = {
  symbol: "SOL-PERP",
  assetId: 1,
  markPriceTicks: "100",
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
    backstopMarginFactorBps: "2000",
    highRiskMarginFactorBps: "1000",
  },
  cancelOrderRiskFactorBps: "7000",
  upnlRiskFactor: "10000",
  upnlRiskFactorForWithdrawals: "10000",
  isolatedOnly: false,
};

const tierBoundaryMarketParams: MarketParams = {
  ...marketParams,
  leverageTiers: [
    {
      upperBoundSize: "10",
      maxLeverage: "20",
      limitOrderRiskFactorBps: "10000",
    },
    {
      upperBoundSize: "20",
      maxLeverage: "10",
      limitOrderRiskFactorBps: "10000",
    },
  ],
};

const leverageOptions: MarginCalculationOptions = {
  orderLeverageLimitsBySymbol: { "SOL-PERP": 5 },
};

const emptySnapshot = (): MarginSnapshotSubaccount => ({
  subaccountIndex: 0,
  collateral: "1000",
});

const positionSnapshot = (): MarginSnapshotSubaccount => ({
  ...emptySnapshot(),
  positions: [
    {
      symbol: "SOL-PERP",
      basePositionLots: "10",
      virtualQuotePositionLots: "-1000",
      entryPriceTicks: "100",
      unsettledFundingQuoteLots: "7",
      accumulatedFundingQuoteLots: "11",
    },
  ],
});

const limitOrder = (
  overrides: Partial<MarginSnapshotMarketLimitOrderRow> = {}
): MarginSnapshotMarketLimitOrderRow => ({
  orderSequenceNumber: "1",
  side: "bid",
  priceTicks: "100",
  sizeRemainingLots: "2",
  initialSizeLots: "2",
  reduceOnly: false,
  status: "active",
  ...overrides,
});

const snapshotWithLimitOrder = (
  order: MarginSnapshotMarketLimitOrderRow = limitOrder()
): MarginSnapshotSubaccount => ({
  ...emptySnapshot(),
  orders: [{ symbol: "SOL-PERP", orders: [order] }],
});

const aggregateOnlyInputs = (): SubaccountMarginInputs => ({
  subaccountIndex: 0,
  collateralBalanceQuoteLots: "1000",
  markets: [
    {
      symbol: "SOL-PERP",
      limitOrderMargin: {
        numAskOrders: 0,
        numBidOrders: 1,
        lowestAsk: "0",
        highestBid: "0",
        totalNonReduceOnlyAskBaseLots: "0",
        totalReduceOnlyAskBaseLots: "0",
        totalNonReduceOnlyBidBaseLots: "10",
        totalReduceOnlyBidBaseLots: "0",
      },
    },
  ],
});

const draftOrder = (
  overrides: Partial<DraftOrderMarginInput> = {}
): DraftOrderMarginInput => ({
  symbol: "SOL-PERP",
  side: "bid",
  orderType: "limit",
  priceTicks: "100",
  sizeBaseLots: "10",
  ...overrides,
});

type DraftOrderWithoutSize = Omit<DraftOrderMarginInput, "sizeBaseLots">;

const draftOrderWithoutSize = (
  overrides: Partial<DraftOrderWithoutSize> = {}
): DraftOrderWithoutSize => ({
  symbol: "SOL-PERP",
  side: "bid",
  orderType: "limit",
  priceTicks: "100",
  ...overrides,
});

const requirementFromSnapshot = (
  params: {
    subaccount?: MarginSnapshotSubaccount;
    markets?: MarginMarketsInput;
    draftOrder?: DraftOrderMarginInput;
    options?: MarginCalculationOptions;
  } = {}
) =>
  computeDraftOrderMarginRequirementFromSnapshot({
    subaccount: params.subaccount ?? emptySnapshot(),
    markets: params.markets ?? [marketParams],
    draftOrder: params.draftOrder ?? draftOrder(),
    options: params.options,
  });

const maxSizeFromSnapshot = (params: {
  subaccount?: MarginSnapshotSubaccount;
  markets?: MarginMarketsInput;
  draftOrder?: DraftOrderWithoutSize;
  maxSizeBaseLots?: string;
  availableMarginQuoteLots: string;
  options?: MarginCalculationOptions;
}) =>
  computeMaxDraftOrderSizeForAvailableMarginFromSnapshot({
    subaccount: params.subaccount ?? emptySnapshot(),
    markets: params.markets ?? [marketParams],
    draftOrder: params.draftOrder ?? draftOrderWithoutSize(),
    maxSizeBaseLots: params.maxSizeBaseLots ?? "10",
    availableMarginQuoteLots: params.availableMarginQuoteLots,
    options: params.options,
  });

describe("draft order margin helpers", () => {
  it.each([
    { orderType: "limit" as const, options: undefined },
    { orderType: "limit" as const, options: leverageOptions },
    { orderType: "market" as const, options: undefined },
    { orderType: "market" as const, options: leverageOptions },
  ])(
    "keeps snapshot and input margin results aligned for $orderType drafts",
    ({ orderType, options }) => {
      const snapshot = snapshotWithLimitOrder();
      const draft = draftOrder({ orderType });

      expect(
        computeDraftOrderMarginRequirementFromInputs({
          subaccount: buildSubaccountMarginInputsFromSnapshot(snapshot),
          markets: buildNormalizedMarketParamsBySymbol([marketParams]),
          draftOrder: draft,
          options,
        })
      ).toEqual(
        requirementFromSnapshot({
          subaccount: snapshot,
          draftOrder: draft,
          options,
        })
      );
    }
  );

  it.each(["limit", "market"] as const)(
    "returns leverage-adjusted margin for %s drafts",
    (orderType) => {
      expect(
        requirementFromSnapshot({
          draftOrder: draftOrder({ orderType }),
          options: leverageOptions,
        })
      ).toEqual({
        marginRequirementQuoteLots: "200",
        protocolMarginRequirementQuoteLots: "50",
        orderLeverageAdjustedMarginRequirementQuoteLots: "200",
      });
    }
  );

  it("uses aggregate margin deltas across leverage tiers", () => {
    const snapshot = snapshotWithLimitOrder(
      limitOrder({ sizeRemainingLots: "10", initialSizeLots: "10" })
    );

    expect(
      requirementFromSnapshot({
        subaccount: snapshot,
        markets: [tierBoundaryMarketParams],
      })
    ).toEqual({
      marginRequirementQuoteLots: "150",
      protocolMarginRequirementQuoteLots: "150",
    });

    expect(
      maxSizeFromSnapshot({
        subaccount: snapshot,
        markets: [tierBoundaryMarketParams],
        availableMarginQuoteLots: "50",
      })
    ).toBe(5n);
  });

  it("preserves adverse fill loss for reduce-only limit drafts", () => {
    expect(
      requirementFromSnapshot({
        subaccount: positionSnapshot(),
        draftOrder: draftOrder({
          side: "ask",
          priceTicks: "80",
          reduceOnly: true,
        }),
      })
    ).toEqual({
      marginRequirementQuoteLots: "200",
      protocolMarginRequirementQuoteLots: "200",
    });
  });

  it.each(["0", "-1"])(
    "rejects non-positive price ticks (%s)",
    (priceTicks) => {
      expect(() =>
        requirementFromSnapshot({ draftOrder: draftOrder({ priceTicks }) })
      ).toThrow("Draft order priceTicks must be positive");
    }
  );

  it.each(["limit", "market"] as const)(
    "rejects %s projections from aggregate-only order state",
    (orderType) => {
      expect(() =>
        computeDraftOrderMarginRequirementFromInputs({
          subaccount: aggregateOnlyInputs(),
          markets: [marketParams],
          draftOrder: draftOrder({ orderType }),
        })
      ).toThrow(/only aggregate limitOrderMargin is provided/);
    }
  );

  it("rebuilds order aggregates after a market draft changes the position", () => {
    const subaccount =
      buildSubaccountMarginInputsFromSnapshot(positionSnapshot());
    const market = subaccount.markets[0]!;
    market.limitOrders = [
      limitOrder({
        side: "ask",
        priceTicks: "80",
        sizeRemainingLots: "10",
        initialSizeLots: "10",
        reduceOnly: true,
      }),
    ];
    market.limitOrderMargin = {
      numAskOrders: 1,
      numBidOrders: 0,
      lowestAsk: "80",
      highestBid: "0",
      totalNonReduceOnlyAskBaseLots: "0",
      totalReduceOnlyAskBaseLots: "10",
      totalNonReduceOnlyBidBaseLots: "0",
      totalReduceOnlyBidBaseLots: "0",
    };

    expect(
      computeDraftOrderMarginRequirementFromInputs({
        subaccount,
        markets: [marketParams],
        draftOrder: draftOrder({
          side: "ask",
          orderType: "market",
          sizeBaseLots: "30",
        }),
      })
    ).toEqual({
      marginRequirementQuoteLots: "0",
      protocolMarginRequirementQuoteLots: "0",
    });
  });

  it("caps reduce-only market drafts and max size to the open position", () => {
    expect(
      requirementFromSnapshot({
        subaccount: positionSnapshot(),
        draftOrder: draftOrder({
          side: "ask",
          orderType: "market",
          sizeBaseLots: "100",
          reduceOnly: true,
        }),
      })
    ).toEqual({
      marginRequirementQuoteLots: "0",
      protocolMarginRequirementQuoteLots: "0",
    });

    expect(
      maxSizeFromSnapshot({
        subaccount: positionSnapshot(),
        draftOrder: draftOrderWithoutSize({
          side: "ask",
          orderType: "market",
          reduceOnly: true,
        }),
        maxSizeBaseLots: "100",
        availableMarginQuoteLots: "-1",
      })
    ).toBe(10n);
  });

  it("finds the largest size that fits available adjusted margin", () => {
    const draft = draftOrderWithoutSize({ orderType: "market" });
    const params = {
      subaccount: emptySnapshot(),
      markets: [marketParams],
      draftOrder: draft,
      maxSizeBaseLots: "10",
      availableMarginQuoteLots: "120",
      options: leverageOptions,
    };

    expect(computeMaxDraftOrderSizeForAvailableMarginFromSnapshot(params)).toBe(
      6n
    );
    expect(
      computeMaxDraftOrderSizeForAvailableMarginFromInputs({
        ...params,
        subaccount: buildSubaccountMarginInputsFromSnapshot(params.subaccount),
        markets: buildNormalizedMarketParamsBySymbol(params.markets),
      })
    ).toBe(6n);
  });

  it("returns the upper bound when it fits", () => {
    expect(
      maxSizeFromSnapshot({
        availableMarginQuoteLots: "200",
        options: leverageOptions,
      })
    ).toBe(10n);
  });

  it("does not mutate caller-owned margin inputs", () => {
    const subaccount = buildSubaccountMarginInputsFromSnapshot(
      snapshotWithLimitOrder()
    );
    const before = structuredClone(subaccount);

    computeDraftOrderMarginRequirementFromInputs({
      subaccount,
      markets: [marketParams],
      draftOrder: draftOrder({
        side: "ask",
        sizeBaseLots: "3",
      }),
    });
    computeMaxDraftOrderSizeForAvailableMarginFromInputs({
      subaccount,
      markets: [marketParams],
      draftOrder: draftOrderWithoutSize({
        orderType: "market",
      }),
      maxSizeBaseLots: "10",
      availableMarginQuoteLots: "100",
    });

    expect(subaccount).toEqual(before);
  });

  it("returns zero for non-positive sizes and negative available margin", () => {
    expect(
      requirementFromSnapshot({
        draftOrder: draftOrder({
          orderType: "market",
          sizeBaseLots: "0",
        }),
      })
    ).toEqual({
      marginRequirementQuoteLots: "0",
      protocolMarginRequirementQuoteLots: "0",
    });

    expect(
      maxSizeFromSnapshot({
        draftOrder: draftOrderWithoutSize({
          orderType: "market",
        }),
        availableMarginQuoteLots: "-1",
      })
    ).toBe(0n);
  });
});
