import {
  SYSTEM_PROGRAM_ADDRESS,
  getPhoenixProgramAddress,
  type ResolvePhoenixInstructionAddressesInput,
} from "@/core/constants";
import type { PhoenixProgramAddress, SystemProgramAddress } from "@/primitives";
import { address } from "@solana/kit";
import type { FlightProgramAddress } from "@/flight/types";

export const FLIGHT_PROGRAM_ADDRESS = address(
  "F1ightu9cujFYo34k9CabifLrJT8qzfDVM2Q7BqhJn2W"
) as FlightProgramAddress;

export interface FlightInstructionAddresses {
  programAddress: FlightProgramAddress;
  phoenixProgramAddress: PhoenixProgramAddress;
  systemProgramAddress: SystemProgramAddress;
}

export interface FlightInstructionAddressSource {
  flightProgramAddress: FlightProgramAddress;
  phoenixProgramAddress: PhoenixProgramAddress;
  systemProgramAddress: SystemProgramAddress;
}

export interface FlightInstructionAddressCarrier {
  addresses: FlightInstructionAddressSource;
}

export type FlightInstructionAddressOverrides =
  Partial<FlightInstructionAddresses>;

export interface ResolveFlightInstructionAddressesInput
  extends
    FlightInstructionAddressOverrides,
    Pick<ResolvePhoenixInstructionAddressesInput, "phoenixEnv"> {}

export const resolveFlightInstructionAddresses = (
  input: ResolveFlightInstructionAddressesInput = {}
): FlightInstructionAddresses => ({
  programAddress: input.programAddress ?? FLIGHT_PROGRAM_ADDRESS,
  phoenixProgramAddress:
    input.phoenixProgramAddress ??
    getPhoenixProgramAddress({ phoenixEnv: input.phoenixEnv }),
  systemProgramAddress: input.systemProgramAddress ?? SYSTEM_PROGRAM_ADDRESS,
});

export const getFlightProgramAddress = (
  input: ResolveFlightInstructionAddressesInput = {}
): FlightProgramAddress =>
  resolveFlightInstructionAddresses(input).programAddress;

export const getFlightInstructionAddresses = (
  input: ResolveFlightInstructionAddressesInput = {}
): FlightInstructionAddresses => resolveFlightInstructionAddresses(input);

export const flightInstructionAddresses = (
  addresses: FlightInstructionAddressSource
): FlightInstructionAddresses => ({
  programAddress: addresses.flightProgramAddress,
  phoenixProgramAddress: addresses.phoenixProgramAddress,
  systemProgramAddress: addresses.systemProgramAddress,
});

export const clientFlightInstructionAddresses = (
  client: FlightInstructionAddressCarrier
): FlightInstructionAddresses => flightInstructionAddresses(client.addresses);

export const GLOBAL_STATE_SEED = "global_state";
export const BUILDER_STATE_SEED = "builder_state";
