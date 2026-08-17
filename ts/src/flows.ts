import { fetchPermission, fetchPerpAssetMap } from "@/accounts";
import {
  type PhoenixAccountExistenceClient,
  type PhoenixInstructionClient,
  type PhoenixMarketDataClient,
} from "@/core/clientTypes";
import { OrderFlags, SelfTradeBehavior } from "@/primitives/OrderPacket";
import {
  allocateIsolatedSubaccount,
  buildCreateAssociatedTokenAccountIdempotent,
  buildCreateAssociatedTokenAccountIdempotentSync,
  buildSplTokenApprove,
  buildSplTokenTransfer,
  DEFAULT_MARKET_ORDER_SLIPPAGE,
  fetchRequiredAccounts,
  fetchSubaccountForAsset,
  getMarketMetadataForSymbol,
} from "@/core/helpers";
import { clientPhoenixInstructionAddresses } from "@/core/constants";
import {
  buildCreatePermissionIx,
  buildSetPermissionIx,
  DEPOSIT_PERMISSION,
} from "@/core/permissionInstructions";
import {
  buildDepositFunds,
  buildEmberDeposit,
  buildEmberWithdraw,
  buildRegisterTrader,
  buildSyncParentToChild,
  buildTransferCollateral,
  buildTransferCollateralChildToParent,
  buildWithdrawFunds,
} from "@/builders";
import {
  type Authority,
  MarginType,
  type MarketAddress,
  type PerpAssetMapAddress,
  quoteLots,
  Side,
  type Symbol,
  type Ticks,
  ticks,
  type BaseLots,
  type QuoteLots,
  type InstructionsWithAccountsAndData,
  type TokenAccountAddress,
  type TraderAddress,
} from "@/primitives";
import {
  getPhoenixPermissionAddress,
  getPhoenixSplineCollectionAddress,
  getPhoenixTraderSubaccountAddress,
  getPhoenixTraderTokenAccountAddress,
} from "@/pdas";
import {
  buildFlameDepositToPhoenixIx,
  deriveFlameDepositAddresses,
} from "@/flame";
import { address, type Address } from "@solana/kit";
import { buildPlaceLimitOrderIx } from "./core/ixBuilders/PlaceLimitOrder";
import { buildPlaceMarketOrderIx } from "./core/ixBuilders/PlaceMarketOrder";
import {
  buildPlaceMultiLimitOrderIx,
  buildPlaceMultiLimitOrderV2Ix,
} from "./core/ixBuilders/PlaceMultiLimitOrder";
import { buildPlacePostOnlyOrderIx } from "./core/ixBuilders/PlacePostOnlyOrder";
import {
  chunkScaleLevelsForTx,
  MAX_SCALE_ORDERS,
  scaleLevelsToMultipleOrderPacket,
  scaleLevelsToMultipleOrderPacketV2,
  type ScaleOrderLevel,
} from "./scaleOrders";
import { isFlightClient } from "./flight/client.js";

const ALLOW_ESCROW_REQUEST_PERMISSION = 1n << 8n;

type SponsorshipUserIdentifier = {
  userId?: string;
  userPubkey?: Authority;
};

interface BaseDepositFlowParams {
  authority: Authority;
  amount: bigint;
  traderPdaIndex?: number;
}

type SponsoredDepositFlowParams = BaseDepositFlowParams &
  SponsorshipUserIdentifier & {
    feePayer: Authority;
    sponsorshipToken: string;
  };

interface NonSponsoredDepositFlowParams extends BaseDepositFlowParams {
  feePayer?: null;
}

export type DepositFlowParams =
  | SponsoredDepositFlowParams
  | NonSponsoredDepositFlowParams;

export interface DepositFlowInstructions {
  createAta: InstructionsWithAccountsAndData;
  emberDeposit: InstructionsWithAccountsAndData;
  depositFunds: InstructionsWithAccountsAndData;
}

export interface DepositFlowResult {
  instructions: InstructionsWithAccountsAndData[];
  named: DepositFlowInstructions;
}

export type FlameDepositFundingFlowParams = DepositFlowParams;

export interface FlameDepositFundingFlowInstructions {
  createProxyAta: InstructionsWithAccountsAndData;
  transferUsdcToProxy: InstructionsWithAccountsAndData;
}

export interface FlameDepositFundingFlowResult {
  instructions: InstructionsWithAccountsAndData[];
  named: FlameDepositFundingFlowInstructions;
  proxyAuthority: Authority;
  depositAddress: TokenAccountAddress;
  proxyAta: TokenAccountAddress;
  traderPdaIndex: number;
}

export type FlameAtomicDepositFlowParams = BaseDepositFlowParams &
  SponsorshipUserIdentifier & {
    feePayer: Authority;
    sponsorshipToken: string;
  };

export interface FlameAtomicDepositFlowInstructions extends FlameDepositFundingFlowInstructions {
  createPermission: InstructionsWithAccountsAndData;
  setPermission: InstructionsWithAccountsAndData;
  depositToPhoenix: InstructionsWithAccountsAndData;
}

export interface FlameAtomicDepositFlowResult {
  instructions: InstructionsWithAccountsAndData[];
  named: FlameAtomicDepositFlowInstructions;
  proxyAuthority: Authority;
  depositAddress: TokenAccountAddress;
  proxyAta: TokenAccountAddress;
  traderPdaIndex: number;
  traderSubaccountIndex: number;
}

const resolveFlowPayer = (params: {
  authority: Authority;
  feePayer?: Authority | null;
}): Authority => {
  const sponsorOverride = process.env.PHOENIX_TEST_SPONSOR_FEE_PAYER;
  return params.feePayer != null
    ? sponsorOverride
      ? (address(sponsorOverride) as Authority)
      : params.feePayer
    : params.authority;
};

