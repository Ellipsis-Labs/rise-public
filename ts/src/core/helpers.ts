import { AccountFetchers, fetchPerpAssetMap, fetchTrader } from "@/accounts";
import type { ExchangeInstructionContext } from "@/exchange-cache/types";
import {
  getSequenceNumberDecoder,
  type SequenceNumber,
} from "@/accounts/internal";
import {
  DEFAULT_MARKET_ORDER_SLIPPAGE,
  SPL_ATA_PROGRAM_ADDRESS,
  SPL_TOKEN_PROGRAM_ADDRESS,
  SYSTEM_PROGRAM_ADDRESS,
} from "@/core/constants";
import type { PhoenixInstructionClient } from "@/core/clientTypes";
import { ACCOUNT_DISCRIMINANTS } from "@/core/discriminants";
import { getFixedLengthArrayCodec } from "@/primitives/_utilityTypes/FixedLengthArray";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import {
  type ActiveTraderBufferAddressArray,
  type ActiveTraderBufferArenaAddress,
  type Authority,
  type GlobalConfigurationAddress,
  type GlobalVaultAddress,
  type EmberStateAddress,
  type GlobalTraderIndexAddressArray,
  type GlobalTraderIndexArenaAddress,
  type MarketAddress,
  type MintAddress,
  type PerpAssetMapAddress,
  type PhoenixProgramAddress,
  type QuoteLots,
  type SPLTokenProgramAddress,
  type SplineCollectionAddress,
  type TokenAccountAddress,
  type TraderAddress,
  type WithdrawQueueAddress,
} from "@/primitives";
import {
  MarginType,
  quoteLots,
  type Symbol,
  toMaxPositions,
} from "@/primitives";
import {
  getPhoenixTraderSubaccountAddress,
  getPhoenixTraderTokenAccountAddress,
} from "@/pdas";
import {
  generateReadonlyAccount,
  generateReadonlySignerAccount,
  generateWritableAccount,
  generateWritableSignerAccount,
} from "@/core/utils/accountMeta";
import {
  getConstantDecoder,
  getHiddenPrefixDecoder,
  getProgramDerivedAddress,
  getStructDecoder,
  getU16Decoder,
  getU32Codec,
  getU32Decoder,
  getU8Codec,
  transformDecoder,
  type Address,
  type Decoder,
} from "@solana/kit";

export interface RequiredAccounts {
  globalConfiguration: Awaited<
    ReturnType<typeof AccountFetchers.GlobalConfiguration>
  >;
  arenaAddresses: ActiveTraderBufferAddressArray;
  globalTraderIndexAddresses: GlobalTraderIndexAddressArray;
}

export interface MarketMetadataForSymbol {
  marketAddress: MarketAddress;
  assetId: bigint;
  tickSize?: number;
  baseLotDecimals?: number;
  splineAddress?: SplineCollectionAddress;
}

export interface SubaccountInfo {
  index: number;
  authority: Authority;
  address: TraderAddress;
  maxPositions: bigint;
}

interface Superblock {
  size: number;
  numArenas: number;
  numActiveArenas: number;
  numNodesPerArena: number;
  bumpIndex: number;
  freeListHead: number;
}

interface ArenaHeader {
  sequenceNumber: SequenceNumber;
  numAdditionalNodes: number;
  superblock: Superblock;
  root: number;
}

export const MAX_SUBACCOUNTS = 100;

type ExchangeLookupClient = Pick<PhoenixInstructionClient, "fetchAccount"> & {
  addresses: Pick<
    PhoenixInstructionClient["addresses"],
    "globalConfigurationAddress" | "phoenixProgramAddress"
  >;
};

const brandAddress = <T extends Address>(value: string): T => value as T;

const toQuoteLots = (value: bigint): QuoteLots => quoteLots(value);

const toAuthority = (value: string): Authority =>
  brandAddress<Authority>(value);

