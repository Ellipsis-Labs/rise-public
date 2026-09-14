import { createStore } from "zustand/vanilla";
import { createResourceRegistry } from "@/internal/_keyedResourceManager";
import {
  createLifecycle,
  normalizeBackgroundError,
  runRetryLoop,
} from "@/internal/_resourceRuntime";
import { buildSubaccountMarginInputsFromSnapshot } from "@/margin/snapshot";
import type {
  TraderStateMarketLimitOrderRow,
  TraderStateOrderHistoryDelta,
  TraderStatePositionDelta,
  TraderStatePositionSnapshot,
  TraderStateServerMessage,
  TraderStateSnapshotResponse,
  TraderStateSplineDelta,
  TraderStateSplineSnapshot,
  TraderStateSubaccountDelta,
  TraderStateTradeHistoryDelta,
  TraderStateTriggerDelta,
  TraderStateTriggerSnapshot,
} from "@/api/traders/traderState";
import type { TraderMarginInputs } from "@/margin/types";
import type {
  PhoenixTraderStateManager,
  PhoenixTraderStateManagerConfig,
  PhoenixTraderStateResource,
  PhoenixTraderStateResourceParams,
  PhoenixTraderStateStoreState,
  TraderStateHealth,
  TraderStateMessageMeta,
  TraderStateOrderBucket,
  TraderStateSnapshotMetadata,
  TraderStateSnapshotState,
  TraderStateSplineBucket,
  TraderStateStatus,
  TraderStateSubaccountSnapshotState,
  TraderStateSubaccountState,
} from "./types";

const DEFAULT_RESYNC_BACKOFF_MS = 1_000;
const HISTORY_LIMIT = 1_000;

const EMPTY_MESSAGE_META: TraderStateMessageMeta = {
  lastMessageAtMs: null,
  lastMessageSlot: null,
  lastMessageType: null,
  lastSnapshotSlot: null,
  lastSnapshotAtMs: null,
  slotDelta: null,
  messageDeltaMs: null,
};

const EMPTY_SUBACCOUNT_INDICES: readonly number[] = [];
const EMPTY_SUBACCOUNTS_BY_INDEX: Readonly<
  Record<number, TraderStateSubaccountState>
> = {};
const EMPTY_MARGIN_INPUTS: TraderMarginInputs | null = null;
const EMPTY_ORDERS: readonly TraderStateMarketLimitOrderRow[] = [];

const normalizeTraderPdaIndex = (traderPdaIndex: number | undefined) =>
  Number.isFinite(traderPdaIndex)
    ? Math.max(0, Math.trunc(traderPdaIndex ?? 0))
    : 0;

const getTraderStateStoreKey = (
  authority: string,
  traderPdaIndex: number | undefined
) => `trader:${authority.trim()}:${normalizeTraderPdaIndex(traderPdaIndex)}`;

const normalizeSymbol = (symbol: string) => symbol.toUpperCase();

const sameNumberArray = (left: readonly number[], right: readonly number[]) =>
  left.length === right.length &&
  left.every((value, index) => value === right[index]);

const sameStringArray = (left: readonly string[], right: readonly string[]) =>
  left.length === right.length &&
  left.every((value, index) => value === right[index]);

const sortedSymbols = (symbols: Iterable<string>): readonly string[] =>
  Array.from(symbols).sort((left, right) => left.localeCompare(right));

const sortedSubaccountIndices = (
  subaccountIndices: Iterable<number>
): readonly number[] =>
  Array.from(subaccountIndices).sort((left, right) => left - right);

const sortedOrderSequenceNumbers = (
  orderSequenceNumbers: Iterable<string>
): readonly string[] =>
  Array.from(orderSequenceNumbers).sort((left, right) =>
    left.localeCompare(right)
  );

const SKIP_SYMBOL_DELTA = Symbol("skipSymbolDelta");

const reuseSortedStringKeys = (
  current: readonly string[],
  next: Iterable<string>,
  sort: (values: Iterable<string>) => readonly string[] = sortedSymbols
): readonly string[] => {
  const sorted = sort(next);
  return sameStringArray(current, sorted) ? current : sorted;
};

const applySymbolCollectionDeltas = <TDelta, TValue>(params: {
  currentBySymbol: Readonly<Record<string, TValue>>;
  currentSymbols: readonly string[];
  deltas: readonly TDelta[];
  getSymbol: (delta: TDelta) => string;
  applyDelta: (
    delta: TDelta,
    current: TValue | undefined
  ) => TValue | null | typeof SKIP_SYMBOL_DELTA;
}): {
  bySymbol: Readonly<Record<string, TValue>>;
  symbols: readonly string[];
} => {
  if (params.deltas.length === 0) {
    return {
      bySymbol: params.currentBySymbol,
      symbols: params.currentSymbols,
    };
  }

  const bySymbol: Record<string, TValue> = {
    ...params.currentBySymbol,
  };

  for (const delta of params.deltas) {
    const symbol = normalizeSymbol(params.getSymbol(delta));
    const next = params.applyDelta(delta, bySymbol[symbol]);
    if (next === SKIP_SYMBOL_DELTA) {
      continue;
    }
    if (next === null) {
      delete bySymbol[symbol];
      continue;
    }
    bySymbol[symbol] = next;
  }

  return {
    bySymbol,
    symbols: reuseSortedStringKeys(
      params.currentSymbols,
      Object.keys(bySymbol)
    ),
  };
};

