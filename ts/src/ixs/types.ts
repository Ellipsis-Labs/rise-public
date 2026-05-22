import type { EscrowAction } from "@/core/ixBuilders/CreateEscrowRequest";
import type { CancelAllIx } from "@/core/ixBuilders/CancelAll";
import type { CancelOrdersByIdIx } from "@/core/ixBuilders/CancelOrdersById";
import type { CancelStopLossIx } from "@/core/ixBuilders/CancelStopLoss";
import type { CreateConditionalOrdersAccountIx } from "@/core/ixBuilders/CreateConditionalOrdersAccount";
import type { CreateEscrowRequestIx } from "@/core/ixBuilders/CreateEscrowRequest";
import type { DelegateTraderIx } from "@/core/ixBuilders/DelegateTrader";
import type { DepositFundsIx } from "@/core/ixBuilders/DepositFunds";
import type { EmberDepositIx } from "@/core/ixBuilders/EmberDeposit";
import type { EmberWithdrawIx } from "@/core/ixBuilders/EmberWithdraw";
import type { PlaceAttachedConditionalOrderIx } from "@/core/ixBuilders/PlaceAttachedConditionalOrder";
import type { PlaceLimitOrderIx } from "@/core/ixBuilders/PlaceLimitOrder";
import type { PlaceLimitOrderWithConditionalsIx } from "@/core/ixBuilders/PlaceLimitOrderWithConditionals";
import type { PlacePositionConditionalOrderIx } from "@/core/ixBuilders/PlacePositionConditionalOrder";
import type { PlaceMarketOrderIx } from "@/core/ixBuilders/PlaceMarketOrder";
import type { PlacePostOnlyOrderIx } from "@/core/ixBuilders/PlacePostOnlyOrder";
import type { PlaceStopLossIx } from "@/core/ixBuilders/PlaceStopLoss";
import type { RegisterTraderIx } from "@/core/ixBuilders/RegisterTrader";
import type { SyncParentToChildIx } from "@/core/ixBuilders/SyncParentToChild";
import type { TransferCollateralChildToParentIx } from "@/core/ixBuilders/TransferCollateralChildToParent";
import type { TransferCollateralIx } from "@/core/ixBuilders/TransferCollateral";
import type { WithdrawFundsIx } from "@/core/ixBuilders/WithdrawFunds";
import type { PhoenixOrderPacketBuilders } from "@/orderPackets";
import type {
  ActiveTraderBufferAddressArray,
  Authority,
  BaseLots,
  CancelId,
  ConditionalOrderPacket,
  Direction,
  EmberStateAddress,
  EmberVaultAddress,
  FIFOOrderId,
  GlobalConfigurationAddress,
  GlobalVaultAddress,
  GlobalTraderIndexAddressArray,
  LogAuthorityAddress,
  MarginType,
  MarketAddress,
  MintAddress,
  PerpAssetMapAddress,
  PhoenixProgramAddress,
  PostOnlyOrderPacket,
  Side,
  SplineCollectionAddress,
  StopLossOrderKind,
  Symbol,
  TokenAccountAddress,
  TraderAddress,
  WithdrawQueueAddress,
} from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type {
  ImmediateOrCancelOrderPacket,
  LimitOrderPacket,
} from "@/primitives/OrderPacket";
import type { Address } from "@solana/kit";
import type { TriggerOrderParamsInput } from "../conditionalOrders";

export interface ResolvedPlaceOrderContext {
  exchange: {
    phoenixProgramAddress: PhoenixProgramAddress;
    logAuthorityAddress: LogAuthorityAddress;
    globalConfigurationAddress: GlobalConfigurationAddress;
    perpAssetMap: PerpAssetMapAddress;
    globalTraderIndex: GlobalTraderIndexAddressArray;
    activeTraderBuffer: ActiveTraderBufferAddressArray;
  };
  market: {
    marketAddress: MarketAddress;
    splineCollection: SplineCollectionAddress;
  };
  trader: {
    authority: Authority;
    funder?: Authority;
    positionAuthority?: Authority;
    traderAccount: TraderAddress;
  };
}

export interface BuildPlaceLimitOrderIxResolvedInput extends ResolvedPlaceOrderContext {
  orderPacket: LimitOrderPacket;
}

export interface BuildPlaceMarketOrderIxResolvedInput extends ResolvedPlaceOrderContext {
  orderPacket: ImmediateOrCancelOrderPacket;
}

export interface BuildPlacePostOnlyOrderIxResolvedInput extends ResolvedPlaceOrderContext {
  orderPacket: PostOnlyOrderPacket;
}

