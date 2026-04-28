import { ACCOUNT_DISCRIMINANTS } from "@/core/discriminants";
import type { Decoder } from "@solana/kit";
import {
  createDecoder,
  getConstantDecoder,
  getHiddenPrefixDecoder,
  getU32Decoder,
} from "@solana/kit";
import {
  getMarketAddressDecoder,
  getSequenceNumberDecoder,
  getSplineDecoder,
  getSymbolDecoder,
} from "../internal";
import type { SplineCollection } from "./types";
import type { Spline } from "../internal";

export const getSplineCollectionDecoder = (): Decoder<SplineCollection> =>
  getHiddenPrefixDecoder(
    createDecoder({
      read: (bytes, offset) => {
        const [market, afterMarket] = getMarketAddressDecoder().read(
          bytes,
          offset
        );
        const [assetSymbol, afterSymbol] = getSymbolDecoder().read(
          bytes,
          afterMarket
        );
        const [sequenceNumber, afterSequence] = getSequenceNumberDecoder().read(
          bytes,
          afterSymbol
        );
        const [numSplines, afterNumSplines] = getU32Decoder().read(
          bytes,
          afterSequence
        );
        const [numActive, afterNumActive] = getU32Decoder().read(
          bytes,
          afterNumSplines
        );
        let currentOffset = afterNumActive + 32;
        const splines: Spline[] = [];
        const splineDecoder = getSplineDecoder();
        for (let i = 0; i < numSplines; i++) {
          const [spline, nextOffset] = splineDecoder.read(bytes, currentOffset);
          splines.push(spline);
          currentOffset = nextOffset;
        }
        return [
          {
            market,
            assetSymbol,
            sequenceNumber,
            numSplines,
            numActive,
            splines,
          },
          currentOffset,
        ];
      },
    }),
    [getConstantDecoder(ACCOUNT_DISCRIMINANTS.SPLINE_COLLECTION)]
  );

export const decodeSplineCollection = (
  bytes: Uint8Array | Readonly<Uint8Array>
): SplineCollection => getSplineCollectionDecoder().decode(bytes);