const buildSplineKey = (
  symbol: string,
  midPriceTicks: string | undefined
): string => `${normalizeSymbol(symbol)}:${midPriceTicks ?? ""}`;

const mergeHistory = <T>(
  existing: readonly T[],
  incoming: readonly T[],
  makeId: (row: T) => string
): readonly T[] => {
  if (incoming.length === 0) {
    return existing;
  }

  const seen = new Set(existing.map(makeId));
  const merged = [...existing];

  for (const row of incoming) {
    const id = makeId(row);
    if (seen.has(id)) continue;
    seen.add(id);
    merged.push(row);
  }

  if (merged.length <= HISTORY_LIMIT) {
    return merged;
  }

  return merged.slice(-HISTORY_LIMIT);
};

const makeTradeHistoryId = (row: TraderStateTradeHistoryDelta) =>
  row.fillId?.trim()
    ? row.fillId
    : `${row.slot}-${row.slotIndex}-${row.instructionIndex}-${row.eventIndex}-${row.market}`;

const makeOrderHistoryId = (row: TraderStateOrderHistoryDelta) =>
  `${row.slot}-${row.slotIndex}-${row.instructionIndex}-${row.eventIndex}-${row.market}`;

const cloneOrder = (
  order: TraderStateMarketLimitOrderRow
): TraderStateMarketLimitOrderRow => ({
  ...order,
});

const clonePosition = (
  position: TraderStatePositionSnapshot
): TraderStatePositionSnapshot => ({
  ...position,
  takeProfitTriggers: [...position.takeProfitTriggers],
  stopLossTriggers: [...position.stopLossTriggers],
  conditionalTakeProfitTriggers: [...position.conditionalTakeProfitTriggers],
  conditionalStopLossTriggers: [...position.conditionalStopLossTriggers],
});

const cloneSpline = (
  spline: TraderStateSplineSnapshot
): TraderStateSplineSnapshot => ({
  ...spline,
  bidRegions: spline.bidRegions.map((region) => ({ ...region })),
  askRegions: spline.askRegions.map((region) => ({ ...region })),
});

const cloneTriggers = (
  triggers: TraderStateTriggerSnapshot
): TraderStateTriggerSnapshot => ({
  ...triggers,
  takeProfitTriggers: [...triggers.takeProfitTriggers],
  stopLossTriggers: [...triggers.stopLossTriggers],
  conditionalTakeProfitTriggers: [...triggers.conditionalTakeProfitTriggers],
  conditionalStopLossTriggers: [...triggers.conditionalStopLossTriggers],
});

const buildOrderBucket = (
  symbol: string,
  orders: readonly TraderStateMarketLimitOrderRow[]
): TraderStateOrderBucket => {
  const ordersBySequence: Record<string, TraderStateMarketLimitOrderRow> = {};
  for (const order of orders) {
    ordersBySequence[order.orderSequenceNumber] = cloneOrder(order);
  }
  const orderSequenceNumbers = sortedOrderSequenceNumbers(
    Object.keys(ordersBySequence)
  );
  return {
    symbol,
    orderSequenceNumbers,
    ordersBySequence,
  };
};

const buildSplineBucket = (
  symbol: string,
  splines: readonly TraderStateSplineSnapshot[]
): TraderStateSplineBucket => {
  const splinesByKey: Record<string, TraderStateSplineSnapshot> = {};
  for (const spline of splines) {
    splinesByKey[buildSplineKey(symbol, spline.midPriceTicks)] =
      cloneSpline(spline);
  }
  const splineKeys = sortedSymbols(Object.keys(splinesByKey));
  return {
    symbol,
    splineKeys,
    splinesByKey,
  };
};

const buildSubaccountSnapshotFromState = (
  state: Omit<
    TraderStateSubaccountState,
    "snapshot" | "marginInputs" | "tradeHistory" | "orderHistory"
  > & {
    tradeHistory: readonly TraderStateTradeHistoryDelta[];
    orderHistory: readonly TraderStateOrderHistoryDelta[];
  }
): TraderStateSubaccountSnapshotState => {
  const positions = state.positionSymbols.map((symbol) =>
    clonePosition(state.positionsBySymbol[symbol])
  );
  const orders = state.orderSymbols.map((symbol) => {
    const bucket = state.ordersBySymbol[symbol];
    return {
      symbol,
      orders: bucket.orderSequenceNumbers.map((orderSequenceNumber) =>
        cloneOrder(bucket.ordersBySequence[orderSequenceNumber])
      ),
    };
  });
  const splines = state.splineSymbols.flatMap((symbol) => {
    const bucket = state.splinesBySymbol[symbol];
    return bucket.splineKeys.map((splineKey) =>
      cloneSpline(bucket.splinesByKey[splineKey])
    );
  });
  const triggers = state.triggerSymbols.map((symbol) =>
    cloneTriggers(state.triggersBySymbol[symbol])
  );

  return {
    subaccountIndex: state.subaccountIndex,
    sequence: state.sequence,
    collateral: state.collateral,
    spotCollaterals: [...(state.spotCollaterals ?? [])],
    capabilities: state.capabilities,
    cooldownStatus: state.cooldownStatus,
    positions,
    orders,
    splines,
    triggers,
    tradeHistory: [...state.tradeHistory],
    orderHistory: [...state.orderHistory],
  };
};

