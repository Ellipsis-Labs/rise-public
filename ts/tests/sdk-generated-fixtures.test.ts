import { decodePerpAssetMap } from "@/index";
import {
  DISCRIMINANTS,
  EMBER_DISCRIMINANTS,
  sha2_const,
} from "@/core/discriminants";
import { FLIGHT_DISCRIMINANTS } from "@/flight/core/discriminants";
import { HAWKEYE_DISCRIMINANTS } from "@/hawkeye";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const TESTS_DIR = dirname(fileURLToPath(import.meta.url));
const FIXTURES_DIR = resolve(TESTS_DIR, "../test-fixtures");

type AccountFixtureFile = {
  schemaVersion: number;
  accounts: AccountFixture[];
};

type AccountFixture = {
  name: string;
  accountType: string;
  dataBase64: string;
  expected: PerpAssetMapExpected;
};

type PerpAssetMapExpected = {
  accountType: "PerpAssetMap";
  sequenceNumber: SequenceNumberExpected;
  numAssets: number;
  capacity: number;
  symbols: string[];
  assets: PerpAssetExpected[];
};

type PerpAssetExpected = {
  symbol: string;
  staticMarketParams: StaticMarketParamsExpected;
  priceSequenceNumber: SequenceNumberExpected;
  markPrice: MarkPriceExpected;
  fundingAccumulator: FundingAccumulatorExpected;
  openInterestParams: OpenInterestParamsExpected;
  assetFlags: AssetFlagsExpected;
  commoditiesAfterHoursRadius: string;
  lastKnownIndexPrice: string | null;
  lastIndexExpiryTimestamp: string;
  commoditiesAfterHoursRadiusBps: string;
};

type StaticMarketParamsExpected = {
  marketAccount: string;
  tickSize: string;
  assetId: number;
  baseLotDecimals: number;
};

type SequenceNumberExpected = {
  sequenceNumber: string;
  lastUpdateSlot: string;
};

type MarkPriceExpected = {
  price: TicksAtSlotExpected;
  spotPriceComponent: SpotPriceComponentExpected;
  perpPriceComponent: PerpPriceComponentExpected;
  bookPriceComponent: BookPriceComponentExpected;
  oracleParameters: OracleParametersExpected;
  oracleResponses: OracleResponseExpected[];
  markPriceLastValidatedSlot: string[];
};

type SpotPriceComponentExpected = {
  lastExchangeSpotPrice: TicksAtSlotExpected[];
  weight: string;
  staleThreshold: string;
  slot: string;
  midSpotDiffEmaTicks: string;
  midSpotDiffEmaTicksDust: string;
  emaPeriodSlots: string;
  emaDiffRadius: string;
};

type PerpPriceComponentExpected = {
  lastExchangePerpPrice: TicksAtSlotExpected[];
  weight: string;
  staleThreshold: string;
};

type BookPriceComponentExpected = {
  clampedBookMidPrice: TicksAtSlotExpected;
  weight: string;
  staleThreshold: string;
  bookPriceRadius: string;
  lastBestBid: TicksAtSlotExpected;
  lastBestAsk: TicksAtSlotExpected;
  lastTradePrice: TicksAtSlotExpected;
};

type TicksAtSlotExpected = {
  slot: string;
  ticks: string;
};

type OracleParametersExpected = {
  oracleDivergenceRadius: number;
  minOracleResponses: number;
};

type OracleResponseExpected = {
  index: number;
  oraclePubkey: string;
  divergenceScore: number;
  lastSpotPrice: TicksAtSlotExpected;
  lastPerpPrice: TicksAtSlotExpected;
  lastUpdatedTimestamp: string;
};

type FundingAccumulatorExpected = {
  acc: string;
  lastDiff: string;
  cumulativeFundingRate: string;
  startIntervalTimestamp: string;
  lastFundingUpdateTimestamp: string;
  fundingIntervalSeconds: string;
  fundingPeriodSeconds: string;
  maxFundingRate: string;
};

type OpenInterestParamsExpected = {
  openInterest: string;
  openInterestCap: string;
};

type AssetFlagsExpected = {
  bits: number;
  isCommodity: boolean;
  isCommoditiesReopen: boolean;
  isCommoditiesAfterHours: boolean;
};

