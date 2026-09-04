import {
  FLICKER_PROGRAM_ADDRESS,
  SYSTEM_PROGRAM_ADDRESS,
  getPhoenixInstructionAddresses,
} from "@/core/constants";
import {
  generateReadonlyAccount,
  generateReadonlySignerAccount,
  generateWritableAccount,
  generateWritableSignerAccount,
} from "@/core/utils/accountMeta";
import {
  POSITION_AUTHORITY_PERMISSION,
  buildCreatePermissionIx,
  buildSetPermissionIx,
} from "@/core/permissionInstructions";
import { address, type AccountMeta, type Address } from "@solana/kit";
import {
  getCancelTwapOrderEncoder,
  getCloseInactiveTwapAccountEncoder,
  getCreateTwapAccountEncoder,
  getExecuteTwapOrderEncoder,
  getPlaceTwapOrderEncoder,
} from "./codec";
import type {
  CancelTwapOrderAccounts,
  CancelTwapOrderIx,
  CancelTwapOrderParams,
  CloseInactiveTwapAccountAccounts,
  CloseInactiveTwapAccountIx,
  CloseInactiveTwapAccountParams,
  CreateTwapAccountAccounts,
  CreateTwapAccountIx,
  CreateTwapAccountParams,
  DisableTwapIx,
  DisableTwapParams,
  EnableTwapIxs,
  EnableTwapParams,
  ExecuteTwapOrderAccounts,
  ExecuteTwapOrderIx,
  ExecuteTwapOrderParams,
  PlaceTwapOrderAccounts,
  PlaceTwapOrderIx,
  PlaceTwapOrderParams,
  ResolvedTwapInstructionAddresses,
  TwapInstructionAddressOverrides,
} from "./types";

const HAWKEYE_PROGRAM_ADDRESS = address(
  "RiSeVw3ZjNfsaXPRb4mgaqYaEEt41pNNJoDvVh7pgQj"
) as Address;

const resolveTwapInstructionAddresses = (
  params: TwapInstructionAddressOverrides
): ResolvedTwapInstructionAddresses => {
  const { programAddress } = getPhoenixInstructionAddresses(params);
  return {
    phoenixProgramAddress: programAddress,
    flickerProgramAddress:
      params.flickerProgramAddress ?? FLICKER_PROGRAM_ADDRESS,
    twapGlobalStateAddress: params.twapGlobalStateAddress,
    twapLogAuthorityAddress: params.twapLogAuthorityAddress,
    hawkeyeProgramAddress:
      params.hawkeyeProgramAddress ?? HAWKEYE_PROGRAM_ADDRESS,
  };
};

export const buildCreateTwapAccountIx = (
  params: CreateTwapAccountParams
): CreateTwapAccountIx => {
  validateCreateTwapAccount(params);
  const addresses = resolveTwapInstructionAddresses(params);
  const data = getCreateTwapAccountEncoder().encode({
    marketId: params.marketId,
  });

  const accounts: CreateTwapAccountAccounts = [
    generateReadonlyAccount(addresses.twapGlobalStateAddress),
    generateReadonlyAccount(addresses.phoenixProgramAddress),
    generateWritableAccount(params.twapAccount),
    generateReadonlyAccount(params.traderAccount),
    generateWritableSignerAccount(params.payer),
    generateReadonlyAccount(SYSTEM_PROGRAM_ADDRESS),
    generateReadonlyAccount(addresses.twapLogAuthorityAddress),
    generateReadonlyAccount(addresses.flickerProgramAddress),
  ] as const;

  return {
    programAddress: addresses.flickerProgramAddress,
    accounts,
    data,
  };
};

export const buildPlaceTwapOrderIx = (
  params: PlaceTwapOrderParams
): PlaceTwapOrderIx => {
  validatePlaceTwapOrder(params);
  const addresses = resolveTwapInstructionAddresses(params);
  const transferAccounts = params.transferAccounts ?? [];
  const data = getPlaceTwapOrderEncoder().encode({
    cooldownSlots: params.cooldownSlots,
    nChildOrders: params.nChildOrders,
    childOrderMaxSlippageBps: params.childOrderMaxSlippageBps,
    childOrderMinPriceInTicks: params.childOrderMinPriceInTicks,
    childOrderMaxPriceInTicks: params.childOrderMaxPriceInTicks,
    childOrderPacket: params.childOrderPacket,
    dustOrderSize: params.dustOrderSize,
    childOrderCollateralQuoteLotsToTransfer:
      params.childOrderCollateralQuoteLotsToTransfer,
    lastValidSlot: params.lastValidSlot,
    transferCollateralAccountCount: transferAccounts.length,
  });

  const accounts: PlaceTwapOrderAccounts = [
    generateReadonlyAccount(addresses.twapGlobalStateAddress),
    generateReadonlyAccount(addresses.phoenixProgramAddress),
    generateReadonlyAccount(addresses.hawkeyeProgramAddress),
    generateReadonlyAccount(addresses.twapLogAuthorityAddress),
    generateWritableAccount(params.twapAccount),
    generateReadonlySignerAccount(params.authority),
    generateReadonlyAccount(addresses.flickerProgramAddress),
    ...transferAccounts,
    ...params.orderAccounts,
  ] as const;

  return {
    programAddress: addresses.flickerProgramAddress,
    accounts,
    data,
  };
};