const buildSubaccountStateFromSnapshot = (
  snapshot: TraderStateSubaccountSnapshotState
): TraderStateSubaccountState => {
  const positionsBySymbol: Record<string, TraderStatePositionSnapshot> = {};
  for (const position of snapshot.positions) {
    positionsBySymbol[normalizeSymbol(position.symbol)] =
      clonePosition(position);
  }
  const positionSymbols = sortedSymbols(Object.keys(positionsBySymbol));

  const ordersBySymbol: Record<string, TraderStateOrderBucket> = {};
  for (const orderEvent of snapshot.orders) {
    const symbol = normalizeSymbol(orderEvent.symbol);
    ordersBySymbol[symbol] = buildOrderBucket(symbol, orderEvent.orders ?? []);
  }
  const orderSymbols = sortedSymbols(Object.keys(ordersBySymbol));

  const splinesBySymbol: Record<string, TraderStateSplineBucket> = {};
  for (const spline of snapshot.splines) {
    const symbol = normalizeSymbol(spline.symbol);
    const existing = splinesBySymbol[symbol];
    const nextSplines = existing
      ? existing.splineKeys.map((key) => existing.splinesByKey[key])
      : [];
    nextSplines.push(spline);
    splinesBySymbol[symbol] = buildSplineBucket(symbol, nextSplines);
  }
  const splineSymbols = sortedSymbols(Object.keys(splinesBySymbol));

  const triggersBySymbol: Record<string, TraderStateTriggerSnapshot> = {};
  for (const trigger of snapshot.triggers) {
    triggersBySymbol[normalizeSymbol(trigger.symbol)] = cloneTriggers(trigger);
  }
  const triggerSymbols = sortedSymbols(Object.keys(triggersBySymbol));

  const nextSnapshot = {
    ...snapshot,
    positions: snapshot.positions.map(clonePosition),
    orders: snapshot.orders.map((event) => ({
      symbol: event.symbol,
      orders: (event.orders ?? []).map(cloneOrder),
    })),
    splines: snapshot.splines.map(cloneSpline),
    triggers: snapshot.triggers.map(cloneTriggers),
    tradeHistory: [...snapshot.tradeHistory],
    orderHistory: [...snapshot.orderHistory],
  };

  return {
    subaccountIndex: snapshot.subaccountIndex,
    sequence: snapshot.sequence,
    collateral: snapshot.collateral,
    spotCollaterals: snapshot.spotCollaterals ?? [],
    capabilities: snapshot.capabilities,
    cooldownStatus: snapshot.cooldownStatus,
    positionSymbols,
    positionsBySymbol,
    orderSymbols,
    ordersBySymbol,
    splineSymbols,
    splinesBySymbol,
    triggerSymbols,
    triggersBySymbol,
    tradeHistory: nextSnapshot.tradeHistory,
    orderHistory: nextSnapshot.orderHistory,
    snapshot: nextSnapshot,
    marginInputs: buildSubaccountMarginInputsFromSnapshot(nextSnapshot),
  };
};

const buildInitialSubaccountState = (
  subaccountIndex: number
): TraderStateSubaccountState =>
  buildSubaccountStateFromSnapshot({
    subaccountIndex,
    sequence: 0,
    collateral: "0",
    spotCollaterals: [],
    positions: [],
    orders: [],
    splines: [],
    triggers: [],
    tradeHistory: [],
    orderHistory: [],
  });

const buildTraderMarginInputs = (
  authority: string,
  traderPdaIndex: number,
  subaccountIndices: readonly number[],
  subaccountsByIndex: Readonly<Record<number, TraderStateSubaccountState>>
): TraderMarginInputs => ({
  authority,
  traderPdaIndex,
  subaccounts: subaccountIndices.map(
    (subaccountIndex) => subaccountsByIndex[subaccountIndex].marginInputs
  ),
});

const buildSnapshotMetadata = (
  snapshot: TraderStateSnapshotState
): TraderStateSnapshotMetadata => ({
  authority: snapshot.authority,
  traderPdaIndex: snapshot.traderPdaIndex,
  slot: snapshot.slot,
  version: snapshot.version,
  capabilities: snapshot.capabilities,
  makerFeeOverrideMultiplier: snapshot.makerFeeOverrideMultiplier,
  takerFeeOverrideMultiplier: snapshot.takerFeeOverrideMultiplier,
});

