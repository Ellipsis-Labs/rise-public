import type { Branded } from "../_utilityTypes/Branded";

export type Ticks = Branded<bigint, "Ticks">; // u64
export type BaseLots = Branded<bigint, "BaseLots">; // u64
export type BaseLotsPerTick = Branded<bigint, "BaseLotsPerTick">; // u64
export type QuoteLots = Branded<bigint, "QuoteLots">; // u64
export type SignedBaseLots = Branded<bigint, "SignedBaseLots">; // i64
export type SignedQuoteLots = Branded<bigint, "SignedQuoteLots">; // i64
