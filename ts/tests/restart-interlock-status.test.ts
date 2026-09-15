import { describe, expect, it } from "vitest";

import { isExchangeEffectivelyActive } from "../src";

describe("restart interlock status", () => {
  const activeConfiguration = {
    exchangeStatus: 0b1000_0001,
    acknowledgedRestartSlot: 42n,
  };

  it("requires the observed restart slot after acknowledgement", () => {
    expect(
      isExchangeEffectivelyActive({
        globalConfiguration: activeConfiguration,
        lastRestartSlot: 42n,
      })
    ).toBe(true);
    expect(
      isExchangeEffectivelyActive({
        globalConfiguration: activeConfiguration,
        lastRestartSlot: 43n,
      })
    ).toBe(false);
    expect(
      isExchangeEffectivelyActive({
        globalConfiguration: activeConfiguration,
        lastRestartSlot: null,
      })
    ).toBe(false);
  });

  it("preserves stored status behavior before acknowledgement", () => {
    expect(
      isExchangeEffectivelyActive({
        globalConfiguration: {
          ...activeConfiguration,
          acknowledgedRestartSlot: 0n,
        },
        lastRestartSlot: null,
      })
    ).toBe(true);
    expect(
      isExchangeEffectivelyActive({
        globalConfiguration: {
          ...activeConfiguration,
          exchangeStatus: 0b1000_0000,
        },
        lastRestartSlot: 42n,
      })
    ).toBe(false);
  });
});
