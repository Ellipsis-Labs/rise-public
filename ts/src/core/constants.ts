import type {
  EmberProgramAddress,
  EmberStateAddress,
  FlickerProgramAddress,
  GlobalConfigurationAddress,
  LogAuthorityAddress,
  MintAddress,
  PhoenixProgramAddress,
  SPLATAProgramAddress,
  SPLTokenProgramAddress,
  SystemProgramAddress,
} from "@/primitives/_addressTypes";
import { address } from "@solana/kit";

export const PHOENIX_PROGRAM_ADDRESS = address(
  "EtrnLzgbS7nMMy5fbD42kXiUzGg8XQzJ972Xtk1cjWih"
) as PhoenixProgramAddress;

export const PHOENIX_LOG_AUTHORITY_ADDRESS = address(
  "GdxfTLSsdSY37G6fZoYtdGDSfgFnbT2EmRpuePZxWShS"
) as LogAuthorityAddress;

export const PHOENIX_GLOBAL_CONFIGURATION_ADDRESS = address(
  "2zskx2iyCvb6Stg7RBZkt1f6MrF4dpYtMG3yMvKwqtUZ"
) as GlobalConfigurationAddress;

export const USDC_MINT_ADDRESS = address(
  "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
) as MintAddress;

export const SPL_TOKEN_PROGRAM_ADDRESS = address(
  "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
) as SPLTokenProgramAddress;

export const SPL_ATA_PROGRAM_ADDRESS = address(
  "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
) as SPLATAProgramAddress;

export const SYSTEM_PROGRAM_ADDRESS = address(
  "11111111111111111111111111111111"
) as SystemProgramAddress;

export const EMBER_PROGRAM_ADDRESS = address(
  "EMBERpYNE6ehWmXymZZS2skiFmCa9V5dp14e1iduM5qy"
) as EmberProgramAddress;

export const FLICKER_PROGRAM_ADDRESS = address(
  "FLickryJ5sBSyBRj7sSzMnsP93jvPxEqNKjvG49q9CEU"
) as FlickerProgramAddress;

export const EMBER_STATE_ADDRESS = address(
  "6ur7v6AXNpnHeEb6xuk7PyezvZ1i5GrgYyWZkNCpzbRz"
) as EmberStateAddress;

const BETA_PHOENIX_PROGRAM_ADDRESS = address(
  "phDEVv4w6BcfkLrLNeXr8HhhgQxnxziVGXpGPcaadMf"
) as PhoenixProgramAddress;

const BETA_PHOENIX_LOG_AUTHORITY_ADDRESS = address(
  "8Q1zeC7qAPUhJ2ncHAW8N1TGcpMVDsSxdSBPLaE8G2ab"
) as LogAuthorityAddress;

const BETA_PHOENIX_GLOBAL_CONFIGURATION_ADDRESS = address(
  "3CkM38UaZW6nyTJku4ABE5jjS5AComQErrkd55LGTfxa"
) as GlobalConfigurationAddress;

export const BETA_USDC_MINT_ADDRESS = address(
  "DPTSTVhvfzQhY8pAJL5EeqfZxf9aNKTmHErfG4R3Z1SE"
) as MintAddress;

const BETA_EMBER_STATE_ADDRESS = address(
  "HVpfk2HMkR85rvaXezoNjXfB5J4ds4PurSfi6n4DPm2Z"
) as EmberStateAddress;

export interface PhoenixInstructionAddresses {
  programAddress: PhoenixProgramAddress;
  logAuthorityAddress: LogAuthorityAddress;
  globalConfigurationAddress: GlobalConfigurationAddress;
}

export interface PhoenixInstructionAddressSource {
  phoenixProgramAddress: PhoenixProgramAddress;
  logAuthorityAddress: LogAuthorityAddress;
  globalConfigurationAddress: GlobalConfigurationAddress;
}

export interface PhoenixInstructionAddressCarrier {
  addresses: PhoenixInstructionAddressSource;
}

export type PhoenixEnv = "prod" | "beta";

export type PhoenixInstructionAddressOverrides =
  Partial<PhoenixInstructionAddresses>;

export interface ResolvePhoenixInstructionAddressesInput extends PhoenixInstructionAddressOverrides {
  phoenixEnv?: string | null;
}

interface RuntimePhoenixEnvShape {
  phoenixEnv?: string;
}