export type BuildCancelAllIxResolvedInput = ResolvedPlaceOrderContext;

export interface BuildCancelOrdersByIdIxResolvedInput extends ResolvedPlaceOrderContext {
  orderIds: CancelId[];
}

export interface BuildPlaceStopLossIxResolvedInput extends ResolvedPlaceOrderContext {
  stopLossAccount: Address;
  triggerPrice: bigint;
  executionPrice: bigint;
  tradeSide: Side;
  executionDirection: Direction;
  orderKind: StopLossOrderKind;
}

export interface ClientPlaceOrderInput<TPacket> {
  authority: Authority;
  positionAuthority?: Authority;
  symbol: Symbol;
  orderPacket: TPacket;
  traderPdaIndex?: number;
  traderSubaccountIndex?: number;
}

export interface ClientCancelAllInput {
  authority: Authority;
  positionAuthority?: Authority;
  symbol: Symbol;
  traderPdaIndex?: number;
  traderSubaccountIndex?: number;
}

export interface ClientCancelOrdersByIdInput extends ClientCancelAllInput {
  orders: Array<{
    price: number | bigint;
    orderSequenceNumber: string | number;
  }>;
}

export interface ClientCancelStopLossInput {
  authority: Authority;
  funder?: Authority;
  symbol: Symbol;
  executionDirection: Direction;
  traderPdaIndex?: number;
  traderSubaccountIndex?: number;
}

export interface ClientCreateEscrowRequestInput {
  senderAuthority: Authority;
  receiverAuthority: Authority;
  actions: EscrowAction[];
  senderPdaIndex?: number;
  senderSubaccountIndex?: number;
  receiverPdaIndex?: number;
  receiverSubaccountIndex?: number;
  lastValidSlot?: bigint | null;
}

export interface ClientDelegateTraderInput {
  traderWallet: Authority;
  traderPdaIndex: number;
  traderSubaccountIndex: number;
  newPositionAuthority: Address;
}

export interface ClientCreateConditionalOrdersAccountInput {
  authority: Authority;
  positionAuthority?: Authority;
  payer?: Authority;
  capacity?: number;
  traderPdaIndex?: number;
  traderSubaccountIndex?: number;
}

export interface ClientDepositInput {
  authority: Authority;
  amount: bigint;
  feePayer?: Authority;
  permissionAddress?: Address;
  traderPdaIndex?: number;
  traderSubaccountIndex?: number;
}

export interface ClientWithdrawInput {
  authority: Authority;
  amount: bigint;
  feePayer?: Authority;
  traderPdaIndex?: number;
  traderSubaccountIndex?: number;
}

export interface ClientTransferCollateralInput {
  authority: Authority;
  positionAuthority?: Authority;
  traderPdaIndex: number;
  srcSubaccountIndex: number;
  dstSubaccountIndex: number;
  amount: bigint;
}

export interface ClientTransferCollateralChildToParentInput {
  authority: Authority;
  positionAuthority?: Authority;
  traderPdaIndex: number;
  childSubaccountIndex: number;
}

export interface ClientPlaceStopLossInput {
  authority: Authority;
  positionAuthority?: Authority;
  payer?: Authority;
  symbol: Symbol;
  triggerPrice: bigint;
  executionPrice?: bigint;
  slippageBps?: number | null;
  tradeSide: Side;
  executionDirection: Direction;
  orderKind: StopLossOrderKind;
  traderPdaIndex?: number;
  traderSubaccountIndex?: number;
}

export interface ClientPlacePositionConditionalOrderInput {
  authority: Authority;
  positionAuthority?: Authority;
  payer?: Authority;
  symbol: Symbol;
  greaterTriggerOrder?: TriggerOrderParamsInput | null;
  lessTriggerOrder?: TriggerOrderParamsInput | null;
  sizeBaseLots?: BaseLots | null;
  sizePercent?: number | null;
  traderPdaIndex?: number;
  traderSubaccountIndex?: number;
}

export interface ClientPlaceAttachedConditionalOrderInput {
  authority: Authority;
  positionAuthority?: Authority;
  payer?: Authority;
  symbol: Symbol;
  orderId: FIFOOrderId;
  greaterTriggerOrder?: TriggerOrderParamsInput | null;
  lessTriggerOrder?: TriggerOrderParamsInput | null;
  traderPdaIndex?: number;
  traderSubaccountIndex?: number;
}

