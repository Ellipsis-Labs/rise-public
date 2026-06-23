/**
 * Minimal HTTP client example mirroring rise/rust/examples/http_client.rs.
 *
 * Run with:
 *   bun examples/01-http-client.ts [TRADER_PUBKEY]
 */

import { createPhoenixClient } from "@/index";

const traderPubkey = process.argv[2];

async function main() {
  const client = createPhoenixClient({
    apiUrl: process.env.PHOENIX_API_URL ?? "https://perp-api.phoenix.trade",
  });

  try {
    const keys = await client.api.exchange().getKeys();
    const market = await client.api.markets().getMarket("SOL");
    const candles = await client.api.candles().getCandles("SOL", {
      timeframe: "1m",
      limit: 5,
    });

    console.log("Exchange keys");
    console.log({
      globalConfig: keys.globalConfig,
      perpAssetMap: keys.perpAssetMap,
      globalVault: keys.globalVault,
    });

    console.log("\nSOL market");
    console.log({
      symbol: market.symbol,
      marketStatus: market.marketStatus,
      marketPubkey: market.marketPubkey,
      splinePubkey: market.splinePubkey,
      tickSize: market.tickSize,
    });

    console.log("\nLatest candles");
    console.log(
      candles.map((candle) => ({
        time: candle.time,
        open: candle.open,
        high: candle.high,
        low: candle.low,
        close: candle.close,
      }))
    );

    if (traderPubkey) {
      const snapshot = await client.api
        .traders()
        .getTraderStateSnapshot(traderPubkey, {
          traderPdaIndex: 0,
        });

      console.log("\nTrader snapshot");
      console.log({
        authority: snapshot.authority,
        traderPdaIndex: snapshot.traderPdaIndex,
        subaccounts: snapshot.snapshot.subaccounts.length,
      });
    }
  } finally {
    client.dispose();
  }
}

void main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