const toServerMessage = (
  response: TraderStateSnapshotResponse
): TraderStateServerMessage => ({
  channel: "traderState",
  authority: response.authority,
  traderPdaIndex: response.traderPdaIndex,
  slot: BigInt(response.slot),
  messageType: "snapshot",
  version: response.snapshot.version,
  capabilities: response.snapshot.capabilities,
  makerFeeOverrideMultiplier: response.snapshot.makerFeeOverrideMultiplier,
  takerFeeOverrideMultiplier: response.snapshot.takerFeeOverrideMultiplier,
  subaccounts: response.snapshot.subaccounts,
});

const applyPositionDeltas = (
  current: TraderStateSubaccountState,
  deltas: readonly TraderStatePositionDelta[]
): {
  positionsBySymbol: Readonly<Record<string, TraderStatePositionSnapshot>>;
  positionSymbols: readonly string[];
} => {
  const { bySymbol, symbols } = applySymbolCollectionDeltas({
    currentBySymbol: current.positionsBySymbol,
    currentSymbols: current.positionSymbols,
    deltas,
    getSymbol: (delta) => delta.symbol,
    applyDelta: (delta) => {
      if (delta.change === "closed") {
        return null;
      }
      if (!delta.position) {
        return SKIP_SYMBOL_DELTA;
      }
      return clonePosition({
        symbol: delta.symbol,
        ...delta.position,
      });
    },
  });

  return {
    positionsBySymbol: bySymbol,
    positionSymbols: symbols,
  };
};

const applyOrderDeltas = (
  current: TraderStateSubaccountState,
  deltas: readonly {
    symbol: string;
    orders: TraderStateMarketLimitOrderRow[];
  }[]
): {
  ordersBySymbol: Readonly<Record<string, TraderStateOrderBucket>>;
  orderSymbols: readonly string[];
} => {
  const { bySymbol, symbols } = applySymbolCollectionDeltas({
    currentBySymbol: current.ordersBySymbol,
    currentSymbols: current.orderSymbols,
    deltas,
    getSymbol: (delta) => delta.symbol,
    applyDelta: (event, currentBucket) => {
      const symbol = normalizeSymbol(event.symbol);
      const ordersBySequence = {
        ...(currentBucket?.ordersBySequence ?? {}),
      };

      for (const order of event.orders ?? []) {
        if (order.change === "closed") {
          delete ordersBySequence[order.orderSequenceNumber];
        } else {
          ordersBySequence[order.orderSequenceNumber] = cloneOrder(order);
        }
      }

      if (Object.keys(ordersBySequence).length === 0) {
        return null;
      }

      return {
        symbol,
        orderSequenceNumbers: reuseSortedStringKeys(
          currentBucket?.orderSequenceNumbers ?? [],
          Object.keys(ordersBySequence),
          sortedOrderSequenceNumbers
        ),
        ordersBySequence,
      };
    },
  });

  return {
    ordersBySymbol: bySymbol,
    orderSymbols: symbols,
  };
};

const applySplineDeltas = (
  current: TraderStateSubaccountState,
  deltas: readonly TraderStateSplineDelta[]
): {
  splinesBySymbol: Readonly<Record<string, TraderStateSplineBucket>>;
  splineSymbols: readonly string[];
} => {
  const { bySymbol, symbols } = applySymbolCollectionDeltas({
    currentBySymbol: current.splinesBySymbol,
    currentSymbols: current.splineSymbols,
    deltas,
    getSymbol: (delta) => delta.symbol,
    applyDelta: (delta, currentBucket) => {
      const symbol = normalizeSymbol(delta.symbol);
      const splinesByKey = {
        ...(currentBucket?.splinesByKey ?? {}),
      };

      if (delta.change === "closed") {
        if (delta.spline?.midPriceTicks !== undefined) {
          delete splinesByKey[
            buildSplineKey(delta.symbol, delta.spline.midPriceTicks)
          ];
        } else {
          for (const key of Object.keys(splinesByKey)) {
            delete splinesByKey[key];
          }
        }
      } else if (delta.spline) {
        splinesByKey[buildSplineKey(delta.symbol, delta.spline.midPriceTicks)] =
          cloneSpline({
            symbol: delta.symbol,
            ...delta.spline,
          });
      } else {
        return SKIP_SYMBOL_DELTA;
      }

      if (Object.keys(splinesByKey).length === 0) {
        return null;
      }

      return {
        symbol,
        splineKeys: reuseSortedStringKeys(
          currentBucket?.splineKeys ?? [],
          Object.keys(splinesByKey)
        ),
        splinesByKey,
      };
    },
  });

  return {
    splinesBySymbol: bySymbol,
    splineSymbols: symbols,
  };
};

const applyTriggerDeltas = (
  current: TraderStateSubaccountState,
  deltas: readonly TraderStateTriggerDelta[]
): {
  triggersBySymbol: Readonly<Record<string, TraderStateTriggerSnapshot>>;
  triggerSymbols: readonly string[];
} => {
  const { bySymbol, symbols } = applySymbolCollectionDeltas({
    currentBySymbol: current.triggersBySymbol,
    currentSymbols: current.triggerSymbols,
    deltas,
    getSymbol: (delta) => delta.symbol,
    applyDelta: (delta) => {
      if (delta.change === "closed") {
        return null;
      }
      if (!delta.triggers) {
        return SKIP_SYMBOL_DELTA;
      }
      return cloneTriggers({
        symbol: delta.symbol,
        ...delta.triggers,
      });
    },
  });

  return {
    triggersBySymbol: bySymbol,
    triggerSymbols: symbols,
  };
};

