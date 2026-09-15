import type { GlobalConfiguration } from "./types";

export type ExchangeRunningState =
  | "unknown"
  | "active"
  | "maintenance"
  | "suspended";

const INITIALIZED_FLAG = 1 << 7;
const ACTIVE_FLAG = 1 << 0;
const MAINTENANCE_FLAG = 1 << 2;

export const getExchangeRunningState = (params: {
  globalConfiguration: Pick<
    GlobalConfiguration,
    "exchangeStatus" | "acknowledgedRestartSlot"
  >;
  lastRestartSlot: bigint | null;
}): ExchangeRunningState => {
  const { globalConfiguration, lastRestartSlot } = params;
  const status = globalConfiguration.exchangeStatus;
  if ((status & INITIALIZED_FLAG) === 0) return "unknown";
  if ((status & ACTIVE_FLAG) === 0) return "suspended";
  if ((status & MAINTENANCE_FLAG) !== 0) return "maintenance";
  if (
    lastRestartSlot === null ||
    lastRestartSlot !== globalConfiguration.acknowledgedRestartSlot
  ) {
    return "maintenance";
  }
  return "active";
};

export const isExchangeEffectivelyActive = (params: {
  globalConfiguration: Pick<
    GlobalConfiguration,
    "exchangeStatus" | "acknowledgedRestartSlot"
  >;
  lastRestartSlot: bigint | null;
}): boolean => getExchangeRunningState(params) === "active";
