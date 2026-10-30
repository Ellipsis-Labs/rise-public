import type { Authority, TraderAddress } from "@/primitives/_addressTypes";
import type {
  SequenceNumber,
  ShortEntries,
  TraderLockedDeposit,
  TraderPosition,
  TraderSpotCollateral,
  TraderState,
} from "../internal";
import type { TraderPreferenceFlags, TraderPreferences } from "./preferences";

export interface Trader {
  sequenceNumber: SequenceNumber;
  key: TraderAddress;
  authority: Authority;
  state: TraderState;
  withdrawQueueNode: number | null;
  maxPositions: number;
  traderPreferenceBits: number;
  traderPreferenceFlags: TraderPreferenceFlags;
  preferences: TraderPreferences;
  disableCollateralSweep: boolean;
  disablePositionAuthoritySwap: boolean;
  positionAuthority: Authority;
  numMarketsWithSplines: number;
  traderPdaIndex: number;
  traderSubaccountIndex: number;
  fundingKey: Authority;
  lastDepositSlot: bigint;
  conditionalOrderBits: number[];
  occupiedConditionalOrderIndices: number[];
  positions: ShortEntries<bigint, TraderPosition>;
  /** @deprecated Use `spotCollaterals` and pick the `NATIVE_SOL_ASSET_INDEX` entry. */
  nativeSolCollateral: bigint;
  /** All spot collateral entries, native SOL included, in account order. */
  spotCollaterals: TraderSpotCollateral[];
  /** The locked deposit, or `null` when the trader has none. */
  lockedDeposit: TraderLockedDeposit | null;
}
