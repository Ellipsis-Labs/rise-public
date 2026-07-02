import { Buffer } from "node:buffer";
import {
  address,
  createKeyPairSignerFromPrivateKeyBytes,
  getArrayEncoder,
  getBase58Decoder,
  getBooleanEncoder,
  getConstantEncoder,
  getHiddenPrefixEncoder,
  getOptionEncoder,
  getStructEncoder,
  getU8Encoder,
  getU64Encoder,
} from "@solana/kit";
import type { TransactionSigner } from "@solana/kit";
import { describe, expect, test } from "vitest";
import {
  Direction,
  HAWKEYE_PROGRAM_ADDRESS,
  OrderFlags,
  SelfTradeBehavior,
  Side,
  StopLossOrderKind,
  buildHawkeyeViewBboIx,
  buildHawkeyeViewFundingIx,
  buildHawkeyeViewLiquidationPriceIx,
  buildHawkeyeViewMarginForAssetIx,
  buildHawkeyeViewMarginIx,
  buildCancelConditionalOrderIx,
  buildCreatePermissionIx,
  buildCancelAllIxResolved,
  buildCancelOrdersById,
  buildCancelOrdersByIdIxResolved,
  buildCancelStopLossIxResolved,
  buildCreateConditionalOrdersAccountIx,
  buildDepositIxsResolved,
  buildLimitOrderPacketFromMarketParams,
  buildMarketOrderPacketFromMarketParams,
  buildPlaceLimitOrderIxResolved,
  buildPlaceLimitOrderWithConditionalsIx,
  buildPlaceMarketOrderIxResolved,
  buildPlaceStopLossIxResolved,
  buildRegisterTraderIxResolved,
  buildSetPermissionIx,
  buildSyncParentToChildIxResolved,
  buildTakeProfitStopLossTriggerOrders,
  buildTransferCollateralChildToParentIxResolved,
  buildTransferCollateralIxResolved,
  buildUpdateTraderStateIx,
  buildWithdrawIxsResolved,
  decodeHawkeyeReturnData,
  executionPriceFromSlippageBps,
  generateArenaAccounts,
  generateReadonlyAccount,
  generateReadonlySignerAccount,
  generateWritableAccount,
  getPhoenixConditionalOrdersAddress,
  getPhoenixPermissionAddress,
  getPhoenixStopLossAddress,
  getPhoenixTraderSubaccountAddress,
  getSymbolEncoder,
  priceUsdToTicksWithMarketParams,
  sha2_const,
} from "../src";
import type { HawkeyeIx, HawkeyeReturnData } from "../src";
import {
  decodeConditionalOrderCollection,
  decodeStopLosses,
} from "../src/accounts";
import type {
  ActiveTraderBufferAddressArray,
  Authority,
  FIFOOrderId,
  GlobalConfigurationAddress,
  GlobalTraderIndexAddressArray,
  MarketAddress,
  PerpAssetMapAddress,
  PhoenixProgramAddress,
  PhoenixInstructionClient,
  SplineCollectionAddress,
  TraderAddress,
} from "../src";
import type {
  FixtureActor,
  SdkLocalnetContext,
  SdkLocalnetInstructionExecution,
  SdkLocalnetSigners,
} from "./test-harness/litesvm";
import {
  buildSdkLocalnetDepositInput,
  buildSdkLocalnetMarketParams,
  buildSdkLocalnetPlaceOrderContext,
  buildSdkLocalnetWithdrawInput,
  createSdkLocalnetContext,
  findSdkLocalnetProgramPaths,
  isSdkLocalnetVmRequired,
  readSdkLocalnetEffectiveTraderPosition,
  readSdkLocalnetEffectiveTraderState,
  readSdkLocalnetOrderbook,
  readSdkLocalnetTrader,
  sdkLocalnetSeedBytes,
  sendSdkLocalnetInstructions,
} from "./test-harness/litesvm";

const programPaths = findSdkLocalnetProgramPaths();
const vmTest = programPaths || isSdkLocalnetVmRequired() ? test : test.skip;
const TEST_TIMEOUT_MS = 120_000;
const WITHDRAW_COOLDOWN_SLOTS = 150n;
const SUBACCOUNT_INDEX = 1;
const STOP_LOSS_PERMISSION = 1n << 2n;
const ORACLE_UPDATE_TIMESTAMP = 1_900_000_001n;
const ORACLE_PRICE_EXPO = 3;
const DISCRIMINANTS = {
  EXECUTE_CONDITIONAL_ORDERS: sha2_const("global:execute_conditional_orders"),
  PING_CONDITIONAL_ORDERS: sha2_const("global:ping_conditional_orders"),
  UPDATE_ORACLE_PRICES_WITH_ORDERING: sha2_const(
    "global:update_oracle_prices_with_ordering"
  ),
  UPDATE_SPLINE_PRICE: sha2_const("global:update_spline_price"),
} as const;

