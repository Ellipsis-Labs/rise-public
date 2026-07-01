import {
  ACCOUNT_DISCRIMINANTS,
  decodeConditionalOrderCollection,
  decodeMint,
  decodeGlobalConfiguration,
  decodeOrderbook,
  decodeOrderbookHeader,
  decodePerpAssetMap,
  decodePermission,
  decodeSplineCollection,
  decodeStopLosses,
  decodeTokenAccount,
  decodeTrader,
  decodeWithdrawQueueHeader,
  Direction,
  Side,
  StopLossOrderKind,
  TokenAccountState,
  TraderPreferenceKind,
  flight,
} from "@/index";
import { getAddressDecoder } from "@solana/kit";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const TEST_TIMEOUT_MS = 120_000;
const TESTS_DIR = dirname(fileURLToPath(import.meta.url));
const MOCKS_DIR = resolve(TESTS_DIR, "mocks");
const ACCOUNT_PARSING_FIXTURES_PATH = resolve(
  TESTS_DIR,
  "fixtures",
  "account-parsing-rust.json"
);
const TRADER_MAX_POSITIONS_OFFSET = 112;
const TRADER_PREFERENCE_BITS_OFFSET = 116;
const TRADER_POSITION_MAP_LEN_OFFSET = 224;
const TRADER_POSITION_ENTRIES_OFFSET = 240;
const TRADER_POSITION_ENTRY_BYTES = 40;
const HEADER_EXTENSION_ASSET_ID = 0xffff0000;
const COLD_ONLY_DISCRIMINANT = 1;

type FixtureFile = {
  account: {
    data: [string, string];
  };
};

let rustFixturesCache: Record<string, unknown> | undefined;

const addressFromFill = (fill: number): string =>
  getAddressDecoder().decode(new Uint8Array(32).fill(fill));

const writeU32LE = (bytes: Uint8Array, offset: number, value: number): void => {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  view.setUint32(offset, value, true);
};

const writeI32LE = (bytes: Uint8Array, offset: number, value: number): void => {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  view.setInt32(offset, value, true);
};

const writeU64LE = (bytes: Uint8Array, offset: number, value: bigint): void => {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  view.setBigUint64(offset, value, true);
};

const writeI64LE = (bytes: Uint8Array, offset: number, value: bigint): void => {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  view.setBigInt64(offset, value, true);
};

const writeAddress = (
  bytes: Uint8Array,
  offset: number,
  fill: number
): void => {
  bytes.fill(fill, offset, offset + 32);
};

const createMintBytes = (): Uint8Array => {
  const bytes = new Uint8Array(82);
  writeU32LE(bytes, 0, 1);
  writeAddress(bytes, 4, 7);
  writeU64LE(bytes, 36, 1_234_567n);
  bytes[44] = 6;
  bytes[45] = 1;
  writeU32LE(bytes, 46, 1);
  writeAddress(bytes, 50, 9);
  return bytes;
};

const createTokenAccountBytes = (): Uint8Array => {
  const bytes = new Uint8Array(165);
  writeAddress(bytes, 0, 3);
  writeAddress(bytes, 32, 4);
  writeU64LE(bytes, 64, 55n);
  writeU32LE(bytes, 72, 1);
  writeAddress(bytes, 76, 5);
  bytes[108] = TokenAccountState.Frozen;
  writeU32LE(bytes, 109, 1);
  writeU64LE(bytes, 113, 88n);
  writeU64LE(bytes, 121, 13n);
  writeU32LE(bytes, 129, 1);
  writeAddress(bytes, 133, 6);
  return bytes;
};

const createPermissionAccountBytes = (): Uint8Array => {
  const bytes = new Uint8Array(168);
  bytes.set([180, 172, 109, 215, 129, 165, 97, 38], 0);
  writeAddress(bytes, 8, 7);
  writeAddress(bytes, 40, 9);
  bytes[72] = 254;
  writeU64LE(bytes, 80, 16n);
  writeI64LE(bytes, 88, 1_775_000_000n);
  writeI64LE(bytes, 96, -1n);
  return bytes;
};

const writeStopLoss = (
  bytes: Uint8Array,
  offset: number,
  config: {
    sequenceNumber: bigint;
    triggerPrice: bigint;
    executionPrice: bigint;
    tradeSize: bigint;
    slot: bigint;
    positionSequenceNumber: number;
    flags: number;
  }
): void => {
  writeU64LE(bytes, offset, config.sequenceNumber);
  writeU64LE(bytes, offset + 8, config.triggerPrice);
  writeU64LE(bytes, offset + 16, config.executionPrice);
  writeU64LE(bytes, offset + 24, config.tradeSize);
  writeU64LE(bytes, offset + 32, config.slot);
  bytes[offset + 40] = config.positionSequenceNumber;
  bytes[offset + 41] = config.flags;
};

