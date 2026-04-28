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
