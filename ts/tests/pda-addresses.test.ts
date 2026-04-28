import {
  EMBER_PROGRAM_ADDRESS,
  PHOENIX_PROGRAM_ADDRESS,
  SPL_ATA_PROGRAM_ADDRESS,
  SPL_TOKEN_PROGRAM_ADDRESS,
  getEmberStateAddress,
  getEmberVaultAddress,
  getPhoenixEscrowAddress,
  getPhoenixGlobalConfigurationAddress,
  getPhoenixGlobalVaultAddress,
  getPhoenixLogAuthorityAddress,
  getPhoenixPermissionAddress,
  getPhoenixSplineCollectionAddress,
  getPhoenixStopLossAddress,
  getPhoenixTraderSubaccountAddress,
  getPhoenixTraderTokenAccountAddress,
} from "@/index";
import type {
  Authority,
  MarketAddress,
  MintAddress,
  PhoenixProgramAddress,
  TraderAddress,
} from "@/primitives";
import {
  address,
  getBase58Encoder,
  getProgramDerivedAddress,
  type Address,
} from "@solana/kit";
import { afterEach, describe, expect, it } from "vitest";

const BETA_PROGRAM_ADDRESS = address(
  "phDEVv4w6BcfkLrLNeXr8HhhgQxnxziVGXpGPcaadMf"
) as PhoenixProgramAddress;
const AUTHORITY = address(
  "Vote111111111111111111111111111111111111111"
) as Authority;
const DELEGATED_KEY = address(
  "Stake11111111111111111111111111111111111111"
) as Address;
const MARKET = address(
  "SysvarRecentB1ockHashes11111111111111111111"
) as MarketAddress;
const MINT = address(
  "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
) as MintAddress;
const TRADER_ACCOUNT = address(
  "SysvarFees111111111111111111111111111111111"
) as TraderAddress;

afterEach(() => {
  delete process.env.PHOENIX_ENV;
  delete process.env.NEXT_PUBLIC_PHOENIX_ENV;
});

const derivePda = async (
  programAddress: Address,
  seeds: Parameters<typeof getProgramDerivedAddress>[0]["seeds"]
) => {
  const [pda] = await getProgramDerivedAddress({ programAddress, seeds });
  return pda;
};

describe("phoenix PDA helpers", () => {
  it("derives instruction PDAs from the supplied program id", async () => {
    const expectedLog = await derivePda(BETA_PROGRAM_ADDRESS, ["log"]);
    const expectedGlobal = await derivePda(BETA_PROGRAM_ADDRESS, ["global"]);

    await expect(
      getPhoenixLogAuthorityAddress(BETA_PROGRAM_ADDRESS)
    ).resolves.toBe(expectedLog);
    await expect(
      getPhoenixGlobalConfigurationAddress(BETA_PROGRAM_ADDRESS)
    ).resolves.toBe(expectedGlobal);
  });

  it("defaults trader subaccount derivation to the env-resolved Phoenix program", async () => {
    const params = {
      authority: AUTHORITY,
      traderPdaIndex: 7,
      subaccountIndex: 2,
    };

    const prodTrader = await getPhoenixTraderSubaccountAddress({
      ...params,
      phoenixProgramAddress: PHOENIX_PROGRAM_ADDRESS,
    });

    process.env.PHOENIX_ENV = "beta";

    const envTrader = await getPhoenixTraderSubaccountAddress(params);
    const explicitBetaTrader = await getPhoenixTraderSubaccountAddress({
      ...params,
      phoenixProgramAddress: BETA_PROGRAM_ADDRESS,
    });

    expect(envTrader).toBe(explicitBetaTrader);
    expect(envTrader).not.toBe(prodTrader);
  });

  it("derives the remaining helper PDAs with the expected seeds", async () => {
    const traderAccountBytes = getBase58Encoder().encode(TRADER_ACCOUNT);
    const authorityBytes = getBase58Encoder().encode(AUTHORITY);
    const marketBytes = getBase58Encoder().encode(MARKET);
    const mintBytes = getBase58Encoder().encode(MINT);
    const splTokenProgramBytes = getBase58Encoder().encode(
      SPL_TOKEN_PROGRAM_ADDRESS
    );
    const assetIdBytes = new Uint8Array(8);
    new DataView(assetIdBytes.buffer).setBigUint64(0, 42n, true);

    const [
      expectedSplineCollection,
      expectedTokenAccount,
      expectedGlobalVault,
      expectedPermission,
      expectedEscrow,
      expectedStopLoss,
      expectedEmberState,
      expectedEmberVault,
    ] = await Promise.all([
      derivePda(BETA_PROGRAM_ADDRESS, ["spline", marketBytes]),
      derivePda(SPL_ATA_PROGRAM_ADDRESS, [
        authorityBytes,
        splTokenProgramBytes,
        mintBytes,
      ]),
      derivePda(BETA_PROGRAM_ADDRESS, ["vault", mintBytes]),
      derivePda(BETA_PROGRAM_ADDRESS, [
        "permission",
        authorityBytes,
        getBase58Encoder().encode(DELEGATED_KEY),
      ]),
      derivePda(BETA_PROGRAM_ADDRESS, ["escrow", authorityBytes]),
      derivePda(BETA_PROGRAM_ADDRESS, [
        "stoploss",
        traderAccountBytes,
        assetIdBytes,
      ]),
      derivePda(EMBER_PROGRAM_ADDRESS, [
        getBase58Encoder().encode(BETA_PROGRAM_ADDRESS),
        "state",
      ]),
      derivePda(EMBER_PROGRAM_ADDRESS, [
        getBase58Encoder().encode(BETA_PROGRAM_ADDRESS),
        "vault",
      ]),
    ]);

    await expect(
      getPhoenixSplineCollectionAddress(MARKET, BETA_PROGRAM_ADDRESS)
    ).resolves.toBe(expectedSplineCollection);
    await expect(
      getPhoenixTraderTokenAccountAddress(AUTHORITY, MINT)
    ).resolves.toBe(expectedTokenAccount);
    await expect(
      getPhoenixGlobalVaultAddress(MINT, BETA_PROGRAM_ADDRESS)
    ).resolves.toBe(expectedGlobalVault);
    await expect(
      getPhoenixPermissionAddress(
        AUTHORITY,
        DELEGATED_KEY,
        BETA_PROGRAM_ADDRESS
      )
    ).resolves.toBe(expectedPermission);
    await expect(
      getPhoenixEscrowAddress(AUTHORITY, BETA_PROGRAM_ADDRESS)
    ).resolves.toBe(expectedEscrow);
    await expect(
      getPhoenixStopLossAddress({
        traderAccount: TRADER_ACCOUNT,
        assetId: 42n,
        phoenixProgramAddress: BETA_PROGRAM_ADDRESS,
      })
    ).resolves.toBe(expectedStopLoss);
    await expect(getEmberStateAddress(BETA_PROGRAM_ADDRESS)).resolves.toBe(
      expectedEmberState
    );
    await expect(getEmberVaultAddress(BETA_PROGRAM_ADDRESS)).resolves.toBe(
      expectedEmberVault
    );
  });
});