export const buildExecuteTwapOrderIx = (
  params: ExecuteTwapOrderParams
): ExecuteTwapOrderIx => {
  validateExecuteTwapOrder(params);
  const addresses = resolveTwapInstructionAddresses(params);
  const transferAccounts = params.transferAccounts ?? [];
  const data = getExecuteTwapOrderEncoder().encode({
    transferCollateralAccountCount: transferAccounts.length,
  });

  const accounts: ExecuteTwapOrderAccounts = [
    generateReadonlyAccount(addresses.twapGlobalStateAddress),
    generateReadonlyAccount(addresses.phoenixProgramAddress),
    generateReadonlyAccount(addresses.hawkeyeProgramAddress),
    generateReadonlyAccount(addresses.twapLogAuthorityAddress),
    generateWritableAccount(params.twapAccount),
    generateReadonlyAccount(addresses.flickerProgramAddress),
    ...transferAccounts,
    ...params.orderAccounts,
  ] as const;

  return {
    programAddress: addresses.flickerProgramAddress,
    accounts,
    data,
  };
};

export const buildCancelTwapOrderIx = (
  params: CancelTwapOrderParams
): CancelTwapOrderIx => {
  validateCancelTwapOrder(params);
  const addresses = resolveTwapInstructionAddresses(params);
  const data = getCancelTwapOrderEncoder().encode(undefined);

  const accounts: CancelTwapOrderAccounts = [
    generateReadonlyAccount(addresses.twapGlobalStateAddress),
    generateReadonlyAccount(addresses.phoenixProgramAddress),
    generateReadonlyAccount(addresses.twapLogAuthorityAddress),
    generateWritableAccount(params.twapAccount),
    generateReadonlySignerAccount(params.authority),
    generateReadonlyAccount(addresses.flickerProgramAddress),
  ] as const;

  return {
    programAddress: addresses.flickerProgramAddress,
    accounts,
    data,
  };
};

export const buildCloseInactiveTwapAccountIx = (
  params: CloseInactiveTwapAccountParams
): CloseInactiveTwapAccountIx => {
  validateCloseInactiveTwapAccount(params);
  const addresses = resolveTwapInstructionAddresses(params);
  const data = getCloseInactiveTwapAccountEncoder().encode(undefined);

  const accounts: CloseInactiveTwapAccountAccounts = [
    generateReadonlyAccount(addresses.twapGlobalStateAddress),
    generateReadonlyAccount(addresses.phoenixProgramAddress),
    generateWritableAccount(params.twapAccount),
    generateWritableAccount(params.recipient),
    generateReadonlyAccount(addresses.twapLogAuthorityAddress),
    generateReadonlyAccount(addresses.flickerProgramAddress),
  ] as const;

  return {
    programAddress: addresses.flickerProgramAddress,
    accounts,
    data,
  };
};

/**
 * Builds the one-time TWAP enrollment pair: CreatePermission (idempotent
 * on-chain when the permission account already exists) followed by
 * SetPermission granting exactly the position authority bit to the global
 * Flicker delegate, with no expiration and unlimited signer actions. The
 * trader authority must sign; after this lands, TWAP placement and execution
 * need only the position authority.
 *
 * SetPermission overwrites the account's permission bits, which is safe here
 * because the (trader, TWAP global state) permission PDA carries only this
 * grant.
 */
export const buildEnableTwapIxs = (params: EnableTwapParams): EnableTwapIxs => {
  validateTwapPermissionParams(params);

  const createPermission = buildCreatePermissionIx({
    ...params,
    payer: params.payer ?? params.traderAuthority,
    permissionAuthority: params.traderAuthority,
    delegatedKey: params.twapGlobalStateAddress,
    permissionPda: params.permissionPda,
  });
  const setPermission = buildSetPermissionIx({
    ...params,
    permissionAuthority: params.traderAuthority,
    delegatedKey: params.twapGlobalStateAddress,
    permissionPda: params.permissionPda,
    permission: POSITION_AUTHORITY_PERMISSION,
    expiresAtTimestamp: null,
    allowedSignerActions: null,
  });

  return {
    createPermission,
    setPermission,
    instructions: [createPermission, setPermission] as const,
  };
};

