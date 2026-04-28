import { getAuthorityDecoder, getTraderAddressDecoder } from "@/primitives";
import {
  createDecoder,
  getConstantDecoder,
  getHiddenPrefixDecoder,
  getU64Decoder,
  type Decoder,
} from "@solana/kit";
import type { BuilderState } from "./types";
import { FLIGHT_ACCOUNT_DISCRIMINANTS } from "@/flight/core/discriminants.js";

const BUILDER_STATE_DISCRIMINANT = getU64Decoder().decode(
  FLIGHT_ACCOUNT_DISCRIMINANTS.BUILDER_STATE
);

export const getBuilderStateDecoder = (): Decoder<BuilderState> =>
  getHiddenPrefixDecoder(
    createDecoder({
      fixedSize: 208,
      read: (bytes, offset) => {
        const authority = getAuthorityDecoder();
        const trader = getTraderAddressDecoder();
        const u64 = getU64Decoder();

        let pos = offset;
        const [authorityKey, afterAuthority] = authority.read(bytes, pos);
        pos = afterAuthority;
        const [traderKey, afterTrader] = trader.read(bytes, pos);
        pos = afterTrader;
        const [status, afterStatus] = u64.read(bytes, pos);
        pos = afterStatus;
        const [feeBps, afterFeeBps] = u64.read(bytes, pos);

        return [
          {
            discriminant: BUILDER_STATE_DISCRIMINANT,
            authorityKey,
            traderKey,
            status,
            isActive: (status & 1n) !== 0n,
            feeBps,
          },
          afterFeeBps + 128,
        ];
      },
    }),
    [getConstantDecoder(FLIGHT_ACCOUNT_DISCRIMINANTS.BUILDER_STATE)]
  );

export const decodeBuilderState = (
  bytes: Uint8Array | Readonly<Uint8Array>
): BuilderState => getBuilderStateDecoder().decode(bytes);