type InstructionFixtureFile = {
  schemaVersion: number;
  instructions: InstructionFixture[];
};

type InstructionFixture = {
  enumName:
    | "PhoenixInstruction"
    | "EmberInstruction"
    | "FlightInstruction"
    | "HawkeyeInstruction";
  instructionName: string;
  snakeCaseName: string;
  preimage: string;
  tag: string;
  discriminatorHex: string;
  discriminatorBase64: string;
};

type DecodedPerpAssetMap = ReturnType<typeof decodePerpAssetMap>;
type DecodedPerpAsset =
  DecodedPerpAssetMap["metadata"]["entries"][number]["value"];
type DecodedTicksAtSlot = DecodedPerpAsset["oraclePrice"]["markPrice"]["price"];
type DecodedBookPriceComponent =
  DecodedPerpAsset["oraclePrice"]["markPrice"]["bookPriceComponent"];
type DecodedOracleData =
  DecodedPerpAsset["oraclePrice"]["markPrice"]["oracleData"][number];

const readJsonFixture = <T>(fileName: string): T =>
  JSON.parse(readFileSync(resolve(FIXTURES_DIR, fileName), "utf8")) as T;

const bytesToHex = (bytes: Uint8Array): string =>
  Buffer.from(bytes).toString("hex");

const bytesToBase64 = (bytes: Uint8Array): string =>
  Buffer.from(bytes).toString("base64");

const discriminatorTag = (bytes: Uint8Array): string =>
  new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
    .getBigUint64(0, true)
    .toString();

const discriminantMapForEnum = (
  enumName: InstructionFixture["enumName"]
): Record<string, Uint8Array> | undefined => {
  switch (enumName) {
    case "PhoenixInstruction":
      return DISCRIMINANTS;
    case "EmberInstruction":
      return EMBER_DISCRIMINANTS;
    case "FlightInstruction":
      return FLIGHT_DISCRIMINANTS;
    case "HawkeyeInstruction":
      return HAWKEYE_DISCRIMINANTS;
  }
};

describe("generated SDK fixtures", () => {
  it("decodes generated PerpAssetMap account bytes", () => {
    const fixture = readJsonFixture<AccountFixtureFile>(
      "sdk-account-fixtures.json"
    );
    expect(fixture.schemaVersion).toBe(1);
    const account = fixture.accounts.find(
      ({ accountType }) => accountType === "PerpAssetMap"
    );
    expect(account).toBeDefined();
    if (!account) return;

    expect(account.name).toBe("mainnetPerpAssetMap");
    const decoded = decodePerpAssetMap(
      Buffer.from(account.dataBase64, "base64")
    );
    const expected = account.expected;

    expect(expected.accountType).toBe("PerpAssetMap");
    expectSequenceNumber(decoded.sequenceNumber, expected.sequenceNumber);
    expect(decoded.numAssets).toBe(expected.numAssets);
    expect(decoded.metadata.capacity.toString()).toBe(
      expected.capacity.toString()
    );
    expect(decoded.metadata.entries.map(({ key }) => key)).toEqual(
      expected.symbols
    );

    for (const expectedAsset of expected.assets) {
      const decodedAsset = decoded.metadata.entries.find(
        ({ key }) => key === expectedAsset.symbol
      )?.value;
      expect(decodedAsset, `missing ${expectedAsset.symbol}`).toBeDefined();
      if (!decodedAsset) continue;
      expectAsset(decodedAsset, expectedAsset);
    }
  });

  it("matches generated instruction discriminators", () => {
    const fixture = readJsonFixture<InstructionFixtureFile>(
      "sdk-instruction-fixtures.json"
    );
    expect(fixture.schemaVersion).toBe(1);
    expect(fixture.instructions).toHaveLength(116);

    for (const instruction of fixture.instructions) {
      const discriminant = sha2_const(instruction.preimage);
      expect(instruction.preimage).toBe(`global:${instruction.snakeCaseName}`);
      expect(bytesToHex(discriminant)).toBe(instruction.discriminatorHex);
      expect(bytesToBase64(discriminant)).toBe(instruction.discriminatorBase64);
      expect(discriminatorTag(discriminant)).toBe(instruction.tag);

      const mapped = discriminantMapForEnum(instruction.enumName)?.[
        instruction.snakeCaseName.toUpperCase()
      ];
      if (mapped) {
        expect(bytesToHex(mapped)).toBe(instruction.discriminatorHex);
      }
    }
  });
});