describe("SDK localnet common flows", () => {
  vmTest(
    "deposits collateral",
    async () => {
      const context = await createContext();
      const actor = context.getActor("taker0");
      const amount = 1_250_000n;
      const initialUsdc = readSplTokenAmount(
        context,
        actor.fakeUsdcTokenAccount
      );
      const initialVault = readSplTokenAmount(
        context,
        context.fixture.addresses.globalVault
      );
      const initialCollateral = readSdkLocalnetEffectiveTraderState(
        context,
        actor.traderAccount
      ).state.quoteLotCollateral;

      const deposit = buildDepositIxsResolved(
        buildSdkLocalnetDepositInput(context, {
          actorName: actor.name,
          amount,
        })
      );

      await expectSuccess(
        context.sendInstructions(deposit.instructions, {
          feePayerSeed: actor.seed,
          label: "flow:deposit-collateral",
        })
      );
      expect(readSplTokenAmount(context, actor.fakeUsdcTokenAccount)).toBe(
        initialUsdc - amount
      );
      expect(
        readSplTokenAmount(context, context.fixture.addresses.globalVault)
      ).toBe(initialVault + amount);
      expect(
        readSdkLocalnetEffectiveTraderState(context, actor.traderAccount).state
          .quoteLotCollateral
      ).toBe(initialCollateral + amount);
    },
    TEST_TIMEOUT_MS
  );

  vmTest(
    "registers a trader subaccount and syncs parent capabilities",
    async () => {
      const context = await createContext();
      const { childTrader } = await registerAndSyncSubaccount(
        context,
        "taker0"
      );
      const child = readSdkLocalnetTrader(context, childTrader);
      const parent = readSdkLocalnetTrader(
        context,
        context.getActor("taker0").traderAccount
      );

      expect(child.traderSubaccountIndex).toBe(SUBACCOUNT_INDEX);
      expect(child.maxPositions).toBe(1);
      expect(child.authority).toBe(context.getActor("taker0").pubkey);
      expect(child.state.flags).toBe(parent.state.flags);
    },
    TEST_TIMEOUT_MS
  );

  vmTest(
    "places a market order",
    async () => {
      const context = await createContext();
      const actor = context.getActor("taker0");

      expect(
        readSdkLocalnetEffectiveTraderPosition(context, actor.traderAccount, 0)
      ).toBeNull();

      await placeMarketOrder(context, {
        actorName: "taker0",
        symbol: "BTC",
        side: Side.Bid,
        baseUnits: "0.01",
        priceLimitUsd: "101000",
      });

      const position = expectTraderPosition(
        context,
        actor.traderAccount,
        "market order"
      );
      expect(position.position.baseLotPosition).toBeGreaterThan(0n);
    },
    TEST_TIMEOUT_MS
  );

  vmTest(
    "executes Hawkeye view functions and decodes return data",
    async () => {
      if (!programPaths?.hawkeye) {
        if (isSdkLocalnetVmRequired()) {
          throw new Error(
            "Missing local Hawkeye SBF artifact; set RISE_SDK_LOCALNET_HAWKEYE_SO"
          );
        }
        console.warn(
          "skipping Hawkeye LiteSVM flow because phoenix_hawkeye.so is missing"
        );
        return;
      }

      const context = await createContext();
      const actor = context.getActor("taker0");
      const market = context.getMarket("BTC");
      const baseAccounts = {
        phoenixProgramAddress: context.fixture.programs
          .phoenixEternal as PhoenixProgramAddress,
        globalConfigurationAddress: context.fixture.addresses
          .globalConfig as GlobalConfigurationAddress,
        globalTraderIndex: context.fixture.addresses
          .globalTraderIndex as GlobalTraderIndexAddressArray,
        activeTraderBuffer: context.fixture.addresses
          .activeTraderBuffer as ActiveTraderBufferAddressArray,
        perpAssetMap: context.fixture.addresses
          .perpAssetMap as PerpAssetMapAddress,
      };
      const traderAccounts = {
        ...baseAccounts,
        traderAccount: actor.traderAccount as TraderAddress,
      };

      await placeMarketOrder(context, {
        actorName: actor.name,
        symbol: "BTC",
        side: Side.Bid,
        baseUnits: "0.01",
        priceLimitUsd: "101000",
      });

      const margin = await executeHawkeyeView(
        context,
        buildHawkeyeViewMarginIx(traderAccounts),
        "hawkeye:view-margin"
      );
      expect(margin.kind).toBe("view_margin");
      if (margin.kind !== "view_margin") {
        throw new Error(`unexpected Hawkeye return kind ${margin.kind}`);
      }
      expect(margin.positionCount).toBeGreaterThan(0);
      expect(margin.initialMarginQuoteLots).toBeGreaterThan(0n);

      const asset = await executeHawkeyeView(
        context,
        buildHawkeyeViewMarginForAssetIx({
          ...traderAccounts,
          assetId: 0,
        }),
        "hawkeye:view-margin-for-asset"
      );
      expect(asset.kind).toBe("view_margin_for_asset");
      if (asset.kind !== "view_margin_for_asset") {
        throw new Error(`unexpected Hawkeye return kind ${asset.kind}`);
      }
      expect(asset.assetId).toBe(0);
      expect(asset.baseLots).not.toBe(0n);
      expect(asset.markPriceTicks).toBeGreaterThan(0n);

      const liquidation = await executeHawkeyeView(
        context,
        buildHawkeyeViewLiquidationPriceIx({
          ...traderAccounts,
          assetId: 0,
        }),
        "hawkeye:view-liquidation-price"
      );
      expect(liquidation.kind).toBe("view_liquidation_price");
      if (liquidation.kind !== "view_liquidation_price") {
        throw new Error(`unexpected Hawkeye return kind ${liquidation.kind}`);
      }
      expect(liquidation.assetId).toBe(0);
      expect(liquidation.markPriceTicks).toBeGreaterThan(0n);

      const bbo = await executeHawkeyeView(
        context,
        buildHawkeyeViewBboIx({
          ...baseAccounts,
          orderbook: market.orderbook as MarketAddress,
          splineCollection: market.spline as SplineCollectionAddress,
        }),
        "hawkeye:view-bbo"
      );
      expect(bbo.kind).toBe("view_bbo");
      if (bbo.kind !== "view_bbo") {
        throw new Error(`unexpected Hawkeye return kind ${bbo.kind}`);
      }
      expect(bbo.bestBidTicks ?? bbo.bestAskTicks).not.toBeNull();
      expect(bbo.markPriceTicks).toBeGreaterThan(0n);
      expect(bbo.indexPriceTicks).toBeGreaterThan(0n);

      const funding = await executeHawkeyeView(
        context,
        buildHawkeyeViewFundingIx({
          ...traderAccounts,
          assetId: 0,
        }),
        "hawkeye:view-funding"
      );
      expect(funding.kind).toBe("view_funding");
      if (funding.kind !== "view_funding") {
        throw new Error(`unexpected Hawkeye return kind ${funding.kind}`);
      }
      expect(funding.assetId).toBe(0);
      expect(funding.totalAccumulatedFundingQuoteLots).not.toBeNull();
    },
    TEST_TIMEOUT_MS
  );

  vmTest(
    "attaches a stop loss after opening a position",
    async () => {
      const context = await createContext();

      await placeMarketOrder(context, {
        actorName: "stopLossTaker",
        symbol: "BTC",
        side: Side.Bid,
        baseUnits: "0.01",
        priceLimitUsd: "101000",
      });
      await placeStopLoss(context, "stopLossTaker");

      const stopLossAccount = await getStopLossAccount(
        context,
        "stopLossTaker"
      );
      const stopLosses = decodeStopLosses(
        readAccountData(context, stopLossAccount)
      );
      const marketParams = buildSdkLocalnetMarketParams(context, "BTC");
      const lessThanStopLoss = stopLosses.stopLosses.find(
        (stopLoss) => stopLoss.isActive
      );

      expect(stopLosses.isInitialized).toBe(true);
      expect(stopLosses.traderKey).toBe(
        context.getActor("stopLossTaker").traderAccount
      );
      expect(lessThanStopLoss).toBeDefined();
      expect(lessThanStopLoss?.executionDirection).toBe(Direction.LessThan);
      expect(lessThanStopLoss?.tradeSide).toBe(Side.Ask);
      expect(lessThanStopLoss?.orderKind).toBe(StopLossOrderKind.IOC);
      expect(lessThanStopLoss?.triggerPrice).toBe(
        priceUsdToTicksWithMarketParams("90000", marketParams)
      );
      expect(lessThanStopLoss?.executionPrice).toBe(
        priceUsdToTicksWithMarketParams("89000", marketParams)
      );
    },
    TEST_TIMEOUT_MS
  );

  vmTest(
    "closes a stop loss account funded by a separate payer",
    async () => {
      const context = await createContext();
      const actor = context.getActor("stopLossTaker");
      const payerSeed = "stop-loss-account-payer";
      const payer = await createExtraLocalnetSigner(context, payerSeed);
      const signers = withExtraLocalnetSigner(
        context.signers,
        payer,
        payerSeed
      );

      await placeMarketOrder(context, {
        actorName: actor.name,
        symbol: "BTC",
        side: Side.Bid,
        baseUnits: "0.01",
        priceLimitUsd: "101000",
      });

      const stopLossAccount = await getStopLossAccount(context, actor.name);
      const marketParams = buildSdkLocalnetMarketParams(context, "BTC");
      await expectSuccess(
        sendSdkLocalnetInstructions(
          context.vm,
          signers,
          [
            buildPlaceStopLossIxResolved({
              ...buildSdkLocalnetPlaceOrderContext(context, {
                actorName: actor.name,
                symbol: "BTC",
              }),
              trader: {
                ...buildSdkLocalnetPlaceOrderContext(context, {
                  actorName: actor.name,
                  symbol: "BTC",
                }).trader,
                funder: payer.address as Authority,
              },
              stopLossAccount: address(stopLossAccount),
              triggerPrice: priceUsdToTicksWithMarketParams(
                "90000",
                marketParams
              ),
              executionPrice: priceUsdToTicksWithMarketParams(
                "89000",
                marketParams
              ),
              tradeSide: Side.Ask,
              executionDirection: Direction.LessThan,
              orderKind: StopLossOrderKind.IOC,
            }),
          ],
          {
            feePayerSeed: payerSeed,
            label: "flow:place-stop-loss-with-external-payer",
          }
        )
      );

      const stopLosses = decodeStopLosses(
        readAccountData(context, stopLossAccount)
      );
      expect(stopLosses.fundingKey).toBe(payer.address);

      const stopLossRentLamports = readLamports(context, stopLossAccount);
      const payerLamportsBeforeCancel = readLamports(context, payer.address);
      await expectSuccess(
        sendSdkLocalnetInstructions(
          context.vm,
          signers,
          [
            buildCancelStopLossIxResolved({
              exchange: phoenixAccounts(context),
              trader: {
                authority: actor.pubkey as Authority,
                funder: payer.address as Authority,
                traderAccount: actor.traderAccount as TraderAddress,
                stopLossAccount: address(stopLossAccount),
              },
              executionDirection: Direction.LessThan,
            }),
          ],
          {
            feePayerSeed: actor.seed,
            label: "flow:cancel-stop-loss-with-external-payer",
          }
        )
      );

      expectAccountClosed(context, stopLossAccount);
      expect(readLamports(context, payer.address)).toBe(
        payerLamportsBeforeCancel + stopLossRentLamports
      );
    },
    TEST_TIMEOUT_MS
  );

  vmTest(
    "closes a position with an opposite market order",
    async () => {
      const context = await createContext();
      const actor = context.getActor("taker0");

      await placeMarketOrder(context, {
        actorName: "taker0",
        symbol: "BTC",
        side: Side.Bid,
        baseUnits: "0.01",
        priceLimitUsd: "101000",
      });
      expectTraderPosition(context, actor.traderAccount, "open before close");

      await placeMarketOrder(context, {
        actorName: "taker0",
        symbol: "BTC",
        side: Side.Ask,
        baseUnits: "0.01",
        priceLimitUsd: "99000",
      });

      expectNoTraderPosition(context, actor.traderAccount);
    },
    TEST_TIMEOUT_MS
  );

  vmTest(
    "places a limit order",
    async () => {
      const context = await createContext();

      const orderId = await placeLimitOrder(context, {
        actorName: "taker0",
        symbol: "BTC",
        side: Side.Bid,
        priceUsd: "88000",
        baseUnits: "0.01",
      });

      expect(orderId.priceInTicks).toBe(
        priceUsdToTicksWithMarketParams(
          "88000",
          buildSdkLocalnetMarketParams(context, "BTC")
        )
      );
      expectOrderbookContainsOrder(context, "BTC", Side.Bid, orderId);
      expectOrderbookHasRestingPrice(context, "BTC", Side.Bid, "99500");
      expectOrderbookHasRestingPrice(context, "BTC", Side.Ask, "100500");

      const position = expectTraderPosition(
        context,
        context.getActor("taker0").traderAccount,
        "limit order"
      );
      expect(position.source).toBe("activeTraderBuffer");
      if (!position.activePositionState) {
        throw new Error(
          "Expected active trader position state for limit order"
        );
      }
      expect(position.activePositionState.bidOrders.head).not.toBeNull();
      expect(
        position.activePositionState.totalNonReduceOnlyBidBaseLots
      ).toBeGreaterThan(0n);
    },
    TEST_TIMEOUT_MS
  );

  vmTest(
    "places a limit order with attached conditional orders",
    async () => {
      const context = await createContext();
      const actor = context.getActor("taker0");
      const market = context.getMarket("BTC");
      const marketParams = buildSdkLocalnetMarketParams(context, "BTC");
      const traderConditionalOrders = await getPhoenixConditionalOrdersAddress({
        traderAccount: actor.traderAccount as TraderAddress,
        phoenixProgramAddress: phoenixProgram(context),
      });

      await expectSuccess(
        context.sendInstructions(
          [
            buildCreateConditionalOrdersAccountIx({
              ...phoenixAccounts(context),
              payer: actor.pubkey as Authority,
              traderWallet: actor.pubkey as Authority,
              traderAccount: actor.traderAccount as TraderAddress,
              traderConditionalOrders,
              capacity: 8,
            }),
          ],
          {
            feePayerSeed: actor.seed,
            label: "flow:create-conditional-orders-account",
          }
        )
      );

      const ix = buildPlaceLimitOrderWithConditionalsIx({
        ...phoenixAccounts(context),
        traderWallet: actor.pubkey as Authority,
        traderAccount: actor.traderAccount as TraderAddress,
        perpAssetMap: context.fixture.addresses.perpAssetMap as never,
        globalTraderIndex: context.fixture.addresses.globalTraderIndex as never,
        activeTraderBuffer: context.fixture.addresses
          .activeTraderBuffer as never,
        orderbook: market.orderbook as MarketAddress,
        splineCollection: market.spline as never,
        payer: actor.pubkey as Authority,
        traderConditionalOrders,
        slot: context.vm.getClock().slot,
        orderPacket: {
          __kind: "Limit",
          ...buildLimitOrderPacketFromMarketParams(
            {
              side: Side.Bid,
              priceUsd: "87000",
              baseUnits: "0.01",
              selfTradeBehavior: SelfTradeBehavior.CancelProvide,
              orderFlags: OrderFlags.None,
            },
            marketParams
          ),
        },
        greaterTriggerOrder: {
          triggerDirection: Direction.GreaterThan,
          tradeSide: Side.Ask,
          orderKind: StopLossOrderKind.Limit,
          triggerPrice: priceUsdToTicksWithMarketParams("110000", marketParams),
          executionPrice: priceUsdToTicksWithMarketParams(
            "109000",
            marketParams
          ),
        },
        lessTriggerOrder: null,
      });

      await expectSuccess(
        context.sendInstructions([ix], {
          feePayerSeed: actor.seed,
          label: "flow:place-limit-with-conditionals",
        })
      );

      const conditionalOrders = decodeConditionalOrderCollection(
        readAccountData(context, traderConditionalOrders)
      );
      const activeOrderIndex = conditionalOrders.activeOrderIndices[0];
      expect(activeOrderIndex).toBeDefined();

      const conditionalOrder =
        activeOrderIndex === undefined
          ? null
          : conditionalOrders.orders[activeOrderIndex];
      expect(conditionalOrder).not.toBeNull();
      expect(conditionalOrder?.assetId).toBe(0);
      expect(conditionalOrder?.greaterTriggerOrder.isActive).toBe(true);
      expect(conditionalOrder?.greaterTriggerOrder.executionDirection).toBe(
        Direction.GreaterThan
      );
      expect(conditionalOrder?.greaterTriggerOrder.tradeSide).toBe(Side.Ask);
      expect(conditionalOrder?.greaterTriggerOrder.triggerPrice).toBe(
        priceUsdToTicksWithMarketParams("110000", marketParams)
      );
      expectOrderbookHasRestingPrice(context, "BTC", Side.Bid, "87000");
    },
    TEST_TIMEOUT_MS
  );

  vmTest(
    "cancels an attached conditional order",
    async () => {
      const context = await createContext();
      const actor = context.getActor("taker0");
      const market = context.getMarket("BTC");
      const marketParams = buildSdkLocalnetMarketParams(context, "BTC");
      const traderConditionalOrders = await getPhoenixConditionalOrdersAddress({
        traderAccount: actor.traderAccount as TraderAddress,
        phoenixProgramAddress: phoenixProgram(context),
      });

      await expectSuccess(
        context.sendInstructions(
          [
            buildCreateConditionalOrdersAccountIx({
              ...phoenixAccounts(context),
              payer: actor.pubkey as Authority,
              traderWallet: actor.pubkey as Authority,
              traderAccount: actor.traderAccount as TraderAddress,
              traderConditionalOrders,
              capacity: 8,
            }),
          ],
          {
            feePayerSeed: actor.seed,
            label: "flow:create-conditional-orders-account-for-cancel",
          }
        )
      );

      await expectSuccess(
        context.sendInstructions(
          [
            buildPlaceLimitOrderWithConditionalsIx({
              ...phoenixAccounts(context),
              traderWallet: actor.pubkey as Authority,
              traderAccount: actor.traderAccount as TraderAddress,
              perpAssetMap: context.fixture.addresses.perpAssetMap as never,
              globalTraderIndex: context.fixture.addresses
                .globalTraderIndex as never,
              activeTraderBuffer: context.fixture.addresses
                .activeTraderBuffer as never,
              orderbook: market.orderbook as MarketAddress,
              splineCollection: market.spline as never,
              payer: actor.pubkey as Authority,
              traderConditionalOrders,
              slot: context.vm.getClock().slot,
              orderPacket: {
                __kind: "Limit",
                ...buildLimitOrderPacketFromMarketParams(
                  {
                    side: Side.Bid,
                    priceUsd: "87200",
                    baseUnits: "0.01",
                    selfTradeBehavior: SelfTradeBehavior.CancelProvide,
                    orderFlags: OrderFlags.None,
                  },
                  marketParams
                ),
              },
              greaterTriggerOrder: {
                triggerDirection: Direction.GreaterThan,
                tradeSide: Side.Ask,
                orderKind: StopLossOrderKind.Limit,
                triggerPrice: priceUsdToTicksWithMarketParams(
                  "110000",
                  marketParams
                ),
                executionPrice: priceUsdToTicksWithMarketParams(
                  "109000",
                  marketParams
                ),
              },
              lessTriggerOrder: null,
            }),
          ],
          {
            feePayerSeed: actor.seed,
            label: "flow:place-limit-with-conditional-for-cancel",
          }
        )
      );

      const beforeCancel = decodeConditionalOrderCollection(
        readAccountData(context, traderConditionalOrders)
      );
      const activeOrderIndex = beforeCancel.activeOrderIndices[0];
      expect(activeOrderIndex).toBeDefined();
      if (activeOrderIndex === undefined) {
        throw new Error("Missing attached conditional order index");
      }

      await expectSuccess(
        context.sendInstructions(
          [
            buildCancelConditionalOrderIx({
              ...phoenixAccounts(context),
              traderAccount: actor.traderAccount as TraderAddress,
              traderWallet: actor.pubkey as Authority,
              orderbook: market.orderbook as MarketAddress,
              traderConditionalOrders,
              conditionalOrderIndex: activeOrderIndex as never,
              disableFirst: true,
              disableSecond: true,
            }),
          ],
          {
            feePayerSeed: actor.seed,
            label: "flow:cancel-attached-conditional-order",
          }
        )
      );

      const afterCancel = decodeConditionalOrderCollection(
        readAccountData(context, traderConditionalOrders)
      );
      expect(afterCancel.activeOrderIndices).not.toContain(activeOrderIndex);
      expect(
        afterCancel.orders[activeOrderIndex]?.greaterTriggerOrder.isActive
      ).toBe(false);
      expectOrderbookHasRestingPrice(context, "BTC", Side.Bid, "87200");
    },
    TEST_TIMEOUT_MS
  );

  vmTest(
    "executes an attached stop loss after oracle and spline reach the trigger",
    async () => {
      const context = await createContext();
      const actor = context.getActor("taker0");
      const market = context.getMarket("BTC");
      const marketParams = buildSdkLocalnetMarketParams(context, "BTC");
      const marketAuthority = context.getSigner("payer").address as Authority;
      const executor = context.getSigner("stop-loss-executor");
      const executorPermission = await getPhoenixPermissionAddress(
        marketAuthority,
        executor.address as Authority,
        phoenixProgram(context)
      );
      const traderConditionalOrders = await getPhoenixConditionalOrdersAddress({
        traderAccount: actor.traderAccount as TraderAddress,
        phoenixProgramAddress: phoenixProgram(context),
      });

      await expectSuccess(
        context.sendInstructions(
          [
            buildCreateConditionalOrdersAccountIx({
              ...phoenixAccounts(context),
              payer: actor.pubkey as Authority,
              traderWallet: actor.pubkey as Authority,
              traderAccount: actor.traderAccount as TraderAddress,
              traderConditionalOrders,
              capacity: 8,
            }),
          ],
          {
            feePayerSeed: actor.seed,
            label: "flow:create-conditional-orders-account-for-execution",
          }
        )
      );

      await expectSuccess(
        context.sendInstructions(
          [
            buildCreatePermissionIx({
              ...phoenixAccounts(context),
              payer: marketAuthority,
              permissionAuthority: marketAuthority,
              delegatedKey: executor.address as Authority,
              permissionPda: executorPermission,
            }),
            buildSetPermissionIx({
              ...phoenixAccounts(context),
              permissionAuthority: marketAuthority,
              delegatedKey: executor.address as Authority,
              permissionPda: executorPermission,
              permission: STOP_LOSS_PERMISSION,
              expiresAtTimestamp: null,
              allowedSignerActions: null,
            }),
          ],
          {
            feePayerSeed: "payer",
            label: "flow:delegate-conditional-order-executor",
          }
        )
      );

      const takeProfitPrice = priceUsdToTicksWithMarketParams(
        "110000",
        marketParams
      );
      const stopLossPrice = priceUsdToTicksWithMarketParams(
        "85000",
        marketParams
      );
      const { greaterTriggerOrder, lessTriggerOrder } =
        buildTakeProfitStopLossTriggerOrders({
          primarySide: Side.Bid,
          takeProfitPrice,
          stopLossPrice,
        });

      expect(greaterTriggerOrder?.orderKind).toBe(StopLossOrderKind.Limit);
      expect(greaterTriggerOrder?.executionPrice).toBe(takeProfitPrice);
      expect(lessTriggerOrder?.orderKind).toBe(StopLossOrderKind.IOC);
      expect(lessTriggerOrder?.executionPrice).toBe(
        executionPriceFromSlippageBps(stopLossPrice, Side.Ask, 1_000)
      );

      await expectSuccess(
        context.sendInstructions(
          [
            buildPlaceLimitOrderWithConditionalsIx({
              ...phoenixAccounts(context),
              traderWallet: actor.pubkey as Authority,
              traderAccount: actor.traderAccount as TraderAddress,
              perpAssetMap: context.fixture.addresses.perpAssetMap as never,
              globalTraderIndex: context.fixture.addresses
                .globalTraderIndex as never,
              activeTraderBuffer: context.fixture.addresses
                .activeTraderBuffer as never,
              orderbook: market.orderbook as MarketAddress,
              splineCollection: market.spline as never,
              payer: actor.pubkey as Authority,
              traderConditionalOrders,
              slot: context.vm.getClock().slot,
              orderPacket: {
                __kind: "Limit",
                ...buildLimitOrderPacketFromMarketParams(
                  {
                    side: Side.Bid,
                    priceUsd: "90000",
                    baseUnits: "0.01",
                    selfTradeBehavior: SelfTradeBehavior.CancelProvide,
                    orderFlags: OrderFlags.None,
                  },
                  marketParams
                ),
              },
              greaterTriggerOrder,
              lessTriggerOrder,
            }),
          ],
          {
            feePayerSeed: actor.seed,
            label: "flow:place-limit-with-tp-sl-conditionals",
          }
        )
      );

      const conditionalOrders = decodeConditionalOrderCollection(
        readAccountData(context, traderConditionalOrders)
      );
      const activeOrderIndex = conditionalOrders.activeOrderIndices[0];
      expect(activeOrderIndex).toBeDefined();
      if (activeOrderIndex === undefined) {
        throw new Error("Missing attached conditional order index");
      }

      await expectFixtureSuccess(
        context.sendFixtureTransaction("orderbookCancelLevels")
      );

      await placeMarketOrder(context, {
        actorName: "orderbookTrader",
        symbol: "BTC",
        side: Side.Ask,
        baseUnits: "0.01",
        priceLimitUsd: "90000",
      });

      const openedPosition = expectTraderPosition(
        context,
        actor.traderAccount,
        "filled conditional parent"
      );
      expect(openedPosition.position.baseLotPosition).toBeGreaterThan(0n);

      await expectSuccess(
        context.sendInstructions(
          [
            buildPingConditionalOrderIx(context, {
              ...phoenixAccounts(context),
              signer: executor.address as Authority,
              permissionAccount: executorPermission,
              traderAccount: actor.traderAccount as TraderAddress,
              orderbook: market.orderbook as MarketAddress,
              globalTraderIndex: context.fixture.addresses
                .globalTraderIndex as never,
              activeTraderBuffer: context.fixture.addresses
                .activeTraderBuffer as never,
              traderConditionalOrders,
            }),
          ],
          {
            feePayerSeed: "stop-loss-executor",
            label: "flow:ping-attached-conditional-order",
          }
        )
      );

      await moveBtcMarkPrice(context, "84000");

      await placeLimitOrder(context, {
        actorName: "orderbookTrader",
        symbol: "BTC",
        side: Side.Bid,
        priceUsd: "85000",
        baseUnits: "0.01",
      });

      await expectSuccess(
        context.sendInstructions(
          [
            buildExecuteConditionalOrderIx(context, {
              ...phoenixAccounts(context),
              signer: executor.address as Authority,
              permissionAccount: executorPermission,
              traderAccount: actor.traderAccount as TraderAddress,
              perpAssetMap: context.fixture.addresses.perpAssetMap as never,
              globalTraderIndex: context.fixture.addresses
                .globalTraderIndex as never,
              activeTraderBuffer: context.fixture.addresses
                .activeTraderBuffer as never,
              orderbook: market.orderbook as MarketAddress,
              splineCollection: market.spline as never,
              traderConditionalOrders,
              conditionalOrderIndex: activeOrderIndex,
              triggerDirection: Direction.LessThan,
            }),
          ],
          {
            feePayerSeed: "stop-loss-executor",
            label: "flow:execute-attached-stop-loss-conditional",
          }
        )
      );

      expectNoTraderPosition(context, actor.traderAccount);
      expectOrderbookDoesNotHaveRestingPrice(context, "BTC", Side.Bid, "85000");
    },
    TEST_TIMEOUT_MS
  );

  vmTest(
    "cancels a limit order by id",
    async () => {
      const context = await createContext();
      const actor = context.getActor("taker0");

      const orderId = await placeLimitOrder(context, {
        actorName: actor.name,
        symbol: "BTC",
        side: Side.Bid,
        priceUsd: "86000",
        baseUnits: "0.01",
      });
      expectOrderbookContainsOrder(context, "BTC", Side.Bid, orderId);

      await expectSuccess(
        context.sendInstructions(
          [
            buildCancelOrdersByIdIxResolved({
              ...buildSdkLocalnetPlaceOrderContext(context, {
                actorName: actor.name,
                symbol: "BTC",
              }),
              orderIds: [{ nodePointer: null, orderId }],
            }),
          ],
          {
            feePayerSeed: actor.seed,
            label: "flow:cancel-order-by-id",
          }
        )
      );
      expectOrderbookDoesNotContainOrder(context, "BTC", Side.Bid, orderId);
    },
    TEST_TIMEOUT_MS
  );

  vmTest(
    "cancels a limit order by id through the public helper using tick price",
    async () => {
      const context = await createContext();
      const actor = context.getActor("taker0");

      const orderId = await placeLimitOrder(context, {
        actorName: actor.name,
        symbol: "BTC",
        side: Side.Bid,
        priceUsd: "86100",
        baseUnits: "0.01",
      });
      expectOrderbookContainsOrder(context, "BTC", Side.Bid, orderId);

      const cancelIx = await buildCancelOrdersById(
        {
          authority: actor.pubkey as Authority,
          symbol: "BTC" as never,
          orders: [
            {
              priceInTicks: orderId.priceInTicks,
              orderSequenceNumber: orderId.orderSequenceNumber,
            },
          ],
        },
        createFixtureInstructionClient(context)
      );

      await expectSuccess(
        context.sendInstructions([cancelIx], {
          feePayerSeed: actor.seed,
          label: "flow:cancel-order-by-id-public-helper-ticks",
        })
      );
      expectOrderbookDoesNotContainOrder(context, "BTC", Side.Bid, orderId);
    },
    TEST_TIMEOUT_MS
  );

  vmTest(
    "cancels all orders",
    async () => {
      const context = await createContext();
      const actor = context.getActor("taker1");

      await placeLimitOrder(context, {
        actorName: actor.name,
        symbol: "BTC",
        side: Side.Bid,
        priceUsd: "85000",
        baseUnits: "0.01",
      });
      expectOrderbookHasRestingPrice(context, "BTC", Side.Bid, "85000");

      await expectSuccess(
        context.sendInstructions(
          [
            buildCancelAllIxResolved(
              buildSdkLocalnetPlaceOrderContext(context, {
                actorName: actor.name,
                symbol: "BTC",
              })
            ),
          ],
          {
            feePayerSeed: actor.seed,
            label: "flow:cancel-all",
          }
        )
      );
      expectOrderbookDoesNotHaveRestingPrice(context, "BTC", Side.Bid, "85000");
    },
    TEST_TIMEOUT_MS
  );

  vmTest(
    "transfers collateral to a subaccount",
    async () => {
      const context = await createContext();
      const { actor, childTrader } = await registerAndSyncSubaccount(
        context,
        "taker0"
      );
      const amount = 500_000n;
      const parentBefore = readSdkLocalnetEffectiveTraderState(
        context,
        actor.traderAccount
      ).state.quoteLotCollateral;
      const childBefore = readSdkLocalnetEffectiveTraderState(
        context,
        childTrader
      ).state.quoteLotCollateral;

      await transferCollateralToSubaccount(context, actor, childTrader, amount);

      expect(
        readSdkLocalnetEffectiveTraderState(context, actor.traderAccount).state
          .quoteLotCollateral
      ).toBe(parentBefore - amount);
      expect(
        readSdkLocalnetEffectiveTraderState(context, childTrader).state
          .quoteLotCollateral
      ).toBe(childBefore + amount);
    },
    TEST_TIMEOUT_MS
  );

  vmTest(
    "places an isolated market order from a subaccount",
    async () => {
      const context = await createContext();
      const { actor, childTrader } = await registerAndFundSubaccount(
        context,
        "taker0"
      );
      expect(
        readSdkLocalnetEffectiveTraderPosition(context, childTrader, 0)
      ).toBeNull();

      await placeMarketOrder(context, {
        actorName: actor.name,
        symbol: "BTC",
        side: Side.Bid,
        baseUnits: "0.01",
        priceLimitUsd: "101000",
        traderAccount: childTrader,
      });

      const position = expectTraderPosition(
        context,
        childTrader,
        "isolated market order"
      );
      expect(position.position.baseLotPosition).toBeGreaterThan(0n);
    },
    TEST_TIMEOUT_MS
  );

  vmTest(
    "places an isolated limit order from a subaccount",
    async () => {
      const context = await createContext();
      const { actor, childTrader } = await registerAndFundSubaccount(
        context,
        "taker0"
      );

      const orderId = await placeLimitOrder(context, {
        actorName: actor.name,
        symbol: "BTC",
        side: Side.Bid,
        priceUsd: "85500",
        baseUnits: "0.01",
        traderAccount: childTrader,
      });

      expectOrderbookContainsOrder(context, "BTC", Side.Bid, orderId);
      const position = expectTraderPosition(
        context,
        childTrader,
        "isolated limit order"
      );
      expect(position.activePositionState?.bidOrders.head).not.toBeNull();
    },
    TEST_TIMEOUT_MS
  );

  vmTest(
    "closes an isolated position",
    async () => {
      const context = await createContext();
      const { actor, childTrader } = await registerAndFundSubaccount(
        context,
        "taker0"
      );

      await openAndCloseIsolatedPosition(context, actor, childTrader);
      expectNoTraderPosition(context, childTrader);
    },
    TEST_TIMEOUT_MS
  );

  vmTest(
    "transfers collateral from a subaccount back to its parent",
    async () => {
      const context = await createContext();
      const { actor, childTrader } = await registerAndFundSubaccount(
        context,
        "taker0"
      );

      await openAndCloseIsolatedPosition(context, actor, childTrader);
      expectNoTraderPosition(context, childTrader);
      await updateTraderState(context, actor, childTrader);

      const parentBefore = readSdkLocalnetEffectiveTraderState(
        context,
        actor.traderAccount
      ).state.quoteLotCollateral;
      const childBefore = readSdkLocalnetEffectiveTraderState(
        context,
        childTrader
      ).state.quoteLotCollateral;
      expect(childBefore).toBeGreaterThan(0n);

      await expectSuccess(
        context.sendInstructions(
          [
            buildTransferCollateralChildToParentIxResolved({
              exchange: collateralExchangeContext(context),
              trader: {
                authority: actor.pubkey as Authority,
                childTraderAccount: childTrader,
                parentTraderAccount: actor.traderAccount as TraderAddress,
              },
            }),
          ],
          {
            feePayerSeed: actor.seed,
            label: "flow:transfer-collateral-child-to-parent",
          }
        )
      );

      expect(
        readSdkLocalnetEffectiveTraderState(context, childTrader).state
          .quoteLotCollateral
      ).toBe(0n);
      expect(
        readSdkLocalnetEffectiveTraderState(context, actor.traderAccount).state
          .quoteLotCollateral
      ).toBe(parentBefore + childBefore);
    },
    TEST_TIMEOUT_MS
  );

  vmTest(
    "rejects withdraw before 150 slots have passed",
    async () => {
      const context = await createContext();
      const actor = context.getActor("taker1");
      const amount = 1_000_000n;
      const initialUsdc = readSplTokenAmount(
        context,
        actor.fakeUsdcTokenAccount
      );
      const initialVault = readSplTokenAmount(
        context,
        context.fixture.addresses.globalVault
      );

      const deposit = buildDepositIxsResolved(
        buildSdkLocalnetDepositInput(context, {
          actorName: actor.name,
          amount,
        })
      );
      await expectSuccess(
        context.sendInstructions(deposit.instructions, {
          feePayerSeed: actor.seed,
          label: "flow:pre-cooldown-withdraw-prereq-deposit",
        })
      );

      const withdraw = buildWithdrawIxsResolved(
        buildSdkLocalnetWithdrawInput(context, {
          actorName: actor.name,
          amount,
        })
      );
      await expect(
        context.sendInstructions(withdraw.instructions, {
          feePayerSeed: actor.seed,
          label: "flow:withdraw-before-150-slots",
        })
      ).rejects.toThrow(/LiteSVM transaction failed/);

      expect(readSplTokenAmount(context, actor.fakeUsdcTokenAccount)).toBe(
        initialUsdc - amount
      );
      expect(
        readSplTokenAmount(context, context.fixture.addresses.globalVault)
      ).toBe(initialVault + amount);
    },
    TEST_TIMEOUT_MS
  );

  vmTest(
    "waits 150 slots and withdraws collateral",
    async () => {
      const context = await createContext();
      const actor = context.getActor("taker1");
      const amount = 1_000_000n;
      const initialUsdc = readSplTokenAmount(
        context,
        actor.fakeUsdcTokenAccount
      );
      const initialVault = readSplTokenAmount(
        context,
        context.fixture.addresses.globalVault
      );

      const deposit = buildDepositIxsResolved(
        buildSdkLocalnetDepositInput(context, {
          actorName: actor.name,
          amount,
        })
      );
      await expectSuccess(
        context.sendInstructions(deposit.instructions, {
          feePayerSeed: actor.seed,
          label: "flow:withdraw-prereq-deposit",
        })
      );

      context.vm.warpToSlot(
        context.vm.getClock().slot + WITHDRAW_COOLDOWN_SLOTS
      );

      const withdraw = buildWithdrawIxsResolved(
        buildSdkLocalnetWithdrawInput(context, {
          actorName: actor.name,
          amount,
        })
      );
      await expectSuccess(
        context.sendInstructions(withdraw.instructions, {
          feePayerSeed: actor.seed,
          label: "flow:withdraw-after-150-slots",
        })
      );

      expect(readSplTokenAmount(context, actor.fakeUsdcTokenAccount)).toBe(
        initialUsdc
      );
      expect(
        readSplTokenAmount(context, context.fixture.addresses.globalVault)
      ).toBe(initialVault);
    },
    TEST_TIMEOUT_MS
  );
});

