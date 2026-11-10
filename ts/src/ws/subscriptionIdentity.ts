import { isRecord } from "./adapters/_utils/messageUtils";

// Market symbol fields the server accepts under several names (`trades`
// requests `symbol` and echoes `marketSymbol`; `l2Book` uses `coin`).
const MARKET_SYMBOL_FIELDS: Readonly<Record<string, true>> = {
  symbol: true,
  marketSymbol: true,
  coin: true,
};

// Wire fields of SubscriptionRequest in sdk/phoenix-api-types/src/websocket.rs
// (including its public, trader_state, active_trader_state and twap requests).
// Keep every variant here: typed server echoes discard unknown request fields.
const SERVER_SUBSCRIPTION_FIELDS: Readonly<Record<string, readonly string[]>> =
  {
    allMids: [],
    market: ["symbol"],
    fundingRate: ["symbol"],
    orderbook: ["symbol", "bypassExecutionBand"],
    l2Book: ["coin", "bypassExecutionBand"],
    events: [],
    trader: ["traderKey"],
    trades: ["symbol", "marketSymbol"],
    fills: ["marketSymbol"],
    traderFills: ["authority", "traderPdaIndex"],
    candles: ["symbol", "timeframe"],
    markPrice: ["symbol"],
    notifications: ["userId", "authority"],
    exchange: ["encoding"],
    exchangeStatus: [],
    marketStats: ["symbol"],
    marketStatsV2: ["symbols"],
    traderState: ["authority", "traderPdaIndex"],
    activeTraderState: ["encoding"],
    traderVolume: ["authority", "traderPdaIndex"],
    twap: ["authority", "traderPdaIndex"],
    liquidations: ["symbol"],
    riskEvents: [],
    transaction: ["userPubkey"],
  };

const foldSymbol = (value: unknown): unknown =>
  typeof value === "string" ? value.trim().toLowerCase() : value;

const foldSymbols = (value: unknown): unknown => {
  if (!Array.isArray(value)) return value;
  const folded = value.map(foldSymbol);
  if (!folded.every((symbol) => typeof symbol === "string")) return folded;
  return Array.from(new Set(folded)).sort();
};

const stableStringify = (value: unknown): string => {
  if (Array.isArray(value)) {
    return `[${value.map(stableStringify).join(",")}]`;
  }
  if (isRecord(value)) {
    const entries = Object.keys(value)
      .filter((key) => value[key] !== undefined)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${stableStringify(value[key])}`);
    return `{${entries.join(",")}}`;
  }
  return JSON.stringify(value) ?? "null";
};

/**
 * Identity of a subscription request as the server sees it, used to match a
 * server echo (`subscriptionStatus.subscription`, `error.subscription`) to the
 * request the client sent.
 *
 * Built-in channels retain only their modeled server fields. Market symbol
 * fields (`symbol`, `marketSymbol`, `coin`) are one lowercased field, `symbols`
 * is lowercased, sorted and deduped, and absent, `null` and
 * `bypassExecutionBand: false` fields are dropped. Custom channels retain their
 * own fields unchanged. Object keys are sorted in either case.
 *
 * Returns `null` when the value is not a subscription request.
 */
export const subscriptionIdentity = (subscription: unknown): string | null => {
  if (!isRecord(subscription)) return null;
  const { channel } = subscription;
  if (typeof channel !== "string") return null;

  const serverFields = Object.hasOwn(SERVER_SUBSCRIPTION_FIELDS, channel)
    ? SERVER_SUBSCRIPTION_FIELDS[channel]
    : undefined;
  const fields: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(subscription)) {
    if (key === "channel" || value === undefined) continue;
    if (serverFields === undefined) {
      fields[key] = value;
      continue;
    }
    if (!serverFields.includes(key) || value === null) continue;
    if (key === "bypassExecutionBand" && value === false) continue;
    if (MARKET_SYMBOL_FIELDS[key] === true) {
      fields.symbol = foldSymbol(value);
    } else if (key === "symbols") {
      fields.symbols = foldSymbols(value);
    } else {
      fields[key] = value;
    }
  }

  return `${channel}${stableStringify(fields)}`;
};
