import { ACCOUNT_DISCRIMINANTS } from "@/core/discriminants";
import {
  createDecoder,
  getAddressDecoder,
  getConstantDecoder,
  getHiddenPrefixDecoder,
  getI64Decoder,
  getU64Decoder,
  type Decoder,
} from "@solana/kit";
import type { Permission } from "./types";

export const getPermissionDecoder = (): Decoder<Permission> =>
  getHiddenPrefixDecoder(
    createDecoder({
      fixedSize: 160,
      read: (bytes, offset) => {
        const addr = getAddressDecoder();
        const u64 = getU64Decoder();
        const i64 = getI64Decoder();

        let pos = offset;
        const [permissionAuthority, afterAuthority] = addr.read(bytes, pos);
        pos = afterAuthority;
        const [delegatedKey, afterDelegatedKey] = addr.read(bytes, pos);
        pos = afterDelegatedKey + 8;
        const [permission, afterPermission] = u64.read(bytes, pos);
        pos = afterPermission;
        const [expiresAtTimestamp, afterExpires] = i64.read(bytes, pos);
        pos = afterExpires;
        const [allowedSignerActions, afterActions] = i64.read(bytes, pos);

        return [
          {
            permissionAuthority,
            delegatedKey,
            permission,
            expiresAtTimestamp,
            allowedSignerActions,
          },
          afterActions + 64,
        ];
      },
    }),
    [getConstantDecoder(ACCOUNT_DISCRIMINANTS.PERMISSION_ACCOUNT)]
  );

export const decodePermission = (
  bytes: Uint8Array | Readonly<Uint8Array>
): Permission => getPermissionDecoder().decode(bytes);