const createContext = async (): Promise<SdkLocalnetContext> =>
  createSdkLocalnetContext({
    programPaths: programPaths ?? undefined,
  });

const createFixtureInstructionClient = (
  context: SdkLocalnetContext
): PhoenixInstructionClient => {
  const fakeUsdcMint = context.fixture.mints.find(
    (mint) => mint.name === "fakeUsdc"
  )?.pubkey;
  if (fakeUsdcMint === undefined) {
    throw new Error("SDK localnet fixture is missing fakeUsdc mint");
  }

  const exchangeSnapshot = createFixtureExchangeSnapshot(context);
  return {
    addresses: {
      phoenixProgramAddress: phoenixProgram(context),
      logAuthorityAddress: context.fixture.addresses.logAuthority as never,
      globalConfigurationAddress: context.fixture.addresses
        .globalConfig as never,
      usdcMintAddress: fakeUsdcMint as never,
      emberStateAddress: context.fixture.addresses.emberState as never,
    },
    fetchAccount: async (accountAddress) => {
      const account = context.vm.getAccount(address(accountAddress));
      if (account === null) {
        throw new Error(`Missing LiteSVM account ${accountAddress}`);
      }
      return { data: account.data };
    },
    exchange: {
      ready: async () => exchangeSnapshot,
      snapshot: () => exchangeSnapshot,
      instructionContext: (symbolName: string) => {
        const market = exchangeSnapshot.markets.find(
          (candidate) => candidate.symbol === symbolName
        );
        return market === undefined
          ? undefined
          : { exchange: exchangeSnapshot.exchange, market };
      },
    } as never,
  };
};