export interface ClientPlaceLimitOrderWithConditionalsInput {
  authority: Authority;
  positionAuthority?: Authority;
  payer?: Authority;
  symbol: Symbol;
  orderPacket: ConditionalOrderPacket;
  slot?: bigint;
  greaterTriggerOrder?: TriggerOrderParamsInput | null;
  lessTriggerOrder?: TriggerOrderParamsInput | null;
  traderPdaIndex?: number;
  traderSubaccountIndex?: number;
}

interface BaseBuildRegisterTraderParams {
  authority: Authority;
  marginType: MarginType;
  traderPdaIndex?: number;
  traderSubaccountIndex?: number;
}

type SponsoredBuildRegisterTraderParams = BaseBuildRegisterTraderParams & {
  userId?: string;
  userPubkey?: Authority;
  feePayer: Authority;
  sponsorshipToken: string;
};

interface NonSponsoredBuildRegisterTraderParams extends BaseBuildRegisterTraderParams {
  feePayer?: null;
}

export type BuildRegisterTraderParams =
  | SponsoredBuildRegisterTraderParams
  | NonSponsoredBuildRegisterTraderParams;

export interface ClientSyncParentToChildInput {
  traderWallet: Authority;
  traderPdaIndex: number;
  traderSubaccountIndex: number;
}

export interface ResolvedCancelStopLossIxInput {
  exchange: {
    phoenixProgramAddress: PhoenixProgramAddress;
    logAuthorityAddress: LogAuthorityAddress;
    globalConfigurationAddress: GlobalConfigurationAddress;
  };
  trader: {
    authority: Authority;
    funder?: Authority;
    traderAccount: TraderAddress;
    stopLossAccount: Address;
  };
  executionDirection: Direction;
}

export interface ResolvedCreateEscrowRequestIxInput {
  exchange: {
    phoenixProgramAddress: PhoenixProgramAddress;
    logAuthorityAddress: LogAuthorityAddress;
    globalConfigurationAddress: GlobalConfigurationAddress;
    perpAssetMap: PerpAssetMapAddress;
    globalTraderIndex: GlobalTraderIndexAddressArray;
    activeTraderBuffer: ActiveTraderBufferAddressArray;
  };
  sender: {
    authority: Authority;
    traderAccount: TraderAddress;
    traderPdaIndex: number;
    traderSubaccountIndex: number;
  };
  receiver: {
    authority: Authority;
    traderAccount: TraderAddress;
    escrowAddress: Address;
    permissionAddress: Address;
    traderPdaIndex: number;
    traderSubaccountIndex: number;
  };
  actions: EscrowAction[];
  lastValidSlot?: bigint | null;
}

export interface ResolvedDelegateTraderIxInput {
  exchange: {
    phoenixProgramAddress: PhoenixProgramAddress;
    logAuthorityAddress?: LogAuthorityAddress;
    globalConfigurationAddress?: GlobalConfigurationAddress;
  };
  trader: {
    authority: Authority;
    traderAccount: TraderAddress;
  };
  newPositionAuthority: Address;
}

export interface ResolvedRegisterTraderIxInput {
  exchange: {
    phoenixProgramAddress: PhoenixProgramAddress;
    logAuthorityAddress: LogAuthorityAddress;
    globalConfigurationAddress: GlobalConfigurationAddress;
  };
  trader: {
    payer: Authority;
    authority: Authority;
    traderAccount: TraderAddress;
  };
  maxPositions: bigint;
  traderPdaIndex: number;
  traderSubaccountIndex: number;
}

export interface ResolvedSyncParentToChildIxInput {
  exchange: {
    phoenixProgramAddress: PhoenixProgramAddress;
    logAuthorityAddress: LogAuthorityAddress;
    globalConfigurationAddress: GlobalConfigurationAddress;
    globalTraderIndex: GlobalTraderIndexAddressArray;
  };
  trader: {
    authority: Authority;
    parentTraderAccount: TraderAddress;
    childTraderAccount: TraderAddress;
  };
}

export interface ResolvedDepositFundsIxInput {
  exchange: {
    phoenixProgramAddress: PhoenixProgramAddress;
    logAuthorityAddress: LogAuthorityAddress;
    globalConfigurationAddress: GlobalConfigurationAddress;
    canonicalMint: MintAddress;
    globalVault: GlobalVaultAddress;
    globalTraderIndex: GlobalTraderIndexAddressArray;
    activeTraderBuffer: ActiveTraderBufferAddressArray;
  };
  trader: {
    authority: Authority;
    traderAccount: TraderAddress;
    traderTokenAccount: TokenAccountAddress;
    permissionAddress?: Address;
  };
  amount: bigint;
}