const expectAsset = (
  actual: DecodedPerpAsset,
  expected: PerpAssetExpected
): void => {
  expect(actual.staticMarketParams.marketAccount).toBe(
    expected.staticMarketParams.marketAccount
  );
  expect(actual.staticMarketParams.tickSize.toString()).toBe(
    expected.staticMarketParams.tickSize
  );
  expect(actual.staticMarketParams.assetId).toBe(
    expected.staticMarketParams.assetId
  );
  expect(actual.staticMarketParams.baseLotDecimals).toBe(
    expected.staticMarketParams.baseLotDecimals
  );
  expectSequenceNumber(
    actual.oraclePrice.priceSequenceNumber,
    expected.priceSequenceNumber
  );
  expectMarkPrice(actual.oraclePrice.markPrice, expected.markPrice);
  expectFundingAccumulator(
    actual.fundingAccumulator,
    expected.fundingAccumulator
  );
  expect(actual.openInterestParams.openInterest.toString()).toBe(
    expected.openInterestParams.openInterest
  );
  expect(actual.openInterestParams.openInterestCap.toString()).toBe(
    expected.openInterestParams.openInterestCap
  );
  expect(actual.assetFlags).toEqual(expected.assetFlags);
  expect(actual.commoditiesAfterHoursRadius.toString()).toBe(
    expected.commoditiesAfterHoursRadius
  );
  expect(actual.lastKnownIndexPrice?.toString() ?? null).toBe(
    expected.lastKnownIndexPrice
  );
  expect(actual.lastIndexExpiryTimestamp.toString()).toBe(
    expected.lastIndexExpiryTimestamp
  );
  expect(actual.commoditiesAfterHoursRadiusBps.toString()).toBe(
    expected.commoditiesAfterHoursRadiusBps
  );
};

const expectSequenceNumber = (
  actual: { sequenceNumber: bigint; lastUpdateSlot: bigint },
  expected: SequenceNumberExpected
): void => {
  expect(actual.sequenceNumber.toString()).toBe(expected.sequenceNumber);
  expect(actual.lastUpdateSlot.toString()).toBe(expected.lastUpdateSlot);
};

const expectMarkPrice = (
  actual: DecodedPerpAsset["oraclePrice"]["markPrice"],
  expected: MarkPriceExpected
): void => {
  expectTicksAtSlot(actual.price, expected.price);
  expectSpotPriceComponent(
    actual.spotPriceComponent,
    expected.spotPriceComponent
  );
  expectPerpPriceComponent(
    actual.perpPriceComponent,
    expected.perpPriceComponent
  );
  expectBookPriceComponent(
    actual.bookPriceComponent,
    expected.bookPriceComponent
  );
  expect(actual.oracleParameters).toEqual(expected.oracleParameters);
  expectOracleResponses(
    actual.oracleData,
    actual.spotPriceComponent.lastExchangeSpotPrice,
    actual.perpPriceComponent.lastExchangePerpPrice,
    actual.oracleLastUpdatedTimestamps,
    expected.oracleResponses
  );
  expect(actual.markPriceLastValidatedSlot.map(String)).toEqual(
    expected.markPriceLastValidatedSlot
  );
};

const expectSpotPriceComponent = (
  actual: DecodedPerpAsset["oraclePrice"]["markPrice"]["spotPriceComponent"],
  expected: SpotPriceComponentExpected
): void => {
  expectTicksAtSlotArray(
    actual.lastExchangeSpotPrice,
    expected.lastExchangeSpotPrice
  );
  expect(actual.weight.toString()).toBe(expected.weight);
  expect(actual.staleThreshold.toString()).toBe(expected.staleThreshold);
  expect(actual.slot.toString()).toBe(expected.slot);
  expect(actual.midSpotDiffEmaTicks.toString()).toBe(
    expected.midSpotDiffEmaTicks
  );
  expect(actual.midSpotDiffEmaTicksDust.toString()).toBe(
    expected.midSpotDiffEmaTicksDust
  );
  expect(actual.emaPeriodSlots.toString()).toBe(expected.emaPeriodSlots);
  expect(actual.emaDiffRadius.toString()).toBe(expected.emaDiffRadius);
};

