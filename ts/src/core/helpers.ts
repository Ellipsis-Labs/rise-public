import {
  AccountFetchers,
  decodeTrader,
  fetchPerpAssetMap,
  fetchTrader,
  type GlobalConfiguration,
} from "@/accounts";
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
import type {
  PhoenixAccountExistenceClient,
  PhoenixInstructionClient,
} from "@/core/clientTypes";
import { ACCOUNT_DISCRIMINANTS } from "@/core/discriminants";
import { getFixedLengthArrayCodec } from "@/primitives/_utilityTypes/FixedLengthArray";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import {
  type ActiveTraderBufferAddressArray,
  type ActiveTraderBufferArenaAddress,
  type Authority,
  type EmberStateAddress,
  type GlobalTraderIndexAddressArray,
  type GlobalTraderIndexArenaAddress,
  type MarketAddress,
  type MintAddress,
  type PerpAssetMapAddress,
  type PhoenixProgramAddress,
  type SPLTokenProgramAddress,
  type SplineCollectionAddress,
  type TokenAccountAddress,
  type TraderAddress,
  type WithdrawQueueAddress,
} from "@/primitives";
import { MarginType, type Symbol, toMaxPositions } from "@/primitives";
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

/**
 * The addresses instruction flows need, and nothing more.
 *
 * Deliberately *not* a `GlobalConfiguration`: one construction path (the
 * exchange metadata snapshot) cannot read the full account, and impersonating
 * it would mean fabricating values for every field the snapshot lacks. When a
 * flow needs actual configuration contents, it must decode the account.
 */
