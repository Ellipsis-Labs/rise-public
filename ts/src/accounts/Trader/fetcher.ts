import type { TraderAddress } from "@/primitives/_addressTypes";
import {
  createAccountFetcher,
  type AccountFetcherFor,
} from "../fetcherFactory";
import { getTraderDecoder } from "./codec";
import type { Trader } from "./types";

export const fetchTrader: AccountFetcherFor<Trader, TraderAddress> =
  createAccountFetcher<Trader, TraderAddress>(getTraderDecoder);
