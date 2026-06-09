import { getPhoenixProgramAddress } from "@/core/constants";
import { getPhoenixTraderTokenAccountAddress } from "@/pdas";
import type {
  Authority,
  FlameProgramAddress,
  MintAddress,
  PhoenixProgramAddress,
  TokenAccountAddress,
} from "@/primitives";
import {
  address,
  getBase58Encoder,
  getProgramDerivedAddress,
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
