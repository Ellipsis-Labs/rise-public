/**
 * Example: register and onboard a trader without a referral code.
 *
 * This uses /v1/exchange/build-register-ixs to fetch the register/onboard
 * instructions, signs the user-controlled signer slots, then submits the
 * transaction to /v1/exchange/send-register-ixs. The API validates, signs with
 * the configured Phoenix onboarding keypair, simulates, and sends the tx.
 *
 * Run with:
 *   PHOENIX_API_URL=http://127.0.0.1:8080 PHOENIX_RPC_URL=<RPC_URL> \
 *   bun examples/10-builder-onboarding-tx.ts --trader-keypair-path ~/.config/solana/id.json
 */

import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import process from "node:process";

import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import {
  createPhoenixClient,
  type Authority,
  type RegisterIxInstruction,
} from "@/index";
import { createKeyPairSignerFromBytes } from "@solana/signers";
import {
  AccountRole,
  address,
  appendTransactionMessageInstructions,
  compileTransaction,
  createSolanaRpc,
  createTransactionMessage,
  getBase64EncodedWireTransaction,
  partiallySignTransaction,
  pipe,
  setTransactionMessageFeePayer,
  setTransactionMessageLifetimeUsingBlockhash,
  type Address,
  type Blockhash,
} from "@solana/kit";

type CliArgs = {
  apiUrl: string;
  rpcUrl: string;
  traderKeypairPath: string;
  feePayerKeypairPath?: string;
  maxPositions: number;
  recentBlockhash?: string;
  lastValidBlockHeight?: bigint;
};

type KeyPairSigner = Awaited<ReturnType<typeof createKeyPairSignerFromBytes>>;

const DEFAULT_API_URL = "https://perp-api.phoenix.trade";
const DEFAULT_RPC_URL = "https://api.mainnet-beta.solana.com";
const DEFAULT_KEYPAIR_PATH = `${homedir()}/.config/solana/id.json`;
const DEFAULT_MAX_POSITIONS = 128;

const usage = `Usage:
  bun examples/10-builder-onboarding-tx.ts [options]

Options:
  --api-url <url>                    Phoenix API URL
  --rpc-url <url>                    Solana RPC URL
  --trader-keypair-path <path>       Trader authority keypair path
  --fee-payer-keypair-path <path>    Fee-payer keypair path (default: trader keypair)
  --max-positions <n>                Max positions when registering (32-128, default: 128)
  --recent-blockhash <blockhash>     Optional blockhash to use for the transaction
  --last-valid-block-height <n>      Required with --recent-blockhash when submitting
  -h, --help                         Show this help

Environment:
  PHOENIX_API_URL
  PHOENIX_RPC_URL / SOLANA_RPC_URL
  TRADER_KEYPAIR_PATH / KEYPAIR_PATH
  FEE_PAYER_KEYPAIR_PATH`;

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
  if (!Number.isInteger(parsed)) {
    fail(`Invalid value for ${flag}: ${value}`);
  }
  return parsed;
};

const parseMaxPositions = (value: string): number => {
  const maxPositions = parseInteger(value, "--max-positions");
  if (maxPositions < 32 || maxPositions > DEFAULT_MAX_POSITIONS) {
    fail("--max-positions must be between 32 and 128");
  }
  return maxPositions;
};

const parseLastValidBlockHeight = (value: string): bigint => {
  try {
    const parsed = BigInt(value);
    if (parsed < 0n) {
      fail("--last-valid-block-height must be non-negative");
    }
    return parsed;
  } catch {
    return fail(`Invalid value for --last-valid-block-height: ${value}`);
  }
};

const parseAddress = <T extends Address>(value: string, label: string): T => {
  try {
    return address(value) as T;
  } catch (error) {
    return fail(`Invalid ${label}: ${formatError(error)}`);
  }
};

const parseArgs = (argv: string[]): CliArgs => {
  let apiUrl = process.env.PHOENIX_API_URL ?? DEFAULT_API_URL;
  let rpcUrl =
    process.env.PHOENIX_RPC_URL ??
    process.env.SOLANA_RPC_URL ??
    DEFAULT_RPC_URL;
  let traderKeypairPath =
    process.env.TRADER_KEYPAIR_PATH ??
    process.env.KEYPAIR_PATH ??
    DEFAULT_KEYPAIR_PATH;
  let feePayerKeypairPath = process.env.FEE_PAYER_KEYPAIR_PATH;
  let maxPositions = DEFAULT_MAX_POSITIONS;
  let recentBlockhash: string | undefined;
  let lastValidBlockHeight: bigint | undefined;

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    switch (arg) {
      case "--api-url":
        apiUrl = argv[++index] ?? fail("Missing value for --api-url");
        break;
      case "--rpc-url":
        rpcUrl = argv[++index] ?? fail("Missing value for --rpc-url");
        break;
      case "--trader-keypair-path":
        traderKeypairPath =
          argv[++index] ?? fail("Missing value for --trader-keypair-path");
        break;
      case "--fee-payer-keypair-path":
        feePayerKeypairPath =
          argv[++index] ?? fail("Missing value for --fee-payer-keypair-path");
        break;
      case "--max-positions":
        maxPositions = parseMaxPositions(
          argv[++index] ?? fail("Missing value for --max-positions")
        );
        break;
      case "--recent-blockhash":
        recentBlockhash =
          argv[++index] ?? fail("Missing value for --recent-blockhash");
        break;
      case "--last-valid-block-height":
        lastValidBlockHeight = parseLastValidBlockHeight(
          argv[++index] ?? fail("Missing value for --last-valid-block-height")
        );
        break;
      case "-h":
      case "--help":
        console.log(usage);
        process.exit(0);
      default:
        fail(`Unknown argument: ${arg}`);
    }
  }

  if (recentBlockhash && lastValidBlockHeight === undefined) {
    fail("--last-valid-block-height is required with --recent-blockhash");
  }

  return {
    apiUrl,
    rpcUrl,
    traderKeypairPath,
    feePayerKeypairPath,
    maxPositions,
    recentBlockhash,
    lastValidBlockHeight,
  };
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