const writeConditionalOrderTrigger = (
  bytes: Uint8Array,
  offset: number,
  config: {
    triggerPrice: bigint;
    executionPrice: bigint;
    positionSequenceNumber: number;
    flags: number;
  }
): void => {
  writeU64LE(bytes, offset, config.triggerPrice);
  writeU64LE(bytes, offset + 8, config.executionPrice);
  bytes[offset + 16] = config.positionSequenceNumber;
  bytes[offset + 17] = config.flags;
};

const createConditionalOrderCollectionBytes = (): Uint8Array => {
  const headerSize = 96;
  const orderSize = 112;
  const capacity = 4;
  const bytes = new Uint8Array(headerSize + orderSize * capacity);
  bytes.set(ACCOUNT_DISCRIMINANTS.CONDITIONAL_ORDER_COLLECTION, 0);

  writeAddress(bytes, 8, 7);
  writeAddress(bytes, 40, 9);
  writeU64LE(bytes, 72, 11n);
  writeU64LE(bytes, 80, 12n);
  bytes[88] = 2;
  bytes[89] = capacity;

  const firstOrderOffset = headerSize + orderSize;
  writeU64LE(bytes, firstOrderOffset, 21n);
  writeU64LE(bytes, firstOrderOffset + 24, 100n);
  writeU64LE(bytes, firstOrderOffset + 32, 90n);
  writeU64LE(bytes, firstOrderOffset + 40, 10n);
  writeU64LE(bytes, firstOrderOffset + 48, 700n);
  writeU32LE(bytes, firstOrderOffset + 56, 5);
  bytes[firstOrderOffset + 60] = 0x01;
  bytes[firstOrderOffset + 61] = 25;
  writeConditionalOrderTrigger(bytes, firstOrderOffset + 64, {
    triggerPrice: 101n,
    executionPrice: 102n,
    positionSequenceNumber: 3,
    flags: 0x0f,
  });

  const secondOrderOffset = headerSize + 2 * orderSize;
  writeU64LE(bytes, secondOrderOffset, 22n);
  writeU64LE(bytes, secondOrderOffset + 8, 33n);
  writeU64LE(bytes, secondOrderOffset + 16, 44n);
  writeU64LE(bytes, secondOrderOffset + 24, 200n);
  writeU64LE(bytes, secondOrderOffset + 32, 150n);
  writeU64LE(bytes, secondOrderOffset + 40, 50n);
  writeU64LE(bytes, secondOrderOffset + 48, 701n);
  writeU32LE(bytes, secondOrderOffset + 56, 6);
  writeConditionalOrderTrigger(bytes, secondOrderOffset + 64, {
    triggerPrice: 201n,
    executionPrice: 202n,
    positionSequenceNumber: 4,
    flags: 0x03,
  });
  writeConditionalOrderTrigger(bytes, secondOrderOffset + 88, {
    triggerPrice: 301n,
    executionPrice: 302n,
    positionSequenceNumber: 4,
    flags: 0x0d,
  });

  return bytes;
};

const createStopLossesBytes = (): Uint8Array => {
  const bytes = new Uint8Array(328);
  bytes.set(ACCOUNT_DISCRIMINANTS.STOP_LOSSES, 0);

  writeStopLoss(bytes, 8, {
    sequenceNumber: 11n,
    triggerPrice: 101n,
    executionPrice: 111n,
    tradeSize: 7n,
    slot: 500n,
    positionSequenceNumber: 4,
    flags: 0x0f,
  });

  writeStopLoss(bytes, 88, {
    sequenceNumber: 12n,
    triggerPrice: 202n,
    executionPrice: 0n,
    tradeSize: 9n,
    slot: 600n,
    positionSequenceNumber: 5,
    flags: 0x00,
  });

  writeU64LE(bytes, 168, 1n);
  writeU64LE(bytes, 176, 91n);
  writeU64LE(bytes, 184, 92n);
  writeAddress(bytes, 192, 17);
  writeAddress(bytes, 224, 18);
  writeU64LE(bytes, 256, 19n);

  return bytes;
};

