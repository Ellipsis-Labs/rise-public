import { createUpdateStream } from "@/ws/adapters/_utils";
import { applyStrictModeRecursive } from "@/ws/zodStrictMode";
import type { WsClient } from "@/ws/types";
import type { TraderStatePort } from "./ports";
import type { TraderStateServerMessage, TraderStateUpdate } from "./wire";
import { TraderStateServerMessageSchema } from "./wire";

export type TraderStateAdapter = TraderStatePort;

export interface TraderStateAdapterOptions {
  buffer?: number;
}

export const buildTraderStateSubscriptionKey = (
  authority: string,
  traderPdaIndex: number
) => `traderState:${authority}:${traderPdaIndex}`;

export const createTraderStateAdapter = (
  ws: WsClient,
  opts?: TraderStateAdapterOptions,
  strictMode?: boolean
): TraderStateAdapter => {
  const schema = strictMode
    ? applyStrictModeRecursive(TraderStateServerMessageSchema)
    : TraderStateServerMessageSchema;

  return createUpdateStream<
    TraderStateServerMessage,
    TraderStateUpdate,
    [authority: string, traderPdaIndex: number]
  >(
    ws,
    {
      channel: "traderState",
      schema,
      buildKey: buildTraderStateSubscriptionKey,
      buildSubParams: (authority, traderPdaIndex) => ({
        authority,
        traderPdaIndex,
      }),
      processMessage: (m, [authority, traderPdaIndex]) => {
        if (m.authority !== authority || m.traderPdaIndex !== traderPdaIndex) {
          console.error(
            "Received TraderState message for incorrect subscriber",
            {
              expectedAuthority: authority,
              receivedAuthority: m.authority,
              expectedPdaIndex: traderPdaIndex,
              receivedPdaIndex: m.traderPdaIndex,
            }
          );
          return null;
        }

        return {
          authority: m.authority,
          traderPdaIndex: m.traderPdaIndex,
          slot: m.slot,
          messageType: m.messageType,
          version: m.version,
          capabilities: m.capabilities,
          makerFeeOverrideMultiplier: m.makerFeeOverrideMultiplier,
          takerFeeOverrideMultiplier: m.takerFeeOverrideMultiplier,
          subaccounts: m.subaccounts ?? [],
          deltas: m.deltas ?? [],
        };
      },
      schemaErrorMessage: "Failed to parse TraderState message",
    },
    opts
  );
};