interface BaseWithdrawFlowParams {
  authority: Authority;
  amount: bigint;
}

type SponsoredWithdrawFlowParams = BaseWithdrawFlowParams &
  SponsorshipUserIdentifier & {
    feePayer: Authority;
    sponsorshipToken: string;
  };

interface NonSponsoredWithdrawFlowParams extends BaseWithdrawFlowParams {
  feePayer?: null;
}

export type WithdrawFlowParams =
  | SponsoredWithdrawFlowParams
  | NonSponsoredWithdrawFlowParams;

export interface WithdrawFlowInstructions {
  createPhoenixAta: InstructionsWithAccountsAndData;
  approveToken: InstructionsWithAccountsAndData;
  createUsdcAta: InstructionsWithAccountsAndData;
  withdrawFunds: InstructionsWithAccountsAndData;
  emberWithdraw: InstructionsWithAccountsAndData;
}

export interface WithdrawFlowResult {
  instructions: InstructionsWithAccountsAndData[];
  named: WithdrawFlowInstructions;
}

export interface PlaceLimitOrderFlowParams {
  authority: Authority;
  positionAuthority?: Authority;
  symbol: Symbol;
  side: Side;
  priceInTicks: Ticks;
  numBaseLots: BaseLots;
  marginType: MarginType;
  transferAmount?: bigint;
  subaccountIndex?: number;
  pdaIndex?: number;
  isReduceOnly?: boolean;
  isPostOnly?: boolean;
  slide?: boolean;
  skipTransferToParent?: boolean;
}

export interface PlaceLimitOrderFlowInstructions {
  transferCollateral?: InstructionsWithAccountsAndData;
  placeLimitOrder: InstructionsWithAccountsAndData;
  transferCollateralChildToParent?: InstructionsWithAccountsAndData;
}

export interface PlaceLimitOrderFlowResult {
  instructions: InstructionsWithAccountsAndData[];
  named: PlaceLimitOrderFlowInstructions;
  subaccountIndex: number;
}

export interface PlaceMarketOrderFlowParams {
  authority: Authority;
  positionAuthority?: Authority;
  symbol: Symbol;
  side: Side;
  numBaseLots: BaseLots;
  marginType: MarginType;
  transferAmount?: bigint;
  subaccountIndex?: number;
  pdaIndex?: number;
  isReduceOnly?: boolean;
  priceInTicksLimit?: Ticks;
  minBaseLotsToFill?: BaseLots;
  minQuoteLotsToFill?: QuoteLots;
  skipTransferToParent?: boolean;
}

export interface PlaceMarketOrderFlowInstructions {
  transferCollateral?: InstructionsWithAccountsAndData;
  placeMarketOrder: InstructionsWithAccountsAndData;
  transferCollateralChildToParent?: InstructionsWithAccountsAndData;
}

export interface PlaceMarketOrderFlowResult {
  instructions: InstructionsWithAccountsAndData[];
  named: PlaceMarketOrderFlowInstructions;
  subaccountIndex: number;
}

export interface PlaceMultiLimitOrderFlowParams {
  authority: Authority;
  positionAuthority?: Authority;
  /**
   * Sponsor fee payer. Isolated-only: when a fresh child subaccount must be
   * registered, this account pays the trader-account rent and signs the
   * register instruction (matching the backend `place-isolated-*` endpoints).
   * Defaults to `authority` — which fails for sponsored + delegated sessions,
   * where neither the sponsor nor the delegate can sign for `authority`.
   */
  feePayer?: Authority | null;
  symbol: Symbol;
  side: Side;
  /** Pre-computed ladder levels (e.g. from `computeScaleOrderLevels`). */
  levels: ScaleOrderLevel[];
  marginType: MarginType;
  /** Collateral to move into the isolated child before placing (cross ignores). */
  transferAmount?: bigint;
  subaccountIndex?: number;
  pdaIndex?: number;
  slide?: boolean;
  clientOrderId?: bigint | null;
  /**
   * Max sub-orders per transaction; defaults to `DEFAULT_MAX_ORDERS_PER_TX`,
   * or `DEFAULT_MAX_ORDERS_PER_TX_V2` on the V2 instruction path.
   */
  maxOrdersPerTx?: number;
  skipTransferToParent?: boolean;
  /**
   * Caller-assigned ladder id in `1..=255`, stamped onto every resting leg.
   * Setting this (non-zero) or `reduceOnly` routes the batch through
   * `place_multi_limit_order_v2`. The caller owns uniqueness — a duplicate id
   * among the trader's resting orders on this market fails the whole batch
   * on-chain. A tagged ladder must fit one transaction; the flow throws if
   * `levels` chunk into more than one.
   */
  scaleSetId?: number;
  /** Marks every leg of the ladder reduce-only. Also routes through `place_multi_limit_order_v2`. */
  reduceOnly?: boolean;
}

export interface PlaceMultiLimitOrderFlowBatchInstructions {
  /** Isolated-only: registers the child subaccount when it does not exist yet. */
  registerTrader?: InstructionsWithAccountsAndData;
  /** Isolated-only: syncs parent (cross) state into the child before funding. */
  syncParentToChild?: InstructionsWithAccountsAndData;
  transferCollateral?: InstructionsWithAccountsAndData;
  placeMultiLimitOrder: InstructionsWithAccountsAndData;
  transferCollateralChildToParent?: InstructionsWithAccountsAndData;
}

