import type { ReadonlyUint8Array } from "@solana/kit";
import {
  getSysvarLastRestartSlotDecoder,
  SYSVAR_LAST_RESTART_SLOT_ADDRESS,
} from "@solana/sysvars";

import type { AccountFetcherClient } from "./fetcherFactory";

const lastRestartSlotDecoder = getSysvarLastRestartSlotDecoder();

export const decodeLastRestartSlot = (data: ReadonlyUint8Array): bigint =>
  lastRestartSlotDecoder.decode(data).lastRestartSlot;

export const fetchLastRestartSlot = async (
  client: Pick<AccountFetcherClient, "fetchAccount">
): Promise<bigint> => {
  const account = await client.fetchAccount(SYSVAR_LAST_RESTART_SLOT_ADDRESS);
  return decodeLastRestartSlot(account.data);
};
