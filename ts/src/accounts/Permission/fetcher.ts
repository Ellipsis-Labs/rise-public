import {
  createAccountFetcher,
  type AccountFetcherFor,
} from "../fetcherFactory";
import { getPermissionDecoder } from "./codec";
import type { Permission } from "./types";
import type { Address } from "@solana/kit";

export const fetchPermission: AccountFetcherFor<Permission, Address> =
  createAccountFetcher<Permission, Address>(getPermissionDecoder);
