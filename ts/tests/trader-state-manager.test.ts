import { describe, expect, it } from "vitest";
import {
  createTraderStateStore,
  createPhoenixTraderStateManager,
  type TraderStateSnapshotResponse,
  type TraderStateUpdate,
} from "@/index";

const CAPABILITIES = {
  flags: 1,
  state: "active" as const,
  capabilities: {
    placeLimitOrder: { immediate: true, viaColdActivation: false },
    placeMarketOrder: { immediate: true, viaColdActivation: false },
    riskIncreasingTrade: { immediate: true, viaColdActivation: false },
    riskReducingTrade: { immediate: true, viaColdActivation: false },
    depositCollateral: { immediate: true, viaColdActivation: false },
    withdrawCollateral: { immediate: true, viaColdActivation: false },
  },
};

const createSnapshotResponse = (): TraderStateSnapshotResponse => ({
  authority: "authority-1",
  traderPdaIndex: 0,
  slot: 10,
  slotIndex: 1,
  snapshot: {
    version: 1,
    capabilities: CAPABILITIES,
    makerFeeOverrideMultiplier: 1,
    takerFeeOverrideMultiplier: 1,
    subaccounts: [
      {
        subaccountIndex: 0,
        sequence: 1,
        collateral: "1000",
        positions: [
          {
            symbol: "SOL-PERP",
            positionSequenceNumber: "1",
            basePositionLots: "2",
            entryPriceTicks: "100",
            virtualQuotePositionLots: "10",
            unsettledFundingQuoteLots: "0",
            accumulatedFundingQuoteLots: "0",
            takeProfitTriggers: [],
            stopLossTriggers: [],
            conditionalTakeProfitTriggers: [],
            conditionalStopLossTriggers: [],
          },
        ],
        orders: [],
        splines: [],
        triggers: [],
      },
      {
        subaccountIndex: 1,
        sequence: 1,
        collateral: "500",
        positions: [],
        orders: [],
        splines: [],
        triggers: [],
      },
    ],
  },
});

const createPushPort = () => {
  const queue: TraderStateUpdate[] = [];
  let notify: (() => void) | null = null;

  const push = (update: TraderStateUpdate) => {
    queue.push(update);
    notify?.();
    notify = null;
  };

  const traderState = async function* (
    authority: string,
    traderPdaIndex: number,
    signal?: AbortSignal
  ): AsyncIterable<TraderStateUpdate> {
    while (!signal?.aborted) {
      if (queue.length === 0) {
        await new Promise<void>((resolve) => {
          notify = resolve;
          signal?.addEventListener("abort", resolve, { once: true });
        });
      }

      while (queue.length > 0) {
        const update = queue.shift()!;
        if (
          update.authority === authority &&
          update.traderPdaIndex === traderPdaIndex
        ) {
          yield update;
        }
      }
    }
  };

  return { push, traderState };
};

