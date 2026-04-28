/**
 * Advanced example: cancel every active conditional-order leg for a trader.
 *
 * Run with:
 *   bun examples/05-cancel-all-conditional-orders.ts <AUTHORITY_PUBKEY> [options]
 *
 * Options:
 *   --api-url <url>                     Phoenix API URL
 *   --api-key <key>                     Phoenix API key
 *   --rpc-url <url>                     Solana RPC URL
 *   --keypair-path <path>               Signer keypair path
 *   --position-authority <pubkey>       Optional delegated signer pubkey
 *   --trader-pda-index <n>              Trader PDA index (default: 0)
 *   --trader-subaccount-index <n>       Trader subaccount index (default: 0)
 *   --isolated                          Set isIsolated=true in API requests
 *   --max-instructions-per-tx <n>       Max cancel ixs per transaction (default: 8)
 *
 * Environment fallbacks:
 *   PHOENIX_API_URL
 *   PHOENIX_API_KEY
 *   PHOENIX_RPC_URL / SOLANA_RPC_URL
 *   KEYPAIR_PATH
 */

import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import process from "node:process";

import {
  createPhoenixClient,
  fetchConditionalOrderCollection,
  fetchGlobalConfiguration,
  fetchPerpAssetMap,
  type AccountFetcherClientWithAddresses,
  type Authority,
} from "@/index";
import {
  addSignersToInstruction,
  createKeyPairSignerFromBytes,
  setTransactionMessageFeePayerSigner,
  signTransactionMessageWithSigners,
} from "@solana/signers";
import {
  address,
  appendTransactionMessageInstructions,
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  createTransactionMessage,
  fetchEncodedAccount,
  getSignatureFromTransaction,
  pipe,
  sendAndConfirmTransactionFactory,
  setTransactionMessageLifetimeUsingBlockhash,
  type Address,
} from "@solana/kit";

type CliArgs = {
  apiUrl: string;
  apiKey?: string;
  rpcUrl: string;
  keypairPath: string;
  authority: string;
  positionAuthority?: string;
  traderPdaIndex: number;
  traderSubaccountIndex: number;
  isIsolated: boolean;
  maxInstructionsPerTx: number;
};

type CancelRequest = {
  authority: string;
  positionAuthority?: string;
  traderPdaIndex: number;
  traderSubaccountIndex: number;
  isIsolated: boolean;
  symbol: string;
  conditionalOrderIndex: number;
  executionDirection: "greater_than" | "less_than";
};

const DEFAULT_API_URL = "https://perp-api.phoenix.trade";
const DEFAULT_RPC_URL = "https://api.mainnet-beta.solana.com";
const DEFAULT_KEYPAIR_PATH = `${homedir()}/.config/solana/id.json`;
const DEFAULT_MAX_INSTRUCTIONS_PER_TX = 8;

const usage = `Usage:
  bun examples/05-cancel-all-conditional-orders.ts <AUTHORITY_PUBKEY> [options]

Options:
  --api-url <url>                     Phoenix API URL
  --api-key <key>                     Phoenix API key
  --rpc-url <url>                     Solana RPC URL
  --keypair-path <path>               Signer keypair path
  --position-authority <pubkey>       Optional delegated signer pubkey
  --trader-pda-index <n>              Trader PDA index (default: 0)
  --trader-subaccount-index <n>       Trader subaccount index (default: 0)
  --isolated                          Set isIsolated=true in API requests
  --max-instructions-per-tx <n>       Max cancel ixs per transaction (default: 8)
  -h, --help                          Show this help`;

const fail = (message: string): never => {
  console.error(message);
  console.error("");
  console.error(usage);
  process.exit(1);
};

const formatError = (error: unknown): string =>
  error instanceof Error ? error.message : String(error);

const parseInteger = (value: string, flag: string): number => {
  const parsed = Number.parseInt(value, 10);
  if (!Number.isInteger(parsed) || parsed < 0) {
    fail(`Invalid value for ${flag}: ${value}`);
  }
  return parsed;
};

const parsePositiveInteger = (value: string, flag: string): number => {
  const parsed = parseInteger(value, flag);
  if (parsed <= 0) {
    fail(`${flag} must be greater than 0`);
  }
  return parsed;
};