const expectPerpPriceComponent = (
  actual: DecodedPerpAsset["oraclePrice"]["markPrice"]["perpPriceComponent"],
  expected: PerpPriceComponentExpected
): void => {
  expectTicksAtSlotArray(
    actual.lastExchangePerpPrice,
    expected.lastExchangePerpPrice
  );
  expect(actual.weight.toString()).toBe(expected.weight);
  expect(actual.staleThreshold.toString()).toBe(expected.staleThreshold);
};

const expectBookPriceComponent = (
  actual: DecodedBookPriceComponent,
  expected: BookPriceComponentExpected
): void => {
  expectTicksAtSlot(actual.clampedBookMidPrice, expected.clampedBookMidPrice);
  expect(actual.weight.toString()).toBe(expected.weight);
  expect(actual.staleThreshold.toString()).toBe(expected.staleThreshold);
  expect(actual.bookPriceRadius.toString()).toBe(expected.bookPriceRadius);
  expectTicksAtSlot(actual.lastBestBid, expected.lastBestBid);
  expectTicksAtSlot(actual.lastBestAsk, expected.lastBestAsk);
  expectTicksAtSlot(actual.lastTradePrice, expected.lastTradePrice);
};

const expectOracleResponses = (
  oracleData: DecodedOracleData[],
  spotPrices: DecodedTicksAtSlot[],
  perpPrices: DecodedTicksAtSlot[],
  timestamps: bigint[],
  expected: OracleResponseExpected[]
): void => {
  expect(oracleData).toHaveLength(expected.length);
  expect(spotPrices).toHaveLength(expected.length);
  expect(perpPrices).toHaveLength(expected.length);
  expect(timestamps).toHaveLength(expected.length);
  for (const response of expected) {
    expect(oracleData[response.index]?.oraclePubkey).toBe(
      response.oraclePubkey
    );
    expect(oracleData[response.index]?.divergenceScore).toBe(
      response.divergenceScore
    );
    expectTicksAtSlot(spotPrices[response.index], response.lastSpotPrice);
    expectTicksAtSlot(perpPrices[response.index], response.lastPerpPrice);
    expect(timestamps[response.index]?.toString()).toBe(
      response.lastUpdatedTimestamp
    );
  }
};

const expectFundingAccumulator = (
  actual: DecodedPerpAsset["fundingAccumulator"],
  expected: FundingAccumulatorExpected
): void => {
  expect(actual.acc.toString()).toBe(expected.acc);
  expect(actual.lastDiff.toString()).toBe(expected.lastDiff);
  expect(actual.cumulativeFundingRate.toString()).toBe(
    expected.cumulativeFundingRate
  );
  expect(actual.startIntervalTimestamp.toString()).toBe(
    expected.startIntervalTimestamp
  );
  expect(actual.lastFundingUpdateTimestamp.toString()).toBe(
    expected.lastFundingUpdateTimestamp
  );
  expect(actual.fundingIntervalSeconds.toString()).toBe(
    expected.fundingIntervalSeconds
  );
  expect(actual.fundingPeriodSeconds.toString()).toBe(
    expected.fundingPeriodSeconds
  );
  expect(actual.maxFundingRate.toString()).toBe(expected.maxFundingRate);
};

const expectTicksAtSlot = (
  actual: DecodedTicksAtSlot | undefined,
  expected: TicksAtSlotExpected
): void => {
  expect(actual?.slot.toString()).toBe(expected.slot);
  expect(actual?.ticks.toString()).toBe(expected.ticks);
};

const expectTicksAtSlotArray = (
  actual: DecodedTicksAtSlot[],
  expected: TicksAtSlotExpected[]
): void => {
  expect(actual).toHaveLength(expected.length);
  for (const [index, expectedValue] of expected.entries()) {
    expectTicksAtSlot(actual[index], expectedValue);
  }
};
