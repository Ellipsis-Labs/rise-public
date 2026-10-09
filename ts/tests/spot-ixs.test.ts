/**
 * Cross-language golden tests for the SPL spot collateral instruction
 * builders.
 *
 * The fixture in `fixtures/spot-ix-goldens.json` is emitted by
 * `eternal-cli sdk-fixtures export` from the eternal program SDK constructors;
 * the export also fails unless the Rust `phoenix-rise-ix` spot builders match
 * those constructors byte-for-byte. These tests assert the TS builders produce
 * identical accounts (address, signer, writable) and instruction data, PDAs
 * included.
 */

import { AccountRole, type Address } from "@solana/kit";
import { describe, expect, it } from "vitest";
import {
  SwapDirection,
  packVenueInstructions,
  type PackedVenueInstructions,
} from "@/core/ixBuilders/NativeSol";
import {
  buildLiquidateSpotIx,
  buildSwapSpotWithSolIx,
  buildSwapSpotWithSpotIx,
  buildSwapSpotWithUsdcIx,
  buildSyncSpotIx,
  buildTransferSpotFromChildToParentIx,
  buildTransferSpotIx,
  buildWithdrawSpotIx,
  type SpotIx,
} from "@/core/ixBuilders/Spot";
import type {
  ActiveTraderBufferAddressArray,
  Authority,
  GlobalTraderIndexAddressArray,
  MintAddress,
  PerpAssetMapAddress,
  TokenAccountAddress,
  TraderAddress,
  WithdrawQueueAddress,
} from "@/primitives";
import goldens from "./fixtures/spot-ix-goldens.json";

interface GoldenInstruction {
  programAddress: string;
  accounts: { address: string; isSigner: boolean; isWritable: boolean }[];
  data: number[];
}

const k = goldens.keys;
const brand = <T extends Address>(value: string): T => value as T;

// The golden emitter runs with the prod address set; the first two accounts of
// every instruction are the program and log authority, and the golden data was
// generated against the prod global configuration.
const addresses = {
  programAddress: brand<Address>(goldens.instructions.syncSpot.programAddress),
  logAuthorityAddress: brand<Address>(
    goldens.instructions.syncSpot.accounts[1]!.address
  ),
  globalConfigurationAddress: brand<Address>(
    goldens.instructions.syncSpot.accounts[2]!.address
  ),
};

const traderIndex = {
  globalTraderIndex: [
    brand(k.k20),
    brand(k.k21),
  ] as unknown as GlobalTraderIndexAddressArray,
  activeTraderBuffer: [
    brand(k.k30),
    brand(k.k31),
  ] as unknown as ActiveTraderBufferAddressArray,
};

const roleOf = (meta: {
  isSigner: boolean;
  isWritable: boolean;
}): AccountRole => {
  if (meta.isSigner) {
    return meta.isWritable
      ? AccountRole.WRITABLE_SIGNER
      : AccountRole.READONLY_SIGNER;
  }
  return meta.isWritable ? AccountRole.WRITABLE : AccountRole.READONLY;
};

const expectMatchesGolden = (ix: SpotIx, golden: GoldenInstruction) => {
  expect(ix.programAddress).toBe(golden.programAddress);
  expect(
    ix.accounts.map((account) => ({
      address: account.address,
      role: account.role,
    }))
  ).toEqual(
    golden.accounts.map((account) => ({
      address: account.address,
      role: roleOf(account),
    }))
  );
  expect(Array.from(ix.data)).toEqual(golden.data);
};

// Mirrors the venue instruction the golden emitter packs: program k40,
// accounts [k41 writable, k9 writable-signer], data [1, 2, 3].
const venue = (): PackedVenueInstructions =>
  packVenueInstructions(k.k9, [
    {
      programAddress: brand<Address>(k.k40),
      accounts: [
        { address: brand<Address>(k.k41), role: AccountRole.WRITABLE },
        { address: brand<Address>(k.k9), role: AccountRole.WRITABLE_SIGNER },
      ],
      data: new Uint8Array([1, 2, 3]),
    },
  ]);

