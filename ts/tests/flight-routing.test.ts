import type {
  PhoenixInstructionClient,
  PhoenixMarketDataClient,
} from "@/core/clientTypes";
import { DISCRIMINANTS } from "@/core/discriminants";
import type { PhoenixExchangeMetadata } from "@/exchange-cache/types";
import {
  MarginType,
  OrderFlags,
  SelfTradeBehavior,
  Side,
  baseLots,
  buildPlaceLimitOrderFlow,
  buildPlaceMarketOrderFlow,
  createPhoenixIxClient,
  createPhoenixPdaClient,
  flight,
  getPhoenixTraderSubaccountAddress,
  quoteLots,
  ticks,
} from "@/index";
import type { ImmediateOrCancelOrderPacket } from "@/primitives";
import { AccountRole, address } from "@solana/kit";
import { describe, expect, it, vi } from "vitest";

describe("flight instruction routing", () => {
  const phoenixProgramAddress = address(
    "phDEVv4w6BcfkLrLNeXr8HhhgQxnxziVGXpGPcaadMf"
  );
  const builderAuthority = address("11111111111111111111111111111111");
  const feeCollectorTrader = address(
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
  ) as never;
  const traderAuthority = address(
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
  );
  const rootAuthority = address("So11111111111111111111111111111111111111112");
  const getCollateralTransferAuthority = () =>
    flight.getFlightCollateralTransferAuthorityAddress(phoenixProgramAddress);

  it("wraps Flight-routable placement instructions", async () => {
    const resolveFeeCollectorTraderAddress = vi.fn(
      async () => feeCollectorTrader
    );
    const resolveRootAuthority = vi.fn(async () => rootAuthority);

    for (const discriminant of [
      DISCRIMINANTS.PLACE_MARKET_ORDER,
      DISCRIMINANTS.PLACE_MARKET_ORDER_DELEGATED,
      DISCRIMINANTS.PLACE_LIMIT_ORDER,
      DISCRIMINANTS.PLACE_STOP_LOSS,
      DISCRIMINANTS.PLACE_POSITION_CONDITIONAL_ORDER,
      DISCRIMINANTS.PLACE_ATTACHED_CONDITIONAL_ORDER,
      DISCRIMINANTS.PLACE_LIMIT_ORDER_WITH_CONDITIONALS,
    ]) {
      const data = new Uint8Array([...discriminant, 1, 2, 3]);
      const wrapped = await flight.wrapInstructionWithFlight({
        phoenixInstruction: {
          programAddress: phoenixProgramAddress,
          accounts: [],
          data,
        },
        signer: traderAuthority,
        phoenixProgramAddress,
        flight: { builderAuthority },
        resolveFeeCollectorTraderAddress,
        resolveRootAuthority,
      });

      expect(wrapped.programAddress).toBe(flight.FLIGHT_PROGRAM_ADDRESS);
      expect(wrapped.accounts[2]?.address).toBe(builderAuthority);
      expect(wrapped.accounts[3]?.address).toBe(feeCollectorTrader);
      expect(wrapped.accounts[5]?.address).toBe(traderAuthority);
      // The collateral-transfer tail is never inferred from the inner
      // instruction — delegated market orders included — so undeclared
      // wraps carry exactly the six proxy accounts.
      expect(wrapped.accounts.length).toBe(6);
      expect(Array.from(wrapped.data?.slice(8) ?? [])).toEqual(
        Array.from(data)
      );
    }

    expect(resolveFeeCollectorTraderAddress).toHaveBeenCalledTimes(7);
    expect(resolveRootAuthority).not.toHaveBeenCalled();
  });

  it("wraps Flight-routable placement instructions with a fee override", async () => {
    const resolveFeeCollectorTraderAddress = vi.fn(
      async () => feeCollectorTrader
    );
    const data = new Uint8Array([...DISCRIMINANTS.PLACE_MARKET_ORDER, 1, 2, 3]);

    const wrapped = await flight.wrapInstructionWithFlight({
      phoenixInstruction: {
        programAddress: phoenixProgramAddress,
        accounts: [],
        data,
      },
      signer: traderAuthority,
      phoenixProgramAddress,
      flight: { builderAuthority, feeBpsOverride: 5n },
      resolveFeeCollectorTraderAddress,
    });

    expect(wrapped.programAddress).toBe(flight.FLIGHT_PROGRAM_ADDRESS);
    expect(Array.from(wrapped.data?.slice(0, 8) ?? [])).toEqual(
      Array.from(
        flight.FLIGHT_DISCRIMINANTS.PROXY_INSTRUCTION_WITH_FEE_OVERRIDE
      )
    );
    expect(wrapped.data?.[8]).toBe(1);
    expect(Array.from(wrapped.data?.slice(9, 17) ?? [])).toEqual([
      5, 0, 0, 0, 0, 0, 0, 0,
    ]);
    expect(Array.from(wrapped.data?.slice(17) ?? [])).toEqual(Array.from(data));
  });

  it("builds the low-level proxy instruction with a fee override", async () => {
    const data = new Uint8Array([...DISCRIMINANTS.PLACE_LIMIT_ORDER, 1, 2, 3]);

    const wrapped = await flight.buildProxyInstructionIx({
      phoenixProgramAddress,
      builderAuthority,
      builderTraderAccount: feeCollectorTrader,
      traderWallet: traderAuthority,
      feeBpsOverride: 7n,
      innerInstruction: {
        programAddress: phoenixProgramAddress,
        accounts: [],
        data,
      },
    });

    expect(wrapped.programAddress).toBe(flight.FLIGHT_PROGRAM_ADDRESS);
    expect(Array.from(wrapped.data?.slice(0, 8) ?? [])).toEqual(
      Array.from(
        flight.FLIGHT_DISCRIMINANTS.PROXY_INSTRUCTION_WITH_FEE_OVERRIDE
      )
    );
    expect(wrapped.data?.[8]).toBe(1);
    expect(Array.from(wrapped.data?.slice(9, 17) ?? [])).toEqual([
      7, 0, 0, 0, 0, 0, 0, 0,
    ]);
    expect(Array.from(wrapped.data?.slice(17) ?? [])).toEqual(Array.from(data));
  });

  it("builds position-authority proxy instructions with transfer permission accounts", async () => {
    const data = new Uint8Array([
      ...DISCRIMINANTS.PLACE_MARKET_ORDER_DELEGATED,
      1,
      2,
      3,
    ]);
    const collateralTransferAuthority = await getCollateralTransferAuthority();
    const collateralTransferPermissionAccount =
      await flight.getFlightAuthorizedCollateralTransferPermissionAddress(
        rootAuthority,
        phoenixProgramAddress
      );

    const wrapped = await flight.buildProxyInstructionIx({
      phoenixProgramAddress,
      builderAuthority,
      builderTraderAccount: feeCollectorTrader,
      traderWallet: traderAuthority,
      rootAuthority,
      innerInstruction: {
        programAddress: phoenixProgramAddress,
        accounts: [],
        data,
      },
    });

    expect(wrapped.programAddress).toBe(flight.FLIGHT_PROGRAM_ADDRESS);
    expect(wrapped.accounts.at(-2)?.address).toBe(collateralTransferAuthority);
    expect(wrapped.accounts.at(-1)?.address).toBe(
      collateralTransferPermissionAccount
    );
    expect(Array.from(wrapped.data?.slice(8) ?? [])).toEqual(Array.from(data));
  });

  it("scopes collateral-transfer authority PDAs to the target program", async () => {
    const otherTargetProgramAddress = address(
      "11111111111111111111111111111111"
    ) as never;

    const [authority, otherAuthority] = await Promise.all([
      getCollateralTransferAuthority(),
      flight.getFlightCollateralTransferAuthorityAddress(
        otherTargetProgramAddress
      ),
    ]);
    const [permission, otherPermission] = await Promise.all([
      flight.getFlightAuthorizedCollateralTransferPermissionAddress(
        rootAuthority,
        phoenixProgramAddress
      ),
      flight.getFlightAuthorizedCollateralTransferPermissionAddress(
        rootAuthority,
        otherTargetProgramAddress
      ),
    ]);

    expect(authority).not.toBe(otherAuthority);
    expect(permission).not.toBe(otherPermission);
  });

  it("builds delegated market proxy instructions without the tail unless position authority is declared", async () => {
    const wrapped = await flight.buildProxyInstructionIx({
      phoenixProgramAddress,
      builderAuthority,
      builderTraderAccount: feeCollectorTrader,
      traderWallet: traderAuthority,
      innerInstruction: {
        programAddress: phoenixProgramAddress,
        accounts: [],
        data: new Uint8Array(DISCRIMINANTS.PLACE_MARKET_ORDER_DELEGATED),
      },
    });

    expect(wrapped.programAddress).toBe(flight.FLIGHT_PROGRAM_ADDRESS);
    expect(wrapped.accounts.length).toBe(6);
  });

  it("rejects an invalid fee override", async () => {
    await expect(
      flight.buildProxyInstructionIx({
        phoenixProgramAddress,
        builderAuthority,
        builderTraderAccount: feeCollectorTrader,
        traderWallet: traderAuthority,
        feeBpsOverride: 10_001n,
        innerInstruction: {
          programAddress: phoenixProgramAddress,
          accounts: [],
          data: new Uint8Array(DISCRIMINANTS.PLACE_LIMIT_ORDER),
        },
      })
    ).rejects.toThrow("Invalid fee bps override (must be in 0..=10000)");
  });

  it("leaves unsupported instructions unchanged", async () => {
    const resolveFeeCollectorTraderAddress = vi.fn(
      async () => feeCollectorTrader
    );
    const data = new Uint8Array([...DISCRIMINANTS.CANCEL_CONDITIONAL_ORDER, 1]);
    const instruction = {
      programAddress: phoenixProgramAddress,
      accounts: [],
      data,
    };

    const wrapped = await flight.wrapInstructionWithFlight({
      phoenixInstruction: instruction,
      signer: traderAuthority,
      phoenixProgramAddress,
      flight: { builderAuthority },
      resolveFeeCollectorTraderAddress,
    });

    expect(wrapped).toBe(instruction);
    expect(resolveFeeCollectorTraderAddress).not.toHaveBeenCalled();
  });

  it("wraps delegated market orders without the tail unless position authority is declared", async () => {
    const resolveFeeCollectorTraderAddress = vi.fn(
      async () => feeCollectorTrader
    );
    const resolveRootAuthority = vi.fn(async () => rootAuthority);
    const collateralTransferAuthority = await getCollateralTransferAuthority();
    const collateralTransferPermissionAccount =
      await flight.getFlightAuthorizedCollateralTransferPermissionAddress(
        rootAuthority,
        phoenixProgramAddress
      );
    const data = new Uint8Array([
      ...DISCRIMINANTS.PLACE_MARKET_ORDER_DELEGATED,
      1,
    ]);
    const instruction = {
      programAddress: phoenixProgramAddress,
      accounts: [],
      data,
    };

    // Owner-signed delegated market orders wrap plainly: nothing is
    // appended and the root authority is never resolved.
    const plain = await flight.wrapInstructionWithFlight({
      phoenixInstruction: instruction,
      signer: traderAuthority,
      phoenixProgramAddress,
      flight: { builderAuthority },
      resolveFeeCollectorTraderAddress,
      resolveRootAuthority,
    });

    expect(plain).not.toBe(instruction);
    expect(plain.programAddress).toBe(flight.FLIGHT_PROGRAM_ADDRESS);
    expect(plain.accounts.length).toBe(6);
    expect(resolveRootAuthority).not.toHaveBeenCalled();

    const declared = await flight.wrapInstructionWithFlight({
      phoenixInstruction: instruction,
      signer: traderAuthority,
      phoenixProgramAddress,
      flight: { builderAuthority },
      usePositionAuthority: true,
      resolveFeeCollectorTraderAddress,
      resolveRootAuthority,
    });

    expect(declared.accounts.slice(0, -2)).toEqual(plain.accounts);
    expect(declared.accounts.at(-2)?.address).toBe(collateralTransferAuthority);
    expect(declared.accounts.at(-1)?.address).toBe(
      collateralTransferPermissionAccount
    );
    expect(resolveRootAuthority).toHaveBeenCalledOnce();
  });

  it("appends the collateral transfer tail to position-authority market order wraps", async () => {
    const resolveFeeCollectorTraderAddress = vi.fn(
      async () => feeCollectorTrader
    );
    const resolveRootAuthority = vi.fn(async () => rootAuthority);
    const collateralTransferAuthority = await getCollateralTransferAuthority();
    const collateralTransferPermissionAccount =
      await flight.getFlightAuthorizedCollateralTransferPermissionAddress(
        rootAuthority,
        phoenixProgramAddress
      );
    const instruction = {
      programAddress: phoenixProgramAddress,
      accounts: [],
      data: new Uint8Array([...DISCRIMINANTS.PLACE_MARKET_ORDER, 1, 2, 3]),
    };

    const plain = await flight.wrapInstructionWithFlight({
      phoenixInstruction: instruction,
      signer: traderAuthority,
      phoenixProgramAddress,
      flight: { builderAuthority },
      resolveFeeCollectorTraderAddress,
    });
    const wrapped = await flight.wrapInstructionWithFlight({
      phoenixInstruction: instruction,
      signer: traderAuthority,
      phoenixProgramAddress,
      flight: { builderAuthority },
      usePositionAuthority: true,
      resolveFeeCollectorTraderAddress,
      resolveRootAuthority,
    });

    expect(wrapped.accounts.slice(0, -2)).toEqual(plain.accounts);
    expect(wrapped.accounts.length).toBe(plain.accounts.length + 2);
    expect(wrapped.accounts.at(-2)?.address).toBe(collateralTransferAuthority);
    expect(wrapped.accounts.at(-2)?.role).toBe(AccountRole.READONLY);
    expect(wrapped.accounts.at(-1)?.address).toBe(
      collateralTransferPermissionAccount
    );
    expect(wrapped.accounts.at(-1)?.role).toBe(AccountRole.WRITABLE);
    expect(resolveRootAuthority).toHaveBeenCalledOnce();
  });

  it("rejects position-authority wraps without a root authority", async () => {
    await expect(
      flight.wrapInstructionWithFlight({
        phoenixInstruction: {
          programAddress: phoenixProgramAddress,
          accounts: [],
          data: new Uint8Array(DISCRIMINANTS.PLACE_MARKET_ORDER),
        },
        signer: traderAuthority,
        phoenixProgramAddress,
        flight: { builderAuthority },
        usePositionAuthority: true,
        resolveFeeCollectorTraderAddress: vi.fn(async () => feeCollectorTrader),
      })
    ).rejects.toThrow("Root authority is required");
  });

  describe("PhoenixFlightClient root authority sourcing", () => {
    const rotatedRootAuthority = address(
      "F952dz4aHVUu75YdxUrGhLejhCfzXCYVDysDw6yL6uT4"
    );

    const createExchangeMetadataStub = (getRootAuthority: () => string) => {
      const ready = vi.fn(async () => undefined);
      const snapshot = vi.fn(() => ({
        exchange: {
          currentAuthorities: { rootAuthority: getRootAuthority() },
        },
      }));
      return {
        metadata: { ready, snapshot } as unknown as PhoenixExchangeMetadata,
        ready,
        snapshot,
      };
    };

    const createInstructionClientStub = (
      exchange?: PhoenixExchangeMetadata
    ): PhoenixInstructionClient =>
      ({
        addresses: { phoenixProgramAddress },
        fetchAccount: async () => ({ data: new Uint8Array() }),
        exchange,
      }) as unknown as PhoenixInstructionClient;

    const delegatedInstruction = () => ({
      programAddress: phoenixProgramAddress,
      accounts: [],
      data: new Uint8Array([
        ...DISCRIMINANTS.PLACE_MARKET_ORDER_DELEGATED,
        1,
        2,
        3,
      ]),
    });

    it("resolves the root authority from the exchange snapshot on every wrap", async () => {
      let currentRootAuthority: string = rootAuthority;
      const exchange = createExchangeMetadataStub(() => currentRootAuthority);
      const flightClient = new flight.PhoenixFlightClient(
        createInstructionClientStub(exchange.metadata),
        { builderAuthority }
      );

      const firstWrap = await flightClient.tryWrapOrderInstruction(
        delegatedInstruction(),
        traderAuthority,
        true
      );
      currentRootAuthority = rotatedRootAuthority;
      const secondWrap = await flightClient.tryWrapOrderInstruction(
        delegatedInstruction(),
        traderAuthority,
        true
      );

      expect(firstWrap.accounts.at(-1)?.address).toBe(
        await flight.getFlightAuthorizedCollateralTransferPermissionAddress(
          rootAuthority,
          phoenixProgramAddress
        )
      );
      expect(secondWrap.accounts.at(-1)?.address).toBe(
        await flight.getFlightAuthorizedCollateralTransferPermissionAddress(
          rotatedRootAuthority,
          phoenixProgramAddress
        )
      );
      expect(exchange.ready).toHaveBeenCalledTimes(2);
      expect(exchange.snapshot).toHaveBeenCalledTimes(2);
    });

    it("resolves position-authority wrap tails from the exchange snapshot", async () => {
      const exchange = createExchangeMetadataStub(() => rootAuthority);
      const flightClient = new flight.PhoenixFlightClient(
        createInstructionClientStub(exchange.metadata),
        { builderAuthority }
      );

      const wrapped = await flightClient.tryWrapOrderInstruction(
        {
          programAddress: phoenixProgramAddress,
          accounts: [],
          data: new Uint8Array([...DISCRIMINANTS.PLACE_MARKET_ORDER, 1]),
        },
        traderAuthority,
        true
      );

      expect(wrapped.accounts.at(-2)?.address).toBe(
        await getCollateralTransferAuthority()
      );
      expect(wrapped.accounts.at(-1)?.address).toBe(
        await flight.getFlightAuthorizedCollateralTransferPermissionAddress(
          rootAuthority,
          phoenixProgramAddress
        )
      );
      expect(exchange.ready).toHaveBeenCalledOnce();
      expect(exchange.snapshot).toHaveBeenCalledOnce();
    });

    it("appends the tail only when usePositionAuthority is set", async () => {
      const exchange = createExchangeMetadataStub(() => rootAuthority);
      const flightClient = new flight.PhoenixFlightClient(
        createInstructionClientStub(exchange.metadata),
        { builderAuthority }
      );
      const delegateSigner = rotatedRootAuthority;

      // Default (owner-signed): plain wrap, no tail, no root-authority
      // resolution.
      const ownerWrap = await flightClient.tryWrapOrderInstruction(
        delegatedInstruction(),
        traderAuthority
      );
      expect(ownerWrap.accounts.length).toBe(6);
      expect(ownerWrap.accounts[5]?.address).toBe(traderAuthority);
      expect(exchange.snapshot).not.toHaveBeenCalled();

      // usePositionAuthority (derived by callers as signer !== owner):
      // position-authority path with the tail appended.
      const delegateWrap = await flightClient.tryWrapOrderInstruction(
        delegatedInstruction(),
        delegateSigner,
        true
      );
      expect(delegateWrap.accounts.length).toBe(8);
      expect(delegateWrap.accounts[5]?.address).toBe(delegateSigner);
      expect(delegateWrap.accounts.at(-2)?.address).toBe(
        await getCollateralTransferAuthority()
      );
      expect(delegateWrap.accounts.at(-1)?.address).toBe(
        await flight.getFlightAuthorizedCollateralTransferPermissionAddress(
          rootAuthority,
          phoenixProgramAddress
        )
      );
      expect(exchange.snapshot).toHaveBeenCalledOnce();
    });

    it("wraps owner-signed delegated market orders without exchange metadata", async () => {
      const flightClient = new flight.PhoenixFlightClient(
        createInstructionClientStub(undefined),
        { builderAuthority }
      );

      const wrapped = await flightClient.tryWrapOrderInstruction(
        delegatedInstruction(),
        traderAuthority
      );

      expect(wrapped.programAddress).toBe(flight.FLIGHT_PROGRAM_ADDRESS);
      expect(wrapped.accounts.length).toBe(6);
    });

    it("throws for position-authority wraps without exchange metadata", async () => {
      const flightClient = new flight.PhoenixFlightClient(
        createInstructionClientStub(undefined),
        { builderAuthority }
      );

      await expect(
        flightClient.tryWrapOrderInstruction(
          {
            programAddress: phoenixProgramAddress,
            accounts: [],
            data: new Uint8Array([...DISCRIMINANTS.PLACE_MARKET_ORDER, 1]),
          },
          traderAuthority,
          true
        )
      ).rejects.toThrow(
        "Flight position-authority orders require exchange metadata to resolve the root authority"
      );
    });

    it("wraps plain orders without exchange metadata", async () => {
      const flightClient = new flight.PhoenixFlightClient(
        createInstructionClientStub(undefined),
        { builderAuthority }
      );
      const data = new Uint8Array([...DISCRIMINANTS.PLACE_LIMIT_ORDER, 1]);

      const wrapped = await flightClient.tryWrapOrderInstruction(
        {
          programAddress: phoenixProgramAddress,
          accounts: [],
          data,
        },
        traderAuthority
      );

      expect(wrapped.programAddress).toBe(flight.FLIGHT_PROGRAM_ADDRESS);
      expect(wrapped.accounts[2]?.address).toBe(builderAuthority);
      expect(Array.from(wrapped.data?.slice(8) ?? [])).toEqual(
        Array.from(data)
      );
    });
  });

  describe("ixs client position-authority routing", () => {
    const ownerAuthority = traderAuthority;
    const delegateSigner = address(
      "F952dz4aHVUu75YdxUrGhLejhCfzXCYVDysDw6yL6uT4"
    );
    // Proxy layout: [0..5] proxy accounts with the signer at [5], inner
    // instruction accounts from [6]. Inner PlaceMarketOrder puts the trader
    // wallet at [3] and the trader PDA at [4]; PlaceMarketOrderDelegated puts
    // the wallet at [3], the permission at [4], and the trader PDA at [5].
    const PROXY_SIGNER = 5;
    const INNER = 6;

    const orderPacket: ImmediateOrCancelOrderPacket = {
      side: Side.Bid,
      priceInTicks: ticks(100n),
      numBaseLots: baseLots(1n),
      numQuoteLots: null,
      minBaseLotsToFill: baseLots(1n),
      minQuoteLotsToFill: quoteLots(1n),
      selfTradeBehavior: SelfTradeBehavior.Abort,
      matchLimit: null,
      clientOrderId: 0n,
      lastValidSlot: null,
      orderFlags: OrderFlags.None,
      cancelExisting: false,
    };

    const createIxClient = () =>
      createPhoenixIxClient({
        exchange: {
          ready: async () => undefined,
          snapshot: () => ({
            markets: [{ symbol: "SOL-PERP" }],
            exchange: { currentAuthorities: { rootAuthority } },
          }),
          instructionContext: (symbol: string) =>
            symbol === "SOL-PERP"
              ? {
                  exchange: {
                    globalConfig: "global-config",
                    perpAssetMap: "perp-asset-map",
                    globalTraderIndex: ["gti-0"],
                    activeTraderBuffer: ["atb-0"],
                  },
                  market: {
                    marketPubkey: "market-address",
                    splinePubkey: "spline-address",
                  },
                }
              : undefined,
        } as unknown as PhoenixExchangeMetadata,
        pda: createPhoenixPdaClient({
          programAddress: phoenixProgramAddress as never,
        }),
        orderPackets: {} as never,
        flight: { builderAuthority },
      });

    const expectedOwnerTraderAccount = () =>
      getPhoenixTraderSubaccountAddress({
        authority: ownerAuthority as never,
        traderPdaIndex: 0,
        subaccountIndex: 0,
        phoenixProgramAddress: phoenixProgramAddress as never,
      });

    it("wraps owner-signed market orders plainly, treating positionAuthority === authority as owner-signed", async () => {
      const client = createIxClient();

      const defaultIx = await client.placeMarketOrder({
        authority: ownerAuthority as never,
        symbol: "SOL-PERP" as never,
        orderPacket,
      });
      const explicitIx = await client.placeMarketOrder({
        authority: ownerAuthority as never,
        positionAuthority: ownerAuthority as never,
        symbol: "SOL-PERP" as never,
        orderPacket,
      });

      for (const ix of [defaultIx, explicitIx]) {
        expect(ix.programAddress).toBe(flight.FLIGHT_PROGRAM_ADDRESS);
        expect(ix.accounts[PROXY_SIGNER]?.address).toBe(ownerAuthority);
        expect(ix.accounts[INNER + 3]?.address).toBe(ownerAuthority);
        expect(ix.accounts[INNER + 4]?.address).toBe(
          await expectedOwnerTraderAccount()
        );
        expect(ix.accounts.at(-2)?.address).not.toBe(
          await getCollateralTransferAuthority()
        );
      }
      expect(explicitIx.accounts.length).toBe(defaultIx.accounts.length);
    });

    it("routes delegate-signed market orders through the position-authority path with the owner's trader PDA", async () => {
      const client = createIxClient();

      const plainIx = await client.placeMarketOrder({
        authority: ownerAuthority as never,
        symbol: "SOL-PERP" as never,
        orderPacket,
      });
      const delegateIx = await client.placeMarketOrder({
        authority: ownerAuthority as never,
        positionAuthority: delegateSigner as never,
        symbol: "SOL-PERP" as never,
        orderPacket,
      });

      expect(delegateIx.accounts.length).toBe(plainIx.accounts.length + 2);
      expect(delegateIx.accounts[PROXY_SIGNER]?.address).toBe(delegateSigner);
      expect(delegateIx.accounts[INNER + 3]?.address).toBe(delegateSigner);
      // The trader PDA still derives from the owner even though the
      // delegate signs.
      expect(delegateIx.accounts[INNER + 4]?.address).toBe(
        await expectedOwnerTraderAccount()
      );
      expect(delegateIx.accounts.at(-2)?.address).toBe(
        await getCollateralTransferAuthority()
      );
      expect(delegateIx.accounts.at(-1)?.address).toBe(
        await flight.getFlightAuthorizedCollateralTransferPermissionAddress(
          rootAuthority,
          phoenixProgramAddress
        )
      );
    });

    it("auto-declares position authority for delegate-signed delegated market orders", async () => {
      const client = createIxClient();
      const permissionAccount = address(
        "9n4nbM75f5Ui33ZbPYXn59EwSgE8CGsHtAeTH5YFeJ9E"
      );

      const ownerIx = await client.placeMarketOrderDelegated({
        authority: ownerAuthority as never,
        symbol: "SOL-PERP" as never,
        orderPacket,
      });
      const delegateIx = await client.placeMarketOrderDelegated({
        authority: ownerAuthority as never,
        positionAuthority: delegateSigner as never,
        permissionAccount,
        symbol: "SOL-PERP" as never,
        orderPacket,
      });

      // Owner-signed delegated market orders carry no tail.
      expect(ownerIx.accounts.at(-2)?.address).not.toBe(
        await getCollateralTransferAuthority()
      );
      expect(delegateIx.accounts.length).toBe(ownerIx.accounts.length + 2);
      expect(delegateIx.accounts[PROXY_SIGNER]?.address).toBe(delegateSigner);
      expect(delegateIx.accounts[INNER + 3]?.address).toBe(delegateSigner);
      expect(delegateIx.accounts[INNER + 4]?.address).toBe(permissionAccount);
      expect(delegateIx.accounts[INNER + 5]?.address).toBe(
        await expectedOwnerTraderAccount()
      );
      expect(delegateIx.accounts.at(-2)?.address).toBe(
        await getCollateralTransferAuthority()
      );
      expect(delegateIx.accounts.at(-1)?.address).toBe(
        await flight.getFlightAuthorizedCollateralTransferPermissionAddress(
          rootAuthority,
          phoenixProgramAddress
        )
      );
    });
  });

  describe("flow builders position-authority routing", () => {
    const ownerAuthority = traderAuthority;
    const delegateSigner = address(
      "F952dz4aHVUu75YdxUrGhLejhCfzXCYVDysDw6yL6uT4"
    );
    const marketAccount = address(
      "9n4nbM75f5Ui33ZbPYXn59EwSgE8CGsHtAeTH5YFeJ9E"
    );
    // Proxy layout: [0..5] proxy accounts with the signer at [5], inner
    // instruction accounts from [6]. Inner PlaceLimitOrder/PlaceMarketOrder
    // put the trader wallet at [3].
    const PROXY_SIGNER = 5;
    const INNER = 6;

    const exchangeSnapshot = {
      markets: [{ symbol: "SOL-PERP" }],
      exchange: {
        globalConfig: "global-config",
        currentAuthorities: {
          rootAuthority,
          riskAuthority: rootAuthority,
          marketAuthority: rootAuthority,
          oracleAuthority: rootAuthority,
          adlAuthority: rootAuthority,
          cancelAuthority: rootAuthority,
          backstopAuthority: rootAuthority,
        },
        canonicalMint: "canonical-mint",
        globalVault: "global-vault",
        perpAssetMap: "perp-asset-map",
        globalTraderIndex: ["gti-0"],
        activeTraderBuffer: ["atb-0"],
        withdrawQueue: "withdraw-queue",
        exchangeStatusBits: 0,
      },
    };

    const exchangeMetadata = {
      ready: async () => exchangeSnapshot,
      snapshot: () => exchangeSnapshot,
      instructionContext: (symbol: string) =>
        symbol === "SOL-PERP"
          ? {
              market: {
                marketPubkey: marketAccount,
                assetId: 0,
                tickSize: 0.001,
                baseLotsDecimals: 3,
                splinePubkey: "spline-address",
              },
            }
          : undefined,
    } as unknown as PhoenixExchangeMetadata;

    const createFlowFlightClient = () =>
      new flight.PhoenixFlightClient(
        {
          addresses: {
            phoenixProgramAddress,
            logAuthorityAddress: "log-authority",
            globalConfigurationAddress: "global-config",
          },
          fetchAccount: async () => ({ data: new Uint8Array() }),
          exchange: exchangeMetadata,
        } as unknown as PhoenixInstructionClient,
        { builderAuthority }
      );

    it("wraps delegate-signed limit-order flows through the position-authority path", async () => {
      const client = createFlowFlightClient();

      const ownerFlow = await buildPlaceLimitOrderFlow(
        {
          authority: ownerAuthority as never,
          symbol: "SOL-PERP" as never,
          side: Side.Bid,
          priceInTicks: ticks(100n),
          numBaseLots: baseLots(1n),
          marginType: MarginType.Cross,
          subaccountIndex: 0,
        },
        client
      );
      const delegateFlow = await buildPlaceLimitOrderFlow(
        {
          authority: ownerAuthority as never,
          positionAuthority: delegateSigner as never,
          symbol: "SOL-PERP" as never,
          side: Side.Bid,
          priceInTicks: ticks(100n),
          numBaseLots: baseLots(1n),
          marginType: MarginType.Cross,
          subaccountIndex: 0,
        },
        client
      );

      const ownerIx = ownerFlow.named.placeLimitOrder;
      const delegateIx = delegateFlow.named.placeLimitOrder;

      expect(ownerIx.programAddress).toBe(flight.FLIGHT_PROGRAM_ADDRESS);
      expect(ownerIx.accounts[PROXY_SIGNER]?.address).toBe(ownerAuthority);
      expect(ownerIx.accounts[INNER + 3]?.address).toBe(ownerAuthority);
      expect(ownerIx.accounts.at(-1)?.address).not.toBe(
        await flight.getFlightAuthorizedCollateralTransferPermissionAddress(
          rootAuthority,
          phoenixProgramAddress
        )
      );

      // The delegate both signs the inner instruction and sits in the proxy
      // trader-wallet slot, and the collateral-transfer tail is appended.
      expect(delegateIx.programAddress).toBe(flight.FLIGHT_PROGRAM_ADDRESS);
      expect(delegateIx.accounts.length).toBe(ownerIx.accounts.length + 2);
      expect(delegateIx.accounts[PROXY_SIGNER]?.address).toBe(delegateSigner);
      expect(delegateIx.accounts[INNER + 3]?.address).toBe(delegateSigner);
      expect(delegateIx.accounts.at(-2)?.address).toBe(
        await getCollateralTransferAuthority()
      );
      expect(delegateIx.accounts.at(-1)?.address).toBe(
        await flight.getFlightAuthorizedCollateralTransferPermissionAddress(
          rootAuthority,
          phoenixProgramAddress
        )
      );
    });

    it("wraps delegate-signed market-order flows through the position-authority path", async () => {
      const client = createFlowFlightClient() as PhoenixInstructionClient &
        PhoenixMarketDataClient;

      const delegateFlow = await buildPlaceMarketOrderFlow(
        {
          authority: ownerAuthority as never,
          positionAuthority: delegateSigner as never,
          symbol: "SOL-PERP" as never,
          side: Side.Bid,
          numBaseLots: baseLots(1n),
          marginType: MarginType.Cross,
          subaccountIndex: 0,
          priceInTicksLimit: ticks(100n),
        },
        client
      );

      const delegateIx = delegateFlow.named.placeMarketOrder;
      expect(delegateIx.programAddress).toBe(flight.FLIGHT_PROGRAM_ADDRESS);
      expect(delegateIx.accounts[PROXY_SIGNER]?.address).toBe(delegateSigner);
      expect(delegateIx.accounts[INNER + 3]?.address).toBe(delegateSigner);
      expect(delegateIx.accounts.at(-2)?.address).toBe(
        await getCollateralTransferAuthority()
      );
      expect(delegateIx.accounts.at(-1)?.address).toBe(
        await flight.getFlightAuthorizedCollateralTransferPermissionAddress(
          rootAuthority,
          phoenixProgramAddress
        )
      );
    });
  });
});
