import {
  EMBER_PROGRAM_ADDRESS,
  FLICKER_PROGRAM_ADDRESS,
  SPL_ATA_PROGRAM_ADDRESS,
  SPL_TOKEN_PROGRAM_ADDRESS,
  getPhoenixProgramAddress,
} from "@/core/constants";
import type {
  Authority,
  EmberStateAddress,
  EmberVaultAddress,
  FlickerProgramAddress,
  GlobalConfigurationAddress,
  GlobalVaultAddress,
  LogAuthorityAddress,
  MarketAddress,
  MintAddress,
  NativeSolAuthorityAddress,
  PhoenixProgramAddress,
  SplineCollectionAddress,
  TokenAccountAddress,
  TraderAddress,
  TwapAccountAddress,
  TwapGlobalStateAddress,
  TwapLogAuthorityAddress,
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

const toLittleEndianU32Bytes = (value: number): Uint8Array => {
  const bytes = new Uint8Array(4);
  new DataView(bytes.buffer).setUint32(0, value, true);
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

export interface TwapAddressParams {
  phoenixProgramAddress?: PhoenixProgramAddress;
  flickerProgramAddress?: FlickerProgramAddress;
}

export interface TwapAccountAddressParams extends TwapAddressParams {
  traderAuthority: Authority;
  traderPdaIndex: number;
  traderSubaccountIndex: number;
  marketId: number;
}

export interface TwapDelegatePermissionAddressParams extends TwapAddressParams {
  traderAuthority: Authority;
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

/**
 * Derives the native SOL authority PDA, which custodies native SOL spot
 * collateral and signs the program's own withdrawals of it.
 *
 * Seeds: `["native_sol"]`.
 */
export const getPhoenixNativeSolAuthorityAddress = async (
  phoenixProgramAddress: PhoenixProgramAddress = getPhoenixProgramAddress()
): Promise<NativeSolAuthorityAddress> => {
  const [pda] = await getProgramDerivedAddress({
    programAddress: phoenixProgramAddress,
    seeds: ["native_sol"],
  });

  return pda as NativeSolAuthorityAddress;
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

export const getTwapGlobalStateAddress = async (
  params: TwapAddressParams = {}
): Promise<TwapGlobalStateAddress> => {
  const [pda] = await getProgramDerivedAddress({
    programAddress: params.flickerProgramAddress ?? FLICKER_PROGRAM_ADDRESS,
    seeds: [
      getBase58Encoder().encode(
        params.phoenixProgramAddress ?? getPhoenixProgramAddress()
      ),
      "global_state",
    ],
  });

  return pda as TwapGlobalStateAddress;
};

/**
 * Derives the Eternal permission PDA that enrolls a trader in Flicker TWAP
 * execution. The delegated key is the Flicker global state PDA — the single
 * program-wide delegate that signs Phoenix CPIs for every TWAP account — so
 * one enrollment covers all of the trader's TWAP orders on every market.
 */
export const getTwapDelegatePermissionAddress = async (
  params: TwapDelegatePermissionAddressParams
): Promise<Address> => {
  const twapGlobalState = await getTwapGlobalStateAddress(params);
  return getPhoenixPermissionAddress(
    params.traderAuthority,
    twapGlobalState,
    params.phoenixProgramAddress ?? getPhoenixProgramAddress()
  );
};

export const getTwapLogAuthorityAddress = async (
  params: Pick<TwapAddressParams, "flickerProgramAddress"> = {}
): Promise<TwapLogAuthorityAddress> => {
  const [pda] = await getProgramDerivedAddress({
    programAddress: params.flickerProgramAddress ?? FLICKER_PROGRAM_ADDRESS,
    seeds: ["log"],
  });

  return pda as TwapLogAuthorityAddress;
};

export const getTwapAccountAddress = async (
  params: TwapAccountAddressParams
): Promise<TwapAccountAddress> => {
  const [pda] = await getProgramDerivedAddress({
    programAddress: params.flickerProgramAddress ?? FLICKER_PROGRAM_ADDRESS,
    seeds: [
      "flicker",
      getBase58Encoder().encode(
        params.phoenixProgramAddress ?? getPhoenixProgramAddress()
      ),
      getBase58Encoder().encode(params.traderAuthority),
      new Uint8Array([params.traderPdaIndex, params.traderSubaccountIndex]),
      toLittleEndianU32Bytes(params.marketId),
    ],
  });

  return pda as TwapAccountAddress;
};
