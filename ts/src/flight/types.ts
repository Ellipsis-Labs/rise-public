import type { Branded } from "@/core/utils";
import type { Address } from "@solana/kit";

export type FlightProgramAddress = Branded<Address, "FlightProgram">;
export type FlightGlobalStateAddress = Branded<Address, "FlightGlobalState">;
export type FlightBuilderStateAddress = Branded<Address, "FlightBuilderState">;
