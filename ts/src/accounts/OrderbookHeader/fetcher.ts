import type { MarketAddress } from "@/primitives/_addressTypes";
import {
  createAccountFetcher,
  type AccountFetcherFor,
} from "../fetcherFactory";
import { getOrderbookHeaderDecoder } from "./codec";
import type { OrderbookHeader } from "./types";

export const fetchOrderbookHeader: AccountFetcherFor<
  OrderbookHeader,
  MarketAddress
> = createAccountFetcher<OrderbookHeader, MarketAddress>(
  getOrderbookHeaderDecoder
);
