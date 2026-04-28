import { createPhoenixWsClient } from "@/ws/PhoenixWsClient";

// Custom JSON replacer to handle BigInt values
const jsonReplacer = (_key: string, value: unknown) => {
  if (typeof value === "bigint") {
    return `${value}n`;
  }
  return value;
};

interface ChannelSubscription {
  channel: string;
  args: string[];
}

const parseChannelArgs = (args: string[]): ChannelSubscription[] => {
  const subscriptions: ChannelSubscription[] = [];
  let i = 0;

  while (i < args.length) {
    const channel = args[i];
    const channelArgs: string[] = [];

    // Parse arguments for this channel until we hit another channel or end
    i++;
    while (i < args.length && !isChannelName(args[i])) {
      channelArgs.push(args[i]);
      i++;
    }

    subscriptions.push({ channel, args: channelArgs });
  }

  return subscriptions;
};

const isChannelName = (name: string): boolean => {
  const validChannels = [
    "allMids",
    "candles",
    "exchangeStatus",
    "fills",
    "fundingRate",
    "l2Book",
    "markPrice",
    "market",
    "marketStats",
    "notifications",
    "orderbook",
    "orderbookSnapshot",
    "traderState",
  ];
  return validChannels.includes(name);
};

const printUsage = () => {
  console.log(`
WebSocket Client Example

Usage: bun run examples/phoenix-ws-example.ts [CHANNEL] [ARGS...] [CHANNEL] [ARGS...] ...

Channels:
  allMids              - All markets mid prices (no args)
  candles SYMBOL TIMEFRAME
                       - Candlestick data (e.g., SOL 1m)
  exchangeStatus       - Exchange status updates (no args)
  fills [SYMBOL]       - Market fills (optional symbol filter)
  fundingRate SYMBOL   - Funding rate updates (e.g., SOL)
  l2Book SYMBOL        - L2 order book (e.g., SOL)
  markPrice SYMBOL     - Mark price updates (e.g., SOL)
  market SYMBOL        - Market updates (e.g., SOL)
  marketStats SYMBOL   - Market statistics (e.g., SOL)
  notifications [AUTHORITY]
                       - Notifications (optional authority/pubkey)
  orderbook SYMBOL     - L2 orderbook snapshot (e.g., SOL)
  orderbookSnapshot SYMBOL
                       - Orderbook snapshot (e.g., SOL)
  traderState PUBKEY PDAINDEX
                       - Trader state updates (e.g., Drm7CaM8... 0)

Examples:
  bun run examples/phoenix-ws-example.ts allMids
  bun run examples/phoenix-ws-example.ts market SOL exchangeStatus
  bun run examples/phoenix-ws-example.ts candles SOL 1m fundingRate SOL market BTC
  bun run examples/phoenix-ws-example.ts traderState Drm7CaM8rniCWAgSkULcVAyXketYByjwjWSMhVHmPvdD 0
  bun run examples/phoenix-ws-example.ts notifications Drm7CaM8rniCWAgSkULcVAyXketYByjwjWSMhVHmPvdD
`);
};

const main = async () => {
  const args = process.argv.slice(2);

  if (args.length === 0) {
    printUsage();
    return;
  }

  if (args[0] === "--help" || args[0] === "-h") {
    printUsage();
    return;
  }

  const subscriptions = parseChannelArgs(args);

  console.log(`🔌 Connecting to Phoenix WebSocket...`);
  const wsClient = createPhoenixWsClient({
    url: process.env.PHOENIX_WS_URL ?? "wss://perp-api.phoenix.trade/v1/ws",
  });

  console.log(`✓ Connected\n`);

  const abortController = new AbortController();

  // Handle graceful shutdown
  process.on("SIGINT", () => {
    console.log("\n\n🛑 Shutting down...");
    abortController.abort();
    wsClient.close();
    process.exit(0);
  });

  // Subscribe to all requested channels
  for (const sub of subscriptions) {
    subscribeToChannel(wsClient, sub.channel, sub.args, abortController.signal);
  }

  // Keep the process alive
  await new Promise(() => {});
};

