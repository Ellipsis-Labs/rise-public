import type { BaseLots, QuoteLots, Ticks } from "../_numberTypes";
import type { Side } from "../Side";

/**
 * Order flags for specifying order behavior
 */
export enum OrderFlags {
  /** No special flags */
  None = 0,
  /** Stop-loss direction bit for TP/SL trigger-origin orders */
  IsStopLossDirection = 1 << 4,
  /** Conditional trigger-origin order bit */
  IsConditionalOrder = 1 << 5,
  /** TP/SL trigger-origin order bit */
  IsStopLoss = 1 << 6,
  /** Reduce only flag - order can only reduce existing position */
  ReduceOnly = 128, // 1 << 7
}

/**
 * Self-trade behavior for orders that can match against existing orders
 */
export enum SelfTradeBehavior {
  /** Abort the new order if it would self-trade */
  Abort = 0,
  /** Cancel the existing order and provide the new order */
  CancelProvide = 1,
  /** Decrement the existing order and provide the new order */
  DecrementTake = 2,
}

/**
 * This order type is used to place a limit order on the book.
 * It will never be matched against other existing limit orders
 */
export interface PostOnlyOrderPacket {
  side: Side;
  /**
   * The price of the order, in ticks
   */
  priceInTicks: Ticks;
  /**
   * The number of base lots to place on the book
   */
  numBaseLots: BaseLots;
  /**
   * The client order id used to identify the order in the response to the client
   */
  clientOrderId: bigint; // u128
  /**
   * Flag for whether or not to slide the order to the top of the book if it would immediately match
   */
  slide: boolean;
  /**
   * If this is set, the order will be invalid after the specified slot
   */
  lastValidSlot: bigint | null; // Option<u64>

  orderFlags: OrderFlags; // u8

  /**
   * If set, cancels resting orders rather than rejecting when margin check fails
   */
  cancelExisting: boolean;
}

/**
 * This order type is used to place a limit order on the book.
 * It can be matched against other existing limit orders, but will be posted
 * at the specified level if it is not matched.
 */
export interface LimitOrderPacket {
  /** Side of the order (Bid or Ask) */
  side: Side;
  /** The price of the order, in ticks */
  priceInTicks: Ticks;
  /** Total number of base lots to place on the book or fill at a better price */
  numBaseLots: BaseLots;
  /** How the matching engine should handle a self trade */
  selfTradeBehavior: SelfTradeBehavior;
  /** Number of orders to match against. If this is null there is no limit */
  matchLimit: bigint | null; // Option<u64>
  /** Client order id used to identify the order in the response to the client */
  clientOrderId: bigint; // u128
  /** If this is set, the order will be invalid after the specified slot */
  lastValidSlot: bigint | null; // Option<u64>

  orderFlags: OrderFlags; // u8

  /**
   * If set, cancels resting orders rather than rejecting when margin check fails
   */
  cancelExisting: boolean;
}

/**
 * This order type is used to place an order that will be matched against
 * existing resting orders. If the order matches fewer than `min_lots`
 * lots, it will be cancelled.
 *
 * Fill or Kill (FOK) orders are a subset of Immediate or Cancel (IOC)
 * orders where either the `numBaseLots` is equal to the
 * `minBaseLotsToFill` of the order, or the `numQuoteLots` is
 * equal to the `minQuoteLotsToFill` of the order.
 */
export interface ImmediateOrCancelOrderPacket {
  /** Side of the order (Bid or Ask) */
  side: Side;
  /**
   * The most aggressive price an order can be matched at. If this value is null,
   * then the order is treated as a market order.
   */
  priceInTicks: Ticks | null;
  /**
   * The number of base lots to fill against the order book. This must be set to a nonzero value.
   */
  numBaseLots: BaseLots;
  /**
   * The number of quote lots to fill against the order book. This can optionally be set to a nonzero value.
   */
  numQuoteLots: QuoteLots | null;
  /**
   * The minimum number of base lots to fill against the order book. If the order does not fill this many base lots, it will be voided.
   */
  minBaseLotsToFill: BaseLots;
  /**
   * The minimum number of quote lots to fill against the order book. If the order does not fill this many quote lots, it will be voided.
   */
  minQuoteLotsToFill: QuoteLots;
  /**
   * How the matching engine should handle a self trade.
   */
  selfTradeBehavior: SelfTradeBehavior;
  /**
   * Number of orders to match against. If set to null, there is no limit.
   */
  matchLimit: bigint | null; // Option<u64>
  /**
   * Client order id used to identify the order in the program's inner instruction data.
   */
  clientOrderId: bigint; // u128
  /**
   * If this is set, the order will be invalid after the specified slot
   */
  lastValidSlot: bigint | null; // Option<u64>

  orderFlags: OrderFlags; // u8

  /**
   * If set, cancels resting orders rather than rejecting when margin check fails
   */
  cancelExisting: boolean;
}

/**
 * Condensed order for multiple order packets
 */
export interface CondensedOrder {
  /** Price in ticks */
  priceInTicks: bigint; // u64
  /** Size in base lots */
  sizeInBaseLots: bigint; // u64
  /** If this is set, the order will be invalid after the specified slot */
  lastValidSlot: bigint | null; // Option<u64>
}

/**
 * Multiple order packet for sending multiple bids and asks as PostOnly orders in a single packet
 */
export interface MultipleOrderPacket {
  /** Bids and asks are in the format (price in ticks, size in base lots) */
  bids: CondensedOrder[];
  asks: CondensedOrder[];
  /** Client order id for the batch */
  clientOrderId: bigint | null;
  /** Whether orders should slide to the top of the book if they would cross. If false, orders that would cross generate PostOnlyCross error */
  slide: boolean;
}
