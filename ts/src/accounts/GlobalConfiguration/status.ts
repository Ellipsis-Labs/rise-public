import type { GlobalConfiguration } from "./types";

export const isExchangeEffectivelyActive = (params: {
  globalConfiguration: Pick<
    GlobalConfiguration,
    "exchangeStatus" | "acknowledgedRestartSlot"
  >;
  lastRestartSlot: bigint | null;
}): boolean => {
  const { globalConfiguration, lastRestartSlot } = params;
  const storedActive =
    (globalConfiguration.exchangeStatus & 0b1000_0000) !== 0 &&
    (globalConfiguration.exchangeStatus & 0b0000_0001) !== 0;
  return (
    storedActive &&
    (globalConfiguration.acknowledgedRestartSlot === 0n ||
      lastRestartSlot === globalConfiguration.acknowledgedRestartSlot)
  );
};