/**
 * One transaction's worth of instructions. A large ladder spans several
 * batches; the caller submits each batch as its own transaction (adding a
 * compute-budget instruction sized to the order count). Batches are NOT atomic
 * across transactions — a later failure can leave a partial ladder, and for
 * isolated margin, collateral funded but not yet swept back.
 */
export interface PlaceMultiLimitOrderFlowBatch {
  instructions: InstructionsWithAccountsAndData[];
  named: PlaceMultiLimitOrderFlowBatchInstructions;
}

export interface PlaceMultiLimitOrderFlowResult {
  batches: PlaceMultiLimitOrderFlowBatch[];
  subaccountIndex: number;
}

const validatePlaceMarketOrderFlowMinimums = ({
  numBaseLots,
  minBaseLotsToFill,
  minQuoteLotsToFill,
}: Pick<
  PlaceMarketOrderFlowParams,
  "numBaseLots" | "minBaseLotsToFill" | "minQuoteLotsToFill"
>) => {
  if (minBaseLotsToFill !== undefined && minBaseLotsToFill < 0n) {
    throw new Error(
      `minBaseLotsToFill must be non-negative, got ${minBaseLotsToFill.toString()}`
    );
  }

  if (minQuoteLotsToFill !== undefined && minQuoteLotsToFill < 0n) {
    throw new Error(
      `minQuoteLotsToFill must be non-negative, got ${minQuoteLotsToFill.toString()}`
    );
  }

  if (minBaseLotsToFill !== undefined && minBaseLotsToFill > numBaseLots) {
    throw new Error(
      `minBaseLotsToFill must be <= numBaseLots, got minBaseLotsToFill=${minBaseLotsToFill.toString()} and numBaseLots=${numBaseLots.toString()}`
    );
  }
};

export interface GrantEscrowPermissionFlowParams {
  permissionAuthority: Address;
  delegatedKey: Address;
  payer?: Address;
  expiresAtTimestamp?: bigint | null;
  allowedSignerActions?: bigint | null;
}

export interface GrantEscrowPermissionFlowInstructions {
  createPermission?: InstructionsWithAccountsAndData;
  setPermission: InstructionsWithAccountsAndData;
}

export interface GrantEscrowPermissionFlowResult {
  instructions: InstructionsWithAccountsAndData[];
  named: GrantEscrowPermissionFlowInstructions;
  permissionPda: Address;
}

export interface RevokeEscrowPermissionFlowParams {
  permissionAuthority: Address;
  delegatedKey: Address;
}

export interface RevokeEscrowPermissionFlowInstructions {
  setPermission: InstructionsWithAccountsAndData;
}

export interface RevokeEscrowPermissionFlowResult {
  instructions: InstructionsWithAccountsAndData[];
  named: RevokeEscrowPermissionFlowInstructions;
  permissionPda: Address;
}

export const buildDepositFlow = async (
  params: DepositFlowParams,
  client: PhoenixInstructionClient
): Promise<DepositFlowResult> => {
  const { authority, amount, traderPdaIndex = 0 } = params;
  const { canonicalTokenMintKey: phoenixMint } =
    await fetchRequiredAccounts(client);

  const payer = resolveFlowPayer(params);

  const [createAta, emberDeposit, depositFunds] = await Promise.all([
    buildCreateAssociatedTokenAccountIdempotent({
      payer,
      owner: authority,
      mint: phoenixMint,
    }),
    buildEmberDeposit({ authority, amount }, client),
    buildDepositFunds({ authority, amount }, client, traderPdaIndex),
  ]);

  return {
    instructions: [createAta, emberDeposit, depositFunds],
    named: {
      createAta,
      emberDeposit,
      depositFunds,
    },
  };
};

export const buildFlameDepositFundingFlow = async (
  params: FlameDepositFundingFlowParams,
  client: PhoenixInstructionClient
): Promise<FlameDepositFundingFlowResult> => {
  const { authority, amount, traderPdaIndex = 0 } = params;
  const payer = resolveFlowPayer(params);
  const { depositAddress, proxyAuthority, proxyAta } =
    await deriveFlameDepositAddresses({
      userAuthority: authority,
      traderPdaIndex,
      mintAddress: client.addresses.usdcMintAddress,
      phoenixProgramAddress: client.addresses.phoenixProgramAddress,
    });
  const userUsdcAta = await getPhoenixTraderTokenAccountAddress(
    authority,
    client.addresses.usdcMintAddress
  );

  const createProxyAta = buildCreateAssociatedTokenAccountIdempotentSync({
    payer,
    ataAddress: proxyAta,
    owner: proxyAuthority,
    mint: client.addresses.usdcMintAddress,
  });
  const transferUsdcToProxy = buildSplTokenTransfer({
    owner: authority,
    sourceTokenAccount: userUsdcAta,
    destinationTokenAccount: proxyAta,
    amount,
  });

  return {
    instructions: [createProxyAta, transferUsdcToProxy],
    named: {
      createProxyAta,
      transferUsdcToProxy,
    },
    proxyAuthority,
    depositAddress,
    proxyAta,
    traderPdaIndex,
  };
};