const createWithdrawQueueHeaderBytes = (): Uint8Array => {
  const bytes = new Uint8Array(120);
  bytes.set(ACCOUNT_DISCRIMINANTS.WITHDRAW_QUEUE_HEADER, 0);
  writeU64LE(bytes, 8, 201n);
  writeU64LE(bytes, 16, 202n);
  writeU64LE(bytes, 24, 1_000n);
  writeU64LE(bytes, 32, 900n);
  writeU64LE(bytes, 40, 50n);
  writeU64LE(bytes, 48, 203n);
  writeU64LE(bytes, 56, 777n);
  writeU64LE(bytes, 64, 12n);
  writeU64LE(bytes, 72, 3n);
  return bytes;
};

const loadMockBytes = (fileName: string): Uint8Array => {
  const fixture = JSON.parse(
    readFileSync(resolve(MOCKS_DIR, fileName), "utf8")
  ) as FixtureFile;
  return Buffer.from(fixture.account.data[0], "base64");
};

const normalize = (value: unknown): unknown => {
  if (typeof value === "bigint") {
    return value.toString();
  }

  if (Array.isArray(value)) {
    return value.map((item) => normalize(item));
  }

  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value as Record<string, unknown>).map(([key, nested]) => [
        key,
        normalize(nested),
      ])
    );
  }

  return value;
};

const getRustFixtures = (): Record<string, unknown> => {
  if (rustFixturesCache) {
    return rustFixturesCache;
  }

  rustFixturesCache = JSON.parse(
    readFileSync(ACCOUNT_PARSING_FIXTURES_PATH, "utf8")
  ) as Record<string, unknown>;
  return rustFixturesCache;
};