export interface ResolvedWithdrawFundsIxInput {
  exchange: {
    phoenixProgramAddress: PhoenixProgramAddress;
    logAuthorityAddress: LogAuthorityAddress;
    globalConfigurationAddress: GlobalConfigurationAddress;
    canonicalMint: MintAddress;
    perpAssetMap: PerpAssetMapAddress;
    globalVault: GlobalVaultAddress;
    withdrawQueue: WithdrawQueueAddress;
    globalTraderIndex: GlobalTraderIndexAddressArray;
    activeTraderBuffer: ActiveTraderBufferAddressArray;
  };
  trader: {
    authority: Authority;
    traderAccount: TraderAddress;
    destinationTokenAccount: TokenAccountAddress;
  };
  amount: bigint;
}

export interface ResolvedTransferCollateralIxInput {
  exchange: {
    phoenixProgramAddress: PhoenixProgramAddress;
    logAuthorityAddress: LogAuthorityAddress;
    globalConfigurationAddress: GlobalConfigurationAddress;
    perpAssetMap: PerpAssetMapAddress;
    globalTraderIndex: GlobalTraderIndexAddressArray;
    activeTraderBuffer: ActiveTraderBufferAddressArray;
  };
  trader: {
    authority: Authority;
    positionAuthority?: Authority;
    srcTraderAccount: TraderAddress;
    dstTraderAccount: TraderAddress;
  };
  amount: bigint;
}

export interface ResolvedTransferCollateralChildToParentIxInput {
  exchange: {
    phoenixProgramAddress: PhoenixProgramAddress;
    logAuthorityAddress: LogAuthorityAddress;
    globalConfigurationAddress: GlobalConfigurationAddress;
    perpAssetMap: PerpAssetMapAddress;
    globalTraderIndex: GlobalTraderIndexAddressArray;
    activeTraderBuffer: ActiveTraderBufferAddressArray;
  };
  trader: {
    authority: Authority;
    positionAuthority?: Authority;
    childTraderAccount: TraderAddress;
    parentTraderAccount: TraderAddress;
  };
}

export interface ResolvedEmberDepositIxInput {
  exchange: {
    usdcMint: MintAddress;
    canonicalMint: MintAddress;
    emberState: EmberStateAddress;
    emberVault: EmberVaultAddress;
  };
  trader: {
    authority: Authority;
    usdcTokenAccount: TokenAccountAddress;
    phoenixTokenAccount: TokenAccountAddress;
  };
  amount: bigint;
}

export interface ResolvedEmberWithdrawIxInput {
  exchange: {
    usdcMint: MintAddress;
    canonicalMint: MintAddress;
    emberState: EmberStateAddress;
    emberVault: EmberVaultAddress;
  };
  trader: {
    authority: Authority;
    usdcTokenAccount: TokenAccountAddress;
    phoenixTokenAccount: TokenAccountAddress;
  };
  amount: bigint | null;
}

export interface BuildDepositIxsResolvedInput {
  feePayer?: Authority;
  exchange: {
    phoenixProgramAddress: PhoenixProgramAddress;
    logAuthorityAddress: LogAuthorityAddress;
    globalConfigurationAddress: GlobalConfigurationAddress;
    canonicalMint: MintAddress;
    usdcMint: MintAddress;
    globalVault: GlobalVaultAddress;
    globalTraderIndex: GlobalTraderIndexAddressArray;
    activeTraderBuffer: ActiveTraderBufferAddressArray;
    emberState: EmberStateAddress;
    emberVault: EmberVaultAddress;
  };
  trader: {
    authority: Authority;
    traderAccount: TraderAddress;
    usdcTokenAccount: TokenAccountAddress;
    phoenixTokenAccount: TokenAccountAddress;
  };
  amount: bigint;
}

export interface BuildWithdrawIxsResolvedInput {
  feePayer?: Authority;
  exchange: {
    phoenixProgramAddress: PhoenixProgramAddress;
    logAuthorityAddress: LogAuthorityAddress;
    globalConfigurationAddress: GlobalConfigurationAddress;
    canonicalMint: MintAddress;
    usdcMint: MintAddress;
    perpAssetMap: PerpAssetMapAddress;
    globalVault: GlobalVaultAddress;
    withdrawQueue: WithdrawQueueAddress;
    globalTraderIndex: GlobalTraderIndexAddressArray;
    activeTraderBuffer: ActiveTraderBufferAddressArray;
    emberState: EmberStateAddress;
    emberVault: EmberVaultAddress;
  };
  trader: {
    authority: Authority;
    traderAccount: TraderAddress;
    usdcTokenAccount: TokenAccountAddress;
    phoenixTokenAccount: TokenAccountAddress;
  };
  amount: bigint;
}

