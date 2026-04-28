import type {
  ActiveTraderBufferAddressArray,
  GlobalTraderIndexAddressArray,
} from "@/primitives";
import type {
  AccountMeta,
  Address,
  ReadonlyAccount,
  ReadonlySignerAccount,
  WritableAccount,
  WritableSignerAccount,
} from "@solana/kit";
import { AccountRole } from "@solana/kit";

export const generateAccountMeta = (
  address: Address,
  role: AccountRole
): AccountMeta => {
  return {
    address: address,
    role: role,
  };
};

export const generateReadonlyAccount = (address: Address): ReadonlyAccount => {
  return {
    address: address,
    role: AccountRole.READONLY,
  };
};

export const generateWritableAccount = (address: Address): WritableAccount => {
  return {
    address: address,
    role: AccountRole.WRITABLE,
  };
};

export const generateReadonlySignerAccount = (
  address: Address
): ReadonlySignerAccount => {
  return {
    address: address,
    role: AccountRole.READONLY_SIGNER,
  };
};

export const generateWritableSignerAccount = (
  address: Address
): WritableSignerAccount => {
  return {
    address: address,
    role: AccountRole.WRITABLE_SIGNER,
  };
};

export const generateArenaAccounts = (
  addresses: ActiveTraderBufferAddressArray | GlobalTraderIndexAddressArray
): AccountMeta[] => {
  return addresses.map((address) => generateWritableAccount(address));
};
