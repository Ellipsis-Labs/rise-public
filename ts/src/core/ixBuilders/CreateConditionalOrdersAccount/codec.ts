import { DISCRIMINANTS } from "@/core/discriminants";
import {
  combineCodec,
  getConstantDecoder,
  getConstantEncoder,
  getHiddenPrefixDecoder,
  getHiddenPrefixEncoder,
  getU8Decoder,
  getU8Encoder,
  type Codec,
  type Decoder,
  type Encoder,
} from "@solana/kit";

export const getCreateConditionalOrdersAccountEncoder = (): Encoder<number> =>
  getHiddenPrefixEncoder(getU8Encoder(), [
    getConstantEncoder(DISCRIMINANTS.CREATE_CONDITIONAL_ORDERS_ACCOUNT),
  ]);

export const getCreateConditionalOrdersAccountDecoder = (): Decoder<number> =>
  getHiddenPrefixDecoder(getU8Decoder(), [
    getConstantDecoder(DISCRIMINANTS.CREATE_CONDITIONAL_ORDERS_ACCOUNT),
  ]);

export const getCreateConditionalOrdersAccountCodec = (): Codec<number> =>
  combineCodec(
    getCreateConditionalOrdersAccountEncoder(),
    getCreateConditionalOrdersAccountDecoder()
  );
