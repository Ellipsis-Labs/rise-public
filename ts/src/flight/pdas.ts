import { getPhoenixProgramAddress } from "@/core/constants";
import type { Authority, PhoenixProgramAddress } from "@/primitives";
import { getBase58Encoder, getProgramDerivedAddress } from "@solana/kit";
import {
  FLIGHT_PROGRAM_ADDRESS,
  BUILDER_STATE_SEED,
  GLOBAL_STATE_SEED,
} from "./core/constants";
import type {
  FlightBuilderStateAddress,
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
