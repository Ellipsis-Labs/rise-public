import {
  FLAME_PROGRAM_ADDRESS,
  buildFlameAtomicDepositFlow,
  buildFlameDepositFundingFlow,
  deriveFlameDepositAddress,
  deriveFlameDepositAddresses,
  deriveFlameProxyAuthorityAddress,
} from "@/index";
import {
  PHOENIX_GLOBAL_CONFIGURATION_ADDRESS,
  PHOENIX_LOG_AUTHORITY_ADDRESS,
  PHOENIX_PROGRAM_ADDRESS,
  SPL_ATA_PROGRAM_ADDRESS,
  SPL_TOKEN_PROGRAM_ADDRESS,
  USDC_MINT_ADDRESS,
} from "@/core/constants";
import { getPhoenixTraderTokenAccountAddress } from "@/pdas";
import type {
  Authority,
  EmberStateAddress,
  FlameAtomicDepositFlowParams,
  MintAddress,
  PhoenixInstructionClient,
} from "@/index";
import type { PhoenixExchangeMetadata } from "@/exchange-cache/types";
import { AccountRole, address } from "@solana/kit";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

const authority = address(
  "EtrnLzgbS7nMMy5fbD42kXiUzGg8XQzJ972Xtk1cjWih"
) as Authority;
const feePayer = address("11111111111111111111111111111111") as Authority;
const vectorAuthority = address(
  "11111111111111111111111111111112"
) as Authority;
const marketAuthority = address(
  "11111111111111111111111111111113"
) as Authority;
const globalVault = address("11111111111111111111111111111114") as Authority;
const perpAssetMap = address("11111111111111111111111111111115") as Authority;
const globalTraderIndex = address(
  "11111111111111111111111111111116"
) as Authority;
const activeTraderBuffer = address(
  "11111111111111111111111111111117"
) as Authority;
const withdrawQueue = address(
  "So11111111111111111111111111111111111111112"
) as Authority;
const vectorMint = address(
  "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
) as MintAddress;

const PROD_EXPECTED = {
  proxyAuthority: "Az9LBWKVUyeXthCZEwNmXi5miNsUUHhkpcBFRJXMWVLy",
  depositAddress: "DDmXL6EvHz73HELyPjZeQr1BNZZkN8J3L9AsZPNYT37p",
  proxyAta: "DDmXL6EvHz73HELyPjZeQr1BNZZkN8J3L9AsZPNYT37p",
};

const BETA_EXPECTED = {
  proxyAuthority: "Hp5BtEBvyZTwX8UVQNgCAxbYxVwRfhebvFJtgyHba1tb",
  depositAddress: "8q93pH7TbkyGyvesVbBueaWve671PBfuL33qdWys9EQR",
  proxyAta: "8q93pH7TbkyGyvesVbBueaWve671PBfuL33qdWys9EQR",
};

const client = {
  addresses: {
    phoenixProgramAddress: PHOENIX_PROGRAM_ADDRESS,
    logAuthorityAddress: PHOENIX_LOG_AUTHORITY_ADDRESS,
    globalConfigurationAddress: PHOENIX_GLOBAL_CONFIGURATION_ADDRESS,
    usdcMintAddress: USDC_MINT_ADDRESS,
    emberStateAddress: address(
      "EMBERpYNE6ehWmXymZZS2skiFmCa9V5dp14e1iduM5qy"
    ) as EmberStateAddress,
  },
  fetchAccount: async () => {
    throw new Error("buildFlameDepositFundingFlow should not fetch accounts");
  },
} satisfies PhoenixInstructionClient;

const atomicClient = {
  ...client,
  exchange: {
    ready: async () => ({
      version: 1,
      slot: 1n,
      slotIndex: 0,
      markets: [],
      exchange: {
        programId: PHOENIX_PROGRAM_ADDRESS,
        globalConfig: PHOENIX_GLOBAL_CONFIGURATION_ADDRESS,
        currentAuthorities: {
          rootAuthority: marketAuthority,
          riskAuthority: marketAuthority,
          marketAuthority,
          oracleAuthority: marketAuthority,
          adlAuthority: marketAuthority,
          cancelAuthority: marketAuthority,
          backstopAuthority: marketAuthority,
        },
        canonicalMint: vectorMint,
        usdcMint: USDC_MINT_ADDRESS,
        globalVault,
        perpAssetMap,
        globalTraderIndex: [globalTraderIndex],
        activeTraderBuffer: [activeTraderBuffer],
        withdrawQueue,
        exchangeStatusBits: 0,
        exchangeStatusFeatures: [],
        active: true,
        gated: false,
        withdrawalsAvailable: true,
      },
    }),
  } as Pick<PhoenixExchangeMetadata, "ready"> as PhoenixExchangeMetadata,
} satisfies PhoenixInstructionClient;

