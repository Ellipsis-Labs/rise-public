import { AccountRole, address } from "@solana/kit";
import { describe, expect, it } from "vitest";
import { buildNativeSolDepositFlow } from "@/flows";
import { DISCRIMINANTS } from "@/core/discriminants";
import { getPhoenixTraderSubaccountAddress } from "@/pdas";
import type { PhoenixExchangeMetadata } from "@/exchange-cache";
import type { PhoenixInstructionClient } from "@/core/clientTypes";
import type { Authority, PhoenixProgramAddress } from "@/primitives";

const phoenixProgramAddress = address(
  "phDEVv4w6BcfkLrLNeXr8HhhgQxnxziVGXpGPcaadMf"
) as PhoenixProgramAddress;
const authority = address(
  "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
) as Authority;
const feePayer = address(
  "9n4nbM75f5Ui33ZbPYXn59EwSgE8CGsHtAeTH5YFeJ9E"
) as Authority;

const exchangeSnapshot = {
  markets: [{ symbol: "SOL-PERP" }],
  exchange: {
    canonicalMint: "canonical-mint",
    perpAssetMap: "perp-asset-map",
    globalTraderIndex: ["gti-0"],
    activeTraderBuffer: ["atb-0"],
    withdrawQueue: "withdraw-queue",
  },
};

const exchangeMetadata = {
  ready: async () => exchangeSnapshot,
  snapshot: () => exchangeSnapshot,
} as unknown as PhoenixExchangeMetadata;

const client = {
  addresses: {
    phoenixProgramAddress,
    logAuthorityAddress: "log-authority",
    globalConfigurationAddress: "global-config",
  },
  fetchAccount: async () => ({ data: new Uint8Array() }),
  exchange: exchangeMetadata,
} as unknown as PhoenixInstructionClient;

describe("buildNativeSolDepositFlow", () => {
  it("emits the System transfer into the trader account, then SyncNative", async () => {
    const flow = await buildNativeSolDepositFlow(
      { authority, lamports: 250_000_000n },
      client
    );

    const expectedTraderAccount = await getPhoenixTraderSubaccountAddress({
      authority,
      traderPdaIndex: 0,
      subaccountIndex: 0,
      phoenixProgramAddress,
    });

    expect(flow.traderAccount).toBe(expectedTraderAccount);
    expect(flow.instructions).toEqual([
      flow.named.transferSol,
      flow.named.syncNative,
    ]);

    const { transferSol, syncNative } = flow.named;
    expect(transferSol.programAddress).toBe("11111111111111111111111111111111");
    expect(transferSol.accounts).toEqual([
      { address: authority, role: AccountRole.WRITABLE_SIGNER },
      { address: expectedTraderAccount, role: AccountRole.WRITABLE },
    ]);

    expect(syncNative.programAddress).toBe(phoenixProgramAddress);
    expect(new Uint8Array(syncNative.data)).toEqual(
      new Uint8Array(DISCRIMINANTS.SYNC_NATIVE)
    );
    // The sync must target the same account the transfer funds — this is the
    // pairing the sponsorship validator checks.
    expect(
      syncNative.accounts.some(
        (account) => account.address === expectedTraderAccount
      )
    ).toBe(true);
  });

  it("derives the trader account from the pda and subaccount indices", async () => {
    const flow = await buildNativeSolDepositFlow(
      { authority, lamports: 1n, traderPdaIndex: 2, subaccountIndex: 3 },
      client
    );

    const expectedTraderAccount = await getPhoenixTraderSubaccountAddress({
      authority,
      traderPdaIndex: 2,
      subaccountIndex: 3,
      phoenixProgramAddress,
    });

    expect(flow.traderAccount).toBe(expectedTraderAccount);
    expect(flow.named.transferSol.accounts[1]?.address).toBe(
      expectedTraderAccount
    );
  });

  it("keeps the authority as the transfer source under sponsorship", async () => {
    const flow = await buildNativeSolDepositFlow(
      {
        authority,
        lamports: 1n,
        feePayer,
        sponsorshipToken: "token",
        userPubkey: authority,
      },
      client
    );

    // Sponsorship pays the network fee; the deposited lamports always leave
    // the trader's own wallet, and that wallet signs the transfer.
    expect(flow.named.transferSol.accounts[0]).toEqual({
      address: authority,
      role: AccountRole.WRITABLE_SIGNER,
    });
  });

  it("rejects a non-positive amount", async () => {
    await expect(
      buildNativeSolDepositFlow({ authority, lamports: 0n }, client)
    ).rejects.toThrow("Deposit amount must be greater than 0");
  });
});
