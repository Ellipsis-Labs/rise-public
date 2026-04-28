import type { MarketResponse } from "@/api/markets/types";
import type { AccountFetcherClient } from "@/accounts/fetcherFactory";
import type { PhoenixExchangeMetadata } from "@/exchange-cache/types";
import type {
  Authority,
  EmberStateAddress,
  GlobalConfigurationAddress,
  LogAuthorityAddress,
  MintAddress,
  PhoenixProgramAddress,
} from "@/primitives";
import type { Instruction, Signature } from "@solana/kit";
import type { Address } from "@solana/kit";

export interface PhoenixBuilderAddresses {
  phoenixProgramAddress: PhoenixProgramAddress;
  logAuthorityAddress: LogAuthorityAddress;
  globalConfigurationAddress: GlobalConfigurationAddress;
  usdcMintAddress: MintAddress;
  emberStateAddress: EmberStateAddress;
  flightBuilderAuthority?: Authority;
}

export interface PhoenixInstructionClient extends AccountFetcherClient {
  readonly addresses: PhoenixBuilderAddresses;
  readonly exchange?: PhoenixExchangeMetadata;
}

export interface PhoenixAccountExistenceClient {
  accountExists: (address: Address) => Promise<boolean>;
}

export interface PhoenixMarketDataClient {
  readonly markets: {
    getMarket: (symbol: string) => Promise<MarketResponse>;
  };
}

export interface SendInstructionOptions {
  commitment?: "processed" | "confirmed" | "finalized";
  priorityFee?: number;
}

export interface PhoenixTransactionClient {
  sendAndConfirmFromInstruction: (
    ix: Instruction | Instruction[],
    options?: SendInstructionOptions
  ) => Promise<Signature>;
}
