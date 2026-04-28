import {
  combineCodec,
  getArrayDecoder,
  getArrayEncoder,
  getU8Decoder,
  getU8Encoder,
  transformDecoder,
  transformEncoder,
  type Codec,
  type Decoder,
  type Encoder,
} from "@solana/kit";

import type { Symbol } from "./types";

export const getSymbolEncoder = (): Encoder<Symbol> =>
  transformEncoder(
    getArrayEncoder(getU8Encoder(), { size: 16 }),
    (symbol: string) => {
      const output = Array<number>(16).fill(0);
      const limit = Math.min(symbol.length, 16);
      for (let i = 0; i < limit; i++) {
        const codePoint = symbol.charCodeAt(i);
        if (codePoint > 0x7f) {
          throw new Error("Symbol contains non-ASCII characters");
        }
        output[i] = codePoint;
      }
      return output;
    }
  );

export const getSymbolDecoder = (): Decoder<Symbol> =>
  transformDecoder(
    getArrayDecoder(getU8Decoder(), { size: 16 }),
    (bytes: number[]) => {
      let end = bytes.indexOf(0);
      if (end === -1) {
        end = bytes.length;
      }
      let result = "";
      for (let i = 0; i < end; i++) {
        const byte = bytes[i];
        if (byte === undefined) {
          break;
        }
        result += String.fromCharCode(byte);
      }
      return result as Symbol;
    }
  );

export const getSymbolCodec = (): Codec<Symbol> =>
  combineCodec(getSymbolEncoder(), getSymbolDecoder());
