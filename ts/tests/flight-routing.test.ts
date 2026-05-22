import { DISCRIMINANTS } from "@/core/discriminants";
import { flight } from "@/index";
import { address } from "@solana/kit";
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

  it("wraps Flight-routable placement instructions", async () => {
    const resolveFeeCollectorTraderAddress = vi.fn(
      async () => feeCollectorTrader
    );

    for (const discriminant of [
      DISCRIMINANTS.PLACE_MARKET_ORDER,
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
        authority: traderAuthority,
        phoenixProgramAddress,
        flight: { builderAuthority },
        resolveFeeCollectorTraderAddress,
      });

      expect(wrapped.programAddress).toBe(flight.FLIGHT_PROGRAM_ADDRESS);
      expect(wrapped.accounts[2]?.address).toBe(builderAuthority);
      expect(wrapped.accounts[3]?.address).toBe(feeCollectorTrader);
      expect(wrapped.accounts[5]?.address).toBe(traderAuthority);
      expect(Array.from(wrapped.data?.slice(8) ?? [])).toEqual(
        Array.from(data)
      );
    }

    expect(resolveFeeCollectorTraderAddress).toHaveBeenCalledTimes(6);
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
      authority: traderAuthority,
      phoenixProgramAddress,
      flight: { builderAuthority },
      resolveFeeCollectorTraderAddress,
    });

    expect(wrapped).toBe(instruction);
    expect(resolveFeeCollectorTraderAddress).not.toHaveBeenCalled();
  });
});
