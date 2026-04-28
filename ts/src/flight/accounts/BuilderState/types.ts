import type { Authority, TraderAddress } from "@/primitives";

export interface BuilderState {
  discriminant: bigint;
  authorityKey: Authority;
  traderKey: TraderAddress;
  status: bigint;
  isActive: boolean;
  feeBps: bigint;
}