describe("spot ix golden parity with the eternal SDK", () => {
  it("sync spot", async () => {
    const ix = await buildSyncSpotIx({
      ...addresses,
      ...traderIndex,
      traderAccount: brand<TraderAddress>(k.k2),
      mint: brand<MintAddress>(k.k6),
      perpAssetMap: brand<PerpAssetMapAddress>(k.k3),
    });
    expectMatchesGolden(ix, goldens.instructions.syncSpot);
  });

  it("withdraw spot", async () => {
    const ix = await buildWithdrawSpotIx({
      ...addresses,
      ...traderIndex,
      trader: brand<Authority>(k.k1),
      traderAccount: brand<TraderAddress>(k.k2),
      mint: brand<MintAddress>(k.k6),
      perpAssetMap: brand<PerpAssetMapAddress>(k.k3),
      destination: brand<TokenAccountAddress>(k.k4),
      withdrawQueue: brand<WithdrawQueueAddress>(k.k5),
      action: { kind: "withExcess", amount: 9n },
    });
    expectMatchesGolden(ix, goldens.instructions.withdrawSpotWithExcess);
  });

  it("transfer spot", async () => {
    const ix = await buildTransferSpotIx({
      ...addresses,
      ...traderIndex,
      trader: brand<Authority>(k.k1),
      srcTraderAccount: brand<TraderAddress>(k.k2),
      dstTraderAccount: brand<TraderAddress>(k.k3),
      mint: brand<MintAddress>(k.k6),
      perpAssetMap: brand<PerpAssetMapAddress>(k.k4),
      amount: 11n,
    });
    expectMatchesGolden(ix, goldens.instructions.transferSpot);
  });

  it("transfer spot from child to parent without a wallet signature", async () => {
    const ix = await buildTransferSpotFromChildToParentIx({
      ...addresses,
      ...traderIndex,
      trader: brand<Authority>(k.k1),
      childTraderAccount: brand<TraderAddress>(k.k2),
      parentTraderAccount: brand<TraderAddress>(k.k3),
      mint: brand<MintAddress>(k.k6),
      perpAssetMap: brand<PerpAssetMapAddress>(k.k4),
      traderSigns: false,
    });
    expectMatchesGolden(ix, goldens.instructions.transferSpotFromChildToParent);
  });

  it("swap spot with usdc", async () => {
    const ix = await buildSwapSpotWithUsdcIx({
      ...addresses,
      ...traderIndex,
      signer: brand<Authority>(k.k9),
      traderAccount: brand<TraderAddress>(k.k2),
      quoteMint: brand<MintAddress>(k.k5),
      spotMint: brand<MintAddress>(k.k6),
      perpAssetMap: brand<PerpAssetMapAddress>(k.k3),
      signerQuoteTokenAccount: brand<TokenAccountAddress>(k.k10),
      signerSpotTokenAccount: brand<TokenAccountAddress>(k.k11),
      withdrawQueue: brand<WithdrawQueueAddress>(k.k12),
      direction: SwapDirection.Sell,
      amountIn: 13n,
      minAmountOut: 14n,
      venue: venue(),
    });
    expectMatchesGolden(ix, goldens.instructions.swapSpotWithUsdc);
  });

  it("swap spot with sol", async () => {
    const ix = await buildSwapSpotWithSolIx({
      ...addresses,
      ...traderIndex,
      signer: brand<Authority>(k.k9),
      traderAccount: brand<TraderAddress>(k.k2),
      spotMint: brand<MintAddress>(k.k6),
      perpAssetMap: brand<PerpAssetMapAddress>(k.k3),
      signerSpotTokenAccount: brand<TokenAccountAddress>(k.k11),
      withdrawQueue: brand<WithdrawQueueAddress>(k.k12),
      direction: SwapDirection.Buy,
      amountIn: 13n,
      minAmountOut: "unprotected",
      venue: venue(),
    });
    expectMatchesGolden(ix, goldens.instructions.swapSpotWithSol);
  });

  it("swap spot with spot", async () => {
    const ix = await buildSwapSpotWithSpotIx({
      ...addresses,
      ...traderIndex,
      signer: brand<Authority>(k.k9),
      traderAccount: brand<TraderAddress>(k.k2),
      srcMint: brand<MintAddress>(k.k6),
      dstMint: brand<MintAddress>(k.k7),
      perpAssetMap: brand<PerpAssetMapAddress>(k.k3),
      signerSrcTokenAccount: brand<TokenAccountAddress>(k.k11),
      signerDstTokenAccount: brand<TokenAccountAddress>(k.k13),
      withdrawQueue: brand<WithdrawQueueAddress>(k.k12),
      amountIn: 13n,
      minAmountOut: 14n,
      venue: venue(),
    });
    expectMatchesGolden(ix, goldens.instructions.swapSpotWithSpot);
  });

  it("liquidate spot with a delegated permission", async () => {
    const ix = await buildLiquidateSpotIx({
      ...addresses,
      ...traderIndex,
      signer: brand<Authority>(k.k9),
      permissionAccount: brand<Address>(k.k50),
      liquidateeAccount: brand<TraderAddress>(k.k2),
      quoteMint: brand<MintAddress>(k.k5),
      spotMint: brand<MintAddress>(k.k6),
      perpAssetMap: brand<PerpAssetMapAddress>(k.k3),
      signerQuoteTokenAccount: brand<TokenAccountAddress>(k.k10),
      signerSpotTokenAccount: brand<TokenAccountAddress>(k.k11),
      maxSpotAmount: 13n,
      extraTraderAccounts: [brand<TraderAddress>(k.k60)],
      venue: venue(),
    });
    expectMatchesGolden(ix, goldens.instructions.liquidateSpot);
  });
});

