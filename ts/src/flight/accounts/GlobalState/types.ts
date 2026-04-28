import type { Authority, PhoenixProgramAddress } from "@/primitives";

export interface GlobalState {
  discriminant: bigint;
  programId: PhoenixProgramAddress;
  currentAuthority: Authority;
  pendingAuthority: Authority;
  maxFeeCapBps: bigint;
}
