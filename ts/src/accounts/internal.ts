import type {
  Authority,
  MarketAddress,
  TraderAddress,
} from "@/primitives/_addressTypes";
import {
  getActiveTraderBufferAddressHeaderDecoder,
  getAuthorityDecoder,
  getGlobalConfigurationAddressDecoder,
  getGlobalTraderIndexAddressHeaderDecoder,
  getGlobalVaultAddressDecoder,
  getMarketAddressDecoder,
  getMintAddressDecoder,
  getPerpAssetMapAddressDecoder,
  getTraderAddressDecoder,
  getWithdrawQueueAddressDecoder,
} from "@/primitives/_addressTypes";
import type {
  BaseLots,
  QuoteLots,
  SignedBaseLots,
  SignedQuoteLots,
  Ticks,
} from "@/primitives/_numberTypes";
import {
  getBaseLotsDecoder,
  getQuoteLotsDecoder,
  getSignedBaseLotsDecoder,
  getSignedQuoteLotsDecoder,
  getTicksDecoder,
} from "@/primitives/_numberTypes";
import type { FIFOOrderId } from "@/primitives/FIFOOrderId";
import { getFIFOOrderIdDecoder } from "@/primitives/FIFOOrderId";
import { OrderFlags, getOrderFlagsDecoder } from "@/primitives/OrderPacket";
import type { Symbol } from "@/primitives/Symbol";
import { getSymbolDecoder } from "@/primitives/Symbol";
import {
  createDecoder,
  getBooleanDecoder,
  getBytesDecoder,
  getI128Decoder,
  getI32Decoder,
  getI64Decoder,
  getI8Decoder,
  getStructDecoder,
  getU16Decoder,
  getU32Decoder,
  getU64Decoder,
  getU8Decoder,
  transformDecoder,
  type Decoder,
  type ReadonlyUint8Array,
} from "@solana/kit";

export const MAX_NUMBER_OF_PERP_ASSETS = 1024;
export const ORDERBOOK_CAPACITY = 8192;
export const CONDITIONAL_ORDER_BITS_LEN = 24;
const TRADER_POSITION_ENTRY_ASSET_INDEX_BOUND = 0xff000000n;
const TRADER_POSITION_ENTRY_ASSET_INDEX_MASK = 0xffffffffn;

export interface SequenceNumber {
  sequenceNumber: bigint;
  lastUpdateSlot: bigint;
}

export interface AuthoritySet {
  rootAuthority: Authority;
  riskAuthority: Authority;
  marketAuthority: Authority;
  oracleAuthority: Authority;
  reservedAuthority: Authority;
  adlAuthority: Authority;
  cancelAuthority: Authority;
  backstopAuthority: Authority;
}

export interface ShortEntries<K, V> {
  len: bigint;
  capacity: bigint;
  entries: Array<{ key: K; value: V }>;
}

export type TraderCapabilityFlags = number;

export interface TraderState {
  quoteLotCollateral: SignedQuoteLots;
  flags: TraderCapabilityFlags;
  globalPositionSequenceNumber: number;
  makerFeeOverrideMultiplier: number;
  takerFeeOverrideMultiplier: number;
}

export interface TraderPosition {
  baseLotPosition: SignedBaseLots;
  virtualQuoteLotPosition: SignedQuoteLots;
  cumulativeFundingSnapshot: bigint;
  positionSequenceNumber: number;
  accumulatedFundingForActivePosition: bigint;
}

export interface TicksAtSlot {
  slot: bigint;
  ticks: Ticks;
}

export interface OracleData {
  oraclePubkey: Authority;
  divergenceScore: number;
}

export interface OracleParameters {
  oracleDivergenceRadius: number;
  minOracleResponses: number;
}

export interface BookPriceComponent {
  clampedBookMidPrice: TicksAtSlot;
  weight: bigint;
  staleThreshold: bigint;
  bookPriceRadius: bigint;
  lastBestBid: TicksAtSlot;
  lastBestAsk: TicksAtSlot;
  lastTradePrice: TicksAtSlot;
}

export interface PerpPriceComponent {
  lastExchangePerpPrice: TicksAtSlot[];
  weight: bigint;
  staleThreshold: bigint;
}

export interface SpotPriceComponent {
  lastExchangeSpotPrice: TicksAtSlot[];
  weight: bigint;
  staleThreshold: bigint;
  slot: bigint;
  midSpotDiffEmaTicks: bigint;
  midSpotDiffEmaTicksDust: bigint;
  emaPeriodSlots: bigint;
  emaDiffRadius: bigint;
}

export interface MarkPrice {
  price: TicksAtSlot;
  spotPriceComponent: SpotPriceComponent;
  perpPriceComponent: PerpPriceComponent;
  bookPriceComponent: BookPriceComponent;
  riskActionPriceValidityRules: number[];
  oracleData: OracleData[];
  oracleParameters: OracleParameters;
  markPriceLastValidatedSlot: bigint[];
  oracleLastUpdatedTimestamps: bigint[];
}

export interface PriceComponent {
  priceSequenceNumber: SequenceNumber;
  markPrice: MarkPrice;
}

export interface StaticMarketParams {
  marketAccount: MarketAddress;
  tickSize: bigint;
  assetId: number;
  baseLotDecimals: number;
}

