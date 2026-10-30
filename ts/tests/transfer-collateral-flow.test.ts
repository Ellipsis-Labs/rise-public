import { AccountRole, address } from "@solana/kit";
import { describe, expect, it } from "vitest";
import { buildTransferCollateralFlow } from "@/flows";
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
const positionAuthority = address(
  "9n4nbM75f5Ui33ZbPYXn59EwSgE8CGsHtAeTH5YFeJ9E"
) as Authority;
const sponsor = address("11111111111111111111111111111112") as Authority;

const exchangeSnapshot = {
  markets: [{ symbol: "SOL-PERP" }],
  exchange: {
    canonicalMint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    perpAssetMap: "SysvarRent111111111111111111111111111111111",
    globalTraderIndex: ["SysvarC1ock11111111111111111111111111111111"],
    activeTraderBuffer: ["Sysvar1nstructions1111111111111111111111111"],
    withdrawQueue: "SysvarS1otHashes111111111111111111111111111",
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

const childAccount = () =>
  getPhoenixTraderSubaccountAddress({
    authority,
    traderPdaIndex: 0,
    subaccountIndex: 1,
    phoenixProgramAddress,
  });

const signers = (ix: {
  accounts?: readonly { address: string; role: AccountRole }[];
}): string[] =>
  (ix.accounts ?? [])
    .filter(
      (meta) =>
        meta.role === AccountRole.WRITABLE_SIGNER ||
        meta.role === AccountRole.READONLY_SIGNER
    )
    .map((meta) => meta.address);

describe("buildTransferCollateralFlow", () => {
  it("reserves a slot on the isolated subaccount before funding it", async () => {
    const flow = await buildTransferCollateralFlow(
      {
        authority,
        srcSubaccountIndex: 0,
        dstSubaccountIndex: 1,
        amount: 1_000_000n,
      },
      client
    );

    const { reallocTrader, transferCollateral } = flow.named;
    if (!reallocTrader) throw new Error("Expected ReallocTrader");
    expect(flow.instructions).toEqual([reallocTrader, transferCollateral]);
    expect([...reallocTrader.data]).toEqual([...DISCRIMINANTS.REALLOC_TRADER]);
    expect(reallocTrader.accounts?.[5]?.address).toBe(await childAccount());
    expect(signers(reallocTrader)).toEqual([authority]);
  });

  it("defaults the rent payer to the position authority", async () => {
    const flow = await buildTransferCollateralFlow(
      {
        authority,
        positionAuthority,
        srcSubaccountIndex: 0,
        dstSubaccountIndex: 1,
        amount: 1_000_000n,
      },
      client
    );

    const { reallocTrader, transferCollateral } = flow.named;
    if (!reallocTrader) throw new Error("Expected ReallocTrader");
    // The owner wallet is referenced but never asked to sign.
    expect(signers(reallocTrader)).toEqual([positionAuthority]);
    expect(signers(transferCollateral)).not.toContain(authority);
  });

  it("uses an explicit payer for rent", async () => {
    const flow = await buildTransferCollateralFlow(
      {
        authority,
        positionAuthority,
        srcSubaccountIndex: 0,
        dstSubaccountIndex: 1,
        amount: 1_000_000n,
        payer: sponsor,
      },
      client
    );

    const { reallocTrader } = flow.named;
    if (!reallocTrader) throw new Error("Expected ReallocTrader");
    expect(signers(reallocTrader)).toEqual([sponsor]);
  });

  it.each([
    { srcSubaccountIndex: 1, dstSubaccountIndex: 0 },
    { srcSubaccountIndex: 1, dstSubaccountIndex: 2 },
  ])(
    "sends only the transfer from $srcSubaccountIndex to $dstSubaccountIndex",
    async ({ srcSubaccountIndex, dstSubaccountIndex }) => {
      const flow = await buildTransferCollateralFlow(
        {
          authority,
          srcSubaccountIndex,
          dstSubaccountIndex,
          amount: 1_000_000n,
        },
        client
      );

      expect(flow.named.reallocTrader).toBeUndefined();
      expect(flow.instructions).toEqual([flow.named.transferCollateral]);
    }
  );
});
