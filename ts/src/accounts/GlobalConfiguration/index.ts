export type { GlobalConfiguration } from "./types";
export { GLOBAL_FEATURE_NATIVE_SOL_COLLATERALS } from "./types";

export {
  decodeGlobalConfiguration,
  getGlobalConfigurationDecoder,
} from "./codec";

export { fetchGlobalConfiguration } from "./fetcher";

export {
  getExchangeRunningState,
  isExchangeEffectivelyActive,
  type ExchangeRunningState,
} from "./status";
