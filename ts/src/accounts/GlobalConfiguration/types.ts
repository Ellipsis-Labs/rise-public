import type {
  ActiveTraderBufferHeaderAddress,
  GlobalConfigurationAddress,
  GlobalTraderIndexHeaderAddress,
  GlobalVaultAddress,
  MintAddress,
  PerpAssetMapAddress,
  WithdrawQueueAddress,
} from "@/primitives/_addressTypes";
import type { QuoteLots } from "@/primitives/_numberTypes";
import type { AuthoritySet } from "../internal";

export interface GlobalConfiguration {
  accountKey: GlobalConfigurationAddress;
  currentAuthorities: AuthoritySet;
  canonicalTokenMintKey: MintAddress;
  globalVaultKey: GlobalVaultAddress;
  perpAssetMapKey: PerpAssetMapAddress;
  globalTraderIndexHeaderKey: GlobalTraderIndexHeaderAddress;
  activeTraderBufferHeaderKey: ActiveTraderBufferHeaderAddress;
  totalQuoteLotFees: QuoteLots;
  unclaimedQuoteLotFees: QuoteLots;
  withdrawQueueKey: WithdrawQueueAddress;
  exchangeStatus: number;
  quoteDecimals: number;
  withdrawalMarginFactorBps: number;
  depositCooldownPeriodInSlots: bigint;
  pendingAuthorities: AuthoritySet;
}
