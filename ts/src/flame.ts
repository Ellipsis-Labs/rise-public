import {
  EMBER_PROGRAM_ADDRESS,
  SPL_ATA_PROGRAM_ADDRESS,
  SPL_TOKEN_PROGRAM_ADDRESS,
  SYSTEM_PROGRAM_ADDRESS,
  getPhoenixInstructionAddresses,
  getPhoenixProgramAddress,
  type ResolvePhoenixInstructionAddressesInput,
} from "@/core/constants";
import { sha2_const } from "@/core/discriminants";
import {
  generateReadonlyAccount,
  generateWritableAccount,
  generateWritableSignerAccount,
  generateArenaAccounts,
} from "@/core/utils/accountMeta";
import {
  getEmberStateAddress,
  getEmberVaultAddress,
  getPhoenixGlobalVaultAddress,
  getPhoenixPermissionAddress,
  getPhoenixTraderSubaccountAddress,
  getPhoenixTraderTokenAccountAddress,
} from "@/pdas";
import type {
  ActiveTraderBufferAddressArray,
  Authority,
  EmberStateAddress,
  EmberVaultAddress,
  FlameProgramAddress,
  GlobalTraderIndexAddressArray,
  GlobalVaultAddress,
  MintAddress,
  PhoenixProgramAddress,
  TokenAccountAddress,
  TraderAddress,
} from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import {
  address,
  getBase58Encoder,
  getProgramDerivedAddress,
  type AccountMeta,
  type Address,
} from "@solana/kit";

export const FLAME_PROGRAM_ADDRESS = address(
  "FLameGi7e6q6oDf4osFqQFtFJureWKKpnMFU4JmiRDvF"
) as FlameProgramAddress;

export interface ResolveFlameAddressesInput {
  phoenixProgramAddress?: PhoenixProgramAddress;
  flameProgramAddress?: FlameProgramAddress;
}

export interface FlameProxyAuthorityAddressInput extends ResolveFlameAddressesInput {
  userAuthority: Authority;
  traderPdaIndex: number;
}

export interface FlameDepositAddressInput extends FlameProxyAuthorityAddressInput {
  mintAddress: MintAddress;
}

export interface FlameDepositAddresses {
  proxyAuthority: Authority;
  depositAddress: TokenAccountAddress;
  proxyAta: TokenAccountAddress;
}

export interface FlameDepositToPhoenixParams
  extends
    ResolveFlameAddressesInput,
    Pick<
      ResolvePhoenixInstructionAddressesInput,
      "logAuthorityAddress" | "globalConfigurationAddress"
    > {
  crank: Authority;
  userAuthority: Authority;
  inputMint: MintAddress;
  outputMint: MintAddress;
  globalTraderIndex: GlobalTraderIndexAddressArray;
  activeTraderBuffer: ActiveTraderBufferAddressArray;
  traderPdaIndex: number;
  traderSubaccountIndex?: number;
}

export type FlameDepositToPhoenixIx = InstructionsWithAccountsAndData;

export interface FlameDepositToPhoenixAddresses {
  globalState: Address;
  proxyAuthority: Authority;
  proxyTokenAccount: TokenAccountAddress;
  feeVaultTokenAccount: TokenAccountAddress;
  emberState: EmberStateAddress;
  emberVault: EmberVaultAddress;
  proxyPhoenixTokenAccount: TokenAccountAddress;
  phoenixTraderAccount: TraderAddress;
  phoenixGlobalVault: GlobalVaultAddress;
  phoenixPermissionAccount: Address;
}

const assertU8 = (value: number, fieldName: string): void => {
  if (!Number.isInteger(value) || value < 0 || value > 255) {
    throw new Error(`${fieldName} must be between 0 and 255`);
  }
};

const resolveFlameAddresses = (
  input: ResolveFlameAddressesInput = {}
): Required<ResolveFlameAddressesInput> => ({
  flameProgramAddress: input.flameProgramAddress ?? FLAME_PROGRAM_ADDRESS,
  phoenixProgramAddress:
    input.phoenixProgramAddress ?? getPhoenixProgramAddress(),
});

export const deriveFlameProxyAuthorityAddress = async ({
  userAuthority,
  traderPdaIndex,
  ...input
}: FlameProxyAuthorityAddressInput): Promise<Authority> => {
  assertU8(traderPdaIndex, "Trader PDA index");
  const { flameProgramAddress, phoenixProgramAddress } =
    resolveFlameAddresses(input);
  const base58Encoder = getBase58Encoder();

  const [proxyAuthority] = await getProgramDerivedAddress({
    programAddress: flameProgramAddress,
    seeds: [
      "proxy",
      base58Encoder.encode(phoenixProgramAddress),
      base58Encoder.encode(userAuthority),
      new Uint8Array([traderPdaIndex]),
    ],
  });

  return proxyAuthority as Authority;
};

export const deriveFlameDepositAddress = async (
  input: FlameDepositAddressInput
): Promise<TokenAccountAddress> => {
  const { depositAddress } = await deriveFlameDepositAddresses(input);
  return depositAddress;
};

export const deriveFlameDepositAddresses = async ({
  mintAddress,
  ...input
}: FlameDepositAddressInput): Promise<FlameDepositAddresses> => {
  const proxyAuthority = await deriveFlameProxyAuthorityAddress(input);
  const proxyAta = await getPhoenixTraderTokenAccountAddress(
    proxyAuthority,
    mintAddress
  );

  return {
    proxyAuthority,
    depositAddress: proxyAta,
    proxyAta,
  };
};