export interface DepositIxsResult {
  instructions: InstructionsWithAccountsAndData[];
  named: {
    createPhoenixAta: InstructionsWithAccountsAndData;
    emberDeposit: EmberDepositIx;
    depositFunds: DepositFundsIx;
  };
}

export interface WithdrawIxsResult {
  instructions: InstructionsWithAccountsAndData[];
  named: {
    createPhoenixAta: InstructionsWithAccountsAndData;
    approveToken: InstructionsWithAccountsAndData;
    createUsdcAta: InstructionsWithAccountsAndData;
    withdrawFunds: WithdrawFundsIx;
    emberWithdraw: EmberWithdrawIx;
  };
}

export interface PhoenixIxClient {
  orderPackets: PhoenixOrderPacketBuilders;
  buildCancelAll(params: ClientCancelAllInput): Promise<CancelAllIx>;
  buildCancelOrdersById(
    params: ClientCancelOrdersByIdInput
  ): Promise<CancelOrdersByIdIx>;
  buildCancelStopLoss(
    params: ClientCancelStopLossInput
  ): Promise<CancelStopLossIx>;
  buildCreateConditionalOrdersAccount(
    params: ClientCreateConditionalOrdersAccountInput
  ): Promise<CreateConditionalOrdersAccountIx>;
  buildCreateEscrowRequest(
    params: ClientCreateEscrowRequestInput
  ): Promise<CreateEscrowRequestIx>;
  buildDelegateTrader(
    params: ClientDelegateTraderInput
  ): Promise<DelegateTraderIx>;
  buildPlaceLimitOrder(
    params: ClientPlaceOrderInput<LimitOrderPacket>
  ): Promise<PlaceLimitOrderIx>;
  buildPlaceMarketOrder(
    params: ClientPlaceOrderInput<ImmediateOrCancelOrderPacket>
  ): Promise<PlaceMarketOrderIx>;
  buildPlacePositionConditionalOrder(
    params: ClientPlacePositionConditionalOrderInput
  ): Promise<PlacePositionConditionalOrderIx>;
  buildPlaceAttachedConditionalOrder(
    params: ClientPlaceAttachedConditionalOrderInput
  ): Promise<PlaceAttachedConditionalOrderIx>;
  buildPlaceLimitOrderWithConditionals(
    params: ClientPlaceLimitOrderWithConditionalsInput
  ): Promise<PlaceLimitOrderWithConditionalsIx>;
  buildPlacePostOnlyOrder(
    params: ClientPlaceOrderInput<PostOnlyOrderPacket>
  ): Promise<PlacePostOnlyOrderIx>;
  placeLimitOrder(
    params: ClientPlaceOrderInput<LimitOrderPacket>
  ): Promise<PlaceLimitOrderIx>;
  placeMarketOrder(
    params: ClientPlaceOrderInput<ImmediateOrCancelOrderPacket>
  ): Promise<PlaceMarketOrderIx>;
  placePositionConditionalOrder(
    params: ClientPlacePositionConditionalOrderInput
  ): Promise<PlacePositionConditionalOrderIx>;
  placePostOnlyOrder(
    params: ClientPlaceOrderInput<PostOnlyOrderPacket>
  ): Promise<PlacePostOnlyOrderIx>;
  buildDepositFunds(params: ClientDepositInput): Promise<DepositFundsIx>;
  buildWithdrawFunds(params: ClientWithdrawInput): Promise<WithdrawFundsIx>;
  buildTransferCollateral(
    params: ClientTransferCollateralInput
  ): Promise<TransferCollateralIx>;
  buildTransferCollateralChildToParent(
    params: ClientTransferCollateralChildToParentInput
  ): Promise<TransferCollateralChildToParentIx>;
  buildEmberDeposit(params: ClientDepositInput): Promise<EmberDepositIx>;
  buildEmberWithdraw(params: ClientWithdrawInput): Promise<EmberWithdrawIx>;
  buildPlaceStopLoss(
    params: ClientPlaceStopLossInput
  ): Promise<PlaceStopLossIx>;
  buildRegisterTrader(
    params: BuildRegisterTraderParams
  ): Promise<RegisterTraderIx>;
  buildSyncParentToChild(
    params: ClientSyncParentToChildInput
  ): Promise<SyncParentToChildIx>;
  buildDepositIxs(params: ClientDepositInput): Promise<DepositIxsResult>;
  buildWithdrawIxs(params: ClientWithdrawInput): Promise<WithdrawIxsResult>;
}
