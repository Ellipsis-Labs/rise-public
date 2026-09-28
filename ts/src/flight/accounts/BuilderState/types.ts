import type { Authority, TraderAddress } from "@/primitives";
import type { Address } from "@solana/kit";

export interface BuilderState {
  discriminant: bigint;
  authorityKey: Authority;
  traderKey: TraderAddress;
  status: bigint;
  isActive: boolean;
  feeBps: bigint;
  onboarderSignerPubkey: Address;
  onboarderNumOnboardingRemaining: bigint;
  onboarderMakerFeeDiscount: number;
  onboarderTakerFeeDiscount: number;
}
