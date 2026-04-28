import type { AllMidsUpdate } from "./wire";

export type AllMidsPort = (
  signal?: AbortSignal
) => AsyncIterable<AllMidsUpdate>;
