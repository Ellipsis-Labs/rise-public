import { ACCOUNT_DISCRIMINANTS } from "@/core/discriminants";
import { Direction, StopLossOrderKind } from "@/primitives/StopLoss";
import { Side } from "@/primitives/Side";
import { getFIFOOrderIdDecoder } from "@/primitives/FIFOOrderId";
import { getBaseLotsDecoder, getTicksDecoder } from "@/primitives/_numberTypes";
import { getSequenceNumberDecoder } from "../internal";
import {
  createDecoder,
  getAddressDecoder,
  getConstantDecoder,
  getHiddenPrefixDecoder,
  getU32Decoder,
  getU64Decoder,
  getU8Decoder,
  type Decoder,
} from "@solana/kit";
import type {
  ConditionalOrder,
  ConditionalOrderCollection,
  ConditionalOrderCollectionHeader,
  ConditionalOrderTrigger,
} from "./types";

const CONDITIONAL_ORDER_SIZE = 112;
const MAX_CONDITIONAL_ORDER_CAPACITY = 192;

const getConditionalOrderTriggerDecoder =
  (): Decoder<ConditionalOrderTrigger> =>
    createDecoder({
      fixedSize: 24,
      read: (bytes, offset) => {
        const ticksDecoder = getTicksDecoder();
        const u8Decoder = getU8Decoder();

        let currentOffset = offset;

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

        const [positionSequenceNumber, afterPositionSequenceNumber] =
          u8Decoder.read(bytes, currentOffset);
        currentOffset = afterPositionSequenceNumber;

        const [flags, afterFlags] = u8Decoder.read(bytes, currentOffset);
        currentOffset = afterFlags + 6;

        return [
          {
            triggerPrice,
            executionPrice,
            positionSequenceNumber,
            isActive: (flags & 0x01) !== 0,
            executionDirection:
              (flags & 0x02) !== 0 ? Direction.GreaterThan : Direction.LessThan,
            tradeSide: (flags & 0x04) !== 0 ? Side.Bid : Side.Ask,
            orderKind:
              (flags & 0x08) !== 0
                ? StopLossOrderKind.Limit
                : StopLossOrderKind.IOC,
          },
          currentOffset,
        ];
      },
    });

const getConditionalOrderDecoder = (): Decoder<ConditionalOrder> =>
  createDecoder({
    fixedSize: CONDITIONAL_ORDER_SIZE,
    read: (bytes, offset) => {
      const u64Decoder = getU64Decoder();
      const u32Decoder = getU32Decoder();
      const u8Decoder = getU8Decoder();
      const baseLotsDecoder = getBaseLotsDecoder();
      const fifoOrderIdDecoder = getFIFOOrderIdDecoder();
      const triggerDecoder = getConditionalOrderTriggerDecoder();

      let currentOffset = offset;

      const [sequenceNumber, afterSequenceNumber] = u64Decoder.read(
        bytes,
        currentOffset
      );
      currentOffset = afterSequenceNumber;

      const [rawOrderId, afterOrderId] = fifoOrderIdDecoder.read(
        bytes,
        currentOffset
      );
      currentOffset = afterOrderId;

      const [maxSize, afterMaxSize] = baseLotsDecoder.read(
        bytes,
        currentOffset
      );
      currentOffset = afterMaxSize;

      const [fillableSize, afterFillableSize] = baseLotsDecoder.read(
        bytes,
        currentOffset
      );
      currentOffset = afterFillableSize;

      const [filledSize, afterFilledSize] = baseLotsDecoder.read(
        bytes,
        currentOffset
      );
      currentOffset = afterFilledSize;

      const [slot, afterSlot] = u64Decoder.read(bytes, currentOffset);
      currentOffset = afterSlot;

      const [assetId, afterAssetId] = u32Decoder.read(bytes, currentOffset);
      currentOffset = afterAssetId;

      const [flags, afterFlags] = u8Decoder.read(bytes, currentOffset);
      currentOffset = afterFlags;

      const [percent, afterPercent] = u8Decoder.read(bytes, currentOffset);
      currentOffset = afterPercent + 2;

      const [greaterTriggerOrder, afterGreaterTriggerOrder] =
        triggerDecoder.read(bytes, currentOffset);
      currentOffset = afterGreaterTriggerOrder;

      const [lessTriggerOrder, afterLessTriggerOrder] = triggerDecoder.read(
        bytes,
        currentOffset
      );
      currentOffset = afterLessTriggerOrder;

      const orderId =
        rawOrderId.priceInTicks === 0n && rawOrderId.orderSequenceNumber === 0n
          ? null
          : rawOrderId;

      return [
        {
          sequenceNumber,
          orderId,
          maxSize,
          fillableSize,
          filledSize,
          slot,
          assetId,
          usePercent: (flags & 0x01) !== 0,
          percent,
          greaterTriggerOrder,
          lessTriggerOrder,
          isActive: greaterTriggerOrder.isActive || lessTriggerOrder.isActive,
        },
        currentOffset,
      ];
    },
  });

export const getConditionalOrderCollectionDecoder =
  (): Decoder<ConditionalOrderCollection> =>
    getHiddenPrefixDecoder(
      createDecoder({
        read: (bytes, offset) => {
          const addressDecoder = getAddressDecoder();
          const sequenceNumberDecoder = getSequenceNumberDecoder();
          const u8Decoder = getU8Decoder();
          const orderDecoder = getConditionalOrderDecoder();

          let currentOffset = offset;

          const [traderKey, afterTraderKey] = addressDecoder.read(
            bytes,
            currentOffset
          );
          currentOffset = afterTraderKey;

          const [fundingKey, afterFundingKey] = addressDecoder.read(
            bytes,
            currentOffset
          );
          currentOffset = afterFundingKey;

          const [sequenceNumber, afterSequenceNumber] =
            sequenceNumberDecoder.read(bytes, currentOffset);
          currentOffset = afterSequenceNumber;

          const [len, afterLen] = u8Decoder.read(bytes, currentOffset);
          currentOffset = afterLen;

          const [capacity, afterCapacity] = u8Decoder.read(
            bytes,
            currentOffset
          );
          currentOffset = afterCapacity + 6;

          if (capacity === 0 || capacity > MAX_CONDITIONAL_ORDER_CAPACITY) {
            throw new Error(
              `ConditionalOrderCollection capacity ${capacity} is outside the valid range 1..=${MAX_CONDITIONAL_ORDER_CAPACITY}`
            );
          }

          const header: ConditionalOrderCollectionHeader = {
            traderKey,
            fundingKey,
            sequenceNumber,
            len,
            capacity,
          };

          const orders: ConditionalOrder[] = [];
          const activeOrderIndices: number[] = [];
          for (let index = 0; index < capacity; index += 1) {
            const [order, afterOrder] = orderDecoder.read(bytes, currentOffset);
            currentOffset = afterOrder;
            orders.push(order);
            if (index !== 0 && order.isActive) {
              activeOrderIndices.push(index);
            }
          }

          return [
            {
              header,
              orders,
              activeOrderIndices,
            },
            currentOffset,
          ];
        },
      }),
      [getConstantDecoder(ACCOUNT_DISCRIMINANTS.CONDITIONAL_ORDER_COLLECTION)]
    );

export const decodeConditionalOrderCollection = (
  bytes: Uint8Array | Readonly<Uint8Array>
): ConditionalOrderCollection =>
  getConditionalOrderCollectionDecoder().decode(bytes);