const applyDeltaToSubaccount = (
  current: TraderStateSubaccountState,
  delta: TraderStateSubaccountDelta
): TraderStateSubaccountState => {
  const positions = applyPositionDeltas(current, delta.positions ?? []);
  const orders = applyOrderDeltas(current, delta.orders ?? []);
  const splines = applySplineDeltas(current, delta.splines ?? []);
  const triggers = applyTriggerDeltas(current, delta.triggers ?? []);
  const tradeHistory = mergeHistory(
    current.tradeHistory,
    delta.tradeHistory ?? [],
    makeTradeHistoryId
  );
  const orderHistory = mergeHistory(
    current.orderHistory,
    delta.orderHistory ?? [],
    makeOrderHistoryId
  );

  const nextBaseState = {
    subaccountIndex: delta.subaccountIndex,
    sequence: delta.sequence,
    collateral: delta.collateral,
    spotCollaterals: delta.spotCollaterals ?? current.spotCollaterals ?? [],
    capabilities: delta.capabilities ?? current.capabilities,
    cooldownStatus: delta.cooldownStatus ?? current.cooldownStatus,
    positionSymbols: positions.positionSymbols,
    positionsBySymbol: positions.positionsBySymbol,
    orderSymbols: orders.orderSymbols,
    ordersBySymbol: orders.ordersBySymbol,
    splineSymbols: splines.splineSymbols,
    splinesBySymbol: splines.splinesBySymbol,
    triggerSymbols: triggers.triggerSymbols,
    triggersBySymbol: triggers.triggersBySymbol,
    tradeHistory,
    orderHistory,
  };

  const snapshot = buildSubaccountSnapshotFromState(nextBaseState);
  return {
    ...nextBaseState,
    snapshot,
    marginInputs: buildSubaccountMarginInputsFromSnapshot(snapshot),
  };
};

const buildStateFromSnapshotMessage = (
  key: string,
  message: TraderStateServerMessage,
  now: number,
  health: TraderStateHealth,
  previousState: PhoenixTraderStateStoreState
): PhoenixTraderStateStoreState => {
  const subaccountsByIndex: Record<number, TraderStateSubaccountState> = {};
  const subaccountIndices = sortedSubaccountIndices(
    (message.subaccounts ?? []).map((subaccount) => subaccount.subaccountIndex)
  );

  for (const subaccount of message.subaccounts ?? []) {
    const snapshot: TraderStateSubaccountSnapshotState = {
      ...subaccount,
      positions: (subaccount.positions ?? []).map(clonePosition),
      orders: (subaccount.orders ?? []).map((event) => ({
        symbol: event.symbol,
        orders: (event.orders ?? []).map(cloneOrder),
      })),
      splines: (subaccount.splines ?? []).map(cloneSpline),
      triggers: (subaccount.triggers ?? []).map(cloneTriggers),
      tradeHistory: [],
      orderHistory: [],
    };
    subaccountsByIndex[subaccount.subaccountIndex] =
      buildSubaccountStateFromSnapshot(snapshot);
  }

  const snapshot: TraderStateSnapshotState = {
    authority: message.authority,
    traderPdaIndex: message.traderPdaIndex,
    slot: message.slot,
    version: message.version,
    capabilities: message.capabilities,
    makerFeeOverrideMultiplier: message.makerFeeOverrideMultiplier,
    takerFeeOverrideMultiplier: message.takerFeeOverrideMultiplier,
    subaccounts: subaccountIndices.map(
      (subaccountIndex) => subaccountsByIndex[subaccountIndex].snapshot
    ),
  };

  const previousMessage = previousState.lastMessage;
  return {
    key,
    authority: message.authority,
    traderPdaIndex: message.traderPdaIndex,
    status: {
      health,
      isConnected: health === "live",
      isLoading: false,
      error: null,
    },
    snapshot,
    snapshotMeta: buildSnapshotMetadata(snapshot),
    subaccountIndices,
    subaccountsByIndex,
    lastMessage: message,
    messageMeta: {
      lastMessageAtMs: now,
      lastMessageSlot: message.slot,
      lastMessageType: "snapshot",
      lastSnapshotSlot: message.slot,
      lastSnapshotAtMs: now,
      slotDelta:
        previousMessage && previousMessage.slot !== message.slot
          ? message.slot - previousMessage.slot
          : null,
      messageDeltaMs:
        previousState.messageMeta.lastMessageAtMs !== null
          ? now - previousState.messageMeta.lastMessageAtMs
          : null,
    },
    marginInputs: buildTraderMarginInputs(
      message.authority,
      message.traderPdaIndex,
      subaccountIndices,
      subaccountsByIndex
    ),
  };
};

