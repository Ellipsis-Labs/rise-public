/**
 * Example: activate a referral through /v1/referral/activate-tx.
 *
 * Missing trader account: register_trader, then delegated onboarding.
 * Existing trader account: delegated onboarding only.
 *
 * Run with:
 *   PHOENIX_API_URL=http://127.0.0.1:8080 \
 *   PHOENIX_RPC_URL=<RPC_URL> \
 *   bun examples/09-referral-activation-tx.ts <REFERRAL_CODE> [TRADER_KEYPAIR_PATH]
 */

import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import process from "node:process";

import { createPhoenixClient, type Authority } from "@/index";
import { createKeyPairSignerFromBytes } from "@solana/signers";
import { createSolanaRpc, partiallySignTransaction } from "@solana/kit";

const DEFAULT_API_URL = "https://perp-api.phoenix.trade";
const DEFAULT_RPC_URL = "https://api.mainnet-beta.solana.com";
const DEFAULT_KEYPAIR_PATH = `${homedir()}/.config/solana/id.json`;
const TRADER_PDA_INDEX = 0;
const TRADER_SUBACCOUNT_INDEX = 0;
const REGISTER_TRADER_MAX_POSITIONS = 128n;

const usage = `Usage:
  PHOENIX_API_URL=http://127.0.0.1:8080 PHOENIX_RPC_URL=<RPC_URL> \\
  bun examples/09-referral-activation-tx.ts <REFERRAL_CODE> [TRADER_KEYPAIR_PATH]`;

const fail = (message: string): never => {
  console.error(message);
  console.error("");
  console.error(usage);
  process.exit(1);
};

const readKeypairBytes = (path: string): Uint8Array => {
  const raw = JSON.parse(readFileSync(path, "utf8")) as unknown;
  if (
    !Array.isArray(raw) ||
    raw.length !== 64 ||
    raw.some(
      (value) =>
        typeof value !== "number" ||
        !Number.isInteger(value) ||
        value < 0 ||
        value > 255
    )
  ) {
    throw new Error(`Keypair file must be a JSON array of 64 bytes: ${path}`);
  }
  return Uint8Array.from(raw);
};

async function main() {
  const [referralCode, traderKeypairPathArg, unexpectedArg] =
    process.argv.slice(2);
  if (!referralCode || referralCode === "-h" || referralCode === "--help") {
    console.log(usage);
    return;
  }
  if (unexpectedArg) {
    fail(`Unexpected argument: ${unexpectedArg}`);
  }

  const apiUrl = process.env.PHOENIX_API_URL ?? DEFAULT_API_URL;
  const rpcUrl =
    process.env.PHOENIX_RPC_URL ??
    process.env.SOLANA_RPC_URL ??
    DEFAULT_RPC_URL;
  const traderKeypairPath =
    traderKeypairPathArg ??
    process.env.TRADER_KEYPAIR_PATH ??
    process.env.KEYPAIR_PATH ??
    DEFAULT_KEYPAIR_PATH;

  const traderSigner = await createKeyPairSignerFromBytes(
    readKeypairBytes(traderKeypairPath)
  );
  const traderAuthority = traderSigner.address as Authority;
  const rpc = createSolanaRpc(rpcUrl);
  const client = createPhoenixClient({
    apiUrl,
    rpcUrl,
    exchangeMetadata: { stream: false },
  });

  try {
    const latestBlockhash = await rpc
      .getLatestBlockhash({ commitment: "finalized" })
      .send();

    const built = await client.api.invite().buildActivateReferralTxRequest({
      referralCode,
      traderAuthority,
      traderPdaIndex: TRADER_PDA_INDEX,
      traderSubaccountIndex: TRADER_SUBACCOUNT_INDEX,
      recentBlockhash: latestBlockhash.value.blockhash,
      lastValidBlockHeight: latestBlockhash.value.lastValidBlockHeight,
      registerTraderMaxPositions: REGISTER_TRADER_MAX_POSITIONS,
      rpc,
      signTransaction: (transaction) =>
        partiallySignTransaction([traderSigner.keyPair], transaction),
    });
    const response = await client.api
      .invite()
      .activateReferralTx(built.request);

    console.log(
      JSON.stringify(
        {
          response,
          traderAuthority,
          traderActivationState: built.traderActivationState,
          includedRegisterTrader: built.includeRegisterTrader,
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
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
});
