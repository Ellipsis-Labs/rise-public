export type { GlobalConfiguration } from "./types";

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