const subscribeToChannel = (
  wsClient: ReturnType<typeof createPhoenixWsClient>,
  channel: string,
  args: string[],
  signal: AbortSignal
) => {
  (async () => {
    try {
      switch (channel) {
        case "allMids":
          console.log(`📡 Subscribing to: allMids`);
          for await (const update of wsClient.allMids(signal)) {
            console.log(`[allMids]`, JSON.stringify(update, jsonReplacer, 2));
          }
          break;

        case "candles": {
          if (args.length < 2) {
            console.error(
              `❌ candles requires 2 arguments: symbol timeframe (got ${args.length})`
            );
            return;
          }
          const [symbol, timeframe] = args;
          console.log(`📡 Subscribing to: candles ${symbol} ${timeframe}`);
          for await (const update of wsClient.candles(
            symbol,
            timeframe,
            signal
          )) {
            console.log(
              `[candles:${symbol}:${timeframe}]`,
              JSON.stringify(update, jsonReplacer, 2)
            );
          }
          break;
        }

        case "exchangeStatus":
          console.log(`📡 Subscribing to: exchangeStatus`);
          for await (const update of wsClient.exchangeStatus(signal)) {
            console.log(
              `[exchangeStatus]`,
              JSON.stringify(update, jsonReplacer, 2)
            );
          }
          break;

        case "fills": {
          const symbol = args[0];
          console.log(`📡 Subscribing to: fills ${symbol || "(all)"}`);
          for await (const update of wsClient.fills(symbol, signal)) {
            console.log(
              `[fills${symbol ? `:${symbol}` : ""}]`,
              JSON.stringify(update, jsonReplacer, 2)
            );
          }
          break;
        }

        case "fundingRate": {
          if (args.length < 1) {
            console.error(
              `❌ fundingRate requires 1 argument: symbol (got ${args.length})`
            );
            return;
          }
          const symbol = args[0];
          console.log(`📡 Subscribing to: fundingRate ${symbol}`);
          for await (const update of wsClient.fundingRate(symbol, signal)) {
            console.log(
              `[fundingRate:${symbol}]`,
              JSON.stringify(update, jsonReplacer, 2)
            );
          }
          break;
        }

        case "l2Book": {
          if (args.length < 1) {
            console.error(
              `❌ l2Book requires 1 argument: symbol (got ${args.length})`
            );
            return;
          }
          const symbol = args[0];
          console.log(`📡 Subscribing to: l2Book ${symbol}`);
          for await (const update of wsClient.l2Book(symbol, signal)) {
            console.log(
              `[l2Book:${symbol}]`,
              JSON.stringify(update, jsonReplacer, 2)
            );
          }
          break;
        }

        case "markPrice": {
          if (args.length < 1) {
            console.error(
              `❌ markPrice requires 1 argument: symbol (got ${args.length})`
            );
            return;
          }
          const symbol = args[0];
          console.log(`📡 Subscribing to: markPrice ${symbol}`);
          for await (const update of wsClient.markPrice(symbol, signal)) {
            console.log(
              `[markPrice:${symbol}]`,
              JSON.stringify(update, jsonReplacer, 2)
            );
          }
          break;
        }

        case "market": {
          if (args.length < 1) {
            console.error(
              `❌ market requires 1 argument: symbol (got ${args.length})`
            );
            return;
          }
          const symbol = args[0];
          console.log(`📡 Subscribing to: market ${symbol}`);
          for await (const update of wsClient.market(symbol, signal)) {
            console.log(
              `[market:${symbol}]`,
              JSON.stringify(update, jsonReplacer, 2)
            );
          }
          break;
        }

        case "marketStats": {
          if (args.length < 1) {
            console.error(
              `❌ marketStats requires 1 argument: symbol (got ${args.length})`
            );
            return;
          }
          const symbol = args[0];
          console.log(`📡 Subscribing to: marketStats ${symbol}`);
          for await (const update of wsClient.marketStats(symbol, signal)) {
            console.log(
              `[marketStats:${symbol}]`,
              JSON.stringify(update, jsonReplacer, 2)
            );
          }
          break;
        }

        case "notifications": {
          const params: { userId?: string; authority?: string } = {};
          if (args[0]?.startsWith("--userId=")) {
            params.userId = args[0].substring(9);
          } else if (args[0]?.startsWith("--authority=")) {
            params.authority = args[0].substring(12);
          } else if (args[0]) {
            // Assume first arg is authority if no flag
            params.authority = args[0];
          }

          if (!params.userId && !params.authority) {
            console.error(
              `❌ notifications requires --userId=VALUE or --authority=VALUE or just AUTHORITY`
            );
            return;
          }

          const desc = params.userId
            ? `notifications (userId: ${params.userId})`
            : `notifications (authority: ${params.authority})`;
          console.log(`📡 Subscribing to: ${desc}`);
          for await (const update of wsClient.notifications(params, signal)) {
            console.log(`[${desc}]`, JSON.stringify(update, jsonReplacer, 2));
          }
          break;
        }

        case "orderbook": {
          if (args.length < 1) {
            console.error(
              `❌ orderbook requires 1 argument: symbol (got ${args.length})`
            );
            return;
          }
          const symbol = args[0];
          console.log(`📡 Subscribing to: orderbook ${symbol}`);
          for await (const update of wsClient.orderbook(symbol, signal)) {
            console.log(
              `[orderbook:${symbol}]`,
              JSON.stringify(update, jsonReplacer, 2)
            );
          }
          break;
        }

        case "orderbookSnapshot": {
          if (args.length < 1) {
            console.error(
              `❌ orderbookSnapshot requires 1 argument: symbol (got ${args.length})`
            );
            return;
          }
          const symbol = args[0];
          console.log(`📡 Subscribing to: orderbookSnapshot ${symbol}`);
          for await (const update of wsClient.orderbookSnapshot(
            symbol,
            signal
          )) {
            console.log(
              `[orderbookSnapshot:${symbol}]`,
              JSON.stringify(update, jsonReplacer, 2)
            );
          }
          break;
        }

        case "traderState": {
          if (args.length < 2) {
            console.error(
              `❌ traderState requires 2 arguments: authority pdaIndex (got ${args.length})`
            );
            return;
          }
          const [authority, pdaIndexStr] = args;
          const pdaIndex = parseInt(pdaIndexStr, 10);
          if (isNaN(pdaIndex)) {
            console.error(`❌ pdaIndex must be a number, got: ${pdaIndexStr}`);
            return;
          }
          console.log(
            `📡 Subscribing to: traderState ${authority} ${pdaIndex}`
          );
          for await (const update of wsClient.traderState(
            authority,
            pdaIndex,
            signal
          )) {
            console.log(
              `[traderState:${authority}:${pdaIndex}]`,
              JSON.stringify(update, jsonReplacer, 2)
            );
          }
          break;
        }

        default:
          console.error(`❌ Unknown channel: ${channel}`);
      }
    } catch (error) {
      if (error instanceof Error && error.name === "AbortError") {
        // Expected on shutdown
        return;
      }
      console.error(`❌ Error in channel ${channel}:`, error);
    }
  })();
};

main().catch(console.error);
