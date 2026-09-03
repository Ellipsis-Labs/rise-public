import type {
  LimitOrderMarginInput,
  MarginPositionState,
  MarketMarginInputs,
  SpotCollateralMarginInput,
  SubaccountMarginInputs,
  TraderMarginInputs,
} from "./types";

export interface MarginSnapshotMarketLimitOrderRow {
  orderSequenceNumber: string;
  side: "bid" | "ask";
  priceTicks: string;
  sizeRemainingLots: string;
  initialSizeLots: string;
  reduceOnly: boolean;
  isStopLoss?: boolean;
  isStopLossDirection?: boolean;
  status?: string;
}

export interface MarginSnapshotLimitOrderEvent {
  symbol: string;
  orders?: MarginSnapshotMarketLimitOrderRow[];
}

export interface MarginSnapshotPosition {
  symbol: string;
  basePositionLots: string;
  virtualQuotePositionLots: string;
  entryPriceTicks: string;
  unsettledFundingQuoteLots: string;
  accumulatedFundingQuoteLots: string;
}

/** Raw spot collateral balance as carried by the traderState stream. */
export interface MarginSnapshotSpotCollateral {
  assetIndex: number;
  symbol: string;
  /** Balance in the asset's native units (lamports for SOL). */
  balance: string;
}

export interface MarginSnapshotSubaccount {
  subaccountIndex: number;
  collateral?: string | null;
  spotCollaterals?: MarginSnapshotSpotCollateral[];
  positions?: MarginSnapshotPosition[];
  orders?: MarginSnapshotLimitOrderEvent[];
}

/**
 * Per-asset valuation context for spot collateral balances: native decimals,
 * pricing market, and the margin discount curve from `/v1/collateral/assets`.
 * A snapshot balance whose asset has no entry here is left unvalued (the
 * pre-spot behavior).
 */
export interface SpotCollateralAssetParams {
  assetIndex: number;
  decimals: number;
  pricingMarketSymbol?: string;
  indexPriceTicks?: string;
  maxGlobalBalance: string;
  minMarginDiscountBps: number;
  maxMarginDiscountBps: number;
}

export interface MarginSnapshotOptions {
  spotCollateralParamsByIndex?: Record<number, SpotCollateralAssetParams>;
}

export interface MarginTraderSnapshotMessage {
  authority: string;
  traderPdaIndex: number;
  messageType: "snapshot";
  subaccounts?: MarginSnapshotSubaccount[];
}

export interface MarginTraderStateSnapshotMessage {
  authority: string;
  traderPdaIndex: number;
  messageType: "snapshot" | "delta";
  subaccounts?: MarginSnapshotSubaccount[];
}

const isActiveOrder = (order: LimitOrderMarginInput): boolean =>
  order.status ? order.status === "active" : true;

const toLimitOrderMarginInput = (
  order: MarginSnapshotMarketLimitOrderRow
): LimitOrderMarginInput => ({
  orderSequenceNumber: order.orderSequenceNumber,
  side: order.side,
  priceTicks: order.priceTicks,
  sizeRemainingLots: order.sizeRemainingLots,
  initialSizeLots: order.initialSizeLots,
  reduceOnly: order.reduceOnly,
  isStopLoss: order.isStopLoss,
  isStopLossDirection: order.isStopLossDirection,
  status: order.status,
});

export { buildLimitOrderMarginStateFromOrders } from "./inputs";

export const buildMarginPositionStateFromSnapshot = (
  position: MarginSnapshotPosition
): MarginPositionState => ({
  basePositionLots: position.basePositionLots,
  virtualQuotePositionLots: position.virtualQuotePositionLots,
  entryPriceTicks: position.entryPriceTicks,
  unsettledFundingQuoteLots: position.unsettledFundingQuoteLots,
  accumulatedFundingQuoteLots: position.accumulatedFundingQuoteLots,
});

export const buildMarketMarginInputsFromSnapshot = (
  symbol: string,
  position: MarginSnapshotPosition | undefined,
  orders: MarginSnapshotMarketLimitOrderRow[]
): MarketMarginInputs => {
  const limitOrders = orders.map(toLimitOrderMarginInput);
  const activeOrders = limitOrders.filter(isActiveOrder);

  return {
    symbol,
    position: position
      ? buildMarginPositionStateFromSnapshot(position)
      : undefined,
    limitOrders: activeOrders.length > 0 ? activeOrders : undefined,
  };
};

export const buildSubaccountMarginInputsFromSnapshot = (
  subaccount: MarginSnapshotSubaccount,
  options?: MarginSnapshotOptions
): SubaccountMarginInputs => {
  const positionsBySymbol = new Map<string, MarginSnapshotPosition>();
  for (const position of subaccount.positions ?? []) {
    positionsBySymbol.set(position.symbol, position);
  }

  const ordersBySymbol = new Map<string, MarginSnapshotMarketLimitOrderRow[]>();
  for (const orderEvent of subaccount.orders ?? []) {
    ordersBySymbol.set(orderEvent.symbol, orderEvent.orders ?? []);
  }

  const symbols = new Set<string>([
    ...positionsBySymbol.keys(),
    ...ordersBySymbol.keys(),
  ]);

  const markets: MarketMarginInputs[] = [];
  for (const symbol of symbols) {
    markets.push(
      buildMarketMarginInputsFromSnapshot(
        symbol,
        positionsBySymbol.get(symbol),
        ordersBySymbol.get(symbol) ?? []
      )
    );
  }

  const spotCollateralParamsByIndex = options?.spotCollateralParamsByIndex;
  const spotCollaterals: SpotCollateralMarginInput[] = [];
  if (spotCollateralParamsByIndex) {
    for (const spot of subaccount.spotCollaterals ?? []) {
      const assetParams = spotCollateralParamsByIndex[spot.assetIndex];
      if (!assetParams) {
        continue;
      }
      spotCollaterals.push({
        assetIndex: spot.assetIndex,
        symbol: spot.symbol,
        balance: spot.balance,
        decimals: assetParams.decimals,
        pricingMarketSymbol: assetParams.pricingMarketSymbol,
        indexPriceTicks: assetParams.indexPriceTicks,
        maxGlobalBalance: assetParams.maxGlobalBalance,
        minMarginDiscountBps: assetParams.minMarginDiscountBps,
        maxMarginDiscountBps: assetParams.maxMarginDiscountBps,
      });
    }
  }

  return {
    subaccountIndex: subaccount.subaccountIndex,
    collateralBalanceQuoteLots: subaccount.collateral ?? "0",
    markets,
    ...(spotCollaterals.length > 0 ? { spotCollaterals } : {}),
  };
};

export const buildTraderMarginInputsFromSnapshot = (
  message: MarginTraderSnapshotMessage | MarginTraderStateSnapshotMessage,
  options?: MarginSnapshotOptions
): TraderMarginInputs => {
  if (message.messageType !== "snapshot") {
    throw new Error("TraderState message must be a snapshot to build inputs");
  }

  const subaccounts = message.subaccounts ?? [];
  return {
    authority: message.authority,
    traderPdaIndex: message.traderPdaIndex,
    subaccounts: subaccounts.map((subaccount) =>
      buildSubaccountMarginInputsFromSnapshot(subaccount, options)
    ),
  };
};
