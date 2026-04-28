import type { TraderStateUpdate } from "./wire";

export type TraderStatePort = (
  authority: string,
  traderPdaIndex: number,
  signal?: AbortSignal
) => AsyncIterable<TraderStateUpdate>;
