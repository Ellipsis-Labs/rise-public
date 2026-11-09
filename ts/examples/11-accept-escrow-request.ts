/**
 * Minimal escrow-claim example: builds an AcceptEscrowRequest instruction for
 * the receiver wallet without sending it.
 *
 * Run with:
 *   bun examples/11-accept-escrow-request.ts <RECEIVER_AUTHORITY> <SENDER_AUTHORITY> [RECEIVER_WALLET]
 */

import { isSignerRole, isWritableRole } from "@solana/kit";
import { createPhoenixClient } from "@/index";

const DEFAULT_EXAMPLE_AUTHORITY = "11111111111111111111111111111111";
const receiverAuthority = process.argv[2] ?? DEFAULT_EXAMPLE_AUTHORITY;
const senderAuthority = process.argv[3] ?? DEFAULT_EXAMPLE_AUTHORITY;
const receiverWallet = process.argv[4];

async function main() {
  if (!process.argv[2] || !process.argv[3]) {
    console.log(
      "Missing authorities. Using the all-ones System Program address for instruction building.\n"
    );
  }

  const client = createPhoenixClient({
    apiUrl: process.env.PHOENIX_API_URL ?? "https://perp-api.phoenix.trade",
    rpcUrl: process.env.SOLANA_RPC_URL ?? "https://api.mainnet-beta.solana.com",
    exchangeMetadata: { stream: false },
  });

  try {
    const ix = await client.ixs.buildAcceptEscrowRequest({
      senderAuthority,
      receiverAuthority,
      receiverWallet,
    });

    console.log({
      programAddress: ix.programAddress,
      accounts: ix.accounts.map((account) => ({
        address: account.address,
        signer: isSignerRole(account.role),
        writable: isWritableRole(account.role),
      })),
      data: Buffer.from(ix.data).toString("hex"),
    });
  } finally {
    client.dispose();
  }
}

void main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
