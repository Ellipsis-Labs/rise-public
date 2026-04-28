import { FLIGHT_ACCOUNT_DISCRIMINANTS } from "@/flight/core/discriminants.js";
import type { PhoenixProgramAddress } from "@/primitives";
import { getAuthorityDecoder, getPubkeyDecoder } from "@/primitives";
import {
  createDecoder,
  getConstantDecoder,
  getHiddenPrefixDecoder,
  getU64Decoder,
  transformDecoder,
  type Decoder,
} from "@solana/kit";
import type { GlobalState } from "./types";

const getPhoenixProgramAddressDecoder = (): Decoder<PhoenixProgramAddress> =>
  transformDecoder(
    getPubkeyDecoder(),
    (programAddress): PhoenixProgramAddress =>
      programAddress as PhoenixProgramAddress
  );

const GLOBAL_STATE_DISCRIMINANT = getU64Decoder().decode(
  FLIGHT_ACCOUNT_DISCRIMINANTS.GLOBAL_STATE
);

export const getGlobalStateDecoder = (): Decoder<GlobalState> =>
  getHiddenPrefixDecoder(
    createDecoder({
      fixedSize: 360,
      read: (bytes, offset) => {
        const phoenixProgram = getPhoenixProgramAddressDecoder();
        const authority = getAuthorityDecoder();
        const u64 = getU64Decoder();

        let pos = offset;
        const [programId, afterProgramId] = phoenixProgram.read(bytes, pos);
        pos = afterProgramId;
        const [currentAuthority, afterCurrentAuthority] = authority.read(
          bytes,
          pos
        );
        pos = afterCurrentAuthority;
        const [pendingAuthority, afterPendingAuthority] = authority.read(
          bytes,
          pos
        );
        pos = afterPendingAuthority;
        const [maxFeeCapBps, afterMaxFeeCapBps] = u64.read(bytes, pos);

        return [
          {
            discriminant: GLOBAL_STATE_DISCRIMINANT,
            programId,
            currentAuthority,
            pendingAuthority,
            maxFeeCapBps,
          },
          afterMaxFeeCapBps + 256,
        ];
      },
    }),
    [getConstantDecoder(FLIGHT_ACCOUNT_DISCRIMINANTS.GLOBAL_STATE)]
  );

export const decodeGlobalState = (
  bytes: Uint8Array | Readonly<Uint8Array>
): GlobalState => getGlobalStateDecoder().decode(bytes);