describe("raw account parsing", () => {
  it(
    "decodes GlobalConfiguration fixtures",
    () => {
      const decoded = normalize(
        decodeGlobalConfiguration(loadMockBytes("global_config.json"))
      );

      expect(decoded).toEqual(getRustFixtures().globalConfiguration);
      expect(decoded).toMatchSnapshot();
    },
    TEST_TIMEOUT_MS
  );

  it(
    "decodes PerpAssetMap fixtures",
    () => {
      const decoded = normalize(
        decodePerpAssetMap(loadMockBytes("perp_asset_map.json"))
      );

      expect(decoded).toEqual(getRustFixtures().perpAssetMap);
      expect(decoded).toMatchSnapshot();
    },
    TEST_TIMEOUT_MS
  );

  it(
    "decodes Trader fixtures",
    () => {
      const decoded = normalize(decodeTrader(loadMockBytes("trader.json")));

      expect(decoded).toEqual(getRustFixtures().trader);
      expect(decoded).toMatchSnapshot();
    },
    TEST_TIMEOUT_MS
  );

  it("decodes Trader preference bits separately from max positions", () => {
    const bytes = loadMockBytes("trader.json");
    writeU32LE(bytes, TRADER_MAX_POSITIONS_OFFSET, 128);
    writeU32LE(bytes, TRADER_PREFERENCE_BITS_OFFSET, 1);

    const decoded = decodeTrader(bytes);

    expect(decoded.maxPositions).toBe(128);
    expect(decoded.traderPreferenceBits).toBe(1);
    expect(decoded.traderPreferenceFlags.enabled).toEqual([
      TraderPreferenceKind.DisableCollateralSweep,
    ]);
    expect(decoded.preferences.disableCollateralSweep).toBe(true);
    expect(decoded.disableCollateralSweep).toBe(true);
  });

  it("decodes only position entries from Trader position map", () => {
    const bytes = loadMockBytes("trader.json");
    writeU64LE(bytes, TRADER_POSITION_MAP_LEN_OFFSET, 3n);
    const secondEntryOffset =
      TRADER_POSITION_ENTRIES_OFFSET + TRADER_POSITION_ENTRY_BYTES;
    writeU32LE(bytes, secondEntryOffset, HEADER_EXTENSION_ASSET_ID);
    writeU32LE(bytes, secondEntryOffset + 4, 0);
    const thirdEntryOffset = secondEntryOffset + TRADER_POSITION_ENTRY_BYTES;
    writeU32LE(bytes, thirdEntryOffset, 12);
    writeU32LE(bytes, thirdEntryOffset + 4, COLD_ONLY_DISCRIMINANT);

    const decoded = decodeTrader(bytes);

    expect(decoded.positions.len).toBe(2n);
    expect(decoded.positions.entries).toHaveLength(2);
    expect(decoded.positions.entries[0]?.key).toBe(11n);
    expect(decoded.positions.entries[1]?.key).toBe(12n);
  });

  it(
    "decodes Orderbook fixtures",
    () => {
      const decoded = normalize(
        decodeOrderbook(loadMockBytes("orderbook.json"))
      );

      expect(decoded).toEqual(getRustFixtures().orderbook);
      expect(decoded).toMatchSnapshot();
    },
    TEST_TIMEOUT_MS
  );

  it(
    "decodes SplineCollection fixtures",
    () => {
      const decoded = normalize(
        decodeSplineCollection(loadMockBytes("splines.json"))
      );

      expect(decoded).toEqual(getRustFixtures().splineCollection);
      expect(decoded).toMatchSnapshot();
    },
    TEST_TIMEOUT_MS
  );

  it("decodes OrderbookHeader from the orderbook account prefix", () => {
    const bytes = loadMockBytes("orderbook.json");

    expect(normalize(decodeOrderbookHeader(bytes))).toEqual(
      normalize(decodeOrderbook(bytes).header)
    );
  });

  it("decodes Mint accounts", () => {
    expect(decodeMint(createMintBytes())).toEqual({
      mintAuthorityOption: 1,
      mintAuthority: addressFromFill(7),
      supply: 1_234_567n,
      decimals: 6,
      isInitialized: true,
      freezeAuthorityOption: 1,
      freezeAuthority: addressFromFill(9),
    });
  });

  it("decodes TokenAccount accounts", () => {
    expect(decodeTokenAccount(createTokenAccountBytes())).toEqual({
      mint: addressFromFill(3),
      owner: addressFromFill(4),
      amount: 55n,
      delegateOption: 1,
      delegate: addressFromFill(5),
      state: TokenAccountState.Frozen,
      isNativeOption: 1,
      isNative: 88n,
      delegatedAmount: 13n,
      closeAuthorityOption: 1,
      closeAuthority: addressFromFill(6),
    });
  });

  it("decodes PermissionAccount accounts", () => {
    expect(decodePermission(createPermissionAccountBytes())).toEqual({
      permissionAuthority: addressFromFill(7),
      delegatedKey: addressFromFill(9),
      permission: 16n,
      expiresAtTimestamp: 1_775_000_000n,
      allowedSignerActions: -1n,
    });
  });

  it("decodes ConditionalOrderCollection accounts", () => {
    expect(
      decodeConditionalOrderCollection(createConditionalOrderCollectionBytes())
    ).toEqual({
      header: {
        traderKey: addressFromFill(7),
        fundingKey: addressFromFill(9),
        sequenceNumber: {
          sequenceNumber: 11n,
          lastUpdateSlot: 12n,
        },
        len: 2,
        capacity: 4,
      },
      orders: [
        {
          sequenceNumber: 0n,
          orderId: null,
          maxSize: 0n,
          fillableSize: 0n,
          filledSize: 0n,
          slot: 0n,
          assetId: 0,
          usePercent: false,
          percent: 0,
          greaterTriggerOrder: {
            triggerPrice: 0n,
            executionPrice: 0n,
            positionSequenceNumber: 0,
            isActive: false,
            executionDirection: Direction.LessThan,
            tradeSide: Side.Ask,
            orderKind: StopLossOrderKind.IOC,
          },
          lessTriggerOrder: {
            triggerPrice: 0n,
            executionPrice: 0n,
            positionSequenceNumber: 0,
            isActive: false,
            executionDirection: Direction.LessThan,
            tradeSide: Side.Ask,
            orderKind: StopLossOrderKind.IOC,
          },
          isActive: false,
        },
        {
          sequenceNumber: 21n,
          orderId: null,
          maxSize: 100n,
          fillableSize: 90n,
          filledSize: 10n,
          slot: 700n,
          assetId: 5,
          usePercent: true,
          percent: 25,
          greaterTriggerOrder: {
            triggerPrice: 101n,
            executionPrice: 102n,
            positionSequenceNumber: 3,
            isActive: true,
            executionDirection: Direction.GreaterThan,
            tradeSide: Side.Bid,
            orderKind: StopLossOrderKind.Limit,
          },
          lessTriggerOrder: {
            triggerPrice: 0n,
            executionPrice: 0n,
            positionSequenceNumber: 0,
            isActive: false,
            executionDirection: Direction.LessThan,
            tradeSide: Side.Ask,
            orderKind: StopLossOrderKind.IOC,
          },
          isActive: true,
        },
        {
          sequenceNumber: 22n,
          orderId: {
            priceInTicks: 33n,
            orderSequenceNumber: 44n,
          },
          maxSize: 200n,
          fillableSize: 150n,
          filledSize: 50n,
          slot: 701n,
          assetId: 6,
          usePercent: false,
          percent: 0,
          greaterTriggerOrder: {
            triggerPrice: 201n,
            executionPrice: 202n,
            positionSequenceNumber: 4,
            isActive: true,
            executionDirection: Direction.GreaterThan,
            tradeSide: Side.Ask,
            orderKind: StopLossOrderKind.IOC,
          },
          lessTriggerOrder: {
            triggerPrice: 301n,
            executionPrice: 302n,
            positionSequenceNumber: 4,
            isActive: true,
            executionDirection: Direction.LessThan,
            tradeSide: Side.Bid,
            orderKind: StopLossOrderKind.Limit,
          },
          isActive: true,
        },
        {
          sequenceNumber: 0n,
          orderId: null,
          maxSize: 0n,
          fillableSize: 0n,
          filledSize: 0n,
          slot: 0n,
          assetId: 0,
          usePercent: false,
          percent: 0,
          greaterTriggerOrder: {
            triggerPrice: 0n,
            executionPrice: 0n,
            positionSequenceNumber: 0,
            isActive: false,
            executionDirection: Direction.LessThan,
            tradeSide: Side.Ask,
            orderKind: StopLossOrderKind.IOC,
          },
          lessTriggerOrder: {
            triggerPrice: 0n,
            executionPrice: 0n,
            positionSequenceNumber: 0,
            isActive: false,
            executionDirection: Direction.LessThan,
            tradeSide: Side.Ask,
            orderKind: StopLossOrderKind.IOC,
          },
          isActive: false,
        },
      ],
      activeOrderIndices: [1, 2],
    });
  });

  it("tolerates stale ConditionalOrderCollection header len values", () => {
    const bytes = createConditionalOrderCollectionBytes();
    bytes[88] = 0;

    expect(decodeConditionalOrderCollection(bytes)).toMatchObject({
      header: {
        len: 0,
        capacity: 4,
      },
      activeOrderIndices: [1, 2],
    });
  });

  it("decodes StopLosses accounts", () => {
    expect(decodeStopLosses(createStopLossesBytes())).toEqual({
      stopLosses: [
        {
          sequenceNumber: 11n,
          triggerPrice: 101n,
          executionPrice: 111n,
          tradeSize: 7n,
          slot: 500n,
          positionSequenceNumber: 4,
          isActive: true,
          executionDirection: Direction.GreaterThan,
          tradeSide: Side.Bid,
          orderKind: StopLossOrderKind.Limit,
        },
        {
          sequenceNumber: 12n,
          triggerPrice: 202n,
          executionPrice: 0n,
          tradeSize: 9n,
          slot: 600n,
          positionSequenceNumber: 5,
          isActive: false,
          executionDirection: Direction.LessThan,
          tradeSide: Side.Ask,
          orderKind: StopLossOrderKind.IOC,
        },
      ],
      isInitialized: true,
      sequenceNumber: {
        sequenceNumber: 91n,
        lastUpdateSlot: 92n,
      },
      fundingKey: addressFromFill(17),
      traderKey: addressFromFill(18),
      assetId: 19n,
    });
  });

  it("decodes WithdrawQueueHeader accounts", () => {
    expect(decodeWithdrawQueueHeader(createWithdrawQueueHeaderBytes())).toEqual(
      {
        sequenceNumber: {
          sequenceNumber: 201n,
          lastUpdateSlot: 202n,
        },
        withdrawThrottle: {
          maxBudget: 1_000n,
          remainingBudget: 900n,
          replenishAmountPerSlot: 50n,
          lastUpdateSlot: 203n,
        },
        totalQueuedAmount: 777n,
        withdrawalFee: 12n,
        enqueuingFee: 3n,
      }
    );
  });

  it("decodes Flight GlobalState from the beta on-chain fixture", () => {
    expect(
      normalize(
        flight.decodeGlobalState(loadMockBytes("flight_global_state.json"))
      )
    ).toEqual({
      discriminant: "12113240955749314245",
      programId: "phDEVv4w6BcfkLrLNeXr8HhhgQxnxziVGXpGPcaadMf",
      currentAuthority: "8fGW86sH2sSbH3bqsFnyCa2paBY3Fu5HxLcdJGi89KL7",
      pendingAuthority: "11111111111111111111111111111111",
      maxFeeCapBps: "10",
    });
  });

  it("decodes Flight BuilderState from the beta on-chain fixture", () => {
    expect(
      normalize(
        flight.decodeBuilderState(loadMockBytes("flight_builder_state.json"))
      )
    ).toEqual({
      discriminant: "9166869205659451169",
      authorityKey: "F952dz4aHVUu75YdxUrGhLejhCfzXCYVDysDw6yL6uT4",
      traderKey: "ADvFCTV83VpkDnu69ZW17Grav5xswkcrmYexYHytoVYm",
      status: "1",
      isActive: true,
      feeBps: "8",
    });
  });
});