const toAuthoritySet = (value: {
  rootAuthority: string;
  riskAuthority: string;
  marketAuthority: string;
  oracleAuthority: string;
  adlAuthority: string;
  cancelAuthority: string;
  backstopAuthority: string;
}) => ({
  rootAuthority: toAuthority(value.rootAuthority),
  riskAuthority: toAuthority(value.riskAuthority),
  marketAuthority: toAuthority(value.marketAuthority),
  oracleAuthority: toAuthority(value.oracleAuthority),
  reservedAuthority: toAuthority(value.rootAuthority),
  adlAuthority: toAuthority(value.adlAuthority),
  cancelAuthority: toAuthority(value.cancelAuthority),
  backstopAuthority: toAuthority(value.backstopAuthority),
});

const getSuperblockDecoder = (): Decoder<Superblock> =>
  transformDecoder(
    getStructDecoder([
      ["size", getU32Decoder()],
      ["numArenas", getU16Decoder()],
      ["numActiveArenas", getU16Decoder()],
      ["numNodesPerArena", getU32Decoder()],
      ["bumpIndex", getU32Decoder()],
      ["freeListHead", getU32Decoder()],
      ["_padding", getFixedLengthArrayCodec(getU32Codec(), 3)],
    ]),
    ({ _padding, ...superblock }) => superblock
  );

const getArenaHeaderDecoder = (
  discriminant: Uint8Array
): Decoder<ArenaHeader> =>
  transformDecoder(
    getHiddenPrefixDecoder(
      getStructDecoder([
        ["sequenceNumber", getSequenceNumberDecoder()],
        ["numAdditionalNodes", getU32Decoder()],
        ["_padding0", getFixedLengthArrayCodec(getU8Codec(), 4)],
        ["_padding1", getFixedLengthArrayCodec(getU8Codec(), 16)],
        ["superblock", getSuperblockDecoder()],
        ["root", getU32Decoder()],
        ["_padding2", getFixedLengthArrayCodec(getU32Codec(), 3)],
      ]),
      [getConstantDecoder(discriminant)]
    ),
    ({ _padding0, _padding1, _padding2, ...header }) => header
  );

const getArenaIndices = (superblock: Superblock) => {
  const numArenas = superblock.numArenas >>> 0;
  const numActiveArenas = superblock.numActiveArenas >>> 0;
  const arenaIndices: number[] = [];
  const arenasToFetch = Math.max(0, Math.min(numArenas, numActiveArenas) - 1);
  for (let i = 1; i <= arenasToFetch; i++) {
    arenaIndices.push(i);
  }
  return arenaIndices;
};

export const getArenaAddresses = async <
  T extends ActiveTraderBufferArenaAddress[] | GlobalTraderIndexArenaAddress[] =
    | ActiveTraderBufferArenaAddress[]
    | GlobalTraderIndexArenaAddress[],
>(
  arenaIndices: number[],
  pdaSeed: string,
  programAddress: PhoenixProgramAddress
): Promise<T> => {
  const arenaAddresses = await Promise.all(
    arenaIndices.map(async (index) => {
      const [pda] = await getProgramDerivedAddress({
        programAddress,
        seeds: [pdaSeed, new Uint8Array([index])],
      });
      return pda;
    })
  );

  return arenaAddresses as T;
};

const decodeArenaHeader = async (
  client: Pick<PhoenixInstructionClient, "fetchAccount">,
  address: Address,
  discriminant: Uint8Array
): Promise<ArenaHeader> => {
  const account = await client.fetchAccount(address);
  return getArenaHeaderDecoder(discriminant).decode(account.data);
};

export const getActiveTraderBufferAddresses = async (
  client: ExchangeLookupClient
): Promise<ActiveTraderBufferAddressArray> => {
  const globalConfiguration = await AccountFetchers.GlobalConfiguration({
    client,
    address: client.addresses.globalConfigurationAddress,
  });

  const headerAddress = globalConfiguration.activeTraderBufferHeaderKey;
  const header = await decodeArenaHeader(
    client,
    headerAddress,
    ACCOUNT_DISCRIMINANTS.ACTIVE_TRADER_BUFFER_HEADER
  );
  const arenaAddresses = await getArenaAddresses<
    ActiveTraderBufferArenaAddress[]
  >(
    getArenaIndices(header.superblock),
    "active_trader_buffer",
    client.addresses.phoenixProgramAddress
  );

  return [headerAddress, ...arenaAddresses];
};