export const buildFlameAtomicDepositFlow = async (
  params: FlameAtomicDepositFlowParams,
  client: PhoenixInstructionClient
): Promise<FlameAtomicDepositFlowResult> => {
  const { authority, traderPdaIndex = 0 } = params;
  const requestedTraderSubaccountIndex = (
    params as FlameAtomicDepositFlowParams & { traderSubaccountIndex?: number }
  ).traderSubaccountIndex;
  if (
    requestedTraderSubaccountIndex != null &&
    requestedTraderSubaccountIndex !== 0
  ) {
    throw new Error(
      "Flame atomic deposit sponsorship only supports traderSubaccountIndex 0"
    );
  }
  const traderSubaccountIndex = 0;
  const payer = resolveFlowPayer(params);
  // The sponsor fee payer cranks the deposit so wallets holding no SOL can
  // deposit: the crank fronts rent for the transient proxy Phoenix ATA and is
  // refunded by the same instruction's close, so net sponsor spend is zero.
  const crank = payer;
  const [
    { canonicalTokenMintKey, arenaAddresses, globalTraderIndexAddresses },
    funding,
  ] = await Promise.all([
    fetchRequiredAccounts(client),
    buildFlameDepositFundingFlow(params, client),
  ]);
  const phoenixAddresses = clientPhoenixInstructionAddresses(client);
  const permissionPda = await getPhoenixPermissionAddress(
    authority,
    funding.proxyAuthority,
    client.addresses.phoenixProgramAddress
  );
  const createPermission = buildCreatePermissionIx({
    ...phoenixAddresses,
    payer,
    permissionAuthority: authority,
    delegatedKey: funding.proxyAuthority,
    permissionPda,
  });
  const setPermission = buildSetPermissionIx({
    ...phoenixAddresses,
    permissionAuthority: authority,
    delegatedKey: funding.proxyAuthority,
    permissionPda,
    permission: DEPOSIT_PERMISSION,
    expiresAtTimestamp: null,
    allowedSignerActions: null,
  });
  const depositToPhoenix = await buildFlameDepositToPhoenixIx({
    crank,
    userAuthority: authority,
    inputMint: client.addresses.usdcMintAddress,
    outputMint: canonicalTokenMintKey,
    globalTraderIndex: globalTraderIndexAddresses,
    activeTraderBuffer: arenaAddresses,
    traderPdaIndex,
    traderSubaccountIndex,
    phoenixProgramAddress: client.addresses.phoenixProgramAddress,
    logAuthorityAddress: client.addresses.logAuthorityAddress,
    globalConfigurationAddress: client.addresses.globalConfigurationAddress,
  });

  return {
    instructions: [
      funding.named.createProxyAta,
      funding.named.transferUsdcToProxy,
      createPermission,
      setPermission,
      depositToPhoenix,
    ],
    named: {
      createProxyAta: funding.named.createProxyAta,
      transferUsdcToProxy: funding.named.transferUsdcToProxy,
      createPermission,
      setPermission,
      depositToPhoenix,
    },
    proxyAuthority: funding.proxyAuthority,
    depositAddress: funding.depositAddress,
    proxyAta: funding.proxyAta,
    traderPdaIndex,
    traderSubaccountIndex,
  };
};

export const buildWithdrawFlow = async (
  params: WithdrawFlowParams,
  client: PhoenixInstructionClient
): Promise<WithdrawFlowResult> => {
  const { authority, amount } = params;
  const { canonicalTokenMintKey: phoenixMint } =
    await fetchRequiredAccounts(client);

  const payer = resolveFlowPayer(params);

  const [
    phoenixAta,
    createPhoenixAta,
    createUsdcAta,
    withdrawIx,
    emberWithdraw,
  ] = await Promise.all([
    getPhoenixTraderTokenAccountAddress(authority, phoenixMint),
    buildCreateAssociatedTokenAccountIdempotent({
      payer,
      owner: authority,
      mint: phoenixMint,
    }),
    buildCreateAssociatedTokenAccountIdempotent({
      payer,
      owner: authority,
      mint: client.addresses.usdcMintAddress,
    }),
    buildWithdrawFunds({ authority, amount }, client),
    buildEmberWithdraw({ authority, amount }, client),
  ]);

  const approve = buildSplTokenApprove({
    owner: authority,
    tokenAccount: phoenixAta,
    delegate: client.addresses.emberStateAddress,
    amount,
  });

  return {
    instructions: [
      createPhoenixAta,
      approve,
      createUsdcAta,
      withdrawIx,
      emberWithdraw,
    ],
    named: {
      createPhoenixAta,
      approveToken: approve,
      createUsdcAta,
      withdrawFunds: withdrawIx,
      emberWithdraw,
    },
  };
};

const resolveMarketMetadata = async (
  marketSymbol: Symbol,
  perpAssetMapKey: PerpAssetMapAddress,
  client: PhoenixInstructionClient
): Promise<{
  assetId: number;
  marketAccount: MarketAddress;
  availableSymbols: string[];
}> => {
  const cached = await getMarketMetadataForSymbol(marketSymbol, client).catch(
    () => null
  );
  if (cached) {
    const snapshot = client.exchange?.snapshot();
    return {
      assetId: Number(cached.assetId),
      marketAccount: cached.marketAddress,
      availableSymbols: snapshot?.markets.map((market) => market.symbol) ?? [],
    };
  }

  const perpAssetMap = await fetchPerpAssetMap({
    client,
    address: perpAssetMapKey,
  });

  let assetId: number | undefined;
  let marketAccount: MarketAddress | undefined;

  for (const { key, value } of perpAssetMap.metadata.entries) {
    if (key.toUpperCase() !== marketSymbol.toUpperCase()) {
      continue;
    }
    assetId = Number(value.staticMarketParams.assetId);
    marketAccount = value.staticMarketParams.marketAccount;
    break;
  }

  if (assetId === undefined || !marketAccount) {
    throw new Error(
      `Market not found for symbol: ${marketSymbol}. Available symbols: ${Array.from(
        perpAssetMap.metadata.entries,
        ({ key }) => key
      ).join(", ")}`
    );
  }

  return {
    assetId,
    marketAccount,
    availableSymbols: Array.from(
      perpAssetMap.metadata.entries,
      ({ key }) => key
    ),
  };
};

