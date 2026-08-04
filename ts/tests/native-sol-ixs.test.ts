import { describe, expect, it } from "vitest";
import { AccountRole, type Address, type Instruction } from "@solana/kit";
import { DISCRIMINANTS } from "@/core/discriminants";
import {
  MAX_PACKED_EXTERNAL_ACCOUNTS,
  SwapDirection,
  buildLiquidateNativeSolIx,
  buildSwapNativeIx,
  buildSyncNativeIx,
  buildTransferNativeSolFromChildToParentIx,
  buildTransferNativeSolIx,
  buildWithdrawNativeSolIx,
  packVenueInstructions,
  packedAccountMetaState,
} from "@/core/ixBuilders/NativeSol";
import { buildTransferCollateralIx } from "@/core/ixBuilders/TransferCollateral";
import {
  getPhoenixGlobalVaultAddress,
  getPhoenixNativeSolAuthorityAddress,
} from "@/pdas";
import type { WithdrawNativeSolAction } from "@/core/ixBuilders/NativeSol";

const testAddress = (value: string): Address => value as Address;

const addresses = {
  programAddress: testAddress("phoenix-program"),
  logAuthorityAddress: testAddress("log-authority"),
  globalConfigurationAddress: testAddress("global-config"),
};

const traderIndex = {
  globalTraderIndex: [testAddress("gti-header"), testAddress("gti-arena")],
  activeTraderBuffer: [testAddress("atb-header"), testAddress("atb-arena")],
};

const bytes = (data: Uint8Array): number[] => Array.from(data);
const discriminant = (name: keyof typeof DISCRIMINANTS): number[] =>
  Array.from(new Uint8Array(DISCRIMINANTS[name]));

describe("sync native ix", () => {
  it("takes no data and writes the global configuration", () => {
    const ix = buildSyncNativeIx({
      ...addresses,
      ...traderIndex,
      traderAccount: testAddress("trader-account"),
    });

    expect(bytes(ix.data)).toEqual(discriminant("SYNC_NATIVE"));
    expect(ix.accounts[2]?.address).toBe("global-config");
    expect(ix.accounts[2]?.role).toBe(AccountRole.WRITABLE);
    expect(ix.accounts[3]?.address).toBe("trader-account");
    // Permissionless: nothing signs.
    expect(
      ix.accounts.every(
        (account) =>
          account.role !== AccountRole.READONLY_SIGNER &&
          account.role !== AccountRole.WRITABLE_SIGNER
      )
    ).toBe(true);
  });
});

describe("withdraw native sol ix", () => {
  const build = (action: WithdrawNativeSolAction) =>
    buildWithdrawNativeSolIx({
      ...addresses,
      ...traderIndex,
      trader: testAddress("trader-wallet"),
      traderAccount: testAddress("trader-account"),
      perpAssetMap: testAddress("perp-asset-map"),
      destination: testAddress("destination"),
      withdrawQueue: testAddress("withdraw-queue"),
      action,
    });

  it("places the withdraw queue before the arenas", () => {
    const ix = build({ kind: "withoutExcess", lamports: 7n });

    expect(ix.accounts.map((account) => account.address)).toEqual([
      "phoenix-program",
      "log-authority",
      "global-config",
      "trader-wallet",
      "trader-account",
      "perp-asset-map",
      "destination",
      "withdraw-queue",
      "gti-header",
      "gti-arena",
      "atb-header",
      "atb-arena",
    ]);
    expect(ix.accounts[3]?.role).toBe(AccountRole.READONLY_SIGNER);
    expect(ix.accounts[7]?.role).toBe(AccountRole.WRITABLE);
  });

  it("encodes each action variant", () => {
    expect(bytes(build({ kind: "allExcess" }).data)).toEqual([
      ...discriminant("WITHDRAW_NATIVE_SOL"),
      0,
    ]);
    expect(bytes(build({ kind: "withExcess", lamports: 7n }).data)).toEqual([
      ...discriminant("WITHDRAW_NATIVE_SOL"),
      1,
      7,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
    ]);
    expect(bytes(build({ kind: "withoutExcess", lamports: 7n }).data)).toEqual([
      ...discriminant("WITHDRAW_NATIVE_SOL"),
      2,
      7,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
    ]);
  });

  it("rejects a zero amount but allows an excess sweep", () => {
    expect(() => build({ kind: "withoutExcess", lamports: 0n })).toThrow(
      /greater than 0/
    );
    expect(() => build({ kind: "allExcess" })).not.toThrow();
  });

  it("rejects the trader account as a destination", () => {
    expect(() =>
      buildWithdrawNativeSolIx({
        ...addresses,
        ...traderIndex,
        trader: testAddress("trader-wallet"),
        traderAccount: testAddress("trader-account"),
        perpAssetMap: testAddress("perp-asset-map"),
        destination: testAddress("trader-account"),
        withdrawQueue: testAddress("withdraw-queue"),
        action: { kind: "withoutExcess", lamports: 7n },
      })
    ).toThrow(/other than the trader account/);
  });
});

