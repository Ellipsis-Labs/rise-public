/**
 * Example: onboard a trader with a delegated trader-onboarder keypair.
 *
 * Run with:
 *   bun examples/07-onboard-trader-delegated.ts <TRADER_AUTHORITY> [options]
 *
 * Options:
 *   --api-url <url>                     Phoenix API URL
 *   --rpc-url <url>                     Solana RPC URL
 *   --onboarder-keypair-path <path>     Trader-onboarder keypair path
 *   --trader-pda-index <n>              Trader PDA index (default: 0)
 *   --trader-subaccount-index <n>       Trader subaccount index (default: 0)
 *   --max-positions <n>                 Max positions if registering first (default: 128 for subaccount 0, 1 otherwise)
 *
 * Environment fallbacks:
 *   PHOENIX_API_URL
 *   PHOENIX_RPC_URL / SOLANA_RPC_URL
 *   TRADER_ONBOARDER_KEYPAIR_PATH / KEYPAIR_PATH
 */

import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import process from "node:process";

import {
  buildRegisterTraderIx,
  createPhoenixClient,
  fetchPermission,
  type AccountFetcherClient,
  type Authority,
  type TraderAddress,
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
  rpcUrl: string;
  onboarderKeypairPath: string;
  traderAuthority: string;
  traderPdaIndex: number;
  traderSubaccountIndex: number;
  maxPositions: number;
};

const DEFAULT_API_URL = "https://perp-api.phoenix.trade";
const DEFAULT_RPC_URL = "https://api.mainnet-beta.solana.com";
const DEFAULT_KEYPAIR_PATH = `${homedir()}/.config/solana/id.json`;