const resolveSubaccount = async (
  client: PhoenixInstructionClient,
  perpAssetMapKey: PerpAssetMapAddress,
  authority: Authority,
  marketSymbol: Symbol,
  marginType: MarginType,
  explicitSubaccountIndex: number | undefined,
  transferAmount: bigint | undefined,
  pdaIndex: number
) => {
  if (
    marginType === MarginType.Cross &&
    explicitSubaccountIndex === undefined
  ) {
    throw new Error(
      "For cross margin orders, you must specify subaccountIndex. Use index 0 for the default cross margin account."
    );
  }

  if (
    marginType === MarginType.Isolated &&
    explicitSubaccountIndex === undefined &&
    transferAmount === undefined
  ) {
    throw new Error(
      "transfer_amount is required for isolated accounts when subaccount_index is not specified. This ensures you have sufficient margin for your position."
    );
  }

  const { assetId, marketAccount } = await resolveMarketMetadata(
    marketSymbol,
    perpAssetMapKey,
    client
  );

  if (explicitSubaccountIndex !== undefined) {
    return {
      subaccountIndex: explicitSubaccountIndex,
      subaccountAddress: await getPhoenixTraderSubaccountAddress({
        authority,
        traderPdaIndex: pdaIndex,
        subaccountIndex: explicitSubaccountIndex,
        phoenixProgramAddress: client.addresses.phoenixProgramAddress,
      }),
      assetId,
      marketAccount,
    };
  }

  const subaccountInfo = await fetchSubaccountForAsset(
    client,
    authority,
    pdaIndex,
    marginType,
    assetId
  );

  return {
    subaccountIndex: subaccountInfo.index,
    subaccountAddress: subaccountInfo.address,
    assetId,
    marketAccount,
  };
};

export const buildPlaceLimitOrderFlow = async (
  params: PlaceLimitOrderFlowParams,
  client: PhoenixInstructionClient
): Promise<PlaceLimitOrderFlowResult> => {
  const {
    authority,
    positionAuthority,
    symbol: marketSymbol,
    side,
    priceInTicks,
    numBaseLots,
    marginType,
    transferAmount,
    subaccountIndex: explicitSubaccountIndex,
    pdaIndex = 0,
    isReduceOnly,
    isPostOnly,
    slide = false,
    skipTransferToParent = false,
  } = params;

  const { perpAssetMapKey, arenaAddresses, globalTraderIndexAddresses } =
    await fetchRequiredAccounts(client);
  const { subaccountIndex, subaccountAddress, marketAccount } =
    await resolveSubaccount(
      client,
      perpAssetMapKey,
      authority,
      marketSymbol,
      marginType,
      explicitSubaccountIndex,
      transferAmount,
      pdaIndex
    );

  const instructions: PlaceLimitOrderFlowResult["instructions"] = [];
  const named: PlaceLimitOrderFlowResult["named"] =
    {} as PlaceLimitOrderFlowResult["named"];

  if (transferAmount !== undefined && transferAmount > 0n) {
    const transferIx = await buildTransferCollateral(
      {
        authority,
        positionAuthority,
        srcSubaccountIndex: 0,
        dstSubaccountIndex: subaccountIndex,
        traderPdaIndex: pdaIndex,
        amount: transferAmount,
      },
      client
    );
    instructions.push(transferIx);
    named.transferCollateral = transferIx;
  }

  const splineCollection = await getPhoenixSplineCollectionAddress(
    marketAccount,
    client.addresses.phoenixProgramAddress
  );
  const orderFlags = isReduceOnly ? OrderFlags.ReduceOnly : OrderFlags.None;

  // Effective signer of the placement instruction; the Flight wrap must name
  // the same wallet and take the position-authority path whenever it is not
  // the owner.
  const signer = positionAuthority ?? authority;

  const placeOrderIx = isPostOnly
    ? buildPlacePostOnlyOrderIx({
        ...clientPhoenixInstructionAddresses(client),
        trader: signer,
        traderAccount: subaccountAddress,
        perpAssetMap: perpAssetMapKey,
        orderbook: marketAccount,
        splineCollection,
        activeTraderBuffer: arenaAddresses,
        globalTraderIndex: globalTraderIndexAddresses,
        orderPacket: {
          side,
          priceInTicks,
          numBaseLots,
          clientOrderId: 0n,
          slide,
          lastValidSlot: null,
          orderFlags,
          cancelExisting: false,
        },
      })
    : buildPlaceLimitOrderIx({
        ...clientPhoenixInstructionAddresses(client),
        trader: signer,
        traderAccount: subaccountAddress,
        perpAssetMap: perpAssetMapKey,
        orderbook: marketAccount,
        splineCollection,
        activeTraderBuffer: arenaAddresses,
        globalTraderIndex: globalTraderIndexAddresses,
        orderPacket: {
          side,
          priceInTicks,
          numBaseLots,
          selfTradeBehavior: SelfTradeBehavior.CancelProvide,
          matchLimit: null,
          clientOrderId: 0n,
          lastValidSlot: null,
          orderFlags,
          cancelExisting: false,
        },
      });

  const maybeWrappedIx = isFlightClient(client)
    ? await client.tryWrapOrderInstruction(
        placeOrderIx,
        signer,
        signer !== authority
      )
    : placeOrderIx;

  instructions.push(maybeWrappedIx);
  named.placeLimitOrder = maybeWrappedIx;

  if (!skipTransferToParent && subaccountIndex > 0) {
    const transferToParentIx = await buildTransferCollateralChildToParent(
      {
        authority,
        positionAuthority,
        traderPdaIndex: pdaIndex,
        childSubaccountIndex: subaccountIndex,
      },
      client
    );
    instructions.push(transferToParentIx);
    named.transferCollateralChildToParent = transferToParentIx;
  }

  return {
    instructions,
    named,
    subaccountIndex,
  };
};