export interface LeverageTier {
  upperBoundSize: BaseLots;
  maxLeverage: bigint;
  limitOrderRiskFactor: bigint;
}

export interface TransferFeeTier {
  positionSizeLimit: BaseLots;
  feeRate: number;
}

export interface RiskParams {
  leverageTiers: LeverageTier[];
  upnlRiskFactor: bigint;
  maxLiquidationSize: BaseLots;
  transferFeeTiers: TransferFeeTier[];
  riskFactors: number[];
  cancelOrderRiskFactor: number;
  upnlRiskFactorForWithdrawals: bigint;
  isolatedOnly: number;
}

export interface FundingAccumulator {
  acc: bigint;
  lastDiff: bigint;
  cumulativeFundingRate: bigint;
  startIntervalTimestamp: bigint;
  lastFundingUpdateTimestamp: bigint;
  fundingIntervalSeconds: bigint;
  fundingPeriodSeconds: bigint;
  maxFundingRate: bigint;
}

export interface OpenInterestParams {
  openInterest: BaseLots;
  openInterestCap: BaseLots;
}

export interface AssetFlags {
  bits: number;
  isCommodity: boolean;
  isCommoditiesReopen: boolean;
  isCommoditiesAfterHours: boolean;
}

export interface PerpAssetMetadata {
  oraclePrice: PriceComponent;
  staticMarketParams: StaticMarketParams;
  riskParams: RiskParams;
  fundingAccumulator: FundingAccumulator;
  openInterestParams: OpenInterestParams;
  assetFlags: AssetFlags;
  commoditiesAfterHoursRadius: Ticks;
  lastKnownIndexPrice: Ticks | null;
  lastIndexExpiryTimestamp: bigint;
  commoditiesAfterHoursRadiusBps: number;
}

export interface TraderPositionEntry {
  assetId: bigint;
  position: TraderPosition;
}

export interface TickRegion {
  startOffset: Ticks;
  endOffset: Ticks;
  density: bigint;
  topLevelHiddenTakeSize: bigint;
  totalSize: BaseLots;
  filledSize: BaseLots;
  hiddenFilledSize: BaseLots;
  lifespan: bigint;
}

export interface Spline {
  trader: TraderAddress;
  isActive: boolean;
  midPrice: Ticks;
  sequenceNumber: SequenceNumber;
  userUpdateSlot: bigint;
  bidOffset: Ticks;
  askOffset: Ticks;
  bidFilledAmount: BaseLots;
  askFilledAmount: BaseLots;
  bidNumRegions: bigint;
  askNumRegions: bigint;
  bidRegions: TickRegion[];
  askRegions: TickRegion[];
  userPriceSequenceNumber: bigint;
  userParameterSequenceNumber: bigint;
  flags: number;
  leverageDecreaseInBps: bigint;
  maxPositionSizeLong: BaseLots;
  maxPositionSizeShort: BaseLots;
  hasMaxPositionSize: boolean;
}

export interface OrderFill {
  price: Ticks;
  quantity: SignedBaseLots;
}

export interface TradeEvent {
  slot: bigint;
  timestamp: bigint;
  tradeSequenceNumber: bigint;
  prevTradeSequenceNumberSlot: bigint;
  makerFill: OrderFill;
  maker: TraderAddress;
}

export interface TraderPositionId {
  traderId: number | null;
  assetId: number;
}

export interface OrderbookRestingOrder {
  traderPositionId: TraderPositionId;
  initialTradeSize: BaseLots;
  numBaseLotsRemaining: BaseLots;
  orderFlags: OrderFlags;
  optionalConditionalOrderIndex: number | null;
  expirationOffset: number | null;
  initialSlot: bigint;
  prevNode: number | null;
  nextNode: number | null;
  lastValidSlot: bigint | null;
  reduceOnly: boolean;
  isStopLoss: boolean;
  isStopLossDirection: boolean;
  isConditionalOrder: boolean;
}

export interface OrderbookHeader {
  marketStatus: number;
  baseLotsDecimals: number;
  sequenceNumber: SequenceNumber;
  assetId: number;
  assetSymbol: Symbol;
  tickSizeInQuoteLotsPerBaseLot: bigint;
  orderSequenceNumber: SequenceNumber;
  tradeSequenceNumber: SequenceNumber;
  defaultTakerFeeMicro: number;
  defaultMakerFeeMicro: number;
  totalMakerQuoteLotFees: SignedQuoteLots;
  totalTakerQuoteLotFees: QuoteLots;
  tradeList: TradeEvent[];
}

export interface OrderbookEntry {
  orderId: FIFOOrderId;
  order: OrderbookRestingOrder;
}

export const getFixedArrayDecoder = <T>(
  getItemDecoder: () => Decoder<T>,
  length: number
): Decoder<T[]> => {
  const itemDecoder = getItemDecoder();
  return createDecoder({
    read: (bytes: ReadonlyUint8Array | Uint8Array, offset: number) => {
      const items: T[] = [];
      let currentOffset = offset;
      for (let i = 0; i < length; i++) {
        const [item, nextOffset] = itemDecoder.read(bytes, currentOffset);
        items.push(item);
        currentOffset = nextOffset;
      }
      return [items, currentOffset];
    },
  });
};

