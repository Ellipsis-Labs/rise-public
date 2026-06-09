/**
 * Minimal Flight market-order ix-building example mirroring
 * `03-build-limit-order-ix.ts`. Configures `createPhoenixClient(...)` with a
 * builder authority so `client.ixs.placeMarketOrder(...)` returns a
 * Flight-wrapped proxy instruction.
 *
 * Run with:
 *   bun examples/06-flight-market-order.ts <BUILDER_AUTHORITY> <TRADER_AUTHORITY> <SYMBOL> <bid|ask> <NUM_BASE_LOTS> [PRICE_LIMIT_TICKS]
 */

import {
  OrderFlags,
  SelfTradeBehavior,
  Side,
  baseLots,
  createPhoenixClient,
  quoteLots,
  ticks,
  type Authority,
  type ImmediateOrCancelOrderPacket,
} from "@/index";

const [
  builderAuthority,
  traderAuthority,
  requestedSymbol,
  sideArg,
  numBaseLotsArg,
  priceLimitTicksArg,
] = process.argv.slice(2);

if (
  !builderAuthority ||
  !traderAuthority ||
  !requestedSymbol ||
  !sideArg ||
  !numBaseLotsArg
) {
  console.error(
    "Usage: bun examples/06-flight-market-order.ts <BUILDER_AUTHORITY> <TRADER_AUTHORITY> <SYMBOL> <bid|ask> <NUM_BASE_LOTS> [PRICE_LIMIT_TICKS]"
  );
  process.exit(1);
}

const parseSide = (value: string): Side => {
  const normalized = value.toLowerCase();
  if (normalized === "bid" || normalized === "buy") return Side.Bid;
  if (normalized === "ask" || normalized === "sell") return Side.Ask;
  throw new Error(`Invalid side '${value}', expected bid|ask`);
};

const normalizeSymbol = (symbol: string) => symbol.trim().toUpperCase();

const resolveMarketSymbol = (
  availableSymbols: readonly string[],
  requested: string
): string => {
  const normalized = normalizeSymbol(requested);
  const exact = availableSymbols.find(
    (symbol) => normalizeSymbol(symbol) === normalized
  );
  if (exact) return exact;
  const perp = availableSymbols.find(
    (symbol) => normalizeSymbol(symbol) === `${normalized}-PERP`
  );
  if (perp) return perp;
  throw new Error(
    `Unknown market symbol '${requested}'. Available symbols: ${availableSymbols.join(", ")}`
  );
};

async function main() {
  const client = createPhoenixClient({
    apiUrl: process.env.PHOENIX_API_URL ?? "https://perp-api.phoenix.trade",
    rpcUrl: process.env.SOLANA_RPC_URL ?? "https://api.mainnet-beta.solana.com",
    exchangeMetadata: { stream: false },
    flight: {
      builderAuthority: builderAuthority as Authority,
    },
  });

  try {
    const snapshot = await client.exchange.ready();
    const marketSymbol = resolveMarketSymbol(
      snapshot.markets.map((market: { symbol: string }) => market.symbol),
      requestedSymbol
    );

    const side = parseSide(sideArg);
    const numBaseLots = baseLots(BigInt(numBaseLotsArg));
    const priceInTicks = priceLimitTicksArg
      ? ticks(BigInt(priceLimitTicksArg))
      : null;

    const orderPacket: ImmediateOrCancelOrderPacket = {
      side,
      priceInTicks,
      numBaseLots,
      numQuoteLots: null,
      minBaseLotsToFill: numBaseLots,
      minQuoteLotsToFill: quoteLots(1n),
      selfTradeBehavior: SelfTradeBehavior.Abort,
      matchLimit: null,
      clientOrderId: 0n,
      lastValidSlot: null,
      orderFlags: OrderFlags.None,
      cancelExisting: false,
    };

    const ix = await client.ixs.placeMarketOrder({
      authority: traderAuthority as Authority,
      symbol: marketSymbol,
      orderPacket,
    });

    console.log({
      builderAuthority,
      traderAuthority,
      requestedSymbol,
      marketSymbol,
      side: Side[side],
      numBaseLots: numBaseLots.toString(),
      priceInTicks: priceInTicks?.toString() ?? null,
      flightProgramAddress: ix.programAddress,
      accountCount: ix.accounts.length,
      dataLength: ix.data.length,
    });
  } finally {
    client.dispose();
  }
}

void main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
