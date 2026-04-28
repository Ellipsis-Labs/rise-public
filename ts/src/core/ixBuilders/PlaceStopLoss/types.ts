import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type {
  ActiveTraderBufferAddressArray,
  Authority,
  Direction,
  GlobalTraderIndexAddressArray,
  MarketAddress,
  PerpAssetMapAddress,
  Side,
  SplineCollectionAddress,
  StopLossOrderKind,
  TraderAddress,
} from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta, Address } from "@solana/kit";

export interface PlaceStopLossParams extends PhoenixInstructionAddressOverrides {
  funder: Authority;
  traderAccount: TraderAddress;
  perpAssetMap: PerpAssetMapAddress;
  orderbook: MarketAddress;
  splineCollection: SplineCollectionAddress;
  globalTraderIndex: GlobalTraderIndexAddressArray;
  activeTraderBuffer: ActiveTraderBufferAddressArray;
  positionAuthority: Authority;
  stopLossAccount: Address;
  triggerPrice: bigint;
  executionPrice: bigint;
  tradeSide: Side;
  executionDirection: Direction;
  orderKind: StopLossOrderKind;
}

export type PlaceStopLossIx = InstructionsWithAccountsAndData;

export type PlaceStopLossAccounts = readonly AccountMeta[];