const parseAddress = <T extends Address>(value: string, label: string): T => {
  try {
    return address(value) as T;
  } catch (error) {
    fail(`Invalid ${label}: ${formatError(error)}`);
  }
};

const readKeypairBytes = (path: string): Uint8Array => {
  const raw = JSON.parse(readFileSync(path, "utf8")) as unknown;
  if (!Array.isArray(raw)) {
    throw new Error(`Keypair file must be a JSON array of bytes: ${path}`);
  }

  const bytes = Uint8Array.from(
    raw.map((value) => {
      if (
        typeof value !== "number" ||
        !Number.isInteger(value) ||
        value < 0 ||
        value > 255
      ) {
        throw new Error(`Invalid keypair byte in ${path}: ${String(value)}`);
      }
      return value;
    })
  );

  if (bytes.length !== 64) {
    throw new Error(
      `Keypair file must contain 64 bytes, found ${bytes.length}: ${path}`
    );
  }

  return bytes;
};

const toRpcWebSocketUrl = (rpcUrl: string): string => {
  if (rpcUrl.startsWith("https://")) {
    return `wss://${rpcUrl.slice("https://".length)}`;
  }
  if (rpcUrl.startsWith("http://")) {
    return `ws://${rpcUrl.slice("http://".length)}`;
  }
  return rpcUrl;
};

const chunk = <T>(items: readonly T[], size: number): T[][] => {
  const chunks: T[][] = [];
  for (let index = 0; index < items.length; index += size) {
    chunks.push(items.slice(index, index + size));
  }
  return chunks;
};

const executionDirectionsForOrder = (
  order: Awaited<
    ReturnType<typeof fetchConditionalOrderCollection>
  >["orders"][number]
): CancelRequest["executionDirection"][] => {
  const directions: CancelRequest["executionDirection"][] = [];
  if (order.greaterTriggerOrder.isActive) {
    directions.push("greater_than");
  }
  if (order.lessTriggerOrder.isActive) {
    directions.push("less_than");
  }
  return directions;
};

const parseArgs = (argv: string[]): CliArgs => {
  let apiUrl = process.env.PHOENIX_API_URL ?? DEFAULT_API_URL;
  let apiKey = process.env.PHOENIX_API_KEY;
  let rpcUrl =
    process.env.PHOENIX_RPC_URL ??
    process.env.SOLANA_RPC_URL ??
    DEFAULT_RPC_URL;
  let keypairPath = process.env.KEYPAIR_PATH ?? DEFAULT_KEYPAIR_PATH;
  let positionAuthority: string | undefined;
  let traderPdaIndex = 0;
  let traderSubaccountIndex = 0;
  let isIsolated = false;
  let maxInstructionsPerTx = DEFAULT_MAX_INSTRUCTIONS_PER_TX;
  const positional: string[] = [];

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    switch (arg) {
      case "--api-url":
        apiUrl = argv[++index] ?? fail("Missing value for --api-url");
        break;
      case "--api-key":
        apiKey = argv[++index] ?? fail("Missing value for --api-key");
        break;
      case "--rpc-url":
        rpcUrl = argv[++index] ?? fail("Missing value for --rpc-url");
        break;
      case "--keypair-path":
        keypairPath = argv[++index] ?? fail("Missing value for --keypair-path");
        break;
      case "--position-authority":
        positionAuthority =
          argv[++index] ?? fail("Missing value for --position-authority");
        break;
      case "--trader-pda-index":
        traderPdaIndex = parseInteger(
          argv[++index] ?? fail("Missing value for --trader-pda-index"),
          "--trader-pda-index"
        );
        break;
      case "--trader-subaccount-index":
        traderSubaccountIndex = parseInteger(
          argv[++index] ?? fail("Missing value for --trader-subaccount-index"),
          "--trader-subaccount-index"
        );
        break;
      case "--isolated":
        isIsolated = true;
        break;
      case "--max-instructions-per-tx":
        maxInstructionsPerTx = parsePositiveInteger(
          argv[++index] ?? fail("Missing value for --max-instructions-per-tx"),
          "--max-instructions-per-tx"
        );
        break;
      case "-h":
      case "--help":
        console.log(usage);
        process.exit(0);
      default:
        if (arg.startsWith("-")) {
          fail(`Unknown argument: ${arg}`);
        }
        positional.push(arg);
    }
  }

  if (positional.length !== 1) {
    fail("Expected exactly one positional argument: <AUTHORITY_PUBKEY>");
  }

  return {
    apiUrl,
    apiKey,
    rpcUrl,
    keypairPath,
    authority: positional[0],
    positionAuthority,
    traderPdaIndex,
    traderSubaccountIndex,
    isIsolated,
    maxInstructionsPerTx,
  };
};

