import { DISCRIMINANTS } from "@/core/discriminants";
import { getCancelIdEncoder } from "@/primitives/CancelId";
import {
  getArrayEncoder,
  getConstantEncoder,
  getHiddenPrefixEncoder,
  getStructEncoder,
  type Encoder,
} from "@solana/kit";
import type { CancelOrdersById } from "./types";

export const getCancelOrdersByIdEncoder = (): Encoder<CancelOrdersById> =>
  getHiddenPrefixEncoder(
    getStructEncoder([["orderIds", getArrayEncoder(getCancelIdEncoder())]]),
    [getConstantEncoder(DISCRIMINANTS.CANCEL_ORDERS_BY_ID)]
  );