const applyDeltaMessage = (
  current: PhoenixTraderStateStoreState,
  message: TraderStateServerMessage,
  now: number,
  health: TraderStateHealth
): PhoenixTraderStateStoreState => {
  if (!current.snapshot) {
    return current;
  }

  let subaccountsByIndex: Record<number, TraderStateSubaccountState> =
    current.subaccountsByIndex as Record<number, TraderStateSubaccountState>;
  let subaccountIndices = current.subaccountIndices;
  let changed = false;

  for (const delta of message.deltas ?? []) {
    const existing =
      current.subaccountsByIndex[delta.subaccountIndex] ??
      buildInitialSubaccountState(delta.subaccountIndex);
    const nextSubaccount = applyDeltaToSubaccount(existing, delta);

    if (subaccountsByIndex === current.subaccountsByIndex) {
      subaccountsByIndex = { ...current.subaccountsByIndex };
    }
    subaccountsByIndex[delta.subaccountIndex] = nextSubaccount;
    changed = true;

    if (!current.subaccountsByIndex[delta.subaccountIndex]) {
      const nextIndices = sortedSubaccountIndices([
        ...subaccountIndices,
        delta.subaccountIndex,
      ]);
      subaccountIndices = sameNumberArray(subaccountIndices, nextIndices)
        ? subaccountIndices
        : nextIndices;
    }
  }

  const snapshotSubaccounts =
    changed || current.snapshot.subaccounts.length !== subaccountIndices.length
      ? subaccountIndices.map(
          (subaccountIndex) => subaccountsByIndex[subaccountIndex].snapshot
        )
      : current.snapshot.subaccounts;

  const snapshot: TraderStateSnapshotState = {
    authority: current.snapshot.authority,
    traderPdaIndex: current.snapshot.traderPdaIndex,
    slot: message.slot,
    version: message.version ?? current.snapshot.version,
    capabilities: message.capabilities ?? current.snapshot.capabilities,
    makerFeeOverrideMultiplier:
      message.makerFeeOverrideMultiplier ??
      current.snapshot.makerFeeOverrideMultiplier,
    takerFeeOverrideMultiplier:
      message.takerFeeOverrideMultiplier ??
      current.snapshot.takerFeeOverrideMultiplier,
    subaccounts: snapshotSubaccounts,
  };

  return {
    ...current,
    status: {
      health,
      isConnected: true,
      isLoading: false,
      error: null,
    },
    snapshot,
    snapshotMeta: buildSnapshotMetadata(snapshot),
    subaccountIndices,
    subaccountsByIndex,
    lastMessage: message,
    messageMeta: {
      lastMessageAtMs: now,
      lastMessageSlot: message.slot,
      lastMessageType: "delta",
      lastSnapshotSlot: current.messageMeta.lastSnapshotSlot,
      lastSnapshotAtMs: current.messageMeta.lastSnapshotAtMs,
      slotDelta:
        current.lastMessage && current.lastMessage.slot !== message.slot
          ? message.slot - current.lastMessage.slot
          : null,
      messageDeltaMs:
        current.messageMeta.lastMessageAtMs !== null
          ? now - current.messageMeta.lastMessageAtMs
          : null,
    },
    marginInputs:
      changed || current.marginInputs === null
        ? buildTraderMarginInputs(
            snapshot.authority,
            snapshot.traderPdaIndex,
            subaccountIndices,
            subaccountsByIndex
          )
        : current.marginInputs,
  };
};

const createInitialStoreState = (
  key: string,
  authority: string,
  traderPdaIndex: number
): PhoenixTraderStateStoreState => ({
  key,
  authority,
  traderPdaIndex,
  status: {
    health: "cold",
    isConnected: false,
    isLoading: false,
    error: null,
  },
  snapshot: null,
  snapshotMeta: null,
  subaccountIndices: EMPTY_SUBACCOUNT_INDICES,
  subaccountsByIndex: EMPTY_SUBACCOUNTS_BY_INDEX,
  lastMessage: null,
  messageMeta: EMPTY_MESSAGE_META,
  marginInputs: EMPTY_MARGIN_INPUTS,
});

class PhoenixTraderStateResourceImpl implements PhoenixTraderStateResource {
  readonly key: string;
  readonly authority: string;
  readonly traderPdaIndex: number;
  readonly store;

  private bootstrapPromise: Promise<TraderStateSnapshotState | null> | null =
    null;
  private streamPromise: Promise<void> | null = null;
  private streamAbort: AbortController | null = null;
  private readonly lifecycle = createLifecycle({
    onDemandStop: () => {
      this.stopStream();
      this.updateStatus((status) => ({
        ...status,
        health: this.snapshot() ? "bootstrapped" : status.health,
        isConnected: false,
      }));
    },
  });

  constructor(
    params: Required<PhoenixTraderStateResourceParams>,
    private readonly config: PhoenixTraderStateManagerConfig,
    private readonly disposeFromManager: () => void
  ) {
    this.authority = params.authority;
    this.traderPdaIndex = params.traderPdaIndex;
    this.key = getTraderStateStoreKey(params.authority, params.traderPdaIndex);
    this.store = createStore<PhoenixTraderStateStoreState>(() =>
      createInitialStoreState(this.key, this.authority, this.traderPdaIndex)
    );
  }

