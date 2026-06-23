/**
 * Minimal WebSocket fills example mirroring rise/rust/examples/subscribe_trades.rs.
 *
 * Run with:
 *   bun examples/02-ws-fills.ts [SYMBOL]
 */

import { createPhoenixWsClient } from "@/index";

const symbol = process.argv[2] ?? "SOL";

async function main() {
  const streams = createPhoenixWsClient({
    url: process.env.PHOENIX_WS_URL ?? "wss://perp-api.phoenix.trade/v1/ws",
    authMode: "anonymous",
  });

  try {
    console.log(`Subscribing to fills for ${symbol}...`);

    for await (const update of streams.fills(symbol)) {
      console.log({
        symbol: update.symbol,
        marketSymbol: update.fill.marketSymbol,
        price: update.fill.price,
        baseQty: update.fill.baseQty,
        instructionType: update.fill.instructionType,
        timestampMs: update.fill.timestampMs,
      });
      break;
    }
  } finally {
    streams.close();
  }
}

void main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
