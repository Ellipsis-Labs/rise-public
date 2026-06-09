import {
  fetchPermission,
  fetchPerpAssetMap,
  type GlobalConfiguration,
} from "@/accounts";
import {
  type PhoenixAccountExistenceClient,
  type PhoenixInstructionClient,
  type PhoenixMarketDataClient,
} from "@/core/clientTypes";
import { OrderFlags, SelfTradeBehavior } from "@/primitives/OrderPacket";
import {
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
} from "@/core/permissionInstructions";
import {
  buildDepositFunds,
  buildEmberDeposit,
  buildEmberWithdraw,
  buildTransferCollateral,
  buildTransferCollateralChildToParent,
  buildWithdrawFunds,
} from "@/builders";
import {
  type Authority,
  MarginType,
  type MarketAddress,
  quoteLots,
  Side,
  type Symbol,
  type Ticks,
  ticks,
  type BaseLots,
  type QuoteLots,
  type InstructionsWithAccountsAndData,
  type TokenAccountAddress,
} from "@/primitives";
import {
  getPhoenixPermissionAddress,
  getPhoenixSplineCollectionAddress,
  getPhoenixTraderSubaccountAddress,
  getPhoenixTraderTokenAccountAddress,
} from "@/pdas";
import { deriveFlameDepositAddresses } from "@/flame";
import { address, type Address } from "@solana/kit";
import { buildPlaceLimitOrderIx } from "./core/ixBuilders/PlaceLimitOrder";
import { buildPlaceMarketOrderIx } from "./core/ixBuilders/PlaceMarketOrder";
import { buildPlacePostOnlyOrderIx } from "./core/ixBuilders/PlacePostOnlyOrder";
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
  const { globalConfiguration } = await fetchRequiredAccounts(client);
  const phoenixMint = globalConfiguration.canonicalTokenMintKey;

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

export const buildWithdrawFlow = async (
  params: WithdrawFlowParams,
  client: PhoenixInstructionClient
): Promise<WithdrawFlowResult> => {
  const { authority, amount } = params;
  const { globalConfiguration } = await fetchRequiredAccounts(client);
  const phoenixMint = globalConfiguration.canonicalTokenMintKey;

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
  globalConfiguration: GlobalConfiguration,
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
    address: globalConfiguration.perpAssetMapKey,
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
  globalConfiguration: GlobalConfiguration,
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
    globalConfiguration,
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

  const { globalConfiguration, arenaAddresses, globalTraderIndexAddresses } =
    await fetchRequiredAccounts(client);
  const { subaccountIndex, subaccountAddress, marketAccount } =
    await resolveSubaccount(
      client,
      globalConfiguration,
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

  const placeOrderIx = isPostOnly
    ? buildPlacePostOnlyOrderIx({
        ...clientPhoenixInstructionAddresses(client),
        trader: positionAuthority ?? authority,
        traderAccount: subaccountAddress,
        perpAssetMap: globalConfiguration.perpAssetMapKey,
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
        trader: positionAuthority ?? authority,
        traderAccount: subaccountAddress,
        perpAssetMap: globalConfiguration.perpAssetMapKey,
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
    ? await client.tryWrapFlightInstruction(placeOrderIx, authority)
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

  const { globalConfiguration, arenaAddresses, globalTraderIndexAddresses } =
    await fetchRequiredAccounts(client);
  const { subaccountIndex, subaccountAddress, marketAccount } =
    await resolveSubaccount(
      client,
      globalConfiguration,
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

  const placeOrderIx = buildPlaceMarketOrderIx({
    ...clientPhoenixInstructionAddresses(client),
    trader: positionAuthority ?? authority,
    traderAccount: subaccountAddress,
    perpAssetMap: globalConfiguration.perpAssetMapKey,
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
    ? await client.tryWrapFlightInstruction(placeOrderIx, authority)
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
