import type { ConditionalOrderCollection } from "./ConditionalOrderCollection";
import { fetchConditionalOrderCollection } from "./ConditionalOrderCollection";
import type {
  MarketAddress,
  MintAddress,
  PerpAssetMapAddress,
  SplineCollectionAddress,
  TokenAccountAddress,
  TraderAddress,
  WithdrawQueueAddress,
} from "@/primitives/_addressTypes";
import type { GlobalConfiguration } from "./GlobalConfiguration";
import { fetchGlobalConfiguration } from "./GlobalConfiguration";
import type { Mint } from "./Mint";
import { fetchMint } from "./Mint";
import type { Permission } from "./Permission";
import { fetchPermission } from "./Permission";
import type { OrderbookHeader } from "./OrderbookHeader";
import { fetchOrderbookHeader } from "./OrderbookHeader";
import type { PerpAssetMap } from "./PerpAssetMap";
import { fetchPerpAssetMap } from "./PerpAssetMap";
import type {
  AccountFetcherFor,
  AccountFetcherWithDefaultFor,
  AccountFetcherClientWithAddresses,
} from "./fetcherFactory";
import type { SplineCollection } from "./SplineCollection";
import { fetchSplineCollection } from "./SplineCollection";
import type { StopLosses } from "./StopLosses";
import { fetchStopLosses } from "./StopLosses";
import type { TokenAccount } from "./TokenAccount";
import { fetchTokenAccount } from "./TokenAccount";
import type { Trader } from "./Trader";
import { fetchTrader } from "./Trader";
import type { WithdrawQueueHeader } from "./WithdrawQueueHeader";
import { fetchWithdrawQueueHeader } from "./WithdrawQueueHeader";
import type { Address } from "@solana/kit";

type AccountFetchersType = {
  ConditionalOrderCollection: AccountFetcherFor<
    ConditionalOrderCollection,
    Address
  >;
  GlobalConfiguration: AccountFetcherWithDefaultFor<
    GlobalConfiguration,
    AccountFetcherClientWithAddresses
  >;
  OrderbookHeader: AccountFetcherFor<OrderbookHeader, MarketAddress>;
  PerpAssetMap: AccountFetcherFor<PerpAssetMap, PerpAssetMapAddress>;
  Permission: AccountFetcherFor<Permission, Address>;
  SplineCollection: AccountFetcherFor<
    SplineCollection,
    SplineCollectionAddress
  >;
  StopLosses: AccountFetcherFor<StopLosses, Address>;
  Trader: AccountFetcherFor<Trader, TraderAddress>;
  Mint: AccountFetcherFor<Mint, MintAddress>;
  TokenAccount: AccountFetcherFor<TokenAccount, TokenAccountAddress>;
  WithdrawQueueHeader: AccountFetcherFor<
    WithdrawQueueHeader,
    WithdrawQueueAddress
  >;
};

export const AccountFetchers: AccountFetchersType = {
  ConditionalOrderCollection: fetchConditionalOrderCollection,
  GlobalConfiguration: fetchGlobalConfiguration,
  OrderbookHeader: fetchOrderbookHeader,
  PerpAssetMap: fetchPerpAssetMap,
  Permission: fetchPermission,
  SplineCollection: fetchSplineCollection,
  StopLosses: fetchStopLosses,
  Trader: fetchTrader,
  Mint: fetchMint,
  TokenAccount: fetchTokenAccount,
  WithdrawQueueHeader: fetchWithdrawQueueHeader,
} as const;
