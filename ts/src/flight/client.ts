import type {
  Authority,
  InstructionsWithAccountsAndData,
  TraderAddress,
} from "@/primitives/index.js";
import {
  buildProxyInstructionIx,
  type ProxyInstructionIx,
} from "./core/index.js";
import type {
  PhoenixBuilderAddresses,
  PhoenixInstructionClient,
} from "@/core/clientTypes.js";
import type { PhoenixExchangeMetadata } from "@/exchange-cache/types.js";
import type { Address, ReadonlyUint8Array } from "@solana/kit";
import { isFlightRoutableInstruction } from "./helper.js";
import { getPhoenixTraderSubaccountAddress } from "@/pdas.js";

export interface PhoenixFlightClientConfig {
  builderAuthority: Authority;
  builderPdaIndex?: number;
  builderSubaccountIndex?: number;
  feeBpsOverride?: bigint | null;
}

export const resolvePhoenixFlightFeeCollectorTraderAddress = async (
  config: PhoenixFlightClientConfig,
  deriveTraderAddress: (
    traderPdaIndex: number,
    subaccountIndex: number
  ) => Promise<TraderAddress>
): Promise<TraderAddress> => {
  return deriveTraderAddress(
    config.builderPdaIndex ?? 0,
    config.builderSubaccountIndex ?? 0
  );
};

export interface PhoenixFlightOrderRequestFields {
  flightBuilderAuthority: Authority;
  flightFeeCollectorTrader: TraderAddress;
}

export const resolvePhoenixFlightOrderRequestFields = async (
  config: PhoenixFlightClientConfig,
  deriveTraderAddress: (
    traderPdaIndex: number,
    subaccountIndex: number
  ) => Promise<TraderAddress>
): Promise<PhoenixFlightOrderRequestFields> => ({
  flightBuilderAuthority: config.builderAuthority,
  flightFeeCollectorTrader: await resolvePhoenixFlightFeeCollectorTraderAddress(
    config,
    deriveTraderAddress
  ),
});

export const wrapInstructionWithFlight = async (params: {
  phoenixInstruction: InstructionsWithAccountsAndData;
  authority: Authority;
  phoenixProgramAddress: PhoenixInstructionClient["addresses"]["phoenixProgramAddress"];
  flight: PhoenixFlightClientConfig;
  resolveFeeCollectorTraderAddress: (
    traderPdaIndex: number,
    subaccountIndex: number
  ) => Promise<TraderAddress>;
}): Promise<InstructionsWithAccountsAndData> => {
  if (!isFlightRoutableInstruction(params.phoenixInstruction)) {
    return params.phoenixInstruction;
  }

  return buildProxyInstructionIx({
    phoenixProgramAddress: params.phoenixProgramAddress,
    builderAuthority: params.flight.builderAuthority,
    builderTraderAccount: await resolvePhoenixFlightFeeCollectorTraderAddress(
      params.flight,
      params.resolveFeeCollectorTraderAddress
    ),
    traderWallet: params.authority,
    feeBpsOverride: params.flight.feeBpsOverride,
    innerInstruction: params.phoenixInstruction,
  });
};

export class PhoenixFlightClient implements PhoenixInstructionClient {
  readonly builderAuthority: Authority;
  readonly builderPdaIndex: number;
  readonly builderSubaccountIndex: number;
  readonly feeBpsOverride: bigint | null;
  readonly instructionClient: PhoenixInstructionClient;

  constructor(
    instructionClient: PhoenixInstructionClient,
    config: PhoenixFlightClientConfig
  ) {
    this.instructionClient = instructionClient;
    this.builderAuthority = config.builderAuthority;
    this.builderPdaIndex = config.builderPdaIndex ?? 0;
    this.builderSubaccountIndex = config.builderSubaccountIndex ?? 0;
    this.feeBpsOverride = config.feeBpsOverride ?? null;
  }

  get addresses(): PhoenixBuilderAddresses {
    return this.instructionClient.addresses;
  }

  get exchange(): PhoenixExchangeMetadata | undefined {
    return this.instructionClient.exchange;
  }

  async fetchAccount(
    address: Address
  ): Promise<{ readonly data: ReadonlyUint8Array }> {
    return this.instructionClient.fetchAccount(address);
  }

  async tryWrapFlightInstruction(
    phoenixInstruction: InstructionsWithAccountsAndData,
    authority: Authority
  ): Promise<ProxyInstructionIx> {
    return wrapInstructionWithFlight({
      phoenixInstruction,
      authority,
      phoenixProgramAddress:
        this.instructionClient.addresses.phoenixProgramAddress,
      flight: this,
      resolveFeeCollectorTraderAddress: (pdaIndex, subaccountIndex) =>
        getPhoenixTraderSubaccountAddress({
          authority: this.builderAuthority,
          traderPdaIndex: pdaIndex,
          subaccountIndex,
          phoenixProgramAddress:
            this.instructionClient.addresses.phoenixProgramAddress,
        }),
    });
  }
}

export const isFlightClient = (
  client: PhoenixInstructionClient
): client is PhoenixFlightClient => {
  return (
    "builderAuthority" in client &&
    "builderPdaIndex" in client &&
    "builderSubaccountIndex" in client
  );
};
