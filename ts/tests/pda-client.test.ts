import {
  createPhoenixClient,
  createPhoenixPdaClient,
  flight,
  getPhoenixConditionalOrdersAddress,
  getPhoenixTraderSubaccountAddress,
  PHOENIX_PROGRAM_ADDRESS,
  type Authority,
} from "@/index";
import { address } from "@solana/kit";
import { describe, expect, it } from "vitest";

const BETA_PROGRAM_ADDRESS = address(
  "phDEVv4w6BcfkLrLNeXr8HhhgQxnxziVGXpGPcaadMf"
);
const TEST_AUTHORITY = address("11111111111111111111111111111112") as Authority;

describe("phoenix pda client", () => {
  it("resolves trader PDAs from an optional phoenix env", async () => {
    const pda = createPhoenixPdaClient({ phoenixEnv: "beta" });

    const actual = await pda.getTraderAddress({
      authority: TEST_AUTHORITY,
      traderPdaIndex: 7,
      subaccountIndex: 0,
    });
    const expected = await getPhoenixTraderSubaccountAddress({
      authority: TEST_AUTHORITY,
      traderPdaIndex: 7,
      subaccountIndex: 0,
      phoenixProgramAddress: BETA_PROGRAM_ADDRESS,
    });

    expect(actual).toBe(expected);
    expect(pda.getProgramAddress()).toBe(BETA_PROGRAM_ADDRESS);
  });

  it("keeps configured address overrides ahead of phoenix env defaults", async () => {
    const pda = createPhoenixPdaClient({
      phoenixEnv: "beta",
      addresses: {
        programAddress: PHOENIX_PROGRAM_ADDRESS,
      },
    });

    const actual = await pda.getTraderAddress({
      authority: TEST_AUTHORITY,
      traderPdaIndex: 7,
      subaccountIndex: 0,
    });
    const expected = await getPhoenixTraderSubaccountAddress({
      authority: TEST_AUTHORITY,
      traderPdaIndex: 7,
      subaccountIndex: 0,
      phoenixProgramAddress: PHOENIX_PROGRAM_ADDRESS,
    });

    expect(actual).toBe(expected);
    expect(pda.getProgramAddress()).toBe(PHOENIX_PROGRAM_ADDRESS);
  });

  it("attaches the pda helper to createPhoenixClient bundles", () => {
    const client = createPhoenixClient({
      baseUrl: "http://localhost:8080",
      phoenixEnv: "beta",
    });

    expect(client.pda.getProgramAddress()).toBe(BETA_PROGRAM_ADDRESS);
    expect(client.api.pda.getProgramAddress()).toBe(BETA_PROGRAM_ADDRESS);
    expect(client.exchange).toBeDefined();

    client.dispose();
  });

  it("resolves flight PDAs from the configured phoenix env", async () => {
    const pda = createPhoenixPdaClient({ phoenixEnv: "beta" });

    const actualGlobal = await pda.getFlightGlobalStateAddress();
    const expectedGlobal =
      await flight.getFlightGlobalStateAddress(BETA_PROGRAM_ADDRESS);
    const actualBuilder =
      await pda.getFlightBuilderStateAddress(TEST_AUTHORITY);
    const expectedBuilder = await flight.getFlightBuilderStateAddress(
      TEST_AUTHORITY,
      BETA_PROGRAM_ADDRESS
    );

    expect(actualGlobal).toBe(expectedGlobal);
    expect(actualBuilder).toBe(expectedBuilder);
  });

  it("keeps explicit phoenix overrides ahead of env defaults for flight PDAs", async () => {
    const pda = createPhoenixPdaClient({ phoenixEnv: "beta" });

    const actual = await pda.getFlightBuilderStateAddress(TEST_AUTHORITY, {
      phoenixProgramAddress: PHOENIX_PROGRAM_ADDRESS,
    });
    const expected = await flight.getFlightBuilderStateAddress(
      TEST_AUTHORITY,
      PHOENIX_PROGRAM_ADDRESS
    );

    expect(actual).toBe(expected);
  });

  it("resolves conditional order PDAs through the cached client helpers", async () => {
    const pda = createPhoenixPdaClient({ phoenixEnv: "beta" });
    const traderAccount = await pda.getTraderAddress({
      authority: TEST_AUTHORITY,
      traderPdaIndex: 7,
      subaccountIndex: 0,
    });

    const actual = await pda.getConditionalOrdersAddress({
      traderAccount,
    });
    const expected = await getPhoenixConditionalOrdersAddress({
      traderAccount,
      phoenixProgramAddress: BETA_PROGRAM_ADDRESS,
    });

    expect(actual).toBe(expected);
  });
});
