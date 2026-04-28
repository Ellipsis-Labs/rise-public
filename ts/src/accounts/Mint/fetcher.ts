import type { MintAddress } from "@/primitives/_addressTypes";
import {
  createAccountFetcher,
  type AccountFetcherFor,
} from "../fetcherFactory";
import { getMintDecoder } from "./codec";
import type { Mint } from "./types";

export const fetchMint: AccountFetcherFor<Mint, MintAddress> =
  createAccountFetcher<Mint, MintAddress>(getMintDecoder);
