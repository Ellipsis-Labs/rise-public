import { getPhoenixProgramAddress } from "@/core/constants";
import type { Authority, PhoenixProgramAddress } from "@/primitives";
import { getBase58Encoder, getProgramDerivedAddress } from "@solana/kit";
import {
  FLIGHT_PROGRAM_ADDRESS,
  BUILDER_STATE_SEED,
  COLLATERAL_TRANSFER_AUTHORITY_SEED,
  GLOBAL_STATE_SEED,
} from "./core/constants";
import type {
  FlightBuilderStateAddress,
  FlightCollateralTransferAuthorityAddress,
  FlightAuthorizedCollateralTransferPermissionAddress,
  FlightGlobalStateAddress,
} from "./types";

export const getFlightGlobalStateAddress = async (
  phoenixProgramAddress: PhoenixProgramAddress = getPhoenixProgramAddress()
): Promise<FlightGlobalStateAddress> => {
  const [pda] = await getProgramDerivedAddress({
    programAddress: FLIGHT_PROGRAM_ADDRESS,
    seeds: [
      getBase58Encoder().encode(phoenixProgramAddress),
      GLOBAL_STATE_SEED,
    ],
  });

  return pda as FlightGlobalStateAddress;
};

export const getFlightBuilderStateAddress = async (
  authority: Authority,
  phoenixProgramAddress: PhoenixProgramAddress = getPhoenixProgramAddress()
): Promise<FlightBuilderStateAddress> => {
  const [pda] = await getProgramDerivedAddress({
    programAddress: FLIGHT_PROGRAM_ADDRESS,
    seeds: [
      getBase58Encoder().encode(phoenixProgramAddress),
      getBase58Encoder().encode(authority),
      BUILDER_STATE_SEED,
    ],
  });

  return pda as FlightBuilderStateAddress;
};

export const getFlightCollateralTransferAuthorityAddress = async (
  phoenixProgramAddress: PhoenixProgramAddress = getPhoenixProgramAddress()
): Promise<FlightCollateralTransferAuthorityAddress> => {
  const [pda] = await getProgramDerivedAddress({
    programAddress: FLIGHT_PROGRAM_ADDRESS,
    seeds: [
      COLLATERAL_TRANSFER_AUTHORITY_SEED,
      getBase58Encoder().encode(phoenixProgramAddress),
    ],
  });

  return pda as FlightCollateralTransferAuthorityAddress;
};

export const getFlightAuthorizedCollateralTransferPermissionAddress = async (
  rootAuthority: Authority,
  phoenixProgramAddress: PhoenixProgramAddress = getPhoenixProgramAddress()
): Promise<FlightAuthorizedCollateralTransferPermissionAddress> => {
  const collateralTransferAuthority =
    await getFlightCollateralTransferAuthorityAddress(phoenixProgramAddress);
  const [pda] = await getProgramDerivedAddress({
    programAddress: phoenixProgramAddress,
    seeds: [
      "permission",
      getBase58Encoder().encode(rootAuthority),
      getBase58Encoder().encode(collateralTransferAuthority),
    ],
  });

  return pda as FlightAuthorizedCollateralTransferPermissionAddress;
};
