import { sha256 } from "@noble/hashes/sha2.js";
import { AccountRole, address } from "@solana/kit";
import type { AccountMeta, Address } from "@solana/kit";
import { decodeBase64ToBytes } from "../../src/base64";
import type { InstructionsWithAccountsAndData } from "../../src/primitives";

export const DEFAULT_SDK_LOCALNET_KEYPAIR_BASE_SEED = "rise-sdk-localnet-v1";

export type DecodedFixtureInstruction = InstructionsWithAccountsAndData & {
  programAddress: Address;
};

export interface SdkLocalnetFixture {
  schemaVersion: number;
  name: string;
  keypairBaseSeed: string;
  keypairDerivation: string;
  programs: ProgramAddresses;
  addresses: FixtureAddresses;
  mints: FixtureMint[];
  signers: FixtureSigner[];
  markets: FixtureMarket[];
  actors: FixtureActor[];
  setupTransactions: FixtureTransaction[];
  actionTemplates: FixtureTransaction[];
}

export interface ProgramAddresses {
  phoenixEternal: string;
  ember: string;
  splToken: string;
  associatedToken: string;
  system: string;
}

export interface FixtureAddresses {
  globalConfig: string;
  logAuthority: string;
  perpAssetMap: string;
  withdrawQueue: string;
  globalVault: string;
  globalTraderIndex: string[];
  activeTraderBuffer: string[];
  emberState: string;
  emberVault: string;
}

export interface FixtureMint {
  name: string;
  seed: string;
  pubkey: string;
  decimals: number;
  role: string;
}

export interface FixtureSigner {
  name: string;
  role: string;
  seed: string;
  pubkey: string;
  initialLamports: number;
}

export interface FixtureMarket {
  symbol: string;
  orderbook: string;
  spline: string;
  signerSeed: string;
  baseLotDecimals: number;
  tickSizeInQuoteLotsPerBaseLot: number;
  initialMarkPriceAtoms: number;
  initialSpotPriceAtoms: number;
  liquidity: FixtureLiquidity;
}

export interface FixtureLiquidity {
  bids: FixtureLiquidityLevel[];
  asks: FixtureLiquidityLevel[];
}

export interface FixtureLiquidityLevel {
  priceUsd: string;
  baseQuantity: string;
}

export interface FixtureActor {
  name: string;
  signer: string;
  seed: string;
  pubkey: string;
  traderAccount: string;
  fakeUsdcTokenAccount: string;
  phoenixTokenAccount: string;
  subaccountIndex: number;
  initialUsdcAtoms: number;
  phoenixDepositAtoms: number;
  capabilities: string[];
}

export interface FixtureTransaction {
  name: string;
  description: string;
  signerSeeds: string[];
  instructions: SerializedInstruction[];
}

export interface SerializedInstruction {
  name: string;
  programId: string;
  accounts: SerializedAccountMeta[];
  dataBase64: string;
}

export interface SerializedAccountMeta {
  pubkey: string;
  isSigner: boolean;
  isWritable: boolean;
}

export interface SdkLocalnetMockActions {
  oracle: {
    setPrices: FixtureTransaction;
  };
  orderbookTrader: {
    placeLevels: FixtureTransaction;
    cancelLevels: FixtureTransaction;
  };
  splineTrader: {
    updatePrices: FixtureTransaction;
  };
  collateral: {
    forActor: (actorName: string) => FixtureTransaction;
  };
}

export const sdkLocalnetSeedBytes = (
  seed: string,
  fixture: Pick<SdkLocalnetFixture, "keypairBaseSeed"> = {
    keypairBaseSeed: DEFAULT_SDK_LOCALNET_KEYPAIR_BASE_SEED,
  }
): Uint8Array =>
  sha256(new TextEncoder().encode(`${fixture.keypairBaseSeed}-${seed}`));

export const decodeFixtureInstruction = (
  instruction: SerializedInstruction
): DecodedFixtureInstruction => ({
  programAddress: address(instruction.programId),
  accounts: instruction.accounts.map(decodeFixtureAccountMeta),
  data: decodeBase64ToBytes(instruction.dataBase64),
});

export const decodeFixtureTransaction = (
  transaction: FixtureTransaction
): InstructionsWithAccountsAndData[] =>
  transaction.instructions.map(decodeFixtureInstruction);

export const getSdkLocalnetTransaction = (
  fixture: SdkLocalnetFixture,
  name: string
): FixtureTransaction => {
  const transaction = [
    ...fixture.setupTransactions,
    ...fixture.actionTemplates,
  ].find((candidate) => candidate.name === name);

  if (!transaction) {
    throw new Error(`Unknown SDK localnet fixture transaction: ${name}`);
  }

  return transaction;
};

export const getSdkLocalnetActionTemplate = (
  fixture: SdkLocalnetFixture,
  name: string
): FixtureTransaction => {
  const transaction = fixture.actionTemplates.find(
    (candidate) => candidate.name === name
  );

  if (!transaction) {
    throw new Error(`Unknown SDK localnet action template: ${name}`);
  }

  return transaction;
};

export const createSdkLocalnetMockActions = (
  fixture: SdkLocalnetFixture
): SdkLocalnetMockActions => ({
  oracle: {
    setPrices: getSdkLocalnetActionTemplate(fixture, "oracleSetPrices"),
  },
  orderbookTrader: {
    placeLevels: getSdkLocalnetActionTemplate(fixture, "orderbookPlaceLevels"),
    cancelLevels: getSdkLocalnetActionTemplate(
      fixture,
      "orderbookCancelLevels"
    ),
  },
  splineTrader: {
    updatePrices: getSdkLocalnetActionTemplate(fixture, "splineUpdatePrices"),
  },
  collateral: {
    forActor: (actorName: string) =>
      getSdkLocalnetTransaction(
        fixture,
        `seed${upperFirst(actorName)}Collateral`
      ),
  },
});

const decodeFixtureAccountMeta = (
  account: SerializedAccountMeta
): AccountMeta => ({
  address: address(account.pubkey),
  role: accountRoleFromFixtureMeta(account),
});

const accountRoleFromFixtureMeta = (
  account: SerializedAccountMeta
): AccountRole => {
  if (account.isSigner) {
    return account.isWritable
      ? AccountRole.WRITABLE_SIGNER
      : AccountRole.READONLY_SIGNER;
  }

  return account.isWritable ? AccountRole.WRITABLE : AccountRole.READONLY;
};

const upperFirst = (value: string): string => {
  if (value.length === 0) return value;
  return `${value[0]?.toUpperCase()}${value.slice(1)}`;
};