const toInstruction = (
  apiInstruction: RegisterIxInstruction
): InstructionsWithAccountsAndData => ({
  programAddress: address(apiInstruction.programId),
  accounts: apiInstruction.keys.map((account) => ({
    address: address(account.pubkey),
    role: account.isSigner
      ? account.isWritable
        ? AccountRole.WRITABLE_SIGNER
        : AccountRole.READONLY_SIGNER
      : account.isWritable
        ? AccountRole.WRITABLE
        : AccountRole.READONLY,
  })),
  data: Uint8Array.from(apiInstruction.data),
});

const uniqueSignerKeyPairs = (
  traderSigner: KeyPairSigner,
  feePayerSigner: KeyPairSigner
): CryptoKeyPair[] =>
  traderSigner.address === feePayerSigner.address
    ? [traderSigner.keyPair]
    : [traderSigner.keyPair, feePayerSigner.keyPair];

async function main() {
  const argv = process.argv.slice(2);
  if (argv.length === 0) {
    console.log("This example submits a live builder onboarding transaction.");
    console.log(
      "Pass --trader-keypair-path, or set TRADER_KEYPAIR_PATH, to register and onboard that trader without a referral code.\n"
    );
    console.log(usage);
    return;
  }

  const args = parseArgs(argv);
  const rpc = createSolanaRpc(args.rpcUrl);
  const client = createPhoenixClient({
    apiUrl: args.apiUrl,
    rpcUrl: args.rpcUrl,
    exchangeMetadata: { stream: false },
  });

  try {
    const traderSigner = await createKeyPairSignerFromBytes(
      readKeypairBytes(args.traderKeypairPath)
    );
    const feePayerSigner = args.feePayerKeypairPath
      ? await createKeyPairSignerFromBytes(
          readKeypairBytes(args.feePayerKeypairPath)
        )
      : traderSigner;
    const traderAuthority = parseAddress<Authority>(
      traderSigner.address,
      "trader authority"
    );
    const txFeePayer = parseAddress<Authority>(
      feePayerSigner.address,
      "transaction fee payer"
    );
    const latestBlockhash = args.recentBlockhash
      ? {
          blockhash: args.recentBlockhash,
          lastValidBlockHeight: args.lastValidBlockHeight!,
        }
      : (await rpc.getLatestBlockhash({ commitment: "finalized" }).send())
          .value;

    const response = await client.api.exchange().buildRegisterIxs({
      traderAuthority,
      txFeePayer,
      maxPositions: args.maxPositions,
    });
    const transactionMessage = pipe(
      createTransactionMessage({ version: 0 }),
      (tx) => setTransactionMessageFeePayer(txFeePayer, tx),
      (tx) =>
        setTransactionMessageLifetimeUsingBlockhash(
          {
            blockhash: latestBlockhash.blockhash as Blockhash,
            lastValidBlockHeight: latestBlockhash.lastValidBlockHeight,
          },
          tx
        ),
      (tx) =>
        appendTransactionMessageInstructions(
          response.instructions.map(toInstruction),
          tx
        )
    );
    const partialTransaction = compileTransaction(transactionMessage);
    const signedTransaction = await partiallySignTransaction(
      uniqueSignerKeyPairs(traderSigner, feePayerSigner),
      partialTransaction
    );
    const signedTransactionBase64 =
      getBase64EncodedWireTransaction(signedTransaction);
    const submitted = await client.api.exchange().sendRegisterIxs({
      transaction: signedTransactionBase64,
      traderAuthority,
      txFeePayer,
      maxPositions: args.maxPositions,
      traderPdaIndex: 0,
      traderSubaccountIndex: 0,
    });

    console.log(
      JSON.stringify(
        {
          signature: submitted.signature,
          signedTransaction: signedTransactionBase64,
          traderAuthority,
          txFeePayer,
          traderPda: submitted.traderPda,
          traderOnboarder: submitted.traderOnboarder,
          recentBlockhash: latestBlockhash.blockhash,
          maxPositions: submitted.maxPositions,
          includedRegisterTrader: submitted.includeRegisterTrader,
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