const createFixtureExchangeSnapshot = (context: SdkLocalnetContext) => {
  const fakeUsdcMint = context.fixture.mints.find(
    (mint) => mint.name === "fakeUsdc"
  )?.pubkey;
  const phoenixMint = context.fixture.mints.find(
    (mint) => mint.name === "phoenixCollateral"
  )?.pubkey;
  if (fakeUsdcMint === undefined || phoenixMint === undefined) {
    throw new Error("SDK localnet fixture is missing expected mints");
  }

  const authority = context.getSigner("payer").address;
  return {
    version: 1,
    slot: context.vm.getClock().slot,
    slotIndex: 0,
    exchange: {
      programId: context.fixture.programs.phoenixEternal,
      globalConfig: context.fixture.addresses.globalConfig,
      currentAuthorities: {
        rootAuthority: authority,
        riskAuthority: authority,
        marketAuthority: authority,
        oracleAuthority: authority,
        adlAuthority: authority,
        cancelAuthority: authority,
        backstopAuthority: authority,
      },
      canonicalMint: phoenixMint,
      usdcMint: fakeUsdcMint,
      globalVault: context.fixture.addresses.globalVault,
      perpAssetMap: context.fixture.addresses.perpAssetMap,
      globalTraderIndex: context.fixture.addresses.globalTraderIndex,
      activeTraderBuffer: context.fixture.addresses.activeTraderBuffer,
      withdrawQueue: context.fixture.addresses.withdrawQueue,
      exchangeStatusBits: 129,
      exchangeStatusFeatures: [],
      active: true,
      gated: false,
      withdrawalsAvailable: true,
    },
    markets: context.fixture.markets.map((market, assetId) => ({
      symbol: market.symbol,
      assetId,
      marketStatus: "active",
      marketPubkey: market.orderbook,
      splinePubkey: market.spline,
      tickSize: market.tickSizeInQuoteLotsPerBaseLot,
      baseLotsDecimals: market.baseLotDecimals,
    })),
  };
};

