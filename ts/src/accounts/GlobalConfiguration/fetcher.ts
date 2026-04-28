import type { Address } from "@solana/kit";
import {
  createAccountFetcher,
  type AccountFetcherClientWithAddresses,
  type AccountFetcherWithDefaultFor,
} from "../fetcherFactory";
import { getGlobalConfigurationDecoder } from "./codec";
import type { GlobalConfiguration } from "./types";

export const fetchGlobalConfiguration: AccountFetcherWithDefaultFor<
  GlobalConfiguration,
  AccountFetcherClientWithAddresses
> = async ({ client, address, skipCache }) =>
  createAccountFetcher<
    GlobalConfiguration,
    Address,
    AccountFetcherClientWithAddresses
  >(getGlobalConfigurationDecoder)({
    client,
    address: address ?? client.addresses.globalConfigurationAddress,
    skipCache,
  });
