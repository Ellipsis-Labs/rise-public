import type { Authority, MintAddress } from "@/primitives/_addressTypes";

export enum TokenAccountState {
  Uninitialized = 0,
  Initialized = 1,
  Frozen = 2,
}

export interface TokenAccount {
  mint: MintAddress;
  owner: Authority;
  amount: bigint;
  delegateOption: 0 | 1;
  delegate: Authority;
  state: TokenAccountState;
  isNativeOption: 0 | 1;
  isNative: bigint;
  delegatedAmount: bigint;
  closeAuthorityOption: 0 | 1;
  closeAuthority: Authority;
}
