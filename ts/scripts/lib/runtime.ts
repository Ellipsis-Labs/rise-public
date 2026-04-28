import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import process from "node:process";

import {
  addSignersToInstruction,
  createKeyPairSignerFromBytes,
  setTransactionMessageFeePayerSigner,
  signTransactionMessageWithSigners,
} from "@solana/signers";
import {
  appendTransactionMessageInstructions,
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  createTransactionMessage,
  getBase64EncodedWireTransaction,
  getSignatureFromTransaction,
  pipe,
  sendAndConfirmTransactionFactory,
  setTransactionMessageLifetimeUsingBlockhash,
  type Instruction,
} from "@solana/kit";

export const DEFAULT_API_URL = "https://perp-api.phoenix.trade";
export const DEFAULT_RPC_URL = "https://api.mainnet-beta.solana.com";
export const DEFAULT_KEYPAIR_PATH = `${homedir()}/.config/solana/id.json`;

export type LocalSigner = Awaited<
  ReturnType<typeof createKeyPairSignerFromBytes>
>;

export type SendInstructionsResult =
  | {
      kind: "dry-run";
      simulationError: unknown;
      logs: readonly string[];
    }
  | {
      kind: "sent";
      signature: string;
      explorerUrl: string;
    };

export const fail = (message: string, usage?: string): never => {
  console.error(message);
  if (usage) {
    console.error("");
    console.error(usage);
  }
  process.exit(1);
};

export const formatErrorMessage = (error: unknown): string => {
  if (error instanceof Error && error.message) {
    return error.message;
  }
  return String(error);
};

export const stringifyForOutput = (value: unknown): string =>
  JSON.stringify(
    value,
    (_key, nestedValue) =>
      typeof nestedValue === "bigint" ? nestedValue.toString() : nestedValue,
    2
  );

export const parseInteger = (value: string, flag: string): number => {
  const parsed = Number.parseInt(value, 10);
  if (!Number.isInteger(parsed) || parsed < 0) {
    throw new Error(`Invalid value for ${flag}: ${value}`);
  }
  return parsed;
};

export const parsePositiveInteger = (value: string, flag: string): number => {
  const parsed = parseInteger(value, flag);
  if (parsed <= 0) {
    throw new Error(`${flag} must be greater than 0`);
  }
  return parsed;
};

export const readKeypairBytes = (path: string): Uint8Array => {
  const raw = readFileSync(path, "utf8");
  const json: unknown = JSON.parse(raw);
  if (!Array.isArray(json)) {
    throw new Error(`Keypair file must be a JSON array of bytes: ${path}`);
  }

  const bytes = Uint8Array.from(
    json.map((value) => {
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

export const loadKeypairSigner = async (path: string): Promise<LocalSigner> =>
  createKeyPairSignerFromBytes(readKeypairBytes(path));

export const normalizeSymbol = (symbol: string) => symbol.trim().toUpperCase();

export const resolveMarketSymbol = (
  availableSymbols: readonly string[],
  requestedSymbol: string
): string => {
  const normalized = normalizeSymbol(requestedSymbol);

  const exact = availableSymbols.find(
    (symbol) => normalizeSymbol(symbol) === normalized
  );
  if (exact) {
    return exact;
  }

  const perp = availableSymbols.find(
    (symbol) => normalizeSymbol(symbol) === `${normalized}-PERP`
  );
  if (perp) {
    return perp;
  }

  throw new Error(
    `Unknown market symbol '${requestedSymbol}'. Available symbols: ${availableSymbols.join(", ")}`
  );
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

export const getExplorerUrl = (signature: string, rpcUrl: string): string => {
  const normalized = rpcUrl.toLowerCase();
  if (normalized.includes("devnet")) {
    return `https://explorer.solana.com/tx/${signature}?cluster=devnet`;
  }
  if (normalized.includes("testnet")) {
    return `https://explorer.solana.com/tx/${signature}?cluster=testnet`;
  }
  return `https://explorer.solana.com/tx/${signature}`;
};

export const sendInstructions = async (params: {
  instructions: readonly Instruction[];
  rpcUrl: string;
  signer: LocalSigner;
  dryRun?: boolean;
}): Promise<SendInstructionsResult> => {
  if (params.instructions.length === 0) {
    throw new Error("No instructions provided");
  }

  const rpc = createSolanaRpc(params.rpcUrl);
  const signedInstructions = params.instructions.map((instruction) =>
    addSignersToInstruction([params.signer], instruction)
  );
  const latestBlockhash = await rpc
    .getLatestBlockhash({ commitment: "finalized" })
    .send();
  const transactionMessage = pipe(
    createTransactionMessage({ version: 0 }),
    (tx) => setTransactionMessageFeePayerSigner(params.signer, tx),
    (tx) =>
      setTransactionMessageLifetimeUsingBlockhash(latestBlockhash.value, tx),
    (tx) => appendTransactionMessageInstructions(signedInstructions, tx)
  );
  const signedTransaction =
    await signTransactionMessageWithSigners(transactionMessage);

  if (params.dryRun ?? process.env.PHOENIX_DRY_RUN !== undefined) {
    const simulation = await rpc
      .simulateTransaction(getBase64EncodedWireTransaction(signedTransaction), {
        encoding: "base64",
        sigVerify: true,
        commitment: "confirmed",
      })
      .send();

    return {
      kind: "dry-run",
      simulationError: simulation.value.err,
      logs: simulation.value.logs ?? [],
    };
  }

  const rpcSubscriptions = createSolanaRpcSubscriptions(
    toRpcWebSocketUrl(params.rpcUrl)
  );
  const sendAndConfirm = sendAndConfirmTransactionFactory({
    rpc,
    rpcSubscriptions,
  });
  const signature = getSignatureFromTransaction(signedTransaction);
  const transactionForConfirmation = {
    ...signedTransaction,
    lifetimeConstraint: {
      lastValidBlockHeight: latestBlockhash.value.lastValidBlockHeight,
    },
  };
  try {
    await sendAndConfirm(transactionForConfirmation, {
      commitment: "confirmed",
    });
  } catch (error) {
    const sendErrorMessage =
      error instanceof Error ? error.message : String(error);
    try {
      const simulation = await rpc
        .simulateTransaction(
          getBase64EncodedWireTransaction(signedTransaction),
          {
            encoding: "base64",
            sigVerify: true,
            commitment: "confirmed",
          }
        )
        .send();
      const renderedLogs = (simulation.value.logs ?? []).join("\n");
      return Promise.reject(
        new Error(
          [
            "Transaction simulation failed",
            `Simulation error: ${stringifyForOutput(simulation.value.err)}`,
            renderedLogs ? "Simulation logs:" : "",
            renderedLogs,
          ]
            .filter((line) => line.length > 0)
            .join("\n")
        )
      );
    } catch (simulationError) {
      const simulationErrorMessage =
        simulationError instanceof Error
          ? simulationError.message
          : String(simulationError);
      throw new Error(
        [
          `Transaction send failed: ${sendErrorMessage}`,
          `Follow-up simulation failed: ${simulationErrorMessage}`,
        ].join("\n")
      );
    }
  }

  return {
    kind: "sent",
    signature,
    explorerUrl: getExplorerUrl(signature, params.rpcUrl),
  };
};
