import type { StoreApi } from "zustand/vanilla";
import type {
  CooldownStatus,
  TraderStateCapabilities,
  TraderStateMarketLimitOrderRow,
  TraderStateOrderHistoryDelta,
  TraderStatePositionSnapshot,
  TraderStateServerMessage,
  TraderStateSnapshotResponse,
  TraderStateSplineSnapshot,
  TraderStateSpotCollateral,
  TraderStateSubaccountSnapshot,
  TraderStateTradeHistoryDelta,
  TraderStateTriggerSnapshot,
} from "@/api/traders/traderState";
import type {
  SubaccountMarginInputs,
  TraderMarginInputs,
} from "@/margin/types";
import type { TraderStatePort } from "@/ws/adapters/trader-state";

export type TraderStateHealth =
  | "cold"
  | "bootstrapped"
  | "live"
  | "recovering"
  | "closed";

export interface TraderStateMessageMeta {
  lastMessageAtMs: number | null;
  lastMessageSlot: bigint | null;
  lastMessageType: "snapshot" | "delta" | null;
  lastSnapshotSlot: bigint | null;
  lastSnapshotAtMs: number | null;
  slotDelta: bigint | null;
  messageDeltaMs: number | null;
}

export interface TraderStateSnapshotMetadata {
  authority: string;
  traderPdaIndex: number;
  slot: bigint;
  version?: number;
  capabilities?: TraderStateCapabilities;
  makerFeeOverrideMultiplier?: number;
  takerFeeOverrideMultiplier?: number;
}

export type TraderStateSubaccountSnapshotState =
  TraderStateSubaccountSnapshot & {
    tradeHistory: TraderStateTradeHistoryDelta[];
    orderHistory: TraderStateOrderHistoryDelta[];
  };

export interface TraderStateSnapshotState {
  authority: string;
  traderPdaIndex: number;
  slot: bigint;
  version?: number;
  capabilities?: TraderStateCapabilities;
  makerFeeOverrideMultiplier?: number;
  takerFeeOverrideMultiplier?: number;
  subaccounts: TraderStateSubaccountSnapshotState[];
}

export interface TraderStateOrderBucket {
  symbol: string;
  orderSequenceNumbers: readonly string[];
  ordersBySequence: Readonly<Record<string, TraderStateMarketLimitOrderRow>>;
}

export interface TraderStateSplineBucket {
  symbol: string;
  splineKeys: readonly string[];
  splinesByKey: Readonly<Record<string, TraderStateSplineSnapshot>>;
}

export interface TraderStateSubaccountState {
  subaccountIndex: number;
  sequence: number;
  collateral: string;
  spotCollaterals: readonly TraderStateSpotCollateral[];
  capabilities?: TraderStateCapabilities;
  cooldownStatus?: CooldownStatus;
  positionSymbols: readonly string[];
  positionsBySymbol: Readonly<Record<string, TraderStatePositionSnapshot>>;
  orderSymbols: readonly string[];
  ordersBySymbol: Readonly<Record<string, TraderStateOrderBucket>>;
  splineSymbols: readonly string[];
  splinesBySymbol: Readonly<Record<string, TraderStateSplineBucket>>;
  triggerSymbols: readonly string[];
  triggersBySymbol: Readonly<Record<string, TraderStateTriggerSnapshot>>;
  tradeHistory: readonly TraderStateTradeHistoryDelta[];
  orderHistory: readonly TraderStateOrderHistoryDelta[];
  snapshot: TraderStateSubaccountSnapshotState;
  marginInputs: SubaccountMarginInputs;
}

export interface TraderStateStatus {
  health: TraderStateHealth;
  isConnected: boolean;
  isLoading: boolean;
  error: Error | null;
}

export interface PhoenixTraderStateStoreState {
  key: string;
  authority: string;
  traderPdaIndex: number;
  status: TraderStateStatus;
  snapshot: TraderStateSnapshotState | null;
  snapshotMeta: TraderStateSnapshotMetadata | null;
  subaccountIndices: readonly number[];
  subaccountsByIndex: Readonly<Record<number, TraderStateSubaccountState>>;
  lastMessage: TraderStateServerMessage | null;
  messageMeta: TraderStateMessageMeta;
  marginInputs: TraderMarginInputs | null;
}

export interface PhoenixTraderStateSnapshotApi {
  getTraderStateSnapshot(
    authority: string,
    request?: { traderPdaIndex?: number }
  ): Promise<TraderStateSnapshotResponse>;
}

export interface PhoenixTraderStateManagerConfig {
  api: PhoenixTraderStateSnapshotApi;
  traderState?: TraderStatePort;
  resyncBackoffMs?: number;
  onBackgroundError?: (error: unknown) => void;
}

export interface PhoenixTraderStateResourceParams {
  authority: string;
  traderPdaIndex?: number;
}

export interface PhoenixTraderStateResource {
  readonly key: string;
  readonly authority: string;
  readonly traderPdaIndex: number;
  readonly store: StoreApi<PhoenixTraderStateStoreState>;
  retain(): () => void;
  ready(): Promise<TraderStateSnapshotState | null>;
  status(): TraderStateStatus;
  snapshot(): TraderStateSnapshotState | null;
  snapshotMeta(): TraderStateSnapshotMetadata | null;
  subaccountIndices(): readonly number[];
  subaccount(subaccountIndex: number): TraderStateSubaccountState | null;
  position(
    subaccountIndex: number,
    symbol: string
  ): TraderStatePositionSnapshot | null;
  orders(
    subaccountIndex: number,
    symbol: string
  ): readonly TraderStateMarketLimitOrderRow[];
  order(
    subaccountIndex: number,
    symbol: string,
    orderSequenceNumber: string
  ): TraderStateMarketLimitOrderRow | null;
  triggers(
    subaccountIndex: number,
    symbol: string
  ): TraderStateTriggerSnapshot | null;
  lastMessage(): TraderStateServerMessage | null;
  messageMeta(): TraderStateMessageMeta;
  marginInputs(): TraderMarginInputs | null;
  refresh(): Promise<TraderStateSnapshotState | null>;
  reconnect(): void;
  subscribe(
    listener: (
      state: PhoenixTraderStateStoreState,
      previousState: PhoenixTraderStateStoreState
    ) => void
  ): () => void;
  close(): void;
}

export interface PhoenixTraderStateManager {
  resource(
    params: PhoenixTraderStateResourceParams
  ): PhoenixTraderStateResource;
  prefetch(
    params: PhoenixTraderStateResourceParams
  ): Promise<PhoenixTraderStateResource>;
  peek(
    params: PhoenixTraderStateResourceParams
  ): PhoenixTraderStateResource | undefined;
  clear(params: PhoenixTraderStateResourceParams): void;
  close(): void;
}