export const getOptionalConditionalOrderIndexDecoder = (): Decoder<
  number | null
> => transformDecoder(getU8Decoder(), (value) => (value === 0 ? null : value));

export const getOptionalNonZeroU32Decoder = (): Decoder<number | null> =>
  transformDecoder(getU32Decoder(), (value) => (value === 0 ? null : value));

const getU32AsBigIntDecoder = (): Decoder<bigint> =>
  transformDecoder(getU32Decoder(), (value) => BigInt(value));

const getSignedI56Decoder = (): Decoder<bigint> =>
  createDecoder({
    read: (bytes: ReadonlyUint8Array | Uint8Array, offset: number) => {
      let value = 0n;
      for (let i = 0; i < 7; i++) {
        value |= BigInt(bytes[offset + i] ?? 0) << (8n * BigInt(i));
      }
      if ((bytes[offset + 6] ?? 0) & 0x80) {
        value |= -1n << 56n;
      }
      return [value, offset + 7];
    },
  });

const hasFlag = (flags: OrderFlags, flag: OrderFlags): boolean =>
  (Number(flags) & Number(flag)) !== 0;

const decodeConditionalOrderBits = (bytes: number[]): number[] => {
  const occupied: number[] = [];
  for (let byteIndex = 0; byteIndex < bytes.length; byteIndex++) {
    const value = bytes[byteIndex] ?? 0;
    for (let bit = 0; bit < 8; bit++) {
      if ((value & (1 << bit)) === 0) {
        continue;
      }
      const index = byteIndex * 8 + bit;
      if (index >= 1 && index <= 191) {
        occupied.push(index);
      }
    }
  }
  return occupied;
};

const readU32LE = (bytes: ReadonlyUint8Array, offset: number): number =>
  (bytes[offset] ?? 0) |
  ((bytes[offset + 1] ?? 0) << 8) |
  ((bytes[offset + 2] ?? 0) << 16) |
  ((bytes[offset + 3] ?? 0) << 24);

const calculateNodeStrideFromKeyStart = <K, V>(
  bytes: ReadonlyUint8Array,
  keyStart: number,
  getKeyDecoder: () => Decoder<K>,
  getValueDecoder: () => Decoder<V>
): number => {
  const keyDecoder = getKeyDecoder();
  const valueDecoder = getValueDecoder();
  const [, afterKey] = keyDecoder.read(bytes, keyStart);
  const [, afterValue] = valueDecoder.read(bytes, afterKey);
  return afterValue;
};

const calculateNodeStride = <K, V>(
  bytes: ReadonlyUint8Array,
  nodesBase: number,
  activeFlagBytes: number,
  getKeyDecoder: () => Decoder<K>,
  getValueDecoder: () => Decoder<V>
): number => {
  const afterValue = calculateNodeStrideFromKeyStart(
    bytes,
    nodesBase + 16 + activeFlagBytes,
    getKeyDecoder,
    getValueDecoder
  );
  return afterValue - nodesBase;
};

const dfsToEntries = <K, V>(
  bytes: ReadonlyUint8Array,
  nodesBase: number,
  nodeStrideBytes: number,
  root: number,
  getKeyDecoder: () => Decoder<K>,
  getValueDecoder: () => Decoder<V>
): Array<{ key: K; value: V }> => {
  const entries: Array<{ key: K; value: V }> = [];
  const keyDecoder = getKeyDecoder();
  const valueDecoder = getValueDecoder();
  const stack: number[] = [];
  let current = root >>> 0;

  while (current !== 0 || stack.length > 0) {
    while (current !== 0) {
      stack.push(current);
      const currentStart = nodesBase + (current - 1) * nodeStrideBytes;
      current = readU32LE(bytes, currentStart);
    }

    const nodeIndex = stack.pop();
    if (nodeIndex === undefined) {
      break;
    }

    const nodeStart = nodesBase + (nodeIndex - 1) * nodeStrideBytes;
    const kvStart = nodeStart + 16 + 8;
    const [key, afterKey] = keyDecoder.read(bytes, kvStart);
    const [value] = valueDecoder.read(bytes, afterKey);
    entries.push({ key, value });

    current = readU32LE(bytes, nodeStart + 4);
  }

  return entries;
};

export const getSequenceNumberDecoder = (): Decoder<SequenceNumber> =>
  getStructDecoder([
    ["sequenceNumber", getU64Decoder()],
    ["lastUpdateSlot", getU64Decoder()],
  ]);

export const getAuthoritySetDecoder = (): Decoder<AuthoritySet> =>
  getStructDecoder([
    ["rootAuthority", getAuthorityDecoder()],
    ["riskAuthority", getAuthorityDecoder()],
    ["marketAuthority", getAuthorityDecoder()],
    ["oracleAuthority", getAuthorityDecoder()],
    ["reservedAuthority", getAuthorityDecoder()],
    ["adlAuthority", getAuthorityDecoder()],
    ["cancelAuthority", getAuthorityDecoder()],
    ["backstopAuthority", getAuthorityDecoder()],
  ]);

