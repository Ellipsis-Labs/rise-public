import {
  generateReadonlyAccount,
  generateWritableAccount,
} from "@/core/utils/accountMeta";
import { getFlightInstructionAddresses } from "@/flight/core/constants";
import {
  getFlightBuilderStateAddress,
  getFlightCollateralTransferAuthorityAddress,
  getFlightAuthorizedCollateralTransferPermissionAddress,
  getFlightGlobalStateAddress,
} from "@/flight/pdas";
import {
  encodeProxyInstructionData,
  encodeProxyInstructionWithFeeOverrideData,
} from "./codec";
import type {
  ProxyInstructionAccounts,
  ProxyInstructionIx,
  ProxyInstructionParams,
} from "./types";
import type { PhoenixProgramAddress } from "@/primitives/index.js";

const MAX_BASIS_POINTS = 10_000n;

export const buildProxyInstructionIx = async (
  params: ProxyInstructionParams
): Promise<ProxyInstructionIx> => {
  const resolvedAddresses = getFlightInstructionAddresses(params);

  const { programAddress, phoenixProgramAddress } = resolvedAddresses;

  validate(params, phoenixProgramAddress);

  const [globalStateAccount, builderStateAccount] = await Promise.all([
    getFlightGlobalStateAddress(phoenixProgramAddress),
    getFlightBuilderStateAddress(
      params.builderAuthority,
      phoenixProgramAddress
    ),
  ]);

  const accounts = [
    generateReadonlyAccount(globalStateAccount),
    generateReadonlyAccount(phoenixProgramAddress),
    generateReadonlyAccount(params.builderAuthority),
    generateWritableAccount(params.builderTraderAccount),
    generateReadonlyAccount(builderStateAccount),
    generateReadonlyAccount(params.traderWallet),
    ...(params.innerInstruction.accounts ?? []),
  ];

  // `rootAuthority` presence is the single source of truth for the
  // collateral-transfer tail: the tail write-locks a global permission
  // account, so it is appended only when the caller declares that
  // `traderWallet` signs as the trader's position authority. Owner-signed
  // orders — including owner-signed `PlaceMarketOrderDelegated` — never
  // need it; Flight detects the owner signer on-chain and collects the fee
  // via the plain transfer.
  if (params.rootAuthority != null) {
    const [collateralTransferAuthority, collateralTransferPermissionAccount] =
      await Promise.all([
        getFlightCollateralTransferAuthorityAddress(phoenixProgramAddress),
        getFlightAuthorizedCollateralTransferPermissionAddress(
          params.rootAuthority,
          phoenixProgramAddress
        ),
      ]);
    accounts.push(generateReadonlyAccount(collateralTransferAuthority));
    accounts.push(generateWritableAccount(collateralTransferPermissionAccount));
  }

  const innerInstructionData =
    params.innerInstruction.data ?? new Uint8Array(0);
  const data =
    params.feeBpsOverride == null
      ? encodeProxyInstructionData(innerInstructionData)
      : encodeProxyInstructionWithFeeOverrideData(
          params.feeBpsOverride,
          innerInstructionData
        );

  return {
    programAddress,
    accounts: accounts as ProxyInstructionAccounts,
    data,
  };
};

const validate = (
  params: ProxyInstructionParams,
  phoenixProgramAddress: PhoenixProgramAddress
) => {
  if (!params.builderAuthority) {
    throw new Error("Builder authority is required");
  }
  if (!params.builderTraderAccount) {
    throw new Error("Builder trader account is required");
  }
  if (!params.traderWallet) {
    throw new Error("Trader wallet is required");
  }
  if (!params.innerInstruction?.programAddress) {
    throw new Error("Inner instruction program address is required");
  }
  if (params.feeBpsOverride != null) {
    if (
      params.feeBpsOverride < 0n ||
      params.feeBpsOverride > MAX_BASIS_POINTS
    ) {
      // Keep this wording in lockstep with the Rust builder's
      // `PhoenixIxError::InvalidFeeBpsOverride` display string.
      throw new Error("Invalid fee bps override (must be in 0..=10000)");
    }
  }

  if (params.innerInstruction.programAddress !== phoenixProgramAddress) {
    throw new Error(
      "Inner instruction program address must match phoenix program address"
    );
  }
};
