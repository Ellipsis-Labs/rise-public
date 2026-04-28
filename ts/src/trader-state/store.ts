import type { PhoenixClient, PhoenixHttpClient } from "@/client";
import { createPhoenixTraderStateManager } from "./manager";
import type {
  PhoenixTraderStateManager,
  PhoenixTraderStateManagerConfig,
} from "./types";
import type { TraderStatePort } from "@/ws/adapters/trader-state";

export type CreateTraderStateStoreConfig = Omit<
  PhoenixTraderStateManagerConfig,
  "api" | "traderState"
>;

export interface TraderStateStoreClientLike {
  api: Pick<PhoenixHttpClient, "traders">;
  streams?: {
    traderState?: TraderStatePort;
  };
}

export const createTraderStateStore = (
  client: Pick<PhoenixClient, "api" | "streams"> | TraderStateStoreClientLike,
  config: CreateTraderStateStoreConfig = {}
): PhoenixTraderStateManager =>
  createPhoenixTraderStateManager({
    ...config,
    api: {
      getTraderStateSnapshot: (authority, request) =>
        client.api.traders().getTraderStateSnapshot(authority, {
          traderPdaIndex: request?.traderPdaIndex,
        }),
    },
    traderState: client.streams?.traderState,
  });
