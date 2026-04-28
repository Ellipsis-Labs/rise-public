import type { TokenAccountAddress } from "@/primitives/_addressTypes";
import {
  createAccountFetcher,
  type AccountFetcherFor,
} from "../fetcherFactory";
import { getTokenAccountDecoder } from "./codec";
import type { TokenAccount } from "./types";

export const fetchTokenAccount: AccountFetcherFor<
  TokenAccount,
  TokenAccountAddress
> = createAccountFetcher<TokenAccount, TokenAccountAddress>(
  getTokenAccountDecoder
);