export const getShortEntriesDecoder = <K, V>(
  getKeyDecoder: () => Decoder<K>,
  getValueDecoder: () => Decoder<V>
): Decoder<ShortEntries<K, V>> =>
  transformDecoder(
    getStructDecoder([
      ["len", getU64Decoder()],
      ["capacity", getU64Decoder()],
      ["data", getBytesDecoder()],
    ]),
    (shortMap): ShortEntries<K, V> => {
      const entries: Array<{ key: K; value: V }> = [];
      const keyDecoder = getKeyDecoder();
      const valueDecoder = getValueDecoder();
      let offset = 0;
      const buffer = new Uint8Array(
        shortMap.data.buffer,
        shortMap.data.byteOffset,
        shortMap.data.byteLength
      );

      for (let i = 0; i < Number(shortMap.len); i++) {
        const [key, afterKey] = keyDecoder.read(buffer, offset);
        const [value, afterValue] = valueDecoder.read(buffer, afterKey);
        entries.push({ key, value });
        offset = afterValue;
      }

      return {
        len: shortMap.len,
        capacity: shortMap.capacity,
        entries,
      };
    }
  );

export const getStaticRedBlackTreeEntriesDecoder = <K, V>(
  getKeyDecoder: () => Decoder<K>,
  getValueDecoder: () => Decoder<V>,
  options: { maxSize: number }
): Decoder<Array<{ key: K; value: V }>> => {
  const { maxSize } = options;

  return createDecoder({
    read: (bytes: ReadonlyUint8Array | Uint8Array, offset: number) => {
      const root = readU32LE(bytes, offset);
      const nodesBase = offset + 16 + 16;
      const nodeStrideBytes = calculateNodeStride(
        bytes,
        nodesBase,
        8,
        getKeyDecoder,
        getValueDecoder
      );
      const consumed = 16 + 16 + nodeStrideBytes * maxSize;
      if (root === 0) {
        return [[], offset + consumed];
      }
      return [
        dfsToEntries(
          bytes,
          nodesBase,
          nodeStrideBytes,
          root,
          getKeyDecoder,
          getValueDecoder
        ),
        offset + consumed,
      ];
    },
  });
};

export const getStaticOrderedListMapEntriesDecoder = <K, V>(
  getKeyDecoder: () => Decoder<K>,
  getValueDecoder: () => Decoder<V>,
  options: { maxSize: number }
): Decoder<Array<{ key: K; value: V }>> => {
  const { maxSize } = options;

  return createDecoder({
    read: (bytes: ReadonlyUint8Array | Uint8Array, offset: number) => {
      const head = readU32LE(bytes, offset);
      const size = Number(getU64Decoder().read(bytes, offset + 16)[0]);
      const nodesBase = offset + 16 + 16;
      const keyStart = nodesBase + 8;
      const afterValue = calculateNodeStrideFromKeyStart(
        bytes,
        keyStart,
        getKeyDecoder,
        getValueDecoder
      );
      const nodeStrideBytes = afterValue - nodesBase;
      const consumed = 16 + 16 + nodeStrideBytes * maxSize;
      const keyDecoder = getKeyDecoder();
      const valueDecoder = getValueDecoder();
      const entries: Array<{ key: K; value: V }> = [];
      let current = head >>> 0;

      for (let remaining = size; remaining > 0 && current !== 0; remaining--) {
        if (current > maxSize) {
          throw new Error(
            `StaticOrderedListMap node pointer ${current} exceeds capacity ${maxSize}`
          );
        }

        const nodeStart = nodesBase + (current - 1) * nodeStrideBytes;
        const [key, afterKey] = keyDecoder.read(bytes, nodeStart + 8);
        const [value] = valueDecoder.read(bytes, afterKey);
        entries.push({ key, value });
        current = readU32LE(bytes, nodeStart + 4);
      }

      if (entries.length !== size) {
        throw new Error(
          `StaticOrderedListMap decoded ${entries.length} entries, expected ${size}`
        );
      }

      return [entries, offset + consumed];
    },
  });
};

export const getCircBufDecoder = <T>(
  getElementDecoder: () => Decoder<T>,
  capacity: number
): Decoder<T[]> => {
  const elementDecoder = getElementDecoder();

  return createDecoder({
    read: (bytes: ReadonlyUint8Array | Uint8Array, offset: number) => {
      const ptr = readU32LE(bytes, offset);
      const len = readU32LE(bytes, offset + 4);
      const [, sampleEnd] = elementDecoder.read(bytes, offset + 8);
      const elementSize = sampleEnd - (offset + 8);
      const bufferOffset = offset + 8;

      const raw: T[] = [];
      let currentOffset = bufferOffset;
      for (let i = 0; i < capacity; i++) {
        const [element] = elementDecoder.read(bytes, currentOffset);
        raw.push(element);
        currentOffset += elementSize;
      }

      const ordered: T[] = [];
      for (let i = 0; i < len; i++) {
        ordered.push(raw[(ptr + i) % capacity]);
      }

      return [ordered, bufferOffset + capacity * elementSize];
    },
  });
};

export const getTraderStateDecoder = (): Decoder<TraderState> =>
  transformDecoder(
    getStructDecoder([
      ["quoteLotCollateral", getSignedQuoteLotsDecoder()],
      ["flags", getU32Decoder()],
      ["_padding", getU8Decoder()],
      ["globalPositionSequenceNumber", getU8Decoder()],
      ["makerFeeOverrideMultiplier", getI8Decoder()],
      ["takerFeeOverrideMultiplier", getI8Decoder()],
    ]),
    ({ _padding, ...state }): TraderState => state
  );

