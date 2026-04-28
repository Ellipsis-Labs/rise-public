import type { ExchangeInstructionContext } from "@/exchange-cache/types";
import type {
  ActiveTraderBufferAddressArray,
  Authority,
  GlobalConfigurationAddress,
  GlobalTraderIndexAddressArray,
  LogAuthorityAddress,
  MarketAddress,
  PerpAssetMapAddress,
  PhoenixProgramAddress,
  SplineCollectionAddress,
  TraderAddress,
} from "@/primitives";
import type { ResolvedPlaceOrderContext } from "./types";

export const buildResolvedPlaceOrderContext = (params: {
  instructionContext: ExchangeInstructionContext;
  phoenixProgramAddress: PhoenixProgramAddress;
  logAuthorityAddress: LogAuthorityAddress;
  authority: Authority;
  positionAuthority?: Authority;
  traderAccount: TraderAddress;
}): ResolvedPlaceOrderContext => ({
  exchange: {
    phoenixProgramAddress: params.phoenixProgramAddress,
    logAuthorityAddress: params.logAuthorityAddress,
    globalConfigurationAddress: params.instructionContext.exchange
      .globalConfig as GlobalConfigurationAddress,
    perpAssetMap: params.instructionContext.exchange
      .perpAssetMap as PerpAssetMapAddress,
    globalTraderIndex: params.instructionContext.exchange
      .globalTraderIndex as GlobalTraderIndexAddressArray,
    activeTraderBuffer: params.instructionContext.exchange
      .activeTraderBuffer as ActiveTraderBufferAddressArray,
  },
  market: {
    marketAddress: params.instructionContext.market
      .marketPubkey as MarketAddress,
    splineCollection: params.instructionContext.market
      .splinePubkey as SplineCollectionAddress,
  },
  trader: {
    authority: params.authority,
    positionAuthority: params.positionAuthority,
    traderAccount: params.traderAccount,
  },
});
