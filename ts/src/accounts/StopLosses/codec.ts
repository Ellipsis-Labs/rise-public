import { ACCOUNT_DISCRIMINANTS } from "@/core/discriminants";
import { Direction, StopLossOrderKind } from "@/primitives/StopLoss";
import { Side } from "@/primitives/Side";
import { getBaseLotsDecoder, getTicksDecoder } from "@/primitives/_numberTypes";
import { getSequenceNumberDecoder } from "../internal";
import {
  createDecoder,
  getAddressDecoder,
  getConstantDecoder,
  getHiddenPrefixDecoder,
  getU64Decoder,
  getU8Decoder,
  type Decoder,
} from "@solana/kit";
import type { StopLoss, StopLosses } from "./types";

const STOP_LOSS_SIZE = 80;
const STOP_LOSSES_ACCOUNT_SIZE = 328;
const STOP_LOSSES_PADDING_SIZE = 64;

const getStopLossDecoder = (): Decoder<StopLoss> =>
  createDecoder({
    fixedSize: STOP_LOSS_SIZE,
    read: (bytes, offset) => {
      const u64Decoder = getU64Decoder();
      const ticksDecoder = getTicksDecoder();
      const baseLotsDecoder = getBaseLotsDecoder();
      const u8Decoder = getU8Decoder();

      let currentOffset = offset;

      const [sequenceNumber, afterSequenceNumber] = u64Decoder.read(
        bytes,
        currentOffset
      );
      currentOffset = afterSequenceNumber;

      const [triggerPrice, afterTriggerPrice] = ticksDecoder.read(
        bytes,
        currentOffset
      );
      currentOffset = afterTriggerPrice;

      const [executionPrice, afterExecutionPrice] = ticksDecoder.read(
        bytes,
        currentOffset
      );
      currentOffset = afterExecutionPrice;

      const [tradeSize, afterTradeSize] = baseLotsDecoder.read(
        bytes,
        currentOffset
      );
      currentOffset = afterTradeSize;

      const [slot, afterSlot] = u64Decoder.read(bytes, currentOffset);
      currentOffset = afterSlot;

      const [positionSequenceNumber, afterPositionSequenceNumber] =
        u8Decoder.read(bytes, currentOffset);
      currentOffset = afterPositionSequenceNumber;

      const [flags, afterFlags] = u8Decoder.read(bytes, currentOffset);
      currentOffset = afterFlags + 38;

      const isActive = (flags & 0x01) !== 0;
      const executionDirection =
        (flags & 0x02) !== 0 ? Direction.GreaterThan : Direction.LessThan;
      const tradeSide = (flags & 0x04) !== 0 ? Side.Bid : Side.Ask;
      const orderKind =
        (flags & 0x08) !== 0 ? StopLossOrderKind.Limit : StopLossOrderKind.IOC;

      return [
        {
          sequenceNumber,
          triggerPrice,
          executionPrice,
          tradeSize,
          slot,
          positionSequenceNumber,
          isActive,
          executionDirection,
          tradeSide,
          orderKind,
        },
        currentOffset,
      ];
    },
  });

export const getStopLossesDecoder = (): Decoder<StopLosses> =>
  getHiddenPrefixDecoder(
    createDecoder({
      fixedSize: STOP_LOSSES_ACCOUNT_SIZE - 8,
      read: (bytes, offset) => {
        const stopLossDecoder = getStopLossDecoder();
        const u64Decoder = getU64Decoder();
        const addressDecoder = getAddressDecoder();
        const sequenceNumberDecoder = getSequenceNumberDecoder();

        let currentOffset = offset;

        const [greaterThanStopLoss, afterGreaterThanStopLoss] =
          stopLossDecoder.read(bytes, currentOffset);
        currentOffset = afterGreaterThanStopLoss;

        const [lessThanStopLoss, afterLessThanStopLoss] = stopLossDecoder.read(
          bytes,
          currentOffset
        );
        currentOffset = afterLessThanStopLoss;

        const [initialized, afterInitialized] = u64Decoder.read(
          bytes,
          currentOffset
        );
        currentOffset = afterInitialized;

        const [sequenceNumber, afterSequenceNumber] =
          sequenceNumberDecoder.read(bytes, currentOffset);
        currentOffset = afterSequenceNumber;

        const [fundingKey, afterFundingKey] = addressDecoder.read(
          bytes,
          currentOffset
        );
        currentOffset = afterFundingKey;

        const [traderKey, afterTraderKey] = addressDecoder.read(
          bytes,
          currentOffset
        );
        currentOffset = afterTraderKey;

        const [assetId, afterAssetId] = u64Decoder.read(bytes, currentOffset);
        currentOffset = afterAssetId + STOP_LOSSES_PADDING_SIZE;

        return [
          {
            stopLosses: [greaterThanStopLoss, lessThanStopLoss],
            isInitialized: initialized !== 0n,
            sequenceNumber,
            fundingKey,
            traderKey,
            assetId,
          },
          currentOffset,
        ];
      },
    }),
    [getConstantDecoder(ACCOUNT_DISCRIMINANTS.STOP_LOSSES)]
  );

export const decodeStopLosses = (
  bytes: Uint8Array | Readonly<Uint8Array>
): StopLosses => getStopLossesDecoder().decode(bytes);
