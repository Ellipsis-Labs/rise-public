import { describe, expect, it } from "vitest";

import {
  getReferralActivationTraderState,
  buildReferralActivationTransaction,
  buildActivateReferralTxRequest,
  type ReferralActivationExchangeAccounts,
  type ReferralActivationRpc,
} from "@/index";
import { decodeBase64ToBytes, encodeBytesToBase64 } from "@/base64";
import { V1InviteClient } from "@/api/invite";
import {
  PHOENIX_GLOBAL_CONFIGURATION_ADDRESS,
  PHOENIX_LOG_AUTHORITY_ADDRESS,
  PHOENIX_PROGRAM_ADDRESS,
  USDC_MINT_ADDRESS,
} from "@/core/constants";
import type { HttpTransport, RequestOptions } from "@/http";
import {
  decompileTransactionMessage,
  getCompiledTransactionMessageDecoder,
  type Transaction,
} from "@solana/kit";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const REFERRAL_CODE = "sdk-code";
const TRADER_AUTHORITY = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";
const INTEGRATOR_PAYER = "F9xvTNv5JjWuUeCv8HqRfqfWWL57vGVGcSk1VSDyvJsR";
const RECENT_BLOCKHASH = "11111111111111111111111111111111";
const TESTS_DIR = dirname(fileURLToPath(import.meta.url));
const TRADER_FLAGS_OFFSET = 96;

const PERMISSION = {
  trader_onboarder: PHOENIX_LOG_AUTHORITY_ADDRESS,
  risk_authority: USDC_MINT_ADDRESS,
  permission_account: PHOENIX_GLOBAL_CONFIGURATION_ADDRESS,
};

const EXCHANGE_ACCOUNTS: ReferralActivationExchangeAccounts = {
  phoenixProgramAddress: PHOENIX_PROGRAM_ADDRESS,
  logAuthorityAddress: PHOENIX_LOG_AUTHORITY_ADDRESS,
  globalConfigurationAddress: PHOENIX_GLOBAL_CONFIGURATION_ADDRESS,
  globalTraderIndex: [PHOENIX_GLOBAL_CONFIGURATION_ADDRESS, USDC_MINT_ADDRESS],
  activeTraderBuffer: [PHOENIX_LOG_AUTHORITY_ADDRESS, USDC_MINT_ADDRESS],
};

interface RequestRecord {
  method: string;
  endpoint: string;
  auth?: RequestOptions["auth"];
  body?: unknown;
}

const createTransport = (
  responses: Map<string, unknown>,
  records: RequestRecord[]
): HttpTransport => ({
  fetch: async (
    method: string,
    endpoint: string,
    options?: RequestOptions,
    body?: unknown
  ) => {
    records.push({
      method,
      endpoint,
      auth: options?.auth,
      body,
    });

    return new Response(JSON.stringify(responses.get(endpoint)), {
      status: 200,
      headers: { "content-type": "application/json" },
    });
  },
});

const getInstructionCount = (transaction: Transaction): number =>
  decompileTransactionMessage(
    getCompiledTransactionMessageDecoder().decode(transaction.messageBytes)
  ).instructions.length;

const readTraderAccountBase64 = (): string => {
  const fixture = JSON.parse(
    readFileSync(resolve(TESTS_DIR, "mocks", "trader.json"), "utf8")
  ) as { account: { data: [string, string] } };
  return fixture.account.data[0];
};

const traderAccountBase64WithFlags = (flags: number): string => {
  const bytes = decodeBase64ToBytes(readTraderAccountBase64());
  new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).setUint32(
    TRADER_FLAGS_OFFSET,
    flags,
    true
  );
  return encodeBytesToBase64(bytes);
};

const createRpcWithTraderAccount = (
  dataBase64: string | null
): ReferralActivationRpc =>
  ({
    getAccountInfo: () => ({
      send: async () => ({
        value: dataBase64
          ? {
              data: [dataBase64, "base64"],
            }
          : null,
      }),
    }),
  }) as unknown as ReferralActivationRpc;

