import {
  HAWKEYE_DISCRIMINANTS,
  HAWKEYE_PROGRAM_ADDRESS,
  OrderFlags,
  SelfTradeBehavior,
  Side,
  baseLots,
  buildHawkeyeViewOrderQuoteIx,
  decodeHawkeyeReturnData,
  encodeHawkeyeSimulationTransaction,
  getConditionalOrderPacketCodec,
  getHawkeyeOrderQuoteNetQuoteLots,
  isHawkeyeOrderQuoteFullyFilled,
  getOrderPacketCodec,
  getPlaceLimitOrderEncoder,
  getPlaceMarketOrderEncoder,
  getPlacePostOnlyOrderEncoder,
  quoteLots,
  ticks,
  type HawkeyeOrderQuoteAccounts,
  type OrderPacket,
} from "@/index";
import {
  AccountRole,
  blockhash,
  getAddressDecoder,
  getBase64Encoder,
  getCompiledTransactionMessageDecoder,
  getTransactionDecoder,
} from "@solana/kit";
import { describe, expect, it } from "vitest";

const addresses = Array.from({ length: 11 }, (_, index) =>
  getAddressDecoder().decode(new Uint8Array(32).fill(index + 1))
);
// Apply the account-role brands to distinct, valid fixture addresses.
const accounts = {
  phoenixProgramAddress: addresses[0],
  globalConfigurationAddress: addresses[1],
  globalTraderIndex: [addresses[2], addresses[3]],
  activeTraderBuffer: [addresses[4], addresses[5]],
  perpAssetMap: addresses[6],
  orderbook: addresses[7],
  splineCollection: addresses[8],
  traderAccount: addresses[9],
  scratch: addresses[10],
} as HawkeyeOrderQuoteAccounts;

const commonPacket = {
  side: Side.Ask,
  priceInTicks: ticks(123n),
  numBaseLots: baseLots(456n),
  clientOrderId: 0x112233445566778899aabbccddeeff00n,
  lastValidSlot: 789n,
  orderFlags: OrderFlags.ReduceOnly,
  cancelExisting: false,
};
const postOnly = { ...commonPacket, slide: true };
const limit = {
  ...commonPacket,
  selfTradeBehavior: SelfTradeBehavior.CancelProvide,
  matchLimit: 12n,
};
const market = {
  ...limit,
  priceInTicks: null,
  numQuoteLots: quoteLots(999n),
  minBaseLotsToFill: baseLots(1n),
  minQuoteLotsToFill: quoteLots(2n),
};

const cases: OrderPacket[] = [
  { __kind: "PostOnly", ...postOnly },
  { __kind: "Limit", ...limit },
  { __kind: "ImmediateOrCancel", ...market },
];
const placementBytes = [
  getPlacePostOnlyOrderEncoder().encode(postOnly),
  getPlaceLimitOrderEncoder().encode(limit),
  getPlaceMarketOrderEncoder().encode(market),
];

// Same return-data vector as Rust's hawkeye::tests::decodes_order_quote_parity_vector.
const returnBytes = Uint8Array.from([
  136, 111, 182, 92, 28, 207, 41, 21, 1, 0, 7, 0, 7, 0, 0, 0, 123, 0, 0, 0, 0,
  0, 0, 0, 20, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 232, 3, 0, 0, 0, 0,
  0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0,
  100, 0, 0, 0, 0, 0, 0, 0, 101, 0, 0, 0, 0, 0, 0, 0, 157, 255, 255, 255, 255,
  255, 255, 255, 20, 0, 0, 0, 0, 0, 0, 0, 99, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0,
]);

