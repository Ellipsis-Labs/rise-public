import type {
  Authority,
  InstructionsWithAccountsAndData,
  TraderAddress,
} from "@/primitives/index.js";
import { buildProxyInstructionIx } from "./core/index.js";
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
  /**
   * Wallet that signs the wrapped instruction — the effective signer
   * (`positionAuthority ?? ownerAuthority`), placed verbatim in the proxy's
   * trader-wallet slot. Not necessarily the trader account's owner.
   */
  signer: Authority;
  phoenixProgramAddress: PhoenixInstructionClient["addresses"]["phoenixProgramAddress"];
  flight: PhoenixFlightClientConfig;
  /**
   * Set when `signer` signs as the trader's position authority rather than
   * as the trader account's owner, so the collateral-transfer tail accounts
   * are appended to the proxy instruction. This declaration is the single
   * source of truth for the tail: wraps never infer it from the inner
   * instruction, so owner-signed delegated market orders wrap without the
   * tail (Flight detects the owner signer on-chain and uses the plain
   * transfer, and the tail would needlessly write-lock a global permission
   * account).
   */
  usePositionAuthority?: boolean;
  resolveFeeCollectorTraderAddress: (
    traderPdaIndex: number,
    subaccountIndex: number
  ) => Promise<TraderAddress>;
  /**
   * Supplies the current Phoenix root authority used to derive the
   * collateral-transfer permission account. Only invoked — and only
   * required — when `usePositionAuthority` is set.
   */
  resolveRootAuthority?: () => Promise<Authority>;
}): Promise<InstructionsWithAccountsAndData> => {
  if (!isFlightRoutableInstruction(params.phoenixInstruction)) {
    return params.phoenixInstruction;
  }

  let rootAuthority: Authority | undefined;
  if (params.usePositionAuthority === true) {
    if (params.resolveRootAuthority === undefined) {
      throw new Error(
        "Root authority is required for position-authority wraps; pass resolveRootAuthority"
      );
    }
    rootAuthority = await params.resolveRootAuthority();
  }

  return buildProxyInstructionIx({
    phoenixProgramAddress: params.phoenixProgramAddress,
    builderAuthority: params.flight.builderAuthority,
    builderTraderAccount: await resolvePhoenixFlightFeeCollectorTraderAddress(
      params.flight,
      params.resolveFeeCollectorTraderAddress
    ),
    traderWallet: params.signer,
    feeBpsOverride: params.flight.feeBpsOverride,
    rootAuthority,
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

  /**
   * Wrap a Flight-routable instruction in a Flight proxy instruction; return
   * unsupported instructions unchanged (hence the
   * `InstructionsWithAccountsAndData` return type — passthroughs are not
   * proxy instructions). Exact mirror of the Rust rise client's
   * `try_wrap_order_instruction`: `signer` is the wallet that signs the
   * wrapped instruction, and `usePositionAuthority` declares that the signer
   * is the trader's position authority rather than the trader account's
   * owner — derive it as `signer !== ownerAuthority` when the owner is
   * known, never from the instruction being wrapped.
   *
   * With `usePositionAuthority` set, the collateral-transfer authority and
   * permission accounts are appended so Flight can collect the builder fee
   * via `AuthorizedTransferCollateral`; the permission account derives from
   * the current Phoenix root authority, resolved from the wrapped
   * instruction client's exchange metadata on every wrap (throws when that
   * metadata is unavailable). Owner-signed orders — including owner-signed
   * `PlaceMarketOrderDelegated` — must leave it `false`: on-chain, Flight
   * detects the owner signature and settles via the plain transfer, and the
   * tail write-locks a global permission account.
   */
  async tryWrapOrderInstruction(
    phoenixInstruction: InstructionsWithAccountsAndData,
    signer: Authority,
    usePositionAuthority = false
  ): Promise<InstructionsWithAccountsAndData> {
    return wrapInstructionWithFlight({
      phoenixInstruction,
      signer,
      usePositionAuthority,
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
      // Only invoked for position-authority wraps, and always resolved from
      // the exchange snapshot at wrap time (never cached): the root authority
      // can rotate on-chain and the snapshot store tracks
      // `exchangeKeysUpdated` deltas. `snapshot()` is a cheap in-memory read
      // once `ready()` has resolved.
      resolveRootAuthority: async () => {
        const exchange = this.instructionClient.exchange;
        if (exchange === undefined) {
          throw new Error(
            "Flight position-authority orders require exchange metadata to resolve the root authority"
          );
        }
        await exchange.ready();
        return exchange.snapshot().exchange.currentAuthorities
          .rootAuthority as Authority;
      },
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
