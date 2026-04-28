import type { Address } from "@solana/kit";
import type { Branded } from "@/core/utils/branded";

export type PhoenixProgramAddress = Branded<Address, "PhoenixProgram">;
export type LogAuthorityAddress = Branded<Address, "LogAuthority">;
export type GlobalConfigurationAddress = Branded<Address, "GlobalConfig">;
export type MintAddress = Branded<Address, "Mint">;
export type SPLTokenProgramAddress = Branded<Address, "SPLTokenProgram">;
export type SPLATAProgramAddress = Branded<Address, "SPLATAProgram">;
export type SystemProgramAddress = Branded<Address, "SystemProgram">;
export type EmberProgramAddress = Branded<Address, "EmberProgram">;

export type Authority = Branded<Address, "Authority">;
export type PerpAssetMapAddress = Branded<Address, "PerpAssetMap">;
export type TraderAddress = Branded<Address, "Trader">;
export type MarketAddress = Branded<Address, "Market">;
export type SplineCollectionAddress = Branded<Address, "SplineCollection">;

export type ActiveTraderBufferHeaderAddress = Branded<
  Address,
  "ActiveTraderBufferHeader"
>;
export type ActiveTraderBufferArenaAddress = Branded<
  Address,
  "ActiveTraderBufferArena"
>;

export type ActiveTraderBufferAddressArray = [
  ActiveTraderBufferHeaderAddress,
  ...ActiveTraderBufferArenaAddress[],
];

export type GlobalTraderIndexHeaderAddress = Branded<
  Address,
  "GlobalTraderIndexHeader"
>;
export type GlobalTraderIndexArenaAddress = Branded<
  Address,
  "GlobalTraderIndexArena"
>;

export type GlobalTraderIndexAddressArray = [
  GlobalTraderIndexHeaderAddress,
  ...GlobalTraderIndexArenaAddress[],
];

export type TokenAccountAddress = Branded<Address, "TokenAccount">;
export type GlobalVaultAddress = Branded<Address, "GlobalVault">;
export type WithdrawQueueAddress = Branded<Address, "WithdrawQueue">;
export type EmberStateAddress = Branded<Address, "EmberState">;
export type EmberVaultAddress = Branded<Address, "EmberVault">;
