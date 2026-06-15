import { Buffer } from "node:buffer";
import { AccountRole } from "@solana/kit";
import { describe, expect, test } from "vitest";
import {
  DEFAULT_SDK_LOCALNET_KEYPAIR_BASE_SEED,
  createSdkLocalnetMockActions,
  decodeFixtureInstruction,
  decodeFixtureTransaction,
  getSdkLocalnetTransaction,
} from "./test-harness/fixture";
import { defaultSdkLocalnetFixture } from "./test-harness/default-fixture";

describe("SDK localnet fixture", () => {
  test("loads shared deterministic exchange setup", () => {
    expect(defaultSdkLocalnetFixture.schemaVersion).toBe(1);
    expect(defaultSdkLocalnetFixture.name).toBe("rise-sdk-default-localnet");
    expect(defaultSdkLocalnetFixture.keypairBaseSeed).toBe(
      DEFAULT_SDK_LOCALNET_KEYPAIR_BASE_SEED
    );
    expect(defaultSdkLocalnetFixture.addresses.logAuthority).toBeTruthy();
    expect(defaultSdkLocalnetFixture.addresses.perpAssetMap).toBeTruthy();
    expect(defaultSdkLocalnetFixture.addresses.withdrawQueue).toBeTruthy();
    expect(
      defaultSdkLocalnetFixture.markets.map((market) => market.symbol)
    ).toEqual(["BTC", "ETH", "SOL"]);
    expect(defaultSdkLocalnetFixture.actors.map((actor) => actor.name)).toEqual(
      ["orderbookTrader", "splineTrader", "taker0", "taker1", "stopLossTaker"]
    );
    expect(
      defaultSdkLocalnetFixture.actors.every(
        (actor) => actor.fakeUsdcTokenAccount && actor.phoenixTokenAccount
      )
    ).toBe(true);
  });

  test("decodes every serialized instruction into Solana Kit shape", () => {
    const transactions = [
      ...defaultSdkLocalnetFixture.setupTransactions,
      ...defaultSdkLocalnetFixture.actionTemplates,
    ];

    for (const transaction of transactions) {
      expect(transaction.signerSeeds.length, transaction.name).toBeGreaterThan(
        0
      );
      expect(transaction.instructions.length, transaction.name).toBeGreaterThan(
        0
      );

      for (const instruction of transaction.instructions) {
        const decoded = decodeFixtureInstruction(instruction);
        expect(decoded.programAddress).toBe(instruction.programId);
        expect(decoded.accounts.length).toBe(instruction.accounts.length);
        expect(decoded.data.length).toBeGreaterThan(0);
      }
    }
  });

  test("exposes named mock action plans", () => {
    const actions = createSdkLocalnetMockActions(defaultSdkLocalnetFixture);

    expect(actions.oracle.setPrices.name).toBe("oracleSetPrices");
    expect(actions.oracle.setPrices.instructions).toHaveLength(1);
    expect(actions.oracle.setPrices.instructions[0]?.name).toBe(
      "updateOraclePrices"
    );
    expect(actions.orderbookTrader.placeLevels.name).toBe(
      "orderbookPlaceLevels"
    );
    expect(actions.orderbookTrader.cancelLevels.name).toBe(
      "orderbookCancelLevels"
    );
    expect(actions.splineTrader.updatePrices.name).toBe("splineUpdatePrices");
    expect(actions.collateral.forActor("taker0").name).toBe(
      "seedTaker0Collateral"
    );
  });

  test("configures the shared withdraw cooldown", () => {
    const exchange = getSdkLocalnetTransaction(
      defaultSdkLocalnetFixture,
      "initializeExchange"
    );
    const setWithdrawCooldown = exchange.instructions.find(
      (instruction) => instruction.name === "setWithdrawCooldown"
    );

    expect(setWithdrawCooldown).toBeDefined();
    if (!setWithdrawCooldown) {
      throw new Error("Missing setWithdrawCooldown instruction");
    }

    const data = Buffer.from(setWithdrawCooldown.dataBase64, "base64");
    expect(data[8]).toBe(1);
    expect(data.readBigUInt64LE(9)).toBe(150n);
    expect(data[17]).toBe(0);
    expect(data[18]).toBe(0);
  });

  test("encodes canned spline updates in ticks", () => {
    const splineUpdatePrices = getSdkLocalnetTransaction(
      defaultSdkLocalnetFixture,
      "splineUpdatePrices"
    );

    for (const market of defaultSdkLocalnetFixture.markets) {
      const instruction = splineUpdatePrices.instructions.find(
        (candidate) => candidate.name === `update${market.symbol}SplinePrice`
      );
      expect(instruction, market.symbol).toBeDefined();
      if (!instruction) {
        throw new Error(`Missing spline update for ${market.symbol}`);
      }

      const data = Buffer.from(instruction.dataBase64, "base64");
      expect(data.length).toBe(18);
      expect(data.readBigUInt64LE(8)).toBe(
        initialMarkPriceTicksForFixtureMarket(market)
      );
      expect(data[16]).toBe(0);
      expect(data[17]).toBe(1);
    }
  });

  test("encodes add-market setup payloads with after-hours radius", () => {
    const initializeMarkets = getSdkLocalnetTransaction(
      defaultSdkLocalnetFixture,
      "initializeMarkets"
    );

    for (const market of defaultSdkLocalnetFixture.markets) {
      const instruction = initializeMarkets.instructions.find(
        (candidate) => candidate.name === `add${market.symbol}Market`
      );
      expect(instruction, market.symbol).toBeDefined();
      if (!instruction) {
        throw new Error(`Missing add-market instruction for ${market.symbol}`);
      }

      const data = Buffer.from(instruction.dataBase64, "base64");
      expect(data.length).toBe(345);
      expect(data.readBigUInt64LE(data.length - 8)).toBe(0n);
    }
  });

  test("preserves signer roles while decoding account metas", () => {
    const exchange = getSdkLocalnetTransaction(
      defaultSdkLocalnetFixture,
      "initializeExchange"
    );
    const [initializeEmber] = decodeFixtureTransaction(exchange);

    expect(
      initializeEmber?.accounts.some(
        (account) => account.role === AccountRole.WRITABLE_SIGNER
      )
    ).toBe(true);
  });
});

const initialMarkPriceTicksForFixtureMarket = (
  market: (typeof defaultSdkLocalnetFixture.markets)[number]
): bigint => {
  let numerator = BigInt(market.initialMarkPriceAtoms) * 1000n;
  let denominator = BigInt(market.tickSizeInQuoteLotsPerBaseLot);
  const decimalScale = 10n ** BigInt(Math.abs(market.baseLotDecimals));

  if (market.baseLotDecimals >= 0) {
    denominator *= decimalScale;
  } else {
    numerator *= decimalScale;
  }

  return numerator / denominator;
};
