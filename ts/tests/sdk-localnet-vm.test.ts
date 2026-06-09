import { address } from "@solana/kit";
import { describe, expect, test } from "vitest";
import {
  buildDepositIxsResolved,
  buildMarketOrderPacketFromMarketParams,
  buildPlaceMarketOrderIxResolved,
  buildWithdrawIxsResolved,
  Side,
} from "../src";
import type { SdkLocalnetContext } from "./test-harness/litesvm";
import {
  buildSdkLocalnetDepositInput,
  buildSdkLocalnetMarketParams,
  buildSdkLocalnetPlaceOrderContext,
  buildSdkLocalnetWithdrawInput,
  createSdkLocalnetContext,
  findSdkLocalnetProgramPaths,
  isSdkLocalnetVmRequired,
} from "./test-harness/litesvm";

const programPaths = findSdkLocalnetProgramPaths();
const vmTest = programPaths || isSdkLocalnetVmRequired() ? test : test.skip;
const WITHDRAW_COOLDOWN_SLOTS = 150n;

describe("SDK localnet VM harness", () => {
  vmTest(
    "loads locally built programs, replays fixture setup, and runs an SDK-built order",
    async () => {
      const context = await createSdkLocalnetContext({
        programPaths: programPaths ?? undefined,
      });

      expect(context.setupResults.length).toBeGreaterThan(0);
      expect(
        context.vm.getAccount(address(context.fixture.addresses.globalConfig))
      ).not.toBeNull();
      expect(
        context.vm.getAccount(address(context.getMarket("BTC").orderbook))
      ).not.toBeNull();
      expect(
        context.vm.getAccount(address(context.getActor("taker0").traderAccount))
      ).not.toBeNull();

      const orderPacket = buildMarketOrderPacketFromMarketParams(
        {
          side: Side.Bid,
          priceLimitUsd: "101000",
          baseUnits: "0.01",
          minBaseUnitsToFill: "0.01",
        },
        buildSdkLocalnetMarketParams(context, "BTC")
      );
      const instruction = buildPlaceMarketOrderIxResolved({
        ...buildSdkLocalnetPlaceOrderContext(context, {
          actorName: "taker0",
          symbol: "BTC",
        }),
        orderPacket,
      });
      const result = await context.sendInstructions([instruction], {
        feePayerSeed: context.getActor("taker0").seed,
        label: "sdk-built-place-market-order",
      });

      expect(result.metadata.computeUnitsConsumed()).toBeGreaterThan(0n);
    },
    120_000
  );

  vmTest(
    "builds and executes SDK deposit and withdraw instructions",
    async () => {
      const context = await createSdkLocalnetContext({
        programPaths: programPaths ?? undefined,
      });
      const actor = context.getActor("taker1");
      const amount = 1_000_000n;

      const initialUsdc = readSplTokenAmount(
        context,
        actor.fakeUsdcTokenAccount
      );
      const initialGlobalVault = readSplTokenAmount(
        context,
        context.fixture.addresses.globalVault
      );

      const deposit = buildDepositIxsResolved(
        buildSdkLocalnetDepositInput(context, {
          actorName: actor.name,
          amount,
        })
      );

      expect(deposit.named.emberDeposit.programAddress).toBe(
        context.fixture.programs.ember
      );
      expect(deposit.named.depositFunds.programAddress).toBe(
        context.fixture.programs.phoenixEternal
      );

      const depositResult = await context.sendInstructions(
        deposit.instructions,
        {
          feePayerSeed: actor.seed,
          label: "sdk-built-deposit",
        }
      );

      expect(depositResult.metadata.computeUnitsConsumed()).toBeGreaterThan(0n);
      expect(readSplTokenAmount(context, actor.fakeUsdcTokenAccount)).toBe(
        initialUsdc - amount
      );
      expect(
        readSplTokenAmount(context, context.fixture.addresses.globalVault)
      ).toBe(initialGlobalVault + amount);

      const currentSlot = context.vm.getClock().slot;
      context.vm.warpToSlot(currentSlot + WITHDRAW_COOLDOWN_SLOTS);

      const withdraw = buildWithdrawIxsResolved(
        buildSdkLocalnetWithdrawInput(context, {
          actorName: actor.name,
          amount,
        })
      );

      expect(withdraw.named.withdrawFunds.programAddress).toBe(
        context.fixture.programs.phoenixEternal
      );
      expect(withdraw.named.emberWithdraw.programAddress).toBe(
        context.fixture.programs.ember
      );

      const withdrawResult = await context.sendInstructions(
        withdraw.instructions,
        {
          feePayerSeed: actor.seed,
          label: "sdk-built-withdraw",
        }
      );

      expect(withdrawResult.metadata.computeUnitsConsumed()).toBeGreaterThan(
        0n
      );
      expect(readSplTokenAmount(context, actor.fakeUsdcTokenAccount)).toBe(
        initialUsdc
      );
      expect(
        readSplTokenAmount(context, context.fixture.addresses.globalVault)
      ).toBe(initialGlobalVault);
    },
    120_000
  );
});

const readSplTokenAmount = (
  context: Pick<SdkLocalnetContext, "vm">,
  tokenAccount: string
): bigint => {
  const account = context.vm.getAccount(address(tokenAccount));
  expect(account).not.toBeNull();
  if (!account) {
    throw new Error(`Missing SPL token account ${tokenAccount}`);
  }

  return new DataView(
    account.data.buffer,
    account.data.byteOffset + 64,
    8
  ).getBigUint64(0, true);
};
