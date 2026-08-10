/**
 * Minimal Flight market-order ix-building example mirroring
 * `03-build-limit-order-ix.ts`. Configures `createPhoenixClient(...)` with a
 * builder authority so `client.ixs.placeMarketOrder(...)` returns a
 * Flight-wrapped proxy instruction.
 *
 * Run with:
 *   bun examples/06-flight-market-order.ts <BUILDER_AUTHORITY> <TRADER_AUTHORITY> <SYMBOL> <bid|ask> <NUM_BASE_LOTS> [PRICE_LIMIT_TICKS]
 *
 * By default the order is owner-signed: TRADER_AUTHORITY is both the trader
 * account owner and the wallet that signs. To build the delegate-signed
 * variant instead, set POSITION_AUTHORITY to the delegate wallet:
 *
 *   POSITION_AUTHORITY=<DELEGATE_ADDRESS> bun examples/06-flight-market-order.ts ...
 *
 * Use it when the transaction is signed by the trader's position authority
 * (a delegate key the owner authorized on-chain), not by the wallet owner.
 * The trader account still derives from TRADER_AUTHORITY; passing the
 * delegate as `positionAuthority` routes the wrap through the Flight
 * position-authority path, which appends the collateral-transfer tail so the
 * builder fee can be collected. The permission account in that tail is
 * derived from the Phoenix root authority, resolved automatically from the
 * exchange snapshot.
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

// Optional delegate wallet that signs instead of the owner. When unset the
// order is owner-signed (the common case).
const positionAuthoritySigner = process.env.POSITION_AUTHORITY;

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

    // `authority` is always the trader account owner — the trader PDA
    // derives from it. When POSITION_AUTHORITY is set, the delegate becomes
    // the wallet on the instruction (the effective signer is
    // `positionAuthority ?? authority`) and the wrap automatically takes the
    // Flight position-authority path because the signer differs from the
    // owner: the collateral-transfer tail is appended, with the permission
    // account derived from the root authority in the exchange snapshot.
    const ix = await client.ixs.placeMarketOrder({
      authority: traderAuthority as Authority,
      ...(positionAuthoritySigner
        ? { positionAuthority: positionAuthoritySigner as Authority }
        : {}),
      symbol: marketSymbol,
      orderPacket,
    });

    console.log({
      builderAuthority,
      traderAuthority,
      signer: positionAuthoritySigner ?? traderAuthority,
      signerKind: positionAuthoritySigner ? "position-authority" : "owner",
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