export const getGlobalTraderIndexAddresses = async (
  client: ExchangeLookupClient
): Promise<GlobalTraderIndexAddressArray> => {
  const globalConfiguration = await AccountFetchers.GlobalConfiguration({
    client,
    address: client.addresses.globalConfigurationAddress,
  });

  const headerAddress = globalConfiguration.globalTraderIndexHeaderKey;
  const header = await decodeArenaHeader(
    client,
    headerAddress,
    ACCOUNT_DISCRIMINANTS.GLOBAL_TRADER_INDEX_HEADER
  );
  const arenaAddresses = await getArenaAddresses<
    GlobalTraderIndexArenaAddress[]
  >(
    getArenaIndices(header.superblock),
    "global_trader_index",
    client.addresses.phoenixProgramAddress
  );

  return [headerAddress, ...arenaAddresses];
};

export const fetchRequiredAccounts = async (
  client: PhoenixInstructionClient
): Promise<RequiredAccounts> => {
  if (client.exchange) {
    const snapshot = await client.exchange.ready();
    const globalTraderIndexHeader = snapshot.exchange.globalTraderIndex[0];
    const activeTraderBufferHeader = snapshot.exchange.activeTraderBuffer[0];
    if (!globalTraderIndexHeader || !activeTraderBufferHeader) {
      throw new Error(
        "Exchange metadata snapshot is missing global trader index or active trader buffer header addresses"
      );
    }
    return {
      globalConfiguration: {
        accountKey: brandAddress<GlobalConfigurationAddress>(
          snapshot.exchange.globalConfig
        ),
        currentAuthorities: toAuthoritySet(
          snapshot.exchange.currentAuthorities
        ),
        canonicalTokenMintKey: brandAddress<MintAddress>(
          snapshot.exchange.canonicalMint
        ),
        globalVaultKey: brandAddress<GlobalVaultAddress>(
          snapshot.exchange.globalVault
        ),
        perpAssetMapKey: brandAddress<PerpAssetMapAddress>(
          snapshot.exchange.perpAssetMap
        ),
        globalTraderIndexHeaderKey: globalTraderIndexHeader as never,
        activeTraderBufferHeaderKey: activeTraderBufferHeader as never,
        totalQuoteLotFees: toQuoteLots(0n),
        unclaimedQuoteLotFees: toQuoteLots(0n),
        withdrawQueueKey: brandAddress<WithdrawQueueAddress>(
          snapshot.exchange.withdrawQueue
        ),
        exchangeStatus: snapshot.exchange.exchangeStatusBits,
        quoteDecimals: 6,
        withdrawalMarginFactorBps: 0,
        depositCooldownPeriodInSlots: 0n,
        pendingAuthorities: toAuthoritySet(
          snapshot.exchange.currentAuthorities
        ),
      },
      arenaAddresses: snapshot.exchange
        .activeTraderBuffer as ActiveTraderBufferAddressArray,
      globalTraderIndexAddresses: snapshot.exchange
        .globalTraderIndex as GlobalTraderIndexAddressArray,
    };
  }

  const [globalConfiguration, arenaAddresses, globalTraderIndexAddresses] =
    await Promise.all([
      AccountFetchers.GlobalConfiguration({
        client,
        address: client.addresses.globalConfigurationAddress,
      }),
      getActiveTraderBufferAddresses(client),
      getGlobalTraderIndexAddresses(client),
    ]);

  return {
    globalConfiguration,
    arenaAddresses,
    globalTraderIndexAddresses,
  };
};

export const resolveExchangeInstructionContext = async (
  symbol: Symbol,
  client: PhoenixInstructionClient
): Promise<ExchangeInstructionContext | undefined> => {
  if (!client.exchange) return undefined;
  await client.exchange.ready();
  return client.exchange.instructionContext(symbol);
};

