import type { RefreshableAsyncIterable } from "@/ws/adapters/_utils";
import type { ExchangeMsg } from "./wire";

export type ExchangeStream = RefreshableAsyncIterable<ExchangeMsg>;

export interface ExchangePort {
  (signal?: AbortSignal): ExchangeStream;
}