export interface RequiredAccounts {
  canonicalTokenMintKey: MintAddress;
  perpAssetMapKey: PerpAssetMapAddress;
  withdrawQueueKey: WithdrawQueueAddress;
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
  maxPositions: number;
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
  client: ExchangeLookupClient,
  globalConfiguration?: GlobalConfiguration
): Promise<ActiveTraderBufferAddressArray> => {
  const configuration =
    globalConfiguration ??
    (await AccountFetchers.GlobalConfiguration({
      client,
      address: client.addresses.globalConfigurationAddress,
    }));

  const headerAddress = configuration.activeTraderBufferHeaderKey;
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
  client: ExchangeLookupClient,
  globalConfiguration?: GlobalConfiguration
): Promise<GlobalTraderIndexAddressArray> => {
  const configuration =
    globalConfiguration ??
    (await AccountFetchers.GlobalConfiguration({
      client,
      address: client.addresses.globalConfigurationAddress,
    }));

  const headerAddress = configuration.globalTraderIndexHeaderKey;
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
  client: PhoenixInstructionClient,
  globalConfiguration?: GlobalConfiguration
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
      canonicalTokenMintKey: brandAddress<MintAddress>(
        snapshot.exchange.canonicalMint
      ),
      perpAssetMapKey: brandAddress<PerpAssetMapAddress>(
        snapshot.exchange.perpAssetMap
      ),
      withdrawQueueKey: brandAddress<WithdrawQueueAddress>(
        snapshot.exchange.withdrawQueue
      ),
      arenaAddresses: snapshot.exchange
        .activeTraderBuffer as ActiveTraderBufferAddressArray,
      globalTraderIndexAddresses: snapshot.exchange
        .globalTraderIndex as GlobalTraderIndexAddressArray,
    };
  }

  const configuration =
    globalConfiguration ??
    (await AccountFetchers.GlobalConfiguration({
      client,
      address: client.addresses.globalConfigurationAddress,
    }));
  const [arenaAddresses, globalTraderIndexAddresses] = await Promise.all([
    getActiveTraderBufferAddresses(client, configuration),
    getGlobalTraderIndexAddresses(client, configuration),
  ]);

  return {
    canonicalTokenMintKey: configuration.canonicalTokenMintKey,
    perpAssetMapKey: configuration.perpAssetMapKey,
    withdrawQueueKey: configuration.withdrawQueueKey,
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

export interface IsolatedSubaccountAllocation {
  index: number;
  address: TraderAddress;
  /** True when the subaccount does not exist yet and must be registered first. */
  needsRegistration: boolean;
}

/** Eligibility of an existing subaccount for an isolated order on an asset. */
type IsolatedSubaccountClass = "asset" | "empty" | "occupied" | "ineligible";

const classifyIsolatedTrader = (
  trader: ReturnType<typeof decodeTrader>,
  assetId: number,
  isolatedMaxPositions: number
): IsolatedSubaccountClass => {
  // Only isolated subaccounts (max_positions === 1) are eligible.
  if (trader.maxPositions !== isolatedMaxPositions) {
    return "ineligible";
  }
  if (trader.positions.entries.some(({ key }) => key === BigInt(assetId))) {
    return "asset";
  }
  if (trader.positions.len === 0n) {
    return "empty";
  }
  return "occupied";
};

/**
 * Pick the isolated subaccount to use for an order on `assetId`, mirroring the
 * backend `find_or_allocate_isolated_subaccount`. Subaccount 0 is reserved for
 * cross margin, so isolated subaccounts start at index 1. Preference order:
 *   1. an existing isolated subaccount already holding a position in `assetId`,
 *   2. an existing empty isolated subaccount (reused, no registration),
 *   3. the first unregistered index (caller must register it first).
 *
 * Unlike {@link fetchSubaccountForAsset}, this never throws on "nothing
 * exists" — it returns the first free index with `needsRegistration: true` so
 * the caller can prepend a `register_trader` instruction.
 *
 * When the client implements {@link AccountFetcherClient.fetchMaybeAccounts},
 * the whole subaccount range is read in a single `getMultipleAccounts`
 * round-trip; otherwise it falls back to a per-address scan (early-returning on
 * an asset match to limit round-trips).
 */
export const allocateIsolatedSubaccount = async (
  client: PhoenixInstructionClient & PhoenixAccountExistenceClient,
  authority: Authority,
  traderPdaIndex: number,
  assetId: number
): Promise<IsolatedSubaccountAllocation> => {
  const isolatedMaxPositions = toMaxPositions(MarginType.Isolated);

  // Subaccount 0 is reserved for cross margin; derive isolated slots 1..MAX.
  // Address derivation is local, so deriving the full range up front is cheap.
  const candidates = await Promise.all(
    Array.from({ length: MAX_SUBACCOUNTS }, (_unused, i) => i + 1).map(
      async (index) => ({
        index,
        address: await getPhoenixTraderSubaccountAddress({
          authority,
          traderPdaIndex,
          subaccountIndex: index,
          phoenixProgramAddress: client.addresses.phoenixProgramAddress,
        }),
      })
    )
  );

  let firstEmpty: { index: number; address: TraderAddress } | undefined;
  let firstUnregistered: { index: number; address: TraderAddress } | undefined;

  const recordEmpty = (slot: { index: number; address: TraderAddress }) => {
    if (firstEmpty === undefined) {
      firstEmpty = slot;
    }
  };
  const recordUnregistered = (slot: {
    index: number;
    address: TraderAddress;
  }) => {
    if (firstUnregistered === undefined) {
      firstUnregistered = slot;
    }
  };

  const resolve = (): IsolatedSubaccountAllocation => {
    if (firstEmpty !== undefined) {
      return { ...firstEmpty, needsRegistration: false };
    }
    if (firstUnregistered !== undefined) {
      return { ...firstUnregistered, needsRegistration: true };
    }
    throw new Error(
      `No available isolated subaccount slot for trader ${authority} and asset ${assetId} ` +
        `(all ${MAX_SUBACCOUNTS} subaccounts are in use by other assets).`
    );
  };

  // Fast path: a single batched fetch for the entire range.
  if (client.fetchMaybeAccounts !== undefined) {
    const accounts = await client.fetchMaybeAccounts(
      candidates.map((candidate) => candidate.address)
    );
    for (let i = 0; i < candidates.length; i++) {
      const { index, address } = candidates[i];
      const account = accounts[i];
      if (!account) {
        recordUnregistered({ index, address });
        continue;
      }
      let trader: ReturnType<typeof decodeTrader>;
      try {
        // decode only reads; the readonly view is safe to pass as the buffer.
        trader = decodeTrader(account.data as Uint8Array);
      } catch {
        // Exists but not a decodable trader account; skip it.
        continue;
      }
      const classification = classifyIsolatedTrader(
        trader,
        assetId,
        isolatedMaxPositions
      );
      if (classification === "asset") {
        return { index, address, needsRegistration: false };
      }
      if (classification === "empty") {
        recordEmpty({ index, address });
      }
    }
    return resolve();
  }

  // Fallback: per-address probe for clients without batch support.
  for (const { index, address } of candidates) {
    if (!(await client.accountExists(address))) {
      recordUnregistered({ index, address });
      continue;
    }

    let trader;
    try {
      trader = await fetchTrader({ client, address, skipCache: true });
    } catch {
      // Account exists but could not be decoded as a trader; skip it rather
      // than risk reusing or re-registering an incompatible account.
      continue;
    }

    const classification = classifyIsolatedTrader(
      trader,
      assetId,
      isolatedMaxPositions
    );
    if (classification === "asset") {
      return { index, address, needsRegistration: false };
    }
    if (classification === "empty") {
      recordEmpty({ index, address });
    }
  }

  return resolve();
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