export const getTraderPositionDecoder = (): Decoder<TraderPosition> =>
  getStructDecoder([
    ["baseLotPosition", getSignedBaseLotsDecoder()],
    ["virtualQuoteLotPosition", getSignedQuoteLotsDecoder()],
    ["cumulativeFundingSnapshot", getI64Decoder()],
    ["positionSequenceNumber", getU8Decoder()],
    ["accumulatedFundingForActivePosition", getSignedI56Decoder()],
  ]);

export const getTraderPositionEntriesDecoder = (): Decoder<
  ShortEntries<bigint, TraderPosition>
> =>
  transformDecoder(
    getStructDecoder([
      ["len", getU64Decoder()],
      ["capacity", getU64Decoder()],
      ["data", getBytesDecoder()],
    ]),
    (shortMap): ShortEntries<bigint, TraderPosition> => {
      const entries: Array<{ key: bigint; value: TraderPosition }> = [];
      const assetIdDecoder = getU64Decoder();
      const positionDecoder = getTraderPositionDecoder();
      let offset = 0;
      const buffer = new Uint8Array(
        shortMap.data.buffer,
        shortMap.data.byteOffset,
        shortMap.data.byteLength
      );

      for (let i = 0; i < Number(shortMap.len); i++) {
        const [assetId, afterAssetId] = assetIdDecoder.read(buffer, offset);
        const [value, afterValue] = positionDecoder.read(buffer, afterAssetId);
        const assetIndex = assetId & TRADER_POSITION_ENTRY_ASSET_INDEX_MASK;
        if (assetIndex < TRADER_POSITION_ENTRY_ASSET_INDEX_BOUND) {
          entries.push({ key: assetIndex, value });
        } else {
          // Non-position entries share this short map but are not exposed here.
        }
        offset = afterValue;
      }

      return {
        len: BigInt(entries.length),
        capacity: shortMap.capacity,
        entries,
      };
    }
  );

export const getTicksAtSlotDecoder = (): Decoder<TicksAtSlot> =>
  getStructDecoder([
    ["slot", getU64Decoder()],
    ["ticks", getTicksDecoder()],
  ]);

export const getOracleDataDecoder = (): Decoder<OracleData> =>
  transformDecoder(
    getStructDecoder([
      ["oraclePubkey", getAuthorityDecoder()],
      ["divergenceScore", getU8Decoder()],
      ["_padding", getFixedArrayDecoder(getU8Decoder, 7)],
    ]),
    ({ _padding, ...data }): OracleData => data
  );

export const getOracleParametersDecoder = (): Decoder<OracleParameters> =>
  transformDecoder(
    getStructDecoder([
      ["oracleDivergenceRadius", getU16Decoder()],
      ["minOracleResponses", getU8Decoder()],
      ["_padding", getFixedArrayDecoder(getU8Decoder, 5)],
    ]),
    ({ _padding, ...parameters }): OracleParameters => parameters
  );

export const getBookPriceComponentDecoder = (): Decoder<BookPriceComponent> =>
  getStructDecoder([
    ["clampedBookMidPrice", getTicksAtSlotDecoder()],
    ["weight", getU64Decoder()],
    ["staleThreshold", getU64Decoder()],
    ["bookPriceRadius", getU64Decoder()],
    ["lastBestBid", getTicksAtSlotDecoder()],
    ["lastBestAsk", getTicksAtSlotDecoder()],
    ["lastTradePrice", getTicksAtSlotDecoder()],
  ]);

export const getPerpPriceComponentDecoder = (): Decoder<PerpPriceComponent> =>
  getStructDecoder([
    ["lastExchangePerpPrice", getFixedArrayDecoder(getTicksAtSlotDecoder, 5)],
    ["weight", getU64Decoder()],
    ["staleThreshold", getU64Decoder()],
  ]);

export const getSpotPriceComponentDecoder = (): Decoder<SpotPriceComponent> =>
  getStructDecoder([
    ["lastExchangeSpotPrice", getFixedArrayDecoder(getTicksAtSlotDecoder, 5)],
    ["weight", getU64Decoder()],
    ["staleThreshold", getU64Decoder()],
    ["slot", getU64Decoder()],
    ["midSpotDiffEmaTicks", getI64Decoder()],
    ["midSpotDiffEmaTicksDust", getI64Decoder()],
    ["emaPeriodSlots", getU64Decoder()],
    ["emaDiffRadius", getU64Decoder()],
  ]);

export const getMarkPriceDecoder = (): Decoder<MarkPrice> =>
  getStructDecoder([
    ["price", getTicksAtSlotDecoder()],
    ["spotPriceComponent", getSpotPriceComponentDecoder()],
    ["perpPriceComponent", getPerpPriceComponentDecoder()],
    ["bookPriceComponent", getBookPriceComponentDecoder()],
    ["riskActionPriceValidityRules", getFixedArrayDecoder(getU8Decoder, 256)],
    ["oracleData", getFixedArrayDecoder(getOracleDataDecoder, 5)],
    ["oracleParameters", getOracleParametersDecoder()],
    ["markPriceLastValidatedSlot", getFixedArrayDecoder(getU64Decoder, 4)],
    ["oracleLastUpdatedTimestamps", getFixedArrayDecoder(getU64Decoder, 5)],
  ]);

