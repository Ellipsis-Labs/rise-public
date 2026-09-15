import z from "zod";

export interface ApiCandle {
  time: number;
  open: number;
  high: number;
  low: number;
  close: number;
  markOpen?: number;
  markHigh?: number;
  markLow?: number;
  markClose?: number;
  volume: number;
  /** Quote volume for the candle period. */
  volumeQuote?: number;
  tradeCount: number;
  /** External candle source name (e.g., "binance", "coinbase"). Omitted for exchange candles. */
  externalSource?: string;
}

const ApiCandleWireSchema = z.object({
  time: z.number().int(),
  open: z.number(),
  high: z.number(),
  low: z.number(),
  close: z.number(),
  markOpen: z.number().optional(),
  markHigh: z.number().optional(),
  markLow: z.number().optional(),
  markClose: z.number().optional(),
  volume: z.number(),
  volumeQuote: z.number().optional(),
  tradeCount: z.number().int(),
  // Accepted for backward compatibility, but intentionally not exposed in SDK types.
  isExternal: z.boolean().optional(),
  externalSource: z.string().optional(),
});

export const ApiCandleSchema: z.ZodType<ApiCandle> =
  ApiCandleWireSchema.transform<ApiCandle>(
    ({ isExternal: _isExternal, ...rest }) => rest
  );

export interface TradingCandlesQuery {
  timeframe: string;
  startTime?: number;
  endTime?: number;
  /** Maximum number of candles to return (server default and max: 2,500). */
  limit?: number;
  /** Opt-in external candles stored in the DB (default: false). */
  enableExternalSource?: boolean;
}

export const TradingCandlesQuerySchema: z.ZodType<TradingCandlesQuery> =
  z.object({
    timeframe: z.string(),
    startTime: z.number().optional(),
    endTime: z.number().optional(),
    limit: z.number().optional(),
    enableExternalSource: z.boolean().optional(),
  });

export interface ApiCandleV2 {
  /** Candle start timestamp in milliseconds since Unix epoch. */
  time: number;
  open: number;
  high: number;
  low: number;
  close: number;
  markOpen: number;
  markHigh: number;
  markLow: number;
  markClose: number;
  volume?: number;
  /** Quote volume for the candle period. */
  volumeQuote?: number;
  tradeCount?: number;
  /** External candle source name when the canonical bar was backfilled. */
  externalSource?: string;
  /** True when the candle bucket is closed and should not continue updating. */
  isFinal: boolean;
}

const ApiCandleV2WireSchema = z.object({
  time: z.number().int(),
  open: z.number(),
  high: z.number(),
  low: z.number(),
  close: z.number(),
  markOpen: z.number(),
  markHigh: z.number(),
  markLow: z.number(),
  markClose: z.number(),
  volume: z.number().nullish(),
  volumeQuote: z.number().nullish(),
  tradeCount: z.number().int().nullish(),
  externalSource: z.string().nullish(),
  isFinal: z.boolean(),
});

export const ApiCandleV2Schema: z.ZodType<ApiCandleV2> =
  ApiCandleV2WireSchema.transform<ApiCandleV2>(
    ({ volume, volumeQuote, tradeCount, externalSource, ...rest }) => ({
      ...rest,
      ...(volume == null ? {} : { volume }),
      ...(volumeQuote == null ? {} : { volumeQuote }),
      ...(tradeCount == null ? {} : { tradeCount }),
      ...(externalSource == null ? {} : { externalSource }),
    })
  );

export interface CandlesV2Page {
  hasMore: boolean;
  nextCursor?: string;
}

const CandlesV2PageWireSchema = z.object({
  hasMore: z.boolean(),
  nextCursor: z.string().nullish(),
});

export const CandlesV2PageSchema: z.ZodType<CandlesV2Page> =
  CandlesV2PageWireSchema.transform<CandlesV2Page>(
    ({ nextCursor, ...rest }) => ({
      ...rest,
      ...(nextCursor == null ? {} : { nextCursor }),
    })
  );

export interface CandlesV2Response {
  symbol: string;
  timeframe: string;
  /** Echoed inclusive lower bound in milliseconds since Unix epoch. */
  from: number;
  /** Echoed exclusive upper bound in milliseconds since Unix epoch. */
  to: number;
  bars: ApiCandleV2[];
  page: CandlesV2Page;
}

export const CandlesV2ResponseSchema: z.ZodType<CandlesV2Response> = z
  .object({
    symbol: z.string(),
    timeframe: z.string(),
    from: z.number().int(),
    to: z.number().int(),
    bars: ApiCandleV2Schema.array(),
    page: CandlesV2PageSchema,
  })
  .transform<CandlesV2Response>((response) => response);

export interface TradingCandlesV2InitialQuery {
  timeframe: string;
  /** Inclusive lower bound in milliseconds since Unix epoch. */
  from: number;
  /** Exclusive upper bound in milliseconds since Unix epoch. */
  to: number;
  /** Maximum bars returned in this page (SDK default: 1,000; server max: 10,000). */
  limit?: number;
  /** Include the current open bucket when available (default: true). */
  includePartial?: boolean;
  cursor?: never;
}

export interface TradingCandlesV2CursorQuery {
  cursor: string;
  timeframe?: never;
  from?: never;
  to?: never;
  limit?: never;
  includePartial?: never;
}

export type TradingCandlesV2Query =
  | TradingCandlesV2InitialQuery
  | TradingCandlesV2CursorQuery;

export const TradingCandlesV2InitialQuerySchema: z.ZodType<TradingCandlesV2InitialQuery> =
  z.object({
    timeframe: z.string(),
    from: z.number().int(),
    to: z.number().int(),
    limit: z.number().int().optional(),
    includePartial: z.boolean().optional(),
    cursor: z.never().optional(),
  });

export const TradingCandlesV2CursorQuerySchema: z.ZodType<TradingCandlesV2CursorQuery> =
  z.object({
    cursor: z.string(),
    timeframe: z.never().optional(),
    from: z.never().optional(),
    to: z.never().optional(),
    limit: z.never().optional(),
    includePartial: z.never().optional(),
  });

export const TradingCandlesV2QuerySchema: z.ZodType<TradingCandlesV2Query> =
  z.union([
    TradingCandlesV2InitialQuerySchema,
    TradingCandlesV2CursorQuerySchema,
  ]);
