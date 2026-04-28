import { base64 } from "@scure/base";
import { decompress } from "fzstd";
import type { ExchangeSnapshotEncoding } from "@/api/exchange/types";
import type { WsClient } from "@/ws/types";
import { createUpdateStream } from "@/ws/adapters/_utils";
import { applyStrictModeRecursive } from "@/ws/zodStrictMode";
import type { ExchangePort } from "./ports";
import {
  ExchangeSnapshotMsgSchema,
  ExchangeWireMsgSchema,
  type ExchangeEncodedSnapshotMsg,
  type ExchangeMsg,
  type ExchangeWireMsg,
} from "./wire";

export type ExchangeAdapter = ExchangePort;

export interface ExchangeAdapterOptions {
  buffer?: number;
  encoding?: ExchangeSnapshotEncoding;
}

export const createExchangeAdapter = (
  ws: WsClient,
  opts?: ExchangeAdapterOptions,
  strictMode?: boolean
): ExchangeAdapter => {
  const schema = strictMode
    ? applyStrictModeRecursive(ExchangeWireMsgSchema)
    : ExchangeWireMsgSchema;
  return createUpdateStream<ExchangeWireMsg, ExchangeMsg, []>(
    ws,
    {
      channel: "exchange",
      schema,
      buildKey: () => "exchange",
      buildSubParams: () =>
        opts?.encoding === undefined ? {} : { encoding: opts.encoding },
      processMessage: (message) =>
        message.messageType === "encodedSnapshot"
          ? decodeExchangeSnapshot(message)
          : message,
      schemaErrorMessage: "Failed to parse exchange message",
    },
    opts
  );
};

const decodeExchangeSnapshot = (
  message: ExchangeEncodedSnapshotMsg
): ExchangeMsg => {
  const compressed = base64.decode(message.payload);
  const decodedBytes = decompress(compressed);
  const decodedText = new TextDecoder().decode(decodedBytes);
  const decoded = JSON.parse(decodedText) as unknown;
  const snapshotResult = ExchangeSnapshotMsgSchema.safeParse({
    channel: "exchange",
    messageType: "snapshot",
    ...(decoded as Record<string, unknown>),
  });

  if (!snapshotResult.success) {
    throw new Error("Failed to decode encoded exchange snapshot");
  }

  const snapshot = snapshotResult.data;
  if (
    snapshot.version !== message.version ||
    snapshot.sequenceNumber !== message.sequenceNumber ||
    snapshot.slot !== message.slot ||
    snapshot.slotIndex !== message.slotIndex ||
    snapshot.reason !== message.reason
  ) {
    throw new Error("Encoded exchange snapshot metadata mismatch");
  }

  return snapshot;
};
