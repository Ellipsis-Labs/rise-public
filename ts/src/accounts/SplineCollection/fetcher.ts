import type { SplineCollectionAddress } from "@/primitives/_addressTypes";
import {
  createAccountFetcher,
  type AccountFetcherFor,
} from "../fetcherFactory";
import { getSplineCollectionDecoder } from "./codec";
import type { SplineCollection } from "./types";

export const fetchSplineCollection: AccountFetcherFor<
  SplineCollection,
  SplineCollectionAddress
> = createAccountFetcher<SplineCollection, SplineCollectionAddress>(
  getSplineCollectionDecoder
);