export const buildPlaceMarketOrderFlow = async (
  params: PlaceMarketOrderFlowParams,
  client: PhoenixInstructionClient & PhoenixMarketDataClient
): Promise<PlaceMarketOrderFlowResult> => {
  const {
    authority,
    positionAuthority,
    symbol: marketSymbol,
    side,
    numBaseLots,
    marginType,
    transferAmount,
    subaccountIndex: explicitSubaccountIndex,
    pdaIndex = 0,
    isReduceOnly,
    priceInTicksLimit,
    minBaseLotsToFill,
    minQuoteLotsToFill,
    skipTransferToParent = false,
  } = params;

  validatePlaceMarketOrderFlowMinimums({
    numBaseLots,
    minBaseLotsToFill,
    minQuoteLotsToFill,
  });

  const { perpAssetMapKey, arenaAddresses, globalTraderIndexAddresses } =
    await fetchRequiredAccounts(client);
  const { subaccountIndex, subaccountAddress, marketAccount } =
    await resolveSubaccount(
      client,
      perpAssetMapKey,
      authority,
      marketSymbol,
      marginType,
      explicitSubaccountIndex,
      transferAmount,
      pdaIndex
    );

  const instructions: PlaceMarketOrderFlowResult["instructions"] = [];
  const named: PlaceMarketOrderFlowResult["named"] =
    {} as PlaceMarketOrderFlowResult["named"];

  if (transferAmount !== undefined && transferAmount > 0n) {
    const transferIx = await buildTransferCollateral(
      {
        authority,
        positionAuthority,
        srcSubaccountIndex: 0,
        dstSubaccountIndex: subaccountIndex,
        traderPdaIndex: pdaIndex,
        amount: transferAmount,
      },
      client
    );
    instructions.push(transferIx);
    named.transferCollateral = transferIx;
  }

  const splineCollection = await getPhoenixSplineCollectionAddress(
    marketAccount,
    client.addresses.phoenixProgramAddress
  );

  let resolvedPriceInTicks = priceInTicksLimit ?? null;
  if (resolvedPriceInTicks === null) {
    const marketView = await client.markets.getMarket(marketSymbol);
    const orderbook = marketView.market.l2Orderbook;
    const mid = orderbook.mid;

    if (mid === undefined || mid === null) {
      throw new Error(
        `No mid price available for ${marketSymbol}; cannot compute market order limit`
      );
    }

    const limitPriceUsd =
      side === Side.Bid
        ? mid * (1 + DEFAULT_MARKET_ORDER_SLIPPAGE)
        : mid * (1 - DEFAULT_MARKET_ORDER_SLIPPAGE);
    const { tickSizeInQuoteLotsPerBaseLot: tickSize, baseLotsDecimals } =
      marketView.market.units;
    const priceTicks =
      (limitPriceUsd * 1_000_000) / (tickSize * Math.pow(10, baseLotsDecimals));
    resolvedPriceInTicks = ticks(BigInt(Math.floor(priceTicks)));
  }

  // Effective signer of the placement instruction; the Flight wrap must name
  // the same wallet and take the position-authority path whenever it is not
  // the owner.
  const signer = positionAuthority ?? authority;

  const placeOrderIx = buildPlaceMarketOrderIx({
    ...clientPhoenixInstructionAddresses(client),
    trader: signer,
    traderAccount: subaccountAddress,
    perpAssetMap: perpAssetMapKey,
    orderbook: marketAccount,
    splineCollection,
    activeTraderBuffer: arenaAddresses,
    globalTraderIndex: globalTraderIndexAddresses,
    orderPacket: {
      side,
      priceInTicks: resolvedPriceInTicks,
      numBaseLots,
      numQuoteLots: null,
      minBaseLotsToFill: minBaseLotsToFill ?? numBaseLots,
      minQuoteLotsToFill: minQuoteLotsToFill ?? quoteLots(1),
      selfTradeBehavior: SelfTradeBehavior.Abort,
      matchLimit: null,
      clientOrderId: 0n,
      lastValidSlot: null,
      orderFlags: isReduceOnly ? OrderFlags.ReduceOnly : OrderFlags.None,
      cancelExisting: false,
    },
  });

  const maybeWrappedIx = isFlightClient(client)
    ? await client.tryWrapOrderInstruction(
        placeOrderIx,
        signer,
        signer !== authority
      )
    : placeOrderIx;

  instructions.push(maybeWrappedIx);
  named.placeMarketOrder = maybeWrappedIx;

  if (!skipTransferToParent && subaccountIndex > 0) {
    const transferToParentIx = await buildTransferCollateralChildToParent(
      {
        authority,
        positionAuthority,
        traderPdaIndex: pdaIndex,
        childSubaccountIndex: subaccountIndex,
      },
      client
    );
    instructions.push(transferToParentIx);
    named.transferCollateralChildToParent = transferToParentIx;
  }

  return {
    instructions,
    named,
    subaccountIndex,
  };
};