const expectSuccess = async (
  result: Promise<SdkLocalnetInstructionExecution>
): Promise<SdkLocalnetInstructionExecution> => {
  const execution = await result;
  expect(execution.metadata.computeUnitsConsumed()).toBeGreaterThan(0n);
  return execution;
};

const executeHawkeyeView = async (
  context: SdkLocalnetContext,
  instruction: HawkeyeIx,
  label: string
): Promise<HawkeyeReturnData> => {
  const execution = await expectSuccess(
    context.sendInstructions([instruction], {
      feePayerSeed: "payer",
      label,
    })
  );
  const returnData = execution.metadata.returnData();
  const bytes = returnData.data();
  expect(bytes.byteLength, `${label} return data length`).toBeGreaterThan(0);
  expect(getBase58Decoder().decode(returnData.programId())).toBe(
    HAWKEYE_PROGRAM_ADDRESS
  );
  return decodeHawkeyeReturnData(bytes);
};

const expectFixtureSuccess = async (
  result: Promise<SdkLocalnetInstructionExecution[]>
): Promise<SdkLocalnetInstructionExecution[]> => {
  const executions = await result;
  expect(executions.length).toBeGreaterThan(0);
  for (const execution of executions) {
    expect(execution.metadata.computeUnitsConsumed()).toBeGreaterThan(0n);
  }
  return executions;
};

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