const originalPhoenixEnv = process.env.PHOENIX_ENV;
const originalNextPublicPhoenixEnv = process.env.NEXT_PUBLIC_PHOENIX_ENV;

beforeEach(() => {
  delete process.env.PHOENIX_ENV;
  delete process.env.NEXT_PUBLIC_PHOENIX_ENV;
});

afterEach(() => {
  if (originalPhoenixEnv == null) {
    delete process.env.PHOENIX_ENV;
  } else {
    process.env.PHOENIX_ENV = originalPhoenixEnv;
  }

  if (originalNextPublicPhoenixEnv == null) {
    delete process.env.NEXT_PUBLIC_PHOENIX_ENV;
  } else {
    process.env.NEXT_PUBLIC_PHOENIX_ENV = originalNextPublicPhoenixEnv;
  }
});

describe("Flame deposit helpers", () => {
  it("derives the prod proxy authority and deposit address", async () => {
    const addresses = await deriveFlameDepositAddresses({
      userAuthority: vectorAuthority,
      traderPdaIndex: 7,
      mintAddress: vectorMint,
    });

    expect(addresses).toEqual(PROD_EXPECTED);
    await expect(
      deriveFlameProxyAuthorityAddress({
        userAuthority: vectorAuthority,
        traderPdaIndex: 7,
      })
    ).resolves.toBe(PROD_EXPECTED.proxyAuthority);
    await expect(
      deriveFlameDepositAddress({
        userAuthority: vectorAuthority,
        traderPdaIndex: 7,
        mintAddress: vectorMint,
      })
    ).resolves.toBe(PROD_EXPECTED.depositAddress);
  });

  it("switches the default Phoenix program when beta env is active", async () => {
    process.env.NEXT_PUBLIC_PHOENIX_ENV = "beta";

    const addresses = await deriveFlameDepositAddresses({
      userAuthority: vectorAuthority,
      traderPdaIndex: 7,
      mintAddress: vectorMint,
    });

    expect(addresses).toEqual(BETA_EXPECTED);
  });

  it("rejects out-of-range trader PDA indices", async () => {
    await expect(
      deriveFlameProxyAuthorityAddress({
        userAuthority: vectorAuthority,
        traderPdaIndex: 256,
      })
    ).rejects.toThrow("Trader PDA index must be between 0 and 255");
  });

  it("does not derive a user-owned deposit ATA", async () => {
    const addresses = await deriveFlameDepositAddresses({
      userAuthority: authority,
      traderPdaIndex: 7,
      mintAddress: USDC_MINT_ADDRESS,
      phoenixProgramAddress: PHOENIX_PROGRAM_ADDRESS,
    });

    expect(addresses.proxyAuthority).not.toBe(authority);
    expect(addresses.depositAddress).toBe(addresses.proxyAta);
    expect(addresses.depositAddress).not.toBe(
      await getPhoenixTraderTokenAccountAddress(authority, USDC_MINT_ADDRESS)
    );
  });

  it("builds a proxy ATA create and user-signed USDC transfer", async () => {
    const amount = 42_000_000n;
    const result = await buildFlameDepositFundingFlow(
      {
        authority,
        amount,
        traderPdaIndex: 7,
        feePayer,
        sponsorshipToken: "test-token",
      },
      client
    );
    const expectedUserUsdcAta = await getPhoenixTraderTokenAccountAddress(
      authority,
      USDC_MINT_ADDRESS
    );

    expect(FLAME_PROGRAM_ADDRESS).toBe(
      "FLameGi7e6q6oDf4osFqQFtFJureWKKpnMFU4JmiRDvF"
    );
    expect(result.instructions).toEqual([
      result.named.createProxyAta,
      result.named.transferUsdcToProxy,
    ]);
    expect(result.traderPdaIndex).toBe(7);
    expect(result.depositAddress).toBe(result.proxyAta);

    const createProxyAta = result.named.createProxyAta;
    expect(createProxyAta.programAddress).toBe(SPL_ATA_PROGRAM_ADDRESS);
    expect(createProxyAta.accounts[0]).toEqual({
      address: feePayer,
      role: AccountRole.WRITABLE_SIGNER,
    });
    expect(createProxyAta.accounts[1]).toEqual({
      address: result.proxyAta,
      role: AccountRole.WRITABLE,
    });
    expect(createProxyAta.accounts[2]).toEqual({
      address: result.proxyAuthority,
      role: AccountRole.READONLY,
    });
    expect(createProxyAta.accounts[3]?.address).toBe(USDC_MINT_ADDRESS);

    const transferUsdcToProxy = result.named.transferUsdcToProxy;
    expect(transferUsdcToProxy.programAddress).toBe(SPL_TOKEN_PROGRAM_ADDRESS);
    expect(transferUsdcToProxy.accounts).toEqual([
      { address: expectedUserUsdcAta, role: AccountRole.WRITABLE },
      { address: result.proxyAta, role: AccountRole.WRITABLE },
      { address: authority, role: AccountRole.READONLY_SIGNER },
    ]);
    expect(Array.from(transferUsdcToProxy.data)).toEqual([
      3, 128, 222, 128, 2, 0, 0, 0, 0,
    ]);
  });

  it("builds an atomic sponsored Flame deposit bundle", async () => {
    const result = await buildFlameAtomicDepositFlow(
      {
        authority,
        amount: 42_000_000n,
        traderPdaIndex: 7,
        feePayer,
        sponsorshipToken: "test-token",
      },
      atomicClient
    );

    expect(result.instructions).toEqual([
      result.named.createProxyAta,
      result.named.transferUsdcToProxy,
      result.named.createPermission,
      result.named.setPermission,
      result.named.depositToPhoenix,
    ]);
    expect(result.traderPdaIndex).toBe(7);
    expect(result.traderSubaccountIndex).toBe(0);

    expect(result.named.createPermission.accounts[2]).toEqual({
      address: feePayer,
      role: AccountRole.WRITABLE_SIGNER,
    });
    expect(result.named.setPermission.accounts[3]).toEqual({
      address: authority,
      role: AccountRole.READONLY_SIGNER,
    });
    expect(result.named.setPermission.accounts[4]).toEqual({
      address: result.proxyAuthority,
      role: AccountRole.READONLY,
    });

    const depositToPhoenix = result.named.depositToPhoenix;
    expect(depositToPhoenix.programAddress).toBe(FLAME_PROGRAM_ADDRESS);
    expect(depositToPhoenix.accounts[2]).toEqual({
      address: feePayer,
      role: AccountRole.WRITABLE_SIGNER,
    });
    expect(depositToPhoenix.accounts[3]).toEqual({
      address: authority,
      role: AccountRole.READONLY,
    });
    expect(depositToPhoenix.accounts[4]).toEqual({
      address: result.proxyAuthority,
      role: AccountRole.READONLY,
    });
    expect(depositToPhoenix.accounts[5]).toEqual({
      address: result.proxyAta,
      role: AccountRole.WRITABLE,
    });
    expect(Array.from(depositToPhoenix.data.slice(8))).toEqual([7, 0]);
  });

  it("defaults the Flame crank to the sponsor fee payer", async () => {
    const result = await buildFlameAtomicDepositFlow(
      {
        authority,
        amount: 42_000_000n,
        traderPdaIndex: 7,
        feePayer,
        sponsorshipToken: "test-token",
      },
      atomicClient
    );

    expect(result.named.depositToPhoenix.accounts[2]).toEqual({
      address: feePayer,
      role: AccountRole.WRITABLE_SIGNER,
    });
  });

  it("rejects non-zero trader subaccounts for sponsored atomic deposits", async () => {
    await expect(
      buildFlameAtomicDepositFlow(
        {
          authority,
          feePayer,
          sponsorshipToken: "token",
          amount: 1_000_000n,
          traderSubaccountIndex: 1,
        } as FlameAtomicDepositFlowParams & { traderSubaccountIndex: number },
        atomicClient
      )
    ).rejects.toThrow(
      "Flame atomic deposit sponsorship only supports traderSubaccountIndex 0"
    );
  });
});
