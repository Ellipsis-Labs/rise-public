import type {
  PhoenixAccountExistenceClient,
  PhoenixInstructionClient,
  PhoenixMarketDataClient,
} from "@/core/clientTypes";
import { DISCRIMINANTS } from "@/core/discriminants";
import type { PhoenixExchangeMetadata } from "@/exchange-cache/types";
import {
  MarginType,
  Side,
  buildPlaceMultiLimitOrderFlow,
  computeScaleOrderLevels,
} from "@/index";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import { address } from "@solana/kit";
import { describe, expect, it } from "vitest";

/** Solana's hard packet limit for a serialized transaction. */
const LEGACY_TX_SIZE_LIMIT = 1232;

const compactU16Len = (value: number): number =>
  value < 0x80 ? 1 : value < 0x4000 ? 2 : 3;

const serializedIxLen = (accountCount: number, dataLen: number): number =>
  1 + // program id index
  compactU16Len(accountCount) +
  accountCount +
  compactU16Len(dataLen) +
  dataLen;

/**
 * Serialized size of a one-signature legacy transaction carrying `ix` plus the
 * two compute-budget instructions the scale-order callers prepend. Mirrors the
 * wire layout: signatures, message header, account keys, blockhash,
 * instructions. Deliberately assumes no Address Lookup Table — that is the
 * worst case a default-chunked batch has to survive.
 */
const legacyTxSizeWithComputeBudget = (
  ix: InstructionsWithAccountsAndData
): number => {
  const keys = new Set(ix.accounts.map((account) => String(account.address)));
  keys.add("ComputeBudget111111111111111111111111111111");
  return (
    compactU16Len(1) +
    64 + // fee payer signature
    3 + // message header
    compactU16Len(keys.size) +
    32 * keys.size +
    32 + // recent blockhash
    compactU16Len(3) + // instruction count
    serializedIxLen(ix.accounts.length, ix.data.length) +
    serializedIxLen(0, 5) + // SetComputeUnitLimit
    serializedIxLen(0, 9) // SetComputeUnitPrice
  );
};

