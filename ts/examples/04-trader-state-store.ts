/**
 * Trader-state store example backed by createPhoenixClient(...).
 *
 * This shows:
 * - creating a unified client
 * - wiring createTraderStateStore(client)
 * - retaining a resource so it subscribes through client.streams.traderState(...)
 * - reading live values from the store
 *
 * Run with:
 *   bun examples/04-trader-state-store.ts [AUTHORITY_PUBKEY] [TRADER_PDA_INDEX] [--watch]
 */

import { createPhoenixClient, createTraderStateStore } from "@/index";

const DEFAULT_EXAMPLE_AUTHORITY = "11111111111111111111111111111111";
const rawArgs = process.argv.slice(2);
const watch = rawArgs.includes("--watch");
const positionalArgs = rawArgs.filter((arg) => arg !== "--watch");
const authority = positionalArgs[0] ?? DEFAULT_EXAMPLE_AUTHORITY;
const traderPdaIndex = Number(positionalArgs[1] ?? "0");

async function main() {
  if (!Number.isInteger(traderPdaIndex) || traderPdaIndex < 0) {
    throw new Error(
      `TRADER_PDA_INDEX must be a non-negative integer, received '${positionalArgs[1] ?? ""}'`
    );
  }

  if (!positionalArgs[0]) {
    console.log(
      "No authority provided. Using the all-ones System Program address for the trader-state subscription."
    );
    console.log(
      "Pass an explicit AUTHORITY_PUBKEY to inspect a specific trader, or add --watch to stay subscribed.\n"
    );
  }

  const client = createPhoenixClient({
    apiUrl: process.env.PHOENIX_API_URL ?? "https://perp-api.phoenix.trade",
    ws: { connectMode: "eager" },
  });

  const traderState = createTraderStateStore(client);
  const resource = traderState.resource({
    authority,
    traderPdaIndex,
  });
  let resolveFirstLiveUpdate: (() => void) | null = null;
  const firstLiveUpdate = new Promise<void>((resolve) => {
    resolveFirstLiveUpdate = resolve;
  });

  const unsubscribe = resource.subscribe((state, previousState) => {
    if (state.snapshot === previousState.snapshot) {
      return;
    }

    const subaccount0 = resource.subaccount(0);
    const firstSymbol = subaccount0?.positionSymbols[0];
    const firstPosition = firstSymbol
      ? resource.position(0, firstSymbol)
      : null;

    console.log("Trader state updated");
    console.log({
      health: state.status.health,
      lastMessageType: state.messageMeta.lastMessageType,
      subaccounts: state.subaccountIndices,
      collateral: subaccount0?.collateral,
      firstPositionSymbol: firstSymbol,
      firstPositionLots: firstPosition?.basePositionLots,
    });

    if (state.status.health === "live") {
      resolveFirstLiveUpdate?.();
      resolveFirstLiveUpdate = null;
    }
  });

  const release = resource.retain();
  let closed = false;

  const shutdown = () => {
    if (closed) {
      return;
    }
    closed = true;
    unsubscribe();
    release();
    traderState.close();
    client.dispose();
  };

  try {
    const snapshot = await resource.ready();

    console.log("Initial trader snapshot");
    console.log({
      authority: snapshot?.authority,
      traderPdaIndex: snapshot?.traderPdaIndex,
      subaccounts: snapshot?.subaccounts.length ?? 0,
    });

    if (watch) {
      console.log("Waiting for trader-state updates. Press Ctrl+C to exit.");
      await new Promise<void>((resolve) => {
        process.once("SIGINT", resolve);
      });
      return;
    }

    console.log("Waiting for the first live trader-state update...");
    const result = await Promise.race([
      firstLiveUpdate.then(() => "live" as const),
      new Promise<"timeout">((resolve) => {
        setTimeout(() => resolve("timeout"), 10_000);
      }),
    ]);

    if (result === "timeout") {
      console.log(
        "No live trader-state update arrived within 10 seconds; exiting after the initial snapshot."
      );
    }
  } finally {
    shutdown();
  }
}

void main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