const usage = `Usage:
  bun examples/07-onboard-trader-delegated.ts <TRADER_AUTHORITY> [options]

Options:
  --api-url <url>                     Phoenix API URL
  --rpc-url <url>                     Solana RPC URL
  --onboarder-keypair-path <path>     Trader-onboarder keypair path
  --trader-pda-index <n>              Trader PDA index (default: 0)
  --trader-subaccount-index <n>       Trader subaccount index (default: 0)
  --max-positions <n>                 Max positions if registering first (default: 128 for subaccount 0, 1 otherwise)
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

const parseArgs = (argv: string[]): CliArgs => {
  let apiUrl = process.env.PHOENIX_API_URL ?? DEFAULT_API_URL;
  let rpcUrl =
    process.env.PHOENIX_RPC_URL ??
    process.env.SOLANA_RPC_URL ??
    DEFAULT_RPC_URL;
  let onboarderKeypairPath =
    process.env.TRADER_ONBOARDER_KEYPAIR_PATH ??
    process.env.KEYPAIR_PATH ??
    DEFAULT_KEYPAIR_PATH;
  let traderPdaIndex = 0;
  let traderSubaccountIndex = 0;
  let maxPositions: number | undefined;
  const positional: string[] = [];

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    switch (arg) {
      case "--api-url":
        apiUrl = argv[++index] ?? fail("Missing value for --api-url");
        break;
      case "--rpc-url":
        rpcUrl = argv[++index] ?? fail("Missing value for --rpc-url");
        break;
      case "--onboarder-keypair-path":
        onboarderKeypairPath =
          argv[++index] ?? fail("Missing value for --onboarder-keypair-path");
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
      case "--max-positions":
        maxPositions = parsePositiveInteger(
          argv[++index] ?? fail("Missing value for --max-positions"),
          "--max-positions"
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
    fail("Expected exactly one positional argument: <TRADER_AUTHORITY>");
  }
  if (traderPdaIndex > 255) {
    fail("--trader-pda-index must be between 0 and 255");
  }
  if (traderSubaccountIndex > 255) {
    fail("--trader-subaccount-index must be between 0 and 255");
  }
  const registrationMaxPositions =
    maxPositions ?? (traderSubaccountIndex === 0 ? 128 : 1);
  if (registrationMaxPositions > 128) {
    fail("--max-positions cannot exceed 128");
  }
  if (traderSubaccountIndex > 0 && registrationMaxPositions !== 1) {
    fail(
      "--max-positions must be 1 when --trader-subaccount-index is greater than 0"
    );
  }

  return {
    apiUrl,
    rpcUrl,
    onboarderKeypairPath,
    traderAuthority: positional[0],
    traderPdaIndex,
    traderSubaccountIndex,
    maxPositions: registrationMaxPositions,
  };
};

const createAccountClient = (
  rpc: ReturnType<typeof createSolanaRpc>
): AccountFetcherClient => ({
  fetchAccount: async (accountAddress: Address) => {
    const account = await fetchEncodedAccount(rpc, accountAddress);
    if (!account.exists || !account.data || account.data.length === 0) {
      throw new Error(`Account does not exist: ${accountAddress}`);
    }
    return { data: account.data };
  },
  _cacheEnabled: false,
});

const accountExists = async (
  rpc: ReturnType<typeof createSolanaRpc>,
  accountAddress: Address
): Promise<boolean> => {
  const account = await fetchEncodedAccount(rpc, accountAddress);
  return account.exists && !!account.data && account.data.length > 0;
};

const sendInstructions = async (params: {
  instructions: ReturnType<typeof addSignersToInstruction>[];
  signer: Awaited<ReturnType<typeof createKeyPairSignerFromBytes>>;
  rpcUrl: string;
}): Promise<string> => {
  const rpc = createSolanaRpc(params.rpcUrl);
  const rpcSubscriptions = createSolanaRpcSubscriptions(
    toRpcWebSocketUrl(params.rpcUrl)
  );
  const sendAndConfirm = sendAndConfirmTransactionFactory({
    rpc,
    rpcSubscriptions,
  });
  const latestBlockhash = await rpc
    .getLatestBlockhash({ commitment: "finalized" })
    .send();
  const transactionMessage = pipe(
    createTransactionMessage({ version: 0 }),
    (tx) => setTransactionMessageFeePayerSigner(params.signer, tx),
    (tx) =>
      setTransactionMessageLifetimeUsingBlockhash(latestBlockhash.value, tx),
    (tx) => appendTransactionMessageInstructions(params.instructions, tx)
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
    { commitment: "confirmed" }
  );

  return signature;
};

async function main() {
  const argv = process.argv.slice(2);
  if (argv.length === 0) {
    console.log("This example submits a live trader-onboarding transaction.");
    console.log(
      "Pass a target trader authority and an onboarder keypair with trader-onboarding delegation permission.\n"
    );
    console.log(usage);
    return;
  }

  const args = parseArgs(argv);
  const traderAuthority = parseAddress<Authority>(
    args.traderAuthority,
    "trader authority"
  );
  const onboarderSigner = await createKeyPairSignerFromBytes(
    readKeypairBytes(args.onboarderKeypairPath)
  );
  const onboarderAuthority = onboarderSigner.address as Authority;
  const rpc = createSolanaRpc(args.rpcUrl);
  const client = createPhoenixClient({
    apiUrl: args.apiUrl,
    rpcUrl: args.rpcUrl,
    exchangeMetadata: { stream: false },
  });

  try {
    const snapshot = await client.exchange.ready();
    const riskAuthority = parseAddress<Address>(
      snapshot.exchange.currentAuthorities.riskAuthority,
      "risk authority"
    );
    const [traderAccount, permissionAccount] = await Promise.all([
      client.pda.getTraderAddress({
        authority: traderAuthority,
        traderPdaIndex: args.traderPdaIndex,
        subaccountIndex: args.traderSubaccountIndex,
      }),
      client.pda.getPermissionAddress({
        permissionAuthority: riskAuthority,
        delegatedKey: onboarderAuthority,
        phoenixProgramAddress: snapshot.exchange.programId,
      }),
    ]);

    const permission = await fetchPermission({
      client: createAccountClient(rpc),
      address: permissionAccount,
      skipCache: true,
    });
    if (permission.permissionAuthority !== riskAuthority) {
      throw new Error(
        `Permission authority mismatch: expected ${riskAuthority}, got ${permission.permissionAuthority}`
      );
    }
    if (permission.delegatedKey !== onboarderAuthority) {
      throw new Error(
        `Permission delegated key mismatch: expected ${onboarderAuthority}, got ${permission.delegatedKey}`
      );
    }

    const instructions = [];
    const traderAlreadyRegistered = await accountExists(rpc, traderAccount);
    if (!traderAlreadyRegistered) {
      const [logAuthorityAddress, globalConfigurationAddress] =
        await Promise.all([
          client.pda.getLogAuthorityAddress(),
          client.pda.getGlobalConfigurationAddress(),
        ]);
      instructions.push(
        buildRegisterTraderIx({
          programAddress: client.pda.getProgramAddress(),
          logAuthorityAddress,
          globalConfigurationAddress,
          payer: onboarderAuthority,
          trader: traderAuthority,
          traderAccount: traderAccount as TraderAddress,
          maxPositions: BigInt(args.maxPositions),
          traderPdaIndex: args.traderPdaIndex,
          traderSubaccountIndex: args.traderSubaccountIndex,
        })
      );
    }

    instructions.push(
      await client.ixs.buildOnboardTraderDelegated({
        authority: onboarderAuthority,
        traderAuthority,
        permissionAccount,
        traderPdaIndex: args.traderPdaIndex,
        traderSubaccountIndex: args.traderSubaccountIndex,
      })
    );

    const signedInstructions = instructions.map((instruction) =>
      addSignersToInstruction([onboarderSigner], instruction)
    );
    const signature = await sendInstructions({
      instructions: signedInstructions,
      signer: onboarderSigner,
      rpcUrl: args.rpcUrl,
    });

    console.log(
      JSON.stringify(
        {
          signature,
          onboarderAuthority,
          traderAuthority,
          traderAccount,
          permissionAccount,
          riskAuthority,
          traderAlreadyRegistered,
          permission: permission.permission.toString(),
          allowedSignerActions: permission.allowedSignerActions.toString(),
          instructionCount: instructions.length,
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
