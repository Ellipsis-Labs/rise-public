import z from "zod";
import { numericBigint } from "@/ws/numericSchemas";
import {
  ExchangeMarketSnapshotSchema,
  ExchangeSnapshotEncodingSchema,
  ExchangeStateSnapshotSchema,
  ExchangeWsCommodityMetadataSchema,
  ExchangeWsFeeConfigSchema,
  ExchangeWsFundingConfigSchema,
  ExchangeWsLeverageTierSchema,
  ExchangeWsMarkPriceParametersSchema,
} from "@/api/exchange/types";
import type { ExchangeSnapshotEncoding } from "@/api/exchange/types";

export type ExchangeSnapshotReason = "snapshot";

export interface ExchangeKeysUpdatedOp {
  kind: "exchangeKeysUpdated";
  exchange: z.infer<typeof ExchangeStateSnapshotSchema>;
}

export interface ExchangeStatusChangedOp {
  kind: "exchangeStatusChanged";
  previousBits: number;
  newBits: number;
  previousFeatures: string[];
  newFeatures: string[];
  enabledFeatures: string[];
  disabledFeatures: string[];
  active: boolean;
  gated: boolean;
}

export interface MarketAddedOp {
  kind: "marketAdded";
  market: z.infer<typeof ExchangeMarketSnapshotSchema>;
}

export interface MarketStatusChangedOp {
  kind: "marketStatusChanged";
  symbol: string;
  previousMarketStatus: string;
  newMarketStatus: string;
}

export interface MarketClosedOp {
  kind: "marketClosed";
  symbol: string;
  previousMarketStatus: string;
  finalizedMarkPrice: bigint;
}

export interface MarketTombstonedOp {
  kind: "marketTombstoned";
  symbol: string;
  previousMarketStatus: string;
  finalSequenceNumber: bigint;
  finalTradeSequenceNumber: bigint;
  finalOrderSequenceNumber: bigint;
}

export interface MarketDeletedOp {
  kind: "marketDeleted";
  symbol: string;
  assetId: number;
}

export interface CancelRiskFactorUpdated {
  kind: "cancelRiskFactorUpdated";
  previous: number;
  new: number;
}

export interface IsolatedOnlyUpdated {
  kind: "isolatedOnlyUpdated";
  previous: boolean;
  new: boolean;
}

export interface LeverageTiersUpdated {
  kind: "leverageTiersUpdated";
  previous: z.infer<typeof ExchangeWsLeverageTierSchema>[];
  new: z.infer<typeof ExchangeWsLeverageTierSchema>[];
}

export interface MarkPriceParametersUpdated {
  kind: "markPriceParametersUpdated";
  previous: z.infer<typeof ExchangeWsMarkPriceParametersSchema>;
  new: z.infer<typeof ExchangeWsMarkPriceParametersSchema>;
}

export interface OpenInterestCapUpdated {
  kind: "openInterestCapUpdated";
  previousBaseLots: bigint;
  newBaseLots: bigint;
}

export interface UpnlRiskFactorUpdated {
  kind: "upnlRiskFactorUpdated";
  previous: number;
  new: number;
}

export interface UpnlRiskFactorForWithdrawalsUpdated {
  kind: "upnlRiskFactorForWithdrawalsUpdated";
  previous: number;
  new: number;
}

export interface FundingParametersUpdated {
  kind: "fundingParametersUpdated";
  previous: z.infer<typeof ExchangeWsFundingConfigSchema>;
  new: z.infer<typeof ExchangeWsFundingConfigSchema>;
}

export interface MarketFeesUpdated {
  kind: "marketFeesUpdated";
  previous: z.infer<typeof ExchangeWsFeeConfigSchema>;
  new: z.infer<typeof ExchangeWsFeeConfigSchema>;
}

export interface CommodityMetadataUpdated {
  kind: "commodityMetadataUpdated";
  previous: z.infer<typeof ExchangeWsCommodityMetadataSchema>;
  new: z.infer<typeof ExchangeWsCommodityMetadataSchema>;
}

export type ExchangeMarketParameterUpdate =
  | CancelRiskFactorUpdated
  | IsolatedOnlyUpdated
  | LeverageTiersUpdated
  | MarkPriceParametersUpdated
  | OpenInterestCapUpdated
  | UpnlRiskFactorUpdated
  | UpnlRiskFactorForWithdrawalsUpdated
  | FundingParametersUpdated
  | MarketFeesUpdated
  | CommodityMetadataUpdated;

export interface MarketParameterUpdatedOp {
  kind: "marketParameterUpdated";
  symbol: string;
  update: ExchangeMarketParameterUpdate;
}

export type ExchangeDeltaOp =
  | ExchangeKeysUpdatedOp
  | ExchangeStatusChangedOp
  | MarketAddedOp
  | MarketStatusChangedOp
  | MarketClosedOp
  | MarketTombstonedOp
  | MarketDeletedOp
  | MarketParameterUpdatedOp;

