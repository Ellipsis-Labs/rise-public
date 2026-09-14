import {
  address,
  createKeyPairSignerFromPrivateKeyBytes,
  getConstantEncoder,
  getHiddenPrefixEncoder,
  getStructEncoder,
  getU64Encoder,
} from "@solana/kit";
import type { TransactionSigner } from "@solana/kit";
import { describe, expect, test } from "vitest";
import {
  buildCreatePermissionIx,
  buildDepositIxsResolved,
  buildMarketOrderPacketFromMarketParams,
  buildPlaceMarketOrderIxResolved,
  buildSetPermissionIx,
  buildWithdrawIxsResolved,
  createPhoenixIxClient,
  createPhoenixPdaClient,
  flight,
  generateReadonlyAccount,
  generateWritableAccount,
  generateWritableSignerAccount,
  getPhoenixPermissionAddress,
  Side,
} from "../src";
import type {
  Authority,
  GlobalConfigurationAddress,
  LogAuthorityAddress,
  PhoenixProgramAddress,
} from "../src";
import type { PhoenixExchangeMetadata } from "../src/exchange-cache/types";
import type {
  FixtureActor,
  SdkLocalnetContext,
  SdkLocalnetInstruction,
  SdkLocalnetSigners,
} from "./test-harness/litesvm";
import {
  buildSdkLocalnetDepositInput,
  buildSdkLocalnetMarketParams,
  buildSdkLocalnetPlaceOrderContext,
  buildSdkLocalnetTransaction,
  buildSdkLocalnetWithdrawInput,
  createSdkLocalnetContext,
  findSdkLocalnetProgramPaths,
  getSdkLocalnetActionTemplate,
  isSdkLocalnetVmRequired,
  readSdkLocalnetEffectiveTraderPosition,
  readSdkLocalnetEffectiveTraderState,
  sdkLocalnetSeedBytes,
  sendSdkLocalnetInstructions,
} from "./test-harness/litesvm";

const programPaths = findSdkLocalnetProgramPaths();
const vmTest = programPaths || isSdkLocalnetVmRequired() ? test : test.skip;
const flightVmTest =
  programPaths?.flight || isSdkLocalnetVmRequired() ? test : test.skip;
const WITHDRAW_COOLDOWN_SLOTS = 150n;
const POSITION_AUTHORITY_PERMISSION = 1n << 13n;
const AUTHORIZED_COLLATERAL_TRANSFER_PERMISSION = 1n << 15n;
const FLIGHT_MAX_FEE_CAP_BPS = 1_000n;
const FLIGHT_BUILDER_FEE_BPS = 100n;
const FLIGHT_DELEGATE_SEED = "flight-position-authority-delegate";

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

describe("Flight delegated market order proxy", () => {
  flightVmTest(
    "flight proxy collects builder fee for delegated market order signed by a secondary position authority",
    async () => {
      const { context, signers, user, builder, delegate, proxyInstruction } =
        await setUpFlightDelegatedProxy();

      const transaction = await buildSdkLocalnetTransaction(
        context.vm,
        signers,
        [proxyInstruction],
        { feePayerSeed: FLIGHT_DELEGATE_SEED }
      );
      const requiredSigners = Object.keys(transaction.signatures);
      expect(requiredSigners).toContain(delegate.address);
      expect(requiredSigners).not.toContain(user.pubkey);

      const userCollateralBefore = readTraderCollateral(
        context,
        user.traderAccount
      );
      const builderCollateralBefore = readTraderCollateral(
        context,
        builder.traderAccount
      );
      expect(
        readSdkLocalnetEffectiveTraderPosition(context, user.traderAccount, 0)
      ).toBeNull();

      const execution = await sendSdkLocalnetInstructions(
        context.vm,
        signers,
        [proxyInstruction],
        {
          feePayerSeed: FLIGHT_DELEGATE_SEED,
          label: "flight:proxy-delegated-market-order",
        }
      );
      expect(execution.metadata.computeUnitsConsumed()).toBeGreaterThan(0n);

      const position = readSdkLocalnetEffectiveTraderPosition(
        context,
        user.traderAccount,
        0
      );
      expect(position).not.toBeNull();
      expect(position?.position.baseLotPosition ?? 0n).toBeGreaterThan(0n);

      const collectedFee = parseFlightCollectedFee(execution.metadata.logs());
      expect(collectedFee).toBeGreaterThan(0n);
      expect(
        readTraderCollateral(context, builder.traderAccount) -
          builderCollateralBefore
      ).toBe(collectedFee);
      expect(
        userCollateralBefore - readTraderCollateral(context, user.traderAccount)
      ).toBeGreaterThanOrEqual(collectedFee);
    },
    120_000
  );

  flightVmTest(
    "flight proxy rejects delegated market order missing the collateral transfer tail",
    async () => {
      const {
        context,
        signers,
        user,
        builder,
        proxyInstruction,
        collateralTransferAuthority,
      } = await setUpFlightDelegatedProxy();

      expect(proxyInstruction.accounts.at(-2)?.address).toBe(
        collateralTransferAuthority
      );
      const withoutTail: SdkLocalnetInstruction = {
        ...proxyInstruction,
        accounts: proxyInstruction.accounts.slice(0, -2),
      };

      const userCollateralBefore = readTraderCollateral(
        context,
        user.traderAccount
      );
      const builderCollateralBefore = readTraderCollateral(
        context,
        builder.traderAccount
      );

      await expect(
        sendSdkLocalnetInstructions(context.vm, signers, [withoutTail], {
          feePayerSeed: FLIGHT_DELEGATE_SEED,
          label: "flight:proxy-delegated-market-order-without-tail",
        })
      ).rejects.toThrow(
        /Position-authority orders require the collateral transfer authority and permission accounts appended/
      );

      expect(readTraderCollateral(context, user.traderAccount)).toBe(
        userCollateralBefore
      );
      expect(readTraderCollateral(context, builder.traderAccount)).toBe(
        builderCollateralBefore
      );
      expect(
        readSdkLocalnetEffectiveTraderPosition(context, user.traderAccount, 0)
      ).toBeNull();
    },
    120_000
  );
});