describe("transfer native sol ix", () => {
  const transferArgs = {
    ...addresses,
    ...traderIndex,
    trader: testAddress("trader-wallet"),
    srcTraderAccount: testAddress("src-trader"),
    dstTraderAccount: testAddress("dst-trader"),
    perpAssetMap: testAddress("perp-asset-map"),
  };

  /**
   * `TransferNativeSol` reuses the quote transfer's account group verbatim.
   * Locking that in catches an accidental divergence in either builder.
   */
  it("matches the quote transfer account layout", () => {
    const native = buildTransferNativeSolIx({ ...transferArgs, lamports: 9n });
    const quote = buildTransferCollateralIx({ ...transferArgs, amount: 9n });

    expect(native.accounts).toEqual(quote.accounts);
    // Same amount encoding, different discriminant.
    expect(bytes(native.data).slice(8)).toEqual(bytes(quote.data).slice(8));
    expect(bytes(native.data).slice(0, 8)).not.toEqual(
      bytes(quote.data).slice(0, 8)
    );
  });

  it("appends an optional permission account", () => {
    const ix = buildTransferNativeSolIx({
      ...transferArgs,
      permissionAccount: testAddress("permission"),
      lamports: 9n,
    });

    expect(ix.accounts.at(-1)?.address).toBe("permission");
    expect(ix.accounts.at(-1)?.role).toBe(AccountRole.WRITABLE);
  });

  it("rejects a zero amount", () => {
    expect(() =>
      buildTransferNativeSolIx({ ...transferArgs, lamports: 0n })
    ).toThrow(/greater than 0/);
  });
});

describe("child to parent native sol sweep", () => {
  const sweepArgs = {
    ...addresses,
    ...traderIndex,
    trader: testAddress("trader-wallet"),
    childTraderAccount: testAddress("child-trader"),
    parentTraderAccount: testAddress("parent-trader"),
    perpAssetMap: testAddress("perp-asset-map"),
  };

  it("can drop the trader signature for cranks", () => {
    expect(
      buildTransferNativeSolFromChildToParentIx({
        ...sweepArgs,
        traderSigns: true,
      }).accounts[3]?.role
    ).toBe(AccountRole.READONLY_SIGNER);
    expect(
      buildTransferNativeSolFromChildToParentIx({
        ...sweepArgs,
        traderSigns: false,
      }).accounts[3]?.role
    ).toBe(AccountRole.READONLY);
    // Defaults to signing, matching the quote sweep builder.
    expect(
      buildTransferNativeSolFromChildToParentIx(sweepArgs).accounts[3]?.role
    ).toBe(AccountRole.READONLY_SIGNER);
  });

  it("takes no instruction data", () => {
    expect(
      bytes(buildTransferNativeSolFromChildToParentIx(sweepArgs).data)
    ).toEqual(discriminant("TRANSFER_NATIVE_SOL_FROM_CHILD_TO_PARENT"));
  });
});