export const deriveFlameGlobalStateAddress = async (
  input: ResolveFlameAddressesInput = {}
): Promise<Address> => {
  const { flameProgramAddress, phoenixProgramAddress } =
    resolveFlameAddresses(input);
  const base58Encoder = getBase58Encoder();
  const [globalState] = await getProgramDerivedAddress({
    programAddress: flameProgramAddress,
    seeds: [base58Encoder.encode(phoenixProgramAddress), "global_state"],
  });
  return globalState;
};

export const deriveFlameDepositToPhoenixAddresses = async ({
  userAuthority,
  inputMint,
  outputMint,
  traderPdaIndex,
  traderSubaccountIndex = 0,
  ...input
}: Omit<
  FlameDepositToPhoenixParams,
  "crank" | "globalTraderIndex" | "activeTraderBuffer"
>): Promise<FlameDepositToPhoenixAddresses> => {
  assertU8(traderPdaIndex, "Trader PDA index");
  assertU8(traderSubaccountIndex, "Trader subaccount index");
  const { phoenixProgramAddress } = resolveFlameAddresses(input);
  const proxyAuthority = await deriveFlameProxyAuthorityAddress({
    userAuthority,
    traderPdaIndex,
    ...input,
  });
  const globalState = await deriveFlameGlobalStateAddress(input);

  const [
    proxyTokenAccount,
    feeVaultTokenAccount,
    emberState,
    emberVault,
    proxyPhoenixTokenAccount,
    phoenixTraderAccount,
    phoenixGlobalVault,
    phoenixPermissionAccount,
  ] = await Promise.all([
    getPhoenixTraderTokenAccountAddress(proxyAuthority, inputMint),
    getPhoenixTraderTokenAccountAddress(globalState as Authority, inputMint),
    getEmberStateAddress(phoenixProgramAddress),
    getEmberVaultAddress(phoenixProgramAddress),
    getPhoenixTraderTokenAccountAddress(proxyAuthority, outputMint),
    getPhoenixTraderSubaccountAddress({
      authority: userAuthority,
      traderPdaIndex,
      subaccountIndex: traderSubaccountIndex,
      phoenixProgramAddress,
    }),
    getPhoenixGlobalVaultAddress(outputMint, phoenixProgramAddress),
    getPhoenixPermissionAddress(
      userAuthority,
      proxyAuthority,
      phoenixProgramAddress
    ),
  ]);

  return {
    globalState,
    proxyAuthority,
    proxyTokenAccount,
    feeVaultTokenAccount,
    emberState,
    emberVault,
    proxyPhoenixTokenAccount,
    phoenixTraderAccount,
    phoenixGlobalVault,
    phoenixPermissionAccount,
  };
};

const getFlameDepositToPhoenixData = (
  traderPdaIndex: number,
  traderSubaccountIndex: number
): Uint8Array => {
  assertU8(traderPdaIndex, "Trader PDA index");
  assertU8(traderSubaccountIndex, "Trader subaccount index");
  const data = new Uint8Array(10);
  data.set(sha2_const("global:deposit_to_phoenix"), 0);
  data[8] = traderPdaIndex;
  data[9] = traderSubaccountIndex;
  return data;
};

export const buildFlameDepositToPhoenixIx = async (
  params: FlameDepositToPhoenixParams
): Promise<FlameDepositToPhoenixIx> => {
  const { flameProgramAddress, phoenixProgramAddress } =
    resolveFlameAddresses(params);
  const { logAuthorityAddress, globalConfigurationAddress, programAddress } =
    getPhoenixInstructionAddresses({
      programAddress: phoenixProgramAddress,
      logAuthorityAddress: params.logAuthorityAddress,
      globalConfigurationAddress: params.globalConfigurationAddress,
    });
  const traderSubaccountIndex = params.traderSubaccountIndex ?? 0;
  const addresses = await deriveFlameDepositToPhoenixAddresses({
    userAuthority: params.userAuthority,
    inputMint: params.inputMint,
    outputMint: params.outputMint,
    traderPdaIndex: params.traderPdaIndex,
    traderSubaccountIndex,
    phoenixProgramAddress: programAddress,
    flameProgramAddress,
  });

  const accounts: AccountMeta[] = [
    generateReadonlyAccount(addresses.globalState),
    generateReadonlyAccount(programAddress),
    generateWritableSignerAccount(params.crank),
    generateReadonlyAccount(params.userAuthority),
    generateReadonlyAccount(addresses.proxyAuthority),
    generateWritableAccount(addresses.proxyTokenAccount),
    generateWritableAccount(addresses.feeVaultTokenAccount),
    generateReadonlyAccount(SPL_TOKEN_PROGRAM_ADDRESS),
    generateReadonlyAccount(SYSTEM_PROGRAM_ADDRESS),
    generateReadonlyAccount(SPL_ATA_PROGRAM_ADDRESS),
    generateReadonlyAccount(EMBER_PROGRAM_ADDRESS),
    generateReadonlyAccount(addresses.emberState),
    generateReadonlyAccount(params.inputMint),
    generateWritableAccount(params.outputMint),
    generateWritableAccount(addresses.proxyPhoenixTokenAccount),
    generateWritableAccount(addresses.emberVault),
    generateReadonlyAccount(logAuthorityAddress),
    generateWritableAccount(globalConfigurationAddress),
    generateWritableAccount(addresses.phoenixTraderAccount),
    generateWritableAccount(addresses.phoenixGlobalVault),
    generateWritableAccount(addresses.phoenixPermissionAccount),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
  ];

  return {
    programAddress: flameProgramAddress,
    accounts,
    data: getFlameDepositToPhoenixData(
      params.traderPdaIndex,
      traderSubaccountIndex
    ),
  };
};
