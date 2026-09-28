import type { ResolvePhoenixInstructionAddressesInput } from "@/core/constants";
import type { ResolveFlightInstructionAddressesInput } from "@/flight/core/constants";
import type {
  ActiveTraderBufferAddressArray,
  Authority,
  GlobalTraderIndexAddressArray,
} from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta } from "@solana/kit";

export interface OnboardTraderParams
  extends
    ResolveFlightInstructionAddressesInput,
    Pick<
      ResolvePhoenixInstructionAddressesInput,
      "logAuthorityAddress" | "globalConfigurationAddress"
    > {
  builderAuthority: Authority;
  onboarderSigner: Authority;
  payer: Authority;
  traderWallet: Authority;
  riskAuthority: Authority;
  marketAuthority: Authority;
  maxPositions: number;
  traderPreferenceBits: number;
  globalTraderIndex: GlobalTraderIndexAddressArray;
  activeTraderBuffer: ActiveTraderBufferAddressArray;
}

export type OnboardTraderAccounts = readonly AccountMeta[];
export type OnboardTraderIx = InstructionsWithAccountsAndData;
