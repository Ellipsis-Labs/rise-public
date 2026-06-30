import { existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { FailedTransactionMetadata, LiteSVM } from "litesvm";
import {
  AccountRole,
  address,
  appendTransactionMessageInstructions,
  createKeyPairSignerFromPrivateKeyBytes,
  createTransactionMessage,
  getBase58Encoder,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  signTransactionMessageWithSigners,
} from "@solana/kit";
import type {
  AccountMeta,
  AccountSignerMeta,
  Address,
  Blockhash,
  Instruction,
  InstructionWithAccounts,
  InstructionWithData,
  InstructionWithSigners,
  ReadonlyUint8Array,
  Transaction,
  TransactionSigner,
} from "@solana/kit";
import { decodeOrderbook, decodeTrader } from "../../src/accounts";
import { HAWKEYE_PROGRAM_ADDRESS } from "../../src/hawkeye";
import type {
  Orderbook,
  Trader,
  TraderPosition,
  TraderState,
} from "../../src/accounts";
import {
  getTraderPositionDecoder,
  getTraderStateDecoder,
} from "../../src/accounts/internal";
import type {
  BuildDepositIxsResolvedInput,
  BuildWithdrawIxsResolvedInput,
} from "../../src/ixs/types";
import type { ResolvedPlaceOrderContext } from "../../src/ixs/types";
import type { OrderPacketMarketParams } from "../../src/orderPackets";
import type { InstructionsWithAccountsAndData } from "../../src/primitives";
import type {
  ActiveTraderBufferAddressArray,
  Authority,
  EmberStateAddress,
  EmberVaultAddress,
  GlobalConfigurationAddress,
  GlobalTraderIndexAddressArray,
  GlobalVaultAddress,
  LogAuthorityAddress,
  MarketAddress,
  MintAddress,
  PerpAssetMapAddress,
  PhoenixProgramAddress,
  SplineCollectionAddress,
  TokenAccountAddress,
  TraderAddress,
  WithdrawQueueAddress,
} from "../../src/primitives";
import {
  decodeFixtureInstruction,
  getSdkLocalnetTransaction,
  sdkLocalnetSeedBytes,
} from "./fixture";
import type {
  DecodedFixtureInstruction,
  FixtureActor,
  FixtureMarket,
  FixtureMint,
  FixtureTransaction,
  SdkLocalnetFixture,
} from "./fixture";

export interface SdkLocalnetProgramPaths {
  phoenixEternal: string;
  ember: string;
  hawkeye?: string;
}

export interface FindSdkLocalnetProgramPathsOptions {
  repoRoot?: string;
  programPaths?: Partial<SdkLocalnetProgramPaths>;
}

export interface CreateSdkLocalnetContextOptions extends FindSdkLocalnetProgramPathsOptions {
  fixture?: SdkLocalnetFixture;
  vm?: LiteSVM;
  executeSetup?: boolean;
}

export interface SdkLocalnetSigners {
  all: readonly TransactionSigner[];
  bySeed: ReadonlyMap<string, TransactionSigner>;
  byName: ReadonlyMap<string, TransactionSigner>;
  byAddress: ReadonlyMap<string, TransactionSigner>;
}

export interface SdkLocalnetSendInstructionsOptions {
  feePayerSeed?: string;
  label?: string;
}

export interface SdkLocalnetInstructionExecution {
  label: string;
  metadata: TransactionMetadata;
}

export interface SdkLocalnetLimitOrderHeadState {
  head: number | null;
}

export interface SdkLocalnetActiveTraderPositionState {
  position: TraderPosition;
  askOrders: SdkLocalnetLimitOrderHeadState;
  bidOrders: SdkLocalnetLimitOrderHeadState;
  totalNonReduceOnlyAskBaseLots: bigint;
  totalNonReduceOnlyBidBaseLots: bigint;
}

export interface SdkLocalnetGlobalTraderIndexEntry {
  nodePointer: number;
  traderAccount: TraderAddress;
  state: TraderState;
}

export interface SdkLocalnetEffectiveTraderState {
  source: "cold" | "globalTraderIndex";
  trader: Trader;
  state: TraderState;
  globalTraderIndexEntry: SdkLocalnetGlobalTraderIndexEntry | null;
}

export interface SdkLocalnetEffectiveTraderPosition {
  source: "cold" | "activeTraderBuffer";
  trader: Trader;
  assetId: number;
  position: TraderPosition;
  activePositionState: SdkLocalnetActiveTraderPositionState | null;
  globalTraderIndexEntry: SdkLocalnetGlobalTraderIndexEntry | null;
}

export interface SdkLocalnetContext {
  fixture: SdkLocalnetFixture;
  vm: LiteSVM;
  programPaths: SdkLocalnetProgramPaths;
  signers: SdkLocalnetSigners;
  setupResults: SdkLocalnetInstructionExecution[];
  getSigner(identifier: string): TransactionSigner;
  getActor(name: string): FixtureActor;
  getMarket(symbol: string): FixtureMarket;
  buildTransaction(
    instructions: readonly SdkLocalnetInstruction[],
    options?: SdkLocalnetSendInstructionsOptions
  ): Promise<Transaction>;
  sendInstructions(
    instructions: readonly SdkLocalnetInstruction[],
    options?: SdkLocalnetSendInstructionsOptions
  ): Promise<SdkLocalnetInstructionExecution>;
  sendFixtureTransaction(
    transaction: string | FixtureTransaction
  ): Promise<SdkLocalnetInstructionExecution[]>;
}

export type SdkLocalnetInstruction = InstructionsWithAccountsAndData & {
  readonly programAddress?: Address | string;
};

type SignableAccountMeta = AccountMeta | AccountSignerMeta;

type SdkLocalnetInstructionWithSigners = Instruction &
  InstructionWithData<ReadonlyUint8Array> &
  InstructionWithAccounts<readonly SignableAccountMeta[]> &
  InstructionWithSigners;

type TransactionMetadata = Exclude<
  ReturnType<LiteSVM["sendTransaction"]>,
  FailedTransactionMetadata
>;

type LiteSvmAddress = Parameters<LiteSVM["addProgramFromFile"]>[0];
type LiteSvmLamports = Parameters<LiteSVM["airdrop"]>[1];
type LiteSvmTransaction = Parameters<LiteSVM["sendTransaction"]>[0];

const REQUIRED_PROGRAM_ARTIFACT_ENV = "RISE_SDK_LOCALNET_REQUIRE_PROGRAMS";
const PHOENIX_REPO_ROOT_ENV = "PHOENIX_REPO_ROOT";
const ETERNAL_PROGRAM_ENV = "RISE_SDK_LOCALNET_ETERNAL_SO";
const EMBER_PROGRAM_ENV = "RISE_SDK_LOCALNET_EMBER_SO";
const HAWKEYE_PROGRAM_ENV = "RISE_SDK_LOCALNET_HAWKEYE_SO";
const DEFAULT_LAST_VALID_BLOCK_HEIGHT = 1_000_000n;

export const isSdkLocalnetVmRequired = (): boolean =>
  process.env[REQUIRED_PROGRAM_ARTIFACT_ENV] === "1" ||
  process.env[REQUIRED_PROGRAM_ARTIFACT_ENV] === "true";

export const findSdkLocalnetProgramPaths = (
  options: FindSdkLocalnetProgramPathsOptions = {}
): SdkLocalnetProgramPaths | null => {
  const explicit = explicitProgramPaths(options.programPaths);
  if (explicit) {
    return allProgramPathsExist(explicit) ? explicit : null;
  }

  for (const candidate of defaultProgramPathCandidates(options.repoRoot)) {
    if (allProgramPathsExist(candidate)) {
      return candidate;
    }
  }

  return null;
};

export const resolveSdkLocalnetProgramPaths = (
  options: FindSdkLocalnetProgramPathsOptions = {}
): SdkLocalnetProgramPaths => {
  const programPaths = findSdkLocalnetProgramPaths(options);
  if (programPaths) {
    return programPaths;
  }

  throw new Error(
    [
      "Missing local SBF program artifacts for the Rise SDK localnet harness.",
      "Run `mise build-sbf` from the repository root, or pass explicit program paths.",
      `Overrides: ${ETERNAL_PROGRAM_ENV}=... ${EMBER_PROGRAM_ENV}=... ${PHOENIX_REPO_ROOT_ENV}=...`,
    ].join(" ")
  );
};

export const createSdkLocalnetSigners = async (
  fixture?: SdkLocalnetFixture
): Promise<SdkLocalnetSigners> => {
  const resolvedFixture = fixture ?? (await loadDefaultSdkLocalnetFixture());
  const bySeed = new Map<string, TransactionSigner>();
  const byName = new Map<string, TransactionSigner>();
  const byAddress = new Map<string, TransactionSigner>();
  const all: TransactionSigner[] = [];

  for (const signerConfig of resolvedFixture.signers) {
    const signer = await createKeyPairSignerFromPrivateKeyBytes(
      sdkLocalnetSeedBytes(signerConfig.seed, resolvedFixture)
    );
    if (signer.address !== signerConfig.pubkey) {
      throw new Error(
        `Derived signer ${signerConfig.name} did not match fixture pubkey ` +
          `${signerConfig.pubkey}; got ${signer.address}`
      );
    }

    bySeed.set(signerConfig.seed, signer);
    byName.set(signerConfig.name, signer);
    byAddress.set(signer.address, signer);
    all.push(signer);
  }

  return { all, bySeed, byName, byAddress };
};

export const createSdkLocalnetContext = async (
  options: CreateSdkLocalnetContextOptions = {}
): Promise<SdkLocalnetContext> => {
  const fixture = options.fixture ?? (await loadDefaultSdkLocalnetFixture());
  const programPaths = resolveSdkLocalnetProgramPaths(options);
  const vm = options.vm ?? new LiteSVM();
  const signers = await createSdkLocalnetSigners(fixture);

  loadSdkLocalnetPrograms(vm, fixture, programPaths);
  fundSdkLocalnetSigners(vm, fixture, signers);

  const context = createContext({
    fixture,
    vm,
    programPaths,
    signers,
    setupResults: [],
  });

  if (options.executeSetup !== false) {
    context.setupResults.push(
      ...(await executeSdkLocalnetFixtureSetup(context))
    );
  }

  return context;
};

const loadDefaultSdkLocalnetFixture = async (): Promise<SdkLocalnetFixture> => {
  const { defaultSdkLocalnetFixture } = await import("./default-fixture");
  return defaultSdkLocalnetFixture;
};

export const executeSdkLocalnetFixtureSetup = async (
  context: Pick<
    SdkLocalnetContext,
    "fixture" | "sendFixtureTransaction" | "setupResults"
  >
): Promise<SdkLocalnetInstructionExecution[]> => {
  const results: SdkLocalnetInstructionExecution[] = [];

  for (const transaction of context.fixture.setupTransactions) {
    results.push(...(await context.sendFixtureTransaction(transaction)));
  }

  return results;
};

export const buildSdkLocalnetTransaction = async (
  vm: LiteSVM,
  signers: SdkLocalnetSigners,
  instructions: readonly SdkLocalnetInstruction[],
  options: SdkLocalnetSendInstructionsOptions = {}
): Promise<Transaction> => {
  const feePayer = getRequiredSigner(signers, options.feePayerSeed ?? "payer");
  const signableInstructions = instructions.map((instruction) =>
    attachSdkLocalnetSigners(asSdkLocalnetInstruction(instruction), signers)
  );
  const message = appendTransactionMessageInstructions(
    signableInstructions,
    setTransactionMessageLifetimeUsingBlockhash(
      {
        blockhash: vm.latestBlockhash() as unknown as Blockhash,
        lastValidBlockHeight: DEFAULT_LAST_VALID_BLOCK_HEIGHT,
      },
      setTransactionMessageFeePayerSigner(
        feePayer,
        createTransactionMessage({ version: 0 })
      )
    )
  );

  return signTransactionMessageWithSigners(message);
};

export const sendSdkLocalnetInstructions = async (
  vm: LiteSVM,
  signers: SdkLocalnetSigners,
  instructions: readonly SdkLocalnetInstruction[],
  options: SdkLocalnetSendInstructionsOptions = {}
): Promise<SdkLocalnetInstructionExecution> => {
  const label = options.label ?? "sdk-localnet-transaction";
  const transaction = await buildSdkLocalnetTransaction(
    vm,
    signers,
    instructions,
    options
  );
  const result = vm.sendTransaction(
    transaction as unknown as LiteSvmTransaction
  );

  return {
    label,
    metadata: assertSuccessfulTransaction(result, label),
  };
};

export const buildSdkLocalnetPlaceOrderContext = (
  context: Pick<SdkLocalnetContext, "fixture" | "getActor" | "getMarket">,
  params: { actorName: string; symbol: string }
): ResolvedPlaceOrderContext => {
  const actor = context.getActor(params.actorName);
  const market = context.getMarket(params.symbol);

  return {
    exchange: {
      phoenixProgramAddress: context.fixture.programs
        .phoenixEternal as PhoenixProgramAddress,
      logAuthorityAddress: context.fixture.addresses
        .logAuthority as LogAuthorityAddress,
      globalConfigurationAddress: context.fixture.addresses
        .globalConfig as GlobalConfigurationAddress,
      perpAssetMap: context.fixture.addresses
        .perpAssetMap as PerpAssetMapAddress,
      globalTraderIndex: context.fixture.addresses
        .globalTraderIndex as GlobalTraderIndexAddressArray,
      activeTraderBuffer: context.fixture.addresses
        .activeTraderBuffer as ActiveTraderBufferAddressArray,
    },
    market: {
      marketAddress: market.orderbook as MarketAddress,
      splineCollection: market.spline as SplineCollectionAddress,
    },
    trader: {
      authority: actor.pubkey as Authority,
      traderAccount: actor.traderAccount as TraderAddress,
    },
  };
};

export const buildSdkLocalnetMarketParams = (
  context: Pick<SdkLocalnetContext, "getMarket">,
  symbol: string
): OrderPacketMarketParams => {
  const market = context.getMarket(symbol);
  return {
    tickSize: market.tickSizeInQuoteLotsPerBaseLot,
    baseLotsDecimals: market.baseLotDecimals,
  };
};

export const buildSdkLocalnetDepositInput = (
  context: Pick<SdkLocalnetContext, "fixture" | "getActor">,
  params: { actorName: string; amount: bigint; feePayer?: string }
): BuildDepositIxsResolvedInput => {
  const actor = context.getActor(params.actorName);
  const fakeUsdc = getFixtureMint(context.fixture, "fakeUsdc");
  const phoenixCollateral = getFixtureMint(
    context.fixture,
    "phoenixCollateral"
  );

  return {
    feePayer: params.feePayer as Authority | undefined,
    exchange: {
      phoenixProgramAddress: context.fixture.programs
        .phoenixEternal as PhoenixProgramAddress,
      logAuthorityAddress: context.fixture.addresses
        .logAuthority as LogAuthorityAddress,
      globalConfigurationAddress: context.fixture.addresses
        .globalConfig as GlobalConfigurationAddress,
      canonicalMint: phoenixCollateral.pubkey as MintAddress,
      usdcMint: fakeUsdc.pubkey as MintAddress,
      globalVault: context.fixture.addresses.globalVault as GlobalVaultAddress,
      globalTraderIndex: context.fixture.addresses
        .globalTraderIndex as GlobalTraderIndexAddressArray,
      activeTraderBuffer: context.fixture.addresses
        .activeTraderBuffer as ActiveTraderBufferAddressArray,
      emberState: context.fixture.addresses.emberState as EmberStateAddress,
      emberVault: context.fixture.addresses.emberVault as EmberVaultAddress,
    },
    trader: {
      authority: actor.pubkey as Authority,
      traderAccount: actor.traderAccount as TraderAddress,
      usdcTokenAccount: actor.fakeUsdcTokenAccount as TokenAccountAddress,
      phoenixTokenAccount: actor.phoenixTokenAccount as TokenAccountAddress,
    },
    amount: params.amount,
  };
};

export const buildSdkLocalnetWithdrawInput = (
  context: Pick<SdkLocalnetContext, "fixture" | "getActor">,
  params: { actorName: string; amount: bigint; feePayer?: string }
): BuildWithdrawIxsResolvedInput => {
  const actor = context.getActor(params.actorName);
  const fakeUsdc = getFixtureMint(context.fixture, "fakeUsdc");
  const phoenixCollateral = getFixtureMint(
    context.fixture,
    "phoenixCollateral"
  );

  return {
    feePayer: params.feePayer as Authority | undefined,
    exchange: {
      phoenixProgramAddress: context.fixture.programs
        .phoenixEternal as PhoenixProgramAddress,
      logAuthorityAddress: context.fixture.addresses
        .logAuthority as LogAuthorityAddress,
      globalConfigurationAddress: context.fixture.addresses
        .globalConfig as GlobalConfigurationAddress,
      canonicalMint: phoenixCollateral.pubkey as MintAddress,
      usdcMint: fakeUsdc.pubkey as MintAddress,
      perpAssetMap: context.fixture.addresses
        .perpAssetMap as PerpAssetMapAddress,
      globalVault: context.fixture.addresses.globalVault as GlobalVaultAddress,
      withdrawQueue: context.fixture.addresses
        .withdrawQueue as WithdrawQueueAddress,
      globalTraderIndex: context.fixture.addresses
        .globalTraderIndex as GlobalTraderIndexAddressArray,
      activeTraderBuffer: context.fixture.addresses
        .activeTraderBuffer as ActiveTraderBufferAddressArray,
      emberState: context.fixture.addresses.emberState as EmberStateAddress,
      emberVault: context.fixture.addresses.emberVault as EmberVaultAddress,
    },
    trader: {
      authority: actor.pubkey as Authority,
      traderAccount: actor.traderAccount as TraderAddress,
      usdcTokenAccount: actor.fakeUsdcTokenAccount as TokenAccountAddress,
      phoenixTokenAccount: actor.phoenixTokenAccount as TokenAccountAddress,
    },
    amount: params.amount,
  };
};

export const TRADER_CAPABILITY_HOT: number = 1 << 0;

export const isHotTraderCapabilityFlags = (flags: number): boolean =>
  (flags & TRADER_CAPABILITY_HOT) !== 0;

export const readSdkLocalnetTrader = (
  context: Pick<SdkLocalnetContext, "vm">,
  traderAccount: string
): Trader => decodeTrader(readSdkLocalnetAccountData(context, traderAccount));

export const readSdkLocalnetOrderbook = (
  context: Pick<SdkLocalnetContext, "fixture" | "vm">,
  symbolOrMarketAddress: string
): Orderbook => {
  const market = context.fixture.markets.find(
    (candidate) =>
      candidate.symbol === symbolOrMarketAddress ||
      candidate.orderbook === symbolOrMarketAddress
  );
  const orderbookAddress = market?.orderbook ?? symbolOrMarketAddress;
  return decodeOrderbook(readSdkLocalnetAccountData(context, orderbookAddress));
};

export const readSdkLocalnetEffectiveTraderState = (
  context: Pick<SdkLocalnetContext, "fixture" | "vm">,
  traderAccount: string
): SdkLocalnetEffectiveTraderState => {
  const trader = readSdkLocalnetTrader(context, traderAccount);
  if (!isHotTraderCapabilityFlags(trader.state.flags)) {
    return {
      source: "cold",
      trader,
      state: trader.state,
      globalTraderIndexEntry: null,
    };
  }

  const globalTraderIndexEntry = readSdkLocalnetGlobalTraderIndexEntry(
    context,
    trader.key
  );
  if (!globalTraderIndexEntry) {
    throw new Error(
      `Trader ${trader.key} is marked hot but is missing from the global trader index`
    );
  }

  return {
    source: "globalTraderIndex",
    trader,
    state: globalTraderIndexEntry.state,
    globalTraderIndexEntry,
  };
};

export const readSdkLocalnetEffectiveTraderPosition = (
  context: Pick<SdkLocalnetContext, "fixture" | "vm">,
  traderAccount: string,
  assetId = 0
): SdkLocalnetEffectiveTraderPosition | null => {
  const trader = readSdkLocalnetTrader(context, traderAccount);
  const coldPosition =
    trader.positions.entries.find((entry) => entry.key === BigInt(assetId))
      ?.value ?? null;

  if (!isHotTraderCapabilityFlags(trader.state.flags)) {
    if (!coldPosition) {
      return null;
    }
    return {
      source: "cold",
      trader,
      assetId,
      position: coldPosition,
      activePositionState: null,
      globalTraderIndexEntry: null,
    };
  }

  const globalTraderIndexEntry = readSdkLocalnetGlobalTraderIndexEntry(
    context,
    trader.key
  );
  if (!globalTraderIndexEntry) {
    throw new Error(
      `Trader ${trader.key} is marked hot but is missing from the global trader index`
    );
  }

  const activePositionState = readSdkLocalnetActiveTraderPositionState(
    context,
    globalTraderIndexEntry.nodePointer,
    assetId
  );
  if (!activePositionState) {
    return null;
  }

  return {
    source: "activeTraderBuffer",
    trader,
    assetId,
    position: activePositionState.position,
    activePositionState,
    globalTraderIndexEntry,
  };
};

export const readSdkLocalnetGlobalTraderIndexEntry = (
  context: Pick<SdkLocalnetContext, "fixture" | "vm">,
  traderAccount: TraderAddress
): SdkLocalnetGlobalTraderIndexEntry | null => {
  const target = getBase58Encoder().encode(traderAccount);
  const node = findSdkLocalnetDynamicRedBlackTreeNode({
    buffers: readSdkLocalnetAccountDataArray(
      context,
      context.fixture.addresses.globalTraderIndex
    ),
    nodeStrideBytes: GLOBAL_TRADER_INDEX_NODE_STRIDE_BYTES,
    compareKey: (bytes, keyOffset) =>
      compareBytes(target, bytes.subarray(keyOffset, keyOffset + PUBKEY_BYTES)),
  });
  if (!node) {
    return null;
  }

  const [state] = getTraderStateDecoder().read(
    node.bytes,
    node.keyOffset + PUBKEY_BYTES
  );
  return {
    nodePointer: node.nodePointer,
    traderAccount,
    state,
  };
};

export const readSdkLocalnetActiveTraderPositionState = (
  context: Pick<SdkLocalnetContext, "fixture" | "vm">,
  traderPointer: number,
  assetId: number
): SdkLocalnetActiveTraderPositionState | null => {
  const node = findSdkLocalnetDynamicRedBlackTreeNode({
    buffers: readSdkLocalnetAccountDataArray(
      context,
      context.fixture.addresses.activeTraderBuffer
    ),
    nodeStrideBytes: ACTIVE_TRADER_BUFFER_NODE_STRIDE_BYTES,
    compareKey: (bytes, keyOffset) =>
      compareTraderPositionId(bytes, keyOffset, traderPointer, assetId),
  });
  if (!node) {
    return null;
  }

  return decodeSdkLocalnetActiveTraderPositionState(
    node.bytes,
    node.keyOffset + TRADER_POSITION_ID_BYTES
  );
};

export const loadSdkLocalnetPrograms = (
  vm: LiteSVM,
  fixture: SdkLocalnetFixture,
  programPaths: SdkLocalnetProgramPaths
): void => {
  vm.addProgramFromFile(
    toLiteSvmAddress(fixture.programs.phoenixEternal),
    programPaths.phoenixEternal
  );
  vm.addProgramFromFile(
    toLiteSvmAddress(fixture.programs.ember),
    programPaths.ember
  );
  if (programPaths.hawkeye) {
    vm.addProgramFromFile(
      toLiteSvmAddress(HAWKEYE_PROGRAM_ADDRESS),
      programPaths.hawkeye
    );
  }
};

export const fundSdkLocalnetSigners = (
  vm: LiteSVM,
  fixture: SdkLocalnetFixture,
  signers: SdkLocalnetSigners
): void => {
  for (const signerConfig of fixture.signers) {
    if (signerConfig.initialLamports === 0) {
      continue;
    }

    const signer = getRequiredSigner(signers, signerConfig.seed);
    const result = vm.airdrop(
      signer.address as unknown as LiteSvmAddress,
      BigInt(signerConfig.initialLamports) as unknown as LiteSvmLamports
    );
    assertSuccessfulTransaction(result, `airdrop:${signerConfig.name}`);
  }
};

const GLOBAL_TRADER_INDEX_ACCOUNT_HEADER_BYTES = 48;
const ACTIVE_TRADER_BUFFER_ACCOUNT_HEADER_BYTES = 48;
const MULTI_ARENA_SUPERBLOCK_BYTES = 32;
const RED_BLACK_TREE_HEADER_BYTES = 16;
const RED_BLACK_TREE_NODE_REGISTER_BYTES = 16;
const INDEXED_ARENA_HEADER_BYTES = 32;
const PUBKEY_BYTES = 32;
const TRADER_STATE_BYTES = 16;
const TRADER_POSITION_ID_BYTES = 8;
const TRADER_POSITION_STATE_BYTES = 64;
const GLOBAL_TRADER_INDEX_NODE_STRIDE_BYTES =
  RED_BLACK_TREE_NODE_REGISTER_BYTES + PUBKEY_BYTES + TRADER_STATE_BYTES;
const ACTIVE_TRADER_BUFFER_NODE_STRIDE_BYTES =
  RED_BLACK_TREE_NODE_REGISTER_BYTES +
  TRADER_POSITION_ID_BYTES +
  TRADER_POSITION_STATE_BYTES;

type SdkLocalnetDynamicTreeNode = {
  nodePointer: number;
  bytes: Uint8Array;
  nodeOffset: number;
  keyOffset: number;
};

const readSdkLocalnetAccountData = (
  context: Pick<SdkLocalnetContext, "vm">,
  accountAddress: string
): Uint8Array => {
  const account = context.vm.getAccount(address(accountAddress));
  if (!account || !("data" in account)) {
    throw new Error(`Missing SDK localnet account ${accountAddress}`);
  }
  return Uint8Array.from(account.data);
};

const readSdkLocalnetAccountDataArray = (
  context: Pick<SdkLocalnetContext, "vm">,
  accountAddresses: readonly string[]
): Uint8Array[] =>
  accountAddresses.map((accountAddress) =>
    readSdkLocalnetAccountData(context, accountAddress)
  );

const findSdkLocalnetDynamicRedBlackTreeNode = (params: {
  buffers: readonly Uint8Array[];
  nodeStrideBytes: number;
  compareKey: (bytes: Uint8Array, keyOffset: number) => number;
}): SdkLocalnetDynamicTreeNode | null => {
  const firstBuffer = params.buffers[0];
  if (!firstBuffer) {
    throw new Error("Dynamic red-black tree has no buffers");
  }

  const accountHeaderBytes =
    params.nodeStrideBytes === GLOBAL_TRADER_INDEX_NODE_STRIDE_BYTES
      ? GLOBAL_TRADER_INDEX_ACCOUNT_HEADER_BYTES
      : ACTIVE_TRADER_BUFFER_ACCOUNT_HEADER_BYTES;
  const superblockOffset = accountHeaderBytes;
  const treeHeaderOffset = superblockOffset + MULTI_ARENA_SUPERBLOCK_BYTES;
  const numNodesPerArena = readU32LE(firstBuffer, superblockOffset + 8);
  let nodePointer = readU32LE(firstBuffer, treeHeaderOffset);
  let guard = 0;

  while (nodePointer !== 0) {
    if (numNodesPerArena === 0) {
      throw new Error("Dynamic red-black tree superblock is uninitialized");
    }
    if (++guard > Number(readU32LE(firstBuffer, superblockOffset) + 1)) {
      throw new Error("Dynamic red-black tree traversal exceeded tree size");
    }

    const node = getSdkLocalnetDynamicTreeNode(
      params.buffers,
      nodePointer,
      numNodesPerArena,
      params.nodeStrideBytes,
      accountHeaderBytes
    );
    const comparison = params.compareKey(node.bytes, node.keyOffset);
    if (comparison === 0) {
      return node;
    }

    nodePointer = readU32LE(
      node.bytes,
      node.nodeOffset + (comparison < 0 ? 0 : 4)
    );
  }

  return null;
};

const getSdkLocalnetDynamicTreeNode = (
  buffers: readonly Uint8Array[],
  nodePointer: number,
  numNodesPerArena: number,
  nodeStrideBytes: number,
  accountHeaderBytes: number
): SdkLocalnetDynamicTreeNode => {
  const pageNo = nodePointer - 1;
  const arenaIndex = Math.floor(pageNo / numNodesPerArena);
  const nodeIndex = pageNo % numNodesPerArena;
  const bytes = buffers[arenaIndex];
  if (!bytes) {
    throw new Error(
      `Dynamic red-black tree node ${nodePointer} references missing arena ${arenaIndex}`
    );
  }

  const arenaOffset =
    arenaIndex === 0
      ? accountHeaderBytes +
        MULTI_ARENA_SUPERBLOCK_BYTES +
        RED_BLACK_TREE_HEADER_BYTES
      : INDEXED_ARENA_HEADER_BYTES;
  const nodeOffset = arenaOffset + nodeIndex * nodeStrideBytes;

  return {
    nodePointer,
    bytes,
    nodeOffset,
    keyOffset: nodeOffset + RED_BLACK_TREE_NODE_REGISTER_BYTES,
  };
};

const decodeSdkLocalnetActiveTraderPositionState = (
  bytes: Uint8Array,
  offset: number
): SdkLocalnetActiveTraderPositionState => {
  const [position, afterPosition] = getTraderPositionDecoder().read(
    bytes,
    offset
  );
  const askOrders = readLimitOrderHead(bytes, afterPosition + 4);
  const bidOrders = readLimitOrderHead(bytes, afterPosition + 12);
  const totalNonReduceOnlyAskBaseLots = BigInt(
    readU32LE(bytes, afterPosition + 16)
  );
  const totalNonReduceOnlyBidBaseLots = BigInt(
    readU32LE(bytes, afterPosition + 24)
  );

  return {
    position,
    askOrders,
    bidOrders,
    totalNonReduceOnlyAskBaseLots,
    totalNonReduceOnlyBidBaseLots,
  };
};

const readLimitOrderHead = (
  bytes: Uint8Array,
  offset: number
): SdkLocalnetLimitOrderHeadState => ({
  head: readOptionalU32LE(bytes, offset),
});

const compareTraderPositionId = (
  bytes: Uint8Array,
  keyOffset: number,
  traderPointer: number,
  assetId: number
): number => {
  const nodeTraderPointer = readU32LE(bytes, keyOffset);
  if (traderPointer !== nodeTraderPointer) {
    return traderPointer < nodeTraderPointer ? -1 : 1;
  }

  const nodeAssetId = readU32LE(bytes, keyOffset + 4);
  if (assetId === nodeAssetId) {
    return 0;
  }
  return assetId < nodeAssetId ? -1 : 1;
};

const compareBytes = (
  left: ReadonlyUint8Array,
  right: ReadonlyUint8Array
): number => {
  const len = Math.min(left.length, right.length);
  for (let index = 0; index < len; index++) {
    const diff = (left[index] ?? 0) - (right[index] ?? 0);
    if (diff !== 0) {
      return diff < 0 ? -1 : 1;
    }
  }

  if (left.length === right.length) {
    return 0;
  }
  return left.length < right.length ? -1 : 1;
};

const readOptionalU32LE = (
  bytes: Uint8Array,
  offset: number
): number | null => {
  const value = readU32LE(bytes, offset);
  return value === 0 ? null : value;
};

const readU32LE = (bytes: Uint8Array, offset: number): number =>
  new DataView(bytes.buffer, bytes.byteOffset + offset, 4).getUint32(0, true);

const readU64LE = (bytes: Uint8Array, offset: number): bigint =>
  new DataView(bytes.buffer, bytes.byteOffset + offset, 8).getBigUint64(
    0,
    true
  );

const createContext = (params: {
  fixture: SdkLocalnetFixture;
  vm: LiteSVM;
  programPaths: SdkLocalnetProgramPaths;
  signers: SdkLocalnetSigners;
  setupResults: SdkLocalnetInstructionExecution[];
}): SdkLocalnetContext => {
  const context: SdkLocalnetContext = {
    ...params,
    getSigner: (identifier) => getRequiredSigner(params.signers, identifier),
    getActor: (name) => getFixtureActor(params.fixture, name),
    getMarket: (symbol) => getFixtureMarket(params.fixture, symbol),
    buildTransaction: (instructions, options) =>
      buildSdkLocalnetTransaction(
        params.vm,
        params.signers,
        instructions,
        options
      ),
    sendInstructions: (instructions, options) =>
      sendSdkLocalnetInstructions(
        params.vm,
        params.signers,
        instructions,
        options
      ),
    sendFixtureTransaction: (transaction) =>
      sendFixtureTransaction(context, transaction),
  };

  return context;
};

const sendFixtureTransaction = async (
  context: Pick<SdkLocalnetContext, "fixture" | "sendInstructions">,
  transactionOrName: string | FixtureTransaction
): Promise<SdkLocalnetInstructionExecution[]> => {
  const transaction =
    typeof transactionOrName === "string"
      ? getSdkLocalnetTransaction(context.fixture, transactionOrName)
      : transactionOrName;
  const feePayerSeed = transaction.signerSeeds.includes("payer")
    ? "payer"
    : transaction.signerSeeds[0];

  if (!feePayerSeed) {
    throw new Error(`${transaction.name} does not list any signer seeds`);
  }

  const results: SdkLocalnetInstructionExecution[] = [];
  for (const instruction of transaction.instructions) {
    results.push(
      await context.sendInstructions([decodeFixtureInstruction(instruction)], {
        feePayerSeed,
        label: `${transaction.name}:${instruction.name}`,
      })
    );
  }

  return results;
};

const attachSdkLocalnetSigners = (
  instruction: DecodedFixtureInstruction,
  signers: SdkLocalnetSigners
): SdkLocalnetInstructionWithSigners => ({
  ...instruction,
  accounts: instruction.accounts.map((account) => {
    if (!isSignerRole(account.role)) {
      return account;
    }

    const signer = signers.byAddress.get(account.address);
    if (!signer) {
      throw new Error(
        `Missing localnet signer for ${account.address} in instruction ` +
          `${instruction.programAddress}`
      );
    }

    return {
      ...account,
      signer,
    };
  }),
});

const asSdkLocalnetInstruction = (
  instruction: SdkLocalnetInstruction
): DecodedFixtureInstruction => {
  const programAddress = (instruction as { readonly programAddress?: unknown })
    .programAddress;
  if (typeof programAddress !== "string") {
    throw new Error("SDK localnet instruction is missing a programAddress");
  }

  return {
    programAddress: address(programAddress),
    accounts: instruction.accounts,
    data: instruction.data,
  };
};

const assertSuccessfulTransaction = (
  result:
    | ReturnType<LiteSVM["sendTransaction"]>
    | ReturnType<LiteSVM["airdrop"]>,
  label: string
): TransactionMetadata => {
  if (result === null) {
    throw new Error(`LiteSVM did not return transaction metadata for ${label}`);
  }
  if (result instanceof FailedTransactionMetadata) {
    const logs = result.meta().logs().join("\n");
    throw new Error(
      `LiteSVM transaction failed for ${label}: ${result.err().toString()}\n${logs}`
    );
  }

  return result;
};

const getRequiredSigner = (
  signers: SdkLocalnetSigners,
  identifier: string
): TransactionSigner => {
  const signer =
    signers.bySeed.get(identifier) ??
    signers.byName.get(identifier) ??
    signers.byAddress.get(identifier);
  if (!signer) {
    throw new Error(`Unknown SDK localnet signer: ${identifier}`);
  }
  return signer;
};

const getFixtureActor = (
  fixture: SdkLocalnetFixture,
  name: string
): FixtureActor => {
  const actor = fixture.actors.find((candidate) => candidate.name === name);
  if (!actor) {
    throw new Error(`Unknown SDK localnet actor: ${name}`);
  }
  return actor;
};

const getFixtureMarket = (
  fixture: SdkLocalnetFixture,
  symbol: string
): FixtureMarket => {
  const market = fixture.markets.find(
    (candidate) => candidate.symbol === symbol
  );
  if (!market) {
    throw new Error(`Unknown SDK localnet market: ${symbol}`);
  }
  return market;
};

const getFixtureMint = (
  fixture: SdkLocalnetFixture,
  name: string
): FixtureMint => {
  const mint = fixture.mints.find((candidate) => candidate.name === name);
  if (!mint) {
    throw new Error(`Unknown SDK localnet mint: ${name}`);
  }
  return mint;
};

const isSignerRole = (role: AccountRole): boolean =>
  role === AccountRole.READONLY_SIGNER || role === AccountRole.WRITABLE_SIGNER;

const toLiteSvmAddress = (value: string): LiteSvmAddress =>
  address(value) as unknown as LiteSvmAddress;

const explicitProgramPaths = (
  programPaths?: Partial<SdkLocalnetProgramPaths>
): SdkLocalnetProgramPaths | null => {
  const phoenixEternal =
    programPaths?.phoenixEternal ?? process.env[ETERNAL_PROGRAM_ENV];
  const ember = programPaths?.ember ?? process.env[EMBER_PROGRAM_ENV];
  const hawkeye = programPaths?.hawkeye ?? process.env[HAWKEYE_PROGRAM_ENV];

  if (!phoenixEternal && !ember && !hawkeye) {
    return null;
  }

  return {
    phoenixEternal: resolveRequiredPath(phoenixEternal, ETERNAL_PROGRAM_ENV),
    ember: resolveRequiredPath(ember, EMBER_PROGRAM_ENV),
    ...(hawkeye ? { hawkeye: resolve(hawkeye) } : {}),
  };
};

const resolveRequiredPath = (
  value: string | undefined,
  envName: string
): string => {
  if (!value) {
    throw new Error(`Missing SDK localnet program path for ${envName}`);
  }
  return resolve(value);
};

const allProgramPathsExist = (programPaths: SdkLocalnetProgramPaths): boolean =>
  existsSync(programPaths.phoenixEternal) &&
  existsSync(programPaths.ember) &&
  (programPaths.hawkeye === undefined || existsSync(programPaths.hawkeye));

const defaultProgramPathCandidates = (
  repoRoot?: string
): SdkLocalnetProgramPaths[] => {
  const roots = new Set(
    [
      repoRoot,
      process.env[PHOENIX_REPO_ROOT_ENV],
      process.cwd(),
      resolve(process.cwd(), "../.."),
      resolve(moduleDirectory(), "../../.."),
      resolve(moduleDirectory(), "../../../.."),
    ]
      .filter((candidate): candidate is string => Boolean(candidate))
      .map((candidate) => resolve(candidate))
  );
  const candidates: SdkLocalnetProgramPaths[] = [];

  for (const root of roots) {
    candidates.push(
      {
        phoenixEternal: resolve(
          root,
          "programs/target/deploy/phoenix_eternal.so"
        ),
        ember: resolve(root, "programs/target/deploy/phoenix_ember_program.so"),
        ...optionalProgramPath(
          root,
          "programs/target/deploy/phoenix_hawkeye.so"
        ),
      },
      {
        phoenixEternal: resolve(root, "target/deploy/phoenix_eternal.so"),
        ember: resolve(root, "target/deploy/phoenix_ember_program.so"),
        ...optionalProgramPath(root, "target/deploy/phoenix_hawkeye.so"),
      },
      {
        phoenixEternal: resolve(
          root,
          "programs/eternal/target/deploy/phoenix_eternal.so"
        ),
        ember: resolve(
          root,
          "programs/ember/target/deploy/phoenix_ember_program.so"
        ),
        ...optionalProgramPath(
          root,
          "programs/phoenix-hawkeye/target/deploy/phoenix_hawkeye.so"
        ),
      }
    );
  }

  return candidates;
};

const optionalProgramPath = (
  root: string,
  path: string
): Pick<SdkLocalnetProgramPaths, "hawkeye"> => {
  const resolved = resolve(root, path);
  return existsSync(resolved) ? { hawkeye: resolved } : {};
};

const moduleDirectory = (): string => dirname(fileURLToPath(import.meta.url));
