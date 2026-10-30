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
import type { SpotCollateralMetadata } from "../SpotCollateralMetadata";

/** Native SOL counts as margin collateral (together with its metadata's
 * `isActive`). */
export const GLOBAL_FEATURE_NATIVE_SOL_COLLATERALS: number = 1 << 1;

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
  /** Exchange feature bits; see the `GLOBAL_FEATURE_*` constants. */
  featureSet: number;
  depositCooldownPeriodInSlots: bigint;
  pendingAuthorities: AuthoritySet;
  /**
   * Spot collateral configuration for native SOL, decoded from the actual
   * account bytes unconditionally — an exchange that never configured spot
   * collateral yields the all-zero metadata with `isActive: false`, not an
   * absent field. Nothing may construct this type synthetically; sources that
   * cannot read the account (e.g. the exchange metadata snapshot) expose only
   * addresses instead (`RequiredAccounts`).
   */
  nativeSolSpotMetadata: SpotCollateralMetadata;
  acknowledgedRestartSlot: bigint;
  pendingWithdrawThrottleReplenishment: QuoteLots;
}