interface FlightDelegatedProxySetup {
  context: SdkLocalnetContext;
  signers: SdkLocalnetSigners;
  user: FixtureActor;
  builder: FixtureActor;
  delegate: TransactionSigner;
  proxyInstruction: SdkLocalnetInstruction;
  collateralTransferAuthority: string;
}

/**
 * Boots the fixture exchange and prepares the Flight delegated
 * place-market-order flow: initializes the Flight global state, registers
 * taker1 as the fee-collecting builder, has taker0 grant a distinct delegate
 * key POSITION_AUTHORITY_PERMISSION (bit 13), and has the exchange root
 * authority (the fixture payer) grant the Flight collateral-transfer PDA
 * AUTHORIZED_COLLATERAL_TRANSFER_PERMISSION (bit 14). Returns the proxied
 * delegate-signed PlaceMarketOrderDelegated instruction ready to send.
 */
const setUpFlightDelegatedProxy =
  async (): Promise<FlightDelegatedProxySetup> => {
    if (!programPaths?.flight) {
      throw new Error(
        "Missing local Flight SBF artifact; run `mise build-sbf` or set RISE_SDK_LOCALNET_FLIGHT_SO"
      );
    }

    const context = await createSdkLocalnetContext({ programPaths });
    const phoenixProgramAddress = phoenixProgram(context);
    const user = context.getActor("taker0");
    const builder = context.getActor("taker1");
    const rootAuthority = context.getSigner("payer").address as Authority;

    // The builder fee leaves the user's trader account through a collateral
    // transfer, which respects the fixture's 150-slot withdraw cooldown that
    // started at the setup deposits. Warping makes the setup-time mark price
    // stale, so replay the oracle and spline price mock actions afterwards.
    context.vm.warpToSlot(context.vm.getClock().slot + WITHDRAW_COOLDOWN_SLOTS);
    await context.sendFixtureTransaction(
      getSdkLocalnetActionTemplate(context.fixture, "oracleSetPrices")
    );
    await context.sendFixtureTransaction(
      getSdkLocalnetActionTemplate(context.fixture, "splineUpdatePrices")
    );

    const delegate = await createExtraLocalnetSigner(
      context,
      FLIGHT_DELEGATE_SEED
    );
    const signers = withExtraLocalnetSigner(
      context.signers,
      delegate,
      FLIGHT_DELEGATE_SEED
    );

    await context.sendInstructions([await buildFlightInitIx(context)], {
      feePayerSeed: "payer",
      label: "flight:init-global-state",
    });

    const registerBuilderIx = await flight.buildRegisterBuilderIx({
      phoenixProgramAddress,
      traderAuthority: builder.pubkey as Authority,
      feeBps: FLIGHT_BUILDER_FEE_BPS,
    });
    expect(registerBuilderIx.accounts[3]?.address).toBe(builder.traderAccount);
    await context.sendInstructions([registerBuilderIx], {
      feePayerSeed: builder.seed,
      label: "flight:register-builder",
    });

    // taker0 (the user wallet) grants the delegate position authority.
    const positionPermission = await getPhoenixPermissionAddress(
      user.pubkey as Authority,
      delegate.address as Authority,
      phoenixProgramAddress
    );
    await context.sendInstructions(
      [
        buildCreatePermissionIx({
          ...phoenixOverrides(context),
          payer: user.pubkey as Authority,
          permissionAuthority: user.pubkey as Authority,
          delegatedKey: delegate.address as Authority,
          permissionPda: positionPermission,
        }),
        buildSetPermissionIx({
          ...phoenixOverrides(context),
          permissionAuthority: user.pubkey as Authority,
          delegatedKey: delegate.address as Authority,
          permissionPda: positionPermission,
          permission: POSITION_AUTHORITY_PERMISSION,
          expiresAtTimestamp: null,
          allowedSignerActions: null,
        }),
      ],
      {
        feePayerSeed: user.seed,
        label: "flight:grant-position-authority",
      }
    );

    // The exchange root authority grants the Flight collateral-transfer PDA
    // the authorized collateral transfer permission.
    const collateralTransferAuthority =
      await flight.getFlightCollateralTransferAuthorityAddress(
        phoenixProgramAddress
      );
    const delegationPermission = await getPhoenixPermissionAddress(
      rootAuthority,
      collateralTransferAuthority,
      phoenixProgramAddress
    );
    await context.sendInstructions(
      [
        buildCreatePermissionIx({
          ...phoenixOverrides(context),
          payer: rootAuthority,
          permissionAuthority: rootAuthority,
          delegatedKey: collateralTransferAuthority,
          permissionPda: delegationPermission,
        }),
        buildSetPermissionIx({
          ...phoenixOverrides(context),
          permissionAuthority: rootAuthority,
          delegatedKey: collateralTransferAuthority,
          permissionPda: delegationPermission,
          permission: AUTHORIZED_COLLATERAL_TRANSFER_PERMISSION,
          expiresAtTimestamp: null,
          allowedSignerActions: null,
        }),
      ],
      {
        feePayerSeed: "payer",
        label: "flight:grant-authorized-collateral-transfer",
      }
    );

    // Delegate-signed PlaceMarketOrderDelegated on taker0's trader account,
    // filling against the fixture maker ask at 100500 (mirrors the plain
    // market-order test above). Built through the public high-level ix
    // client: `authority` is the owner (the trader PDA derives from it),
    // `positionAuthority` is the delegate that signs, and the wrap takes the
    // position-authority path purely from the effective signer differing
    // from the owner.
    const orderPacket = buildMarketOrderPacketFromMarketParams(
      {
        side: Side.Bid,
        priceLimitUsd: "101000",
        baseUnits: "0.01",
        minBaseUnitsToFill: "0.01",
      },
      buildSdkLocalnetMarketParams(context, "BTC")
    );
    const ixClient = createPhoenixIxClient({
      exchange: createFixtureExchangeMetadata(context, rootAuthority),
      pda: createPhoenixPdaClient({
        programAddress: phoenixProgramAddress,
      }),
      orderPackets: {} as never,
      flight: { builderAuthority: builder.pubkey as Authority },
    });
    const proxyInstruction = (await ixClient.placeMarketOrderDelegated({
      authority: user.pubkey as Authority,
      positionAuthority: delegate.address as Authority,
      permissionAccount: positionPermission,
      symbol: "BTC" as never,
      orderPacket,
    })) as SdkLocalnetInstruction;

    // The public path must target the fixture accounts: the builder fee
    // collector and the user trader PDA both derive from their owners.
    expect(proxyInstruction.programAddress).toBe(flight.FLIGHT_PROGRAM_ADDRESS);
    expect(proxyInstruction.accounts[3]?.address).toBe(builder.traderAccount);
    expect(proxyInstruction.accounts[5]?.address).toBe(delegate.address);
    // Inner PlaceMarketOrderDelegated starts at proxy index 6: wallet at
    // +3, permission at +4, trader PDA (derived from the owner) at +5.
    expect(proxyInstruction.accounts[9]?.address).toBe(delegate.address);
    expect(proxyInstruction.accounts[10]?.address).toBe(positionPermission);
    expect(proxyInstruction.accounts[11]?.address).toBe(user.traderAccount);

    return {
      context,
      signers,
      user,
      builder,
      delegate,
      proxyInstruction,
      collateralTransferAuthority,
    };
  };

