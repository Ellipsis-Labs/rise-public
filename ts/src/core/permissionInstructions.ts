import {
  getPhoenixInstructionAddresses,
  SYSTEM_PROGRAM_ADDRESS,
} from "@/core/constants";
import { DISCRIMINANTS } from "@/core/discriminants";
import {
  generateReadonlyAccount,
  generateReadonlySignerAccount,
  generateWritableAccount,
} from "@/core/utils/accountMeta";
import {
  getOptionToNullDecoder,
  getOptionToNullEncoder,
} from "@/core/utils/optionCodec";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta, Address } from "@solana/kit";
import {
  combineCodec,
  getConstantDecoder,
  getConstantEncoder,
  getHiddenPrefixDecoder,
  getHiddenPrefixEncoder,
  getI64Decoder,
  getI64Encoder,
  getStructDecoder,
  getStructEncoder,
  getU64Decoder,
  getU64Encoder,
  type Codec,
  type Decoder,
  type Encoder,
} from "@solana/kit";
import type { PhoenixInstructionAddressOverrides } from "./constants";

export interface CreatePermissionParams extends PhoenixInstructionAddressOverrides {
  payer: Address;
  permissionAuthority: Address;
  delegatedKey: Address;
  permissionPda: Address;
}

export interface SetPermissionData {
  permission: bigint;
  expiresAtTimestamp: bigint | null;
  allowedSignerActions: bigint | null;
}

export interface SetPermissionParams
  extends PhoenixInstructionAddressOverrides, SetPermissionData {
  permissionAuthority: Address;
  delegatedKey: Address;
  permissionPda: Address;
}

export type CreatePermissionIx = InstructionsWithAccountsAndData;
export type CreatePermissionAccounts = readonly AccountMeta[];
export type SetPermissionIx = InstructionsWithAccountsAndData;
export type SetPermissionAccounts = readonly AccountMeta[];

export const getCreatePermissionEncoder = (): Encoder<void> =>
  getConstantEncoder(DISCRIMINANTS.CREATE_PERMISSION);

export const buildCreatePermissionIx = (
  params: CreatePermissionParams
): CreatePermissionIx => {
  const { programAddress, logAuthorityAddress } =
    getPhoenixInstructionAddresses(params);

  return {
    programAddress,
    accounts: [
      generateReadonlyAccount(programAddress),
      generateReadonlyAccount(logAuthorityAddress),
      generateReadonlySignerAccount(params.payer),
      generateWritableAccount(params.permissionPda),
      generateReadonlyAccount(params.permissionAuthority),
      generateReadonlyAccount(params.delegatedKey),
      generateReadonlyAccount(SYSTEM_PROGRAM_ADDRESS),
    ] as const,
    data: getCreatePermissionEncoder().encode(undefined),
  };
};

export const getSetPermissionParamsEncoder = (): Encoder<SetPermissionData> =>
  getStructEncoder([
    ["permission", getU64Encoder()],
    ["expiresAtTimestamp", getOptionToNullEncoder(getI64Encoder())],
    ["allowedSignerActions", getOptionToNullEncoder(getI64Encoder())],
  ]);

export const getSetPermissionParamsDecoder = (): Decoder<SetPermissionData> =>
  getStructDecoder([
    ["permission", getU64Decoder()],
    ["expiresAtTimestamp", getOptionToNullDecoder(getI64Decoder())],
    ["allowedSignerActions", getOptionToNullDecoder(getI64Decoder())],
  ]);

export const getSetPermissionParamsCodec = (): Codec<SetPermissionData> =>
  combineCodec(
    getSetPermissionParamsEncoder(),
    getSetPermissionParamsDecoder()
  );

export const getSetPermissionInstructionEncoder =
  (): Encoder<SetPermissionData> =>
    getHiddenPrefixEncoder(getSetPermissionParamsEncoder(), [
      getConstantEncoder(DISCRIMINANTS.SET_PERMISSION),
    ]);

export const getSetPermissionInstructionDecoder =
  (): Decoder<SetPermissionData> =>
    getHiddenPrefixDecoder(getSetPermissionParamsDecoder(), [
      getConstantDecoder(DISCRIMINANTS.SET_PERMISSION),
    ]);

export const getSetPermissionInstructionCodec = (): Codec<SetPermissionData> =>
  combineCodec(
    getSetPermissionInstructionEncoder(),
    getSetPermissionInstructionDecoder()
  );

export const buildSetPermissionIx = (
  params: SetPermissionParams
): SetPermissionIx => {
  const { programAddress, logAuthorityAddress } =
    getPhoenixInstructionAddresses(params);

  return {
    programAddress,
    accounts: [
      generateReadonlyAccount(programAddress),
      generateReadonlyAccount(logAuthorityAddress),
      generateWritableAccount(params.permissionPda),
      generateReadonlySignerAccount(params.permissionAuthority),
      generateReadonlyAccount(params.delegatedKey),
    ] as const,
    data: getSetPermissionInstructionEncoder().encode({
      permission: params.permission,
      expiresAtTimestamp: params.expiresAtTimestamp,
      allowedSignerActions: params.allowedSignerActions,
    }),
  };
};
