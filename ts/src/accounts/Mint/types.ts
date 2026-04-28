import type { Authority } from "@/primitives/_addressTypes";

export interface Mint {
  mintAuthorityOption: 0 | 1;
  mintAuthority: Authority;
  supply: bigint;
  decimals: number;
  isInitialized: boolean;
  freezeAuthorityOption: 0 | 1;
  freezeAuthority: Authority;
}
