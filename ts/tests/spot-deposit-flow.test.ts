import { AccountRole, address } from "@solana/kit";
import { describe, expect, it } from "vitest";
import { buildSpotDepositFlow } from "@/flows";
import { clientPhoenixInstructionAddresses } from "@/core/constants";
import { buildReallocTraderIx } from "@/core/ixBuilders/ReallocTrader";
import { buildSyncSpotIx } from "@/core/ixBuilders/Spot";
import {
  getAssociatedTokenAccountAddress,
  getPhoenixTraderSubaccountAddress,
  getPhoenixTraderTokenAccountAddress,
  getPhoenixTraderWalletAddress,
} from "@/pdas";
import type { PhoenixExchangeMetadata } from "@/exchange-cache";
import type { PhoenixInstructionClient } from "@/core/clientTypes";
import type {
  ActiveTraderBufferAddressArray,
  Authority,
  GlobalTraderIndexAddressArray,
  MintAddress,
  PerpAssetMapAddress,
  PhoenixProgramAddress,
} from "@/primitives";

const phoenixProgramAddress = address(
  "phDEVv4w6BcfkLrLNeXr8HhhgQxnxziVGXpGPcaadMf"
) as PhoenixProgramAddress;
const authority = address(
  "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
) as Authority;
const mint = address(
  "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
) as MintAddress;

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

describe("buildSpotDepositFlow", () => {
  it("reserves capacity, creates the custody ATA, transfers into it, then SyncSpot", async () => {
    const flow = await buildSpotDepositFlow(
      { authority, mint, amount: 1_000_000n },
      client
    );

    const expectedTraderAccount = await getPhoenixTraderSubaccountAddress({
      authority,
      traderPdaIndex: 0,
      subaccountIndex: 0,
      phoenixProgramAddress,
    });
    const expectedWallet = await getPhoenixTraderWalletAddress(
      expectedTraderAccount,
      phoenixProgramAddress
    );
    const expectedCustodyAta = await getAssociatedTokenAccountAddress(
      expectedWallet,
      mint
    );
    const expectedSourceAta = await getPhoenixTraderTokenAccountAddress(
      authority,
      mint
    );

    expect(flow.traderAccount).toBe(expectedTraderAccount);
    expect(flow.custodyTokenAccount).toBe(expectedCustodyAta);
    expect(flow.instructions).toEqual([
      flow.named.reallocTrader,
      flow.named.createCustodyAta,
      flow.named.transferTokens,
      flow.named.syncSpot,
    ]);

    const { reallocTrader, createCustodyAta, transferTokens, syncSpot } =
      flow.named;
    // A first deposit inserts a new trader map entry, so capacity is reserved
    // first, as in the native SOL flow.
    expect(reallocTrader).toEqual(
      buildReallocTraderIx({
        ...clientPhoenixInstructionAddresses(client),
        payer: authority,
        trader: authority,
        traderAccount: expectedTraderAccount,
      })
    );
    // Idempotent ATA create for the wallet-PDA custody account, paid by the
    // authority when not sponsored.
    expect(createCustodyAta.accounts[0]).toEqual({
      address: authority,
      role: AccountRole.WRITABLE_SIGNER,
    });
    expect(createCustodyAta.accounts[1]?.address).toBe(expectedCustodyAta);
    expect(createCustodyAta.accounts[2]?.address).toBe(expectedWallet);

    // SPL transfer from the authority's ATA into the custody ATA, signed by
    // the authority.
    expect(transferTokens.accounts.map((account) => account.address)).toEqual([
      expectedSourceAta,
      expectedCustodyAta,
      authority,
    ]);
    expect(transferTokens.accounts[2]?.role).toBe(AccountRole.READONLY_SIGNER);

    // The sync must be exactly the standalone builder's output for the same
    // trader, mint, and exchange accounts, so account order is checked too.
    expect(syncSpot).toEqual(
      await buildSyncSpotIx({
        ...clientPhoenixInstructionAddresses(client),
        traderAccount: expectedTraderAccount,
        mint,
        perpAssetMap: exchangeSnapshot.exchange
          .perpAssetMap as PerpAssetMapAddress,
        globalTraderIndex: exchangeSnapshot.exchange
          .globalTraderIndex as unknown as GlobalTraderIndexAddressArray,
        activeTraderBuffer: exchangeSnapshot.exchange
          .activeTraderBuffer as unknown as ActiveTraderBufferAddressArray,
      })
    );
  });

  it("honors an explicit source token account and rejects a zero amount", async () => {
    const sourceTokenAccount = await getAssociatedTokenAccountAddress(
      address("9n4nbM75f5Ui33ZbPYXn59EwSgE8CGsHtAeTH5YFeJ9E"),
      mint
    );
    const flow = await buildSpotDepositFlow(
      { authority, mint, amount: 5n, sourceTokenAccount },
      client
    );
    expect(flow.named.transferTokens.accounts[0]?.address).toBe(
      sourceTokenAccount
    );

    await expect(
      buildSpotDepositFlow({ authority, mint, amount: 0n }, client)
    ).rejects.toThrow(/greater than 0/);
  });

  it("rejects sponsored callers that have not confirmed capacity preparation", async () => {
    // Exercise an untyped JavaScript caller at the public runtime boundary.
    await expect(
      Reflect.apply(buildSpotDepositFlow, undefined, [
        {
          authority,
          mint,
          amount: 1n,
          feePayer: authority,
          sponsorshipToken: "token",
          userPubkey: authority,
        },
        client,
      ])
    ).rejects.toThrow("require traderCapacityPrepared: true");
  });
});
