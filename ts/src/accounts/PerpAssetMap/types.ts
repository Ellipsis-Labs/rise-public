import type { Symbol } from "@/primitives/Symbol";
import type {
  PerpAssetMetadata,
  SequenceNumber,
  ShortEntries,
} from "../internal";

export interface PerpAssetMap {
  sequenceNumber: SequenceNumber;
  numAssets: number;
  metadata: ShortEntries<Symbol, PerpAssetMetadata>;
}
