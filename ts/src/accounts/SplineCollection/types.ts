import type { MarketAddress } from "@/primitives/_addressTypes";
import type { Symbol } from "@/primitives/Symbol";
import type { SequenceNumber, Spline } from "../internal";

export interface SplineCollection {
  market: MarketAddress;
  assetSymbol: Symbol;
  sequenceNumber: SequenceNumber;
  numSplines: number;
  numActive: number;
  splines: Spline[];
}
