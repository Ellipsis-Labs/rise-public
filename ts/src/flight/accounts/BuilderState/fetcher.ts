import {
  createAccountFetcher,
  type AccountFetcherClient,
} from "@/accounts/fetcherFactory";
import { getFlightBuilderStateAddress } from "@/flight/pdas";
import type { FlightBuilderStateAddress } from "@/flight/types";
import type { Authority, PhoenixProgramAddress } from "@/primitives";
import { getBuilderStateDecoder } from "./codec";
import type { BuilderState } from "./types";

export interface FetchBuilderStateParams<
  TClient extends AccountFetcherClient = AccountFetcherClient,
> {
  client: TClient;
  address?: FlightBuilderStateAddress;
  authority?: Authority;
  phoenixProgramAddress?: PhoenixProgramAddress;
  skipCache?: boolean;
}

export const fetchBuilderState = async <
  TClient extends AccountFetcherClient = AccountFetcherClient,
>({
  client,
  address,
  authority,
  phoenixProgramAddress,
  skipCache,
}: FetchBuilderStateParams<TClient>): Promise<BuilderState> => {
  const resolvedAddress =
    address ??
    (authority
      ? await getFlightBuilderStateAddress(authority, phoenixProgramAddress)
      : undefined);

  if (!resolvedAddress) {
    throw new Error(
      "BuilderState fetch requires either an explicit address or an authority"
    );
  }

  return createAccountFetcher<BuilderState, FlightBuilderStateAddress, TClient>(
    getBuilderStateDecoder
  )({
    client,
    address: resolvedAddress,
    skipCache,
  });
};