export const getPriceComponentDecoder = (): Decoder<PriceComponent> =>
  getStructDecoder([
    ["priceSequenceNumber", getSequenceNumberDecoder()],
    ["markPrice", getMarkPriceDecoder()],
  ]);

export const getStaticMarketParamsDecoder = (): Decoder<StaticMarketParams> =>
  transformDecoder(
    getStructDecoder([
      ["marketAccount", getMarketAddressDecoder()],
      ["tickSize", getU64Decoder()],
      ["assetIdLowerBytes", getU16Decoder()],
      ["baseLotDecimals", getI8Decoder()],
      ["_padding0", getU8Decoder()],
      ["assetIdUpperBytes", getU16Decoder()],
      ["_padding1", getFixedArrayDecoder(getU8Decoder, 2)],
    ]),
    ({
      assetIdLowerBytes,
      assetIdUpperBytes,
      _padding0,
      _padding1,
      marketAccount,
      tickSize,
      baseLotDecimals,
    }): StaticMarketParams => ({
      marketAccount,
      tickSize,
      assetId: assetIdLowerBytes + (assetIdUpperBytes << 16),
      baseLotDecimals,
    })
  );

export const getLeverageTierDecoder = (): Decoder<LeverageTier> =>
  getStructDecoder([
    ["upperBoundSize", getBaseLotsDecoder()],
    ["maxLeverage", getU64Decoder()],
    ["limitOrderRiskFactor", getU64Decoder()],
  ]);

export const getTransferFeeTierDecoder = (): Decoder<TransferFeeTier> =>
  transformDecoder(
    getStructDecoder([
      ["positionSizeLimit", getBaseLotsDecoder()],
      ["feeRate", getU16Decoder()],
      ["_padding", getFixedArrayDecoder(getU8Decoder, 6)],
    ]),
    ({ _padding, ...tier }): TransferFeeTier => tier
  );

export const getRiskParamsDecoder = (): Decoder<RiskParams> =>
  transformDecoder(
    getStructDecoder([
      ["leverageTiers", getFixedArrayDecoder(getLeverageTierDecoder, 4)],
      ["upnlRiskFactor", getU64Decoder()],
      ["maxLiquidationSize", getBaseLotsDecoder()],
      ["transferFeeTiers", getFixedArrayDecoder(getTransferFeeTierDecoder, 4)],
      ["riskFactors", getFixedArrayDecoder(getU16Decoder, 3)],
      ["cancelOrderRiskFactor", getU16Decoder()],
      ["upnlRiskFactorForWithdrawals", getU64Decoder()],
      ["isolatedOnly", getU8Decoder()],
      ["_padding", getFixedArrayDecoder(getU8Decoder, 7)],
    ]),
    ({ _padding, ...params }): RiskParams => params
  );

export const getFundingAccumulatorDecoder = (): Decoder<FundingAccumulator> =>
  transformDecoder(
    getStructDecoder([
      ["acc", getI128Decoder()],
      ["lastDiff", getI128Decoder()],
      ["cumulativeFundingRate", getI64Decoder()],
      ["startIntervalTimestamp", getU64Decoder()],
      ["lastFundingUpdateTimestamp", getU64Decoder()],
      ["fundingIntervalSeconds", getU64Decoder()],
      ["fundingPeriodSeconds", getU64Decoder()],
      ["maxFundingRate", getI64Decoder()],
      ["flags", getU8Decoder()],
      ["_padding", getFixedArrayDecoder(getU8Decoder, 15)],
    ]),
    ({ flags, _padding, ...accumulator }): FundingAccumulator => accumulator
  );

export const getOpenInterestParamsDecoder = (): Decoder<OpenInterestParams> =>
  getStructDecoder([
    ["openInterest", getBaseLotsDecoder()],
    ["openInterestCap", getBaseLotsDecoder()],
  ]);

const ASSET_FLAG_COMMODITY = 1 << 0;
const ASSET_FLAG_COMMODITIES_REOPEN = 1 << 1;
const ASSET_FLAG_COMMODITIES_AFTER_HOURS = 1 << 2;

export const getAssetFlagsDecoder = (): Decoder<AssetFlags> =>
  transformDecoder(
    getU8Decoder(),
    (bits): AssetFlags => ({
      bits,
      isCommodity: (bits & ASSET_FLAG_COMMODITY) !== 0,
      isCommoditiesReopen: (bits & ASSET_FLAG_COMMODITIES_REOPEN) !== 0,
      isCommoditiesAfterHours:
        (bits & ASSET_FLAG_COMMODITIES_AFTER_HOURS) !== 0,
    })
  );

const getOptionalTicksDecoder = (): Decoder<Ticks | null> =>
  transformDecoder(getTicksDecoder(), (value) => (value === 0n ? null : value));

interface PerpAssetMetadataInternal extends PerpAssetMetadata {
  shortMapMetadata: {
    indexNum: number;
    isTombstoned: number;
  };
}