const buildCancelRequests = async (
  client: ReturnType<typeof createPhoenixClient>,
  rpcUrl: string,
  args: CliArgs
): Promise<{
  traderAccount: Address;
  conditionalOrdersAddress: Address;
  activeOrderCount: number;
  requests: CancelRequest[];
}> => {
  const authority = parseAddress<Authority>(args.authority, "authority");
  const rpc = createSolanaRpc(rpcUrl);
  const traderAccount = await client.pda.getTraderAddress({
    authority,
    traderPdaIndex: args.traderPdaIndex,
    subaccountIndex: args.traderSubaccountIndex,
  });
  const conditionalOrdersAddress = await client.pda.getConditionalOrdersAddress(
    {
      traderAccount,
    }
  );

  const conditionalOrdersAccount = await fetchEncodedAccount(
    rpc,
    conditionalOrdersAddress
  );
  if (
    !conditionalOrdersAccount.exists ||
    !conditionalOrdersAccount.data ||
    conditionalOrdersAccount.data.length === 0
  ) {
    return {
      traderAccount,
      conditionalOrdersAddress,
      activeOrderCount: 0,
      requests: [],
    };
  }

  const globalConfigurationAddress =
    await client.pda.getGlobalConfigurationAddress();
  const accountClient: AccountFetcherClientWithAddresses = {
    fetchAccount: async (accountAddress: Address) => {
      const account = await fetchEncodedAccount(rpc, accountAddress);
      if (!account.exists || !account.data || account.data.length === 0) {
        throw new Error(`Account does not exist: ${accountAddress}`);
      }
      return { data: account.data };
    },
    _accountCache: new Map<string, unknown>(),
    _cacheEnabled: true,
    addresses: {
      globalConfigurationAddress,
    },
  };

  const conditionalOrders = await fetchConditionalOrderCollection({
    client: accountClient,
    address: conditionalOrdersAddress,
    skipCache: true,
  });
  if (conditionalOrders.activeOrderIndices.length === 0) {
    return {
      traderAccount,
      conditionalOrdersAddress,
      activeOrderCount: 0,
      requests: [],
    };
  }

  const globalConfiguration = await fetchGlobalConfiguration({
    client: accountClient,
  });
  const perpAssetMap = await fetchPerpAssetMap({
    client: accountClient,
    address: globalConfiguration.perpAssetMapKey,
  });

  const assetIdToSymbol = new Map<number, string>();
  for (const entry of perpAssetMap.metadata.entries) {
    assetIdToSymbol.set(entry.value.staticMarketParams.assetId, entry.key);
  }

  const requests: CancelRequest[] = [];
  for (const orderIndex of conditionalOrders.activeOrderIndices) {
    const order = conditionalOrders.orders[orderIndex];
    const symbol = assetIdToSymbol.get(order.assetId);
    if (!symbol) {
      throw new Error(`Missing market symbol for assetId ${order.assetId}`);
    }

    for (const executionDirection of executionDirectionsForOrder(order)) {
      requests.push({
        authority: args.authority,
        positionAuthority: args.positionAuthority,
        traderPdaIndex: args.traderPdaIndex,
        traderSubaccountIndex: args.traderSubaccountIndex,
        isIsolated: args.isIsolated,
        symbol,
        conditionalOrderIndex: orderIndex,
        executionDirection,
      });
    }
  }

  return {
    traderAccount,
    conditionalOrdersAddress,
    activeOrderCount: conditionalOrders.activeOrderIndices.length,
    requests,
  };
};

const buildCancelInstructions = async (
  client: ReturnType<typeof createPhoenixClient>,
  requests: readonly CancelRequest[],
  signer: Awaited<ReturnType<typeof createKeyPairSignerFromBytes>>
) => {
  const instructions = [];

  for (const request of requests) {
    const built = await client.api.orders().cancelConditionalOrder(request);
    if (built.length !== 1) {
      throw new Error(
        `Expected exactly 1 instruction, received ${built.length} for ${request.symbol} index ${request.conditionalOrderIndex}`
      );
    }
    instructions.push(addSignersToInstruction([signer], built[0]));
  }

  return instructions;
};