describe("spot ix validation", () => {
  it("rejects a withdraw whose destination is the custody ATA", async () => {
    const custodyAta = goldens.instructions.withdrawSpotWithExcess.accounts[8]!;
    await expect(
      buildWithdrawSpotIx({
        ...addresses,
        ...traderIndex,
        trader: brand<Authority>(k.k1),
        traderAccount: brand<TraderAddress>(k.k2),
        mint: brand<MintAddress>(k.k6),
        perpAssetMap: brand<PerpAssetMapAddress>(k.k3),
        destination: brand<TokenAccountAddress>(custodyAta.address),
        withdrawQueue: brand<WithdrawQueueAddress>(k.k5),
        action: { kind: "allExcess" },
      })
    ).rejects.toThrow(/custody ATA/);
  });

  it("requires explicit slippage protection on swaps", async () => {
    const base = {
      ...addresses,
      ...traderIndex,
      signer: brand<Authority>(k.k9),
      traderAccount: brand<TraderAddress>(k.k2),
      quoteMint: brand<MintAddress>(k.k5),
      spotMint: brand<MintAddress>(k.k6),
      perpAssetMap: brand<PerpAssetMapAddress>(k.k3),
      signerQuoteTokenAccount: brand<TokenAccountAddress>(k.k10),
      signerSpotTokenAccount: brand<TokenAccountAddress>(k.k11),
      withdrawQueue: brand<WithdrawQueueAddress>(k.k12),
      direction: SwapDirection.Sell,
      amountIn: 13n,
      venue: venue(),
    };
    await expect(
      buildSwapSpotWithUsdcIx({ ...base, minAmountOut: undefined as never })
    ).rejects.toThrow(/minAmountOut is required/);
    await expect(
      buildSwapSpotWithUsdcIx({ ...base, minAmountOut: 0n })
    ).rejects.toThrow(/greater than 0/);
  });
});