const getPerpAssetMetadataInternalDecoder =
  (): Decoder<PerpAssetMetadataInternal> =>
    transformDecoder(
      getStructDecoder([
        ["oraclePrice", getPriceComponentDecoder()],
        ["_padding0", getFixedArrayDecoder(getU64Decoder, 2)],
        ["staticMarketParams", getStaticMarketParamsDecoder()],
        ["_padding1", getFixedArrayDecoder(getU64Decoder, 8)],
        ["riskParams", getRiskParamsDecoder()],
        ["_padding2", getFixedArrayDecoder(getU64Decoder, 8)],
        ["fundingAccumulator", getFundingAccumulatorDecoder()],
        ["_padding3", getFixedArrayDecoder(getU64Decoder, 6)],
        ["openInterestParams", getOpenInterestParamsDecoder()],
        ["finalizedMarkPrice", getU64Decoder()],
        [
          "shortMapMetadata",
          transformDecoder(
            getStructDecoder([
              ["indexNum", getU16Decoder()],
              ["isTombstoned", getU8Decoder()],
              ["_padding", getFixedArrayDecoder(getU8Decoder, 5)],
            ]),
            ({ _padding, ...metadata }) => metadata
          ),
        ],
        ["assetFlags", getAssetFlagsDecoder()],
        ["_paddingFlags", getFixedArrayDecoder(getU8Decoder, 7)],
        ["commoditiesAfterHoursRadius", getTicksDecoder()],
        ["lastKnownIndexPrice", getOptionalTicksDecoder()],
        ["lastIndexExpiryTimestamp", getU64Decoder()],
        ["commoditiesAfterHoursRadiusBps", getU16Decoder()],
        ["_padding4a", getFixedArrayDecoder(getU8Decoder, 6)],
        ["_padding4", getFixedArrayDecoder(getU64Decoder, 9)],
      ]),
      ({
        _padding0,
        _padding1,
        _padding2,
        _padding3,
        _padding4a,
        finalizedMarkPrice,
        _padding4,
        _paddingFlags,
        ...metadata
      }): PerpAssetMetadataInternal => metadata
    );

export const getPerpAssetMetadataDecoder = (): Decoder<PerpAssetMetadata> =>
  transformDecoder(
    getPerpAssetMetadataInternalDecoder(),
    ({ shortMapMetadata, ...metadata }): PerpAssetMetadata => metadata
  );

export const getPerpAssetMapMetadataEntriesDecoder = (): Decoder<
  ShortEntries<Symbol, PerpAssetMetadata>
> =>
  transformDecoder(
    getStructDecoder([
      ["slotsUsed", getU32Decoder()],
      ["tombstones", getU32Decoder()],
      ["capacity", getU64Decoder()],
      [
        "data",
        getFixedArrayDecoder(
          () =>
            getStructDecoder([
              ["key", getSymbolDecoder()],
              ["value", getPerpAssetMetadataInternalDecoder()],
            ]),
          MAX_NUMBER_OF_PERP_ASSETS
        ),
      ],
    ]),
    (raw) => {
      const entries = raw.data
        .slice(0, raw.slotsUsed)
        .filter((entry) => entry.value.shortMapMetadata.isTombstoned === 0)
        .map((entry) => ({
          key: entry.key,
          value: (() => {
            const { shortMapMetadata, ...metadata } = entry.value;
            return metadata;
          })(),
        }));

      return {
        len: BigInt(entries.length),
        capacity: raw.capacity,
        entries,
      };
    }
  );

export const getTickRegionDecoder = (): Decoder<TickRegion> =>
  getStructDecoder([
    ["startOffset", getTicksDecoder()],
    ["endOffset", getTicksDecoder()],
    ["density", getU32AsBigIntDecoder()],
    ["topLevelHiddenTakeSize", getU32AsBigIntDecoder()],
    ["totalSize", getBaseLotsDecoder()],
    [
      "filledSize",
      transformDecoder(getU32Decoder(), (v) => BigInt(v) as BaseLots),
    ],
    [
      "hiddenFilledSize",
      transformDecoder(getU32Decoder(), (v) => BigInt(v) as BaseLots),
    ],
    ["lifespan", getU64Decoder()],
  ]);

export const getSplineDecoder = (): Decoder<Spline> =>
  transformDecoder(
    getStructDecoder([
      ["trader", getTraderAddressDecoder()],
      ["isActive", getBooleanDecoder()],
      ["_padding", getFixedArrayDecoder(getU8Decoder, 7)],
      ["midPrice", getTicksDecoder()],
      ["sequenceNumber", getSequenceNumberDecoder()],
      ["userUpdateSlot", getU64Decoder()],
      ["bidOffset", getTicksDecoder()],
      ["askOffset", getTicksDecoder()],
      ["bidFilledAmount", getBaseLotsDecoder()],
      ["askFilledAmount", getBaseLotsDecoder()],
      ["bidNumRegions", getU64Decoder()],
      ["askNumRegions", getU64Decoder()],
      ["bidRegions", getFixedArrayDecoder(getTickRegionDecoder, 10)],
      ["askRegions", getFixedArrayDecoder(getTickRegionDecoder, 10)],
      ["userPriceSequenceNumber", getU64Decoder()],
      ["userParameterSequenceNumber", getU64Decoder()],
      ["flags", getU32Decoder()],
      ["leverageDecreaseInBps", getU32AsBigIntDecoder()],
      [
        "maxPositionSizeLong",
        transformDecoder(getU32Decoder(), (v) => BigInt(v) as BaseLots),
      ],
      [
        "maxPositionSizeShort",
        transformDecoder(getU32Decoder(), (v) => BigInt(v) as BaseLots),
      ],
      ["_padding2", getFixedArrayDecoder(getU64Decoder, 28)],
    ]),
    ({ _padding, _padding2, flags, ...spline }): Spline => ({
      ...spline,
      flags,
      hasMaxPositionSize: (flags & 1) !== 0,
    })
  );

