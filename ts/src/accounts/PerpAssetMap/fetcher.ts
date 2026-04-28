import type { PerpAssetMapAddress } from "@/primitives/_addressTypes";
import {
  createAccountFetcher,
  type AccountFetcherFor,
} from "../fetcherFactory";
import { getPerpAssetMapDecoder } from "./codec";
import type { PerpAssetMap } from "./types";

export const fetchPerpAssetMap: AccountFetcherFor<
  PerpAssetMap,
  PerpAssetMapAddress
> = createAccountFetcher<PerpAssetMap, PerpAssetMapAddress>(
  getPerpAssetMapDecoder
);