describe("buildPlaceMultiLimitOrderFlow V2 dispatch", () => {
  const phoenixProgramAddress = address(
    "phDEVv4w6BcfkLrLNeXr8HhhgQxnxziVGXpGPcaadMf"
  );
  const authority = address("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
  const marketAccount = address("9n4nbM75f5Ui33ZbPYXn59EwSgE8CGsHtAeTH5YFeJ9E");
  const rootAuthority = address("So11111111111111111111111111111111111111112");

  const exchangeSnapshot = {
    markets: [{ symbol: "SOL-PERP" }],
    exchange: {
      globalConfig: "global-config",
      currentAuthorities: {
        rootAuthority,
        riskAuthority: rootAuthority,
        marketAuthority: rootAuthority,
        oracleAuthority: rootAuthority,
        adlAuthority: rootAuthority,
        cancelAuthority: rootAuthority,
        backstopAuthority: rootAuthority,
      },
      canonicalMint: "canonical-mint",
      globalVault: "global-vault",
      perpAssetMap: "perp-asset-map",
      globalTraderIndex: ["gti-0"],
      activeTraderBuffer: ["atb-0"],
      withdrawQueue: "withdraw-queue",
      exchangeStatusBits: 0,
    },
  };

  const exchangeMetadata = {
    ready: async () => exchangeSnapshot,
    snapshot: () => exchangeSnapshot,
    instructionContext: (symbol: string) =>
      symbol === "SOL-PERP"
        ? {
            market: {
              marketPubkey: marketAccount,
              assetId: 0,
              tickSize: 0.001,
              baseLotsDecimals: 3,
              splinePubkey: "spline-address",
            },
          }
        : undefined,
  } as unknown as PhoenixExchangeMetadata;

  const client = {
    addresses: {
      phoenixProgramAddress,
      logAuthorityAddress: "log-authority",
      globalConfigurationAddress: "global-config",
    },
    fetchAccount: async () => ({ data: new Uint8Array() }),
    accountExists: async () => true,
    exchange: exchangeMetadata,
  } as unknown as PhoenixInstructionClient &
    PhoenixMarketDataClient &
    PhoenixAccountExistenceClient;

  const levels = computeScaleOrderLevels({
    lowerPrice: 100,
    upperPrice: 110,
    orderCount: 4,
    totalBaseLots: 1000,
    priceBias: 0,
    sizeBias: 0,
    baseLotsDecimals: 3,
    tickSize: 1,
  });

  it("emits the V1 discriminant when no scaleSetId or reduceOnly is set (live frontend guard)", async () => {
    const result = await buildPlaceMultiLimitOrderFlow(
      {
        authority: authority as never,
        symbol: "SOL-PERP" as never,
        side: Side.Bid,
        levels,
        marginType: MarginType.Cross,
        subaccountIndex: 0,
        slide: false,
      },
      client
    );

    expect(result.batches).toHaveLength(1);
    const data = result.batches[0]?.named.placeMultiLimitOrder.data;
    expect(Array.from(data?.slice(0, 8) ?? [])).toEqual(
      Array.from(DISCRIMINANTS.PLACE_MULTI_LIMIT_ORDER)
    );
  });

  it("emits the V2 discriminant when scaleSetId is set", async () => {
    const result = await buildPlaceMultiLimitOrderFlow(
      {
        authority: authority as never,
        symbol: "SOL-PERP" as never,
        side: Side.Bid,
        levels,
        marginType: MarginType.Cross,
        subaccountIndex: 0,
        scaleSetId: 3,
      },
      client
    );

    const data = result.batches[0]?.named.placeMultiLimitOrder.data;
    expect(Array.from(data?.slice(0, 8) ?? [])).toEqual(
      Array.from(DISCRIMINANTS.PLACE_MULTI_LIMIT_ORDER_V2)
    );
  });

  it("emits the V2 discriminant when reduceOnly is set even without a scaleSetId", async () => {
    const result = await buildPlaceMultiLimitOrderFlow(
      {
        authority: authority as never,
        symbol: "SOL-PERP" as never,
        side: Side.Bid,
        levels,
        marginType: MarginType.Cross,
        subaccountIndex: 0,
        reduceOnly: true,
      },
      client
    );

    const data = result.batches[0]?.named.placeMultiLimitOrder.data;
    expect(Array.from(data?.slice(0, 8) ?? [])).toEqual(
      Array.from(DISCRIMINANTS.PLACE_MULTI_LIMIT_ORDER_V2)
    );
  });

  const bigLevels = computeScaleOrderLevels({
    lowerPrice: 100,
    upperPrice: 200,
    orderCount: 64,
    totalBaseLots: 100000,
    priceBias: 0,
    sizeBias: 0,
    baseLotsDecimals: 3,
    tickSize: 1,
  });

  it("keeps a default-chunked V2 batch inside the legacy transaction limit", async () => {
    const result = await buildPlaceMultiLimitOrderFlow(
      {
        authority: authority as never,
        symbol: "SOL-PERP" as never,
        side: Side.Bid,
        levels: bigLevels,
        marginType: MarginType.Cross,
        subaccountIndex: 0,
        // No scaleSetId: reduceOnly still routes through V2 but may span
        // transactions, so this exercises the default chunk size directly.
        reduceOnly: true,
        clientOrderId: 7n,
      },
      client
    );

    expect(result.batches.length).toBeGreaterThan(1);
    for (const batch of result.batches) {
      expect(
        legacyTxSizeWithComputeBudget(batch.named.placeMultiLimitOrder)
      ).toBeLessThanOrEqual(LEGACY_TX_SIZE_LIMIT);
    }
  });

  it("throws when a scaleSetId is set but the ladder spans more than one transaction", async () => {
    await expect(
      buildPlaceMultiLimitOrderFlow(
        {
          authority: authority as never,
          symbol: "SOL-PERP" as never,
          side: Side.Bid,
          levels: bigLevels,
          marginType: MarginType.Cross,
          subaccountIndex: 0,
          scaleSetId: 5,
          maxOrdersPerTx: 30,
        },
        client
      )
    ).rejects.toThrow(/scaleSetId cannot span transactions/);
  });

  it("rejects an invalid scaleSetId before the span guard can fire", async () => {
    await expect(
      buildPlaceMultiLimitOrderFlow(
        {
          authority: authority as never,
          symbol: "SOL-PERP" as never,
          side: Side.Bid,
          levels: bigLevels,
          marginType: MarginType.Cross,
          subaccountIndex: 0,
          scaleSetId: Number.NaN,
          maxOrdersPerTx: 30,
        },
        client
      )
    ).rejects.toThrow(/scaleSetId must be an integer in 0\.\.=255/);
    await expect(
      buildPlaceMultiLimitOrderFlow(
        {
          authority: authority as never,
          symbol: "SOL-PERP" as never,
          side: Side.Bid,
          levels,
          marginType: MarginType.Cross,
          subaccountIndex: 0,
          scaleSetId: 0.5,
        },
        client
      )
    ).rejects.toThrow(/scaleSetId must be an integer in 0\.\.=255/);
  });
});