export const getMarketMetadataForSymbol = async (
  symbol: Symbol,
  client: PhoenixInstructionClient
): Promise<MarketMetadataForSymbol> => {
  const exchangeContext = await resolveExchangeInstructionContext(
    symbol,
    client
  );
  if (exchangeContext) {
    return {
      marketAddress: exchangeContext.market.marketPubkey as MarketAddress,
      assetId: BigInt(exchangeContext.market.assetId),
      tickSize: exchangeContext.market.tickSize,
      baseLotDecimals: exchangeContext.market.baseLotsDecimals,
      splineAddress: exchangeContext.market
        .splinePubkey as SplineCollectionAddress,
    };
  }

  const globalConfiguration = await AccountFetchers.GlobalConfiguration({
    client,
    address: client.addresses.globalConfigurationAddress,
  });
  const perpAssetMap = await fetchPerpAssetMap({
    client,
    address: globalConfiguration.perpAssetMapKey,
  });

  const assetMetadata = perpAssetMap.metadata.entries.find(
    ({ key }) => key.toUpperCase() === symbol.toUpperCase()
  )?.value;
  if (!assetMetadata) {
    throw new Error(`Market for symbol '${symbol}' not found in PerpAssetMap`);
  }

  return {
    marketAddress: brandAddress<MarketAddress>(
      assetMetadata.staticMarketParams.marketAccount
    ),
    assetId: BigInt(assetMetadata.staticMarketParams.assetId),
  };
};

export const getMarketAddressForSymbol = async (
  symbol: Symbol,
  client: PhoenixInstructionClient
): Promise<MarketAddress> =>
  (await getMarketMetadataForSymbol(symbol, client)).marketAddress;

export const deriveTraderAddresses = (
  authority: Authority,
  traderPdaIndex: number,
  subaccountIndex: number,
  phoenixProgramAddress?: PhoenixProgramAddress
): {
  traderAccount: () => Promise<TraderAddress>;
  traderTokenAccount: (mint: MintAddress) => Promise<TokenAccountAddress>;
} => ({
  traderAccount: () =>
    getPhoenixTraderSubaccountAddress({
      authority,
      traderPdaIndex,
      subaccountIndex,
      phoenixProgramAddress,
    }),
  traderTokenAccount: (mint: MintAddress) =>
    getPhoenixTraderTokenAccountAddress(authority, mint),
});

export const getTraderAddresses = async (
  authority: Authority,
  mint: MintAddress,
  traderPdaIndex: number,
  traderSubaccountIndex: number,
  phoenixProgramAddress?: PhoenixProgramAddress
): Promise<{
  traderAccount: TraderAddress;
  traderTokenAccount: TokenAccountAddress;
}> => {
  const [traderAccount, traderTokenAccount] = await Promise.all([
    getPhoenixTraderSubaccountAddress({
      authority,
      traderPdaIndex,
      subaccountIndex: traderSubaccountIndex,
      phoenixProgramAddress,
    }),
    getPhoenixTraderTokenAccountAddress(authority, mint),
  ]);

  return {
    traderAccount,
    traderTokenAccount,
  };
};

export const getClientTraderAddresses = async (
  client: Pick<PhoenixInstructionClient, "addresses">,
  authority: Authority,
  mint: MintAddress,
  traderPdaIndex: number,
  traderSubaccountIndex: number
): Promise<{
  traderAccount: TraderAddress;
  traderTokenAccount: TokenAccountAddress;
}> =>
  getTraderAddresses(
    authority,
    mint,
    traderPdaIndex,
    traderSubaccountIndex,
    client.addresses.phoenixProgramAddress
  );