export interface ExchangeSnapshotMsg {
  channel: "exchange";
  messageType: "snapshot";
  version: number;
  sequenceNumber: bigint;
  slot: bigint;
  slotIndex: number;
  reason: ExchangeSnapshotReason;
  exchange: z.infer<typeof ExchangeStateSnapshotSchema>;
  markets: z.infer<typeof ExchangeMarketSnapshotSchema>[];
}

export interface ExchangeDeltaMsg {
  channel: "exchange";
  messageType: "delta";
  version: number;
  sequenceNumber: bigint;
  slot: bigint;
  slotIndex: number;
  ops: ExchangeDeltaOp[];
}

export type ExchangeMsg = ExchangeSnapshotMsg | ExchangeDeltaMsg;

export interface ExchangeEncodedSnapshotMsg {
  channel: "exchange";
  messageType: "encodedSnapshot";
  version: number;
  sequenceNumber: bigint;
  slot: bigint;
  slotIndex: number;
  reason: ExchangeSnapshotReason;
  encoding: ExchangeSnapshotEncoding;
  payload: string;
}

export type ExchangeWireMsg = ExchangeMsg | ExchangeEncodedSnapshotMsg;

const normalizeExchangeDeltaOp = (value: unknown): unknown => {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return value;
  }

  const op = { ...(value as Record<string, unknown>) };

  switch (op.kind) {
    case "exchangeStatusChanged":
      if (op.previousBits === undefined) {
        op.previousBits = op.previous_bits;
      }
      if (op.newBits === undefined) {
        op.newBits = op.new_bits;
      }
      if (op.previousFeatures === undefined) {
        op.previousFeatures = op.previous_features;
      }
      if (op.newFeatures === undefined) {
        op.newFeatures = op.new_features;
      }
      if (op.enabledFeatures === undefined) {
        op.enabledFeatures = op.enabled_features;
      }
      if (op.disabledFeatures === undefined) {
        op.disabledFeatures = op.disabled_features;
      }
      return op;
    case "marketStatusChanged":
      if (op.previousMarketStatus === undefined) {
        op.previousMarketStatus = op.previous_market_status;
      }
      if (op.newMarketStatus === undefined) {
        op.newMarketStatus = op.new_market_status;
      }
      return op;
    case "marketClosed":
      if (op.previousMarketStatus === undefined) {
        op.previousMarketStatus = op.previous_market_status;
      }
      if (op.finalizedMarkPrice === undefined) {
        op.finalizedMarkPrice = op.finalized_mark_price;
      }
      return op;
    case "marketTombstoned":
      if (op.previousMarketStatus === undefined) {
        op.previousMarketStatus = op.previous_market_status;
      }
      if (op.finalSequenceNumber === undefined) {
        op.finalSequenceNumber = op.final_sequence_number;
      }
      if (op.finalTradeSequenceNumber === undefined) {
        op.finalTradeSequenceNumber = op.final_trade_sequence_number;
      }
      if (op.finalOrderSequenceNumber === undefined) {
        op.finalOrderSequenceNumber = op.final_order_sequence_number;
      }
      return op;
    case "marketDeleted":
      if (op.assetId === undefined) {
        op.assetId = op.asset_id;
      }
      return op;
    default:
      return op;
  }
};

const ExchangeStatusChangedOpSchema = z.object({
  kind: z.literal("exchangeStatusChanged"),
  previousBits: z.number(),
  newBits: z.number(),
  previousFeatures: z.array(z.string()),
  newFeatures: z.array(z.string()),
  enabledFeatures: z.array(z.string()),
  disabledFeatures: z.array(z.string()),
  active: z.boolean(),
  gated: z.boolean(),
});

const ExchangeKeysUpdatedOpSchema = z.object({
  kind: z.literal("exchangeKeysUpdated"),
  exchange: ExchangeStateSnapshotSchema,
});

const MarketAddedOpSchema = z.object({
  kind: z.literal("marketAdded"),
  market: ExchangeMarketSnapshotSchema,
});

const MarketStatusChangedOpSchema = z.object({
  kind: z.literal("marketStatusChanged"),
  symbol: z.string(),
  previousMarketStatus: z.string(),
  newMarketStatus: z.string(),
});

const MarketClosedOpSchema = z.object({
  kind: z.literal("marketClosed"),
  symbol: z.string(),
  previousMarketStatus: z.string(),
  finalizedMarkPrice: numericBigint("finalizedMarkPrice"),
});

const MarketTombstonedOpSchema = z.object({
  kind: z.literal("marketTombstoned"),
  symbol: z.string(),
  previousMarketStatus: z.string(),
  finalSequenceNumber: numericBigint("finalSequenceNumber"),
  finalTradeSequenceNumber: numericBigint("finalTradeSequenceNumber"),
  finalOrderSequenceNumber: numericBigint("finalOrderSequenceNumber"),
});

const MarketDeletedOpSchema = z.object({
  kind: z.literal("marketDeleted"),
  symbol: z.string(),
  assetId: z.number(),
});

