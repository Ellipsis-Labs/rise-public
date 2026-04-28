import type { Address } from "@solana/kit";

export interface Permission {
  permissionAuthority: Address;
  delegatedKey: Address;
  permission: bigint;
  expiresAtTimestamp: bigint;
  allowedSignerActions: bigint;
}
