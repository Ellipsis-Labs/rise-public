import type { WithdrawQueueAddress } from "@/primitives/_addressTypes";
import {
  createAccountFetcher,
  type AccountFetcherFor,
} from "../fetcherFactory";
import { getWithdrawQueueHeaderDecoder } from "./codec";
import type { WithdrawQueueHeader } from "./types";

export const fetchWithdrawQueueHeader: AccountFetcherFor<
  WithdrawQueueHeader,
  WithdrawQueueAddress
> = createAccountFetcher<WithdrawQueueHeader, WithdrawQueueAddress>(
  getWithdrawQueueHeaderDecoder
);
