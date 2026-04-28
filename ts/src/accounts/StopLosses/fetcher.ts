import type { Address } from "@solana/kit";
import {
  createAccountFetcher,
  type AccountFetcherFor,
} from "../fetcherFactory";
import { getStopLossesDecoder } from "./codec";
import type { StopLosses } from "./types";

export const fetchStopLosses: AccountFetcherFor<StopLosses, Address> =
  createAccountFetcher<StopLosses, Address>(getStopLossesDecoder);
