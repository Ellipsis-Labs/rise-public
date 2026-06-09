/**
 * Minimal ix-building example mirroring rise/rust/sdk/examples/send_limit_order.rs.
 *
 * Run with:
 *   bun examples/03-build-limit-order-ix.ts [AUTHORITY_PUBKEY] [SYMBOL]
 */

import { Side, createPhoenixClient } from "@/index";

const DEFAULT_EXAMPLE_AUTHORITY = "11111111111111111111111111111111";
const authority = process.argv[2] ?? DEFAULT_EXAMPLE_AUTHORITY;
const requestedSymbol = process.argv[3] ?? "SOL";

const normalizeSymbol = (symbol: string) => symbol.trim().toUpperCase();

const resolveMarketSymbol = (
  availableSymbols: readonly string[],
  requested: string
): string => {
  const normalized = normalizeSymbol(requested);

  const exact = availableSymbols.find(
    (symbol) => normalizeSymbol(symbol) === normalized
  );
  if (exact) {
    return exact;
  }

  const perp = availableSymbols.find(
    (symbol) => normalizeSymbol(symbol) === `${normalized}-PERP`
  );
  if (perp) {
    return perp;
  }

  throw new Error(
    `Unknown market symbol '${requested}'. Available symbols: ${availableSymbols.join(", ")}`
  );
};

async function main() {
  if (!process.argv[2]) {
    console.log(
      "No authority provided. Using the all-ones System Program address for instruction building."
    );
    console.log(
      "Pass an explicit AUTHORITY_PUBKEY to build the instruction for a specific trader.\n"
    );
  }

  const client = createPhoenixClient({
    apiUrl: process.env.PHOENIX_API_URL ?? "https://perp-api.phoenix.trade",
    rpcUrl: process.env.SOLANA_RPC_URL ?? "https://api.mainnet-beta.solana.com",
    exchangeMetadata: { stream: false },
  });

  try {
    const snapshot = await client.exchange.ready();
    const marketSymbol = resolveMarketSymbol(
      snapshot.markets.map((market) => market.symbol),
      requestedSymbol
    );

    const orderPacket = await client.orderPackets.buildLimitOrderPacket({
      symbol: marketSymbol,
      side: Side.Bid,
      priceUsd: "150.50",
      baseUnits: "0.25",
    });

    const ix = await client.ixs.buildPlaceLimitOrder({
      authority,
      symbol: marketSymbol,
      orderPacket,
      traderPdaIndex: 0,
    });

    console.log({
      requestedSymbol,
      marketSymbol,
      programAddress: ix.programAddress,
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
