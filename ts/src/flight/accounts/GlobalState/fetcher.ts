import {
  createAccountFetcher,
  type AccountFetcherClient,
} from "@/accounts/fetcherFactory";
import { getFlightGlobalStateAddress } from "@/flight/pdas";
import type { FlightGlobalStateAddress } from "@/flight/types";
import type { PhoenixProgramAddress } from "@/primitives";
import { getGlobalStateDecoder } from "./codec";
import type { GlobalState } from "./types";

export interface FetchGlobalStateParams<
  TClient extends AccountFetcherClient = AccountFetcherClient,
> {
  client: TClient;
  address?: FlightGlobalStateAddress;
  phoenixProgramAddress?: PhoenixProgramAddress;
  skipCache?: boolean;
}

export const fetchGlobalState = async <
  TClient extends AccountFetcherClient = AccountFetcherClient,
>({
  client,
  address,
  phoenixProgramAddress,
  skipCache,
}: FetchGlobalStateParams<TClient>): Promise<GlobalState> => {
  const resolvedAddress =
    address ?? (await getFlightGlobalStateAddress(phoenixProgramAddress));

  return createAccountFetcher<GlobalState, FlightGlobalStateAddress, TClient>(
    getGlobalStateDecoder
  )({
    client,
    address: resolvedAddress,
    skipCache,
  });
};
