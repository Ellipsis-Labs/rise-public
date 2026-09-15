import { describe, expect, it } from "vitest";

import {
  ACCOUNT_DISCRIMINANTS,
  decodeGlobalConfiguration,
  getExchangeRunningState,
  isExchangeEffectivelyActive,
} from "../src";

const decodeConfiguration = (reservedByte = 0) => {
  const bytes = new Uint8Array(2560);
  bytes.set(ACCOUNT_DISCRIMINANTS.GLOBAL_CONFIGURATION);
  const view = new DataView(bytes.buffer);
  view.setUint8(504, 0b1000_0001);
  bytes.fill(reservedByte, 508, 512);
  view.setBigUint64(1096, 42n, true);
  return decodeGlobalConfiguration(bytes);
};

describe("exchange running state", () => {
  const activeConfiguration = decodeConfiguration();

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

  it("ignores reserved bytes when decoding and resolving status", () => {
    const globalConfiguration = decodeConfiguration(255);
    expect(globalConfiguration).toEqual(activeConfiguration);
    for (const configuration of [
      globalConfiguration,
      { ...globalConfiguration },
    ]) {
      expect(
        getExchangeRunningState({
          globalConfiguration: configuration,
          lastRestartSlot: 42n,
        })
      ).toBe("active");
      for (const lastRestartSlot of [43n, null]) {
        expect(
          getExchangeRunningState({
            globalConfiguration: configuration,
            lastRestartSlot,
          })
        ).toBe("maintenance");
      }
    }
  });

  it("requires a matching observed slot even when acknowledgement is zero", () => {
    const globalConfiguration = {
      exchangeStatus: 0b1000_0001,
      acknowledgedRestartSlot: 0n,
    };
    expect(
      isExchangeEffectivelyActive({ globalConfiguration, lastRestartSlot: 0n })
    ).toBe(true);
    for (const lastRestartSlot of [43n, null]) {
      expect(
        getExchangeRunningState({ globalConfiguration, lastRestartSlot })
      ).toBe("maintenance");
    }
  });

  it("resolves explicit maintenance and suspension before the restart interlock", () => {
    expect(
      getExchangeRunningState({
        globalConfiguration: {
          ...activeConfiguration,
          exchangeStatus: 0b1000_0101,
        },
        lastRestartSlot: 42n,
      })
    ).toBe("maintenance");
    expect(
      getExchangeRunningState({
        globalConfiguration: {
          ...activeConfiguration,
          exchangeStatus: 0b1000_0000,
        },
        lastRestartSlot: 42n,
      })
    ).toBe("suspended");
  });
});