export const fetchSubaccountForAsset = async (
  client: PhoenixInstructionClient,
  authority: Authority,
  traderPdaIndex: number,
  marginType: MarginType,
  assetId: number
): Promise<SubaccountInfo> => {
  const maxPositions = toMaxPositions(marginType);

  for (let index = 0; index <= MAX_SUBACCOUNTS; index++) {
    const subaccountAddress = await getPhoenixTraderSubaccountAddress({
      authority,
      traderPdaIndex,
      subaccountIndex: index,
      phoenixProgramAddress: client.addresses.phoenixProgramAddress,
    });

    try {
      const trader = await fetchTrader({
        client,
        address: subaccountAddress,
        skipCache: true,
      });

      if (trader.maxPositions !== maxPositions) {
        continue;
      }

      let isSuitable = false;
      if (trader.positions.len === 0n) {
        isSuitable = true;
      } else if (marginType === MarginType.Isolated) {
        isSuitable = trader.positions.entries.some(
          ({ key }) => key === BigInt(assetId)
        );
      } else {
        isSuitable = true;
      }

      if (isSuitable) {
        return {
          index,
          authority,
          address: subaccountAddress,
          maxPositions,
        };
      }
    } catch {
      continue;
    }
  }

  const marginTypeStr =
    marginType === MarginType.Isolated ? "isolated" : "cross";

  throw new Error(
    `No suitable ${marginTypeStr} subaccount found for trader ${authority} and asset ${assetId}. ` +
      `For isolated margin, you need either an empty subaccount or one with an existing position in asset ${assetId}.`
  );
};

export const buildCreateAssociatedTokenAccountIdempotent = async (params: {
  payer: Authority;
  owner: Authority;
  mint: MintAddress;
  tokenProgram?: SPLTokenProgramAddress;
}): Promise<InstructionsWithAccountsAndData> => {
  const {
    payer,
    owner,
    mint,
    tokenProgram = SPL_TOKEN_PROGRAM_ADDRESS,
  } = params;
  const ataAddress = await getPhoenixTraderTokenAccountAddress(owner, mint);

  return buildCreateAssociatedTokenAccountIdempotentSync({
    payer,
    ataAddress,
    owner,
    mint,
    tokenProgram,
  });
};

export const buildCreateAssociatedTokenAccountIdempotentSync = (params: {
  payer: Authority;
  ataAddress: Address;
  owner: Authority;
  mint: MintAddress;
  tokenProgram?: SPLTokenProgramAddress;
}): InstructionsWithAccountsAndData => {
  const {
    payer,
    ataAddress,
    owner,
    mint,
    tokenProgram = SPL_TOKEN_PROGRAM_ADDRESS,
  } = params;

  return {
    programAddress: SPL_ATA_PROGRAM_ADDRESS,
    accounts: [
      generateWritableSignerAccount(payer),
      generateWritableAccount(ataAddress),
      generateReadonlyAccount(owner),
      generateReadonlyAccount(mint),
      generateReadonlyAccount(SYSTEM_PROGRAM_ADDRESS),
      generateReadonlyAccount(tokenProgram),
    ] as const,
    data: new Uint8Array([1]),
  };
};

const buildSplTokenAmountInstructionData = (
  discriminator: number,
  amount: bigint
): Uint8Array => {
  const amountBytes = new Uint8Array(8);
  new DataView(amountBytes.buffer).setBigUint64(0, amount, true);

  const data = new Uint8Array(9);
  data.set([discriminator], 0);
  data.set(amountBytes, 1);
  return data;
};

export const buildSplTokenApprove = (params: {
  owner: Authority;
  tokenAccount: TokenAccountAddress;
  delegate: Authority | EmberStateAddress;
  amount: bigint;
}): InstructionsWithAccountsAndData => {
  return {
    programAddress: SPL_TOKEN_PROGRAM_ADDRESS,
    accounts: [
      generateWritableAccount(params.tokenAccount),
      generateReadonlyAccount(params.delegate),
      generateReadonlySignerAccount(params.owner),
    ] as const,
    data: buildSplTokenAmountInstructionData(4, params.amount),
  };
};

export const buildSplTokenTransfer = (params: {
  owner: Authority;
  sourceTokenAccount: TokenAccountAddress;
  destinationTokenAccount: TokenAccountAddress;
  amount: bigint;
}): InstructionsWithAccountsAndData => {
  return {
    programAddress: SPL_TOKEN_PROGRAM_ADDRESS,
    accounts: [
      generateWritableAccount(params.sourceTokenAccount),
      generateWritableAccount(params.destinationTokenAccount),
      generateReadonlySignerAccount(params.owner),
    ] as const,
    data: buildSplTokenAmountInstructionData(3, params.amount),
  };
};

export { DEFAULT_MARKET_ORDER_SLIPPAGE };