describe("venue instruction packing", () => {
  const venueInstruction = (
    accounts: Instruction["accounts"],
    data: Uint8Array
  ): Instruction =>
    ({ programAddress: "venue-program", accounts, data }) as Instruction;

  it("deduplicates accounts and unions their bits", () => {
    const packed = packVenueInstructions("signer", [
      venueInstruction(
        [
          { address: testAddress("shared"), role: AccountRole.READONLY },
          {
            address: testAddress("signer"),
            role: AccountRole.WRITABLE_SIGNER,
          },
        ],
        Uint8Array.from([1, 2, 3])
      ),
      venueInstruction(
        [{ address: testAddress("shared"), role: AccountRole.WRITABLE }],
        Uint8Array.from([4])
      ),
    ]);

    // venue-program, shared, signer — the program id is itself an account.
    expect(packed.accounts.map((account) => account.address)).toEqual([
      "venue-program",
      "shared",
      "signer",
    ]);
    // Read-only in the first instruction, writable in the second.
    expect(packed.accounts[1]?.role).toBe(AccountRole.WRITABLE);
    expect(packed.accounts[2]?.role).toBe(AccountRole.WRITABLE_SIGNER);

    expect(packed.instructions[0]?.programIdIndex).toBe(0);
    expect(packed.instructions[0]?.accountMetas).toEqual([
      { index: 1, isSigner: false, isWritable: false },
      { index: 2, isSigner: true, isWritable: true },
    ]);
  });

  it("rejects a foreign signer", () => {
    expect(() =>
      packVenueInstructions("signer", [
        venueInstruction(
          [
            {
              address: testAddress("someone-else"),
              role: AccountRole.READONLY_SIGNER,
            },
          ],
          new Uint8Array()
        ),
      ])
    ).toThrow(/Only the swap signer/);
  });

  it("rejects more accounts than the encoding can address", () => {
    const accounts = Array.from(
      { length: MAX_PACKED_EXTERNAL_ACCOUNTS },
      (_, i) => ({
        address: testAddress(`account-${i}`),
        role: AccountRole.READONLY,
      })
    );

    // The program id occupies one slot, so a full complement of distinct
    // accounts overflows.
    expect(() =>
      packVenueInstructions("signer", [
        venueInstruction(accounts, new Uint8Array()),
      ])
    ).toThrow(/cannot address/);
  });
});