const moveBtcMarkPrice = async (
  context: SdkLocalnetContext,
  priceUsd: string
): Promise<void> => {
  await updateBtcOraclePrice(context, priceUsd);
  await updateBtcSplinePrice(context, priceUsd);
};

const updateBtcOraclePrice = async (
  context: SdkLocalnetContext,
  priceUsd: string
): Promise<void> => {
  const payer = context.getSigner("payer");
  const oracle = context.getSigner("price-oracle");
  const permissionAccount = await getPhoenixPermissionAddress(
    payer.address as Authority,
    oracle.address as Authority,
    phoenixProgram(context)
  );

  await expectSuccess(
    context.sendInstructions(
      [
        buildUpdateOraclePricesWithOrderingIx(context, {
          oracle: oracle.address as Authority,
          permissionAccount,
          priceUsd,
        }),
      ],
      {
        feePayerSeed: "price-oracle",
        label: `flow:update-btc-oracle-price:${priceUsd}`,
      }
    )
  );
};

const updateBtcSplinePrice = async (
  context: SdkLocalnetContext,
  priceUsd: string
): Promise<void> => {
  const splineTrader = context.getActor("splineTrader");
  const marketParams = buildSdkLocalnetMarketParams(context, "BTC");

  await expectSuccess(
    context.sendInstructions(
      [
        buildUpdateSplinePriceIx(context, {
          signer: splineTrader.pubkey as Authority,
          traderAccount: splineTrader.traderAccount as TraderAddress,
          priceInTicks: priceUsdToTicksWithMarketParams(priceUsd, marketParams),
        }),
      ],
      {
        feePayerSeed: splineTrader.seed,
        label: `flow:update-btc-spline-price:${priceUsd}`,
      }
    )
  );
};

const buildPingConditionalOrderIx = (
  context: SdkLocalnetContext,
  params: {
    signer: Authority;
    permissionAccount: string;
    traderAccount: TraderAddress;
    orderbook: MarketAddress;
    globalTraderIndex: Parameters<typeof generateArenaAccounts>[0];
    activeTraderBuffer: Parameters<typeof generateArenaAccounts>[0];
    traderConditionalOrders: string;
  }
) => ({
  programAddress: phoenixProgram(context),
  accounts: [
    generateReadonlyAccount(phoenixProgram(context)),
    generateReadonlyAccount(context.fixture.addresses.logAuthority as never),
    generateWritableAccount(context.fixture.addresses.globalConfig as never),
    generateReadonlySignerAccount(params.signer),
    generateWritableAccount(address(params.permissionAccount)),
    generateWritableAccount(params.orderbook),
    generateWritableAccount(params.traderAccount),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
    generateWritableAccount(address(params.traderConditionalOrders)),
  ],
  data: getConstantEncoder(DISCRIMINANTS.PING_CONDITIONAL_ORDERS).encode(
    undefined
  ),
});

const buildExecuteConditionalOrderIx = (
  context: SdkLocalnetContext,
  params: {
    signer: Authority;
    permissionAccount: string;
    traderAccount: TraderAddress;
    perpAssetMap: string;
    globalTraderIndex: Parameters<typeof generateArenaAccounts>[0];
    activeTraderBuffer: Parameters<typeof generateArenaAccounts>[0];
    orderbook: MarketAddress;
    splineCollection: string;
    traderConditionalOrders: string;
    conditionalOrderIndex: number;
    triggerDirection: Direction;
  }
) => ({
  programAddress: phoenixProgram(context),
  accounts: [
    generateReadonlyAccount(phoenixProgram(context)),
    generateReadonlyAccount(context.fixture.addresses.logAuthority as never),
    generateWritableAccount(context.fixture.addresses.globalConfig as never),
    generateReadonlySignerAccount(params.signer),
    generateWritableAccount(address(params.permissionAccount)),
    generateWritableAccount(params.traderAccount),
    generateWritableAccount(address(params.perpAssetMap)),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
    generateWritableAccount(params.orderbook),
    generateWritableAccount(address(params.splineCollection)),
    generateWritableAccount(address(params.traderConditionalOrders)),
  ],
  data: getExecuteConditionalOrderEncoder().encode({
    conditionalOrderIndex: params.conditionalOrderIndex,
    triggerDirection: params.triggerDirection,
  }),
});