describe("Hawkeye order quotes", () => {
  it.each(cases)("encodes the existing $__kind order packet", (orderPacket) => {
    const ix = buildHawkeyeViewOrderQuoteIx({
      ...accounts,
      orderPacket,
      referencePriceTicks: 0x0102030405060708n,
    });
    expect(ix.programAddress).toBe(HAWKEYE_PROGRAM_ADDRESS);
    expect(Array.from(ix.data)).toEqual([
      ...HAWKEYE_DISCRIMINANTS.VIEW_ORDER_QUOTE,
      8,
      7,
      6,
      5,
      4,
      3,
      2,
      1,
      ...placementBytes[cases.indexOf(orderPacket)].slice(8),
    ]);
    const packetBytes = ix.data.slice(16);
    expect(getOrderPacketCodec().decode(packetBytes)).toEqual(orderPacket);
    expect(getConditionalOrderPacketCodec().decode(packetBytes)).toEqual(
      orderPacket
    );
    expect(getConditionalOrderPacketCodec().encode(orderPacket)).toEqual(
      packetBytes
    );
    expect(ix.accounts).toEqual(
      addresses.map((address, index) => ({
        address,
        role:
          index === addresses.length - 1
            ? AccountRole.WRITABLE
            : AccountRole.READONLY,
      }))
    );
  });

  it("uses a normal market packet with absent optional fields", () => {
    const packet = {
      ...market,
      numQuoteLots: null,
      matchLimit: null,
      lastValidSlot: null,
    };
    const orderPacket: OrderPacket = {
      __kind: "ImmediateOrCancel",
      ...packet,
    };
    const ix = buildHawkeyeViewOrderQuoteIx({ ...accounts, orderPacket });
    expect(ix.data.slice(16)).toEqual(
      getPlaceMarketOrderEncoder().encode(packet).slice(8)
    );
    expect(getOrderPacketCodec().decode(ix.data.slice(16))).toEqual(
      orderPacket
    );
  });

  it("defaults the reference to zero and rejects out-of-range references", () => {
    const params = { ...accounts, orderPacket: cases[0] };
    expect(
      Array.from(buildHawkeyeViewOrderQuoteIx(params).data.slice(8, 16))
    ).toEqual(Array(8).fill(0));
    for (const referencePriceTicks of [-1n, 1n << 64n]) {
      expect(() =>
        buildHawkeyeViewOrderQuoteIx({ ...params, referencePriceTicks })
      ).toThrow();
    }
  });

  it("includes preparation instructions before the quote in simulation", () => {
    const instruction = buildHawkeyeViewOrderQuoteIx({
      ...accounts,
      orderPacket: cases[0],
    });
    const preparation = {
      programAddress: addresses[0],
      accounts: [],
      data: Uint8Array.of(42),
    };
    const wire = encodeHawkeyeSimulationTransaction({
      instruction,
      preInstructions: [preparation],
      blockhash: blockhash(addresses[0]),
      lastValidBlockHeight: 123n,
    });
    const transaction = getTransactionDecoder().decode(
      getBase64Encoder().encode(wire)
    );
    const message = getCompiledTransactionMessageDecoder().decode(
      transaction.messageBytes
    );
    expect(message.instructions).toHaveLength(3);
    expect(message.instructions[1].data).toEqual(preparation.data);
    expect(message.instructions[2].data).toEqual(instruction.data);
    expect(
      message.staticAccounts[message.instructions[2].programAddressIndex]
    ).toBe(HAWKEYE_PROGRAM_ADDRESS);
  });

  it("decodes the Rust return layout from an unaligned slice", () => {
    const bytes = new Uint8Array(121);
    bytes.set(returnBytes, 1);
    expect(decodeHawkeyeReturnData(bytes.subarray(1))).toEqual({
      kind: "view_order_quote",
      magic: { decimal: "1524977669563117448", hex: "0x1529cf1c5cb66f88" },
      version: 1,
      flags: 7,
      side: { code: 0, label: "bid" },
      assetId: 7,
      slot: 123n,
      requestedBaseLots: 20n,
      filledBaseLots: 10n,
      filledQuoteLots: 1_000n,
      feeQuoteLots: 2n,
      postedBaseLots: 3n,
      unfilledBaseLots: 10n,
      averagePriceQuoteLotsPerBaseLot: 100n,
      referencePriceTicks: 101n,
      slippageBps: -99n,
      effectiveBaseLots: 20n,
      postedPriceTicks: 99n,
      outcome: { code: 0, label: "accepted" },
      rejectionReason: { code: 0, label: "none" },
    });
  });

  it("respects absent price/slippage flags and decodes asks", () => {
    const bytes = returnBytes.slice();
    bytes[10] = 0;
    bytes[11] = 1;
    expect(decodeHawkeyeReturnData(bytes)).toMatchObject({
      side: { code: 1, label: "ask" },
      averagePriceQuoteLotsPerBaseLot: null,
      slippageBps: null,
    });
    bytes[10] = 1;
    expect(decodeHawkeyeReturnData(bytes)).toMatchObject({
      averagePriceQuoteLotsPerBaseLot: 100n,
      slippageBps: null,
    });
  });

  it("derives fill completion and net quote flow without losing integer precision", () => {
    const quote = decodeHawkeyeReturnData(returnBytes);
    if (quote.kind !== "view_order_quote") throw new Error("Wrong return kind");
    expect(isHawkeyeOrderQuoteFullyFilled(quote)).toBe(false);
    expect(getHawkeyeOrderQuoteNetQuoteLots(quote)).toBe(-1_002n);
    expect(
      getHawkeyeOrderQuoteNetQuoteLots({
        ...quote,
        side: { code: 1, label: "ask" },
      })
    ).toBe(998n);
    expect(
      isHawkeyeOrderQuoteFullyFilled({ ...quote, requestedBaseLots: 10n })
    ).toBe(true);
    const rejected = {
      ...quote,
      outcome: { code: 1, label: "rejected" as const },
    };
    expect(getHawkeyeOrderQuoteNetQuoteLots(rejected)).toBe(0n);
    expect(isHawkeyeOrderQuoteFullyFilled(rejected)).toBe(false);
    const unknown = {
      ...quote,
      outcome: { code: 255, label: "unknown" as const },
    };
    expect(getHawkeyeOrderQuoteNetQuoteLots(unknown)).toBeNull();
    expect(isHawkeyeOrderQuoteFullyFilled(unknown)).toBe(false);
    expect(
      getHawkeyeOrderQuoteNetQuoteLots({
        ...quote,
        filledQuoteLots: (1n << 64n) - 1n,
      })
    ).toBe(-(1n << 64n) - 1n);
  });

  it("decodes stable rejection codes, unknown codes and absent optional quantities", () => {
    const bytes = returnBytes.slice();
    bytes[10] = 0;
    bytes.fill(0, 56, 64);
    bytes[112] = 1;
    bytes[113] = 3;
    expect(decodeHawkeyeReturnData(bytes)).toMatchObject({
      outcome: { code: 1, label: "rejected" },
      rejectionReason: { code: 3, label: "minimum_fill_not_met" },
      effectiveBaseLots: null,
      postedPriceTicks: null,
    });
    bytes[112] = 255;
    bytes[113] = 255;
    expect(decodeHawkeyeReturnData(bytes)).toMatchObject({
      outcome: { code: 255, label: "unknown" },
      rejectionReason: { code: 255, label: "unknown" },
    });
  });

  it("rejects truncated or oversized return data", () => {
    expect(() => decodeHawkeyeReturnData(returnBytes.subarray(0, 119))).toThrow(
      "expected 120 bytes"
    );
    expect(() =>
      decodeHawkeyeReturnData(new Uint8Array([...returnBytes, 0]))
    ).toThrow("expected 120 bytes");
  });
});