/**
 * Build a scale (multi-limit) order: a laddered set of PostOnly limit orders
 * placed via `place_multi_limit_order`. Works for both cross (subaccount 0) and
 * isolated margin.
 *
 * **Cross** targets subaccount 0 directly and may span several transaction
 * batches (a full 64-order side does not fit one transaction); the caller
 * submits each batch in order.
 *
 * **Isolated** mirrors the backend `place-isolated-*` endpoints: it allocates
 * (and, when missing, registers) the child subaccount, syncs parent → child,
 * funds the child, places the ladder, then sweeps the child back to the parent.
 * The register/sync/fund setup lands in the first batch and the sweep in the
 * last, so a single-batch ladder is fully atomic. Larger isolated ladders may
 * span several batches (like cross) up to the {@link MAX_SCALE_ORDERS} per-side
 * cap; batches are NOT atomic across transactions, so a mid-batch failure can
 * leave a partial ladder and collateral funded-but-not-yet-swept. Callers that
 * need atomicity should keep isolated ladders to a single batch (size the order
 * count to `maxOrdersPerTx`).
 */
export const buildPlaceMultiLimitOrderFlow = async (
  params: PlaceMultiLimitOrderFlowParams,
  client: PhoenixInstructionClient &
    PhoenixMarketDataClient &
    PhoenixAccountExistenceClient
): Promise<PlaceMultiLimitOrderFlowResult> => {
  const {
    authority,
    positionAuthority,
    symbol: marketSymbol,
    side,
    levels,
    marginType,
    transferAmount,
    subaccountIndex: explicitSubaccountIndex,
    pdaIndex = 0,
    slide = false,
    clientOrderId = null,
    maxOrdersPerTx,
    skipTransferToParent = false,
    scaleSetId,
    reduceOnly = false,
  } = params;

  if (
    scaleSetId !== undefined &&
    (!Number.isInteger(scaleSetId) || scaleSetId < 0 || scaleSetId > 255)
  ) {
    throw new Error(
      `scaleSetId must be an integer in 0..=255; got ${scaleSetId}`
    );
  }

  const hasScaleSetId = (scaleSetId ?? 0) !== 0;
  // Must match the Rust SDK's `uses_v2_instruction()` dispatch rule.
  const usesV2Instruction = hasScaleSetId || reduceOnly;

  const placeableLevels = levels.filter((level) => level.sizeBaseLots > 0);
  if (placeableLevels.length === 0) {
    throw new Error("Scale order has no levels with positive size");
  }
  if (placeableLevels.length > MAX_SCALE_ORDERS) {
    throw new Error(
      `Scale order side may have at most ${MAX_SCALE_ORDERS} orders; got ${placeableLevels.length}`
    );
  }

  const { perpAssetMapKey, arenaAddresses, globalTraderIndexAddresses } =
    await fetchRequiredAccounts(client);

  const isIsolated = marginType === MarginType.Isolated;

  let subaccountIndex: number;
  let subaccountAddress: TraderAddress;
  let marketAccount: MarketAddress;
  let needsRegistration = false;

  if (isIsolated && explicitSubaccountIndex === undefined) {
    if (transferAmount === undefined || transferAmount <= 0n) {
      throw new Error(
        "transfer_amount is required (and must be > 0) for isolated scale orders when subaccount_index is not specified. " +
          "It funds the isolated child subaccount."
      );
    }
    const metadata = await resolveMarketMetadata(
      marketSymbol,
      perpAssetMapKey,
      client
    );
    marketAccount = metadata.marketAccount;
    const allocation = await allocateIsolatedSubaccount(
      client,
      authority,
      pdaIndex,
      metadata.assetId
    );
    subaccountIndex = allocation.index;
    subaccountAddress = allocation.address;
    needsRegistration = allocation.needsRegistration;
  } else {
    const resolved = await resolveSubaccount(
      client,
      perpAssetMapKey,
      authority,
      marketSymbol,
      marginType,
      explicitSubaccountIndex,
      transferAmount,
      pdaIndex
    );
    subaccountIndex = resolved.subaccountIndex;
    subaccountAddress = resolved.subaccountAddress;
    marketAccount = resolved.marketAccount;
  }

  const splineCollection = await getPhoenixSplineCollectionAddress(
    marketAccount,
    client.addresses.phoenixProgramAddress
  );

  const registerIx =
    isIsolated && needsRegistration
      ? await buildRegisterTrader(
          {
            authority,
            marginType: MarginType.Isolated,
            traderPdaIndex: pdaIndex,
            traderSubaccountIndex: subaccountIndex,
            // Rent payer + register signer. Resolves to the sponsor fee payer
            // when provided so sponsored sessions (where `authority` never
            // signs) don't add an unsatisfiable third required signer; the
            // token is a type-level discriminator only, unused by the builder.
            feePayer: resolveFlowPayer(params),
            sponsorshipToken: "",
          },
          client
        )
      : undefined;

  const syncIx = isIsolated
    ? await buildSyncParentToChild(
        {
          traderWallet: authority,
          traderPdaIndex: pdaIndex,
          traderSubaccountIndex: subaccountIndex,
        },
        client
      )
    : undefined;

  const transferIx =
    transferAmount !== undefined && transferAmount > 0n
      ? await buildTransferCollateral(
          {
            authority,
            positionAuthority,
            srcSubaccountIndex: 0,
            dstSubaccountIndex: subaccountIndex,
            traderPdaIndex: pdaIndex,
            amount: transferAmount,
          },
          client
        )
      : undefined;

  const sweepIx =
    !skipTransferToParent && subaccountIndex > 0
      ? await buildTransferCollateralChildToParent(
          {
            authority,
            positionAuthority,
            traderPdaIndex: pdaIndex,
            childSubaccountIndex: subaccountIndex,
          },
          client
        )
      : undefined;

  const chunks = chunkScaleLevelsForTx(placeableLevels, {
    maxOrdersPerTx,
    usesV2Instruction,
  });

  // A scale_set_id cannot span transactions: the on-chain duplicate-id check
  // (`reject_duplicate_scale_set_id`) would reject every batch after the first.
  if (hasScaleSetId && chunks.length > 1) {
    throw new Error(
      `A scaleSetId cannot span transactions; got ${chunks.length} chunks for ${placeableLevels.length} orders. Raise maxOrdersPerTx (capped at ${MAX_SCALE_ORDERS}) or reduce the order count so the ladder fits one transaction.`
    );
  }

  const batches: PlaceMultiLimitOrderFlowBatch[] = [];

  // Effective signer of the placement instructions. Multi-limit orders are
  // not Flight-routable today, so the wrap below is a passthrough; the
  // signer is still named for uniformity with the other flows.
  const signer = positionAuthority ?? authority;

  const commonPlaceIxParams = {
    ...clientPhoenixInstructionAddresses(client),
    trader: signer,
    traderAccount: subaccountAddress,
    perpAssetMap: perpAssetMapKey,
    orderbook: marketAccount,
    splineCollection,
    activeTraderBuffer: arenaAddresses,
    globalTraderIndex: globalTraderIndexAddresses,
  };

  for (let i = 0; i < chunks.length; i++) {
    const placeIx = usesV2Instruction
      ? buildPlaceMultiLimitOrderV2Ix({
          ...commonPlaceIxParams,
          multipleOrderPacket: scaleLevelsToMultipleOrderPacketV2(
            chunks[i],
            side,
            { slide, reduceOnly, clientOrderId, scaleSetId }
          ),
        })
      : buildPlaceMultiLimitOrderIx({
          ...commonPlaceIxParams,
          multipleOrderPacket: scaleLevelsToMultipleOrderPacket(
            chunks[i],
            side,
            { slide, clientOrderId }
          ),
        });

    const placeMultiLimitOrder = isFlightClient(client)
      ? await client.tryWrapOrderInstruction(
          placeIx,
          signer,
          signer !== authority
        )
      : placeIx;

    const instructions: InstructionsWithAccountsAndData[] = [];
    const named: PlaceMultiLimitOrderFlowBatchInstructions = {
      placeMultiLimitOrder,
    };

    if (i === 0) {
      // Isolated setup runs once, before the first placement: register (if
      // new) → sync parent→child → fund the child.
      if (registerIx !== undefined) {
        instructions.push(registerIx);
        named.registerTrader = registerIx;
      }
      if (syncIx !== undefined) {
        instructions.push(syncIx);
        named.syncParentToChild = syncIx;
      }
      if (transferIx !== undefined) {
        instructions.push(transferIx);
        named.transferCollateral = transferIx;
      }
    }
    instructions.push(placeMultiLimitOrder);
    if (i === chunks.length - 1 && sweepIx !== undefined) {
      instructions.push(sweepIx);
      named.transferCollateralChildToParent = sweepIx;
    }

    batches.push({ instructions, named });
  }

  return { batches, subaccountIndex };
};