describe("createPhoenixTraderStateManager", () => {
  it("bootstraps a trader resource from the HTTP snapshot", async () => {
    const manager = createPhoenixTraderStateManager({
      api: {
        getTraderStateSnapshot: async () => createSnapshotResponse(),
      },
    });

    const resource = manager.resource({
      authority: "authority-1",
      traderPdaIndex: 0,
    });

    const snapshot = await resource.ready();

    expect(snapshot?.authority).toBe("authority-1");
    expect(snapshot?.subaccounts).toHaveLength(2);
    expect(resource.subaccountIndices()).toEqual([0, 1]);
    expect(resource.position(0, "sol-perp")?.basePositionLots).toBe("2");
    expect(resource.marginInputs()?.subaccounts).toHaveLength(2);
    expect(resource.status().health).toBe("bootstrapped");
    expect(resource.status().isConnected).toBe(false);

    manager.close();
  });

  it("applies websocket deltas with structural sharing for untouched subaccounts", async () => {
    const port = createPushPort();
    const manager = createPhoenixTraderStateManager({
      api: {
        getTraderStateSnapshot: async () => createSnapshotResponse(),
      },
      traderState: port.traderState,
    });

    const resource = manager.resource({
      authority: "authority-1",
      traderPdaIndex: 0,
    });
    const release = resource.retain();
    await resource.ready();

    const beforeUntouched = resource.subaccount(1);

    port.push({
      authority: "authority-1",
      traderPdaIndex: 0,
      slot: 11n,
      messageType: "delta",
      version: 1,
      capabilities: CAPABILITIES,
      makerFeeOverrideMultiplier: 1,
      takerFeeOverrideMultiplier: 1,
      subaccounts: [],
      deltas: [
        {
          subaccountIndex: 0,
          sequence: 2,
          collateral: "1250",
          positions: [
            {
              symbol: "SOL-PERP",
              change: "updated",
              position: {
                positionSequenceNumber: "2",
                basePositionLots: "3",
                entryPriceTicks: "105",
                virtualQuotePositionLots: "15",
                unsettledFundingQuoteLots: "0",
                accumulatedFundingQuoteLots: "0",
                takeProfitTriggers: [],
                stopLossTriggers: [],
                conditionalTakeProfitTriggers: [],
                conditionalStopLossTriggers: [],
              },
            },
          ],
          orders: [
            {
              symbol: "SOL-PERP",
              orders: [
                {
                  orderSequenceNumber: "42",
                  side: "bid",
                  orderType: "limit",
                  priceTicks: "101",
                  priceUsd: "101",
                  sizeRemainingLots: "1",
                  initialSizeLots: "1",
                  reduceOnly: false,
                  status: "active",
                },
              ],
            },
          ],
          splines: [],
          triggers: [],
          tradeHistory: [],
          orderHistory: [],
        },
      ],
    });

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(resource.position(0, "SOL-PERP")?.basePositionLots).toBe("3");
    expect(resource.order(0, "SOL-PERP", "42")?.status).toBe("active");
    expect(resource.subaccount(1)).toBe(beforeUntouched);
    expect(resource.messageMeta().lastMessageType).toBe("delta");
    expect(resource.status().health).toBe("live");
    expect(resource.status().isConnected).toBe(true);

    release();
    expect(resource.status().isConnected).toBe(false);
    manager.close();
  });

  it("dedupes trader trade history by fillId when available", async () => {
    const port = createPushPort();
    const manager = createPhoenixTraderStateManager({
      api: {
        getTraderStateSnapshot: async () => createSnapshotResponse(),
      },
      traderState: port.traderState,
    });

    const resource = manager.resource({
      authority: "authority-1",
      traderPdaIndex: 0,
    });
    const release = resource.retain();
    await resource.ready();

    port.push({
      authority: "authority-1",
      traderPdaIndex: 0,
      slot: 11n,
      messageType: "delta",
      version: 1,
      capabilities: CAPABILITIES,
      makerFeeOverrideMultiplier: 1,
      takerFeeOverrideMultiplier: 1,
      subaccounts: [],
      deltas: [
        {
          subaccountIndex: 0,
          sequence: 2,
          collateral: "1000",
          positions: [],
          orders: [],
          splines: [],
          triggers: [],
          tradeHistory: [
            {
              timestamp: 1,
              slot: 10,
              slotIndex: 0,
              instructionIndex: 0,
              eventIndex: 0,
              market: "SOL-PERP",
              instructionType: "Fill",
              tradeType: "limit",
              fillId: "fill-1",
              baseQtyBefore: "0",
              baseQtyAfter: "1",
              size: "1",
              liquidity: "maker",
              price: "10",
              fee: "0.01",
              realizedPnl: "0",
              signature: "sig-1",
            },
          ],
          orderHistory: [],
        },
      ],
    });

    port.push({
      authority: "authority-1",
      traderPdaIndex: 0,
      slot: 12n,
      messageType: "delta",
      version: 1,
      capabilities: CAPABILITIES,
      makerFeeOverrideMultiplier: 1,
      takerFeeOverrideMultiplier: 1,
      subaccounts: [],
      deltas: [
        {
          subaccountIndex: 0,
          sequence: 3,
          collateral: "1000",
          positions: [],
          orders: [],
          splines: [],
          triggers: [],
          tradeHistory: [
            {
              timestamp: 2,
              slot: 999,
              slotIndex: 9,
              instructionIndex: 9,
              eventIndex: 9,
              market: "SOL-PERP",
              instructionType: "Fill",
              tradeType: "limit",
              fillId: "fill-1",
              baseQtyBefore: "1",
              baseQtyAfter: "2",
              size: "1",
              liquidity: "maker",
              price: "11",
              fee: "0.01",
              realizedPnl: "0",
              signature: "sig-2",
            },
          ],
          orderHistory: [],
        },
      ],
    });

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(resource.subaccount(0)?.tradeHistory).toHaveLength(1);
    expect(resource.subaccount(0)?.tradeHistory[0]?.fillId).toBe("fill-1");
    expect(resource.subaccount(0)?.tradeHistory[0]?.signature).toBe("sig-1");

    release();
    manager.close();
  });

  it("notifies store subscribers when bootstrap and delta updates arrive", async () => {
    const port = createPushPort();
    const resourceEvents: Array<{
      health: string;
      lastMessageType: string | null;
      collateral: string | null;
    }> = [];

    const manager = createTraderStateStore({
      api: {
        traders: () => ({
          getTraderStateSnapshot: async () => createSnapshotResponse(),
        }),
      },
      streams: {
        traderState: port.traderState,
      },
    } as never);

    const resource = manager.resource({
      authority: "authority-1",
      traderPdaIndex: 0,
    });

    const unsubscribe = resource.subscribe((state, previous) => {
      if (
        state.lastMessage !== previous.lastMessage ||
        state.snapshot !== previous.snapshot
      ) {
        resourceEvents.push({
          health: state.status.health,
          lastMessageType: state.messageMeta.lastMessageType,
          collateral: state.subaccountsByIndex[0]?.collateral ?? null,
        });
      }
    });

    const release = resource.retain();
    await resource.ready();

    port.push({
      authority: "authority-1",
      traderPdaIndex: 0,
      slot: 11n,
      messageType: "delta",
      version: 1,
      capabilities: CAPABILITIES,
      makerFeeOverrideMultiplier: 1,
      takerFeeOverrideMultiplier: 1,
      subaccounts: [],
      deltas: [
        {
          subaccountIndex: 0,
          sequence: 2,
          collateral: "1250",
          positions: [],
          orders: [],
          splines: [],
          triggers: [],
          tradeHistory: [],
          orderHistory: [],
        },
      ],
    });

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(resourceEvents).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          health: "bootstrapped",
          lastMessageType: "snapshot",
          collateral: "1000",
        }),
        expect.objectContaining({
          health: "live",
          lastMessageType: "delta",
          collateral: "1250",
        }),
      ])
    );

    unsubscribe();
    release();
    manager.close();
  });
});
