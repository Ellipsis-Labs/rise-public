import {
  createAccountFetcher,
  type AccountFetcherFor,
} from "../fetcherFactory";
import { getConditionalOrderCollectionDecoder } from "./codec";
import type { ConditionalOrderCollection } from "./types";
import type { Address } from "@solana/kit";

export const fetchConditionalOrderCollection: AccountFetcherFor<
  ConditionalOrderCollection,
  Address
> = createAccountFetcher<ConditionalOrderCollection, Address>(
  getConditionalOrderCollectionDecoder
);