/**
 * Minimal `PhoenixExchangeMetadata` view over the localnet fixture: enough
 * for the public ix client to resolve the BTC market accounts and the
 * current root authority (the fixture payer) at wrap time.
 */
const createFixtureExchangeMetadata = (
  context: SdkLocalnetContext,
  rootAuthority: Authority
): PhoenixExchangeMetadata => {
  const market = context.getMarket("BTC");
  return {
    ready: async () => undefined,
    snapshot: () => ({
      markets: [{ symbol: "BTC" }],
      exchange: { currentAuthorities: { rootAuthority } },
    }),
    instructionContext: (symbol: string) =>
      symbol === "BTC"
        ? {
            exchange: {
              globalConfig: context.fixture.addresses.globalConfig,
              perpAssetMap: context.fixture.addresses.perpAssetMap,
              globalTraderIndex: context.fixture.addresses.globalTraderIndex,
              activeTraderBuffer: context.fixture.addresses.activeTraderBuffer,
            },
            market: {
              marketPubkey: market.orderbook,
              splinePubkey: market.spline,
            },
          }
        : undefined,
  } as unknown as PhoenixExchangeMetadata;
};

const buildFlightInitIx = async (
  context: SdkLocalnetContext
): Promise<SdkLocalnetInstruction> => {
  const phoenixProgramAddress = phoenixProgram(context);
  const globalStateAccount = await flight.getFlightGlobalStateAddress(
    phoenixProgramAddress
  );

  return {
    programAddress: flight.FLIGHT_PROGRAM_ADDRESS,
    accounts: [
      generateWritableAccount(globalStateAccount),
      generateReadonlyAccount(phoenixProgramAddress),
      generateWritableSignerAccount(context.getSigner("payer").address),
      generateReadonlyAccount(address(context.fixture.programs.system)),
    ],
    data: getFlightInitInstructionEncoder().encode({
      maxFeeCapBps: FLIGHT_MAX_FEE_CAP_BPS,
    }),
  };
};

