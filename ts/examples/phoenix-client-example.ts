/**
 * PhoenixHttpClient Examples
 *
 * This script demonstrates the examples from the phoenix-client documentation.
 * Run with: `bun examples/phoenix-client-example.ts [pubkey]`
 *
 * Arguments:
 *   pubkey - (optional) Solana public key for trader-specific queries
 *           If not provided, trader queries will show expected 404 errors
 */

import { createPhoenixClient } from "@/client";

async function main() {
  // Get trader pubkey from command line arguments
  const args = process.argv.slice(2);
  const traderPubkey = args[0] || "11111111111111111111111111111111";

  if (!args[0]) {
    console.log(
      "ℹ No trader pubkey provided. Using dummy pubkey for trader queries."
    );
    console.log("  To test trader-specific queries, run:");
    console.log(
      "  bun examples/phoenix-client-example.ts <your-solana-pubkey>"
    );
    console.log("");
  } else {
    console.log(`ℹ Using trader pubkey: ${traderPubkey}\n`);
  }
  // Example 1: Create a client with default configuration
  console.log("=== Example 1: Creating a client ===");
  const client = createPhoenixClient();
  // const client = createPhoenixClient({ apiUrl: "https://perp-api.phoenix.trade" });
  // const client = createPhoenixClient({ apiKey: "your-api-key" });
  console.log("✓ Client created successfully");
  console.log(
    `  API client: ${client.api ? "initialized" : "not initialized"}`
  );

  try {
    // Example 2: Get exchange configuration
    console.log("\n=== Example 2: Get exchange metadata ===");
    const exchange = await client.api.exchange().getExchange();
    console.log("✓ Exchange metadata fetched");
    console.log(`  Markets count: ${exchange.markets?.length ?? "N/A"}`);

    // Example 3: Get list of markets
    console.log("\n=== Example 3: Get markets list ===");
    const markets = await client.api.markets().getMarkets();
    console.log("✓ Markets fetched");
    console.log(`  Markets count: ${markets.length}`);
    if (markets.length > 0) {
      console.log(`  First market symbol: ${markets[0].symbol || "N/A"}`);
    }

    // Example 4: Get specific market data
    console.log("\n=== Example 4: Get specific market (SOL) ===");
    const market = await client.api.markets().getMarket("SOL");
    console.log("✓ Market fetched");
    console.log(`  Symbol: ${market.symbol || "N/A"}`);
    console.log(`  Status: ${market.marketStatus || "N/A"}`);

    // Example 6: Get trader view (requires valid authority pubkey)
    console.log("\n=== Example 6: Get trader view ===");
    try {
      const traderState = await client.api
        .traders()
        .getTraderState(traderPubkey);
      console.log("✓ Trader state fetched");
      if (traderState.traders.length > 0) {
        const traderView = traderState.traders[0];
        console.log(
          `  First trader view: ${JSON.stringify(traderView).substring(0, 80)}...`
        );
        const capabilitiesValid = traderView.verifyCapabilities();
        console.log(`  Capabilities valid: ${capabilitiesValid}`);
      } else {
        console.log("  No traders in state");
      }
    } catch (error) {
      console.log(
        `⚠ Could not fetch trader state: ${(error as Error).message}`
      );
    }

    // Example 7: Get order history
    console.log("\n=== Example 7: Get trader order history ===");
    try {
      const orders = await client.api
        .orders()
        .getTraderOrderHistory(traderPubkey, {
          limit: 10,
        });
      console.log("✓ Order history fetched");
      console.log(`  Orders count: ${orders.data?.length ?? "N/A"}`);
    } catch (error) {
      console.log(`⚠ Could not fetch orders: ${(error as Error).message}`);
    }

    // Example 8: Get trade history
    console.log("\n=== Example 8: Get trader trade history ===");
    try {
      const trades = await client.api
        .trades()
        .getTraderTradesHistory(traderPubkey, {
          marketSymbol: "SOL",
          limit: 2,
        });
      console.log("✓ Trade history fetched");
      console.log(`  Trades count: ${trades.data?.length ?? "N/A"}`);
    } catch (error) {
      console.log(`⚠ Could not fetch trades: ${(error as Error).message}`);
    }

    // Example 9: Get collateral history
    console.log("\n=== Example 9: Get trader collateral history ===");
    try {
      const collateral = await client.api
        .collateral()
        .getTraderCollateralHistory(traderPubkey, {
          limit: 10,
        });
      console.log("✓ Collateral history fetched");
      console.log(`  Events count: ${collateral.data?.length ?? "N/A"}`);
    } catch (error) {
      console.log(
        `⚠ Could not fetch collateral history: ${(error as Error).message}`
      );
    }

    // Example 10: Get funding history
    console.log("\n=== Example 10: Get trader funding history ===");
    try {
      const funding = await client.api
        .funding()
        .getTraderFundingHistory(traderPubkey, {
          symbol: "SOL",
          limit: 10,
        });
      console.log("✓ Funding history fetched");
      console.log(`  Events count: ${funding.events?.length ?? "N/A"}`);
    } catch (error) {
      console.log(
        `⚠ Could not fetch funding history: ${(error as Error).message}`
      );
    }

    // Example 11: Get funding rate history (market data)
    console.log("\n=== Example 11: Get funding rate history (SOL) ===");
    try {
      const fundingRates = await client.api
        .funding()
        .getFundingRateHistory("SOL", {
          limit: 10,
        });
      console.log("✓ Funding rate history fetched");
      console.log(`  Data points: ${fundingRates.rates?.length ?? "N/A"}`);
    } catch (error) {
      console.log(
        `⚠ Could not fetch funding rates: ${(error as Error).message}`
      );
    }

    // Example 12: Get candles (OHLCV data)
    console.log("\n=== Example 12: Get candles (OHLCV) ===");
    try {
      const candles = await client.api.candles().getCandles("SOL", {
        timeframe: "1m",
        limit: 10,
      });
      console.log("✓ Candles fetched");
      console.log(`  Candles count: ${candles.length}`);
      if (candles.length > 0) {
        const c = candles[0];
        console.log(
          `  Latest candle: O=${c.open} H=${c.high} L=${c.low} C=${c.close}`
        );
      }
    } catch (error) {
      console.log(`⚠ Could not fetch candles: ${(error as Error).message}`);
    }

    // Example 13: Get market fills
    console.log("\n=== Example 13: Get market fills (SOL) ===");
    try {
      const fills = await client.api.trades().getMarketFills("SOL", {
        limit: 10,
      });
      console.log("✓ Market fills fetched");
      console.log(`  Fills count: ${fills.data?.length ?? "N/A"}`);
    } catch (error) {
      console.log(
        `⚠ Could not fetch market fills: ${(error as Error).message}`
      );
    }

    // Example 14: Get trader PnL
    console.log("\n=== Example 14: Get trader PnL ===");
    try {
      const pnl = await client.api.traders().getTraderPnl(traderPubkey, {
        resolution: "1h",
        limit: 10,
      });
      console.log("✓ Trader PnL fetched");
      console.log(`  Data points: ${pnl.length}`);
    } catch (error) {
      console.log(`⚠ Could not fetch PnL: ${(error as Error).message}`);
    }

    console.log("\n=== All examples completed ===");
  } catch (error) {
    console.error("Error during examples:", error);
  } finally {
    // Cleanup
    client.dispose?.();
  }
}

main().catch(console.error);