const exchangeMarketParameterUpdateSchema = z.discriminatedUnion("kind", [
  z.object({
    kind: z.literal("cancelRiskFactorUpdated"),
    previous: z.number(),
    new: z.number(),
  }),
  z.object({
    kind: z.literal("isolatedOnlyUpdated"),
    previous: z.boolean(),
    new: z.boolean(),
  }),
  z.object({
    kind: z.literal("leverageTiersUpdated"),
    previous: z.array(ExchangeWsLeverageTierSchema),
    new: z.array(ExchangeWsLeverageTierSchema),
  }),
  z.object({
    kind: z.literal("markPriceParametersUpdated"),
    previous: ExchangeWsMarkPriceParametersSchema,
    new: ExchangeWsMarkPriceParametersSchema,
  }),
  z.object({
    kind: z.literal("openInterestCapUpdated"),
    previousBaseLots: numericBigint("previousBaseLots"),
    newBaseLots: numericBigint("newBaseLots"),
  }),
  z.object({
    kind: z.literal("upnlRiskFactorUpdated"),
    previous: z.number(),
    new: z.number(),
  }),
  z.object({
    kind: z.literal("upnlRiskFactorForWithdrawalsUpdated"),
    previous: z.number(),
    new: z.number(),
  }),
  z.object({
    kind: z.literal("fundingParametersUpdated"),
    previous: ExchangeWsFundingConfigSchema,
    new: ExchangeWsFundingConfigSchema,
  }),
  z.object({
    kind: z.literal("marketFeesUpdated"),
    previous: ExchangeWsFeeConfigSchema,
    new: ExchangeWsFeeConfigSchema,
  }),
  z.object({
    kind: z.literal("commodityMetadataUpdated"),
    previous: ExchangeWsCommodityMetadataSchema,
    new: ExchangeWsCommodityMetadataSchema,
  }),
]);

export const ExchangeMarketParameterUpdateSchema: z.ZodType<ExchangeMarketParameterUpdate> =
  exchangeMarketParameterUpdateSchema;

const MarketParameterUpdatedOpSchema = z.object({
  kind: z.literal("marketParameterUpdated"),
  symbol: z.string(),
  update: exchangeMarketParameterUpdateSchema,
});

const exchangeDeltaOpSchema = z.preprocess(
  normalizeExchangeDeltaOp,
  z.discriminatedUnion("kind", [
    ExchangeKeysUpdatedOpSchema,
    ExchangeStatusChangedOpSchema,
    MarketAddedOpSchema,
    MarketStatusChangedOpSchema,
    MarketClosedOpSchema,
    MarketTombstonedOpSchema,
    MarketDeletedOpSchema,
    MarketParameterUpdatedOpSchema,
  ])
);

export const ExchangeDeltaOpSchema: z.ZodType<ExchangeDeltaOp> =
  exchangeDeltaOpSchema;

const exchangeSnapshotMsgSchema = z.object({
  channel: z.literal("exchange"),
  messageType: z.literal("snapshot"),
  version: z.number(),
  sequenceNumber: numericBigint("sequenceNumber"),
  slot: numericBigint("slot"),
  slotIndex: z.number(),
  reason: z.literal("snapshot"),
  exchange: ExchangeStateSnapshotSchema,
  markets: z.array(ExchangeMarketSnapshotSchema),
});

export const ExchangeSnapshotMsgSchema: z.ZodType<ExchangeSnapshotMsg> =
  exchangeSnapshotMsgSchema;

const exchangeEncodedSnapshotMsgSchema = z.object({
  channel: z.literal("exchange"),
  messageType: z.literal("encodedSnapshot"),
  version: z.number(),
  sequenceNumber: numericBigint("sequenceNumber"),
  slot: numericBigint("slot"),
  slotIndex: z.number(),
  reason: z.literal("snapshot"),
  encoding: ExchangeSnapshotEncodingSchema,
  payload: z.string(),
});

export const ExchangeEncodedSnapshotMsgSchema: z.ZodType<ExchangeEncodedSnapshotMsg> =
  exchangeEncodedSnapshotMsgSchema;

const exchangeDeltaMsgSchema = z.object({
  channel: z.literal("exchange"),
  messageType: z.literal("delta"),
  version: z.number(),
  sequenceNumber: numericBigint("sequenceNumber"),
  slot: numericBigint("slot"),
  slotIndex: z.number(),
  ops: z.array(exchangeDeltaOpSchema),
});

export const ExchangeDeltaMsgSchema: z.ZodType<ExchangeDeltaMsg> =
  exchangeDeltaMsgSchema;

const exchangeWireMsgSchema = z.discriminatedUnion("messageType", [
  exchangeSnapshotMsgSchema,
  exchangeEncodedSnapshotMsgSchema,
  exchangeDeltaMsgSchema,
]);

export const ExchangeWireMsgSchema: z.ZodType<ExchangeWireMsg> =
  exchangeWireMsgSchema;

const exchangeMsgSchema = z.discriminatedUnion("messageType", [
  exchangeSnapshotMsgSchema,
  exchangeDeltaMsgSchema,
]);

export const ExchangeMsgSchema: z.ZodType<ExchangeMsg> = exchangeMsgSchema;
