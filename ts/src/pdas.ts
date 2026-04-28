import {
  EMBER_PROGRAM_ADDRESS,
  SPL_ATA_PROGRAM_ADDRESS,
  SPL_TOKEN_PROGRAM_ADDRESS,
  getPhoenixProgramAddress,
} from "@/core/constants";
import type {
  Authority,
  EmberStateAddress,
  EmberVaultAddress,
  GlobalConfigurationAddress,
  GlobalVaultAddress,
  LogAuthorityAddress,
  MarketAddress,
  MintAddress,
  PhoenixProgramAddress,
  SplineCollectionAddress,
  TokenAccountAddress,
  TraderAddress,
} from "@/primitives";
import {
  getBase58Encoder,
  getProgramDerivedAddress,
  type Address,
} from "@solana/kit";

const toLittleEndianU64Bytes = (value: bigint): Uint8Array => {
  const bytes = new Uint8Array(8);
  new DataView(bytes.buffer).setBigUint64(0, value, true);
  return bytes;
};

export interface TraderSubaccountAddressParams {
  authority: Authority;
  traderPdaIndex: number;
  subaccountIndex: number;
  phoenixProgramAddress?: PhoenixProgramAddress;
}

export interface StopLossAddressParams {
  traderAccount: TraderAddress;
  assetId: bigint;
  phoenixProgramAddress?: PhoenixProgramAddress;
}

export interface ConditionalOrdersAddressParams {
  traderAccount: TraderAddress;
  phoenixProgramAddress?: PhoenixProgramAddress;
}

export const getPhoenixLogAuthorityAddress = async (
  phoenixProgramAddress: PhoenixProgramAddress = getPhoenixProgramAddress()
): Promise<LogAuthorityAddress> => {
  const [pda] = await getProgramDerivedAddress({
    programAddress: phoenixProgramAddress,
    seeds: ["log"],
  });
  return pda as LogAuthorityAddress;
};

export const getPhoenixGlobalConfigurationAddress = async (
  phoenixProgramAddress: PhoenixProgramAddress = getPhoenixProgramAddress()
): Promise<GlobalConfigurationAddress> => {
  const [pda] = await getProgramDerivedAddress({
    programAddress: phoenixProgramAddress,
    seeds: ["global"],
  });
  return pda as GlobalConfigurationAddress;
};

export const getPhoenixSplineCollectionAddress = async (
  marketAccountAddress: MarketAddress,
  phoenixProgramAddress: PhoenixProgramAddress = getPhoenixProgramAddress()
): Promise<SplineCollectionAddress> => {
  const [pda] = await getProgramDerivedAddress({
    programAddress: phoenixProgramAddress,
    seeds: ["spline", getBase58Encoder().encode(marketAccountAddress)],
  });
  return pda as SplineCollectionAddress;
};

export const getPhoenixTraderSubaccountAddress = async (
  params: TraderSubaccountAddressParams
): Promise<TraderAddress> => {
  const [pda] = await getProgramDerivedAddress({
    programAddress: params.phoenixProgramAddress ?? getPhoenixProgramAddress(),
    seeds: [
      "trader",
      getBase58Encoder().encode(params.authority),
      new Uint8Array([params.traderPdaIndex]),
      new Uint8Array([params.subaccountIndex]),
    ],
  });

  return pda as TraderAddress;
};

export const getPhoenixTraderTokenAccountAddress = async (
  authority: Authority,
  mint: MintAddress
): Promise<TokenAccountAddress> => {
  const [pda] = await getProgramDerivedAddress({
    programAddress: SPL_ATA_PROGRAM_ADDRESS,
    seeds: [
      getBase58Encoder().encode(authority),
      getBase58Encoder().encode(SPL_TOKEN_PROGRAM_ADDRESS),
      getBase58Encoder().encode(mint),
    ],
  });

  return pda as TokenAccountAddress;
};

export const getPhoenixGlobalVaultAddress = async (
  mint: MintAddress,
  phoenixProgramAddress: PhoenixProgramAddress = getPhoenixProgramAddress()
): Promise<GlobalVaultAddress> => {
  const [pda] = await getProgramDerivedAddress({
    programAddress: phoenixProgramAddress,
    seeds: ["vault", getBase58Encoder().encode(mint)],
  });

  return pda as GlobalVaultAddress;
};

export const getEmberStateAddress = async (
  phoenixProgramAddress: PhoenixProgramAddress = getPhoenixProgramAddress()
): Promise<EmberStateAddress> => {
  const [pda] = await getProgramDerivedAddress({
    programAddress: EMBER_PROGRAM_ADDRESS,
    seeds: [getBase58Encoder().encode(phoenixProgramAddress), "state"],
  });

  return pda as EmberStateAddress;
};

export const getEmberVaultAddress = async (
  phoenixProgramAddress: PhoenixProgramAddress = getPhoenixProgramAddress()
): Promise<EmberVaultAddress> => {
  const [pda] = await getProgramDerivedAddress({
    programAddress: EMBER_PROGRAM_ADDRESS,
    seeds: [getBase58Encoder().encode(phoenixProgramAddress), "vault"],
  });

  return pda as EmberVaultAddress;
};

export const getPhoenixPermissionAddress = async (
  permissionAuthority: Address,
  delegatedKey: Address,
  phoenixProgramAddress: PhoenixProgramAddress = getPhoenixProgramAddress()
): Promise<Address> => {
  const [pda] = await getProgramDerivedAddress({
    programAddress: phoenixProgramAddress,
    seeds: [
      "permission",
      getBase58Encoder().encode(permissionAuthority),
      getBase58Encoder().encode(delegatedKey),
    ],
  });

  return pda;
};

export const getPhoenixEscrowAddress = async (
  authority: Address,
  phoenixProgramAddress: PhoenixProgramAddress = getPhoenixProgramAddress()
): Promise<Address> => {
  const [pda] = await getProgramDerivedAddress({
    programAddress: phoenixProgramAddress,
    seeds: ["escrow", getBase58Encoder().encode(authority)],
  });

  return pda;
};

export const getPhoenixStopLossAddress = async (
  params: StopLossAddressParams
): Promise<Address> => {
  const [pda] = await getProgramDerivedAddress({
    programAddress: params.phoenixProgramAddress ?? getPhoenixProgramAddress(),
    seeds: [
      "stoploss",
      getBase58Encoder().encode(params.traderAccount),
      toLittleEndianU64Bytes(params.assetId),
    ],
  });

  return pda;
};

export const getPhoenixConditionalOrdersAddress = async (
  params: ConditionalOrdersAddressParams
): Promise<Address> => {
  const [pda] = await getProgramDerivedAddress({
    programAddress: params.phoenixProgramAddress ?? getPhoenixProgramAddress(),
    seeds: [
      "conditional_orders",
      getBase58Encoder().encode(params.traderAccount),
    ],
  });

  return pda;
};