describe("liquidate native sol ix", () => {
  const programAddress = testAddress(
    "EtrnLzgbS7nMMy5fbD42kXiUzGg8XQzJ972Xtk1cjWih"
  );
  const mint = testAddress("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
  const signer = testAddress("GPgADQrhzGoUgLqxsZMKvSpwcLaJFVTq6gEixKhmcwpm");

  it("encodes the direct authority path and shortfall trader count", async () => {
    const ix = await buildLiquidateNativeSolIx({
      ...addresses,
      programAddress,
      ...traderIndex,
      signer,
      liquidateeAccount: testAddress("liquidatee"),
      mint,
      perpAssetMap: testAddress("perp-asset-map"),
      signerQuoteTokenAccount: testAddress("signer-quote"),
      withdrawQueue: testAddress("withdraw-queue"),
      maxNativeSolAmount: 7n,
      extraTraderAccounts: [testAddress("other-trader")],
      venue: packVenueInstructions(signer, []),
    });

    expect(bytes(ix.data)).toEqual([
      ...discriminant("LIQUIDATE_NATIVE_SOL"),
      7,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      1,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
    ]);
    // Full account order, mirroring the on-chain
    // `LiquidateNativeSolInstructionGroup` behind the log prefix. A wrong
    // position here is a wrong account on a liquidation.
    const globalVault = await getPhoenixGlobalVaultAddress(
      mint,
      programAddress
    );
    const nativeSolAuthority =
      await getPhoenixNativeSolAuthorityAddress(programAddress);
    expect(
      ix.accounts.map((account) => [account.address, account.role])
    ).toEqual([
      [programAddress, AccountRole.READONLY],
      ["log-authority", AccountRole.READONLY],
      ["global-config", AccountRole.WRITABLE],
      ["perp-asset-map", AccountRole.WRITABLE],
      [globalVault, AccountRole.WRITABLE],
      [ix.accounts[5]?.address, AccountRole.READONLY], // SPL token program
      [nativeSolAuthority, AccountRole.READONLY],
      [signer, AccountRole.WRITABLE_SIGNER],
      [signer, AccountRole.WRITABLE], // direct path: permission = signer
      ["signer-quote", AccountRole.WRITABLE],
      ["liquidatee", AccountRole.WRITABLE],
      ["withdraw-queue", AccountRole.WRITABLE],
      ["gti-header", AccountRole.WRITABLE],
      ["gti-arena", AccountRole.WRITABLE],
      ["atb-header", AccountRole.WRITABLE],
      ["atb-arena", AccountRole.WRITABLE],
      ["other-trader", AccountRole.WRITABLE],
    ]);
    expect(ix.accounts[5]?.address).toBe(
      "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
    );
  });

  it("uses a delegated permission account when supplied", async () => {
    const ix = await buildLiquidateNativeSolIx({
      ...addresses,
      programAddress,
      ...traderIndex,
      signer,
      permissionAccount: testAddress("permission"),
      liquidateeAccount: testAddress("liquidatee"),
      mint,
      perpAssetMap: testAddress("perp-asset-map"),
      signerQuoteTokenAccount: testAddress("signer-quote"),
      withdrawQueue: testAddress("withdraw-queue"),
      maxNativeSolAmount: 7n,
      venue: packVenueInstructions(signer, []),
    });

    expect(ix.accounts[8]?.address).toBe("permission");
    expect(ix.accounts[8]?.role).toBe(AccountRole.WRITABLE);
  });
});

describe("swap native ix", () => {
  // The swap builder derives real PDAs, so these must be valid base58
  // addresses rather than the readable placeholders the other builders accept.
  const REAL_PROGRAM = testAddress(
    "EtrnLzgbS7nMMy5fbD42kXiUzGg8XQzJ972Xtk1cjWih"
  );
  const REAL_MINT = testAddress("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
  const REAL_SIGNER = testAddress(
    "GPgADQrhzGoUgLqxsZMKvSpwcLaJFVTq6gEixKhmcwpm"
  );

  const swapArgs = {
    ...addresses,
    programAddress: REAL_PROGRAM,
    ...traderIndex,
    signer: REAL_SIGNER,
    traderAccount: testAddress("trader-account"),
    mint: REAL_MINT,
    perpAssetMap: testAddress("perp-asset-map"),
    signerQuoteTokenAccount: testAddress("signer-quote-ata"),
    withdrawQueue: testAddress("withdraw-queue"),
    direction: SwapDirection.Sell,
    amountIn: 1_000n,
    minAmountOut: 900n,
    venue: packVenueInstructions(REAL_SIGNER, []),
  };

  it("requires explicit slippage protection", async () => {
    await expect(
      buildSwapNativeIx({
        ...swapArgs,
        minAmountOut: undefined as never,
      })
    ).rejects.toThrow(/minAmountOut is required/);
    await expect(
      buildSwapNativeIx({ ...swapArgs, minAmountOut: 0n })
    ).rejects.toThrow(/greater than 0/);
  });

  it("rejects a zero input amount", async () => {
    await expect(
      buildSwapNativeIx({ ...swapArgs, amountIn: 0n })
    ).rejects.toThrow(/greater than 0/);
  });

  it("encodes direction, amounts and packed venue instructions", async () => {
    const venue = packVenueInstructions(REAL_SIGNER, [
      {
        programAddress: "venue-program",
        accounts: [
          {
            address: testAddress("venue-account"),
            role: AccountRole.WRITABLE,
          },
        ],
        data: Uint8Array.from([9, 9]),
      } as Instruction,
    ]);

    const ix = await buildSwapNativeIx({
      ...swapArgs,
      direction: SwapDirection.Buy,
      venue,
    });

    expect(bytes(ix.data)).toEqual([
      ...discriminant("SWAP_NATIVE"),
      1, // Buy
      232,
      3,
      0,
      0,
      0,
      0,
      0,
      0, // amountIn = 1000
      132,
      3,
      0,
      0,
      0,
      0,
      0,
      0, // minAmountOut = 900
      1,
      0,
      0,
      0, // one packed instruction
      0, // program id index
      2,
      0,
      0,
      0, // data length
      9,
      9,
      1,
      0,
      0,
      0, // one account meta
      packedAccountMetaState({ index: 1, isSigner: false, isWritable: true }),
    ]);

    // Venue accounts trail the protocol accounts.
    expect(ix.accounts.at(-2)?.address).toBe("venue-program");
    expect(ix.accounts.at(-1)?.address).toBe("venue-account");
  });

  it("marks the signer writable and includes the program accounts", async () => {
    const ix = await buildSwapNativeIx(swapArgs);

    expect(ix.accounts[2]?.address).toBe("global-config");
    expect(ix.accounts[2]?.role).toBe(AccountRole.WRITABLE);
    expect(ix.accounts[7]?.role).toBe(AccountRole.READONLY); // native sol authority
    expect(ix.accounts[8]?.address).toBe(REAL_SIGNER);
    expect(ix.accounts[8]?.role).toBe(AccountRole.WRITABLE_SIGNER);
  });

  it("encodes an unprotected swap as a zero min amount out", async () => {
    const ix = await buildSwapNativeIx({
      ...swapArgs,
      minAmountOut: "unprotected",
    });

    // discriminant (8) + direction (1) + amountIn (8) => minAmountOut.
    expect(bytes(ix.data).slice(17, 25)).toEqual([0, 0, 0, 0, 0, 0, 0, 0]);
  });
});