export const buildGrantEscrowPermissionFlow = async (
  params: GrantEscrowPermissionFlowParams,
  client: PhoenixInstructionClient & PhoenixAccountExistenceClient
): Promise<GrantEscrowPermissionFlowResult> => {
  const permissionPda = await getPhoenixPermissionAddress(
    params.permissionAuthority,
    params.delegatedKey,
    client.addresses.phoenixProgramAddress
  );

  const instructions: InstructionsWithAccountsAndData[] = [];
  let createPermission: InstructionsWithAccountsAndData | undefined;
  let currentPermission = 0n;

  if (!(await client.accountExists(permissionPda))) {
    createPermission = buildCreatePermissionIx({
      ...clientPhoenixInstructionAddresses(client),
      payer: params.payer ?? params.permissionAuthority,
      permissionAuthority: params.permissionAuthority,
      delegatedKey: params.delegatedKey,
      permissionPda,
    });
    instructions.push(createPermission);
  } else {
    const current = await fetchPermission({ client, address: permissionPda });
    currentPermission = current.permission;
  }

  const setPermission = buildSetPermissionIx({
    ...clientPhoenixInstructionAddresses(client),
    permissionAuthority: params.permissionAuthority,
    delegatedKey: params.delegatedKey,
    permissionPda,
    permission: currentPermission | ALLOW_ESCROW_REQUEST_PERMISSION,
    expiresAtTimestamp: params.expiresAtTimestamp ?? null,
    allowedSignerActions: params.allowedSignerActions ?? null,
  });
  instructions.push(setPermission);

  return {
    instructions,
    named: { createPermission, setPermission },
    permissionPda,
  };
};

export const buildRevokeEscrowPermissionFlow = async (
  params: RevokeEscrowPermissionFlowParams,
  client: PhoenixInstructionClient
): Promise<RevokeEscrowPermissionFlowResult> => {
  const permissionPda = await getPhoenixPermissionAddress(
    params.permissionAuthority,
    params.delegatedKey,
    client.addresses.phoenixProgramAddress
  );
  const current = await fetchPermission({ client, address: permissionPda });

  const setPermission = buildSetPermissionIx({
    ...clientPhoenixInstructionAddresses(client),
    permissionAuthority: params.permissionAuthority,
    delegatedKey: params.delegatedKey,
    permissionPda,
    permission: current.permission & ~ALLOW_ESCROW_REQUEST_PERMISSION,
    expiresAtTimestamp:
      current.expiresAtTimestamp < 0n ? null : current.expiresAtTimestamp,
    allowedSignerActions:
      current.allowedSignerActions < 0n ? null : current.allowedSignerActions,
  });

  return {
    instructions: [setPermission],
    named: { setPermission },
    permissionPda,
  };
};
