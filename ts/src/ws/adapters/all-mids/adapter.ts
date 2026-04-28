import { createUpdateStream } from "@/ws/adapters/_utils";
import type { WsClient } from "@/ws/types";
import { applyStrictModeRecursive } from "@/ws/zodStrictMode";
import type { AllMidsPort } from "./ports";
import { AllMidsMsgSchema, type AllMidsMsg, type AllMidsUpdate } from "./wire";

export type AllMidsAdapter = AllMidsPort;

export interface AllMidsAdapterOptions {
  buffer?: number;
}

export const createAllMidsAdapter = (
  ws: WsClient,
  opts?: AllMidsAdapterOptions,
  strictMode?: boolean
): AllMidsAdapter => {
  const schema = strictMode
    ? applyStrictModeRecursive(AllMidsMsgSchema)
    : AllMidsMsgSchema;

  return createUpdateStream<AllMidsMsg, AllMidsUpdate, []>(
    ws,
    {
      channel: "allMids",
      schema,
      buildKey: () => "allMids",
      buildSubParams: () => ({}),
      processMessage: async (message) => ({
        mids: message.mids,
        slot: message.slot,
        slotIndex: message.slotIndex,
      }),
      schemaErrorMessage: "Failed to parse AllMids message",
    },
    opts
  );
};