const buildUpdateOraclePricesWithOrderingIx = (
  context: SdkLocalnetContext,
  params: {
    oracle: Authority;
    permissionAccount: string;
    priceUsd: string;
  }
) => {
  const price = {
    value: priceUsdToOracleAtoms(params.priceUsd),
    expo: ORACLE_PRICE_EXPO,
  };

  return {
    programAddress: phoenixProgram(context),
    accounts: [
      generateReadonlyAccount(phoenixProgram(context)),
      generateReadonlyAccount(context.fixture.addresses.logAuthority as never),
      generateReadonlyAccount(context.fixture.addresses.globalConfig as never),
      generateReadonlySignerAccount(params.oracle),
      generateWritableAccount(address(params.permissionAccount)),
      generateWritableAccount(context.fixture.addresses.perpAssetMap as never),
    ],
    data: getUpdateOraclePricesWithOrderingEncoder().encode({
      updateTimestamp: ORACLE_UPDATE_TIMESTAMP,
      updates: [
        {
          perpAssetId: "BTC" as never,
          newExchangePerpPrice: {
            __option: "Some" as const,
            value: price,
          },
          newExchangeSpotPrice: price,
        },
      ],
      flags: { flags: 0 },
    }),
  };
};

const buildUpdateSplinePriceIx = (
  context: SdkLocalnetContext,
  params: {
    signer: Authority;
    traderAccount: TraderAddress;
    priceInTicks: bigint;
  }
) => {
  const market = context.getMarket("BTC");

  return {
    programAddress: phoenixProgram(context),
    accounts: [
      generateReadonlyAccount(phoenixProgram(context)),
      generateReadonlyAccount(context.fixture.addresses.logAuthority as never),
      generateReadonlySignerAccount(params.signer),
      generateReadonlyAccount(params.traderAccount),
      generateWritableAccount(market.spline as never),
    ],
    data: getUpdateSplinePriceEncoder().encode({
      newMidPrice: params.priceInTicks,
      userUpdateSlot: { __option: "None" as const },
      refreshRegions: true,
    }),
  };
};

const getPriceEncoder = () =>
  getStructEncoder([
    ["value", getU64Encoder()],
    ["expo", getU8Encoder()],
  ]);

const getUpdateOraclePricesWithOrderingEncoder = () =>
  getHiddenPrefixEncoder(
    getStructEncoder([
      ["updateTimestamp", getU64Encoder()],
      [
        "updates",
        getArrayEncoder(
          getStructEncoder([
            ["perpAssetId", getSymbolEncoder()],
            ["newExchangePerpPrice", getOptionEncoder(getPriceEncoder())],
            ["newExchangeSpotPrice", getPriceEncoder()],
          ])
        ),
      ],
      ["flags", getStructEncoder([["flags", getU8Encoder()]])],
    ]),
    [getConstantEncoder(DISCRIMINANTS.UPDATE_ORACLE_PRICES_WITH_ORDERING)]
  );

const getUpdateSplinePriceEncoder = () =>
  getHiddenPrefixEncoder(
    getStructEncoder([
      ["newMidPrice", getU64Encoder()],
      ["userUpdateSlot", getOptionEncoder(getU64Encoder())],
      ["refreshRegions", getBooleanEncoder()],
    ]),
    [getConstantEncoder(DISCRIMINANTS.UPDATE_SPLINE_PRICE)]
  );

const getExecuteConditionalOrderEncoder = () =>
  getHiddenPrefixEncoder(
    getStructEncoder([
      ["conditionalOrderIndex", getU8Encoder()],
      ["triggerDirection", getU8Encoder()],
    ]),
    [getConstantEncoder(DISCRIMINANTS.EXECUTE_CONDITIONAL_ORDERS)]
  );