describe("referral activation transaction helpers", () => {
  it("builds the expected partial transaction request fields", async () => {
    const built = await buildReferralActivationTransaction({
      referralCode: REFERRAL_CODE,
      traderAuthority: TRADER_AUTHORITY,
      traderPdaIndex: 0,
      traderSubaccountIndex: 0,
      recentBlockhash: RECENT_BLOCKHASH,
      permission: PERMISSION,
      exchangeAccounts: EXCHANGE_ACCOUNTS,
    });

    expect(built.requestFields).toEqual({
      referral_code: REFERRAL_CODE,
      trader_authority: TRADER_AUTHORITY,
      trader_pda_index: 0,
      trader_subaccount_index: 0,
      recent_blockhash: RECENT_BLOCKHASH,
    });
    expect(built.traderPda).toMatch(/^[1-9A-HJ-NP-Za-km-z]+$/);
    expect(built.unsignedTransactionBase64).toMatch(/^[A-Za-z0-9+/]+=*$/);
    expect(getInstructionCount(built.transaction)).toBe(1);
    expect(Object.keys(built.transaction.signatures).sort()).toEqual(
      [TRADER_AUTHORITY, PERMISSION.trader_onboarder].sort()
    );
  });

  it("prepends register trader when requested for a missing trader", async () => {
    const built = await buildReferralActivationTransaction({
      referralCode: REFERRAL_CODE,
      traderAuthority: TRADER_AUTHORITY,
      traderPdaIndex: 0,
      traderSubaccountIndex: 0,
      recentBlockhash: RECENT_BLOCKHASH,
      permission: PERMISSION,
      exchangeAccounts: EXCHANGE_ACCOUNTS,
      includeRegisterTrader: true,
      registerTraderMaxPositions: 32n,
    });

    expect(getInstructionCount(built.transaction)).toBe(2);
    expect(Object.keys(built.transaction.signatures).sort()).toEqual(
      [TRADER_AUTHORITY, PERMISSION.trader_onboarder].sort()
    );
  });

  it("uses deprecated rentPayer as a fallback fee payer only when registering", async () => {
    const built = await buildReferralActivationTransaction({
      referralCode: REFERRAL_CODE,
      traderAuthority: TRADER_AUTHORITY,
      traderPdaIndex: 0,
      traderSubaccountIndex: 0,
      recentBlockhash: RECENT_BLOCKHASH,
      permission: PERMISSION,
      exchangeAccounts: EXCHANGE_ACCOUNTS,
      includeRegisterTrader: true,
      rentPayer: INTEGRATOR_PAYER,
    });

    expect(getInstructionCount(built.transaction)).toBe(2);
    expect(Object.keys(built.transaction.signatures).sort()).toEqual(
      [INTEGRATOR_PAYER, TRADER_AUTHORITY, PERMISSION.trader_onboarder].sort()
    );
  });

  it("ignores deprecated rentPayer when registration is not included", async () => {
    const built = await buildReferralActivationTransaction({
      referralCode: REFERRAL_CODE,
      traderAuthority: TRADER_AUTHORITY,
      recentBlockhash: RECENT_BLOCKHASH,
      permission: PERMISSION,
      exchangeAccounts: EXCHANGE_ACCOUNTS,
      rentPayer: INTEGRATOR_PAYER,
    });

    expect(getInstructionCount(built.transaction)).toBe(1);
    expect(Object.keys(built.transaction.signatures).sort()).toEqual(
      [TRADER_AUTHORITY, PERMISSION.trader_onboarder].sort()
    );
  });

  it("rejects register trader max positions below the activate-tx minimum", async () => {
    await expect(
      buildReferralActivationTransaction({
        referralCode: REFERRAL_CODE,
        traderAuthority: TRADER_AUTHORITY,
        recentBlockhash: RECENT_BLOCKHASH,
        permission: PERMISSION,
        exchangeAccounts: EXCHANGE_ACCOUNTS,
        includeRegisterTrader: true,
        registerTraderMaxPositions: 31n,
      })
    ).rejects.toThrow("registerTraderMaxPositions must be between 32 and 128");
  });

  it("auto-includes register trader when RPC shows the trader account is missing", async () => {
    const built = await buildActivateReferralTxRequest({
      referralCode: REFERRAL_CODE,
      traderAuthority: TRADER_AUTHORITY,
      recentBlockhash: RECENT_BLOCKHASH,
      permission: PERMISSION,
      exchangeAccounts: EXCHANGE_ACCOUNTS,
      rpc: createRpcWithTraderAccount(null),
      signTransaction: (transaction) => {
        expect(getInstructionCount(transaction)).toBe(2);
        return {
          serialize: () => Uint8Array.of(1, 2, 3, 4),
        };
      },
    });

    expect(built.includeRegisterTrader).toBe(true);
    expect(built.traderActivationState?.status).toBe("missing");
  });

  it("builds set-capabilities only when RPC shows a registered trader without activation capabilities", async () => {
    const built = await buildActivateReferralTxRequest({
      referralCode: REFERRAL_CODE,
      traderAuthority: TRADER_AUTHORITY,
      recentBlockhash: RECENT_BLOCKHASH,
      permission: PERMISSION,
      exchangeAccounts: EXCHANGE_ACCOUNTS,
      rpc: createRpcWithTraderAccount(traderAccountBase64WithFlags(0)),
      signTransaction: (transaction) => {
        expect(getInstructionCount(transaction)).toBe(1);
        return {
          serialize: () => Uint8Array.of(1, 2, 3, 4),
        };
      },
    });

    expect(built.includeRegisterTrader).toBe(false);
    expect(built.traderActivationState?.status).toBe("registered");
  });

  it("reports activated when RPC shows the trader already has activation capabilities", async () => {
    const state = await getReferralActivationTraderState({
      traderAuthority: TRADER_AUTHORITY,
      exchangeAccounts: EXCHANGE_ACCOUNTS,
      rpc: createRpcWithTraderAccount(readTraderAccountBase64()),
    });

    expect(state.status).toBe("activated");
    expect(state.flags).toBe(62);
  });

  it("allows an integrator fee payer while still requiring the trader authority signature", async () => {
    const built = await buildReferralActivationTransaction({
      referralCode: REFERRAL_CODE,
      traderAuthority: TRADER_AUTHORITY,
      feePayer: INTEGRATOR_PAYER,
      recentBlockhash: RECENT_BLOCKHASH,
      permission: PERMISSION,
      exchangeAccounts: EXCHANGE_ACCOUNTS,
    });

    expect(Object.keys(built.transaction.signatures).sort()).toEqual(
      [INTEGRATOR_PAYER, TRADER_AUTHORITY, PERMISSION.trader_onboarder].sort()
    );
  });

  it("builds an activation request with a browser-wallet-style serialized transaction", async () => {
    const records: RequestRecord[] = [];
    const transport = createTransport(
      new Map<string, unknown>([
        ["/v1/referral/activation-permission", PERMISSION],
        [
          "/v1/exchange/snapshot",
          {
            version: 1,
            slot: "1",
            slotIndex: 0,
            exchange: {
              programId: PHOENIX_PROGRAM_ADDRESS,
              globalConfig: PHOENIX_GLOBAL_CONFIGURATION_ADDRESS,
              currentAuthorities: {
                rootAuthority: USDC_MINT_ADDRESS,
                riskAuthority: USDC_MINT_ADDRESS,
                marketAuthority: USDC_MINT_ADDRESS,
                oracleAuthority: USDC_MINT_ADDRESS,
                adlAuthority: USDC_MINT_ADDRESS,
                cancelAuthority: USDC_MINT_ADDRESS,
                backstopAuthority: USDC_MINT_ADDRESS,
              },
              canonicalMint: USDC_MINT_ADDRESS,
              usdcMint: USDC_MINT_ADDRESS,
              globalVault: USDC_MINT_ADDRESS,
              perpAssetMap: USDC_MINT_ADDRESS,
              globalTraderIndex: EXCHANGE_ACCOUNTS.globalTraderIndex,
              activeTraderBuffer: EXCHANGE_ACCOUNTS.activeTraderBuffer,
              withdrawQueue: USDC_MINT_ADDRESS,
              exchangeStatusBits: 1,
              exchangeStatusFeatures: ["initialized"],
              active: true,
              gated: false,
            },
            markets: [],
          },
        ],
        [
          "/v1/referral/activate-tx",
          {
            trader_pda: "trader-pda",
            referral_code: REFERRAL_CODE,
            status: "submitted",
          },
        ],
      ]),
      records
    );
    const invite = new V1InviteClient(transport);

    const built = await invite.buildActivateReferralTxRequest({
      referralCode: REFERRAL_CODE,
      traderAuthority: TRADER_AUTHORITY,
      recentBlockhash: RECENT_BLOCKHASH,
      signTransaction: (_transaction, context) => {
        expect(context.unsignedTransactionBytes.length).toBeGreaterThan(0);
        expect(context.requestFields.referral_code).toBe(REFERRAL_CODE);
        return {
          serialize: () => Uint8Array.of(1, 2, 3, 4),
        };
      },
    });
    const response = await invite.activateReferralTx(built.request);

    expect(built.request).toEqual({
      referral_code: REFERRAL_CODE,
      trader_authority: TRADER_AUTHORITY,
      trader_pda_index: 0,
      trader_subaccount_index: 0,
      recent_blockhash: RECENT_BLOCKHASH,
      transaction: "AQIDBA==",
    });
    expect(response.status).toBe("submitted");
    expect(records).toEqual([
      {
        method: "GET",
        endpoint: "/v1/referral/activation-permission",
        auth: undefined,
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/v1/exchange/snapshot",
        auth: undefined,
        body: undefined,
      },
      {
        method: "POST",
        endpoint: "/v1/referral/activate-tx",
        auth: undefined,
        body: built.request,
      },
    ]);
  });
});