  retain(): () => void {
    if (this.lifecycle.isClosed()) {
      throw new Error(`Trader state resource ${this.key} is closed`);
    }

    const release = this.lifecycle.retain();
    void this.ensureBootstrapped(false);
    this.ensureStreamLoop();
    return release;
  }

  async ready(): Promise<TraderStateSnapshotState | null> {
    return this.ensureBootstrapped(false);
  }

  status(): TraderStateStatus {
    return this.store.getState().status;
  }

  snapshot(): TraderStateSnapshotState | null {
    return this.store.getState().snapshot;
  }

  snapshotMeta(): TraderStateSnapshotMetadata | null {
    return this.store.getState().snapshotMeta;
  }

  subaccountIndices(): readonly number[] {
    return this.store.getState().subaccountIndices;
  }

  subaccount(subaccountIndex: number): TraderStateSubaccountState | null {
    return this.store.getState().subaccountsByIndex[subaccountIndex] ?? null;
  }

  position(
    subaccountIndex: number,
    symbol: string
  ): TraderStatePositionSnapshot | null {
    const subaccount = this.subaccount(subaccountIndex);
    if (!subaccount) return null;
    return subaccount.positionsBySymbol[normalizeSymbol(symbol)] ?? null;
  }

  orders(
    subaccountIndex: number,
    symbol: string
  ): readonly TraderStateMarketLimitOrderRow[] {
    const subaccount = this.subaccount(subaccountIndex);
    if (!subaccount) return EMPTY_ORDERS;
    const bucket = subaccount.ordersBySymbol[normalizeSymbol(symbol)];
    if (!bucket) return EMPTY_ORDERS;
    return bucket.orderSequenceNumbers.map(
      (orderSequenceNumber) => bucket.ordersBySequence[orderSequenceNumber]
    );
  }

  order(
    subaccountIndex: number,
    symbol: string,
    orderSequenceNumber: string
  ): TraderStateMarketLimitOrderRow | null {
    const subaccount = this.subaccount(subaccountIndex);
    if (!subaccount) return null;
    return (
      subaccount.ordersBySymbol[normalizeSymbol(symbol)]?.ordersBySequence[
        orderSequenceNumber
      ] ?? null
    );
  }

  triggers(
    subaccountIndex: number,
    symbol: string
  ): TraderStateTriggerSnapshot | null {
    const subaccount = this.subaccount(subaccountIndex);
    if (!subaccount) return null;
    return subaccount.triggersBySymbol[normalizeSymbol(symbol)] ?? null;
  }

  lastMessage(): TraderStateServerMessage | null {
    return this.store.getState().lastMessage;
  }

  messageMeta(): TraderStateMessageMeta {
    return this.store.getState().messageMeta;
  }

  marginInputs(): TraderMarginInputs | null {
    return this.store.getState().marginInputs;
  }

  async refresh(): Promise<TraderStateSnapshotState | null> {
    return this.ensureBootstrapped(true);
  }

  reconnect(): void {
    if (this.lifecycle.isClosed()) return;
    this.stopStream();
    this.updateStatus((status) => ({
      ...status,
      health: this.snapshot() ? "recovering" : "cold",
      isLoading: this.snapshot() === null,
      error: null,
    }));
    void this.ensureBootstrapped(true).finally(() => {
      this.ensureStreamLoop();
    });
  }

  subscribe(
    listener: (
      state: PhoenixTraderStateStoreState,
      previousState: PhoenixTraderStateStoreState
    ) => void
  ): () => void {
    return this.store.subscribe(listener);
  }

  close(): void {
    if (!this.lifecycle.close()) return;
    this.stopStream();
    this.store.setState(
      {
        ...this.store.getState(),
        status: {
          ...this.store.getState().status,
          health: "closed",
          isConnected: false,
          isLoading: false,
        },
      },
      true
    );
    this.disposeFromManager();
  }

  private updateStatus(
    updater: (status: TraderStateStatus) => TraderStateStatus
  ): void {
    const current = this.store.getState();
    this.store.setState(
      {
        ...current,
        status: updater(current.status),
      },
      true
    );
  }

  private stopStream(): void {
    this.streamAbort?.abort();
    this.streamAbort = null;
  }

  private ensureStreamLoop(): void {
    if (
      this.lifecycle.isClosed() ||
      !this.config.traderState ||
      !this.lifecycle.hasDemand() ||
      this.streamPromise
    ) {
      return;
    }

    this.streamPromise = this.runStreamLoop().finally(() => {
      this.streamPromise = null;
    });
  }

