import type { Authority, TraderAddress } from "@/primitives/_addressTypes";
import type {
  SequenceNumber,
  ShortEntries,
  TraderPosition,
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
  positionAuthority: Authority;
  numMarketsWithSplines: number;
  traderPdaIndex: number;
  traderSubaccountIndex: number;
  fundingKey: Authority;
  lastDepositSlot: bigint;
  conditionalOrderBits: number[];
  occupiedConditionalOrderIndices: number[];
  positions: ShortEntries<bigint, TraderPosition>;
}