const sendInstructionBatches = async (params: {
  instructions: Awaited<ReturnType<typeof buildCancelInstructions>>;
  signer: Awaited<ReturnType<typeof createKeyPairSignerFromBytes>>;
  rpcUrl: string;
  maxInstructionsPerTx: number;
}): Promise<string[]> => {
  const rpc = createSolanaRpc(params.rpcUrl);
  const rpcSubscriptions = createSolanaRpcSubscriptions(
    toRpcWebSocketUrl(params.rpcUrl)
  );
  const sendAndConfirm = sendAndConfirmTransactionFactory({
    rpc,
    rpcSubscriptions,
  });

  const batches = chunk(params.instructions, params.maxInstructionsPerTx);
  const signatures: string[] = [];

  for (const [batchIndex, batch] of batches.entries()) {
    console.log(
      `Submitting batch ${batchIndex + 1}/${batches.length} (${batch.length} instruction(s))...`
    );

    const latestBlockhash = await rpc
      .getLatestBlockhash({ commitment: "finalized" })
      .send();
    const transactionMessage = pipe(
      createTransactionMessage({ version: 0 }),
      (tx) => setTransactionMessageFeePayerSigner(params.signer, tx),
      (tx) =>
        setTransactionMessageLifetimeUsingBlockhash(latestBlockhash.value, tx),
      (tx) => appendTransactionMessageInstructions(batch, tx)
    );
    const signedTransaction =
      await signTransactionMessageWithSigners(transactionMessage);
    const signature = getSignatureFromTransaction(signedTransaction);

    await sendAndConfirm(
      {
        ...signedTransaction,
        lifetimeConstraint: {
          lastValidBlockHeight: latestBlockhash.value.lastValidBlockHeight,
        },
      },
      {
        commitment: "confirmed",
      }
    );

    console.log(`  Confirmed ${signature}`);
    signatures.push(signature);
  }

  return signatures;
};

async function main() {
  const argv = process.argv.slice(2);
  if (argv.length === 0) {
    console.log(
      "This example submits live conditional-order cancel transactions."
    );
    console.log(
      "Pass an explicit <AUTHORITY_PUBKEY> and a matching signer before running it.\n"
    );
    console.log(usage);
    return;
  }

  const args = parseArgs(argv);
  const authority = parseAddress<Authority>(args.authority, "authority");
  const positionAuthority = args.positionAuthority
    ? parseAddress<Authority>(args.positionAuthority, "position-authority")
    : undefined;
  const signer = await createKeyPairSignerFromBytes(
    readKeypairBytes(args.keypairPath)
  );
  const requiredSigner = positionAuthority ?? authority;

  if (signer.address !== requiredSigner) {
    throw new Error(
      `Signer ${signer.address} does not match the required instruction signer ${requiredSigner}`
    );
  }

  const client = createPhoenixClient({
    apiUrl: args.apiUrl,
    apiKey: args.apiKey,
  });

  try {
    const {
      traderAccount,
      conditionalOrdersAddress,
      activeOrderCount,
      requests,
    } = await buildCancelRequests(client, args.rpcUrl, args);

    if (activeOrderCount === 0) {
      console.log("No active conditional orders found for this trader.");
      console.log({
        traderAccount,
        conditionalOrdersAddress,
      });
      return;
    }

    console.log(
      `Found ${activeOrderCount} active conditional orders across ${requests.length} active legs.`
    );

    if (requests.length === 0) {
      console.log("No active conditional-order legs to cancel.");
      return;
    }

    const instructions = await buildCancelInstructions(
      client,
      requests,
      signer
    );
    const signatures = await sendInstructionBatches({
      instructions,
      signer,
      rpcUrl: args.rpcUrl,
      maxInstructionsPerTx: args.maxInstructionsPerTx,
    });

    console.log(
      JSON.stringify(
        {
          traderAccount,
          conditionalOrdersAddress,
          activeOrderCount,
          instructionCount: instructions.length,
          signatures,
        },
        null,
        2
      )
    );
  } finally {
    client.dispose();
  }
}

void main().catch((error) => {
  console.error(formatError(error));
  process.exitCode = 1;
});