const priceUsdToOracleAtoms = (priceUsd: string): bigint => {
  const match = /^(?:0|[1-9]\d*)(?:\.(\d{1,3})?)?$/.exec(priceUsd);
  if (!match) {
    throw new Error("Oracle price must be a non-negative decimal string");
  }
  const [whole, fraction = ""] = priceUsd.split(".");
  return BigInt(whole) * 1000n + BigInt(fraction.padEnd(3, "0"));
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

const expectTraderPosition = (
  context: SdkLocalnetContext,
  traderAccount: string,
  label: string
) => {
  const position = readSdkLocalnetEffectiveTraderPosition(
    context,
    traderAccount,
    0
  );
  expect(position, `missing ${label} trader position`).not.toBeNull();
  if (!position) {
    throw new Error(`Missing ${label} trader position`);
  }
  return position;
};

const expectNoTraderPosition = (
  context: SdkLocalnetContext,
  traderAccount: string
): void => {
  const position = readSdkLocalnetEffectiveTraderPosition(
    context,
    traderAccount,
    0
  );
  if (!position) {
    return;
  }
  expect(position.position.baseLotPosition).toBe(0n);
};

const expectOrderbookContainsOrder = (
  context: SdkLocalnetContext,
  symbol: string,
  side: Side,
  orderId: FIFOOrderId
): void => {
  expect(findOrderbookOrder(context, symbol, side, orderId)).toBeDefined();
};

const expectOrderbookDoesNotContainOrder = (
  context: SdkLocalnetContext,
  symbol: string,
  side: Side,
  orderId: FIFOOrderId
): void => {
  expect(findOrderbookOrder(context, symbol, side, orderId)).toBeUndefined();
};

const expectOrderbookHasRestingPrice = (
  context: SdkLocalnetContext,
  symbol: string,
  side: Side,
  priceUsd: string
): void => {
  const priceInTicks = priceUsdToTicksWithMarketParams(
    priceUsd,
    buildSdkLocalnetMarketParams(context, symbol)
  );
  expect(
    orderbookEntriesForSide(context, symbol, side).some(
      (entry) => entry.orderId.priceInTicks === priceInTicks
    )
  ).toBe(true);
};

const expectOrderbookDoesNotHaveRestingPrice = (
  context: SdkLocalnetContext,
  symbol: string,
  side: Side,
  priceUsd: string
): void => {
  const priceInTicks = priceUsdToTicksWithMarketParams(
    priceUsd,
    buildSdkLocalnetMarketParams(context, symbol)
  );
  expect(
    orderbookEntriesForSide(context, symbol, side).some(
      (entry) => entry.orderId.priceInTicks === priceInTicks
    )
  ).toBe(false);
};

const findOrderbookOrder = (
  context: SdkLocalnetContext,
  symbol: string,
  side: Side,
  orderId: FIFOOrderId
) =>
  orderbookEntriesForSide(context, symbol, side).find(
    (entry) =>
      entry.orderId.priceInTicks === orderId.priceInTicks &&
      entry.orderId.orderSequenceNumber === orderId.orderSequenceNumber
  );

const orderbookEntriesForSide = (
  context: SdkLocalnetContext,
  symbol: string,
  side: Side
) => {
  const orderbook = readSdkLocalnetOrderbook(context, symbol);
  return side === Side.Bid ? orderbook.bids : orderbook.asks;
};

const expectAccountClosed = (
  context: Pick<SdkLocalnetContext, "vm">,
  accountAddress: string
): void => {
  const account = context.vm.getAccount(address(accountAddress));
  expect(
    account === null || ("exists" in account && account.exists === false)
  ).toBe(true);
};

const placeLimitOrder = async (
  context: SdkLocalnetContext,
  params: {
    actorName: string;
    symbol: string;
    side: Side;
    priceUsd: string;
    baseUnits: string;
    traderAccount?: TraderAddress;
  }
): Promise<FIFOOrderId> => {
  const actor = context.getActor(params.actorName);
  const marketParams = buildSdkLocalnetMarketParams(context, params.symbol);

  const execution = await expectSuccess(
    context.sendInstructions(
      [
        buildPlaceLimitOrderIxResolved({
          ...placeOrderContext(context, params),
          orderPacket: buildLimitOrderPacketFromMarketParams(
            {
              side: params.side,
              priceUsd: params.priceUsd,
              baseUnits: params.baseUnits,
              selfTradeBehavior: SelfTradeBehavior.CancelProvide,
            },
            marketParams
          ),
        }),
      ],
      {
        feePayerSeed: actor.seed,
        label: `flow:place-limit:${params.actorName}:${params.symbol}`,
      }
    )
  );

  return getOrderIdFromMatchingEngineReturn(execution.metadata.logs());
};

const placeMarketOrder = async (
  context: SdkLocalnetContext,
  params: {
    actorName: string;
    symbol: string;
    side: Side;
    baseUnits: string;
    priceLimitUsd: string;
    traderAccount?: TraderAddress;
  }
): Promise<void> => {
  const actor = context.getActor(params.actorName);
  const marketParams = buildSdkLocalnetMarketParams(context, params.symbol);

  await expectSuccess(
    context.sendInstructions(
      [
        buildPlaceMarketOrderIxResolved({
          ...placeOrderContext(context, params),
          orderPacket: buildMarketOrderPacketFromMarketParams(
            {
              side: params.side,
              priceLimitUsd: params.priceLimitUsd,
              baseUnits: params.baseUnits,
              minBaseUnitsToFill: params.baseUnits,
            },
            marketParams
          ),
        }),
      ],
      {
        feePayerSeed: actor.seed,
        label: `flow:place-market:${params.actorName}:${params.symbol}`,
      }
    )
  );
};

const placeStopLoss = async (
  context: SdkLocalnetContext,
  actorName: string
): Promise<void> => {
  const actor = context.getActor(actorName);
  const marketParams = buildSdkLocalnetMarketParams(context, "BTC");
  const stopLossAccount = await getStopLossAccount(context, actorName);

  await expectSuccess(
    context.sendInstructions(
      [
        buildPlaceStopLossIxResolved({
          ...buildSdkLocalnetPlaceOrderContext(context, {
            actorName,
            symbol: "BTC",
          }),
          stopLossAccount: address(stopLossAccount),
          triggerPrice: priceUsdToTicksWithMarketParams("90000", marketParams),
          executionPrice: priceUsdToTicksWithMarketParams(
            "89000",
            marketParams
          ),
          tradeSide: Side.Ask,
          executionDirection: Direction.LessThan,
          orderKind: StopLossOrderKind.IOC,
        }),
      ],
      {
        feePayerSeed: actor.seed,
        label: `flow:place-stop-loss:${actorName}`,
      }
    )
  );
};

const getStopLossAccount = async (
  context: SdkLocalnetContext,
  actorName: string
): Promise<string> => {
  const actor = context.getActor(actorName);
  return getPhoenixStopLossAddress({
    traderAccount: actor.traderAccount as TraderAddress,
    assetId: 0n,
    phoenixProgramAddress: phoenixProgram(context),
  });
};

const registerAndFundSubaccount = async (
  context: SdkLocalnetContext,
  actorName: string
): Promise<{ actor: FixtureActor; childTrader: TraderAddress }> => {
  const registered = await registerAndSyncSubaccount(context, actorName);
  await transferCollateralToSubaccount(
    context,
    registered.actor,
    registered.childTrader,
    2_000_000_000n
  );
  return registered;
};

const registerAndSyncSubaccount = async (
  context: SdkLocalnetContext,
  actorName: string
): Promise<{ actor: FixtureActor; childTrader: TraderAddress }> => {
  const actor = context.getActor(actorName);
  const childTrader = await getPhoenixTraderSubaccountAddress({
    authority: actor.pubkey as Authority,
    traderPdaIndex: 0,
    subaccountIndex: SUBACCOUNT_INDEX,
    phoenixProgramAddress: phoenixProgram(context),
  });

  await expectSuccess(
    context.sendInstructions(
      [
        buildRegisterTraderIxResolved({
          exchange: traderExchangeContext(context),
          trader: {
            payer: actor.pubkey as Authority,
            authority: actor.pubkey as Authority,
            traderAccount: childTrader,
          },
          maxPositions: 1n,
          traderPdaIndex: 0,
          traderSubaccountIndex: SUBACCOUNT_INDEX,
        }),
      ],
      {
        feePayerSeed: actor.seed,
        label: "flow:register-subaccount",
      }
    )
  );

  await expectSuccess(
    context.sendInstructions(
      [
        buildSyncParentToChildIxResolved({
          exchange: {
            ...traderExchangeContext(context),
            globalTraderIndex: context.fixture.addresses
              .globalTraderIndex as never,
          },
          trader: {
            authority: actor.pubkey as Authority,
            parentTraderAccount: actor.traderAccount as TraderAddress,
            childTraderAccount: childTrader,
          },
        }),
      ],
      {
        feePayerSeed: actor.seed,
        label: "flow:sync-parent-to-child",
      }
    )
  );

  return { actor, childTrader };
};

const transferCollateralToSubaccount = async (
  context: SdkLocalnetContext,
  actor: FixtureActor,
  childTrader: TraderAddress,
  amount: bigint
): Promise<void> => {
  await expectSuccess(
    context.sendInstructions(
      [
        buildTransferCollateralIxResolved({
          exchange: collateralExchangeContext(context),
          trader: {
            authority: actor.pubkey as Authority,
            srcTraderAccount: actor.traderAccount as TraderAddress,
            dstTraderAccount: childTrader,
          },
          amount,
        }),
      ],
      {
        feePayerSeed: actor.seed,
        label: "flow:transfer-collateral-to-subaccount",
      }
    )
  );
};

const openAndCloseIsolatedPosition = async (
  context: SdkLocalnetContext,
  actor: FixtureActor,
  childTrader: TraderAddress
): Promise<void> => {
  await placeMarketOrder(context, {
    actorName: actor.name,
    symbol: "BTC",
    side: Side.Bid,
    baseUnits: "0.01",
    priceLimitUsd: "101000",
    traderAccount: childTrader,
  });
  await placeMarketOrder(context, {
    actorName: actor.name,
    symbol: "BTC",
    side: Side.Ask,
    baseUnits: "0.01",
    priceLimitUsd: "99000",
    traderAccount: childTrader,
  });
};

const updateTraderState = async (
  context: SdkLocalnetContext,
  actor: FixtureActor,
  traderAccount: TraderAddress
): Promise<void> => {
  await expectSuccess(
    context.sendInstructions(
      [
        buildUpdateTraderStateIx({
          ...phoenixAccounts(context),
          trader: actor.pubkey as Authority,
          traderAccount,
          perpAssetMap: context.fixture.addresses.perpAssetMap as never,
          globalTraderIndex: context.fixture.addresses
            .globalTraderIndex as never,
          activeTraderBuffer: context.fixture.addresses
            .activeTraderBuffer as never,
        }),
      ],
      {
        feePayerSeed: actor.seed,
        label: "flow:update-trader-state",
      }
    )
  );
};

const placeOrderContext = (
  context: SdkLocalnetContext,
  params: { actorName: string; symbol: string; traderAccount?: TraderAddress }
): ReturnType<typeof buildSdkLocalnetPlaceOrderContext> => {
  const resolved = buildSdkLocalnetPlaceOrderContext(context, params);
  if (!params.traderAccount) return resolved;

  return {
    ...resolved,
    trader: {
      ...resolved.trader,
      traderAccount: params.traderAccount,
    },
  };
};

const readAccountData = (
  context: Pick<SdkLocalnetContext, "vm">,
  accountAddress: string
) => {
  const account = context.vm.getAccount(address(accountAddress));
  expect(account).not.toBeNull();
  if (!account) throw new Error(`Missing account ${accountAddress}`);
  return account.data;
};

const readSplTokenAmount = (
  context: Pick<SdkLocalnetContext, "vm">,
  tokenAccount: string
): bigint => {
  const account = context.vm.getAccount(address(tokenAccount));
  expect(account).not.toBeNull();
  if (!account) throw new Error(`Missing SPL token account ${tokenAccount}`);

  return new DataView(
    account.data.buffer,
    account.data.byteOffset + 64,
    8
  ).getBigUint64(0, true);
};

const readLamports = (
  context: Pick<SdkLocalnetContext, "vm">,
  accountAddress: string
): bigint => {
  const account = context.vm.getAccount(address(accountAddress));
  expect(account).not.toBeNull();
  if (!account || !("lamports" in account)) {
    throw new Error(`Missing account ${accountAddress}`);
  }
  return account.lamports;
};

const phoenixProgram = (context: SdkLocalnetContext): PhoenixProgramAddress =>
  context.fixture.programs.phoenixEternal as PhoenixProgramAddress;

const phoenixAccounts = (context: SdkLocalnetContext) => ({
  phoenixProgramAddress: phoenixProgram(context),
  logAuthorityAddress: context.fixture.addresses.logAuthority as never,
  globalConfigurationAddress: context.fixture.addresses.globalConfig as never,
});

const traderExchangeContext = (context: SdkLocalnetContext) =>
  phoenixAccounts(context);

const collateralExchangeContext = (context: SdkLocalnetContext) => ({
  ...phoenixAccounts(context),
  perpAssetMap: context.fixture.addresses.perpAssetMap as never,
  globalTraderIndex: context.fixture.addresses.globalTraderIndex as never,
  activeTraderBuffer: context.fixture.addresses.activeTraderBuffer as never,
});

const getOrderIdFromMatchingEngineReturn = (
  logs: readonly string[]
): FIFOOrderId => {
  const returnLog = [...logs]
    .reverse()
    .find((log) => log.startsWith("Program return: "));
  if (!returnLog) {
    throw new Error("Missing matching engine return data");
  }

  const dataBase64 = returnLog.split(" ").at(-1);
  if (!dataBase64) {
    throw new Error(`Malformed matching engine return log: ${returnLog}`);
  }

  const data = Buffer.from(dataBase64, "base64");
  const priceInTicks = data.readBigUInt64LE(0);
  const orderSequenceNumber = data.readBigUInt64LE(8);
  if (priceInTicks === 0n && orderSequenceNumber === 0n) {
    throw new Error("Matching engine did not return a resting order id");
  }

  return {
    priceInTicks: priceInTicks as never,
    orderSequenceNumber,
  };
};