export const getOrderFillDecoder = (): Decoder<OrderFill> =>
  getStructDecoder([
    ["price", getTicksDecoder()],
    ["quantity", getSignedBaseLotsDecoder()],
  ]);

export const getTradeEventDecoder = (): Decoder<TradeEvent> =>
  getStructDecoder([
    ["slot", getU64Decoder()],
    ["timestamp", getI64Decoder()],
    ["tradeSequenceNumber", getU64Decoder()],
    ["prevTradeSequenceNumberSlot", getU64Decoder()],
    ["makerFill", getOrderFillDecoder()],
    ["maker", getTraderAddressDecoder()],
  ]);

export const getTraderPositionIdDecoder = (): Decoder<TraderPositionId> =>
  getStructDecoder([
    ["traderId", getOptionalNonZeroU32Decoder()],
    ["assetId", getU32Decoder()],
  ]);

export const getOrderbookRestingOrderDecoder =
  (): Decoder<OrderbookRestingOrder> =>
    transformDecoder(
      getStructDecoder([
        ["traderPositionId", getTraderPositionIdDecoder()],
        ["initialTradeSize", getBaseLotsDecoder()],
        ["numBaseLotsRemaining", getBaseLotsDecoder()],
        ["orderFlags", getOrderFlagsDecoder()],
        [
          "optionalConditionalOrderIndex",
          getOptionalConditionalOrderIndexDecoder(),
        ],
        ["_padding", getFixedArrayDecoder(getU8Decoder, 2)],
        ["expirationOffset", getOptionalNonZeroU32Decoder()],
        ["initialSlot", getU64Decoder()],
        ["prevNode", getOptionalNonZeroU32Decoder()],
        ["nextNode", getOptionalNonZeroU32Decoder()],
      ]),
      ({ _padding, expirationOffset, orderFlags, initialSlot, ...order }) => ({
        ...order,
        orderFlags,
        expirationOffset,
        initialSlot,
        lastValidSlot:
          expirationOffset === null
            ? null
            : initialSlot + BigInt(expirationOffset),
        reduceOnly: hasFlag(orderFlags, OrderFlags.ReduceOnly),
        isStopLoss: hasFlag(orderFlags, OrderFlags.IsStopLoss),
        isStopLossDirection: hasFlag(
          orderFlags,
          OrderFlags.IsStopLossDirection
        ),
        isConditionalOrder: hasFlag(orderFlags, OrderFlags.IsConditionalOrder),
      })
    );

export const getOrderbookHeaderDecoder = (): Decoder<OrderbookHeader> =>
  transformDecoder(
    getStructDecoder([
      ["marketStatus", getU8Decoder()],
      ["baseLotsDecimals", getI8Decoder()],
      ["_padding0", getFixedArrayDecoder(getU8Decoder, 6)],
      ["sequenceNumber", getSequenceNumberDecoder()],
      ["assetId", getU32Decoder()],
      ["_assetIdPadding", getFixedArrayDecoder(getU8Decoder, 4)],
      ["assetSymbol", getSymbolDecoder()],
      ["tickSizeInQuoteLotsPerBaseLot", getU64Decoder()],
      ["orderSequenceNumber", getSequenceNumberDecoder()],
      ["tradeSequenceNumber", getSequenceNumberDecoder()],
      ["defaultTakerFeeMicro", getU32Decoder()],
      ["defaultMakerFeeMicro", getI32Decoder()],
      ["totalMakerQuoteLotFees", getSignedQuoteLotsDecoder()],
      ["totalTakerQuoteLotFees", getQuoteLotsDecoder()],
      ["_padding1", getFixedArrayDecoder(getU64Decoder, 32)],
      ["tradeList", getCircBufDecoder(getTradeEventDecoder, 64)],
    ]),
    ({ _padding0, _assetIdPadding, _padding1, ...header }) => header
  );

export const getOccupiedConditionalOrderIndices = (
  conditionalOrderBits: number[]
): number[] => decodeConditionalOrderBits(conditionalOrderBits);

export {
  getActiveTraderBufferAddressHeaderDecoder,
  getAuthorityDecoder,
  getFIFOOrderIdDecoder,
  getGlobalConfigurationAddressDecoder,
  getGlobalTraderIndexAddressHeaderDecoder,
  getGlobalVaultAddressDecoder,
  getMarketAddressDecoder,
  getMintAddressDecoder,
  getPerpAssetMapAddressDecoder,
  getTraderAddressDecoder,
  getSymbolDecoder,
  getWithdrawQueueAddressDecoder,
};