const getFlightInitInstructionEncoder = () =>
  getHiddenPrefixEncoder(
    getStructEncoder([["maxFeeCapBps", getU64Encoder()]]),
    [getConstantEncoder(flight.FLIGHT_DISCRIMINANTS.INIT)]
  );

const parseFlightCollectedFee = (logs: readonly string[]): bigint => {
  const feeLog = logs.find((log) => log.includes("collected fee:"));
  const match =
    feeLog === undefined ? null : /collected fee: (\d+)/.exec(feeLog);
  if (!match?.[1]) {
    throw new Error(
      `Missing flight collected-fee log line in:\n${logs.join("\n")}`
    );
  }
  return BigInt(match[1]);
};

const readTraderCollateral = (
  context: Pick<SdkLocalnetContext, "fixture" | "vm">,
  traderAccount: string
): bigint =>
  readSdkLocalnetEffectiveTraderState(context, traderAccount).state
    .quoteLotCollateral;

const createExtraLocalnetSigner = async (
  context: SdkLocalnetContext,
  seed: string
): Promise<TransactionSigner> => {
  const signer = await createKeyPairSignerFromPrivateKeyBytes(
    sdkLocalnetSeedBytes(seed, context.fixture)
  );
  const result = context.vm.airdrop(
    address(signer.address) as never,
    10_000_000_000n as never
  );
  expect(result).not.toBeNull();
  return signer;
};

const withExtraLocalnetSigner = (
  signers: SdkLocalnetSigners,
  signer: TransactionSigner,
  seed: string
): SdkLocalnetSigners => ({
  all: [...signers.all, signer],
  bySeed: new Map(signers.bySeed).set(seed, signer),
  byName: new Map(signers.byName).set(seed, signer),
  byAddress: new Map(signers.byAddress).set(signer.address, signer),
});

const phoenixProgram = (context: SdkLocalnetContext): PhoenixProgramAddress =>
  context.fixture.programs.phoenixEternal as PhoenixProgramAddress;

const phoenixOverrides = (context: SdkLocalnetContext) => ({
  programAddress: phoenixProgram(context),
  logAuthorityAddress: context.fixture.addresses
    .logAuthority as LogAuthorityAddress,
  globalConfigurationAddress: context.fixture.addresses
    .globalConfig as GlobalConfigurationAddress,
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
