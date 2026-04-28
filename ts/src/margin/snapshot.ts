import { buildLimitOrderMarginStateFromOrders } from "./inputs";
import { toBigInt } from "./math";
import type {
  LimitOrderMarginInput,
  MarginPositionState,
  MarketMarginInputs,
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

export interface MarginSnapshotSubaccount {
  subaccountIndex: number;
  collateral?: string | null;
  positions?: MarginSnapshotPosition[];
  orders?: MarginSnapshotLimitOrderEvent[];
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

export { buildLimitOrderMarginStateFromOrders };

export const buildMarginPositionStateFromSnapshot = (
  position: MarginSnapshotPosition
): MarginPositionState => ({
  basePositionLots: position.basePositionLots,
  virtualQuotePositionLots: position.virtualQuotePositionLots,
  entryPriceTicks: position.entryPriceTicks,
  unsettledFundingQuoteLots: (-toBigInt(
    position.unsettledFundingQuoteLots
  )).toString(),
  accumulatedFundingQuoteLots: position.accumulatedFundingQuoteLots,
});

export const buildMarketMarginInputsFromSnapshot = (
  symbol: string,
  position: MarginSnapshotPosition | undefined,
  orders: MarginSnapshotMarketLimitOrderRow[]
): MarketMarginInputs => {
  const limitOrders = orders.map(toLimitOrderMarginInput);
  const activeOrders = limitOrders.filter(isActiveOrder);
  const limitOrderMargin =
    activeOrders.length > 0
      ? buildLimitOrderMarginStateFromOrders(activeOrders)
      : undefined;

  return {
    symbol,
    position: position
      ? buildMarginPositionStateFromSnapshot(position)
      : undefined,
    limitOrderMargin,
    limitOrders: activeOrders.length > 0 ? activeOrders : undefined,
  };
};

export const buildSubaccountMarginInputsFromSnapshot = (
  subaccount: MarginSnapshotSubaccount
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

  return {
    subaccountIndex: subaccount.subaccountIndex,
    collateralBalanceQuoteLots: subaccount.collateral ?? "0",
    markets,
  };
};

export const buildTraderMarginInputsFromSnapshot = (
  message: MarginTraderSnapshotMessage | MarginTraderStateSnapshotMessage
): TraderMarginInputs => {
  if (message.messageType !== "snapshot") {
    throw new Error("TraderState message must be a snapshot to build inputs");
  }

  const subaccounts = message.subaccounts ?? [];
  return {
    authority: message.authority,
    traderPdaIndex: message.traderPdaIndex,
    subaccounts: subaccounts.map(buildSubaccountMarginInputsFromSnapshot),
  };
};