/**
 * Revokes TWAP enrollment: SetPermission with no bits, which the program
 * treats as a revoke of the (trader, TWAP global state) permission account.
 */
export const buildDisableTwapIx = (
  params: DisableTwapParams
): DisableTwapIx => {
  validateTwapPermissionParams(params);

  return buildSetPermissionIx({
    ...params,
    permissionAuthority: params.traderAuthority,
    delegatedKey: params.twapGlobalStateAddress,
    permissionPda: params.permissionPda,
    permission: 0n,
    expiresAtTimestamp: null,
    allowedSignerActions: null,
  });
};

const validateTwapPermissionParams = (
  params: Pick<
    EnableTwapParams,
    "traderAuthority" | "twapGlobalStateAddress" | "permissionPda"
  >
) => {
  if (!params.traderAuthority) throw new Error("Trader authority is required");
  if (!params.twapGlobalStateAddress) {
    throw new Error("TWAP global state account is required");
  }
  if (!params.permissionPda) throw new Error("Permission PDA is required");
};

const validateTwapAddresses = (params: TwapInstructionAddressOverrides) => {
  if (!params.twapGlobalStateAddress) {
    throw new Error("TWAP global state account is required");
  }
  if (!params.twapLogAuthorityAddress) {
    throw new Error("TWAP log authority account is required");
  }
};

const validateTransferAccountCount = (
  accounts: readonly AccountMeta[] | undefined
) => {
  if ((accounts?.length ?? 0) > 255) {
    throw new Error("Transfer collateral account count cannot exceed 255");
  }
};

const validateCpiTail = (params: {
  transferAccounts?: readonly AccountMeta[];
  orderAccounts: readonly AccountMeta[];
}) => {
  validateTransferAccountCount(params.transferAccounts);
  if (!params.orderAccounts || params.orderAccounts.length === 0) {
    throw new Error("Order CPI accounts are required");
  }
};

const validateCreateTwapAccount = (params: CreateTwapAccountParams) => {
  validateTwapAddresses(params);
  if (!params.twapAccount) throw new Error("TWAP account is required");
  if (!params.traderAccount) throw new Error("Trader account is required");
  if (!params.payer) throw new Error("Payer is required");
  if (params.marketId === undefined || params.marketId === null) {
    throw new Error("Market id is required");
  }
};

const validatePlaceTwapOrder = (params: PlaceTwapOrderParams) => {
  validateTwapAddresses(params);
  validateCpiTail(params);
  if (!params.twapAccount) throw new Error("TWAP account is required");
  if (!params.authority) throw new Error("Authority is required");
  if (params.cooldownSlots === undefined || params.cooldownSlots === null) {
    throw new Error("Cooldown slots is required");
  }
  if (params.nChildOrders === undefined || params.nChildOrders === null) {
    throw new Error("Number of child orders is required");
  }
  if (params.nChildOrders <= 0n) {
    throw new Error("Number of child orders must be greater than 0");
  }
  if (
    params.childOrderMaxSlippageBps === undefined ||
    params.childOrderMaxSlippageBps === null
  ) {
    throw new Error("Child order max slippage bps is required");
  }
  if (!params.childOrderPacket) {
    throw new Error("Child order packet is required");
  }
  const dustOrderSize = params.dustOrderSize ?? 0n;
  if (dustOrderSize < 0n) {
    throw new Error("Dust order size must be non-negative");
  }
  if (dustOrderSize > 0n) {
    if (params.nChildOrders < 2n) {
      throw new Error("Dust order size requires at least 2 child orders");
    }
    if (dustOrderSize >= params.childOrderPacket.numBaseLots) {
      throw new Error("Dust order size must be less than the child order size");
    }
  }
};

const validateExecuteTwapOrder = (params: ExecuteTwapOrderParams) => {
  validateTwapAddresses(params);
  validateCpiTail(params);
  if (!params.twapAccount) throw new Error("TWAP account is required");
};

const validateCancelTwapOrder = (params: CancelTwapOrderParams) => {
  validateTwapAddresses(params);
  if (!params.twapAccount) throw new Error("TWAP account is required");
  if (!params.authority) throw new Error("Authority is required");
};

const validateCloseInactiveTwapAccount = (
  params: CloseInactiveTwapAccountParams
) => {
  validateTwapAddresses(params);
  if (!params.twapAccount) throw new Error("TWAP account is required");
  if (!params.recipient) throw new Error("Recipient is required");
};