declare global {
  interface Window {
    __PHOENIX_ENV__?: RuntimePhoenixEnvShape;
  }
}

const PROD_PHOENIX_INSTRUCTION_ADDRESSES: PhoenixInstructionAddresses = {
  programAddress: PHOENIX_PROGRAM_ADDRESS,
  logAuthorityAddress: PHOENIX_LOG_AUTHORITY_ADDRESS,
  globalConfigurationAddress: PHOENIX_GLOBAL_CONFIGURATION_ADDRESS,
};

const BETA_PHOENIX_INSTRUCTION_ADDRESSES: PhoenixInstructionAddresses = {
  programAddress: BETA_PHOENIX_PROGRAM_ADDRESS,
  logAuthorityAddress: BETA_PHOENIX_LOG_AUTHORITY_ADDRESS,
  globalConfigurationAddress: BETA_PHOENIX_GLOBAL_CONFIGURATION_ADDRESS,
};

const normalizePhoenixEnv = (
  value: string | null | undefined
): PhoenixEnv | undefined => {
  const normalized = value?.trim().toLowerCase();
  if (!normalized) return undefined;
  if (normalized === "beta") return "beta";
  if (normalized === "prod" || normalized === "production") return "prod";
  return undefined;
};

const readRuntimePhoenixEnv = (): PhoenixEnv | undefined => {
  const windowEnv =
    typeof window !== "undefined"
      ? normalizePhoenixEnv(window.__PHOENIX_ENV__?.phoenixEnv)
      : undefined;
  if (windowEnv) return windowEnv;

  if (typeof document !== "undefined") {
    const domEnv = document.documentElement?.getAttribute("data-phoenix-env");
    const normalizedDomEnv = normalizePhoenixEnv(domEnv);
    if (normalizedDomEnv) return normalizedDomEnv;
  }

  return undefined;
};

const readProcessPhoenixEnv = (): PhoenixEnv | undefined => {
  if (typeof process === "undefined" || process === null) return undefined;
  if (typeof process.env !== "object" || process.env === null) return undefined;

  return (
    normalizePhoenixEnv(process.env.NEXT_PUBLIC_PHOENIX_ENV) ??
    normalizePhoenixEnv(process.env.PHOENIX_ENV)
  );
};

const knownPhoenixInstructionAddresses = (
  programAddress?: PhoenixProgramAddress
): PhoenixInstructionAddresses | undefined => {
  if (!programAddress) return undefined;
  if (programAddress === BETA_PHOENIX_PROGRAM_ADDRESS) {
    return BETA_PHOENIX_INSTRUCTION_ADDRESSES;
  }
  if (programAddress === PHOENIX_PROGRAM_ADDRESS) {
    return PROD_PHOENIX_INSTRUCTION_ADDRESSES;
  }
  return undefined;
};

export const resolvePhoenixEnv = (
  input: Pick<ResolvePhoenixInstructionAddressesInput, "phoenixEnv"> = {}
): PhoenixEnv => {
  return (
    normalizePhoenixEnv(input.phoenixEnv) ??
    readRuntimePhoenixEnv() ??
    readProcessPhoenixEnv() ??
    "prod"
  );
};

export const resolvePhoenixInstructionAddresses = (
  input: ResolvePhoenixInstructionAddressesInput = {}
): PhoenixInstructionAddresses => {
  const defaults =
    knownPhoenixInstructionAddresses(input.programAddress) ??
    (resolvePhoenixEnv(input) === "beta"
      ? BETA_PHOENIX_INSTRUCTION_ADDRESSES
      : PROD_PHOENIX_INSTRUCTION_ADDRESSES);

  return {
    programAddress: input.programAddress ?? defaults.programAddress,
    logAuthorityAddress:
      input.logAuthorityAddress ?? defaults.logAuthorityAddress,
    globalConfigurationAddress:
      input.globalConfigurationAddress ?? defaults.globalConfigurationAddress,
  };
};

/**
 * Full env-resolved address set for constructing a PhoenixInstructionClient's
 * `addresses`, including the per-environment USDC mint and ember state.
 * Unlike `resolvePhoenixInstructionAddresses`, this covers every address the
 * builders read from the client, so a beta client cannot silently fall back
 * to the mainnet USDC mint.
 */