  private async runStreamLoop(): Promise<void> {
    await runRetryLoop({
      shouldContinue: () =>
        !this.lifecycle.isClosed() &&
        this.lifecycle.hasDemand() &&
        this.config.traderState !== undefined,
      backoffMs: this.config.resyncBackoffMs ?? DEFAULT_RESYNC_BACKOFF_MS,
      onController: (controller) => {
        this.streamAbort = controller;
      },
      stream: (signal) =>
        this.config.traderState!(this.authority, this.traderPdaIndex, signal),
      onMessage: (update) => {
        const message: TraderStateServerMessage = {
          channel: "traderState",
          authority: update.authority,
          traderPdaIndex: update.traderPdaIndex,
          slot: update.slot,
          messageType: update.messageType,
          version: update.version,
          capabilities: update.capabilities,
          makerFeeOverrideMultiplier: update.makerFeeOverrideMultiplier,
          takerFeeOverrideMultiplier: update.takerFeeOverrideMultiplier,
          subaccounts: update.subaccounts ?? [],
          deltas: update.deltas ?? [],
        };
        this.applyMessage(message, "live");
      },
      onDisconnect: () => {
        this.updateStatus((status) => ({
          ...status,
          health: this.snapshot() ? "recovering" : "cold",
          isConnected: this.snapshot() !== null,
        }));
      },
      onError: (error) => {
        const normalizedError = normalizeBackgroundError(
          error,
          "Trader state websocket stream failed"
        );
        this.config.onBackgroundError?.(normalizedError);
        this.updateStatus((status) => ({
          ...status,
          health: this.snapshot() ? "recovering" : "cold",
          isConnected: this.snapshot() !== null,
          error: normalizedError,
        }));
      },
      afterBackoff: async () => {
        try {
          await this.ensureBootstrapped(true);
        } catch (error) {
          this.config.onBackgroundError?.(error);
        }
      },
    });
  }

  private applyMessage(
    message: TraderStateServerMessage,
    health: TraderStateHealth
  ): void {
    const current = this.store.getState();
    const now = Date.now();
    const next =
      message.messageType === "snapshot"
        ? buildStateFromSnapshotMessage(this.key, message, now, health, current)
        : applyDeltaMessage(current, message, now, health);
    this.store.setState(next, true);
  }

  private async ensureBootstrapped(
    force: boolean
  ): Promise<TraderStateSnapshotState | null> {
    if (this.lifecycle.isClosed()) {
      return this.snapshot();
    }
    if (this.bootstrapPromise) {
      return this.bootstrapPromise;
    }
    if (!force && this.snapshot()) {
      return this.snapshot();
    }

    const current = this.store.getState();
    this.store.setState(
      {
        ...current,
        status: {
          ...current.status,
          isLoading: current.snapshot === null,
          error: null,
          health:
            current.snapshot === null
              ? "cold"
              : current.status.health === "live"
                ? "live"
                : "bootstrapped",
        },
      },
      true
    );

    this.bootstrapPromise = this.config.api
      .getTraderStateSnapshot(this.authority, {
        traderPdaIndex: this.traderPdaIndex,
      })
      .then((response) => {
        const message = toServerMessage(response);
        this.applyMessage(message, "bootstrapped");
        this.ensureStreamLoop();
        return this.snapshot();
      })
      .catch((error) => {
        const normalizedError =
          error instanceof Error
            ? error
            : new Error("Failed to bootstrap trader state");
        this.store.setState(
          {
            ...this.store.getState(),
            status: {
              ...this.store.getState().status,
              isLoading: false,
              isConnected: this.snapshot() !== null,
              error: normalizedError,
              health: this.snapshot() ? "recovering" : "cold",
            },
          },
          true
        );
        throw normalizedError;
      })
      .finally(() => {
        this.bootstrapPromise = null;
      });

    return this.bootstrapPromise;
  }
}

class PhoenixTraderStateManagerImpl implements PhoenixTraderStateManager {
  private readonly resources =
    createResourceRegistry<PhoenixTraderStateResourceImpl>();

  constructor(private readonly config: PhoenixTraderStateManagerConfig) {}

  resource(
    params: PhoenixTraderStateResourceParams
  ): PhoenixTraderStateResource {
    const normalizedParams = {
      authority: params.authority.trim(),
      traderPdaIndex: normalizeTraderPdaIndex(params.traderPdaIndex),
    };
    const key = getTraderStateStoreKey(
      normalizedParams.authority,
      normalizedParams.traderPdaIndex
    );
    return this.resources.getOrCreate(
      key,
      () =>
        new PhoenixTraderStateResourceImpl(
          normalizedParams,
          this.config,
          () => {
            this.resources.delete(key);
          }
        )
    );
  }

  async prefetch(
    params: PhoenixTraderStateResourceParams
  ): Promise<PhoenixTraderStateResource> {
    const resource = this.resource(params);
    await resource.ready();
    return resource;
  }

  peek(
    params: PhoenixTraderStateResourceParams
  ): PhoenixTraderStateResource | undefined {
    return this.resources.peek(
      getTraderStateStoreKey(params.authority, params.traderPdaIndex)
    );
  }

  clear(params: PhoenixTraderStateResourceParams): void {
    const key = getTraderStateStoreKey(params.authority, params.traderPdaIndex);
    const resource = this.resources.peek(key);
    if (!resource) {
      return;
    }
    resource.close();
  }

  close(): void {
    for (const resource of Array.from(this.resources.values())) {
      resource.close();
    }
    this.resources.clear();
  }
}

export const createPhoenixTraderStateManager = (
  config: PhoenixTraderStateManagerConfig
): PhoenixTraderStateManager => new PhoenixTraderStateManagerImpl(config);