export interface PhoenixBuilderAddressDefaults {
  phoenixProgramAddress: PhoenixProgramAddress;
  logAuthorityAddress: LogAuthorityAddress;
  globalConfigurationAddress: GlobalConfigurationAddress;
  usdcMintAddress: MintAddress;
  emberStateAddress: EmberStateAddress;
}

export interface ResolvePhoenixBuilderAddressesInput extends Partial<PhoenixBuilderAddressDefaults> {
  phoenixEnv?: string | null;
}

const PROD_PHOENIX_BUILDER_ADDRESSES: PhoenixBuilderAddressDefaults = {
  phoenixProgramAddress: PHOENIX_PROGRAM_ADDRESS,
  logAuthorityAddress: PHOENIX_LOG_AUTHORITY_ADDRESS,
  globalConfigurationAddress: PHOENIX_GLOBAL_CONFIGURATION_ADDRESS,
  usdcMintAddress: USDC_MINT_ADDRESS,
  emberStateAddress: EMBER_STATE_ADDRESS,
};

const BETA_PHOENIX_BUILDER_ADDRESSES: PhoenixBuilderAddressDefaults = {
  phoenixProgramAddress: BETA_PHOENIX_PROGRAM_ADDRESS,
  logAuthorityAddress: BETA_PHOENIX_LOG_AUTHORITY_ADDRESS,
  globalConfigurationAddress: BETA_PHOENIX_GLOBAL_CONFIGURATION_ADDRESS,
  usdcMintAddress: BETA_USDC_MINT_ADDRESS,
  emberStateAddress: BETA_EMBER_STATE_ADDRESS,
};

const knownPhoenixBuilderAddresses = (
  phoenixProgramAddress?: PhoenixProgramAddress
): PhoenixBuilderAddressDefaults | undefined => {
  if (!phoenixProgramAddress) return undefined;
  if (phoenixProgramAddress === BETA_PHOENIX_PROGRAM_ADDRESS) {
    return BETA_PHOENIX_BUILDER_ADDRESSES;
  }
  if (phoenixProgramAddress === PHOENIX_PROGRAM_ADDRESS) {
    return PROD_PHOENIX_BUILDER_ADDRESSES;
  }
  return undefined;
};

export const resolvePhoenixBuilderAddresses = (
  input: ResolvePhoenixBuilderAddressesInput = {}
): PhoenixBuilderAddressDefaults => {
  const defaults =
    knownPhoenixBuilderAddresses(input.phoenixProgramAddress) ??
    (resolvePhoenixEnv(input) === "beta"
      ? BETA_PHOENIX_BUILDER_ADDRESSES
      : PROD_PHOENIX_BUILDER_ADDRESSES);

  return {
    phoenixProgramAddress:
      input.phoenixProgramAddress ?? defaults.phoenixProgramAddress,
    logAuthorityAddress:
      input.logAuthorityAddress ?? defaults.logAuthorityAddress,
    globalConfigurationAddress:
      input.globalConfigurationAddress ?? defaults.globalConfigurationAddress,
    usdcMintAddress: input.usdcMintAddress ?? defaults.usdcMintAddress,
    emberStateAddress: input.emberStateAddress ?? defaults.emberStateAddress,
  };
};

export const getPhoenixProgramAddress = (
  input: ResolvePhoenixInstructionAddressesInput = {}
): PhoenixProgramAddress =>
  resolvePhoenixInstructionAddresses(input).programAddress;

export const getPhoenixInstructionAddresses = (
  input: ResolvePhoenixInstructionAddressesInput = {}
): PhoenixInstructionAddresses => resolvePhoenixInstructionAddresses(input);

export const phoenixInstructionAddresses = (
  addresses: PhoenixInstructionAddressSource
): PhoenixInstructionAddresses => ({
  programAddress: addresses.phoenixProgramAddress,
  logAuthorityAddress: addresses.logAuthorityAddress,
  globalConfigurationAddress: addresses.globalConfigurationAddress,
});

export const clientPhoenixInstructionAddresses = (
  client: PhoenixInstructionAddressCarrier
): PhoenixInstructionAddresses => phoenixInstructionAddresses(client.addresses);

// Keep this aligned with the API default used for market-order slippage guards.
export const DEFAULT_MARKET_ORDER_SLIPPAGE = 0.02;
